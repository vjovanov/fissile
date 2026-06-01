//! Exception registries (§FS-003-exceptions): typed, reviewable rationales that
//! accept an oversized file. Severity comes from which registry an entry lives
//! in, not a field; each entry records the largest accepted measurement.

use std::error::Error;
use std::fmt;

use serde::Deserialize;

use crate::config::UnitSpec;
use crate::{Glob, Rule, Severity, Unit};

/// The only supported registry version (§FS-003-exceptions.1).
pub const SUPPORTED_VERSION: u32 = 1;

/// Whether an entry's `path` is an exact repo-relative path or a glob.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MatchKind {
    Exact,
    Glob,
}

/// `max_accepted = { value, unit }` — the ceiling this entry accepts.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MaxAccepted {
    pub value: u64,
    pub unit: UnitSpec,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RegistryFile {
    fissile_exceptions_version: u32,
    #[serde(default)]
    exceptions: Vec<RawException>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawException {
    id: String,
    path: String,
    #[serde(rename = "match")]
    match_kind: MatchKind,
    rules: Vec<String>,
    max_accepted: MaxAccepted,
    until: String,
    reason: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    owner: Option<String>,
    #[serde(default)]
    issue: Option<String>,
    #[serde(default)]
    replaces: Option<String>,
}

/// A parsed, structurally-valid exception entry with a compiled path matcher.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Exception {
    pub id: String,
    pub severity: Severity,
    pub path: String,
    pub match_kind: MatchKind,
    pub rules: Vec<String>,
    pub max_value: u64,
    pub max_unit: Unit,
    pub until: String,
    pub reason: String,
    pub title: Option<String>,
    pub owner: Option<String>,
    pub issue: Option<String>,
    pub replaces: Option<String>,
    matcher: Matcher,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Matcher {
    Exact(String),
    Glob(Glob),
}

impl Exception {
    /// Whether the entry's `["*"]` wildcard or explicit list covers `rule_id`.
    pub fn applies_to_rule(&self, rule_id: &str) -> bool {
        self.rules.iter().any(|r| r == "*" || r == rule_id)
    }

    /// Whether the entry's path matcher covers a repo-relative `/`-path.
    pub fn matches_path(&self, path: &str) -> bool {
        match &self.matcher {
            Matcher::Exact(expected) => expected == path,
            Matcher::Glob(glob) => glob.matches(path),
        }
    }
}

/// How an overflow relates to the exception registry of its severity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict<'a> {
    /// No entry matches the `(path, rule, unit)` condition.
    None,
    /// An entry matches and the measurement is within its accepted ceiling.
    Silenced(&'a Exception),
    /// An entry matches but the file grew past its ceiling, so the finding stands.
    Exceeded(&'a Exception),
}

/// Both severity registries, loaded and structurally validated.
#[derive(Clone, Debug, Default)]
pub struct Registries {
    pub soft: Vec<Exception>,
    pub hard: Vec<Exception>,
}

impl Registries {
    /// Parse the soft and hard registry documents. A `None` text means the
    /// registry file is absent, which is treated as empty.
    pub fn load(soft: Option<&str>, hard: Option<&str>) -> Result<Self, ExceptionError> {
        let soft = match soft {
            Some(text) => parse_registry(text, Severity::Soft, "soft")?,
            None => Vec::new(),
        };
        let hard = match hard {
            Some(text) => parse_registry(text, Severity::Hard, "hard")?,
            None => Vec::new(),
        };
        let registries = Self { soft, hard };
        registries.check_unique_ids()?;
        Ok(registries)
    }

    fn check_unique_ids(&self) -> Result<(), ExceptionError> {
        let mut seen = std::collections::HashSet::new();
        for entry in self.all() {
            if !seen.insert(entry.id.as_str()) {
                return Err(ExceptionError::DuplicateId {
                    id: entry.id.clone(),
                });
            }
        }
        Ok(())
    }

    /// Every entry across both registries.
    pub fn all(&self) -> impl Iterator<Item = &Exception> {
        self.soft.iter().chain(self.hard.iter())
    }

    fn registry(&self, severity: Severity) -> &[Exception] {
        match severity {
            Severity::Soft => &self.soft,
            Severity::Hard => &self.hard,
        }
    }

    /// Validate every entry against the configured rules (§FS-003-exceptions.4):
    /// listed rules exist, share the entry's unit, carry the relevant severity
    /// limit, and the accepted ceiling is at least that limit.
    pub fn validate_against(&self, rules: &[Rule]) -> Result<(), ExceptionError> {
        for entry in self.all() {
            // Mixed-unit rule lists are rejected for explicit lists too: a single
            // entry may only silence rules that share its unit (§FS-003.3).
            for rule_id in &entry.rules {
                if rule_id == "*" {
                    continue;
                }
                let Some(rule) = rules.iter().find(|r| &r.id == rule_id) else {
                    return Err(ExceptionError::UnknownRule {
                        id: entry.id.clone(),
                        rule: rule_id.clone(),
                    });
                };
                if rule.budget.unit != entry.max_unit {
                    return Err(ExceptionError::UnitMismatch {
                        id: entry.id.clone(),
                        rule: rule_id.clone(),
                    });
                }
                let limit = match entry.severity {
                    Severity::Soft => rule.budget.soft,
                    Severity::Hard => rule.budget.hard,
                };
                let Some(limit) = limit else {
                    return Err(ExceptionError::NoSeverityLimit {
                        id: entry.id.clone(),
                        rule: rule_id.clone(),
                        severity: entry.severity,
                    });
                };
                if entry.max_value < limit {
                    return Err(ExceptionError::BelowLimit {
                        id: entry.id.clone(),
                        rule: rule_id.clone(),
                        max: entry.max_value,
                        limit,
                    });
                }
            }
        }
        Ok(())
    }

    /// Resolve how the severity-matching registry treats one overflow. Returns a
    /// schema error when more than one entry matches the same `(path, rule, unit)`
    /// condition (§FS-003-exceptions.3).
    pub fn verdict(
        &self,
        severity: Severity,
        path: &str,
        rule_id: &str,
        unit: Unit,
        actual: u64,
    ) -> Result<Verdict<'_>, ExceptionError> {
        let mut matched: Option<&Exception> = None;
        for entry in self.registry(severity) {
            if entry.max_unit == unit && entry.applies_to_rule(rule_id) && entry.matches_path(path)
            {
                if matched.is_some() {
                    return Err(ExceptionError::MultipleMatches {
                        path: path.to_owned(),
                        rule: rule_id.to_owned(),
                        unit,
                    });
                }
                matched = Some(entry);
            }
        }

        Ok(match matched {
            None => Verdict::None,
            Some(entry) if actual <= entry.max_value => Verdict::Silenced(entry),
            Some(entry) => Verdict::Exceeded(entry),
        })
    }

    /// Entries whose path/glob matches none of `scanned` (§FS-004-check-audit.2).
    pub fn stale<'a>(&'a self, scanned: &[String]) -> Vec<&'a Exception> {
        self.all()
            .filter(|entry| !scanned.iter().any(|path| entry.matches_path(path)))
            .collect()
    }
}

