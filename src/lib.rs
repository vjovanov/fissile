//! Core library for `fissile`.
//!
//! `fissile` keeps files small by evaluating measured files against configured
//! budgets and returning structured overflow findings with project-owned,
//! architecture-aware remediation messages.

use std::cmp::Ordering;
use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

pub mod audit;
pub mod check;
pub mod cli;
mod comments;
pub mod config;
pub mod exception;
pub mod exceptions;
mod glob;
pub mod init;
pub mod json;
pub mod report;
pub mod scan;

pub use glob::Glob;

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

/// A pair of optional soft and hard limits for one unit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Budget {
    pub unit: Unit,
    pub soft: Option<u64>,
    pub hard: Option<u64>,
}

impl Budget {
    pub fn new(unit: Unit, soft: Option<u64>, hard: Option<u64>) -> Self {
        Self { unit, soft, hard }
    }

    fn validate(&self, rule_id: &str) -> Result<(), FissileError> {
        if self.soft.is_none() && self.hard.is_none() {
            return Err(FissileError::InvalidBudget {
                rule_id: rule_id.to_owned(),
                reason: "at least one of soft or hard must be set".to_owned(),
            });
        }

        if let (Some(soft), Some(hard)) = (self.soft, self.hard)
            && soft > hard
        {
            return Err(FissileError::InvalidBudget {
                rule_id: rule_id.to_owned(),
                reason: "soft limit cannot be greater than hard limit".to_owned(),
            });
        }

        Ok(())
    }
}

/// Which paths a rule applies to.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Selector {
    All,
    Extension(String),
    Prefix(String),
    Exact(String),
    /// One or more globs; the selector matches when any glob matches. This is
    /// the variant produced from a config rule's `include` list
    /// (§FS-001-config.3).
    Glob(Vec<Glob>),
}

impl Selector {
    pub fn matches(&self, path: &Path) -> bool {
        match self {
            Selector::All => true,
            Selector::Extension(extension) => path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value == extension.trim_start_matches('.')),
            Selector::Prefix(prefix) => path.to_string_lossy().starts_with(prefix),
            Selector::Exact(expected) => path.to_string_lossy() == expected.as_str(),
            Selector::Glob(globs) => {
                let path = path.to_string_lossy();
                globs.iter().any(|glob| glob.matches(&path))
            }
        }
    }

    fn specificity(&self) -> (u8, usize) {
        match self {
            Selector::All => (0, 0),
            Selector::Extension(extension) => (1, extension.trim_start_matches('.').len()),
            Selector::Prefix(prefix) => (2, prefix.len()),
            Selector::Exact(path) => (3, path.len()),
            // Only consulted when a glob selector is compared against a
            // non-glob one, which a config-built checker never does; glob-vs-glob
            // overlap uses the partial order in `selector_specificity_cmp`.
            Selector::Glob(_) => (2, 0),
        }
    }
}

/// Static, project-owned guidance rendered when a rule overflows.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MessageTemplate {
    pub id: String,
    pub text: String,
}

impl MessageTemplate {
    pub fn new(id: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
        }
    }

    fn validate(&self, rule_id: &str) -> Result<(), FissileError> {
        if self.id.trim().is_empty() {
            return Err(FissileError::InvalidMessage {
                rule_id: rule_id.to_owned(),
                reason: "message id cannot be empty".to_owned(),
            });
        }

        if self.text.trim().is_empty() {
            return Err(FissileError::InvalidMessage {
                rule_id: rule_id.to_owned(),
                reason: "message text cannot be empty".to_owned(),
            });
        }

        Ok(())
    }

    fn render(&self, context: &MessageContext<'_>) -> RenderedMessage {
        let text = render_template(&self.text, context);

        RenderedMessage {
            id: self.id.clone(),
            text,
        }
    }
}

/// A size rule selected by path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rule {
    pub id: String,
    pub selector: Selector,
    pub budget: Budget,
    pub message: MessageTemplate,
    pub priority: i32,
    /// Whether blank lines count toward a `lines` budget. Default `false`
    /// (§FS-001-config.3.1).
    pub count_blank_lines: bool,
    /// Whether whole-line comments count toward a `lines` budget; default `true`
    /// (§FS-001-config.3.1). Applied to the per-rule line count in [`Checker::check`].
    pub count_comment_lines: bool,
}

