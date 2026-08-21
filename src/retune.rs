//! `fissile exception retune` (§FS-008-exception-retune): move the ceiling of an
//! entry that already exists. It locates the entry, quantizes the new value, and
//! rewrites that one `max_accepted` line — leaving the rest of the registry alone.

use std::fs;
use std::path::PathBuf;

use crate::cli::{self, CommandError, Loaded};
use crate::entry::{self, Address, Sizing};
use crate::exceptions::MatchKind;
use crate::{Severity, Unit, scan};

/// Inputs to `exception retune`.
#[derive(Clone, Debug)]
pub struct RetuneOptions {
    pub root: PathBuf,
    pub config_path: Option<PathBuf>,
    pub path: String,
    pub severity: Severity,
    pub rules: Vec<String>,
    pub match_kind: MatchKind,
    pub max: Option<u64>,
    pub unit: Option<Unit>,
    pub dry_run: bool,
}

#[derive(Clone, Debug)]
pub struct Run {
    pub output: String,
}

pub fn run(options: &RetuneOptions) -> Result<Run, CommandError> {
    let loaded = cli::load(&options.root, options.config_path.as_deref())?;
    let path = match options.match_kind {
        MatchKind::Exact => scan::normalize_repo_path(&loaded.root, &options.path)?,
        MatchKind::Glob => options.path.replace('\\', "/"),
    };

    entry::validate_match(options.match_kind, &path)?;
    let rules = entry::resolve_rules(&loaded, &options.rules)?;
    let unit = rules[0].budget.unit;
    let registry_rel = entry::registry_path(&loaded, options.severity);

    let address = Address {
        severity: options.severity,
        path: &path,
        match_kind: options.match_kind,
        rules: &options.rules,
        unit,
    };
    // Retune edits; it never creates. An address with no entry behind it is a
    // call for the other command (§FS-008-exception-retune.1).
    let Some((index, existing)) = entry::locate(&loaded.registries, &address)? else {
        return Err(CommandError::Usage(format!(
            "{}: no entry accepts {path} for this rule and unit; \
             create one with `fissile exception add`",
            registry_rel.display()
        )));
    };
    // A glob's ceiling covers every file in its class, so deriving it from one
    // member's measurement could lower it under the others. The caller has to
    // address the glob and state the number (§FS-003-exceptions.7).
    if existing.match_kind == MatchKind::Glob && options.match_kind == MatchKind::Exact {
        return Err(CommandError::Usage(format!(
            "{}: {path} is covered by the glob entry {}; retune it as \
             `--match glob \"{}\" --max <N> --unit {unit}`, since one file's size \
             cannot set a ceiling for the class",
            registry_rel.display(),
            existing.path,
            existing.path
        )));
    }
    let recorded = existing.max_value;

    let sizing = Sizing {
        path: &path,
        match_kind: options.match_kind,
        max: options.max,
        unit: options.unit,
    };
    let base = entry::resolve_base(sizing, &loaded, unit, rules[0])?;
    entry::check_min_limit(&rules, options.severity, base)?;
    let ceiling = entry::quantize(base, loaded.config.exceptions.bump.step(unit));

    // A caller about to leave two registries disagreeing should learn it here
    // rather than from a later run (§FS-008-exception-retune.3).
    let note = twin_note(&loaded, options, &path, unit, ceiling);

    if ceiling == recorded {
        // An edit that stayed inside the step is a normal outcome, not a failure.
        return Ok(Run {
            output: with_note(
                format!(
                    "{}: {path} already accepts {recorded} {unit}",
                    registry_rel.display()
                ),
                note,
            ),
        });
    }

    let registry_path = loaded.root.join(&registry_rel);
    let text = cli::read_optional(&registry_path)?
        .ok_or_else(|| CommandError::Usage(format!("{} is missing", registry_rel.display())))?;
    let new_text = rewrite_ceiling(&text, index, ceiling, unit, &registry_rel, &path)?;
    entry::validate_combined(&loaded, options.severity, &new_text)?;

    let change = format!(
        "{}: {path} {recorded} -> {ceiling} {unit}",
        registry_rel.display()
    );
    if options.dry_run {
        return Ok(Run {
            output: with_note(
                format!("{change}\nwould update {}", registry_rel.display()),
                note,
            ),
        });
    }

    fs::write(&registry_path, &new_text)?;
    Ok(Run {
        output: with_note(change, note),
    })
}

fn with_note(head: String, note: Option<String>) -> String {
    match note {
        Some(note) => format!("{head}\n{note}"),
        None => head,
    }
}

