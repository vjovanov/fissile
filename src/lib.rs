//! Core library for `fissile`.
//!
//! `fissile` keeps files small by evaluating measured files against configured
//! budgets and returning structured overflow findings with project-owned,
//! architecture-aware remediation messages.

use std::cmp::Ordering;
use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

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
        }
    }

    fn specificity(&self) -> (u8, usize) {
        match self {
            Selector::All => (0, 0),
            Selector::Extension(extension) => (1, extension.trim_start_matches('.').len()),
            Selector::Prefix(prefix) => (2, prefix.len()),
            Selector::Exact(path) => (3, path.len()),
        }
    }
}

/// Static, project-owned guidance rendered when a rule overflows.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MessageTemplate {
    pub id: String,
    pub text: String,
    pub architecture_ref: Option<String>,
    pub owner: Option<String>,
    pub destination: Option<String>,
    pub action: Option<String>,
}

impl MessageTemplate {
    pub fn new(id: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            architecture_ref: None,
            owner: None,
            destination: None,
            action: None,
        }
    }

    pub fn with_architecture_ref(mut self, value: impl Into<String>) -> Self {
        self.architecture_ref = Some(value.into());
        self
    }

    pub fn with_owner(mut self, value: impl Into<String>) -> Self {
        self.owner = Some(value.into());
        self
    }

    pub fn with_destination(mut self, value: impl Into<String>) -> Self {
        self.destination = Some(value.into());
        self
    }

    pub fn with_action(mut self, value: impl Into<String>) -> Self {
        self.action = Some(value.into());
        self
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
            architecture_ref: self.architecture_ref.clone(),
            owner: self.owner.clone(),
            destination: self.destination.clone(),
            action: self.action.clone(),
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
        }
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
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

/// File measurements consumed by the checker.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileMeasurement {
    pub path: PathBuf,
    pub bytes: u64,
    pub lines: Option<u64>,
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

    pub fn with_lines(mut self, lines: u64) -> Self {
        self.lines = Some(lines);
        self
    }

    pub fn with_tokens(mut self, tokens: u64) -> Self {
        self.tokens = Some(tokens);
        self
    }

    fn value(&self, unit: Unit) -> Option<u64> {
        match unit {
            Unit::Bytes => Some(self.bytes),
            Unit::Lines => self.lines,
            Unit::Tokens => self.tokens,
        }
    }
}

