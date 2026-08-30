//! Unit tests for config parsing, defaults, and checker construction
//! (§FS-001-config). Kept in a sibling file so `config.rs` stays under its own
//! line budget, the way `report.rs` and `exceptions.rs` already do.

use super::*;
use crate::{Severity, Unit, measure_bytes};

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
    // Every ceiling `add` and `retune` write is a multiple of these
    // (§DF-006-quantized-ceilings.1), so a drift here is a silent change to
    // every registry fissile touches.
    assert_eq!(config.exceptions.bump.step(Unit::Lines), 100);
    assert_eq!(config.exceptions.bump.step(Unit::Bytes), 4096);
    assert_eq!(config.exceptions.bump.step(Unit::Tokens), 1000);
}

/// §FS-001-config.5: the table is per-unit and each field defaults on its own,
/// so naming one step does not silently reset the other two.
#[test]
fn parses_a_partial_bump_table() {
    let config =
        Config::parse(&format!("{SAMPLE}\n[exceptions.bump]\nlines = 25\n")).expect("valid config");
    assert_eq!(config.exceptions.bump.step(Unit::Lines), 25);
    assert_eq!(config.exceptions.bump.step(Unit::Bytes), 4096);
    assert_eq!(config.exceptions.bump.step(Unit::Tokens), 1000);
}

/// A typo in a step is a config that quantizes to something the author did not
/// choose, which is exactly the reading §DF-006-quantized-ceilings retires.
#[test]
fn rejects_an_unknown_bump_field() {
    let error = Config::parse(&format!("{SAMPLE}\n[exceptions.bump]\nline = 25\n"))
        .expect_err("a misspelled unit is rejected");
    assert!(matches!(error, ConfigError::Parse { .. }));
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

/// §FS-001-config.3: a rule may carry different guidance per severity, and
/// the shared `message` stays the fallback for the severity it omits.
#[test]
fn severity_messages_override_the_shared_message() {
    let config = Config::parse(
        r#"
fissile_config_version = 1

[[messages]]
id = "shared"
text = "Split it."

[[messages]]
id = "must"
text = """
Must split.
Ask a human when no safe split exists.
"""

[[rules]]
id = "rust"
include = ["src/**/*.rs"]
unit = "lines"
soft = 2
hard = 4
message = "shared"
hard_message = "must"
"#,
    )
    .expect("valid config");
    let checker = config.to_checker().expect("valid checker");

    let soft = checker
        .check(&crate::measure_text("src/a.rs", "a\nb\nc\n"))
        .expect("check succeeds");
    assert_eq!(soft[0].message.id, "shared");

    let hard = checker
        .check(&crate::measure_text("src/b.rs", "a\nb\nc\nd\ne\n"))
        .expect("check succeeds");
    assert_eq!(hard[0].message.id, "must");
    // The multi-line template is trimmed, so rendering starts at the text.
    assert_eq!(
        hard[0].message.text,
        "Must split.\nAsk a human when no safe split exists."
    );
}

/// §FS-001-config.3: a declared threshold with no guidance is a schema error,
/// not a silently empty message.
#[test]
fn rejects_a_threshold_with_no_message() {
    let config = Config::parse(
        r#"
fissile_config_version = 1

[[messages]]
id = "must"
text = "Must split."

[[rules]]
id = "rust"
include = ["src/**/*.rs"]
unit = "lines"
soft = 2
hard = 4
hard_message = "must"
"#,
    )
    .expect("valid config");

    let error = config.to_checker().expect_err("soft limit has no message");
    assert_eq!(
        error,
        ConfigError::MissingMessage {
            rule: "rust".to_owned(),
            severity: Severity::Soft,
        }
    );
}

/// A rule that declares only one threshold needs guidance only for it.
#[test]
fn one_sided_rule_needs_only_its_own_message() {
    let config = Config::parse(
        r#"
fissile_config_version = 1

[[messages]]
id = "must"
text = "Must split."

[[rules]]
id = "rust"
include = ["src/**/*.rs"]
unit = "lines"
hard = 2
hard_message = "must"
"#,
    )
    .expect("valid config");

    let checker = config.to_checker().expect("valid checker");
    let overflows = checker
        .check(&crate::measure_text("src/a.rs", "a\nb\nc\n"))
        .expect("check succeeds");
    assert_eq!(overflows[0].message.id, "must");
}

#[test]
fn load_names_the_file_in_parse_errors() {
    // §FS-001-config.1: every config diagnostic names its document.
    let root = std::env::temp_dir().join(format!("fissile-config-{}", std::process::id()));
    std::fs::create_dir_all(root.join(".agents")).unwrap();
    let text = "fissile_config_version = 1\nbogus = 1\n";
    std::fs::write(root.join(".agents/fissile.toml"), text).unwrap();
    let error = Config::load(&root, None).expect_err("parse error");
    let rendered = error.to_string();
    assert!(
        rendered.contains(".agents/fissile.toml") && rendered.contains("config parse error"),
        "diagnostic must name the file: {rendered}"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn rejects_unknown_keys() {
    let error = Config::parse("fissile_config_version = 1\nbogus = true\n")
        .expect_err("unknown key is rejected");
    assert!(matches!(error, ConfigError::Parse { .. }));
}

#[test]
fn rejects_unsupported_version() {
    let error = Config::parse("fissile_config_version = 2\n").expect_err("version 2 unsupported");
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