fn parse_registry(
    text: &str,
    severity: Severity,
    label: &str,
) -> Result<Vec<Exception>, ExceptionError> {
    let file: RegistryFile = toml::from_str(text).map_err(|error| ExceptionError::Parse {
        registry: label.to_owned(),
        reason: error.message().to_owned(),
    })?;

    if file.fissile_exceptions_version != SUPPORTED_VERSION {
        return Err(ExceptionError::UnsupportedVersion {
            registry: label.to_owned(),
            version: file.fissile_exceptions_version,
        });
    }

    file.exceptions
        .into_iter()
        .map(|raw| build_exception(raw, severity))
        .collect()
}

fn build_exception(raw: RawException, severity: Severity) -> Result<Exception, ExceptionError> {
    if raw.reason.trim().is_empty() {
        return Err(ExceptionError::EmptyReason { id: raw.id });
    }
    if raw.max_accepted.value == 0 {
        return Err(ExceptionError::NonPositiveMax { id: raw.id });
    }
    if raw.rules.is_empty() {
        return Err(ExceptionError::NoRules { id: raw.id });
    }

    let matcher = match raw.match_kind {
        MatchKind::Exact => Matcher::Exact(raw.path.clone()),
        MatchKind::Glob => Matcher::Glob(Glob::new(raw.path.clone())),
    };

    Ok(Exception {
        id: raw.id,
        severity,
        path: raw.path,
        match_kind: raw.match_kind,
        rules: raw.rules,
        max_value: raw.max_accepted.value,
        max_unit: raw.max_accepted.unit.into(),
        until: raw.until,
        reason: raw.reason,
        title: raw.title,
        owner: raw.owner,
        issue: raw.issue,
        replaces: raw.replaces,
        matcher,
    })
}

/// A failure while loading or validating an exception registry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExceptionError {
    Parse {
        registry: String,
        reason: String,
    },
    UnsupportedVersion {
        registry: String,
        version: u32,
    },
    EmptyReason {
        id: String,
    },
    NonPositiveMax {
        id: String,
    },
    NoRules {
        id: String,
    },
    DuplicateId {
        id: String,
    },
    UnknownRule {
        id: String,
        rule: String,
    },
    UnitMismatch {
        id: String,
        rule: String,
    },
    NoSeverityLimit {
        id: String,
        rule: String,
        severity: Severity,
    },
    BelowLimit {
        id: String,
        rule: String,
        max: u64,
        limit: u64,
    },
    MultipleMatches {
        path: String,
        rule: String,
        unit: Unit,
    },
}

impl fmt::Display for ExceptionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExceptionError::Parse { registry, reason } => {
                write!(f, "{registry} exception registry parse error: {reason}")
            }
            ExceptionError::UnsupportedVersion { registry, version } => write!(
                f,
                "{registry} exception registry version {version} is unsupported; this build supports {SUPPORTED_VERSION}"
            ),
            ExceptionError::EmptyReason { id } => {
                write!(f, "exception {id} has an empty reason")
            }
            ExceptionError::NonPositiveMax { id } => {
                write!(
                    f,
                    "exception {id} max_accepted.value must be a positive integer"
                )
            }
            ExceptionError::NoRules { id } => {
                write!(f, "exception {id} must list at least one rule id")
            }
            ExceptionError::DuplicateId { id } => {
                write!(f, "exception id {id} is declared more than once")
            }
            ExceptionError::UnknownRule { id, rule } => {
                write!(f, "exception {id} references unknown rule id {rule}")
            }
            ExceptionError::UnitMismatch { id, rule } => write!(
                f,
                "exception {id} max_accepted.unit does not match the unit of rule {rule}"
            ),
            ExceptionError::NoSeverityLimit { id, rule, severity } => write!(
                f,
                "exception {id} targets rule {rule}, which has no {severity} limit to accept"
            ),
            ExceptionError::BelowLimit {
                id,
                rule,
                max,
                limit,
            } => write!(
                f,
                "exception {id} max_accepted.value {max} is below rule {rule} limit {limit}"
            ),
            ExceptionError::MultipleMatches { path, rule, unit } => write!(
                f,
                "more than one exception in the same registry matches {path} for {unit} rule {rule}"
            ),
        }
    }
}

impl Error for ExceptionError {}

#[cfg(test)]
#[path = "exceptions_tests.rs"]
mod tests;
