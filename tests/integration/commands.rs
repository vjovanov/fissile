//! End-to-end coverage for the library command surfaces: `check`, `audit`, and
//! `exception add` (§FS-004-check-audit, §FS-005-exception-add).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use fissile::audit::{self, AuditOptions};
use fissile::check::{self, CheckOptions};
use fissile::cli::Format;
use fissile::exception::{self, AddOptions, Rationale};
use fissile::exceptions::{Kind, MatchKind};
use fissile::retune::{self, RetuneOptions};
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
    assert!(
        run.output
            .contains("src/big.rs: 250 non-blank lines (budget 200)")
    );
    assert!(run.output.contains("[rule: rust, message:"));
}

/// §GOAL-006-graded-limits.1, §FS-004-check-audit.1, §FS-004-check-audit.2:
/// both command surfaces pass equality, report the first value above each
/// threshold, and preserve the soft-versus-hard exit contract.
#[test]
fn limits_are_strictly_above_in_check_and_audit() {
    let root = temp_repo();
    for (actual, expected_severity, failed) in [
        (99, None, false),  // below soft
        (100, None, false), // equal soft
        (101, Some("soft"), false),
        (199, Some("soft"), false), // below hard
        (200, Some("soft"), false), // equal hard
        (201, Some("hard"), true),
    ] {
        fs::write(root.join("src/big.rs"), rust_lines(actual)).unwrap();

        let check_run = check::run(&check_options(&root)).expect("check runs");
        assert_eq!(check_run.failed, failed, "check failed at actual={actual}");
        assert_severity(&check_run.output, actual, expected_severity);

        let audit_run = audit::run(&AuditOptions {
            root: root.clone(),
            no_color: true,
            ..AuditOptions::default()
        })
        .expect("audit runs");
        assert_eq!(audit_run.failed, failed, "audit failed at actual={actual}");
        assert_severity(&audit_run.output, actual, expected_severity);
    }
}

