//! Public data model for measurements, findings, severities, and core errors.

use std::error::Error;
use std::fmt;
use std::path::PathBuf;

/// The unit a size budget is measured in.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Unit {
    Bytes,
    Lines,
    Tokens,
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Unit::Bytes => f.write_str("bytes"),
            Unit::Lines => f.write_str("lines"),
            Unit::Tokens => f.write_str("tokens"),
        }
    }
}

/// A physical-line breakdown for one file (§FS-001-config.3.1). `total` is the
/// physical line count; `blank` and `comment` are the disjoint subsets that a
/// per-rule policy may exclude from the measured count.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LineStats {
    pub total: u64,
    pub blank: u64,
    pub comment: u64,
}

impl LineStats {
    /// The line count a budget sees: blank lines drop unless `count_blank_lines`,
    /// whole-line comments drop unless `count_comment_lines` (§FS-001-config.3.1).
    /// `blank` and `comment` are disjoint, so the subtractions never overlap.
    pub fn counted(&self, count_blank_lines: bool, count_comment_lines: bool) -> u64 {
        let mut counted = self.total;
        if !count_blank_lines {
            counted -= self.blank;
        }
        if !count_comment_lines {
            counted -= self.comment;
        }
        counted
    }
}

/// File measurements consumed by the checker.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileMeasurement {
    pub path: PathBuf,
    pub bytes: u64,
    /// Physical line breakdown for text files; `None` for binary measurements.
    pub lines: Option<LineStats>,
    pub tokens: Option<u64>,
}

impl FileMeasurement {
    pub fn new(path: impl Into<PathBuf>, bytes: u64) -> Self {
        Self {
            path: path.into(),
            bytes,
            lines: None,
            tokens: None,
        }
    }

    /// Set a physical line count with no blank/comment breakdown. The whole count
    /// is treated as content, so line policy leaves it unchanged.
    pub fn with_lines(mut self, lines: u64) -> Self {
        self.lines = Some(LineStats {
            total: lines,
            blank: 0,
            comment: 0,
        });
        self
    }

    pub fn with_line_stats(mut self, lines: LineStats) -> Self {
        self.lines = Some(lines);
        self
    }

    pub fn with_tokens(mut self, tokens: u64) -> Self {
        self.tokens = Some(tokens);
        self
    }

    /// The measured value for a non-line unit. Line budgets are resolved against
    /// the rule's policy in the checker, not here.
    pub(crate) fn value(&self, unit: Unit) -> Option<u64> {
        match unit {
            Unit::Bytes => Some(self.bytes),
            Unit::Lines => self.lines.map(|stats| stats.total),
            Unit::Tokens => self.tokens,
        }
    }
}

/// Measure UTF-8 text by bytes and a policy-ready line breakdown. Whole-line
/// comments are classified by file extension (§FS-001-config.3.1).
pub fn measure_text(path: impl Into<PathBuf>, text: &str) -> FileMeasurement {
    let path = path.into();
    let stats = crate::comments::classify(&path, text);
    FileMeasurement::new(path, text.len() as u64).with_line_stats(stats)
}

/// Measure arbitrary bytes by byte count only.
pub fn measure_bytes(path: impl Into<PathBuf>, bytes: &[u8]) -> FileMeasurement {
    FileMeasurement::new(path, bytes.len() as u64)
}

/// Overflow severity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Severity {
    Soft,
    Hard,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Soft => "soft",
            Severity::Hard => "hard",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The rendered, architecture-aware message attached to an overflow.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderedMessage {
    pub id: String,
    pub text: String,
}

/// A structured finding for a file that crossed a budget.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Overflow {
    pub path: PathBuf,
    pub rule_id: String,
    pub severity: Severity,
    pub unit: Unit,
    pub actual: u64,
    pub limit: u64,
    pub message: RenderedMessage,
}

impl Overflow {
    pub fn finding_line(&self) -> String {
        format!(
            "{}: {} {} > {} {} [{}, rule: {}, message: {}]",
            self.path.display(),
            self.actual,
            self.unit,
            self.limit,
            self.unit,
            self.severity,
            self.rule_id,
            self.message.id
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FissileError {
    InvalidRule {
        reason: String,
    },
    InvalidBudget {
        rule_id: String,
        reason: String,
    },
    InvalidMessage {
        rule_id: String,
        reason: String,
    },
    MissingMeasurement {
        path: PathBuf,
        rule_id: String,
        unit: Unit,
    },
    AmbiguousRules {
        path: PathBuf,
        unit: Unit,
        rule_ids: Vec<String>,
    },
}

impl fmt::Display for FissileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FissileError::InvalidRule { reason } => write!(f, "invalid rule: {reason}"),
            FissileError::InvalidBudget { rule_id, reason } => {
                write!(f, "invalid budget for rule {rule_id}: {reason}")
            }
            FissileError::InvalidMessage { rule_id, reason } => {
                write!(f, "invalid message for rule {rule_id}: {reason}")
            }
            FissileError::MissingMeasurement {
                path,
                rule_id,
                unit,
            } => write!(
                f,
                "missing {unit} measurement for {} under rule {rule_id}",
                path.display()
            ),
            FissileError::AmbiguousRules {
                path,
                unit,
                rule_ids,
            } => write!(
                f,
                "ambiguous {unit} rules for {}: {}",
                path.display(),
                rule_ids.join(", ")
            ),
        }
    }
}

impl Error for FissileError {}
