//! Shared evaluation and rendering for `check` and `audit` (§FS-004-check-audit).
//! Runs the checker over measurements, applies the exception registries and the
//! hard-implies-soft rule, and turns the result into text or JSON.

use std::error::Error;
use std::fmt;

use crate::exceptions::{ExceptionError, Kind, Registries, Verdict};
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
        // (§GOAL-006-graded-limits). A silenced hard leaves the soft finding to
        // the accepting entry's kind (§FS-003-exceptions.3).
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
                Verdict::Silenced(entry) => {
                    outcomes.push(Outcome::Silenced {
                        overflow: render_overflow(file, rule, Severity::Hard, actual, hard),
                        exception_max: entry.max_value,
                    });
                    // Structural: the split is illegal, so there is nothing to
                    // minimize and the soft registry goes unconsulted. Deferred
                    // falls through and still warns (§FS-003-exceptions.3).
                    if entry.kind == Kind::Structural {
                        continue;
                    }
                }
            }
        }

        if let Some(soft) = rule.budget.soft.filter(|soft| actual >= *soft) {
            match registries.verdict(Severity::Soft, &path, &rule.id, unit, actual)? {
                Verdict::None | Verdict::Exceeded(_) => outcomes.push(Outcome::Reported(
                    render_overflow(file, rule, Severity::Soft, actual, soft),
                )),
                Verdict::Silenced(entry) => outcomes.push(Outcome::Silenced {
                    overflow: render_overflow(file, rule, Severity::Soft, actual, soft),
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

/// Guidance wraps at a fixed width, not the terminal's, so a block is
/// byte-identical in a narrow terminal and in CI (§GOAL-006-graded-limits.2).
const GUIDANCE_COLUMNS: usize = 78;

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

/// The standing findings as grouped text blocks: one per `(severity, rule,
/// rendered guidance)`, hard first, files largest first, and shared guidance
/// written once under a severity-tinted header (§FS-004-check-audit.1).
pub fn finding_blocks(outcomes: &[Outcome], color: bool) -> Vec<String> {
    let mut groups: Vec<Group<'_>> = Vec::new();

    for overflow in outcomes
        .iter()
        .filter(|outcome| outcome.is_reported())
        .map(Outcome::overflow)
    {
        match groups.iter_mut().find(|group| group.accepts(overflow)) {
            Some(group) => group.overflows.push(overflow),
            None => groups.push(Group {
                head: overflow,
                overflows: vec![overflow],
            }),
        }
    }

    groups.sort_by(|left, right| left.order().cmp(&right.order()));
    for group in &mut groups {
        // Worst first: the file that most needs splitting leads its block.
        group.overflows.sort_by(|left, right| {
            right
                .actual
                .cmp(&left.actual)
                .then(left.path.cmp(&right.path))
        });
    }

    groups.iter().map(|group| group.render(color)).collect()
}

/// The findings that share a severity, a rule, and one rendered guidance string.
struct Group<'a> {
    head: &'a Overflow,
    overflows: Vec<&'a Overflow>,
}

impl<'a> Group<'a> {
    /// Guidance is compared as rendered text, not by message ID: a template that
    /// interpolates `{path}` says something different about each file, so those
    /// findings must not be collapsed under one line (§FS-001-config.4).
    fn accepts(&self, overflow: &Overflow) -> bool {
        self.head.severity == overflow.severity
            && self.head.rule_id == overflow.rule_id
            && self.head.message.text == overflow.message.text
    }

    /// Hard before soft, then by rule ID, then by message ID.
    fn order(&self) -> (u8, &str, &str) {
        let severity = match self.head.severity {
            Severity::Hard => 0,
            Severity::Soft => 1,
        };
        (severity, &self.head.rule_id, &self.head.message.id)
    }

    fn header(&self) -> String {
        let files = if self.overflows.len() == 1 {
            "1 file".to_owned()
        } else {
            format!("{} files", self.overflows.len())
        };
        format!(
            "{}: {files} over the {}-{} budget [rule: {}, message: {}]",
            self.head.severity,
            self.head.limit,
            self.head.unit.singular(),
            self.head.rule_id,
            self.head.message.id,
        )
    }

    fn render(&self, color: bool) -> String {
        let code = match self.head.severity {
            Severity::Hard => BOLD_RED,
            Severity::Soft => BOLD_YELLOW,
        };
        let mut block = paint(color, code, &self.header());

        for line in wrap(&self.head.message.text, GUIDANCE_COLUMNS) {
            block.push_str("\n  ");
            block.push_str(&line);
        }

        for overflow in &self.overflows {
            block.push_str(&format!(
                "\n    {}: {} {}",
                overflow.path.display(),
                overflow.actual,
                overflow.unit
            ));
        }

        block
    }
}

/// Greedy word wrap at `columns`, measured in characters. Explicit newlines in
/// the message are kept as paragraph breaks; a word longer than `columns` (a
/// path, a URL) takes its own line rather than being broken.
fn wrap(text: &str, columns: usize) -> Vec<String> {
    let mut lines = Vec::new();

    for paragraph in text.lines() {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            let width = line.chars().count();
            if !line.is_empty() && width + 1 + word.chars().count() > columns {
                lines.push(std::mem::take(&mut line));
            } else if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
        lines.push(line);
    }

    lines
}

/// The success marker, tinted green when `color` is set (§FS-001-config.6).
pub fn success_marker(marker: &str, color: bool) -> String {
    paint(color, GREEN, marker)
}

/// The audit attribution line for a silenced overflow (§FS-003-exceptions.5).
/// Path plus severity locate the entry — the registry the severity names, under
/// this path — so there is no id to quote (§DF-005-exception-identity).
pub fn silenced_line(overflow: &Overflow, exception_max: u64) -> String {
    format!(
        "{}: {} exception (accepted up to {} {})",
        overflow.path.display(),
        overflow.severity,
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
    if let Outcome::Silenced { exception_max, .. } = outcome {
        fields.push(("exception_max", Json::UInt(*exception_max)));
    }
    Json::Object(fields)
}

#[cfg(test)]
#[path = "report_tests.rs"]
mod tests;
