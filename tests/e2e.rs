//! Fixture-driven end-to-end suite (§GOAL-003-friendly-output.3). Every directory
//! under `e2e/cases/` named `E2E-*` is one scenario: a `case.toml` manifest plus a
//! `repo/` working tree. The harness drives the real `fissile` binary against a
//! throwaway copy of the tree and asserts the exit code and output the manifest
//! declares, so each documented behavior under `docs/functional-spec/` has at least
//! one executable scenario.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

/// One scenario manifest (`case.toml`).
#[derive(Deserialize)]
struct Case {
    /// Arguments passed to `fissile`.
    args: Vec<String>,
    /// Expected process exit code.
    #[serde(default)]
    exit: i32,
    /// Initialize a git repo and stage the tree before running (for `--staged`
    /// checks and the pre-commit hook install).
    #[serde(default)]
    git: bool,
    /// Substrings that must appear in stdout.
    #[serde(default)]
    stdout_contains: Vec<String>,
    /// Exact stdout (trailing newline ignored), when the bytes must be stable.
    stdout_equals: Option<String>,
    /// Substrings that must appear in stderr.
    #[serde(default)]
    stderr_contains: Vec<String>,
    /// Repo-relative paths that must exist after the run (for `init`).
    #[serde(default)]
    creates: Vec<String>,
    /// Skip on non-Unix hosts. Used by the token-unit case, whose external
    /// counter is a POSIX shell stub (§DA-001-token-external-command); token
    /// mode itself is cross-platform, but a portable counter stub is not, so
    /// this single fixture is Unix-gated the way the bench job is Linux-only
    /// (§DA-002-instruction-count-benchmarks).
    #[serde(default)]
    unix_only: bool,
}

fn cases_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("e2e/cases")
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Run one case; return Err(reason) on failure so the caller can report all.
fn run_case(dir: &Path) -> Result<(), String> {
    let manifest = fs::read_to_string(dir.join("case.toml"))
        .map_err(|error| format!("reading case.toml: {error}"))?;
    let case: Case =
        toml::from_str(&manifest).map_err(|error| format!("parsing case.toml: {error}"))?;

    if case.unix_only && !cfg!(unix) {
        return Ok(());
    }

    let work = std::env::temp_dir().join(format!(
        "fissile-e2e-{}-{}",
        std::process::id(),
        dir.file_name().unwrap().to_string_lossy()
    ));
    let _ = fs::remove_dir_all(&work);
    let repo = dir.join("repo");
    if repo.is_dir() {
        copy_tree(&repo, &work);
    } else {
        fs::create_dir_all(&work).unwrap();
    }

    if case.git {
        git(&work, &["init", "-q"]);
        git(&work, &["add", "-A"]);
    }

    let args: Vec<&str> = case.args.iter().map(String::as_str).collect();
    let output = Command::new(env!("CARGO_BIN_EXE_fissile"))
        .current_dir(&work)
        .args(&args)
        .output()
        .map_err(|error| format!("spawning fissile: {error}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let code = output.status.code().unwrap_or(-1);
    let mut problems = Vec::new();

    if code != case.exit {
        problems.push(format!("exit {code}, expected {}", case.exit));
    }
    if let Some(expected) = &case.stdout_equals
        && stdout.trim_end_matches('\n') != expected.trim_end_matches('\n')
    {
        problems.push(format!(
            "stdout != expected\n--- got ---\n{stdout}\n--- want ---\n{expected}"
        ));
    }
    for needle in &case.stdout_contains {
        if !stdout.contains(needle) {
            problems.push(format!("stdout missing {needle:?}"));
        }
    }
    for needle in &case.stderr_contains {
        if !stderr.contains(needle) {
            problems.push(format!("stderr missing {needle:?}"));
        }
    }
    for relative in &case.creates {
        if !work.join(relative).exists() {
            problems.push(format!("expected {relative} to be created"));
        }
    }

    let _ = fs::remove_dir_all(&work);
    if problems.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "\n  stdout:\n{stdout}\n  stderr:\n{stderr}\n  {}",
            problems.join("\n  ")
        ))
    }
}

#[test]
fn every_case_passes() {
    let mut cases: Vec<PathBuf> = fs::read_dir(cases_dir())
        .expect("cases dir")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_dir()
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("E2E-"))
        })
        .collect();
    cases.sort();

    assert!(
        cases.len() >= 8,
        "expected the documented FS behaviors to each have a case, found {}",
        cases.len()
    );

    let mut failures = Vec::new();
    for case in &cases {
        let name = case.file_name().unwrap().to_string_lossy().into_owned();
        if let Err(reason) = run_case(case) {
            failures.push(format!("{name}: {reason}"));
        }
    }
    assert!(
        failures.is_empty(),
        "e2e failures:\n{}",
        failures.join("\n\n")
    );
}
