//! `fissile check` (§FS-004-check-audit.1): the commit-time gate. Measures a
//! file set — git-staged, caller-passed, or the configured scan scope — applies
//! the rules and exception registries, and emits findings in text or JSON.

use std::path::PathBuf;

use crate::cli::{self, CommandError, Format, Loaded};
use crate::exceptions::{Exception, MatchKind};
use crate::json::Json;
use crate::report::{self, Outcome};
use crate::scan;

/// Inputs to a `check` run.
#[derive(Clone, Debug)]
pub struct CheckOptions {
    pub root: PathBuf,
    pub config_path: Option<PathBuf>,
    pub staged: bool,
    pub format: Option<Format>,
    /// Force plain text output regardless of `[output].color` (§FS-001-config.6).
    pub no_color: bool,
    /// Caller-passed repo-relative paths; empty means "use the scan scope".
    pub paths: Vec<String>,
}

/// The result of a `check` run: rendered output, whether it fails the build,
/// notes stderr owns because stdout holds a machine shape, and file-level errors
/// that force exit 2 without hiding the findings (§FS-004-check-audit.5).
pub struct Run {
    pub output: String,
    pub failed: bool,
    pub notes: Vec<String>,
    pub errors: Vec<String>,
}

/// Why a run is blocked, decided once so the exit code and the commit-gate
/// epilogue cannot answer it differently (§FS-004-check-audit.1.2).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Blocked {
    No,
    /// A standing hard overflow: the remedy is a split.
    Overflow,
    /// An exception entry that has outlived its file, under `stale = "error"`:
    /// the remedy is the registry (§FS-004-check-audit.1.3).
    DeadEntry,
}

pub fn run(options: &CheckOptions) -> Result<Run, CommandError> {
    let loaded = cli::load(&options.root, options.config_path.as_deref())?;
    let files = collect_files(options, &loaded)?;
    let format = options
        .format
        .unwrap_or_else(|| loaded.config.output.format.into());

    // A path that cannot be measured is skipped, not fatal: one odd file must not
    // hide every other finding (§FS-004-check-audit.5).
    let (measured_files, errors) = cli::measure_each_with_context(&loaded, options.staged, &files);
    let mut contexts = Vec::new();
    let mut outcomes = Vec::new();
    for measured_file in &measured_files {
        let measurement = &measured_file.measurement;
        let hits = loaded
            .checker
            .evaluate(measurement)
            .map_err(report::EvalError::from)?;
        contexts.extend(report::contexts_for_file(
            measurement,
            &hits,
            measured_file.utf8,
            &loaded.config.exceptions.bump,
        ));
        outcomes.extend(report::evaluate_hits(
            &loaded.registries,
            measurement,
            &hits,
        )?);
    }

    // An exact-path entry this run's file set proves is gone accepts nothing,
    // and the commit that removed the file is where that is worth saying
    // (§FS-004-check-audit.1.3).
    let stale = stale_entries(options, &loaded, &files)?;
    let blocked = if report::has_hard_failure(&outcomes) {
        Blocked::Overflow
    } else if !stale.is_empty() && loaded.config.exceptions.stale.fails() {
        Blocked::DeadEntry
    } else {
        Blocked::No
    };

    let (output, notes) = match format {
        Format::Text => {
            let color = cli::use_color(loaded.config.output.color, options.no_color, format);
            let text = Text {
                outcomes: &outcomes,
                contexts: &contexts,
                stale: &stale,
                success: &loaded.config.output.success,
                color,
                staged: options.staged,
                blocked,
                has_errors: !errors.is_empty(),
            };
            (render_text(&text), Vec::new())
        }
        // stdout keeps the stable findings shape, so the block about a registry
        // goes to stderr — where it is still the run's own account of why it
        // failed, rather than an unexplained exit code (§FS-004-check-audit.5).
        Format::Json => (
            render_json(&outcomes, &contexts),
            report::stale_blocks(&stale, false),
        ),
    };
    Ok(Run {
        output,
        failed: blocked != Blocked::No,
        notes,
        errors,
    })
}

