//! Shared evaluation and rendering for `check` and `audit` (§FS-004-check-audit).
//! Runs the checker over measurements, applies the exception registries and the
//! hard-implies-soft rule, and turns the result into text or JSON.

use std::error::Error;
use std::fmt;

use crate::exceptions::{ExceptionError, Registries, Verdict};
use crate::json::Json;
use crate::{Checker, FileMeasurement, FissileError, Overflow, Severity, render_overflow};

/// What evaluating one `(file, rule, severity)` produced.
#[derive(Clone, Debug)]
pub enum Outcome {
    /// A standing finding: no exception silenced it.
    Reported(Overflow),
    /// An overflow accepted by an exception. Carried for audit attribution
    /// (§FS-003-exceptions.5); never fails a build.
    Silenced {
        overflow: Overflow,
        exception_id: String,
        exception_max: u64,
    },
}

impl Outcome {
    pub fn overflow(&self) -> &Overflow {
        match self {
            Outcome::Reported(overflow) => overflow,
            Outcome::Silenced { overflow, .. } => overflow,
        }
    }

    pub fn is_reported(&self) -> bool {
        matches!(self, Outcome::Reported(_))
    }
}

/// A failure while evaluating: a config/engine error or an exception schema error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvalError {
    Engine(FissileError),
    Exceptions(ExceptionError),
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvalError::Engine(error) => write!(f, "{error}"),
            EvalError::Exceptions(error) => write!(f, "{error}"),
        }
    }
}

impl Error for EvalError {}

impl From<FissileError> for EvalError {
    fn from(error: FissileError) -> Self {
        EvalError::Engine(error)
    }
}

impl From<ExceptionError> for EvalError {
    fn from(error: ExceptionError) -> Self {
        EvalError::Exceptions(error)
    }
}

/// Evaluate one measured file against the checker and exception registries.
pub fn evaluate_file(
    checker: &Checker,
    registries: &Registries,
    file: &FileMeasurement,
) -> Result<Vec<Outcome>, EvalError> {
    let path = file.path.to_string_lossy().replace('\\', "/");
    let mut outcomes = Vec::new();

    for hit in checker.evaluate(file)? {
        let rule = hit.rule;
        let unit = rule.budget.unit;
        let actual = hit.actual;

        // Hard overflow: a standing hard finding suppresses the soft one
        // (§GOAL-006-graded-limits). A silenced hard still lets the soft finding
        // through so agents keep minimizing accepted debt (§FS-003-exceptions.3).
        if let Some(hard) = rule.budget.hard.filter(|hard| actual >= *hard) {
            match registries.verdict(Severity::Hard, &path, &rule.id, unit, actual)? {
                Verdict::None | Verdict::Exceeded(_) => {
                    outcomes.push(Outcome::Reported(render_overflow(
                        file,
                        rule,
                        Severity::Hard,
                        actual,
                        hard,
                    )));
                    continue;
                }
                Verdict::Silenced(entry) => outcomes.push(Outcome::Silenced {
                    overflow: render_overflow(file, rule, Severity::Hard, actual, hard),
                    exception_id: entry.id.clone(),
                    exception_max: entry.max_value,
                }),
            }
        }

        if let Some(soft) = rule.budget.soft.filter(|soft| actual >= *soft) {
            match registries.verdict(Severity::Soft, &path, &rule.id, unit, actual)? {
                Verdict::None | Verdict::Exceeded(_) => outcomes.push(Outcome::Reported(
                    render_overflow(file, rule, Severity::Soft, actual, soft),
                )),
                Verdict::Silenced(entry) => outcomes.push(Outcome::Silenced {
                    overflow: render_overflow(file, rule, Severity::Soft, actual, soft),
                    exception_id: entry.id.clone(),
                    exception_max: entry.max_value,
                }),
            }
        }
    }

    Ok(outcomes)
}

/// Whether any outcome is a standing hard finding — the build-failing condition.
pub fn has_hard_failure(outcomes: &[Outcome]) -> bool {
    outcomes
        .iter()
        .any(|outcome| outcome.is_reported() && outcome.overflow().severity == Severity::Hard)
}

const BOLD_RED: &str = "\x1b[1;31m";
const BOLD_YELLOW: &str = "\x1b[1;33m";
const GREEN: &str = "\x1b[32m";
const RESET: &str = "\x1b[0m";

fn paint(color: bool, code: &str, text: &str) -> String {
    if color {
        format!("{code}{text}{RESET}")
    } else {
        text.to_owned()
    }
}

/// The two-line text block for one standing finding (§FS-004-check-audit.1). The
/// finding line is tinted by severity when `color` is set; the guidance line is
/// left plain so it stays easy to copy.
pub fn finding_block(overflow: &Overflow, color: bool) -> String {
    let code = match overflow.severity {
        Severity::Hard => BOLD_RED,
        Severity::Soft => BOLD_YELLOW,
    };
    format!(
        "{}\n  {}",
        paint(color, code, &overflow.finding_line()),
        overflow.message.text
    )
}

/// The success marker, tinted green when `color` is set (§FS-001-config.6).
pub fn success_marker(marker: &str, color: bool) -> String {
    paint(color, GREEN, marker)
}

/// The audit attribution line for a silenced overflow (§FS-003-exceptions.5).
pub fn silenced_line(overflow: &Overflow, exception_id: &str, exception_max: u64) -> String {
    format!(
        "{}: {} exception {} (accepted up to {} {})",
        overflow.path.display(),
        overflow.severity,
        exception_id,
        exception_max,
        overflow.unit,
    )
}

/// One JSON finding record (§FS-004-check-audit.1). Exception fields are added
/// only for silenced audit records.
pub fn overflow_json(outcome: &Outcome) -> Json {
    let overflow = outcome.overflow();
    let mut fields = vec![
        ("path", Json::str(overflow.path.to_string_lossy())),
        ("unit", Json::str(overflow.unit.to_string())),
        ("actual", Json::UInt(overflow.actual)),
        ("limit", Json::UInt(overflow.limit)),
        ("severity", Json::str(overflow.severity.as_str())),
        ("rule_id", Json::str(overflow.rule_id.clone())),
        ("message_id", Json::str(overflow.message.id.clone())),
        ("message", Json::str(overflow.message.text.clone())),
    ];
    if let Outcome::Silenced {
        exception_id,
        exception_max,
        ..
    } = outcome
    {
        fields.push(("exception_id", Json::str(exception_id.clone())));
        fields.push(("exception_max", Json::UInt(*exception_max)));
    }
    Json::Object(fields)
}
