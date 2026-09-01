//! Tests for exception registry parsing, validation, and matching (§FS-003-exceptions).

use super::*;
use crate::config::Config;

const SOFT: &str = r#"
fissile_exceptions_version = 2

[[exceptions]]
path = "tests/fixtures/large.json"
match = "exact"
rules = ["fixtures"]
max_accepted = { value = 300000, unit = "bytes" }
until = "indefinite"
reason = "golden corpus copied from production incidents"
"#;

const SOFT_REGISTRY: &str = "docs/file-size-agent-exceptions.toml";
const HARD_REGISTRY: &str = "docs/file-size-human-exceptions.toml";

/// Load `text` as the soft registry, the shape most cases only need.
fn load_soft(text: &str) -> Result<Registries, ExceptionError> {
    Registries::load(Some(RegistrySource::new(SOFT_REGISTRY, text)), None)
}

fn load_both(soft: &str, hard: &str) -> Result<Registries, ExceptionError> {
    Registries::load(
        Some(RegistrySource::new(SOFT_REGISTRY, soft)),
        Some(RegistrySource::new(HARD_REGISTRY, hard)),
    )
}

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
    let registries = load_soft(SOFT).expect("loads");
    registries.validate_against(&rules()).expect("validates");
    assert_eq!(registries.soft.len(), 1);
    assert_eq!(registries.soft[0].severity, Severity::Soft);
}

#[test]
fn silences_within_ceiling_and_reports_when_exceeded() {
    let registries = load_soft(SOFT).expect("loads");
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
    let registries = load_soft(SOFT).expect("loads");
    let verdict = registries
        .verdict(Severity::Soft, "src/lib.rs", "fixtures", Unit::Bytes, 1)
        .expect("verdict");
    assert_eq!(verdict, Verdict::None);
}

#[test]
fn rejects_empty_reason() {
    let toml = r#"
fissile_exceptions_version = 2
[[exceptions]]
path = "a"
match = "exact"
rules = ["*"]
max_accepted = { value = 1, unit = "bytes" }
until = "x"
reason = "   "
"#;
    let error = load_soft(toml).expect_err("empty reason");
    assert!(matches!(error, ExceptionError::EmptyReason { .. }));
    // The entry has no name, so the message locates it: the registry file and
    // the entry's own `path` are the line to edit (§FS-003-exceptions.4).
    assert_eq!(
        error.to_string(),
        format!("{SOFT_REGISTRY}: a has an empty reason")
    );
}

/// §FS-003-exceptions.2.1: an entry that omits the field still loads, and reads
/// as deferred — the reading that keeps `until` meaningful and asserts no
/// constraint the author never claimed.
#[test]
fn undeclared_kind_reads_as_deferred() {
    let registries = load_soft(SOFT).expect("loads");
    assert_eq!(registries.soft[0].kind, Kind::Deferred);
    // The kind/until agreement is not applied to it, so `until = "indefinite"`
    // on an entry that declares no kind is not retroactively an error.
    assert_eq!(registries.soft[0].until, "indefinite");
}

/// §FS-003-exceptions.2.1: a declared kind must agree with `until`. Structural
/// never expires; deferred has to name what retires it.
#[test]
fn rejects_kind_that_disagrees_with_until() {
    let structural = SOFT.replace(
        "until = \"indefinite\"",
        "kind = \"structural\"\nuntil = \"the generator lands\"",
    );
    let error = load_soft(&structural).expect_err("dated structural");
    assert!(matches!(
        error,
        ExceptionError::KindUntilMismatch {
            kind: Kind::Structural,
            ..
        }
    ));

    let deferred = SOFT.replace("until =", "kind = \"deferred\"\nuntil =");
    let error = load_soft(&deferred).expect_err("open-ended deferred");
    assert!(matches!(
        error,
        ExceptionError::KindUntilMismatch {
            kind: Kind::Deferred,
            ..
        }
    ));
}

#[test]
fn accepts_agreeing_kind_and_counts_by_kind() {
    let toml = SOFT.replace("until =", "kind = \"structural\"\nuntil =");
    let registries = load_soft(&toml).expect("loads");
    assert_eq!(registries.soft[0].kind, Kind::Structural);
    let counts = registries.kind_counts();
    assert_eq!(counts.structural, 1);
    assert_eq!(counts.deferred, 0);
}

/// §FS-004-check-audit.2: a soft/hard twin contributes two entry totals but one
/// distinct path-expression total.
#[test]
fn kind_path_counts_deduplicate_a_path_across_registries() {
    let registries = load_both(SOFT, SOFT).expect("one deferred path in both registries");

    assert_eq!(registries.kind_counts().deferred, 2);
    assert_eq!(
        registries.kind_path_counts(),
        KindPathCounts {
            structural: 0,
            deferred: 1,
        }
    );
}

/// §FS-004-check-audit.2: structural takes precedence for a path, regardless
/// of which registry supplies it first.
#[test]
fn structural_path_precedence_is_independent_of_registry_order() {
    let structural = SOFT.replace(
        "until = \"indefinite\"",
        "kind = \"structural\"\nuntil = \"indefinite\"",
    );
    let deferred_first = load_both(SOFT, &structural).expect("registries load");
    let structural_first = load_both(&structural, SOFT).expect("registries load");

    let expected = KindPathCounts {
        structural: 1,
        deferred: 0,
    };
    assert_eq!(deferred_first.kind_path_counts(), expected);
    assert_eq!(structural_first.kind_path_counts(), expected);
}

