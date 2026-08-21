//! `fissile measure` (§FS-007-measure): what the budgets count, for a passing
//! file as much as a failing one. The arithmetic is fissile's own and nothing
//! outside fissile reproduces it (§FS-001-config.3.1).

use std::path::PathBuf;

use crate::cli::{self, CommandError, Format, Loaded};
use crate::exceptions::Verdict;
use crate::json::Json;
use crate::report;
use crate::{FileMeasurement, Severity, Unit, scan};

/// Inputs to a `measure` run.
#[derive(Clone, Debug)]
pub struct MeasureOptions {
    pub root: PathBuf,
    pub config_path: Option<PathBuf>,
    pub staged: bool,
    pub format: Option<Format>,
    pub no_color: bool,
    pub paths: Vec<String>,
}

/// The rendered output plus the file-level errors that force exit 2 without
/// hiding the rest (§FS-004-check-audit.5). No `failed`: a file over a hard
/// limit is an answer, not a verdict (§FS-007-measure.1).
pub struct Run {
    pub output: String,
    pub errors: Vec<String>,
}

/// One `(file, rule)` pair, or a file no rule measures.
enum Row {
    Measured(Measured),
    Unruled { path: String },
}

struct Measured {
    path: String,
    unit: Unit,
    actual: u64,
    rule_id: String,
    soft: Option<u64>,
    hard: Option<u64>,
    soft_accepted: Option<u64>,
    hard_accepted: Option<u64>,
}

/// The distance to the nearest threshold above the value, or past the highest
/// one below it (§FS-007-measure.2).
struct Headroom {
    distance: u64,
    label: &'static str,
    over: bool,
}

pub fn run(options: &MeasureOptions) -> Result<Run, CommandError> {
    let loaded = cli::load(&options.root, options.config_path.as_deref())?;
    // No whole-repo default: "these files" is `measure`, "the repository" is
    // `audit --top` (§FS-007-measure.1).
    let files = cli::selected_files(&loaded, options.staged, &options.paths)?.ok_or_else(|| {
        CommandError::Usage(
            "measure needs <paths>... or --staged; `fissile audit --top <N>` ranks the \
             whole repository"
                .to_owned(),
        )
    })?;
    let format = options
        .format
        .unwrap_or_else(|| loaded.config.output.format.into());

    let mut rows = Vec::new();
    let mut errors = Vec::new();
    for rel in files {
        let measurement = if options.staged {
            scan::measure_staged_file(&loaded.root, &rel, &loaded.config.tokens)
        } else {
            scan::measure_file(&loaded.root, &rel, &loaded.config.tokens)
        };
        // A path that cannot be measured is skipped, not fatal, exactly as in
        // `check` (§FS-004-check-audit.5).
        match measurement {
            Ok(measurement) => rows.extend(rows_for(&loaded, &measurement)?),
            Err(error) => errors.push(scan::measure_error_line(&rel, &error)),
        }
    }

    let output = match format {
        Format::Text => {
            let color = cli::use_color(loaded.config.output.color, options.no_color, format);
            rows.iter()
                .map(|row| render_row(row, color))
                .collect::<Vec<_>>()
                .join("\n")
        }
        Format::Json => Json::Array(rows.iter().map(row_json).collect()).render(),
    };
    Ok(Run { output, errors })
}

/// Every rule that measures this file, with the ceilings each registry records
/// for it. A rule and its exceptions are read together because the ceiling is
/// what the headroom is measured against once a file is over limit.
fn rows_for(loaded: &Loaded, file: &FileMeasurement) -> Result<Vec<Row>, CommandError> {
    let path = file.path.to_string_lossy().replace('\\', "/");
    let hits = loaded
        .checker
        .evaluate(file)
        .map_err(crate::report::EvalError::from)?;

    if hits.is_empty() {
        // "No budget applies here" is an answer; silence would read as zero.
        return Ok(vec![Row::Unruled { path }]);
    }

    let mut rows = Vec::with_capacity(hits.len());
    for hit in hits {
        let unit = hit.rule.budget.unit;
        rows.push(Row::Measured(Measured {
            soft_accepted: accepted(
                loaded,
                Severity::Soft,
                &path,
                &hit.rule.id,
                unit,
                hit.actual,
            )?,
            hard_accepted: accepted(
                loaded,
                Severity::Hard,
                &path,
                &hit.rule.id,
                unit,
                hit.actual,
            )?,
            path: path.clone(),
            unit,
            actual: hit.actual,
            rule_id: hit.rule.id.clone(),
            soft: hit.rule.budget.soft,
            hard: hit.rule.budget.hard,
        }));
    }
    Ok(rows)
}