/// Measure UTF-8 text by bytes and logical line count.
pub fn measure_text(path: impl Into<PathBuf>, text: &str) -> FileMeasurement {
    let lines = if text.is_empty() {
        0
    } else {
        text.as_bytes()
            .iter()
            .filter(|byte| **byte == b'\n')
            .count() as u64
            + u64::from(!text.ends_with('\n'))
    };

    FileMeasurement::new(path, text.len() as u64).with_lines(lines)
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
    pub architecture_ref: Option<String>,
    pub owner: Option<String>,
    pub destination: Option<String>,
    pub action: Option<String>,
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

    pub fn check(&self, file: &FileMeasurement) -> Result<Vec<Overflow>, FissileError> {
        let mut overflows = Vec::new();
        let rules = self.effective_rules(file)?;

        for rule in rules {
            let actual =
                file.value(rule.budget.unit)
                    .ok_or_else(|| FissileError::MissingMeasurement {
                        path: file.path.clone(),
                        rule_id: rule.id.clone(),
                        unit: rule.budget.unit,
                    })?;

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
                Some(index) => match compare_rules(rule, selected[index].rule) {
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

struct EffectiveRule<'a> {
    rule: &'a Rule,
    tied_rule_ids: Vec<String>,
}

fn compare_rules(left: &Rule, right: &Rule) -> Ordering {
    left.priority.cmp(&right.priority).then_with(|| {
        left.selector
            .specificity()
            .cmp(&right.selector.specificity())
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
mod tests {
    use super::*;

    #[test]
    fn reports_hard_overflow_and_suppresses_soft_for_same_rule() {
        let checker = Checker::new(vec![Rule::new(
            "rust",
            Selector::Extension("rs".to_owned()),
            Budget::new(Unit::Lines, Some(2), Some(3)),
            MessageTemplate::new("split-rust", "Split {path}: {actual} {unit}."),
        )])
        .expect("valid checker");

        let file = measure_text("src/lib.rs", "a\nb\nc\n");
        let overflows = checker.check(&file).expect("check succeeds");

        assert_eq!(overflows.len(), 1);
        assert_eq!(overflows[0].severity, Severity::Hard);
        assert_eq!(overflows[0].actual, 3);
        assert_eq!(overflows[0].limit, 3);
        assert_eq!(
            overflows[0].finding_line(),
            "src/lib.rs: 3 lines > 3 lines [hard, rule: rust, message: split-rust]"
        );
    }

    #[test]
    fn renders_architecture_aware_message_fields() {
        let checker = Checker::new(vec![Rule::new(
            "domain",
            Selector::Prefix("src/domain/".to_owned()),
            Budget::new(Unit::Bytes, Some(5), None),
            MessageTemplate::new(
                "domain-split",
                "{severity} overflow in {path}; move code toward {rule}.",
            )
            .with_architecture_ref("§GOAL-008-architecture-aware-messages")
            .with_owner("@domain")
            .with_destination("src/domain/services/")
            .with_action("extract a service"),
        )])
        .expect("valid checker");

        let file = measure_bytes("src/domain/order.rs", b"abcdef");
        let overflows = checker.check(&file).expect("check succeeds");
        let message = &overflows[0].message;

        assert_eq!(overflows[0].severity, Severity::Soft);
        assert_eq!(
            message.text,
            "soft overflow in src/domain/order.rs; move code toward domain."
        );
        assert_eq!(
            message.architecture_ref.as_deref(),
            Some("§GOAL-008-architecture-aware-messages")
        );
        assert_eq!(message.owner.as_deref(), Some("@domain"));
        assert_eq!(message.destination.as_deref(), Some("src/domain/services/"));
        assert_eq!(message.action.as_deref(), Some("extract a service"));
    }

    #[test]
    fn validates_budget_order() {
        let error = Checker::new(vec![Rule::new(
            "bad",
            Selector::All,
            Budget::new(Unit::Lines, Some(10), Some(5)),
            MessageTemplate::new("bad-message", "Split it."),
        )])
        .expect_err("invalid checker");

        assert_eq!(
            error.to_string(),
            "invalid budget for rule bad: soft limit cannot be greater than hard limit"
        );
    }

    #[test]
    fn token_rules_require_token_measurements() {
        let checker = Checker::new(vec![Rule::new(
            "tokens",
            Selector::All,
            Budget::new(Unit::Tokens, Some(100), None),
            MessageTemplate::new("token-split", "Reduce token load."),
        )])
        .expect("valid checker");

        let error = checker
            .check(&measure_text("README.md", "text"))
            .expect_err("tokens are missing");

        assert_eq!(
            error.to_string(),
            "missing tokens measurement for README.md under rule tokens"
        );
    }

    #[test]
    fn selects_one_effective_rule_per_unit() {
        let checker = Checker::new(vec![
            Rule::new(
                "all-rust",
                Selector::Extension("rs".to_owned()),
                Budget::new(Unit::Lines, Some(5), Some(10)),
                MessageTemplate::new("all", "All rust."),
            ),
            Rule::new(
                "domain-rust",
                Selector::Prefix("src/domain/".to_owned()),
                Budget::new(Unit::Lines, Some(100), Some(200)),
                MessageTemplate::new("domain", "Domain rust."),
            ),
        ])
        .expect("valid checker");

        let file = measure_text("src/domain/order.rs", &"line\n".repeat(50));
        let overflows = checker.check(&file).expect("check succeeds");

        assert!(overflows.is_empty());
    }

    #[test]
    fn keeps_different_units_together() {
        let checker = Checker::new(vec![
            Rule::new(
                "bytes",
                Selector::All,
                Budget::new(Unit::Bytes, Some(5), None),
                MessageTemplate::new("bytes", "Bytes."),
            ),
            Rule::new(
                "lines",
                Selector::All,
                Budget::new(Unit::Lines, Some(2), None),
                MessageTemplate::new("lines", "Lines."),
            ),
        ])
        .expect("valid checker");

        let file = measure_text("README.md", "one\ntwo\nthree\n");
        let overflows = checker.check(&file).expect("check succeeds");

        assert_eq!(overflows.len(), 2);
        assert!(
            overflows
                .iter()
                .any(|overflow| overflow.unit == Unit::Bytes)
        );
        assert!(
            overflows
                .iter()
                .any(|overflow| overflow.unit == Unit::Lines)
        );
    }

    #[test]
    fn reports_ambiguous_same_unit_rules() {
        let checker = Checker::new(vec![
            Rule::new(
                "first",
                Selector::All,
                Budget::new(Unit::Bytes, Some(1), None),
                MessageTemplate::new("first", "First."),
            ),
            Rule::new(
                "second",
                Selector::All,
                Budget::new(Unit::Bytes, Some(2), None),
                MessageTemplate::new("second", "Second."),
            ),
        ])
        .expect("valid checker");

        let error = checker
            .check(&measure_bytes("README.md", b"abcdef"))
            .expect_err("rules are ambiguous");

        assert_eq!(
            error.to_string(),
            "ambiguous bytes rules for README.md: first, second"
        );
    }

    #[test]
    fn priority_breaks_specificity_ties() {
        let checker = Checker::new(vec![
            Rule::new(
                "strict",
                Selector::All,
                Budget::new(Unit::Bytes, Some(1), None),
                MessageTemplate::new("strict", "Strict."),
            ),
            Rule::new(
                "relaxed",
                Selector::All,
                Budget::new(Unit::Bytes, Some(100), None),
                MessageTemplate::new("relaxed", "Relaxed."),
            )
            .with_priority(10),
        ])
        .expect("valid checker");

        let overflows = checker
            .check(&measure_bytes("README.md", b"abcdef"))
            .expect("check succeeds");

        assert!(overflows.is_empty());
    }

    #[test]
    fn renders_template_values_without_replacing_inserted_placeholders() {
        let checker = Checker::new(vec![Rule::new(
            "rust",
            Selector::Exact("src/{limit}.rs".to_owned()),
            Budget::new(Unit::Lines, Some(1), None),
            MessageTemplate::new("split-rust", "Split {path}: limit {limit} {unit}."),
        )])
        .expect("valid checker");

        let file = measure_text("src/{limit}.rs", "one\ntwo\n");
        let overflows = checker.check(&file).expect("check succeeds");

        assert_eq!(
            overflows[0].message.text,
            "Split src/{limit}.rs: limit 1 lines."
        );
    }

    #[test]
    fn large_batch_smoke() {
        let checker = Checker::new(vec![Rule::new(
            "rust",
            Selector::Extension("rs".to_owned()),
            Budget::new(Unit::Lines, Some(200), Some(400)),
            MessageTemplate::new("split-rust", "Split {path}."),
        )])
        .expect("valid checker");

        let overflow_count: usize = (0..10_000)
            .map(|index| {
                let line_count = if index % 10 == 0 { 450 } else { 40 };
                let text = "fn helper() {}\n".repeat(line_count);
                let file = measure_text(format!("src/module_{index:05}.rs"), &text);
                checker.check(&file).expect("check succeeds").len()
            })
            .sum();

        assert_eq!(overflow_count, 1_000);
    }
}