impl Rule {
    pub fn new(
        id: impl Into<String>,
        selector: Selector,
        budget: Budget,
        message: MessageTemplate,
    ) -> Self {
        Self {
            id: id.into(),
            selector,
            budget,
            message,
            priority: 0,
            count_blank_lines: false,
            count_comment_lines: true,
        }
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Set the line-counting policy (§FS-001-config.3.1).
    pub fn with_line_policy(mut self, count_blank_lines: bool, count_comment_lines: bool) -> Self {
        self.count_blank_lines = count_blank_lines;
        self.count_comment_lines = count_comment_lines;
        self
    }

    fn validate(&self) -> Result<(), FissileError> {
        if self.id.trim().is_empty() {
            return Err(FissileError::InvalidRule {
                reason: "rule id cannot be empty".to_owned(),
            });
        }

        self.budget.validate(&self.id)?;
        self.message.validate(&self.id)
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
    /// the rule's policy in [`Checker::check`], not here.
    fn value(&self, unit: Unit) -> Option<u64> {
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
    let stats = comments::classify(&path, text);
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

/// Evaluates configured rules against file measurements.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Checker {
    rules: Vec<Rule>,
}

impl Checker {
    pub fn new(rules: Vec<Rule>) -> Result<Self, FissileError> {
        for rule in &rules {
            rule.validate()?;
        }

        Ok(Self { rules })
    }

    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    /// Evaluate a file and return the effective rule plus its measured value for
    /// every rule that applies. Callers that need exception logic and the
    /// hard-implies-soft rule build findings from this (§FS-004-check-audit).
    pub fn evaluate<'a>(
        &'a self,
        file: &FileMeasurement,
    ) -> Result<Vec<RuleHit<'a>>, FissileError> {
        self.effective_rules(file)?
            .into_iter()
            .map(|rule| {
                let actual = self.measured_value(file, rule)?;
                Ok(RuleHit { rule, actual })
            })
            .collect()
    }

    fn measured_value(&self, file: &FileMeasurement, rule: &Rule) -> Result<u64, FissileError> {
        let measured = match rule.budget.unit {
            Unit::Lines => file
                .lines
                .map(|stats| stats.counted(rule.count_blank_lines, rule.count_comment_lines)),
            unit => file.value(unit),
        };
        measured.ok_or_else(|| FissileError::MissingMeasurement {
            path: file.path.clone(),
            rule_id: rule.id.clone(),
            unit: rule.budget.unit,
        })
    }

    /// The commit-time view: one overflow per rule, hard suppressing soft
    /// (§GOAL-006-graded-limits). Exception registries are not consulted here.
    pub fn check(&self, file: &FileMeasurement) -> Result<Vec<Overflow>, FissileError> {
        let mut overflows = Vec::new();

        for hit in self.evaluate(file)? {
            let RuleHit { rule, actual } = hit;
            if let Some(hard) = rule.budget.hard
                && actual >= hard
            {
                overflows.push(render_overflow(file, rule, Severity::Hard, actual, hard));
                continue;
            }
            if let Some(soft) = rule.budget.soft
                && actual >= soft
            {
                overflows.push(render_overflow(file, rule, Severity::Soft, actual, soft));
            }
        }

        Ok(overflows)
    }

    fn effective_rules<'a>(
        &'a self,
        file: &FileMeasurement,
    ) -> Result<Vec<&'a Rule>, FissileError> {
        let mut selected: Vec<EffectiveRule<'a>> = Vec::new();

        for rule in self
            .rules
            .iter()
            .filter(|rule| rule.selector.matches(&file.path))
        {
            match selected
                .iter()
                .position(|candidate| candidate.rule.budget.unit == rule.budget.unit)
            {
                Some(index) => match compare_rules(rule, selected[index].rule, &file.path) {
                    Ordering::Greater => {
                        selected[index] = EffectiveRule {
                            rule,
                            tied_rule_ids: Vec::new(),
                        };
                    }
                    Ordering::Equal => {
                        selected[index].tied_rule_ids.push(rule.id.clone());
                    }
                    Ordering::Less => {}
                },
                None => selected.push(EffectiveRule {
                    rule,
                    tied_rule_ids: Vec::new(),
                }),
            }
        }

