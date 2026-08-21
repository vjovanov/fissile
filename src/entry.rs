//! Locating, sizing, and spelling one exception entry — what `exception add` and
//! `exception retune` share (§FS-005-exception-add, §FS-008-exception-retune).
//! An entry is addressed by its registry and condition (§DF-005-exception-identity).

use std::path::PathBuf;

use crate::cli::{self, CommandError, Loaded};
use crate::exceptions::{Exception, MatchKind, Registries, RegistrySource};
use crate::{Glob, Rule, Severity, Unit, scan};

/// What identifies one entry, across both commands that write entries.
#[derive(Clone, Copy, Debug)]
pub struct Address<'a> {
    pub severity: Severity,
    pub path: &'a str,
    pub match_kind: MatchKind,
    pub rules: &'a [String],
    pub unit: Unit,
}

/// The written ceiling: the smallest multiple of `step` at or above the value,
/// so a registry records a round number, not one commit's measurement
/// (§FS-005-exception-add.2, §DF-006-quantized-ceilings.1). `0`/`1` disable it.
pub fn quantize(value: u64, step: u64) -> u64 {
    if step <= 1 {
        return value;
    }
    value.div_ceil(step).saturating_mul(step)
}

/// Every entry in the addressed registry whose matcher, rules, and unit overlap
/// `address`, each with its index — the block order an in-place rewrite needs
/// (§FS-008-exception-retune.3).
pub fn matching<'a>(
    registries: &'a Registries,
    address: &Address<'_>,
) -> Vec<(usize, &'a Exception)> {
    let registry = match address.severity {
        Severity::Soft => &registries.soft,
        Severity::Hard => &registries.hard,
    };
    registry
        .iter()
        .enumerate()
        .filter(|(_, entry)| {
            entry.max_unit == address.unit
                && address.rules.iter().any(|rule| entry.applies_to_rule(rule))
                && path_matchers_overlap(entry, address.match_kind, address.path)
        })
        .collect()
}

/// The one entry `address` names (§FS-008-exception-retune.4). Matching two is
/// an ambiguous address, not a broken registry — two exact entries under one
/// glob is a registry §FS-003-exceptions.4 accepts — so the refusal names both.
pub fn locate<'a>(
    registries: &'a Registries,
    address: &Address<'_>,
) -> Result<Option<(usize, &'a Exception)>, CommandError> {
    let mut found = matching(registries, address).into_iter();
    let Some((index, entry)) = found.next() else {
        return Ok(None);
    };
    if let Some((_, second)) = found.next() {
        return Err(CommandError::Usage(format!(
            "{}: {} spans more than one entry ({} and {}); address one at a time — \
             each entry is named by its own path matcher",
            entry.registry, address.path, entry.path, second.path
        )));
    }
    Ok(Some((index, entry)))
}

/// Whether an existing entry's matcher and a proposed one can cover one path.
pub fn path_matchers_overlap(entry: &Exception, match_kind: MatchKind, path: &str) -> bool {
    match (entry.match_kind, match_kind) {
        (MatchKind::Exact, MatchKind::Exact) => entry.path == path,
        (MatchKind::Glob, MatchKind::Exact) => entry.matches_path(path),
        (MatchKind::Exact, MatchKind::Glob) => Glob::new(path).matches(&entry.path),
        (MatchKind::Glob, MatchKind::Glob) => Glob::new(&entry.path).intersects(&Glob::new(path)),
    }
}

/// `--match` must agree with the path's shape: a glob entry needs a glob, and a
/// metacharacter in an exact path is a mistake worth naming.
pub fn validate_match(match_kind: MatchKind, path: &str) -> Result<(), CommandError> {
    let has_meta = path.contains(['*', '?', '[']);
    match match_kind {
        MatchKind::Glob if !has_meta => Err(CommandError::Usage(
            "--match glob requires a glob metacharacter in <path>".to_owned(),
        )),
        MatchKind::Exact if has_meta => Err(CommandError::Usage(
            "<path> contains a glob metacharacter; pass --match glob".to_owned(),
        )),
        _ => Ok(()),
    }
}

