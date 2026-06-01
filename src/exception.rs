//! `fissile exception add` (§FS-005-exception-add): append a structured exception
//! entry — measure the file or take `--max`, pick the soft or hard registry,
//! validate against §FS-003-exceptions, then write (appending at the end).

use std::fs;
use std::path::PathBuf;

use crate::cli::{self, CommandError, Loaded};
use crate::exceptions::{Exception, MatchKind, Registries};
use crate::{Glob, Rule, Severity, Unit, scan};

/// Inputs to `exception add`.
#[derive(Clone, Debug)]
pub struct AddOptions {
    pub root: PathBuf,
    pub config_path: Option<PathBuf>,
    pub path: String,
    pub severity: Severity,
    pub rules: Vec<String>,
    pub reason: String,
    pub until: String,
    pub match_kind: MatchKind,
    pub id: Option<String>,
    pub title: Option<String>,
    pub owner: Option<String>,
    pub issue: Option<String>,
    pub replaces: Option<String>,
    pub max: Option<u64>,
    pub unit: Option<Unit>,
    pub dry_run: bool,
}

pub struct Run {
    pub output: String,
}

pub fn run(options: &AddOptions) -> Result<Run, CommandError> {
    let loaded = cli::load(&options.root, options.config_path.as_deref())?;
    let path = match options.match_kind {
        MatchKind::Exact => scan::normalize_repo_path(&loaded.root, &options.path)?,
        MatchKind::Glob => options.path.replace('\\', "/"),
    };

    validate_match(&options.match_kind, &path)?;
    let rules = resolve_rules(&loaded, &options.rules)?;
    let unit = rules[0].budget.unit;
    let max = resolve_max(options, &loaded, &path, unit, rules[0])?;
    check_min_limit(&rules, options.severity, max)?;
    let id = resolve_id(options, &loaded)?;
    check_conflict(&loaded, options, &path, unit)?;

    let entry = render_entry(options, &path, &id, unit, max);
    let registry_rel = match options.severity {
        Severity::Soft => loaded.soft_registry.clone(),
        Severity::Hard => loaded.hard_registry.clone(),
    };
    let registry_path = loaded.root.join(&registry_rel);

    let existing = cli::read_optional(&registry_path)?;
    let base = existing.unwrap_or_else(|| "fissile_exceptions_version = 1\n".to_owned());
    let new_text = format!("{}\n{}\n", base.trim_end(), entry);

    // Final guard: the combined registry must still validate (§FS-005-exception-add.4).
    validate_combined(&loaded, options.severity, &new_text)?;

    if options.dry_run {
        return Ok(Run {
            output: format!("{entry}\nwould update {}", registry_rel.display()),
        });
    }

    if let Some(parent) = registry_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&registry_path, &new_text)?;
    Ok(Run {
        output: format!("appended {} to {}", id, registry_rel.display()),
    })
}

fn validate_match(match_kind: &MatchKind, path: &str) -> Result<(), CommandError> {
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

fn resolve_rules<'a>(
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

fn resolve_max(
    options: &AddOptions,
    loaded: &Loaded,
    path: &str,
    unit: Unit,
    rule: &Rule,
) -> Result<u64, CommandError> {
    match options.max {
        Some(max) => {
            let declared = options
                .unit
                .ok_or_else(|| CommandError::Usage("--max requires --unit".to_owned()))?;
            if declared != unit {
                return Err(CommandError::Usage(
                    "--unit must match the selected rule unit".to_owned(),
                ));
            }
            // For an exact path, the accepted ceiling cannot be below the current
            // measurement (§FS-005-exception-add.2).
            if options.match_kind == MatchKind::Exact {
                let measured = measure_value(loaded, path, unit, rule)?;
                if max < measured {
                    return Err(CommandError::Usage(format!(
                        "--max {max} is below the current measurement {measured} {unit}"
                    )));
                }
            }
            Ok(max)
        }
        None => match options.match_kind {
            MatchKind::Exact => measure_value(loaded, path, unit, rule),
            MatchKind::Glob => Err(CommandError::Usage(
                "--match glob requires --max and --unit".to_owned(),
            )),
        },
    }
}

fn measure_value(
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

fn check_min_limit(rules: &[&Rule], severity: Severity, max: u64) -> Result<(), CommandError> {
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
            return Err(CommandError::Usage(format!(
                "--max {max} is below rule {} {severity} limit {limit}",
                rule.id
            )));
        }
    }
    Ok(())
}

