//! Versioned TOML config loading (§FS-001-config). The config is data, not code
//! (§GOAL-002-tiny-footprint); every field is optional and falls back to a
//! default, while the file `fissile init` writes is explicit (§DF-002-explicit-config).

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use serde::Deserialize;

use crate::{Budget, Checker, FissileError, Glob, MessageTemplate, Rule, Selector, Unit};

/// The only supported major config version (§FS-001-config.1).
pub const SUPPORTED_VERSION: u32 = 1;

/// A parsed, validated config document.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// `fissile_config_version`; must equal [`SUPPORTED_VERSION`].
    pub fissile_config_version: u32,
    #[serde(default)]
    pub scan: Scan,
    #[serde(default)]
    pub output: Output,
    #[serde(default)]
    pub exceptions: Exceptions,
    #[serde(default)]
    pub tokens: Tokens,
    #[serde(default)]
    pub messages: Vec<Message>,
    #[serde(default)]
    pub rules: Vec<RuleSpec>,
}

/// `[scan]` — whole-repo audit traversal scope (§FS-001-config.2).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Scan {
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default = "default_true")]
    pub respect_gitignore: bool,
}

/// `[output]` — default output presentation (§FS-001-config.6).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Output {
    #[serde(default)]
    pub format: Format,
    #[serde(default)]
    pub color: Color,
    #[serde(default = "default_success")]
    pub success: String,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    #[default]
    Text,
    Json,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Color {
    #[default]
    Auto,
    Always,
    Never,
}

/// `[exceptions]` — registry paths and stale handling (§FS-001-config.5).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Exceptions {
    #[serde(default = "default_soft_registry")]
    pub soft_registry: String,
    #[serde(default = "default_hard_registry")]
    pub hard_registry: String,
    #[serde(default)]
    pub stale: Stale,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Stale {
    #[default]
    Warn,
    Error,
    Ignore,
}

/// `[tokens]` — opt-in token counting (§FS-001-config.7).
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Tokens {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub command: Vec<String>,
}

/// A `[[messages]]` entry (§FS-001-config.4).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Message {
    pub id: String,
    pub text: String,
}

/// A `[[rules]]` entry (§FS-001-config.3).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuleSpec {
    pub id: String,
    pub include: Vec<String>,
    pub unit: UnitSpec,
    #[serde(default)]
    pub soft: Option<u64>,
    #[serde(default)]
    pub hard: Option<u64>,
    #[serde(default)]
    pub priority: i32,
    pub message: String,
    #[serde(default)]
    pub count_blank_lines: bool,
    #[serde(default = "default_true")]
    pub count_comment_lines: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UnitSpec {
    Bytes,
    Lines,
    Tokens,
}

impl From<UnitSpec> for Unit {
    fn from(unit: UnitSpec) -> Self {
        match unit {
            UnitSpec::Bytes => Unit::Bytes,
            UnitSpec::Lines => Unit::Lines,
            UnitSpec::Tokens => Unit::Tokens,
        }
    }
}

impl Default for Scan {
    fn default() -> Self {
        Self {
            include: Vec::new(),
            exclude: Vec::new(),
            respect_gitignore: true,
        }
    }
}

impl Default for Output {
    fn default() -> Self {
        Self {
            format: Format::default(),
            color: Color::default(),
            success: default_success(),
        }
    }
}

impl Default for Exceptions {
    fn default() -> Self {
        Self {
            soft_registry: default_soft_registry(),
            hard_registry: default_hard_registry(),
            stale: Stale::default(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_success() -> String {
    "ok".to_owned()
}

fn default_soft_registry() -> String {
    "docs/file-size-agent-exceptions.toml".to_owned()
}

fn default_hard_registry() -> String {
    "docs/file-size-human-exceptions.toml".to_owned()
}

impl Config {
    /// Parse and validate a config document.
    pub fn parse(toml_text: &str) -> Result<Self, ConfigError> {
        let config: Config = toml::from_str(toml_text).map_err(|error| ConfigError::Parse {
            reason: error.message().to_owned(),
        })?;

        if config.fissile_config_version != SUPPORTED_VERSION {
            return Err(ConfigError::UnsupportedVersion {
                version: config.fissile_config_version,
            });
        }

        Ok(config)
    }

    /// Build a [`Checker`] from the rules and messages in this config.
    pub fn to_checker(&self) -> Result<Checker, ConfigError> {
        let messages: HashMap<&str, &Message> = self
            .messages
            .iter()
            .map(|message| (message.id.as_str(), message))
            .collect();

        let mut rules = Vec::with_capacity(self.rules.len());
        for spec in &self.rules {
            if spec.include.is_empty() {
                return Err(ConfigError::EmptyInclude {
                    rule: spec.id.clone(),
                });
            }

            let message = messages
                .get(spec.message.as_str())
                .ok_or_else(|| ConfigError::UnknownMessage {
                    rule: spec.id.clone(),
                    message: spec.message.clone(),
                })?;

            let selector = Selector::Glob(spec.include.iter().map(Glob::new).collect());
            let budget = Budget::new(spec.unit.into(), spec.soft, spec.hard);
            let template = MessageTemplate::new(message.id.clone(), message.text.clone());

            rules.push(
                Rule::new(spec.id.clone(), selector, budget, template)
                    .with_priority(spec.priority)
                    .with_line_policy(spec.count_blank_lines, spec.count_comment_lines),
            );
        }

        Checker::new(rules).map_err(ConfigError::Engine)
    }
}

/// A failure while loading a config document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfigError {
    Parse { reason: String },
    UnsupportedVersion { version: u32 },
    EmptyInclude { rule: String },
    UnknownMessage { rule: String, message: String },
    Engine(FissileError),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Parse { reason } => write!(f, "config parse error: {reason}"),
            ConfigError::UnsupportedVersion { version } => write!(
                f,
                "unsupported fissile_config_version {version}; this build supports version {SUPPORTED_VERSION}"
            ),
            ConfigError::EmptyInclude { rule } => {
                write!(f, "rule {rule} must list at least one include glob")
            }
            ConfigError::UnknownMessage { rule, message } => {
                write!(f, "rule {rule} references unknown message id {message}")
            }
            ConfigError::Engine(error) => write!(f, "{error}"),
        }
    }
}

impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ConfigError::Engine(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Severity, measure_bytes};

