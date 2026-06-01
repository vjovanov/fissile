//! The published JSON schema and the bytes `fissile` actually emits stay in
//! lockstep (§GOAL-003-friendly-output.1, §GOAL-004-token-thrift.1). A new or
//! renamed field that is not reflected in `schema/` fails here.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use fissile::Severity;
use fissile::audit::{self, AuditOptions};
use fissile::check::{self, CheckOptions};
use fissile::cli::Format;
use fissile::exception::{self, AddOptions};
use fissile::exceptions::MatchKind;

/// Required keys on every finding record (§FS-004-check-audit.1).
const REQUIRED: &[&str] = &[
    "path",
    "unit",
    "actual",
    "limit",
    "severity",
    "rule_id",
    "message_id",
    "message",
];
/// Extra keys only silenced `audit` records carry (§FS-003-exceptions.5).
const SILENCED_EXTRA: &[&str] = &["exception_id", "exception_max"];

// Values in the fixture are free of `,` and `:` so a flat object splits cleanly.
const CONFIG: &str = r#"
fissile_config_version = 1
[scan]
include = ["src"]
exclude = []
respect_gitignore = false
[[messages]]
id = "m"
text = "Split the file."
[[rules]]
id = "rust"
include = ["src/**/*.rs"]
unit = "lines"
soft = 100
hard = 200
message = "m"
"#;

fn schema_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("schema")
}

fn temp_repo() -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("fissile-schema-{}-{n}", std::process::id()));
    fs::create_dir_all(dir.join(".agents")).unwrap();
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join(".agents/fissile.toml"), CONFIG).unwrap();
    let body: String = (0..250).map(|i| format!("fn f{i}() {{}}\n")).collect();
    fs::write(dir.join("src/big.rs"), body).unwrap();
    dir
}

/// Top-level keys of one flat JSON object whose values contain no `,` or `:`.
fn object_keys(object: &str) -> Vec<String> {
    let inner = object.trim().trim_start_matches('{').trim_end_matches('}');
    inner
        .split(',')
        .map(|pair| {
            let key = pair.split(':').next().expect("key before colon").trim();
            key.trim_matches('"').to_owned()
        })
        .collect()
}

/// Pull each `{...}` object out of a flat JSON array of flat objects.
fn array_objects(array: &str) -> Vec<String> {
    let inner = array.trim().trim_start_matches('[').trim_end_matches(']');
    if inner.trim().is_empty() {
        return Vec::new();
    }
    inner
        .split("},")
        .map(|chunk| {
            let chunk = chunk.trim();
            if chunk.ends_with('}') {
                chunk.to_owned()
            } else {
                format!("{chunk}}}")
            }
        })
        .collect()
}

#[test]
fn schema_declares_every_finding_field() {
    let finding = fs::read_to_string(schema_dir().join("finding.schema.json")).unwrap();
    for field in REQUIRED.iter().chain(SILENCED_EXTRA) {
        assert!(
            finding.contains(&format!("\"{field}\"")),
            "schema/finding.schema.json is missing field `{field}`"
        );
    }
    // The check/audit schemas reference the shared finding shape.
    let check = fs::read_to_string(schema_dir().join("check.schema.json")).unwrap();
    let audit = fs::read_to_string(schema_dir().join("audit.schema.json")).unwrap();
    assert!(check.contains("finding.schema.json"));
    assert!(audit.contains("finding.schema.json"));
}

#[test]
fn check_json_records_match_the_schema() {
    let root = temp_repo();
    let run = check::run(&CheckOptions {
        root,
        config_path: None,
        staged: false,
        format: Some(Format::Json),
        no_color: false,
        paths: Vec::new(),
    })
    .expect("check runs");

    let records = array_objects(&run.output);
    assert_eq!(records.len(), 1, "one hard record for the 250-line file");
    let keys = object_keys(&records[0]);
    assert_eq!(
        sorted(&keys),
        sorted(&REQUIRED.iter().map(|s| s.to_string()).collect::<Vec<_>>()),
        "check record keys must be exactly the required finding fields"
    );
    assert_schema_known(&keys);
}

#[test]
fn audit_silenced_records_carry_documented_exception_fields() {
    let root = temp_repo();
    exception::run(&AddOptions {
        root: root.clone(),
        config_path: None,
        path: "src/big.rs".to_owned(),
        severity: Severity::Hard,
        rules: vec!["rust".to_owned()],
        reason: "accepted while splitting".to_owned(),
        until: "indefinite".to_owned(),
        match_kind: MatchKind::Exact,
        id: None,
        title: None,
        owner: None,
        issue: None,
        replaces: None,
        max: None,
        unit: None,
        dry_run: false,
    })
    .expect("exception add runs");

    let run = audit::run(&AuditOptions {
        root,
        config_path: None,
        format: Some(Format::Json),
        no_color: false,
        top: Some(1),
        stale_exceptions: true,
        rule_coverage: false,
    })
    .expect("audit runs");

    // Envelope keys present and documented.
    assert!(run.output.contains("\"findings\""));
    assert!(run.output.contains("\"silenced\""));
    assert!(run.output.contains("\"top\""));
    assert!(run.output.contains("\"stale\""));

    // The silenced hard overflow carries the exception attribution fields.
    let silenced = extract_array(&run.output, "silenced");
    let records = array_objects(&silenced);
    assert_eq!(records.len(), 1, "the hard overflow is silenced once");
    let keys = object_keys(&records[0]);
    for field in REQUIRED.iter().chain(SILENCED_EXTRA) {
        assert!(
            keys.iter().any(|k| k == field),
            "silenced record missing `{field}`"
        );
    }
    assert_schema_known(&keys);
}

/// Every emitted key must be a property the schema declares.
fn assert_schema_known(keys: &[String]) {
    let finding = fs::read_to_string(schema_dir().join("finding.schema.json")).unwrap();
    for key in keys {
        assert!(
            finding.contains(&format!("\"{key}\"")),
            "emitted field `{key}` is not declared in schema/finding.schema.json"
        );
    }
}

/// Pull the JSON array that follows `"<name>":` out of the audit envelope.
fn extract_array(envelope: &str, name: &str) -> String {
    let marker = format!("\"{name}\":");
    let start = envelope.find(&marker).expect("array present") + marker.len();
    let bytes = &envelope[start..];
    let open = bytes.find('[').expect("array open");
    let mut depth = 0;
    for (index, ch) in bytes[open..].char_indices() {
        match ch {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return bytes[open..=open + index].to_owned();
                }
            }
            _ => {}
        }
    }
    panic!("unterminated array for {name}");
}

fn sorted(items: &[String]) -> Vec<String> {
    let mut out = items.to_vec();
    out.sort();
    out
}