/// The exact-path entries this run's own file set proves are gone
/// (§FS-004-check-audit.1.3). Absence from the working tree is not that proof:
/// an unbuilt file, or an unstaged deletion, has outlived no entry.
fn stale_entries<'a>(
    options: &CheckOptions,
    loaded: &'a Loaded,
    files: &[String],
) -> Result<Vec<&'a Exception>, CommandError> {
    if !loaded.config.exceptions.stale.reports() {
        return Ok(Vec::new());
    }
    let exact = |entry: &&'a Exception| entry.match_kind == MatchKind::Exact;
    // Only an exact-path entry can be reported here, so a registry holding none
    // has no answer to compute — and `--staged` runs on the pre-commit path,
    // where the git call below is pure cost (§FS-004-check-audit.1.3).
    if !loaded.registries.all().any(|entry| exact(&entry)) {
        return Ok(Vec::new());
    }

    if options.staged {
        // The commit is what removes the file, and it is the thing being judged.
        let removed = scan::staged_removals(&loaded.root)?;
        return Ok(loaded
            .registries
            .all()
            .filter(exact)
            .filter(|entry| removed.contains(&entry.path))
            .collect());
    }
    // Caller-passed paths are a window, not an inventory: they prove nothing
    // about an entry that names some other file.
    if !options.paths.is_empty() {
        return Ok(Vec::new());
    }
    // The whole inventory, the one §FS-004-check-audit.2 compares against — but a
    // path the scope excludes or git ignores is missing from it while sitting
    // exactly where the entry says, and calling that dead would be false.
    Ok(loaded
        .registries
        .stale(files)
        .into_iter()
        .filter(exact)
        .filter(|entry| !loaded.root.join(&entry.path).exists())
        .collect())
}

fn collect_files(options: &CheckOptions, loaded: &Loaded) -> Result<Vec<String>, CommandError> {
    match cli::selected_files(loaded, options.staged, &options.paths)? {
        Some(files) => Ok(files),
        // Neither `--staged` nor explicit paths: the configured scan scope.
        None => Ok(scan::walk_scope(&loaded.root, &loaded.config.scan)?),
    }
}

/// What one text render needs: the findings, what the registries no longer
/// accept, whether this run is a commit, and why it is blocked
/// (§FS-004-check-audit.1).
struct Text<'a> {
    outcomes: &'a [Outcome],
    contexts: &'a [report::FindingContext],
    stale: &'a [&'a Exception],
    success: &'a str,
    color: bool,
    staged: bool,
    blocked: Blocked,
    has_errors: bool,
}

fn render_text(text: &Text<'_>) -> String {
    let mut blocks = report::finding_blocks_with_context(text.outcomes, text.color, text.contexts);

    // The marker is withheld when a file could not be measured: `ok` next to an
    // exit-2 diagnostic would be a lie (§FS-004-check-audit.5). The run still
    // owes a commit its epilogue below, so this returns only on success.
    if blocks.is_empty() && text.stale.is_empty() && !text.has_errors {
        return report::success_marker(text.success, text.color);
    }

    // The hint answers where a split can put code, so it follows the findings
    // it is about and nothing else (§FS-004-check-audit.1.1).
    if !blocks.is_empty() {
        blocks.push(report::MEASURE_HINT.to_owned());
    }
    blocks.extend(report::stale_blocks(text.stale, text.color));
    // Only `--staged` is a commit, and it closes by saying what to do about
    // whatever blocked it (§FS-004-check-audit.1.2).
    if text.staged {
        match text.blocked {
            // Nothing stands against the files that were measured, but one this
            // commit stages could not be: exit 2 aborts the commit all the same,
            // and a bare diagnostic reads as advisory (§FS-004-check-audit.1.2).
            Blocked::No if text.has_errors => {
                blocks.push(report::COMMIT_GATE_UNMEASURED.to_owned());
            }
            Blocked::No => {}
            Blocked::Overflow => blocks.push(report::COMMIT_GATE.to_owned()),
            Blocked::DeadEntry => blocks.push(report::COMMIT_GATE_STALE.to_owned()),
        }
    }

    // Blocks are separated by a blank line (§FS-004-check-audit.1).
    blocks.join("\n\n")
}

fn render_json(outcomes: &[Outcome], contexts: &[report::FindingContext]) -> String {
    let records: Vec<Json> = outcomes
        .iter()
        .filter(|outcome| outcome.is_reported())
        .map(|outcome| report::overflow_json_with_context(outcome, contexts))
        .collect();
    Json::Array(records).render()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An unmeasurable staged file still names the gate that rejects the commit
    /// (§FS-004-check-audit.1.2).
    #[test]
    fn staged_measurement_error_names_the_commit_gate() {
        let text = Text {
            outcomes: &[],
            contexts: &[],
            stale: &[],
            success: "ok",
            color: false,
            staged: true,
            blocked: Blocked::No,
            has_errors: true,
        };

        assert_eq!(render_text(&text), report::COMMIT_GATE_UNMEASURED);
    }
}