        for candidate in &selected {
            if !candidate.tied_rule_ids.is_empty() {
                let mut rule_ids = Vec::with_capacity(candidate.tied_rule_ids.len() + 1);
                rule_ids.push(candidate.rule.id.clone());
                rule_ids.extend(candidate.tied_rule_ids.iter().cloned());
                return Err(FissileError::AmbiguousRules {
                    path: file.path.clone(),
                    unit: candidate.rule.budget.unit,
                    rule_ids,
                });
            }
        }

        Ok(selected
            .into_iter()
            .map(|candidate| candidate.rule)
            .collect())
    }
}

/// An effective rule paired with the file's measured value for that rule's unit
/// (§FS-004-check-audit). Produced by [`Checker::evaluate`].
#[derive(Clone, Copy, Debug)]
pub struct RuleHit<'a> {
    pub rule: &'a Rule,
    pub actual: u64,
}

struct EffectiveRule<'a> {
    rule: &'a Rule,
    tied_rule_ids: Vec<String>,
}

fn compare_rules(left: &Rule, right: &Rule, path: &Path) -> Ordering {
    left.priority
        .cmp(&right.priority)
        .then_with(|| selector_specificity_cmp(&left.selector, &right.selector, path))
}

/// Compare two selectors' specificity for a path. Glob-vs-glob uses the glob
/// engine's partial order — incomparable globs collapse to `Equal` so the caller
/// raises an ambiguity error (§FS-001-config.3.2); other pairings use tiers.
fn selector_specificity_cmp(left: &Selector, right: &Selector, path: &Path) -> Ordering {
    match (left, right) {
        (Selector::Glob(left_globs), Selector::Glob(right_globs)) => {
            let path = path.to_string_lossy();
            match (
                best_glob_spec(left_globs, &path),
                best_glob_spec(right_globs, &path),
            ) {
                (Some(left_spec), Some(right_spec)) => left_spec.cmp_specificity(&right_spec),
                _ => Ordering::Equal,
            }
        }
        _ => left.specificity().cmp(&right.specificity()),
    }
}

/// The specificity of the most-specific glob in `globs` that matches `path`.
fn best_glob_spec(globs: &[Glob], path: &str) -> Option<glob::GlobSpec> {
    globs
        .iter()
        .filter(|glob| glob.matches(path))
        .map(Glob::spec)
        .reduce(|best, candidate| match candidate.cmp_specificity(&best) {
            Ordering::Greater => candidate,
            _ => best,
        })
}

fn render_template(template: &str, context: &MessageContext<'_>) -> String {
    let path = context.path.to_string_lossy();
    let actual = context.actual.to_string();
    let limit = context.limit.to_string();
    let unit = context.unit.to_string();

    let mut rendered = String::with_capacity(template.len());
    let mut rest = template;

    while let Some(start) = rest.find('{') {
        rendered.push_str(&rest[..start]);
        rest = &rest[start..];

        let Some(end) = rest.find('}') else {
            rendered.push_str(rest);
            return rendered;
        };

        let placeholder = &rest[..=end];
        match placeholder {
            "{path}" => rendered.push_str(&path),
            "{rule}" => rendered.push_str(context.rule_id),
            "{severity}" => rendered.push_str(context.severity.as_str()),
            "{actual}" => rendered.push_str(&actual),
            "{limit}" => rendered.push_str(&limit),
            "{unit}" => rendered.push_str(&unit),
            _ => rendered.push_str(placeholder),
        }
        rest = &rest[end + 1..];
    }

    rendered.push_str(rest);
    rendered
}

fn render_overflow(
    file: &FileMeasurement,
    rule: &Rule,
    severity: Severity,
    actual: u64,
    limit: u64,
) -> Overflow {
    let context = MessageContext {
        path: &file.path,
        rule_id: &rule.id,
        severity,
        unit: rule.budget.unit,
        actual,
        limit,
    };

    Overflow {
        path: file.path.clone(),
        rule_id: rule.id.clone(),
        severity,
        unit: rule.budget.unit,
        actual,
        limit,
        message: rule.message.render(&context),
    }
}

struct MessageContext<'a> {
    path: &'a Path,
    rule_id: &'a str,
    severity: Severity,
    unit: Unit,
    actual: u64,
    limit: u64,
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

#[cfg(test)]
#[path = "checker_tests.rs"]
mod tests;