    const SAMPLE: &str = r#"
fissile_config_version = 1

[[messages]]
id = "split-rust"
text = "Split {path}."

[[rules]]
id = "rust"
include = ["src/**/*.rs"]
unit = "lines"
soft = 200
hard = 400
count_blank_lines = false
count_comment_lines = true
message = "split-rust"
"#;

    #[test]
    fn parses_defaults_when_tables_absent() {
        let config = Config::parse(SAMPLE).expect("valid config");
        assert!(config.scan.respect_gitignore);
        assert_eq!(config.output.success, "ok");
        assert_eq!(config.output.format, Format::Text);
        assert_eq!(
            config.exceptions.soft_registry,
            "docs/file-size-agent-exceptions.toml"
        );
        assert_eq!(config.exceptions.stale, Stale::Warn);
        assert!(!config.tokens.enabled);
    }

    #[test]
    fn builds_a_working_checker() {
        let config = Config::parse(SAMPLE).expect("valid config");
        let checker = config.to_checker().expect("valid checker");
        let file = crate::measure_text("src/lib.rs", &"line\n".repeat(450));
        let overflows = checker.check(&file).expect("check succeeds");
        assert_eq!(overflows.len(), 1);
        assert_eq!(overflows[0].severity, Severity::Hard);
        assert_eq!(overflows[0].rule_id, "rust");
    }

    #[test]
    fn rejects_unknown_keys() {
        let error = Config::parse("fissile_config_version = 1\nbogus = true\n")
            .expect_err("unknown key is rejected");
        assert!(matches!(error, ConfigError::Parse { .. }));
    }

    #[test]
    fn rejects_unsupported_version() {
        let error =
            Config::parse("fissile_config_version = 2\n").expect_err("version 2 unsupported");
        assert_eq!(error, ConfigError::UnsupportedVersion { version: 2 });
    }

    #[test]
    fn rejects_unknown_message_reference() {
        let toml = r#"
fissile_config_version = 1

[[rules]]
id = "rust"
include = ["src/**/*.rs"]
unit = "lines"
soft = 200
message = "missing"
"#;
        let config = Config::parse(toml).expect("parses");
        let error = config.to_checker().expect_err("dangling message id");
        assert_eq!(
            error,
            ConfigError::UnknownMessage {
                rule: "rust".to_owned(),
                message: "missing".to_owned(),
            }
        );
    }

    #[test]
    fn cross_dimension_overlap_is_ambiguous() {
        let toml = r#"
fissile_config_version = 1

[[messages]]
id = "m"
text = "Split {path}."

[[rules]]
id = "generated-rust"
include = ["src/**/*.gen.rs"]
unit = "lines"
soft = 1200
message = "m"

[[rules]]
id = "domain-rust"
include = ["src/domain/**/*.rs"]
unit = "lines"
soft = 350
message = "m"
"#;
        let checker = Config::parse(toml).unwrap().to_checker().unwrap();
        let file = crate::measure_text("src/domain/schema.gen.rs", &"x\n".repeat(10));
        let error = checker.check(&file).expect_err("overlap is ambiguous");
        assert!(matches!(error, FissileError::AmbiguousRules { .. }));
    }

    #[test]
    fn explicit_priority_resolves_overlap() {
        let toml = r#"
fissile_config_version = 1

[[messages]]
id = "m"
text = "Split {path}."

[[rules]]
id = "generated-rust"
include = ["src/**/*.gen.rs"]
unit = "lines"
soft = 1200
priority = 20
message = "m"

[[rules]]
id = "domain-rust"
include = ["src/domain/**/*.rs"]
unit = "lines"
soft = 5
message = "m"
"#;
        let checker = Config::parse(toml).unwrap().to_checker().unwrap();
        let file = crate::measure_text("src/domain/schema.gen.rs", &"x\n".repeat(10));
        // generated-rust wins on priority; its soft limit of 1200 is not crossed.
        let overflows = checker.check(&file).expect("priority breaks the tie");
        assert!(overflows.is_empty());
    }

    #[test]
    fn byte_rule_matches_via_glob() {
        let toml = r#"
fissile_config_version = 1

[[messages]]
id = "m"
text = "Large {path}."

[[rules]]
id = "large-file-default"
include = ["**/*"]
unit = "bytes"
soft = 4
message = "m"
"#;
        let checker = Config::parse(toml).unwrap().to_checker().unwrap();
        let overflows = checker
            .check(&measure_bytes("anything.bin", b"abcdef"))
            .expect("check succeeds");
        assert_eq!(overflows.len(), 1);
    }
}