fn assert_severity(output: &str, actual: usize, expected: Option<&str>) {
    match expected {
        None => assert_eq!(output.trim(), "ok", "unexpected finding at actual={actual}"),
        Some(severity) => {
            assert!(
                output.contains(&format!("{severity}: 1 file over the")),
                "missing {severity} finding at actual={actual}: {output}"
            );
            assert!(
                output.contains(&format!(
                    "src/big.rs: {actual} non-blank lines (budget {})",
                    if severity == "soft" { 100 } else { 200 }
                )),
                "missing contextual detail at actual={actual}: {output}"
            );
            if severity == "soft" {
                assert!(!output.contains("hard: 1 file over the"), "{output}");
            }
        }
    }
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
        rationale: Rationale::Stated {
            kind: Kind::Deferred,
            reason: "no module owns the generated cases yet".to_owned(),
            until: Some("the case-builder module lands".to_owned()),
        },
        match_kind: MatchKind::Exact,
        title: None,
        owner: None,
        issue: None,
        max: None,
        unit: None,
        interactive: true,
        force: false,
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

/// §FS-005-exception-add.3: a created registry declares version 2, and the entry
/// carries no name — it is identified by this registry and what it accepts
/// (§DF-005-exception-identity).
#[test]
fn exception_add_writes_a_version_2_registry_and_no_name() {
    let root = temp_repo();
    let run = exception::run(&add_options(
        &root,
        Kind::Deferred,
        Some("the parser lands"),
    ))
    .expect("exception add runs");
    assert!(run.output.contains("appended src/big.rs to"));

    let registry = fs::read_to_string(root.join("docs/file-size-human-exceptions.toml")).unwrap();
    assert!(registry.starts_with("fissile_exceptions_version = 2"));
    assert!(!registry.contains("id = "), "{registry}");
    assert!(!registry.contains("replaces = "), "{registry}");
}

/// §FS-003-exceptions.2.2: an unmigrated registry is refused, and the message
/// names both edits rather than only the version this build supports. The `id`
/// keys are what the strict parse would trip on first, so this also pins that
/// the version outranks them.
#[test]
fn unmigrated_registry_is_refused_with_both_edits_named() {
    let root = temp_repo();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(
        root.join("docs/file-size-human-exceptions.toml"),
        "fissile_exceptions_version = 1\n\n\
         [[exceptions]]\n\
         id = \"EX-001-big\"\n\
         replaces = \"EX-000-big\"\n\
         path = \"src/big.rs\"\n\
         match = \"exact\"\n\
         rules = [\"rust\"]\n\
         kind = \"structural\"\n\
         max_accepted = { value = 250, unit = \"lines\" }\n\
         until = \"indefinite\"\n\
         reason = \"asserted byte-identical by the snapshot test\"\n",
    )
    .unwrap();

    let message = match check::run(&check_options(&root)) {
        Ok(_) => panic!("a version 1 registry must be refused"),
        Err(error) => error.to_string(),
    };
    for clause in [
        "docs/file-size-human-exceptions.toml: exception registry version 1 is unsupported",
        "set fissile_exceptions_version = 2",
        "delete every id and replaces line",
    ] {
        assert!(message.contains(clause), "missing `{clause}`: {message}");
    }
}

/// §FS-003-exceptions.3: a `structural` hard entry silences the soft finding too.
/// Splitting the file is illegal, so the warning names work nobody may do, and no
/// amount of work can clear it — the whole file goes quiet on one entry.
#[test]
fn structural_hard_exception_also_silences_soft() {
    let root = temp_repo();
    let mut options = add_options(&root, Kind::Structural, None);
    options.rationale = Rationale::Stated {
        kind: Kind::Structural,
        reason: "a generated table the snapshot test asserts byte-identical".to_owned(),
        until: None,
    };
    exception::run(&options).expect("exception add runs");

    let run = check::run(&check_options(&root)).expect("check runs");
    assert!(!run.failed, "hard overflow is accepted");
    assert_eq!(
        run.output, "ok",
        "a structural entry leaves nothing to report"
    );

    // Audit still attributes the silenced hard overflow to the entry that
    // accepted it, and reports no soft finding beside it (§FS-003-exceptions.5).
    let audited = audit::run(&AuditOptions {
        root,
        config_path: None,
        format: Some(Format::Text),
        no_color: false,
        top: None,
        stale_exceptions: false,
        rule_coverage: false,
    })
    .expect("audit runs");
    // Attribution names no id since version 2 removed it (§DF-005-exception-identity).
    assert!(
        audited
            .output
            .contains("src/big.rs: hard exception (accepted up to")
    );
    assert!(!audited.output.contains("soft: "), "{}", audited.output);
    assert!(audited.output.contains("structural (never expires): 1"));
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

/// §FS-004-check-audit.2: empty registries remain silent in text while JSON
/// keeps its unconditional all-zero exceptions object.
#[test]
fn audit_empty_exception_inventory_keeps_text_and_json_contracts() {
    let root = temp_repo();
    let text = audit::run(&AuditOptions {
        root: root.clone(),
        format: Some(Format::Text),
        no_color: true,
        ..AuditOptions::default()
    })
    .expect("text audit runs")
    .output;
    assert!(!text.contains("exceptions:"), "{text}");

    let json = audit::run(&AuditOptions {
        root,
        format: Some(Format::Json),
        ..AuditOptions::default()
    })
    .expect("JSON audit runs")
    .output;
    assert!(
        json.contains(
            "\"exceptions\":{\"structural\":0,\"deferred\":0,\"structural_paths\":0,\"deferred_paths\":0}"
        ),
        "{json}"
    );
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
        rationale: Rationale::Stated {
            kind: Kind::Structural,
            reason: "generated tree asserted byte-identical by the snapshot test".to_owned(),
            until: None,
        },
        match_kind: MatchKind::Glob,
        title: None,
        owner: None,
        issue: None,
        max: Some(300),
        unit: Some(Unit::Lines),
        interactive: true,
        force: false,
        dry_run: false,
    })
    .expect("glob exception add runs");

    let error = match exception::run(&AddOptions {
        root: root.clone(),
        config_path: None,
        path: "src/big.rs".to_owned(),
        severity: Severity::Hard,
        rules: vec!["rust".to_owned()],
        rationale: Rationale::Stated {
            kind: Kind::Deferred,
            reason: "accepted exact file".to_owned(),
            until: Some("the case-builder module lands".to_owned()),
        },
        match_kind: MatchKind::Exact,
        title: None,
        owner: None,
        issue: None,
        max: None,
        unit: None,
        interactive: true,
        force: false,
        dry_run: false,
    }) {
        Ok(_) => panic!("overlapping exception should be rejected"),
        Err(error) => error,
    };

    // The blocking entry is named by where it lives and what it matches, so the
    // reader can go edit it. The refusal reports the recorded ceiling beside
    // the measurement and the command that moves it (§FS-005-exception-add.4).
    assert_eq!(
        error.to_string(),
        "docs/file-size-human-exceptions.toml: src/** already has an entry covering \
         src/big.rs for this rule and unit (accepts up to 300 lines; the file is 250) \
         — move the ceiling with `fissile exception retune`"
    );
}

