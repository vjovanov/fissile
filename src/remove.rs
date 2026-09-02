//! `fissile exception remove` (§FS-009-exception-remove): delete one exception
//! entry. It addresses the entry exactly as `retune` does, refuses to delete one
//! that is still silencing a finding, and — alone among the commands — loads a
//! registry the rule check rejects, because repairing that state is what it is
//! for (§FS-009-exception-remove.2).

use std::fs;
use std::path::{Path, PathBuf};

use crate::cli::{self, CommandError, Loaded};
use crate::entry::{self, Address};
use crate::exceptions::{Exception, MatchKind, Registries, RegistrySource};
use crate::report::{self, EvalError};
use crate::toml_lines;
use crate::{FileMeasurement, RuleHit, Severity, Unit, scan};

/// Inputs to `exception remove`. No `--max` and no `--unit`: the command states
/// no ceiling (§FS-009-exception-remove.1).
#[derive(Clone, Debug)]
pub struct RemoveOptions {
    pub root: PathBuf,
    pub config_path: Option<PathBuf>,
    pub path: String,
    pub severity: Severity,
    pub rules: Vec<String>,
    pub match_kind: MatchKind,
    pub dry_run: bool,
}

#[derive(Clone, Debug)]
pub struct Run {
    pub output: String,
}

pub fn run(options: &RemoveOptions) -> Result<Run, CommandError> {
    // The registry this command repairs is one the rule check aborts on, so it
    // is the one command that loads without it (§FS-009-exception-remove.2).
    let loaded = cli::load_unvalidated(&options.root, options.config_path.as_deref())?;
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
    // An address with no entry behind it has nothing to delete, and the audit is
    // what lists the entries that are there (§FS-009-exception-remove.1).
    let Some((index, existing)) = entry::locate(&loaded.registries, &address)? else {
        return Err(CommandError::Usage(format!(
            "{}: no entry accepts {path} for this rule and unit, so there is nothing to \
             remove; `fissile audit --stale-exceptions` lists the entries that are there",
            registry_rel.display()
        )));
    };
    let existing = existing.clone();
    check_matcher(&existing, options.match_kind, &path, &registry_rel)?;

    // What the registry holds once the entry is gone. The refusal below reads it,
    // and the write is held to producing exactly this (§FS-009-exception-remove.5).
    let after = without(&loaded.registries, options.severity, index);
    check_silenced(&loaded, &after, &existing, &registry_rel)?;

    let registry_path = loaded.root.join(&registry_rel);
    let text = cli::read_optional(&registry_path)?
        .ok_or_else(|| CommandError::Usage(format!("{} is missing", registry_rel.display())))?;
    let new_text = delete_block(&text, index, &registry_rel, &existing.path)?;
    check_written(&loaded, options.severity, &after, &new_text, &registry_rel)?;

    let note = twin_note(twin(&loaded, options, &path, unit), unit);
    let head = format!(
        "{}: {} {} (accepted up to {} {unit})",
        registry_rel.display(),
        if options.dry_run {
            "would remove"
        } else {
            "removed"
        },
        existing.path,
        existing.max_value
    );
    if options.dry_run {
        return Ok(Run {
            output: with_note(
                format!("{head}\nwould update {}", registry_rel.display()),
                note,
            ),
        });
    }

    fs::write(&registry_path, &new_text)?;
    Ok(Run {
        output: with_note(head, note),
    })
}