fn resolve_id(options: &AddOptions, loaded: &Loaded) -> Result<String, CommandError> {
    if let Some(id) = &options.id {
        if loaded.registries.all().any(|entry| &entry.id == id) {
            return Err(CommandError::Usage(format!(
                "exception id {id} already exists"
            )));
        }
        return Ok(id.clone());
    }
    let next = next_number(&loaded.registries);
    Ok(format!("EX-{next:03}-{}", slug(&options.path)))
}

fn next_number(registries: &Registries) -> u32 {
    registries
        .all()
        .filter_map(|entry| {
            entry
                .id
                .strip_prefix("EX-")
                .and_then(|rest| rest.split('-').next())
                .and_then(|digits| digits.parse::<u32>().ok())
        })
        .max()
        .map(|max| max + 1)
        .unwrap_or(1)
}

fn slug(path: &str) -> String {
    let base = path.rsplit('/').next().unwrap_or(path);
    let mut slug = String::new();
    let mut prev_dash = false;
    for ch in base.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash {
            slug.push('-');
            prev_dash = true;
        }
    }
    let trimmed = slug.trim_matches('-').to_owned();
    if trimmed.is_empty() {
        "entry".to_owned()
    } else {
        trimmed
    }
}

/// Reject another same-severity entry already accepting the same `(path, rule,
/// unit)` condition (§FS-005-exception-add.4).
fn check_conflict(
    loaded: &Loaded,
    options: &AddOptions,
    path: &str,
    unit: Unit,
) -> Result<(), CommandError> {
    let registry = match options.severity {
        Severity::Soft => &loaded.registries.soft,
        Severity::Hard => &loaded.registries.hard,
    };
    for entry in registry {
        let shared_rule = options.rules.iter().any(|rule| entry.applies_to_rule(rule));
        if entry.max_unit == unit
            && shared_rule
            && path_matchers_overlap(entry, options.match_kind, path)
        {
            return Err(CommandError::Usage(format!(
                "exception {} already accepts {path} for this unit and rule",
                entry.id
            )));
        }
    }
    Ok(())
}

fn path_matchers_overlap(entry: &Exception, match_kind: MatchKind, path: &str) -> bool {
    match (entry.match_kind, match_kind) {
        (MatchKind::Exact, MatchKind::Exact) => entry.path == path,
        (MatchKind::Glob, MatchKind::Exact) => entry.matches_path(path),
        (MatchKind::Exact, MatchKind::Glob) => Glob::new(path).matches(&entry.path),
        (MatchKind::Glob, MatchKind::Glob) => Glob::new(&entry.path).intersects(&Glob::new(path)),
    }
}

fn validate_combined(
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
    let registries = Registries::load(soft.as_deref(), hard.as_deref())?;
    registries.validate_against(loaded.checker.rules())?;
    Ok(())
}

fn render_entry(options: &AddOptions, path: &str, id: &str, unit: Unit, max: u64) -> String {
    let mut lines = vec!["[[exceptions]]".to_owned(), format!("id = {}", quote(id))];
    if let Some(title) = &options.title {
        lines.push(format!("title = {}", quote(title)));
    }
    lines.push(format!("path = {}", quote(path)));
    lines.push(format!("match = {}", quote(match_str(&options.match_kind))));
    lines.push(format!("rules = [{}]", rule_list(&options.rules)));
    lines.push(format!(
        "max_accepted = {{ value = {max}, unit = {} }}",
        quote(&unit.to_string())
    ));
    lines.push(format!("until = {}", quote(&options.until)));
    if let Some(owner) = &options.owner {
        lines.push(format!("owner = {}", quote(owner)));
    }
    if let Some(issue) = &options.issue {
        lines.push(format!("issue = {}", quote(issue)));
    }
    if let Some(replaces) = &options.replaces {
        lines.push(format!("replaces = {}", quote(replaces)));
    }
    lines.push(format!(
        "reason = \"\"\"\n{}\n\"\"\"",
        options.reason.trim()
    ));
    lines.join("\n")
}

fn rule_list(rules: &[String]) -> String {
    rules
        .iter()
        .map(|rule| quote(rule))
        .collect::<Vec<_>>()
        .join(", ")
}

fn match_str(match_kind: &MatchKind) -> &'static str {
    match match_kind {
        MatchKind::Exact => "exact",
        MatchKind::Glob => "glob",
    }
}

/// Quote a TOML basic string, escaping the characters that would break it.
fn quote(value: &str) -> String {
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
    fn slug_uses_basename() {
        assert_eq!(
            slug("tests/fixtures/large-corpus.json"),
            "large-corpus-json"
        );
        assert_eq!(slug("a/b/c"), "c");
    }

    #[test]
    fn quote_escapes_specials() {
        assert_eq!(quote("a\"b"), "\"a\\\"b\"");
    }
}