fn add_options(root: &Path, kind: Kind, until: Option<&str>) -> AddOptions {
    AddOptions {
        root: root.to_path_buf(),
        config_path: None,
        path: "src/big.rs".to_owned(),
        severity: Severity::Hard,
        rules: vec!["rust".to_owned()],
        rationale: Rationale::Stated {
            kind,
            reason: "a reason".to_owned(),
            until: until.map(str::to_owned),
        },
        match_kind: MatchKind::Exact,
        title: None,
        owner: None,
        issue: None,
        max: None,
        unit: None,
        interactive: true,
        force: false,
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
/// and an entry that omits `kind` counts as deferred
/// (§FS-003-exceptions.2.1).
#[test]
fn audit_counts_exceptions_by_kind() {
    let root = temp_repo();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(
        root.join("docs/file-size-agent-exceptions.toml"),
        r#"fissile_exceptions_version = 2

[[exceptions]]
path = "src/big.rs"
match = "exact"
rules = ["rust"]
kind = "structural"
max_accepted = { value = 250, unit = "lines" }
until = "indefinite"
reason = "the soft twin shares the structural constraint"
"#,
    )
    .unwrap();
    fs::write(
        root.join("docs/file-size-human-exceptions.toml"),
        "fissile_exceptions_version = 2\n\n\
         [[exceptions]]\n\
         path = \"src/big.rs\"\n\
         match = \"exact\"\n\
         rules = [\"rust\"]\n\
         kind = \"structural\"\n\
         max_accepted = { value = 250, unit = \"lines\" }\n\
         until = \"indefinite\"\n\
         reason = \"asserted byte-identical by the snapshot test\"\n\n\
         [[exceptions]]\n\
         path = \"src/ok.rs\"\n\
         match = \"exact\"\n\
         rules = [\"rust\"]\n\
         max_accepted = { value = 250, unit = \"lines\" }\n\
         until = \"the reader module lands\"\n\
         reason = \"omits kind, so it counts as deferred\"\n",
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
    assert!(
        run.output
            .contains("structural (never expires): 2 entries across 1 paths")
    );
    assert!(
        run.output
            .contains("deferred (carrying debt): 1 entries across 1 paths")
    );
}

/// §FS-004-check-audit.2: the stale list spans both registries, so each entry is
/// named by its registry and its `path` — the same path can be stale in each,
/// and two bare paths would not say which file to edit (§FS-003-exceptions.4).
#[test]
fn stale_exceptions_name_the_registry_they_live_in() {
    let root = temp_repo();
    fs::create_dir_all(root.join("docs")).unwrap();
    for (registry, max) in [
        ("docs/file-size-agent-exceptions.toml", 150),
        ("docs/file-size-human-exceptions.toml", 250),
    ] {
        fs::write(
            root.join(registry),
            format!(
                "fissile_exceptions_version = 2\n\n\
                 [[exceptions]]\n\
                 path = \"src/gone.rs\"\n\
                 match = \"exact\"\n\
                 rules = [\"rust\"]\n\
                 kind = \"deferred\"\n\
                 max_accepted = {{ value = {max}, unit = \"lines\" }}\n\
                 until = \"the module lands\"\n\
                 reason = \"no module owns it yet\"\n"
            ),
        )
        .unwrap();
    }

    let options = AuditOptions {
        root,
        config_path: None,
        format: Some(Format::Text),
        no_color: false,
        top: None,
        stale_exceptions: true,
        rule_coverage: false,
    };
    let text = audit::run(&options).expect("audit runs").output;
    assert!(
        text.contains("  docs/file-size-agent-exceptions.toml: src/gone.rs")
            && text.contains("  docs/file-size-human-exceptions.toml: src/gone.rs"),
        "{text}"
    );

    let json = audit::run(&AuditOptions {
        format: Some(Format::Json),
        ..options
    })
    .expect("audit runs")
    .output;
    assert!(
        json.contains(
            "{\"registry\":\"docs/file-size-agent-exceptions.toml\",\"path\":\"src/gone.rs\"}"
        ),
        "{json}"
    );
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

/// A main repository with one commit and a linked worktree of it checked out
/// on a new branch, named uniquely so parallel tests never collide.
fn git_worktree_pair(tag: &str) -> (PathBuf, PathBuf) {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let main = std::env::temp_dir().join(format!("fissile-worktree-main-{tag}-{pid}-{n}"));
    let worktree = std::env::temp_dir().join(format!("fissile-worktree-branch-{tag}-{pid}-{n}"));
    let _ = fs::remove_dir_all(&main);
    let _ = fs::remove_dir_all(&worktree);
    fs::create_dir_all(&main).unwrap();

    git(&main, &["init", "-q"]);
    git(
        &main,
        &[
            "-c",
            "user.email=e2e@fissile.invalid",
            "-c",
            "user.name=e2e",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "core.hooksPath=e2e-no-hooks",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "base",
        ],
    );
    git(
        &main,
        &[
            "worktree",
            "add",
            "-q",
            worktree.to_str().expect("temp path is utf-8"),
            "-b",
            "feature",
        ],
    );
    assert!(
        fs::symlink_metadata(worktree.join(".git"))
            .unwrap()
            .is_file()
    );
    (main, worktree)
}

/// The hook `init` should have installed in a main repository's shared
/// `hooks/pre-commit`, holding the current managed block and, on Unix,
/// executable.
fn assert_hook_installed(main: &Path) {
    let hook_path = main.join(".git/hooks/pre-commit");
    let hook = fs::read_to_string(&hook_path).expect("hook installed in the main repository");
    assert!(hook.contains("fissile check --staged || exit 1"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&hook_path).unwrap().permissions().mode();
        assert_eq!(mode & 0o111, 0o111, "hook is executable");
    }
}

/// §FS-002-init.6, §FS-002-init.5: a linked worktree's `.git` is a file naming
/// the main repository's private gitdir, not a directory — `init` has to
/// resolve through it and install into the main repository's shared
/// `hooks/pre-commit`, instead of reporting "not a git repository" and sending
/// the reader to `git init` inside a directory git already tracks (issue #36).
#[test]
fn init_in_a_linked_worktree_installs_the_hook_in_the_main_repository() {
    const HOOK_STEP: &str =
        "Commit a change to see the pre-commit hook run fissile check --staged.";

    let (main, worktree) = git_worktree_pair("auto");
    let automatic = Command::new(env!("CARGO_BIN_EXE_fissile"))
        .args(["init", "--agents-md"])
        .current_dir(&worktree)
        .output()
        .expect("fissile runs");
    let automatic_stderr = String::from_utf8_lossy(&automatic.stderr).into_owned();
    assert!(
        automatic.status.success(),
        "automatic init failed: {automatic_stderr}"
    );
    assert!(automatic_stderr.contains("pre-commit"));
    assert!(automatic_stderr.contains(HOOK_STEP));
    assert!(!automatic_stderr.contains("is not a git repository"));
    assert!(!automatic_stderr.contains("git init"));
    assert_hook_installed(&main);
    // The worktree's own `.git` is still the pointer file: nothing was created
    // under it.
    assert!(
        fs::symlink_metadata(worktree.join(".git"))
            .unwrap()
            .is_file()
    );
    let _ = fs::remove_dir_all(&worktree);
    let _ = fs::remove_dir_all(&main);

    let (main, worktree) = git_worktree_pair("hook-flag");
    let forced = Command::new(env!("CARGO_BIN_EXE_fissile"))
        .args(["init", "--agents-md", "--hook"])
        .current_dir(&worktree)
        .output()
        .expect("fissile runs");
    let forced_stderr = String::from_utf8_lossy(&forced.stderr).into_owned();
    assert!(
        forced.status.success(),
        "fissile init --hook failed: {forced_stderr}"
    );
    assert!(forced_stderr.contains(HOOK_STEP));
    assert_hook_installed(&main);
    assert!(
        fs::symlink_metadata(worktree.join(".git"))
            .unwrap()
            .is_file()
    );
    let _ = fs::remove_dir_all(&worktree);
    let _ = fs::remove_dir_all(&main);
}

/// §DF-010-stated-ceilings-are-exact.1: a `--max` is written as stated, where
/// the measured form would have rounded 250 up to 300.
#[test]
fn a_stated_max_is_written_as_stated() {
    let root = temp_repo();
    let mut options = add_options(&root, Kind::Deferred, Some("the parser lands"));
    options.max = Some(260);
    options.unit = Some(Unit::Lines);
    exception::run(&options).expect("exception add runs");

    let registry = fs::read_to_string(root.join("docs/file-size-human-exceptions.toml")).unwrap();
    assert!(
        registry.contains("max_accepted = { value = 260, unit = \"lines\" }"),
        "{registry}"
    );
    assert!(!registry.contains("value = 300"), "{registry}");
}

/// A 150-line file under a 100/200-line rule with the default 100-line step: the
/// measured form lands on 200, the hard limit.
fn soft_options_for_mid(root: &Path) -> AddOptions {
    fs::write(root.join("src/mid.rs"), rust_lines(150)).unwrap();
    let mut options = add_options(root, Kind::Deferred, Some("the parser lands"));
    options.path = "src/mid.rs".to_owned();
    options.severity = Severity::Soft;
    options
}

/// §FS-005-exception-add.4, §DF-010-stated-ceilings-are-exact.2: a soft ceiling
/// the step lands on the hard limit is refused, and the refusal is this call
/// with `--max <N>` and the range that keeps it under the limit.
#[test]
fn a_soft_ceiling_landing_on_the_hard_limit_is_refused_with_the_stated_form() {
    let root = temp_repo();
    let options = soft_options_for_mid(&root);
    let error = exception::run(&options).expect_err("refused");
    assert_eq!(
        error.to_string(),
        "src/mid.rs measures 150 lines; without --max the ceiling is the measurement rounded \
         up to the 100-line step, and that lands on 200 — rule rust's hard limit is 200, where \
         a soft ceiling never fires. State the ceiling instead:\n  \
         fissile exception add src/mid.rs --severity soft --rule rust --kind deferred \
         --until 'the parser lands' --reason 'a reason' --max <N> --unit lines\n\
         with 150 <= N < 200."
    );
    assert!(!root.join("docs/file-size-agent-exceptions.toml").exists());

    let mut stated = options;
    stated.max = Some(180);
    stated.unit = Some(Unit::Lines);
    exception::run(&stated).expect("the stated form runs");
    let registry = fs::read_to_string(root.join("docs/file-size-agent-exceptions.toml")).unwrap();
    assert!(
        registry.contains("max_accepted = { value = 180, unit = \"lines\" }"),
        "{registry}"
    );
}

/// A stated soft ceiling at the hard limit gets the same refusal, and also the
/// hard-severity call — with the stated `--max` carried through, since a hard
/// entry may hold it (§FS-005-exception-add.4).
#[test]
fn a_stated_soft_ceiling_at_the_hard_limit_names_the_hard_route() {
    let root = temp_repo();
    let mut options = soft_options_for_mid(&root);
    options.max = Some(200);
    options.unit = Some(Unit::Lines);
    let error = exception::run(&options).expect_err("refused");
    assert_eq!(
        error.to_string(),
        "--max 200 is at or above rule rust hard limit 200; a soft ceiling there silences \
         nothing. Stay under it:\n  \
         fissile exception add src/mid.rs --severity soft --rule rust --kind deferred \
         --until 'the parser lands' --reason 'a reason' --max <N> --unit lines\n\
         with 150 <= N < 200, or accept the file in the hard registry:\n  \
         fissile exception add src/mid.rs --severity hard --rule rust --kind deferred \
         --until 'the parser lands' --reason 'a reason' --max 200 --unit lines"
    );
}

/// §DF-010-stated-ceilings-are-exact.2: a soft ceiling past the hard limit for a
/// file still under it is refused — unless the hard registry holds the address,
/// where a deferred hard entry keeps the soft finding alive above the limit and
/// the soft ceiling is doing work. A file already past the limit is the other
/// case: its soft entry is the record of debt
/// §DF-008-hard-severity-needs-a-terminal.1 offers, and runs as it always did.
#[test]
fn a_hard_twin_makes_a_soft_ceiling_above_the_hard_limit_legitimate() {
    let root = temp_repo();
    let mut soft = soft_options_for_mid(&root);
    soft.max = Some(250);
    soft.unit = Some(Unit::Lines);
    let error = exception::run(&soft).expect_err("refused without a twin");
    assert!(
        error
            .to_string()
            .starts_with("--max 250 is at or above rule rust hard limit 200"),
        "{error}"
    );

    let mut hard = soft_options_for_mid(&root);
    hard.severity = Severity::Hard;
    hard.max = Some(250);
    hard.unit = Some(Unit::Lines);
    exception::run(&hard).expect("the hard entry runs");
    let run = exception::run(&soft).expect("the soft twin runs");
    assert!(
        run.output.contains("accepted up to 250 lines"),
        "{}",
        run.output
    );

    let mut debt = add_options(&root, Kind::Deferred, Some("the parser lands"));
    debt.severity = Severity::Soft;
    let run = exception::run(&debt).expect("a file past the hard limit keeps its soft route");
    assert!(
        run.output.contains("accepted up to 300 lines"),
        "{}",
        run.output
    );
}

/// §FS-003-exceptions.2.3: a twin inherits one rationale, and two hard entries
/// listing the same rules are told apart by nothing — so the refusal reports the
/// duplicate rather than printing one rule list twice as if it distinguished
/// them (§DF-007-instructions-at-the-error-site).
#[test]
fn a_duplicated_hard_entry_is_refused_as_a_duplicate_not_as_two_rule_lists() {
    let root = temp_repo();
    let entry = "\n[[exceptions]]\npath = \"src/big.rs\"\nmatch = \"exact\"\nrules = [\"rust\"]\n\
                 kind = \"deferred\"\nmax_accepted = { value = 300, unit = \"lines\" }\n\
                 until = \"the parser lands\"\nreason = \"a reason\"\n";
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(
        root.join("docs/file-size-human-exceptions.toml"),
        format!("fissile_exceptions_version = 2\n{entry}{entry}"),
    )
    .unwrap();

    let mut options = add_options(&root, Kind::Deferred, Some("the parser lands"));
    options.severity = Severity::Soft;
    options.rationale = Rationale::ShadowsHard;
    let error = exception::run(&options).expect_err("two entries answer the address");
    assert_eq!(
        error.to_string(),
        "--shadows-hard inherits one rationale, and docs/file-size-human-exceptions.toml holds \
         more than one entry for src/big.rs, each listing rules [rust]. Delete the duplicate \
         entry there."
    );
    assert!(!root.join("docs/file-size-agent-exceptions.toml").exists());
}

fn retune_options(root: &Path, max: Option<u64>) -> RetuneOptions {
    RetuneOptions {
        root: root.to_path_buf(),
        config_path: None,
        path: "src/big.rs".to_owned(),
        severity: Severity::Hard,
        rules: vec!["rust".to_owned()],
        match_kind: MatchKind::Exact,
        max,
        unit: max.map(|_| Unit::Lines),
        dry_run: false,
    }
}

/// §FS-008-exception-retune.2: a stated ceiling moves by exactly what was
/// stated, so `--max` brings a rounded ceiling back down by less than a step —
/// and the result names the step's next multiple rather than applying it.
#[test]
fn retune_with_max_moves_by_less_than_a_step() {
    let root = temp_repo();
    exception::run(&add_options(
        &root,
        Kind::Deferred,
        Some("the parser lands"),
    ))
    .expect("the hard entry runs at 300");

    let run = retune::run(&retune_options(&root, Some(260))).expect("retune runs");
    assert_eq!(
        run.output,
        "docs/file-size-human-exceptions.toml: src/big.rs 300 -> 260 lines (next 100-line step: 300)"
    );
    let registry = fs::read_to_string(root.join("docs/file-size-human-exceptions.toml")).unwrap();
    assert!(
        registry.contains("max_accepted = { value = 260, unit = \"lines\" }"),
        "{registry}"
    );

    // The measured form is unchanged: 250 lines round back up to 300.
    let run = retune::run(&retune_options(&root, None)).expect("retune runs");
    assert_eq!(
        run.output,
        "docs/file-size-human-exceptions.toml: src/big.rs 260 -> 300 lines (measured 250 lines; quantized to 100-line step)"
    );
}

/// §DF-010-stated-ceilings-are-exact.2: the rule is about the ceiling, not about
/// having measured something. A glob measures nothing, so it can never claim the
/// exemption a file already past the limit has — and its ceiling above the hard
/// limit is just as dead, since every member under the limit is silenced by the
/// entry and every member above it is a hard finding.
#[test]
fn a_glob_soft_ceiling_on_the_hard_limit_is_refused() {
    let root = temp_repo();
    let mut options = add_options(&root, Kind::Structural, None);
    options.severity = Severity::Soft;
    options.path = "src/**".to_owned();
    options.match_kind = MatchKind::Glob;
    options.max = Some(900);
    options.unit = Some(Unit::Lines);

    let error =
        exception::run(&options).expect_err("a glob ceiling above the hard limit is refused");
    let message = error.to_string();
    assert!(
        message.contains("--max 900 is at or above rule rust hard limit 200"),
        "{message}"
    );
    // No measurement to name, so the floor is the soft limit the entry accepts.
    assert!(message.contains("with 100 <= N < 200"), "{message}");

    // Under the hard limit the glob is written as stated (E2E-048).
    options.max = Some(150);
    exception::run(&options).expect("a glob ceiling under the hard limit is written");
}

/// §DF-010-stated-ceilings-are-exact.2: the exemption is a *deferred* hard twin,
/// which keeps the soft finding alive above the limit. A structural one ends
/// evaluation instead (§FS-003-exceptions.3), so the soft ceiling it would
/// exempt silences exactly nothing.
#[test]
fn only_a_deferred_hard_twin_exempts_a_soft_ceiling() {
    for (kind, until, exempt) in [
        (Kind::Structural, None, false),
        (Kind::Deferred, Some("the split lands"), true),
    ] {
        let root = temp_repo();
        fs::write(root.join("src/big.rs"), rust_lines(150)).unwrap();

        let mut twin = add_options(&root, kind, until);
        twin.severity = Severity::Hard;
        twin.max = Some(400);
        twin.unit = Some(Unit::Lines);
        exception::run(&twin).expect("the hard twin is written");

        let mut soft = add_options(&root, Kind::Structural, None);
        soft.severity = Severity::Soft;
        soft.max = Some(900);
        soft.unit = Some(Unit::Lines);
        let outcome = exception::run(&soft);
        assert_eq!(
            outcome.is_ok(),
            exempt,
            "{kind:?} twin: expected exempt={exempt}, got {outcome:?}"
        );
    }
}

/// §DF-007-instructions-at-the-error-site: the range a refusal prints has to be
/// one the next call accepts. `check_min_limit` refuses a ceiling below *any*
/// listed rule's soft limit, so the floor is the highest of them — not the soft
/// limit of whichever rule happens to set the hard one.
#[test]
fn the_offered_range_clears_every_rule_soft_limit() {
    const TWO_RULES: &str = r#"
fissile_config_version = 1
[scan]
include = ["src"]
exclude = []
respect_gitignore = false
[[messages]]
id = "m"
text = "Split {path}."
[[rules]]
id = "a"
include = ["src/**/*.rs"]
unit = "lines"
soft = 100
hard = 200
message = "m"
[[rules]]
id = "b"
include = ["src/**/*.rs"]
unit = "lines"
soft = 180
hard = 300
message = "m"
"#;
    let root = temp_repo();
    fs::write(root.join(".agents/fissile.toml"), TWO_RULES).unwrap();
    fs::write(root.join("src/big.rs"), rust_lines(150)).unwrap();

    let mut options = add_options(&root, Kind::Structural, None);
    options.severity = Severity::Soft;
    options.rules = vec!["a".to_owned(), "b".to_owned()];
    options.max = Some(900);
    options.unit = Some(Unit::Lines);

    let message = exception::run(&options)
        .expect_err("900 is at or above rule a's hard limit")
        .to_string();
    // Rule a sets the hard limit at 200 and its soft limit is 100, but a ceiling
    // under rule b's soft limit of 180 is refused by the very next call.
    assert!(message.contains("with 180 <= N < 200"), "{message}");

    options.max = Some(180);
    exception::run(&options).expect("the floor the refusal named is accepted");
}

/// §DF-010-stated-ceilings-are-exact.2 forbids a circle between two refusals.
/// The hard-limit refusal offers the hard `add`; the severity gate turns that
/// down non-interactively — and must not answer with the soft command that was
/// just refused for carrying that same `--max`.
#[test]
fn the_severity_gate_does_not_hand_back_a_refused_ceiling() {
    let root = temp_repo();
    fs::write(root.join("src/big.rs"), rust_lines(150)).unwrap();

    let mut options = add_options(&root, Kind::Structural, None);
    options.severity = Severity::Hard;
    options.interactive = false;
    options.max = Some(900);
    options.unit = Some(Unit::Lines);

    let message = exception::run(&options)
        .expect_err("a hard add is refused off a terminal")
        .to_string();
    assert!(message.contains("--severity soft"), "{message}");
    assert!(
        message.contains("--max <N> --unit lines"),
        "the route asks for a ceiling under the hard limit: {message}"
    );
    assert!(
        !message.contains("--max 900"),
        "repeating the refused ceiling closes the circle: {message}"
    );

    // A ceiling a soft entry could actually hold is carried through unchanged.
    options.max = Some(150);
    let message = exception::run(&options)
        .expect_err("a hard add is refused off a terminal")
        .to_string();
    assert!(message.contains("--max 150 --unit lines"), "{message}");
}

/// §FS-008-exception-retune.4: the hard route has to run as printed. Without the
/// ceiling flags the rerun measures the file, finds it under the hard limit, and
/// is refused for needing no exception — a second refusal from the command
/// printed to prevent one.
#[test]
fn the_retune_hard_route_carries_the_ceiling() {
    let root = temp_repo();
    fs::write(root.join("src/big.rs"), rust_lines(150)).unwrap();

    let mut seed = add_options(&root, Kind::Structural, None);
    seed.severity = Severity::Soft;
    seed.max = Some(150);
    seed.unit = Some(Unit::Lines);
    exception::run(&seed).expect("the soft entry is written");

    let mut options = retune_options(&root, Some(900));
    options.severity = Severity::Soft;
    let message = retune::run(&options)
        .expect_err("900 is at or above the hard limit")
        .to_string();
    assert!(
        message.contains("--severity hard --rule rust --max 900 --unit lines --kind structural"),
        "{message}"
    );
    assert!(
        message.contains("--kind deferred --until '<what retires it>'"),
        "only deferred takes --until, so the route names both spellings: {message}"
    );
}
