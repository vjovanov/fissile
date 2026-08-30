//! `fissile audit` (§FS-004-check-audit.2): the whole-repo inventory and
//! migration surface. Beyond current overflows it can report the largest files
//! per unit, stale exceptions, and rule coverage gaps.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::cli::{self, CommandError, Format, Loaded};
use crate::entry;
use crate::exceptions::{EntrySite, Exception, KindCounts, MatchKind};
use crate::json::Json;
use crate::report::{self, EvalError, Outcome};
use crate::{FileMeasurement, RuleHit, Selector, Severity, Unit, scan};

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
    /// Ceilings standing more than one bump step above the file they accept
    /// (§FS-003-exceptions.7). Requested by the same flag as `stale`: both answer
    /// "which entries no longer say something true?".
    loose: Option<Vec<Loose>>,
    coverage: Option<Coverage>,
    kinds: KindCounts,
}

/// One entry accepting far more of a file that is still there. Stale is the
/// other half: an entry accepting a file that is gone (§FS-003-exceptions.7).
struct Loose {
    site: EntrySite,
    severity: Severity,
    unit: Unit,
    accepted: u64,
    actual: u64,
    limit: u64,
    /// What to do about the slack, which is one thing or the other and never
    /// neither (§DF-010-stated-ceilings-are-exact.2).
    remedy: Remedy,
    /// The file no longer crosses that limit at all, so the entry silences
    /// nothing and lowering it is not the remedy — removing it is.
    silences_nothing: bool,
}

/// What closes the gap between an entry's ceiling and the file under it.
#[derive(Clone, Copy, Debug)]
enum Remedy {
    /// The ceiling `exception retune` would write from the measurement, never
    /// below the limit the entry exists to accept.
    RetuneTo(u64),
    /// The step lands a soft ceiling on the hard limit, which `retune` refuses
    /// (§DF-010-stated-ceilings-are-exact.2), so the remedy is the stated form:
    /// the file's size up to, excluding, that limit.
    StateWithin { floor: u64, hard: u64 },
}

pub fn run(options: &AuditOptions) -> Result<Run, CommandError> {
    let loaded = cli::load(&options.root, options.config_path.as_deref())?;
    let files = scan::walk_scope(&loaded.root, &loaded.config.scan)?;
    let format = options
        .format
        .unwrap_or_else(|| loaded.config.output.format.into());

    let mut measured_files = Vec::with_capacity(files.len());
    let mut errors = Vec::new();
    for rel in &files {
        // Skip what cannot be measured; the walk goes on (§FS-004-check-audit.5).
        match scan::measure_file_with_context(&loaded.root, rel, &loaded.config.tokens) {
            Ok(measured_file) => measured_files.push(measured_file),
            Err(error) => errors.push(scan::measure_error_line(rel, &error)),
        }
    }

    // The hits are read three times — findings, `--top`, loose ceilings — so the
    // walk evaluates each file once and the sections share the result.
    let mut hits = Vec::with_capacity(measured_files.len());
    let mut contexts = Vec::new();
    for measured_file in &measured_files {
        let measurement = &measured_file.measurement;
        let file_hits = loaded
            .checker
            .evaluate(measurement)
            .map_err(EvalError::from)?;
        contexts.extend(report::contexts_for_file(
            measurement,
            &file_hits,
            measured_file.utf8,
        ));
        hits.push(file_hits);
    }

    let mut outcomes = Vec::new();
    for (measured_file, hits) in measured_files.iter().zip(&hits) {
        outcomes.extend(report::evaluate_hits(
            &loaded.registries,
            &measured_file.measurement,
            hits,
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
        if !entries.is_empty() && loaded.config.exceptions.stale.fails() {
            failed = true;
        }
        entries
    });
    // `ignore` suppresses the report entirely (§FS-003-exceptions.4).
    let stale = stale.filter(|_| loaded.config.exceptions.stale.reports());

    // Path and hits paired once, for the two sections that read them by file.
    let files: Vec<Measured<'_>> = measured_files
        .iter()
        .zip(&hits)
        .map(|(measurement, hits)| Measured {
            path: repo_path(&measurement.measurement),
            measurement: &measurement.measurement,
            hits,
        })
        .collect();

    let inventory = Inventory {
        top: options.top.map(|n| top_files(&files, n)),
        stale,
        // Unfiltered by `[exceptions].stale`, which governs stale entries only: a
        // loose ceiling accepts everything it accepted yesterday and breaks
        // nothing today (§FS-003-exceptions.7).
        loose: options
            .stale_exceptions
            .then(|| loose_entries(&loaded, &files)),
        coverage: options
            .rule_coverage
            .then(|| coverage(&loaded, &measured_files)),
        // Default-on, no flag: the two numbers are what an inventory is for
        // (§FS-004-check-audit.2).
        kinds: loaded.registries.kind_counts(),
    };

    let output = match format {
        Format::Text => {
            let color = cli::use_color(loaded.config.output.color, options.no_color, format);
            render_text(&loaded, &outcomes, &contexts, &inventory, color, &errors)
        }
        Format::Json => render_json(&outcomes, &inventory),
    };
    Ok(Run {
        output,
        failed,
        errors,
    })
}

/// The largest `n` measured files per unit (§FS-004-check-audit.2). A rule's own
/// value where one measures the file, so `--top` and a finding never disagree;
/// the default policy elsewhere, so an unruled file still ranks.
fn top_files(files: &[Measured<'_>], n: usize) -> TopFiles {
    UNITS
        .iter()
        .filter_map(|&unit| {
            let mut ranked: Vec<(u64, String)> = files
                .iter()
                .filter_map(|file| Some((file.value(unit)?, file.path.clone())))
                .collect();
            ranked.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
            ranked.truncate(n);
            (!ranked.is_empty()).then_some((unit, ranked))
        })
        .collect()
}

/// One scanned file with its repo-relative path and its rule hits, paired once
/// so the sections that read them do not each re-derive the path or re-scan the
/// list (§GOAL-001-fast-feedback).
struct Measured<'a> {
    path: String,
    measurement: &'a FileMeasurement,
    hits: &'a [RuleHit<'a>],
}