/// An address is a matcher, not just a path (§DF-005-exception-identity), so a
/// matcher that only *overlaps* the entry is the wrong address — and for
/// removal each direction is wrong for its own reason
/// (§FS-009-exception-remove.1).
fn check_matcher(
    existing: &Exception,
    addressed: MatchKind,
    path: &str,
    registry_rel: &Path,
) -> Result<(), CommandError> {
    match (existing.match_kind, addressed) {
        // Deleting the class-wide entry from under one member drops the ceiling
        // for every other file the glob names, which the caller did not ask for.
        (MatchKind::Glob, MatchKind::Exact) => Err(CommandError::Usage(format!(
            "{}: {path} is covered by the glob entry {}, whose ceiling covers every file \
             that glob names; remove the class as `--match glob \"{}\"` if that is what \
             should go",
            registry_rel.display(),
            existing.path,
            existing.path
        ))),
        // The reverse deletes one file's entry under a spelling no entry carries,
        // and reports the change against a path the registry never held.
        (MatchKind::Exact, MatchKind::Glob) => Err(CommandError::Usage(format!(
            "{}: no glob entry accepts {path}; it spans the exact entry {}, which is \
             removed as `--match exact {}`",
            registry_rel.display(),
            existing.path,
            existing.path
        ))),
        _ => Ok(()),
    }
}

/// The registries as they stand with the addressed entry gone. Every check
/// §FS-003-exceptions.4 applies is per entry, so this is the whole effect of the
/// removal on the document's validity (§FS-009-exception-remove.2).
fn without(registries: &Registries, severity: Severity, index: usize) -> Registries {
    let mut after = registries.clone();
    match severity {
        Severity::Soft => {
            after.soft.remove(index);
        }
        Severity::Hard => {
            after.hard.remove(index);
        }
    }
    after
}

/// One finding, by what identifies it: the same finding standing before and
/// after the removal is the same tuple, and one that appears only after it is
/// what the removal would surface (§FS-009-exception-remove.3).
#[derive(Clone, Debug, PartialEq, Eq)]
struct Finding {
    path: String,
    rule_id: String,
    severity: Severity,
    unit: Unit,
    actual: u64,
    limit: u64,
}

/// Refuse a removal that would report a file the repository decided to accept
/// (§FS-009-exception-remove.3). Only the files the entry's own matcher covers
/// can change verdict, so those are the only ones measured.
fn check_silenced(
    loaded: &Loaded,
    after: &Registries,
    existing: &Exception,
    registry_rel: &Path,
) -> Result<(), CommandError> {
    let mut surfaced: Vec<Finding> = Vec::new();
    for file in scan::walk_scope(&loaded.root, &loaded.config.scan)? {
        if !existing.matches_path(&file) {
            continue;
        }
        // A path that cannot be measured reports nothing under either registry,
        // so it silences nothing either (§FS-004-check-audit.5).
        let Ok(measurement) = scan::measure_file(&loaded.root, &file, &loaded.config.tokens) else {
            continue;
        };
        let hits = loaded
            .checker
            .evaluate(&measurement)
            .map_err(EvalError::from)?;
        let standing = reported(&loaded.registries, &measurement, &hits)?;
        for finding in reported(after, &measurement, &hits)? {
            if !standing.contains(&finding) {
                surfaced.push(finding);
            }
        }
    }

    let Some(first) = surfaced.first() else {
        return Ok(());
    };
    let more = match surfaced.len() {
        1 => String::new(),
        count => format!(", and {} more", count - 1),
    };
    Err(CommandError::Usage(format!(
        "{}: {} still silences a finding: {} measures {} {}, over rule {} {} limit {}{more}. \
         Removing the entry would report the file rather than record it — split the file \
         first, or leave the entry where it is",
        registry_rel.display(),
        existing.path,
        first.path,
        first.actual,
        first.unit,
        first.rule_id,
        first.severity,
        first.limit,
    )))
}

