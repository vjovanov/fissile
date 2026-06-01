//! `fissile audit` (§FS-004-check-audit.2): the whole-repo inventory and
//! migration surface. Beyond current overflows it can report the largest files
//! per unit, stale exceptions, and rule coverage gaps.

use std::path::PathBuf;

use crate::cli::{self, CommandError, Format, Loaded};
use crate::config::Stale;
use crate::json::Json;
use crate::report::{self, Outcome};
use crate::{FileMeasurement, Selector, Unit, scan};

/// Inputs to an `audit` run.
#[derive(Clone, Debug, Default)]
pub struct AuditOptions {
    pub root: PathBuf,
    pub config_path: Option<PathBuf>,
    pub format: Option<Format>,
    /// Force plain text output regardless of `[output].color` (§FS-001-config.6).
    pub no_color: bool,
    pub top: Option<usize>,
    pub stale_exceptions: bool,
    pub rule_coverage: bool,
}

pub struct Run {
    pub output: String,
    pub failed: bool,
}

const UNITS: [Unit; 3] = [Unit::Bytes, Unit::Lines, Unit::Tokens];

/// The largest files per unit: one ranked `(value, path)` list per measurement
/// unit (§FS-004-check-audit.2).
type TopFiles = Vec<(Unit, Vec<(u64, String)>)>;

pub fn run(options: &AuditOptions) -> Result<Run, CommandError> {
    let loaded = cli::load(&options.root, options.config_path.as_deref())?;
    let files = scan::walk_scope(&loaded.root, &loaded.config.scan)?;
    let format = options
        .format
        .unwrap_or_else(|| loaded.config.output.format.into());

    let mut measurements = Vec::with_capacity(files.len());
    for rel in &files {
        measurements.push(scan::measure_file(
            &loaded.root,
            rel,
            &loaded.config.tokens,
        )?);
    }

    let mut outcomes = Vec::new();
    for measurement in &measurements {
        outcomes.extend(report::evaluate_file(
            &loaded.checker,
            &loaded.registries,
            measurement,
        )?);
    }

    let mut failed = report::has_hard_failure(&outcomes);

    let stale = options.stale_exceptions.then(|| {
        let entries: Vec<(String, String)> = loaded
            .registries
            .stale(&files)
            .iter()
            .map(|entry| (entry.id.clone(), entry.path.clone()))
            .collect();
        if !entries.is_empty() && loaded.config.exceptions.stale == Stale::Error {
            failed = true;
        }
        entries
    });
    // `ignore` suppresses the report entirely (§FS-003-exceptions.4).
    let stale = stale.filter(|_| loaded.config.exceptions.stale != Stale::Ignore);

    let top = options.top.map(|n| top_files(&measurements, n));
    let coverage = options
        .rule_coverage
        .then(|| coverage(&loaded, &measurements));

    let output = match format {
        Format::Text => {
            let color = cli::use_color(loaded.config.output.color, options.no_color, format);
            render_text(
                &loaded,
                &outcomes,
                top.as_ref(),
                stale.as_ref(),
                coverage.as_ref(),
                color,
            )
        }
        Format::Json => render_json(&outcomes, top.as_ref(), stale.as_ref(), coverage.as_ref()),
    };
    Ok(Run { output, failed })
}

fn unit_value(measurement: &FileMeasurement, unit: Unit) -> Option<u64> {
    match unit {
        Unit::Bytes => Some(measurement.bytes),
        Unit::Lines => measurement.lines.map(|stats| stats.total),
        Unit::Tokens => measurement.tokens,
    }
}

/// The largest `n` measured files per unit (§FS-004-check-audit.2).
fn top_files(measurements: &[FileMeasurement], n: usize) -> TopFiles {
    UNITS
        .iter()
        .filter_map(|&unit| {
            let mut ranked: Vec<(u64, String)> = measurements
                .iter()
                .filter_map(|measurement| {
                    unit_value(measurement, unit)
                        .map(|value| (value, measurement.path.to_string_lossy().replace('\\', "/")))
                })
                .collect();
            ranked.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
            ranked.truncate(n);
            (!ranked.is_empty()).then_some((unit, ranked))
        })
        .collect()
}

struct Coverage {
    unmatched_rules: Vec<String>,
    catch_all_only: Vec<String>,
    unused_messages: Vec<String>,
}

fn is_catch_all(selector: &Selector) -> bool {
    match selector {
        Selector::All => true,
        Selector::Glob(globs) => globs
            .iter()
            .all(|glob| matches!(glob.pattern(), "**/*" | "**")),
        _ => false,
    }
}

