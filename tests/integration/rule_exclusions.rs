//! Cross-surface coverage for rule-local negative scope (§FS-001-config.3.4).

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use fissile::audit::{self, AuditOptions};
use fissile::cli::Format;
use fissile::config::Config;
use fissile::init::DEFAULT_CONFIG;
use fissile::limits::{self, LimitsOptions};
use fissile::measure_text;

const PREFIX: &str = r#"
fissile_config_version = 1

[scan]
include = ["src"]
exclude = []
respect_gitignore = false

[[messages]]
id = "m"
text = "Split it."
"#;

fn temp_repo(config: &str) -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "fissile-rule-exclusions-{}-{n}",
        std::process::id()
    ));
    fs::create_dir_all(root.join(".agents")).unwrap();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join(".agents/fissile.toml"), config).unwrap();
    root
}

fn one_rule(exclude_line: &str) -> String {
    format!(
        r#"{PREFIX}
[[rules]]
id = "rust"
include = ["src/**/*.rs"]
{exclude_line}unit = "lines"
soft = 1
message = "m"
"#
    )
}

/// Omission and an explicit empty list compile to the same behavior and remain
/// absent from both inventories (§FS-001-config.3.4, §FS-010-limits.3).
#[test]
fn omitted_and_empty_exclusions_are_compatible() {
    let omitted = one_rule("");
    let empty = one_rule("exclude = []\n");
    assert_eq!(
        Config::parse(&omitted).unwrap(),
        Config::parse(&empty).unwrap()
    );

    for format in [Format::Text, Format::Json] {
        let outputs: Vec<_> = [omitted.as_str(), empty.as_str()]
            .into_iter()
            .map(|config| {
                limits::run(&LimitsOptions {
                    root: temp_repo(config),
                    format: Some(format),
                    ..LimitsOptions::default()
                })
                .unwrap()
                .output
            })
            .collect();
        assert_eq!(outputs[0], outputs[1]);
        assert!(!outputs[0].contains("exclude"));
    }
}

/// Exhaustive generated and example configs state the new empty default rather
/// than making their reader reconstruct it (§DF-002-explicit-config).
#[test]
fn exhaustive_configs_spell_empty_rule_exclusions() {
    let example =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/fissile.toml"))
            .unwrap();
    for (name, source) in [("generated", DEFAULT_CONFIG), ("example", &example)] {
        let rule_count = Config::parse(source).unwrap().rules.len();
        assert_eq!(
            source.matches("\nexclude = []\n").count(),
            rule_count,
            "{name} config must spell the default on every rule"
        );
    }
}

/// Negative scope is decided before priority: an excluded high-priority rule
/// cannot hide a remaining lower-priority budget (§FS-001-config.3.4).
#[test]
fn excluded_high_priority_rule_cannot_win() {
    let config = format!(
        r#"{PREFIX}
[[rules]]
id = "high"
include = ["src/**/*.rs"]
exclude = ["src/kept.rs"]
unit = "lines"
soft = 100
priority = 100
message = "m"

[[rules]]
id = "remaining"
include = ["src/**/*.rs"]
unit = "lines"
soft = 1
message = "m"
"#
    );
    let checker = Config::parse(&config).unwrap().to_checker().unwrap();
    let findings = checker
        .check(&measure_text("src/kept.rs", "one\ntwo\n"))
        .unwrap();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "remaining");
}

/// An excluded member leaves no same-unit tie for ambiguity resolution to
/// reject (§FS-001-config.3.4).
#[test]
fn excluded_rule_cannot_create_an_ambiguity() {
    let config = format!(
        r#"{PREFIX}
[[rules]]
id = "excluded"
include = ["src/**/*.rs"]
exclude = ["src/kept.rs"]
unit = "lines"
soft = 1
message = "m"

[[rules]]
id = "remaining"
include = ["src/**/*.rs"]
unit = "lines"
soft = 1
message = "m"
"#
    );
    let checker = Config::parse(&config).unwrap().to_checker().unwrap();
    let findings = checker
        .check(&measure_text("src/kept.rs", "one\ntwo\n"))
        .expect("the excluded rule is not an ambiguity candidate");
    assert_eq!(findings[0].rule_id, "remaining");
}

/// Audit coverage uses the checker's applicability decision: an excluded rule
/// is unmatched and the file remains reachable through the catch-all
/// (§FS-001-config.3.4).
#[test]
fn audit_coverage_respects_rule_exclusions() {
    let config = format!(
        r#"{PREFIX}
[[rules]]
id = "bytes"
include = ["**/*"]
unit = "bytes"
soft = 1000
message = "m"

[[rules]]
id = "rust"
include = ["src/**/*.rs"]
exclude = ["src/kept.rs"]
unit = "lines"
soft = 1000
message = "m"
"#
    );
    let root = temp_repo(&config);
    fs::write(root.join("src/kept.rs"), "fn kept() {}\n").unwrap();
    let output = audit::run(&AuditOptions {
        root,
        format: Some(Format::Text),
        rule_coverage: true,
        ..AuditOptions::default()
    })
    .unwrap()
    .output;

    assert!(output.contains("rules matching no file: rust"), "{output}");
    assert!(
        output.contains("files only under catch-all: src/kept.rs"),
        "{output}"
    );
}

/// The published inventory schema admits the optional field, whose JSON value
/// follows `include` and preserves declaration order (§FS-010-limits.4).
#[test]
fn limits_schema_declares_rule_exclusions() {
    let config = one_rule("exclude = [\"src/generated.rs\"]\n");
    let output = limits::run(&LimitsOptions {
        root: temp_repo(&config),
        format: Some(Format::Json),
        ..LimitsOptions::default()
    })
    .unwrap()
    .output;
    assert!(
        output.starts_with(
            r#"{"rules":[{"id":"rust","include":["src/**/*.rs"],"exclude":["src/generated.rs"],"unit":"lines""#
        ),
        "{output}"
    );

    let schema = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("schema/limits.schema.json"),
    )
    .unwrap();
    assert!(schema.contains(r#""exclude": {"#));
    assert!(!schema.contains(r#""required": ["id", "include", "exclude""#));
}
