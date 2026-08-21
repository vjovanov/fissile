//! `fissile check` (§FS-004-check-audit.1): the commit-time gate. Measures a
//! file set — git-staged, caller-passed, or the configured scan scope — applies
//! the rules and exception registries, and emits findings in text or JSON.

use std::path::PathBuf;

use crate::cli::{self, CommandError, Format, Loaded};
use crate::config::Stale;
use crate::exceptions::Exception;
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

/// The result of a `check` run: the rendered output, whether it should fail
/// the build (a standing hard overflow), and the file-level errors that must
/// force exit 2 without hiding the other findings (§FS-004-check-audit.5).
pub struct Run {
    pub output: String,
    pub failed: bool,
    pub errors: Vec<String>,
}

pub fn run(options: &CheckOptions) -> Result<Run, CommandError> {
    let loaded = cli::load(&options.root, options.config_path.as_deref())?;
    let files = collect_files(options, &loaded)?;
    let format = options
        .format
        .unwrap_or_else(|| loaded.config.output.format.into());

    // A path that cannot be measured is skipped, not fatal: one odd file must not
    // hide every other finding (§FS-004-check-audit.5).
    let (measurements, errors) = cli::measure_each(&loaded, options.staged, &files);
    let mut outcomes = Vec::new();
    for measurement in &measurements {
        outcomes.extend(report::evaluate_file(
            &loaded.checker,
            &loaded.registries,
            measurement,
        )?);
    }

    // An exact-path entry whose file is gone accepts nothing, and the commit
    // that removed the file is where that is worth saying (§FS-004-check-audit.1.3).
    let dangling = match loaded.config.exceptions.stale {
        Stale::Ignore => Vec::new(),
        _ => loaded.registries.dangling(&loaded.root),
    };
    let failed = report::has_hard_failure(&outcomes)
        || (!dangling.is_empty() && loaded.config.exceptions.stale == Stale::Error);

    let output = match format {
        Format::Text => {
            let color = cli::use_color(loaded.config.output.color, options.no_color, format);
            let text = Text {
                outcomes: &outcomes,
                dangling: &dangling,
                success: &loaded.config.output.success,
                color,
                staged: options.staged,
                errors: &errors,
            };
            render_text(&text)
        }
        Format::Json => render_json(&outcomes),
    };
    Ok(Run {
        output,
        failed,
        errors,
    })
}

fn collect_files(options: &CheckOptions, loaded: &Loaded) -> Result<Vec<String>, CommandError> {
    match cli::selected_files(loaded, options.staged, &options.paths)? {
        Some(files) => Ok(files),
        // Neither `--staged` nor explicit paths: the configured scan scope.
        None => Ok(scan::walk_scope(&loaded.root, &loaded.config.scan)?),
    }
}

/// What one text render needs: the findings, what the registries no longer
/// accept, and whether this run is a commit (§FS-004-check-audit.1).
struct Text<'a> {
    outcomes: &'a [Outcome],
    dangling: &'a [&'a Exception],
    success: &'a str,
    color: bool,
    staged: bool,
    errors: &'a [String],
}

fn render_text(text: &Text<'_>) -> String {
    let mut blocks = report::finding_blocks(text.outcomes, text.color);
    let found = !blocks.is_empty();
    blocks.extend(report::dangling_blocks(text.dangling, text.color));

    if blocks.is_empty() {
        // The marker is withheld when a file could not be measured: `ok` next
        // to an exit-2 diagnostic would be a lie (§FS-004-check-audit.5).
        return if text.errors.is_empty() {
            report::success_marker(text.success, text.color)
        } else {
            String::new()
        };
    }

    // The hint answers where a split can put code, so it follows findings, not
    // a lone stale entry (§FS-004-check-audit.1.1).
    if found {
        blocks.push(report::MEASURE_HINT.to_owned());
    }
    // Only `--staged` is a commit (§FS-004-check-audit.1.2).
    if text.staged && report::has_hard_failure(text.outcomes) {
        blocks.push(report::COMMIT_GATE.to_owned());
    }

    // Blocks are separated by a blank line (§FS-004-check-audit.1).
    blocks.join("\n\n")
}

fn render_json(outcomes: &[Outcome]) -> String {
    let records: Vec<Json> = outcomes
        .iter()
        .filter(|outcome| outcome.is_reported())
        .map(report::overflow_json)
        .collect();
    Json::Array(records).render()
}