/// Rules matching no file, files reachable only through catch-all rules, and
/// messages no rule uses (§FS-004-check-audit.2).
fn coverage(loaded: &Loaded, measurements: &[FileMeasurement]) -> Coverage {
    let rules = loaded.checker.rules();

    let unmatched_rules = rules
        .iter()
        .filter(|rule| {
            !measurements
                .iter()
                .any(|measurement| rule.selector.matches(&measurement.path))
        })
        .map(|rule| rule.id.clone())
        .collect();

    let catch_all_only = measurements
        .iter()
        .filter(|measurement| {
            let matching: Vec<&_> = rules
                .iter()
                .filter(|rule| rule.selector.matches(&measurement.path))
                .collect();
            !matching.is_empty() && matching.iter().all(|rule| is_catch_all(&rule.selector))
        })
        .map(|measurement| measurement.path.to_string_lossy().replace('\\', "/"))
        .collect();

    let unused_messages = loaded
        .config
        .messages
        .iter()
        .filter(|message| {
            !loaded
                .config
                .rules
                .iter()
                .any(|rule| rule.message == message.id)
        })
        .map(|message| message.id.clone())
        .collect();

    Coverage {
        unmatched_rules,
        catch_all_only,
        unused_messages,
    }
}

fn render_text(
    loaded: &Loaded,
    outcomes: &[Outcome],
    top: Option<&TopFiles>,
    stale: Option<&Vec<(String, String)>>,
    coverage: Option<&Coverage>,
    color: bool,
) -> String {
    let mut sections = Vec::new();

    let reported: Vec<String> = outcomes
        .iter()
        .filter(|outcome| outcome.is_reported())
        .map(|outcome| report::finding_block(outcome.overflow(), color))
        .collect();
    sections.push(if reported.is_empty() {
        report::success_marker(&loaded.config.output.success, color)
    } else {
        reported.join("\n")
    });

    let silenced: Vec<String> = outcomes
        .iter()
        .filter_map(|outcome| match outcome {
            Outcome::Silenced {
                overflow,
                exception_id,
                exception_max,
            } => Some(report::silenced_line(
                overflow,
                exception_id,
                *exception_max,
            )),
            Outcome::Reported(_) => None,
        })
        .collect();
    if !silenced.is_empty() {
        sections.push(silenced.join("\n"));
    }

    if let Some(top) = top {
        for (unit, ranked) in top {
            let mut lines = vec![format!("top {unit}:")];
            for (value, path) in ranked {
                lines.push(format!("  {value} {path}"));
            }
            sections.push(lines.join("\n"));
        }
    }

    if let Some(stale) = stale {
        let mut lines = vec!["stale exceptions:".to_owned()];
        if stale.is_empty() {
            lines.push("  none".to_owned());
        }
        for (id, path) in stale {
            lines.push(format!("  {id} ({path})"));
        }
        sections.push(lines.join("\n"));
    }

    if let Some(coverage) = coverage {
        sections.push(render_coverage_text(coverage));
    }

    sections.join("\n\n")
}

fn render_coverage_text(coverage: &Coverage) -> String {
    let mut lines = vec!["rule coverage:".to_owned()];
    lines.push(format!(
        "  rules matching no file: {}",
        join_or_none(&coverage.unmatched_rules)
    ));
    lines.push(format!(
        "  files only under catch-all: {}",
        join_or_none(&coverage.catch_all_only)
    ));
    lines.push(format!(
        "  unused messages: {}",
        join_or_none(&coverage.unused_messages)
    ));
    lines.join("\n")
}

fn join_or_none(items: &[String]) -> String {
    if items.is_empty() {
        "none".to_owned()
    } else {
        items.join(", ")
    }
}

fn render_json(
    outcomes: &[Outcome],
    top: Option<&TopFiles>,
    stale: Option<&Vec<(String, String)>>,
    coverage: Option<&Coverage>,
) -> String {
    let findings: Vec<Json> = outcomes
        .iter()
        .filter(|outcome| outcome.is_reported())
        .map(report::overflow_json)
        .collect();
    let silenced: Vec<Json> = outcomes
        .iter()
        .filter(|outcome| !outcome.is_reported())
        .map(report::overflow_json)
        .collect();

    let mut fields = vec![
        ("findings", Json::Array(findings)),
        ("silenced", Json::Array(silenced)),
    ];

    if let Some(top) = top {
        let groups: Vec<Json> = top
            .iter()
            .map(|(unit, ranked)| {
                let entries = ranked
                    .iter()
                    .map(|(value, path)| {
                        Json::Object(vec![
                            ("value", Json::UInt(*value)),
                            ("path", Json::str(path.clone())),
                        ])
                    })
                    .collect();
                Json::Object(vec![
                    ("unit", Json::str(unit.to_string())),
                    ("files", Json::Array(entries)),
                ])
            })
            .collect();
        fields.push(("top", Json::Array(groups)));
    }

    if let Some(stale) = stale {
        let entries = stale
            .iter()
            .map(|(id, path)| {
                Json::Object(vec![
                    ("id", Json::str(id.clone())),
                    ("path", Json::str(path.clone())),
                ])
            })
            .collect();
        fields.push(("stale", Json::Array(entries)));
    }

    if let Some(coverage) = coverage {
        fields.push((
            "coverage",
            Json::Object(vec![
                ("unmatched_rules", str_array(&coverage.unmatched_rules)),
                ("catch_all_only", str_array(&coverage.catch_all_only)),
                ("unused_messages", str_array(&coverage.unused_messages)),
            ]),
        ));
    }

    Json::Object(fields).render()
}

fn str_array(items: &[String]) -> Json {
    Json::Array(items.iter().map(|item| Json::str(item.clone())).collect())
}