/// §FS-004-check-audit.2: a repeated glob is one literal expression, not the
/// number of files it happens to match.
#[test]
fn kind_path_counts_count_repeated_glob_once_without_expansion() {
    let glob = SOFT
        .replace("tests/fixtures/large.json", "tests/fixtures/**/*.json")
        .replace("match = \"exact\"", "match = \"glob\"");
    let registries = load_both(&glob, &glob).expect("one deferred glob in both registries");

    assert_eq!(registries.kind_counts().deferred, 2);
    assert_eq!(
        registries.kind_path_counts(),
        KindPathCounts {
            structural: 0,
            deferred: 1,
        }
    );
}

#[test]
fn rejects_empty_until() {
    let toml = SOFT.replace("until = \"indefinite\"", "until = \"  \"");
    let error = load_soft(&toml).expect_err("empty until");
    assert!(matches!(error, ExceptionError::EmptyUntil { .. }));
}

/// §FS-003-exceptions.2.2: version 2 removed `id`/`replaces` rather than
/// tolerating them. A version-1 registry is refused with both migration edits
/// named, and a version-2 entry that kept a name fails on the unknown key.
#[test]
fn version_1_is_refused_and_names_both_edits() {
    let v1 = SOFT.replace("_version = 2", "_version = 1");
    let error = load_soft(&v1).expect_err("version 1 is not supported");
    let message = error.to_string();
    assert!(message.starts_with(SOFT_REGISTRY), "{message}");
    for edit in [
        "set fissile_exceptions_version = 2",
        "delete every id and replaces line",
    ] {
        assert!(message.contains(edit), "migration edit missing: {message}");
    }
}

#[test]
fn a_leftover_id_is_an_unknown_key() {
    let named = SOFT.replace("path =", "id = \"EX-001-fixture\"\npath =");
    let error = load_soft(&named).expect_err("id is no longer a field");
    assert!(matches!(error, ExceptionError::Parse { .. }));
    assert!(error.to_string().contains("id"), "{error}");
}

/// The same path in both registries is two entries making two claims at two
/// severities, so a diagnostic naming only the path would be ambiguous
/// (§DF-005-exception-identity).
#[test]
fn the_same_path_in_both_registries_is_two_entries() {
    let broken = SOFT.replace(
        "reason = \"golden corpus copied from production incidents\"",
        "reason = \"  \"",
    );
    let registries = Registries::load(
        Some(RegistrySource::new(SOFT_REGISTRY, SOFT)),
        Some(RegistrySource::new(HARD_REGISTRY, SOFT)),
    )
    .expect("one path, two registries");
    assert_eq!(registries.soft.len(), 1);
    assert_eq!(registries.hard.len(), 1);

    let error = Registries::load(
        Some(RegistrySource::new(SOFT_REGISTRY, SOFT)),
        Some(RegistrySource::new(HARD_REGISTRY, &broken)),
    )
    .expect_err("the hard copy has an empty reason");
    assert!(error.to_string().starts_with(HARD_REGISTRY));
}

#[test]
fn rejects_unknown_rule() {
    let toml = SOFT.replace("\"fixtures\"", "\"nope\"");
    let registries = load_soft(&toml).expect("loads");
    let error = registries
        .validate_against(&rules())
        .expect_err("unknown rule");
    assert!(matches!(error, ExceptionError::UnknownRule { .. }));
}

#[test]
fn rejects_max_below_limit() {
    let toml = SOFT.replace("value = 300000", "value = 1000");
    let registries = load_soft(&toml).expect("loads");
    let error = registries
        .validate_against(&rules())
        .expect_err("below soft limit");
    assert!(matches!(error, ExceptionError::BelowLimit { .. }));
}

#[test]
fn rejects_unit_mismatch() {
    let toml = SOFT.replace("unit = \"bytes\"", "unit = \"lines\"");
    let registries = load_soft(&toml).expect("loads");
    let error = registries
        .validate_against(&rules())
        .expect_err("unit mismatch");
    assert!(matches!(error, ExceptionError::UnitMismatch { .. }));
}

#[test]
fn reports_multiple_matches_as_schema_error() {
    let toml = r#"
fissile_exceptions_version = 2
[[exceptions]]
path = "tests/**"
match = "glob"
rules = ["fixtures"]
max_accepted = { value = 300000, unit = "bytes" }
until = "x"
reason = "first"
[[exceptions]]
path = "tests/fixtures/**"
match = "glob"
rules = ["fixtures"]
max_accepted = { value = 300000, unit = "bytes" }
until = "x"
reason = "second"
"#;
    let registries = load_soft(toml).expect("loads");
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
    let registries = load_soft(SOFT).expect("loads");
    let stale = registries.stale(&["src/lib.rs".to_owned()]);
    assert_eq!(stale.len(), 1);
    let live = registries.stale(&["tests/fixtures/large.json".to_owned()]);
    assert!(live.is_empty());
}
