//! `fissile audit` (§FS-004-check-audit.2): the whole-repo inventory and
//! migration surface. Beyond current overflows it can report the largest files
//! per unit, stale exceptions, and rule coverage gaps.

use std::path::PathBuf;

use crate::cli::{self, CommandError, Format, Loaded};
use crate::config::Stale;
use crate::exceptions::{EntrySite, Exception, KindCounts};
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
    /// File-level measurement errors; force exit 2 (§FS-004-check-audit.5).
    pub errors: Vec<String>,
}

const UNITS: [Unit; 3] = [Unit::Bytes, Unit::Lines, Unit::Tokens];

/// The largest files per unit: one ranked `(value, path)` list per measurement
/// unit (§FS-004-check-audit.2).
type TopFiles = Vec<(Unit, Vec<(u64, String)>)>;

/// What `audit` reports beyond the standing findings (§FS-004-check-audit.2).
/// A `None` section was not requested; `kinds` is default-on and carries its own
/// emptiness.
struct Inventory {
    top: Option<TopFiles>,
    stale: Option<Vec<EntrySite>>,
    coverage: Option<Coverage>,
    kinds: KindCounts,
}

pub fn run(options: &AuditOptions) -> Result<Run, CommandError> {
    let loaded = cli::load(&options.root, options.config_path.as_deref())?;
    let files = scan::walk_scope(&loaded.root, &loaded.config.scan)?;
    let format = options
        .format
        .unwrap_or_else(|| loaded.config.output.format.into());

    let mut measurements = Vec::with_capacity(files.len());
    let mut errors = Vec::new();
    for rel in &files {
        // Skip what cannot be measured; the walk goes on (§FS-004-check-audit.5).
        match scan::measure_file(&loaded.root, rel, &loaded.config.tokens) {
            Ok(measurement) => measurements.push(measurement),
            Err(error) => errors.push(scan::measure_error_line(rel, &error)),
        }
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
        // Registry plus the entry's own `path`: the list spans both registries,
        // and the same path can be stale in each (§FS-003-exceptions.4).
        let entries: Vec<EntrySite> = loaded
            .registries
            .stale(&files)
            .iter()
            .map(|entry| Exception::site(entry))
            .collect();
        if !entries.is_empty() && loaded.config.exceptions.stale == Stale::Error {
            failed = true;
        }
        entries
    });
    // `ignore` suppresses the report entirely (§FS-003-exceptions.4).
    let stale = stale.filter(|_| loaded.config.exceptions.stale != Stale::Ignore);

    let inventory = Inventory {
        top: options.top.map(|n| top_files(&measurements, n)),
        stale,
        coverage: options
            .rule_coverage
            .then(|| coverage(&loaded, &measurements)),
        // Default-on, no flag: the two numbers are what an inventory is for
        // (§FS-004-check-audit.2).
        kinds: loaded.registries.kind_counts(),
    };

    let output = match format {
        Format::Text => {
            let color = cli::use_color(loaded.config.output.color, options.no_color, format);
            render_text(&loaded, &outcomes, &inventory, color, &errors)
        }
        Format::Json => render_json(&outcomes, &inventory),
    };
    Ok(Run {
        output,
        failed,
        errors,
    })
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
                .flat_map(|rule| rule.message_ids())
                .any(|id| id == message.id)
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
    inventory: &Inventory,
    color: bool,
    errors: &[String],
) -> String {
    let mut sections = Vec::new();

    // Each grouped block is its own section, so the blank-line separation is the
    // same between blocks as between audit sections (§FS-004-check-audit.1).
    let reported = report::finding_blocks(outcomes, color);
    if reported.is_empty() {
        // Withheld when a file could not be measured (§FS-004-check-audit.5).
        if errors.is_empty() {
            sections.push(report::success_marker(&loaded.config.output.success, color));
        }
    } else {
        sections.extend(reported);
    }

    let silenced: Vec<String> = outcomes
        .iter()
        .filter_map(|outcome| match outcome {
            Outcome::Silenced {
                overflow,
                exception_max,
            } => Some(report::silenced_line(overflow, *exception_max)),
            Outcome::Reported(_) => None,
        })
        .collect();
    if !silenced.is_empty() {
        sections.push(silenced.join("\n"));
    }

    // Omitted when there is nothing to inventory: a repository with no
    // exceptions pays no lines for the section (§FS-004-check-audit.2).
    let kinds = inventory.kinds;
    if !kinds.is_empty() {
        sections.push(format!(
            "exceptions:\n  structural (never expires): {}\n  deferred (carrying debt): {}",
            kinds.structural, kinds.deferred
        ));
    }

    if let Some(top) = &inventory.top {
        for (unit, ranked) in top {
            let mut lines = vec![format!("top {unit}:")];
            for (value, path) in ranked {
                lines.push(format!("  {value} {path}"));
            }
            sections.push(lines.join("\n"));
        }
    }

    if let Some(stale) = &inventory.stale {
        let mut lines = vec!["stale exceptions:".to_owned()];
        if stale.is_empty() {
            lines.push("  none".to_owned());
        }
        for site in stale {
            lines.push(format!("  {site}"));
        }
        sections.push(lines.join("\n"));
    }

    if let Some(coverage) = &inventory.coverage {
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

fn render_json(outcomes: &[Outcome], inventory: &Inventory) -> String {
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

    // Unconditional, unlike the text section: a consumer should not have to tell
    // "no exceptions" from "this build does not report them"
    // (§FS-004-check-audit.2).
    let mut fields = vec![
        ("findings", Json::Array(findings)),
        ("silenced", Json::Array(silenced)),
        (
            "exceptions",
            Json::Object(vec![
                ("structural", Json::UInt(inventory.kinds.structural as u64)),
                ("deferred", Json::UInt(inventory.kinds.deferred as u64)),
            ]),
        ),
    ];

    if let Some(top) = &inventory.top {
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

    if let Some(stale) = &inventory.stale {
        let entries = stale
            .iter()
            .map(|site| {
                Json::Object(vec![
                    ("registry", Json::str(site.registry.clone())),
                    ("path", Json::str(site.path.clone())),
                ])
            })
            .collect();
        fields.push(("stale", Json::Array(entries)));
    }

    if let Some(coverage) = &inventory.coverage {
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