impl Measured<'_> {
    /// What this file counts in `unit`: the effective rule's value where one
    /// measures it there, and fissile's default arithmetic otherwise
    /// (§FS-001-config.3.1).
    fn value(&self, unit: Unit) -> Option<u64> {
        if let Some(hit) = self.hits.iter().find(|hit| hit.rule.budget.unit == unit) {
            return Some(hit.actual);
        }
        match unit {
            Unit::Bytes => Some(self.measurement.bytes),
            Unit::Lines => self
                .measurement
                .lines
                .map(|stats| stats.counted(false, true)),
            Unit::Tokens => self.measurement.tokens,
        }
    }
}

fn repo_path(measurement: &FileMeasurement) -> String {
    measurement.path.to_string_lossy().replace('\\', "/")
}

/// Entries whose ceiling has drifted more than one bump step above their file
/// (§FS-003-exceptions.7). Exact paths only: a glob's ceiling is a policy for a
/// class of files, so lowering it to today's largest member breaks the next one.
fn loose_entries(loaded: &Loaded, files: &[Measured<'_>]) -> Vec<Loose> {
    // Indexed once: a linear scan per entry is a full pass over the repository
    // for every exception, which is the shape §GOAL-001-fast-feedback rules out.
    let by_path: HashMap<&str, &Measured<'_>> = files
        .iter()
        .map(|file| (file.path.as_str(), file))
        .collect();
    let mut loose = Vec::new();
    for exception in loaded.registries.all() {
        if exception.match_kind != MatchKind::Exact {
            continue;
        }
        let Some(file) = by_path.get(exception.path.as_str()) else {
            continue;
        };
        // The measurement to compare is the one this entry can silence: the
        // effective rule for its unit, among the rules it lists.
        let Some(hit) = file.hits.iter().find(|hit| {
            hit.rule.budget.unit == exception.max_unit && exception.applies_to_rule(&hit.rule.id)
        }) else {
            continue;
        };

        let step = loaded.config.exceptions.bump.step(exception.max_unit);
        if exception.max_value.saturating_sub(hit.actual) <= step {
            continue;
        }
        let limit = match exception.severity {
            Severity::Soft => hit.rule.budget.soft,
            Severity::Hard => hit.rule.budget.hard,
        }
        .unwrap_or(0);
        let quantized = entry::quantize(hit.actual, step).max(limit);
        // The value `retune` would write from a measurement — unless it is one
        // `retune` refuses (§DF-010-stated-ceilings-are-exact.2).
        let refused = hit.rule.budget.hard.filter(|hard| {
            exception.severity == Severity::Soft
                && hit.actual < *hard
                && quantized >= *hard
                && !entry::has_deferred_hard_twin(
                    &loaded.registries,
                    &exception.path,
                    exception.match_kind,
                    std::slice::from_ref(&hit.rule.id),
                    exception.max_unit,
                )
        });
        loose.push(Loose {
            site: exception.site(),
            severity: exception.severity,
            unit: exception.max_unit,
            accepted: exception.max_value,
            actual: hit.actual,
            limit,
            remedy: match refused {
                Some(hard) => Remedy::StateWithin {
                    floor: hit.actual.max(limit),
                    hard,
                },
                None => Remedy::RetuneTo(quantized),
            },
            silences_nothing: hit.actual < limit,
        });
    }
    loose
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
fn coverage(loaded: &Loaded, measured_files: &[scan::MeasuredFile]) -> Coverage {
    let rules = loaded.checker.rules();

    let unmatched_rules = rules
        .iter()
        .filter(|rule| {
            !measured_files
                .iter()
                .any(|file| rule.selector.matches(&file.measurement.path))
        })
        .map(|rule| rule.id.clone())
        .collect();

    let catch_all_only = measured_files
        .iter()
        .filter(|file| {
            let measurement = &file.measurement;
            let matching: Vec<&_> = rules
                .iter()
                .filter(|rule| rule.selector.matches(&measurement.path))
                .collect();
            !matching.is_empty() && matching.iter().all(|rule| is_catch_all(&rule.selector))
        })
        .map(|file| file.measurement.path.to_string_lossy().replace('\\', "/"))
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
    contexts: &[report::FindingContext],
    inventory: &Inventory,
    color: bool,
    errors: &[String],
) -> String {
    let mut sections = Vec::new();

    // Each grouped block is its own section, so the blank-line separation is the
    // same between blocks as between audit sections (§FS-004-check-audit.1).
    let reported = report::finding_blocks_with_context(outcomes, color, contexts);
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

    if let Some(loose) = &inventory.loose {
        let mut lines = vec!["loose ceilings:".to_owned()];
        if loose.is_empty() {
            lines.push("  none".to_owned());
        }
        for item in loose {
            lines.push(format!("  {}", render_loose_text(item)));
        }
        sections.push(lines.join("\n"));
    }

    if let Some(coverage) = &inventory.coverage {
        sections.push(render_coverage_text(coverage));
    }

    sections.join("\n\n")
}

fn render_loose_text(item: &Loose) -> String {
    let Loose {
        site,
        unit,
        accepted,
        actual,
        ..
    } = item;
    let advice = if item.silences_nothing {
        format!(
            "silences nothing now; the {} limit is {}",
            item.severity, item.limit
        )
    } else {
        // `retune` refuses the measured form on the hard limit, so the line names
        // the one it accepts and the range that keeps it under
        // (§DF-010-stated-ceilings-are-exact.2).
        match item.remedy {
            Remedy::RetuneTo(to) => format!("retune to {to}"),
            Remedy::StateWithin { floor, hard } => {
                format!("retune with --max <N> --unit {unit}, {floor} <= N < {hard}")
            }
        }
    };
    format!("{site} accepts {accepted} {unit}, now {actual} — {advice}")
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

    if let Some(loose) = &inventory.loose {
        let entries = loose
            .iter()
            .map(|item| {
                // `severity` and `limit` travel with the record: the text line
                // states both, and a consumer must not have to re-derive them by
                // matching the registry filename against the config.
                Json::Object(vec![
                    ("registry", Json::str(item.site.registry.clone())),
                    ("path", Json::str(item.site.path.clone())),
                    ("severity", Json::str(item.severity.to_string())),
                    ("unit", Json::str(item.unit.to_string())),
                    ("accepted", Json::UInt(item.accepted)),
                    ("actual", Json::UInt(item.actual)),
                    ("limit", Json::UInt(item.limit)),
                    (
                        "retune_to",
                        match item.remedy {
                            Remedy::RetuneTo(to) => Json::UInt(to),
                            Remedy::StateWithin { .. } => Json::Null,
                        },
                    ),
                    // The remedy a `null` `retune_to` stands for, so a consumer
                    // reading JSON has the range the text line prints rather
                    // than an absence (§FS-004-check-audit.2).
                    (
                        "stated_range",
                        match item.remedy {
                            Remedy::RetuneTo(_) => Json::Null,
                            Remedy::StateWithin { floor, hard } => Json::Object(vec![
                                ("min", Json::UInt(floor)),
                                ("max_excluded", Json::UInt(hard)),
                            ]),
                        },
                    ),
                    (
                        "silences_nothing",
                        Json::UInt(u64::from(item.silences_nothing)),
                    ),
                ])
            })
            .collect();
        fields.push(("loose", Json::Array(entries)));
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