/// The standing findings for one measured file under a given registry state.
/// The rule hits are passed in because both states read the same ones, and
/// evaluating a file twice is waste (§GOAL-001-fast-feedback).
fn reported(
    registries: &Registries,
    measurement: &FileMeasurement,
    hits: &[RuleHit<'_>],
) -> Result<Vec<Finding>, CommandError> {
    Ok(report::evaluate_hits(registries, measurement, hits)?
        .iter()
        .filter(|outcome| outcome.is_reported())
        .map(|outcome| {
            let overflow = outcome.overflow();
            Finding {
                path: overflow.path.to_string_lossy().replace('\\', "/"),
                rule_id: overflow.rule_id.clone(),
                severity: overflow.severity,
                unit: overflow.unit,
                actual: overflow.actual,
                limit: overflow.limit,
            }
        })
        .collect())
}

/// Delete the `index`-th `[[exceptions]]` block, preserving every other byte
/// (§FS-009-exception-remove.4).
fn delete_block(
    text: &str,
    index: usize,
    registry: &Path,
    path: &str,
) -> Result<String, CommandError> {
    // Split rather than `lines()`: this round-trips the bytes that stay, the
    // final newline and any `\r` among them.
    let mut lines: Vec<String> = text.split('\n').map(str::to_owned).collect();
    let starts = toml_lines::block_starts(&lines);
    // Reached only by a registry whose blocks the parser and the line reader
    // count differently. Refusing beats cutting lines out of the wrong entry.
    let Some(&start) = starts.get(index) else {
        return Err(CommandError::Usage(format!(
            "{}: cannot find the entry for {path} in the document; edit it by hand",
            registry.display()
        )));
    };
    match starts.get(index + 1) {
        // The blank line that separated the two goes with the block, so the
        // entries that remain keep the spacing they had.
        Some(&next) => {
            lines.drain(start..next);
        }
        // Nothing follows, so the document ends where this block began — with
        // one trailing newline, not the blank lines that led into the entry.
        None => {
            lines.truncate(start);
            while lines.last().is_some_and(|line| line.trim().is_empty()) {
                lines.pop();
            }
            lines.push(String::new());
        }
    }
    Ok(lines.join("\n"))
}

/// The document about to be written must hold exactly the entries that were
/// read, less the one addressed (§FS-009-exception-remove.5). Re-validating it
/// the way `add` and `retune` do would be the wrong guard here: the registry
/// `remove` repairs is one validation already rejects.
fn check_written(
    loaded: &Loaded,
    severity: Severity,
    after: &Registries,
    new_text: &str,
    registry_rel: &Path,
) -> Result<(), CommandError> {
    let configured = match severity {
        Severity::Soft => &loaded.config.exceptions.soft_registry,
        Severity::Hard => &loaded.config.exceptions.hard_registry,
    };
    let source = RegistrySource::new(configured, new_text);
    let (written, expected) = match severity {
        Severity::Soft => (Registries::load(Some(source), None)?.soft, &after.soft),
        Severity::Hard => (Registries::load(None, Some(source))?.hard, &after.hard),
    };
    if &written != expected {
        return Err(CommandError::Usage(format!(
            "{}: removing that entry would change the ones that remain; edit the file by hand",
            registry_rel.display()
        )));
    }
    Ok(())
}

fn with_note(head: String, note: Option<String>) -> String {
    match note {
        Some(note) => format!("{head}\n{note}"),
        None => head,
    }
}

/// The same address in the registry the caller did not select.
fn twin<'a>(
    loaded: &'a Loaded,
    options: &RemoveOptions,
    path: &str,
    unit: Unit,
) -> Option<&'a Exception> {
    let address = Address {
        severity: options.severity.other(),
        path,
        match_kind: options.match_kind,
        rules: &options.rules,
        unit,
    };
    let (_, twin) = entry::locate(&loaded.registries, &address).ok().flatten()?;
    Some(twin)
}

/// `remove` never writes to the twin's registry — twin consistency is a
/// repository's policy, not the tool's — but a caller who has just deleted one
/// half should learn the other half is still accepting the file
/// (§FS-009-exception-remove.4).
fn twin_note(twin: Option<&Exception>, unit: Unit) -> Option<String> {
    let twin = twin?;
    Some(format!(
        "note: {} still accepts {} up to {} {unit}",
        twin.registry, twin.path, twin.max_value
    ))
}

#[cfg(test)]
#[path = "remove_tests.rs"]
mod tests;
