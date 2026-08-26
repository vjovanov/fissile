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
    options.reason = "a generated table the snapshot test asserts byte-identical".to_owned();
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
        kind: Kind::Deferred,
        reason: "accepted exact file".to_owned(),
        until: Some("the case-builder module lands".to_owned()),
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
    // reader can go edit that entry. The refusal reports the recorded ceiling
    // beside the current measurement and names the command that moves it —
    // "an entry exists" is not "the file is accepted" (§FS-005-exception-add.4).
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
        kind,
        reason: "a reason".to_owned(),
        until: until.map(str::to_owned),
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
    assert!(run.output.contains("structural (never expires): 1"));
    assert!(run.output.contains("deferred (carrying debt): 1"));
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
