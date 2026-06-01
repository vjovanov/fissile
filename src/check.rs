//! `fissile check` (§FS-004-check-audit.1): the commit-time gate. Measures a
//! file set — git-staged, caller-passed, or the configured scan scope — applies
//! the rules and exception registries, and emits findings in text or JSON.

use std::path::PathBuf;

use crate::cli::{self, CommandError, Format, Loaded};
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

/// The result of a `check` run: the rendered output and whether it should fail
/// the build (a standing hard overflow).
pub struct Run {
    pub output: String,
    pub failed: bool,
}

pub fn run(options: &CheckOptions) -> Result<Run, CommandError> {
    let loaded = cli::load(&options.root, options.config_path.as_deref())?;
    let files = collect_files(options, &loaded)?;
    let format = options
        .format
        .unwrap_or_else(|| loaded.config.output.format.into());

    let mut outcomes = Vec::new();
    for rel in files {
        let measurement = if options.staged {
            scan::measure_staged_file(&loaded.root, &rel, &loaded.config.tokens)?
        } else {
            scan::measure_file(&loaded.root, &rel, &loaded.config.tokens)?
        };
        outcomes.extend(report::evaluate_file(
            &loaded.checker,
            &loaded.registries,
            &measurement,
        )?);
    }

    let failed = report::has_hard_failure(&outcomes);
    let output = match format {
        Format::Text => {
            let color = cli::use_color(loaded.config.output.color, options.no_color, format);
            render_text(&outcomes, &loaded.config.output.success, color)
        }
        Format::Json => render_json(&outcomes),
    };
    Ok(Run { output, failed })
}

fn collect_files(options: &CheckOptions, loaded: &Loaded) -> Result<Vec<String>, CommandError> {
    if options.staged {
        return Ok(scan::staged_files(&loaded.root, &loaded.config.scan)?);
    }
    if !options.paths.is_empty() {
        // Caller-passed paths bypass scope/exclusion filtering, but still use
        // the repo-relative spelling consumed by rules and exceptions.
        return options
            .paths
            .iter()
            .map(|path| scan::normalize_repo_path(&loaded.root, path))
            .collect::<Result<Vec<_>, _>>()
            .map_err(CommandError::Io);
    }
    Ok(scan::walk_scope(&loaded.root, &loaded.config.scan)?)
}

fn render_text(outcomes: &[Outcome], success: &str, color: bool) -> String {
    let blocks: Vec<String> = outcomes
        .iter()
        .filter(|outcome| outcome.is_reported())
        .map(|outcome| report::finding_block(outcome.overflow(), color))
        .collect();
    if blocks.is_empty() {
        report::success_marker(success, color)
    } else {
        blocks.join("\n")
    }
}

fn render_json(outcomes: &[Outcome]) -> String {
    let records: Vec<Json> = outcomes
        .iter()
        .filter(|outcome| outcome.is_reported())
        .map(report::overflow_json)
        .collect();
    Json::Array(records).render()
}
