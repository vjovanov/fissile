//! End-to-end coverage for the library command surfaces: `check`, `audit`, and
//! `exception add` (§FS-004-check-audit, §FS-005-exception-add).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use fissile::audit::{self, AuditOptions};
use fissile::check::{self, CheckOptions};
use fissile::cli::Format;
use fissile::exception::{self, AddOptions};
use fissile::exceptions::{Kind, MatchKind};
use fissile::{Severity, Unit};

const CONFIG: &str = r#"
fissile_config_version = 1
[scan]
include = ["src"]
exclude = []
respect_gitignore = false
[[messages]]
id = "m"
text = "Split {path}."
[[rules]]
id = "rust"
include = ["src/**/*.rs"]
unit = "lines"
soft = 100
hard = 200
message = "m"
"#;

fn temp_repo() -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("fissile-it-{}-{n}", std::process::id()));
    fs::create_dir_all(dir.join(".agents")).unwrap();
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join(".agents/fissile.toml"), CONFIG).unwrap();
    fs::write(dir.join("src/big.rs"), rust_lines(250)).unwrap();
    fs::write(dir.join("src/ok.rs"), "fn ok() {}\n").unwrap();
    dir
}

fn rust_lines(count: usize) -> String {
    (0..count).map(|i| format!("fn f{i}() {{}}\n")).collect()
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .expect("git command runs");
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}

fn check_options(root: &Path) -> CheckOptions {
    CheckOptions {
        root: root.to_path_buf(),
        config_path: None,
        staged: false,
        format: None,
        no_color: false,
        paths: Vec::new(),
    }
}

#[test]
fn check_reports_hard_overflow_and_fails() {
    let root = temp_repo();
    let run = check::run(&check_options(&root)).expect("check runs");
    assert!(run.failed, "a 250-line file crosses the hard limit");
    assert!(run.output.contains("src/big.rs"));
    assert!(run.output.contains("hard: 1 file over the"));
    assert!(run.output.contains("[rule: rust, message:"));
}

/// Spec: `docs/functional-spec/FS-004-check-audit.md#1-check`.
#[test]
fn check_normalizes_explicit_paths_to_repo_relative_form() {
    let root = temp_repo();
    for path in [
        "./src/big.rs".to_owned(),
        root.join("src/big.rs").to_string_lossy().into_owned(),
    ] {
        let mut options = check_options(&root);
        options.paths = vec![path];
        let run = check::run(&options).expect("check runs");
        assert!(run.failed);
        assert!(run.output.contains("src/big.rs"));
        assert!(run.output.contains("hard: 1 file over the"));
        assert!(run.output.contains("[rule: rust, message:"));
    }
}

/// Spec: `docs/functional-spec/FS-004-check-audit.md#1-check`.
#[test]
fn staged_check_measures_the_staged_blob() {
    let root = temp_repo();
    git(&root, &["init"]);
    fs::write(root.join("src/big.rs"), rust_lines(250)).unwrap();
    git(&root, &["add", "src/big.rs"]);
    fs::write(root.join("src/big.rs"), rust_lines(10)).unwrap();

    let mut options = check_options(&root);
    options.staged = true;
    let run = check::run(&options).expect("check runs");

    assert!(run.failed);
    assert!(run.output.contains("src/big.rs"));
    assert!(run.output.contains("hard: 1 file over the"));
    assert!(run.output.contains("[rule: rust, message:"));
}

