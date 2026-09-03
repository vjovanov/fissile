//! `fissile limits` (§FS-010-limits): what this tree enforces, printed from the
//! configuration rather than inferred from findings. It is the one command that
//! answers with no file in hand, so a documented limit can be generated or
//! compared instead of copied by hand.

use std::path::PathBuf;

use crate::cli::{CommandError, Format};
use crate::config::Config;
use crate::json::Json;
use crate::{Rule, Selector, Severity, Unit};

/// Inputs to a `limits` run.
#[derive(Clone, Debug, Default)]
pub struct LimitsOptions {
    pub root: PathBuf,
    pub config_path: Option<PathBuf>,
    pub format: Option<Format>,
    /// Accepted so the flag means the same thing on every command, and inert:
    /// nothing here is a verdict, so nothing is tinted (§FS-010-limits.1).
    pub no_color: bool,
}

/// The rendered inventory. No `failed` and no `errors`: `limits` measures no
/// file, and a config that will not load is the one thing that can go wrong
/// (§FS-010-limits.1).
pub struct Run {
    pub output: String,
}

/// Said when the config declares no rules, because printing nothing would read
/// as a command that failed quietly rather than as a tree that enforces nothing
/// (§FS-010-limits.2).
const NO_RULES: &str = "no rules configured";

pub fn run(options: &LimitsOptions) -> Result<Run, CommandError> {
    // The config and nothing else: the registries add nothing to the answer and
    // could only stop it being given while a tree is broken (§FS-010-limits.5).
    let config = Config::load(&options.root, options.config_path.as_deref())?;
    let checker = config.to_checker()?;
    let format = options
        .format
        .unwrap_or_else(|| config.output.format.into());

    // Declaration order, unfiltered by what matched: the rule matching no file
    // is the one a reader is most likely to be wrong about (§FS-010-limits.2).
    let rules = checker.rules();
    let output = match format {
        Format::Text if rules.is_empty() => NO_RULES.to_owned(),
        Format::Text => rules.iter().map(text_line).collect::<Vec<_>>().join("\n"),
        Format::Json => Json::Object(vec![(
            "rules",
            Json::Array(rules.iter().map(rule_json).collect()),
        )])
        .render(),
    };
    Ok(Run { output })
}

/// `<id> [<include>, …] <unit> soft <N> hard <M>`, with only the thresholds the
/// rule declares — a placeholder for the other would state a limit the config
/// does not set (§FS-010-limits.3).
fn text_line(rule: &Rule) -> String {
    let mut line = format!(
        "{} [{}] {}",
        rule.id,
        include_patterns(&rule.selector).join(", "),
        rule.budget.unit
    );
    for (label, value) in [("soft", rule.budget.soft), ("hard", rule.budget.hard)] {
        if let Some(value) = value {
            line.push_str(&format!(" {label} {value}"));
        }
    }
    line
}

/// The `include` patterns as the config spells them. A config-built rule set is
/// always `Selector::Glob` (§FS-001-config.3); the other variants reach a
/// `Checker` only from a library caller, and are given the glob that selects
/// what they select, so one line has one shape.
fn include_patterns(selector: &Selector) -> Vec<String> {
    match selector {
        Selector::Glob(globs) => globs.iter().map(|glob| glob.pattern().to_owned()).collect(),
        Selector::All => vec!["**/*".to_owned()],
        Selector::Extension(extension) => {
            vec![format!("**/*.{}", extension.trim_start_matches('.'))]
        }
        Selector::Prefix(prefix) => vec![format!("{prefix}**")],
        Selector::Exact(path) => vec![path.clone()],
    }
}

/// One rule as the machine surface (§FS-010-limits.4): the text form's fields,
/// plus what a generator needs and a terminal reader does not. A field that
/// would describe nothing is omitted, never nulled.
fn rule_json(rule: &Rule) -> Json {
    let mut fields = vec![
        ("id", Json::str(rule.id.clone())),
        (
            "include",
            Json::Array(
                include_patterns(&rule.selector)
                    .into_iter()
                    .map(Json::Str)
                    .collect(),
            ),
        ),
        ("unit", Json::str(rule.budget.unit.to_string())),
    ];
    for (key, value) in [("soft", rule.budget.soft), ("hard", rule.budget.hard)] {
        if let Some(value) = value {
            fields.push((key, Json::UInt(value)));
        }
    }
    // Always present: every rule has one, and it is what settles an overlap
    // between two of them (§FS-001-config.3.2).
    fields.push(("priority", Json::Int(i64::from(rule.priority))));
    // An undeclared severity borrows the other's message template (§FS-001-config.3);
    // emitting that id would misattach guidance, so each id appears only beside its
    // own declared threshold (§FS-010-limits.4).
    for (key, declared, severity) in [
        ("soft_message", rule.budget.soft, Severity::Soft),
        ("hard_message", rule.budget.hard, Severity::Hard),
    ] {
        if declared.is_some() {
            fields.push((key, Json::str(rule.message(severity).id.clone())));
        }
    }
    // The line-counting policy says nothing about a byte or token budget
    // (§FS-010-limits.4).
    if rule.budget.unit == Unit::Lines {
        fields.push(("count_blank_lines", Json::Bool(rule.count_blank_lines)));
        fields.push(("count_comment_lines", Json::Bool(rule.count_comment_lines)));
    }
    Json::Object(fields)
}