/// Resolve `--rule` ids against the effective config. Every selected rule must
/// share one unit, since the entry records exactly one (§FS-003-exceptions.2).
pub fn resolve_rules<'a>(
    loaded: &'a Loaded,
    rule_ids: &[String],
) -> Result<Vec<&'a Rule>, CommandError> {
    if rule_ids.is_empty() {
        return Err(CommandError::Usage(
            "at least one --rule is required".to_owned(),
        ));
    }
    let mut rules = Vec::new();
    for id in rule_ids {
        let rule = loaded
            .checker
            .rules()
            .iter()
            .find(|rule| &rule.id == id)
            .ok_or_else(|| CommandError::Usage(format!("unknown rule id {id}")))?;
        rules.push(rule);
    }
    let unit = rules[0].budget.unit;
    if rules.iter().any(|rule| rule.budget.unit != unit) {
        return Err(CommandError::Usage(
            "all selected rules must share one unit".to_owned(),
        ));
    }
    Ok(rules)
}

/// Measure one file in `unit`, under the line policy the rule declares
/// (§FS-001-config.3.1) — the same arithmetic a finding reports.
pub fn measure_value(
    loaded: &Loaded,
    path: &str,
    unit: Unit,
    rule: &Rule,
) -> Result<u64, CommandError> {
    let measurement = scan::measure_file(&loaded.root, path, &loaded.config.tokens)?;
    match unit {
        Unit::Bytes => Ok(measurement.bytes),
        Unit::Lines => Ok(measurement
            .lines
            .map(|stats| stats.counted(rule.count_blank_lines, rule.count_comment_lines))
            .unwrap_or(0)),
        Unit::Tokens => measurement.tokens.ok_or_else(|| {
            CommandError::Usage(format!("no token measurement available for {path}"))
        }),
    }
}

/// What the caller said about size: `--max`/`--unit` when given, plus the
/// matcher that decides whether a single measurement exists to fall back on.
#[derive(Clone, Copy, Debug)]
pub struct Sizing<'a> {
    pub path: &'a str,
    pub match_kind: MatchKind,
    pub max: Option<u64>,
    pub unit: Option<Unit>,
}

/// The value a ceiling has to clear, before quantization, and where it came
/// from. `measured` is the file's own size whenever the address named one file,
/// carried out so a diagnostic can quote it without measuring twice.
#[derive(Clone, Copy, Debug)]
pub struct Base<'a> {
    pub value: u64,
    pub measured: Option<u64>,
    pub source: BaseSource<'a>,
}

/// Where a ceiling's value came from, so a refusal names the input the caller
/// actually gave (§GOAL-003-friendly-output).
#[derive(Clone, Copy, Debug)]
pub enum BaseSource<'a> {
    /// `--max <N>`, straight from the command line.
    Max,
    /// The file's own measurement, taken because `--max` was omitted.
    Measured(&'a str),
}

/// The value a ceiling has to clear, before quantization: `--max` when given,
/// otherwise the file's current measurement (§FS-005-exception-add.2).
pub fn resolve_base<'a>(
    sizing: Sizing<'a>,
    loaded: &Loaded,
    unit: Unit,
    rule: &Rule,
) -> Result<Base<'a>, CommandError> {
    match sizing.max {
        Some(max) => {
            let declared = sizing
                .unit
                .ok_or_else(|| CommandError::Usage("--max requires --unit".to_owned()))?;
            if declared != unit {
                return Err(CommandError::Usage(
                    "--unit must match the selected rule unit".to_owned(),
                ));
            }
            // A glob names no file to measure; an exact path does, and its
            // ceiling may not fall below it (§FS-008-exception-retune.2).
            if sizing.match_kind != MatchKind::Exact {
                return Ok(Base {
                    value: max,
                    measured: None,
                    source: BaseSource::Max,
                });
            }
            let measured = measure_value(loaded, sizing.path, unit, rule)?;
            if max < measured {
                return Err(CommandError::Usage(format!(
                    "--max {max} is below the current measurement {measured} {unit}"
                )));
            }
            Ok(Base {
                value: max,
                measured: Some(measured),
                source: BaseSource::Max,
            })
        }
        None => match sizing.match_kind {
            MatchKind::Exact => {
                let measured = measure_value(loaded, sizing.path, unit, rule)?;
                Ok(Base {
                    value: measured,
                    measured: Some(measured),
                    source: BaseSource::Measured(sizing.path),
                })
            }
            MatchKind::Glob => Err(CommandError::Usage(
                "--match glob requires --max and --unit".to_owned(),
            )),
        },
    }
}