#[test]
fn hard_exception_silences_hard_but_keeps_soft() {
    let root = temp_repo();
    exception::run(&AddOptions {
        root: root.clone(),
        config_path: None,
        path: "./src/big.rs".to_owned(),
        severity: Severity::Hard,
        rules: vec!["rust".to_owned()],
        kind: Kind::Deferred,
        reason: "no module owns the generated cases yet".to_owned(),
        until: Some("the case-builder module lands".to_owned()),
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

    // Specs: `docs/functional-spec/FS-003-exceptions.md#3-matching` and
    // `docs/functional-spec/FS-005-exception-add.md#3-generated-entry`.
    let registry = fs::read_to_string(root.join("docs/file-size-human-exceptions.toml")).unwrap();
    assert!(registry.contains("path = \"src/big.rs\""));
    assert!(registry.contains("kind = \"deferred\""));

    let run = check::run(&check_options(&root)).expect("check runs");
    assert!(!run.failed, "hard overflow is now accepted");
    // The soft finding survives so agents keep minimizing (§FS-003-exceptions.3).
    assert!(run.output.contains("soft: 1 file over the"));
    assert!(run.output.contains("[rule: rust, message:"));
}

#[test]
fn check_json_emits_records_or_empty_array() {
    let root = temp_repo();
    let mut options = check_options(&root);
    options.format = Some(Format::Json);
    let run = check::run(&options).expect("check runs");
    assert!(run.output.starts_with('['));
    assert!(run.output.contains("\"rule_id\":\"rust\""));
    assert!(run.output.contains("\"severity\":\"hard\""));
}

#[test]
fn check_uses_configured_format_default() {
    let root = temp_repo();
    let json_default = CONFIG.replace("[scan]", "[output]\nformat = \"json\"\n\n[scan]");
    fs::write(root.join(".agents/fissile.toml"), json_default).unwrap();

    let run = check::run(&check_options(&root)).expect("check runs");
    assert!(run.output.starts_with('['));
    assert!(run.output.contains("\"rule_id\":\"rust\""));
}

#[test]
fn color_is_emitted_only_when_enabled() {
    let root = temp_repo();
    // Flip the config to always-color so the result does not depend on a TTY.
    let colored = CONFIG.replace("[scan]", "[output]\ncolor = \"always\"\n\n[scan]");
    fs::write(root.join(".agents/fissile.toml"), colored).unwrap();

    let mut options = check_options(&root);
    let run = check::run(&options).expect("check runs");
    assert!(
        run.output.contains('\u{1b}'),
        "always-color emits ANSI codes"
    );

    options.no_color = true;
    let run = check::run(&options).expect("check runs");
    assert!(
        !run.output.contains('\u{1b}'),
        "--no-color forces plain output"
    );
}

#[test]
fn audit_top_ranks_largest_files() {
    let root = temp_repo();
    let run = audit::run(&AuditOptions {
        root: root.clone(),
        config_path: None,
        format: Some(Format::Text),
        no_color: false,
        top: Some(2),
        stale_exceptions: true,
        rule_coverage: true,
    })
    .expect("audit runs");
    assert!(run.failed, "the oversized file is a hard overflow");
    assert!(run.output.contains("top lines:"));
    assert!(!run.output.contains("top tokens:"));
    assert!(run.output.contains("src/big.rs"));
    assert!(run.output.contains("stale exceptions:"));
    assert!(run.output.contains("rule coverage:"));
}

#[test]
fn audit_uses_configured_format_default() {
    let root = temp_repo();
    let json_default = CONFIG.replace("[scan]", "[output]\nformat = \"json\"\n\n[scan]");
    fs::write(root.join(".agents/fissile.toml"), json_default).unwrap();

    let run = audit::run(&AuditOptions {
        root: root.clone(),
        config_path: None,
        format: None,
        no_color: false,
        top: None,
        stale_exceptions: false,
        rule_coverage: false,
    })
    .expect("audit runs");

    assert!(run.output.starts_with('{'));
    assert!(run.output.contains("\"findings\""));
}

#[test]
fn audit_json_top_omits_unmeasured_units() {
    let root = temp_repo();
    let run = audit::run(&AuditOptions {
        root,
        config_path: None,
        format: Some(Format::Json),
        no_color: false,
        top: Some(2),
        stale_exceptions: false,
        rule_coverage: false,
    })
    .expect("audit runs");

    assert!(run.output.contains("\"unit\":\"lines\""));
    assert!(!run.output.contains("\"unit\":\"tokens\""));
}

#[test]
fn exception_add_rejects_overlapping_path_matchers() {
    let root = temp_repo();
    exception::run(&AddOptions {
        root: root.clone(),
        config_path: None,
        path: "src/**".to_owned(),
        severity: Severity::Hard,
        rules: vec!["rust".to_owned()],
        kind: Kind::Structural,
        reason: "generated tree asserted byte-identical by the snapshot test".to_owned(),
        until: None,
        match_kind: MatchKind::Glob,
        id: Some("EX-001-src".to_owned()),
        title: None,
        owner: None,
        issue: None,
        replaces: None,
        max: Some(300),
        unit: Some(Unit::Lines),
        dry_run: false,
    })
    .expect("glob exception add runs");

    let error = match exception::run(&AddOptions {
        root: root.clone(),
        config_path: None,
        path: "src/big.rs".to_owned(),
        severity: Severity::Hard,
        rules: vec!["rust".to_owned()],
        kind: Kind::Deferred,
        reason: "accepted exact file".to_owned(),
        until: Some("the case-builder module lands".to_owned()),
        match_kind: MatchKind::Exact,
        id: Some("EX-002-big".to_owned()),
        title: None,
        owner: None,
        issue: None,
        replaces: None,
        max: None,
        unit: None,
        dry_run: false,
    }) {
        Ok(_) => panic!("overlapping exception should be rejected"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("already accepts src/big.rs"));
}

fn add_options(root: &Path, kind: Kind, until: Option<&str>) -> AddOptions {
    AddOptions {
        root: root.to_path_buf(),
        config_path: None,
        path: "src/big.rs".to_owned(),
        severity: Severity::Hard,
        rules: vec!["rust".to_owned()],
        kind,
        reason: "a reason".to_owned(),
        until: until.map(str::to_owned),
        match_kind: MatchKind::Exact,
        id: None,
        title: None,
        owner: None,
        issue: None,
        replaces: None,
        max: None,
        unit: None,
        dry_run: false,
    }
}

/// §FS-005-exception-add.1: the kind decides what `until` may say. A structural
/// entry never expires and a deferred one must name what retires it, so each
/// wrong pairing is refused with the other kind named as the likely fix.
#[test]
fn exception_add_reconciles_kind_with_until() {
    let root = temp_repo();

    // Structural with no --until takes the `indefinite` default, and writes both
    // fields rather than leaving one to be inferred (§FS-005-exception-add.3).
    let run = exception::run(&add_options(&root, Kind::Structural, None)).expect("structural adds");
    assert!(run.output.contains("appended"));
    let registry = fs::read_to_string(root.join("docs/file-size-human-exceptions.toml")).unwrap();
    assert!(registry.contains("kind = \"structural\""));
    assert!(registry.contains("until = \"indefinite\""));

    let dated = exception::run(&add_options(
        &root,
        Kind::Structural,
        Some("the parser lands"),
    ))
    .expect_err("a structural entry cannot have a retirement condition");
    assert!(dated.to_string().contains("--kind deferred"));

    let open_ended = exception::run(&add_options(&root, Kind::Deferred, Some("indefinite")))
        .expect_err("deferred debt cannot be open-ended");
    assert!(open_ended.to_string().contains("--kind structural"));

    let undated = exception::run(&add_options(&root, Kind::Deferred, None))
        .expect_err("deferred debt needs a retirement condition");
    assert!(undated.to_string().contains("--until"));
}

/// §FS-004-check-audit.2: accepted-permanently and carrying-debt are two numbers,
/// and an entry written before `kind` existed counts as deferred
/// (§FS-003-exceptions.2.1).
#[test]
fn audit_counts_exceptions_by_kind() {
    let root = temp_repo();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(
        root.join("docs/file-size-human-exceptions.toml"),
        "fissile_exceptions_version = 1\n\n\
         [[exceptions]]\n\
         id = \"EX-001-big\"\n\
         path = \"src/big.rs\"\n\
         match = \"exact\"\n\
         rules = [\"rust\"]\n\
         kind = \"structural\"\n\
         max_accepted = { value = 250, unit = \"lines\" }\n\
         until = \"indefinite\"\n\
         reason = \"asserted byte-identical by the snapshot test\"\n\n\
         [[exceptions]]\n\
         id = \"EX-002-ok\"\n\
         path = \"src/ok.rs\"\n\
         match = \"exact\"\n\
         rules = [\"rust\"]\n\
         max_accepted = { value = 250, unit = \"lines\" }\n\
         until = \"the reader module lands\"\n\
         reason = \"written before the kind field existed\"\n",
    )
    .unwrap();

    let run = audit::run(&AuditOptions {
        root,
        config_path: None,
        format: Some(Format::Text),
        no_color: false,
        top: None,
        stale_exceptions: false,
        rule_coverage: false,
    })
    .expect("audit runs");

    assert!(run.output.contains("exceptions:"));
    assert!(run.output.contains("structural (never expires): 1"));
    assert!(run.output.contains("deferred (carrying debt): 1"));
}

/// §FS-002-init.4: a dry run prints the managed block on stdout, so an agent can
/// read what the repository expects of it without writing a file. The text is
/// the constant `init` installs, so printed and written cannot drift.
#[test]
fn init_dry_run_prints_the_managed_block() {
    let root = temp_repo();
    let output = Command::new(env!("CARGO_BIN_EXE_fissile"))
        .args(["init", ".", "--dry-run"])
        .current_dir(&root)
        .output()
        .expect("fissile runs");

    let stdout = String::from_utf8(output.stdout).expect("stdout is utf-8");
    let stderr = String::from_utf8(output.stderr).expect("stderr is utf-8");

    assert!(output.status.success());
    assert_eq!(stdout.trim_end(), fissile::init::MANAGED_BLOCK.trim_end());
    // Planned writes stay on stderr, so the two are separable.
    assert!(stderr.contains("would-write"));
    assert!(!stdout.contains("would-write"));
    assert!(!root.join("AGENTS.md").exists(), "a dry run writes nothing");
}