/// The same address in the registry the caller did not select. `retune` never
/// writes there — twin consistency is a repository's policy, not the tool's — but
/// it reports a ceiling the edit is about to contradict.
fn twin_note(
    loaded: &Loaded,
    options: &RetuneOptions,
    path: &str,
    unit: Unit,
    ceiling: u64,
) -> Option<String> {
    let address = Address {
        severity: options.severity.other(),
        path,
        match_kind: options.match_kind,
        rules: &options.rules,
        unit,
    };
    let (_, twin) = entry::locate(&loaded.registries, &address).ok().flatten()?;
    (twin.max_value != ceiling).then(|| {
        format!(
            "note: {} accepts {} up to {} {unit}",
            twin.registry, twin.path, twin.max_value
        )
    })
}

/// Rewrite the `max_accepted` line of the `index`-th `[[exceptions]]` block,
/// preserving every other byte, so the diff is the one decision that changed
/// (§FS-008-exception-retune.3).
fn rewrite_ceiling(
    text: &str,
    index: usize,
    value: u64,
    unit: Unit,
    registry: &std::path::Path,
    path: &str,
) -> Result<String, CommandError> {
    // Split rather than `lines()`: this round-trips the original bytes, including
    // the final newline and any `\r`, since only one line may change.
    let mut lines: Vec<String> = text.split('\n').map(str::to_owned).collect();
    let mut block: Option<usize> = None;
    let mut target = None;
    let mut in_multiline = false;

    for (number, line) in lines.iter().enumerate() {
        let fences = line.matches("\"\"\"").count();
        if !in_multiline {
            let trimmed = line.trim();
            if trimmed.starts_with("[[exceptions]]") {
                block = Some(block.map_or(0, |current| current + 1));
            } else if block == Some(index) && is_max_accepted(trimmed) {
                target = Some(number);
            }
        }
        // A `reason = """` opens the string and a lone `"""` closes it; a
        // single-line triple-quoted value has two fences and toggles nothing.
        if fences % 2 == 1 {
            in_multiline = !in_multiline;
        }
    }

    // Reached only by a registry that spells the ceiling as a sub-table rather
    // than the inline form both commands write. Refusing beats guessing at a
    // rewrite that could silently corrupt the entry.
    let Some(number) = target else {
        return Err(CommandError::Usage(format!(
            "{}: cannot rewrite the ceiling for {path}; the entry does not spell \
             `max_accepted` as one inline table — edit it by hand",
            registry.display()
        )));
    };

    let indent: String = lines[number]
        .chars()
        .take_while(|character| character.is_whitespace())
        .collect();
    lines[number] = format!("{indent}{}", entry::max_accepted_line(value, unit));
    Ok(lines.join("\n"))
}

fn is_max_accepted(trimmed: &str) -> bool {
    trimmed
        .strip_prefix("max_accepted")
        .is_some_and(|rest| rest.trim_start().starts_with('='))
}

#[cfg(test)]
mod tests {
    use super::*;

    const REGISTRY: &str = "fissile_exceptions_version = 2\n\
        \n\
        [[exceptions]]\n\
        path = \"a.rs\"\n\
        max_accepted = { value = 400, unit = \"lines\" }\n\
        reason = \"\"\"\n\
        not this one: max_accepted = { value = 1, unit = \"lines\" }\n\
        \"\"\"\n\
        \n\
        [[exceptions]]\n\
        path = \"b.rs\"\n\
        max_accepted = { value = 500, unit = \"lines\" }\n";

    #[test]
    fn rewrites_only_the_addressed_block() {
        let out = rewrite_ceiling(
            REGISTRY,
            1,
            600,
            Unit::Lines,
            std::path::Path::new("r"),
            "b.rs",
        )
        .unwrap();
        assert!(out.contains("max_accepted = { value = 400, unit = \"lines\" }"));
        assert!(out.contains("max_accepted = { value = 600, unit = \"lines\" }"));
        assert!(!out.contains("value = 500"));
    }

    /// A `[[exceptions]]`-shaped line inside a reason must not shift the count,
    /// and a `max_accepted` written there is prose, not a field.
    #[test]
    fn a_reason_body_is_not_scanned() {
        let out = rewrite_ceiling(
            REGISTRY,
            0,
            700,
            Unit::Lines,
            std::path::Path::new("r"),
            "a.rs",
        )
        .unwrap();
        assert!(out.contains("not this one: max_accepted = { value = 1, unit = \"lines\" }"));
        assert!(out.contains("max_accepted = { value = 700, unit = \"lines\" }"));
    }

    #[test]
    fn a_sub_table_ceiling_is_refused_not_guessed() {
        let registry = "[[exceptions]]\npath = \"a.rs\"\n[exceptions.max_accepted]\nvalue = 400\n";
        let error = rewrite_ceiling(
            registry,
            0,
            500,
            Unit::Lines,
            std::path::Path::new("r"),
            "a.rs",
        );
        assert!(error.is_err());
    }
}