/// A ceiling below the limit it is meant to accept would silence nothing
/// (§FS-005-exception-add.2). The refusal blames the base's own source — never a
/// flag the caller did not pass — and offers `remedy`, since no `--max` is one.
pub fn check_min_limit(
    rules: &[&Rule],
    severity: Severity,
    unit: Unit,
    base: &Base<'_>,
    remedy: &str,
) -> Result<(), CommandError> {
    let max = base.value;
    for rule in rules {
        let limit = match severity {
            Severity::Soft => rule.budget.soft,
            Severity::Hard => rule.budget.hard,
        };
        let Some(limit) = limit else {
            return Err(CommandError::Usage(format!(
                "rule {} has no {severity} limit to accept",
                rule.id
            )));
        };
        if max < limit {
            return Err(CommandError::Usage(match base.source {
                BaseSource::Max => format!(
                    "--max {max} is below rule {} {severity} limit {limit}",
                    rule.id
                ),
                BaseSource::Measured(path) => format!(
                    "{path} measures {max} {unit}, under rule {} {severity} limit {limit}; \
                     an entry accepting it would silence nothing — {remedy}",
                    rule.id
                ),
            }));
        }
    }
    Ok(())
}

/// The configured registry path for a severity, relative to the repo root.
pub fn registry_path(loaded: &Loaded, severity: Severity) -> PathBuf {
    match severity {
        Severity::Soft => loaded.soft_registry.clone(),
        Severity::Hard => loaded.hard_registry.clone(),
    }
}

/// Re-validate both registries with one of them replaced by the text about to be
/// written, so a command never leaves a registry it cannot read back
/// (§FS-005-exception-add.4).
pub fn validate_combined(
    loaded: &Loaded,
    severity: Severity,
    new_target_text: &str,
) -> Result<(), CommandError> {
    let (soft, hard) = match severity {
        Severity::Soft => (
            Some(new_target_text.to_owned()),
            cli::read_optional(&loaded.root.join(&loaded.hard_registry))?,
        ),
        Severity::Hard => (
            cli::read_optional(&loaded.root.join(&loaded.soft_registry))?,
            Some(new_target_text.to_owned()),
        ),
    };
    // The configured paths, the same labels `cli::load` uses, so a diagnostic
    // from this dry run reads identically to one from `check`.
    let registries = Registries::load(
        soft.as_deref()
            .map(|text| RegistrySource::new(&loaded.config.exceptions.soft_registry, text)),
        hard.as_deref()
            .map(|text| RegistrySource::new(&loaded.config.exceptions.hard_registry, text)),
    )?;
    registries.validate_against(loaded.checker.rules())?;
    Ok(())
}

/// The one spelling of `max_accepted` both commands write.
pub fn max_accepted_line(value: u64, unit: Unit) -> String {
    format!(
        "max_accepted = {{ value = {value}, unit = {} }}",
        quote(&unit.to_string())
    )
}

/// Quote a TOML basic string, escaping the characters that would break it.
pub fn quote(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\t' => escaped.push_str("\\t"),
            c => escaped.push(c),
        }
    }
    escaped.push('"');
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quote_escapes_specials() {
        assert_eq!(quote("a\"b"), "\"a\\\"b\"");
    }

    #[test]
    fn quantize_rounds_up_to_the_step() {
        assert_eq!(quantize(488, 100), 500);
        assert_eq!(quantize(500, 100), 500);
        assert_eq!(quantize(501, 100), 600);
    }

    #[test]
    fn a_step_of_one_writes_the_measurement() {
        assert_eq!(quantize(488, 1), 488);
        assert_eq!(quantize(488, 0), 488);
    }
}
