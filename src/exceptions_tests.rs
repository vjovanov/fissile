//! Tests for exception registry parsing, validation, and matching (§FS-003).

use super::*;
use crate::config::Config;

const SOFT: &str = r#"
fissile_exceptions_version = 1

[[exceptions]]
id = "EX-001-fixture"
path = "tests/fixtures/large.json"
match = "exact"
rules = ["fixtures"]
max_accepted = { value = 300000, unit = "bytes" }
until = "indefinite"
reason = "golden corpus copied from production incidents"
"#;

fn rules() -> Vec<Rule> {
    let toml = r#"
fissile_config_version = 1
[[messages]]
id = "m"
text = "Split {path}."
[[rules]]
id = "fixtures"
include = ["tests/fixtures/**"]
unit = "bytes"
soft = 65536
hard = 262144
message = "m"
"#;
    Config::parse(toml)
        .unwrap()
        .to_checker()
        .unwrap()
        .rules()
        .to_vec()
}

#[test]
fn loads_and_validates_against_rules() {
    let registries = Registries::load(Some(SOFT), None).expect("loads");
    registries.validate_against(&rules()).expect("validates");
    assert_eq!(registries.soft.len(), 1);
    assert_eq!(registries.soft[0].severity, Severity::Soft);
}

#[test]
fn silences_within_ceiling_and_reports_when_exceeded() {
    let registries = Registries::load(Some(SOFT), None).expect("loads");
    let silenced = registries
        .verdict(
            Severity::Soft,
            "tests/fixtures/large.json",
            "fixtures",
            Unit::Bytes,
            250000,
        )
        .expect("verdict");
    assert!(matches!(silenced, Verdict::Silenced(_)));

    let grew = registries
        .verdict(
            Severity::Soft,
            "tests/fixtures/large.json",
            "fixtures",
            Unit::Bytes,
            400000,
        )
        .expect("verdict");
    assert!(matches!(grew, Verdict::Exceeded(_)));
}

#[test]
fn unmatched_path_is_none() {
    let registries = Registries::load(Some(SOFT), None).expect("loads");
    let verdict = registries
        .verdict(Severity::Soft, "src/lib.rs", "fixtures", Unit::Bytes, 1)
        .expect("verdict");
    assert_eq!(verdict, Verdict::None);
}

#[test]
fn rejects_empty_reason() {
    let toml = r#"
fissile_exceptions_version = 1
[[exceptions]]
id = "EX-001-x"
path = "a"
match = "exact"
rules = ["*"]
max_accepted = { value = 1, unit = "bytes" }
until = "x"
reason = "   "
"#;
    let error = Registries::load(Some(toml), None).expect_err("empty reason");
    assert!(matches!(error, ExceptionError::EmptyReason { .. }));
}

#[test]
fn rejects_duplicate_ids_across_registries() {
    let dup = SOFT.replace("tests/fixtures/large.json", "tests/fixtures/other.json");
    let error = Registries::load(Some(SOFT), Some(&dup)).expect_err("dup id");
    assert!(matches!(error, ExceptionError::DuplicateId { .. }));
}

#[test]
fn rejects_unknown_rule() {
    let toml = SOFT.replace("\"fixtures\"", "\"nope\"");
    let registries = Registries::load(Some(&toml), None).expect("loads");
    let error = registries
        .validate_against(&rules())
        .expect_err("unknown rule");
    assert!(matches!(error, ExceptionError::UnknownRule { .. }));
}

#[test]
fn rejects_max_below_limit() {
    let toml = SOFT.replace("value = 300000", "value = 1000");
    let registries = Registries::load(Some(&toml), None).expect("loads");
    let error = registries
        .validate_against(&rules())
        .expect_err("below soft limit");
    assert!(matches!(error, ExceptionError::BelowLimit { .. }));
}

#[test]
fn rejects_unit_mismatch() {
    let toml = SOFT.replace("unit = \"bytes\"", "unit = \"lines\"");
    let registries = Registries::load(Some(&toml), None).expect("loads");
    let error = registries
        .validate_against(&rules())
        .expect_err("unit mismatch");
    assert!(matches!(error, ExceptionError::UnitMismatch { .. }));
}

#[test]
fn reports_multiple_matches_as_schema_error() {
    let toml = r#"
fissile_exceptions_version = 1
[[exceptions]]
id = "EX-001-a"
path = "tests/**"
match = "glob"
rules = ["fixtures"]
max_accepted = { value = 300000, unit = "bytes" }
until = "x"
reason = "first"
[[exceptions]]
id = "EX-002-b"
path = "tests/fixtures/**"
match = "glob"
rules = ["fixtures"]
max_accepted = { value = 300000, unit = "bytes" }
until = "x"
reason = "second"
"#;
    let registries = Registries::load(Some(toml), None).expect("loads");
    let error = registries
        .verdict(
            Severity::Soft,
            "tests/fixtures/large.json",
            "fixtures",
            Unit::Bytes,
            1,
        )
        .expect_err("multiple matches");
    assert!(matches!(error, ExceptionError::MultipleMatches { .. }));
}

#[test]
fn reports_stale_entries() {
    let registries = Registries::load(Some(SOFT), None).expect("loads");
    let stale = registries.stale(&["src/lib.rs".to_owned()]);
    assert_eq!(stale.len(), 1);
    let live = registries.stale(&["tests/fixtures/large.json".to_owned()]);
    assert!(live.is_empty());
}