/// The ceiling one registry records for this `(path, rule, unit)`, whether or
/// not the file is currently within it — a ceiling above the file is exactly the
/// number a caller sizing an edit needs.
fn accepted(
    loaded: &Loaded,
    severity: Severity,
    path: &str,
    rule_id: &str,
    unit: Unit,
    actual: u64,
) -> Result<Option<u64>, CommandError> {
    Ok(
        match loaded
            .registries
            .verdict(severity, path, rule_id, unit, actual)?
        {
            Verdict::Silenced(entry) | Verdict::Exceeded(entry) => Some(entry.max_value),
            Verdict::None => None,
        },
    )
}

fn thresholds(row: &Measured) -> Vec<(&'static str, u64)> {
    [
        ("soft", row.soft),
        ("hard", row.hard),
        ("soft-accepted", row.soft_accepted),
        ("hard-accepted", row.hard_accepted),
    ]
    .into_iter()
    .filter_map(|(label, value)| value.map(|value| (label, value)))
    .collect()
}

fn headroom(row: &Measured) -> Option<Headroom> {
    let thresholds = thresholds(row);
    if let Some(&(label, value)) = thresholds
        .iter()
        .filter(|(_, value)| *value > row.actual)
        .min_by_key(|(_, value)| *value)
    {
        return Some(Headroom {
            distance: value - row.actual,
            label,
            over: false,
        });
    }
    // Above everything: report the distance past the highest threshold, which is
    // the one a reader has to answer for.
    let &(label, value) = thresholds.iter().max_by_key(|(_, value)| *value)?;
    Some(Headroom {
        distance: row.actual - value,
        label,
        over: true,
    })
}

fn render_row(row: &Row, color: bool) -> String {
    let row = match row {
        Row::Unruled { path } => return format!("{path} — no rule applies"),
        Row::Measured(row) => row,
    };
    let mut line = format!("{} {} {} [{}]", row.path, row.actual, row.unit, row.rule_id);
    for (label, value) in thresholds(row) {
        line.push_str(&format!(" {label} {value}"));
    }
    if let Some(headroom) = headroom(row) {
        let Headroom {
            distance,
            label,
            over,
        } = headroom;
        let clause = if over {
            format!("{distance} over {label}")
        } else {
            format!("{distance} to {label}")
        };
        let tint = if !over {
            None
        } else if label.starts_with("hard") {
            Some(report::BOLD_RED)
        } else {
            Some(report::BOLD_YELLOW)
        };
        let clause = match tint {
            Some(code) => report::paint(color, code, &clause),
            None => clause,
        };
        line.push_str(&format!(" — {clause}"));
    }
    line
}

fn row_json(row: &Row) -> Json {
    let row = match row {
        Row::Unruled { path } => {
            return Json::Object(vec![
                ("path", Json::str(path.clone())),
                ("unruled", Json::UInt(1)),
            ]);
        }
        Row::Measured(row) => row,
    };
    let mut fields = vec![
        ("path", Json::str(row.path.clone())),
        ("unit", Json::str(row.unit.to_string())),
        ("actual", Json::UInt(row.actual)),
        ("rule_id", Json::str(row.rule_id.clone())),
    ];
    // A threshold that does not exist is omitted, not nulled (§FS-007-measure.2).
    for (key, value) in [
        ("soft", row.soft),
        ("hard", row.hard),
        ("soft_accepted", row.soft_accepted),
        ("hard_accepted", row.hard_accepted),
    ] {
        if let Some(value) = value {
            fields.push((key, Json::UInt(value)));
        }
    }
    if let Some(headroom) = headroom(row) {
        // Signed: positive is room left below the threshold, negative is the
        // distance past it, so one field carries both directions.
        let signed = i64::try_from(headroom.distance).unwrap_or(i64::MAX);
        fields.push((
            "headroom",
            Json::Int(if headroom.over { -signed } else { signed }),
        ));
        fields.push(("headroom_to", Json::str(headroom.label)));
    }
    Json::Object(fields)
}
