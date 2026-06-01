//! Filesystem traversal and measurement (§FS-004-check-audit): turn a config's
//! scan scope or a git-staged set into [`FileMeasurement`]s, honoring
//! `[scan].exclude`, `respect_gitignore`, and the opt-in `[tokens].command`.

use std::fs;
use std::io;
use std::path::Component;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::{Scan, Tokens};
use crate::{FileMeasurement, Glob, measure_bytes, measure_text};

/// Compile a list of glob patterns once for repeated matching.
pub fn compile_globs(patterns: &[String]) -> Vec<Glob> {
    patterns.iter().map(Glob::new).collect()
}

/// Whether a repo-relative `/`-path is removed by `[scan].exclude`.
pub fn is_excluded(path: &str, exclude: &[Glob]) -> bool {
    exclude.iter().any(|glob| glob.matches(path))
}

/// Walk the configured scan scope and return repo-relative paths to measure,
/// after exclusion and gitignore filtering (§FS-001-config.2).
pub fn walk_scope(root: &Path, scan: &Scan) -> io::Result<Vec<String>> {
    let exclude = compile_globs(&scan.exclude);
    let mut out = Vec::new();

    for entry in &scan.include {
        if has_glob_meta(entry) {
            let include = Glob::new(entry);
            let base = root.join(glob_literal_prefix(entry));
            if base.is_file() {
                if let Some(rel) = relative(root, &base)
                    && include.matches(&rel)
                {
                    push_if_kept(&mut out, &rel, &exclude);
                }
            } else if base.is_dir() {
                walk_matching(root, &base, &include, &exclude, &mut out)?;
            }
        } else {
            let full = root.join(entry);
            if full.is_file() {
                push_if_kept(&mut out, entry, &exclude);
            } else if full.is_dir() {
                walk_dir(root, &full, &exclude, &mut out)?;
            }
        }
    }

    if scan.respect_gitignore {
        filter_gitignored(root, &mut out);
    }

    out.sort();
    out.dedup();
    Ok(out)
}

fn walk_matching(
    root: &Path,
    dir: &Path,
    include: &Glob,
    exclude: &[Glob],
    out: &mut Vec<String>,
) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let Some(rel) = relative(root, &path) else {
            continue;
        };
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            if rel == ".git" || is_excluded(&format!("{rel}/"), exclude) {
                continue;
            }
            walk_matching(root, &path, include, exclude, out)?;
        } else if file_type.is_file() && include.matches(&rel) {
            push_if_kept(out, &rel, exclude);
        }
    }
    Ok(())
}

fn walk_dir(root: &Path, dir: &Path, exclude: &[Glob], out: &mut Vec<String>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let Some(rel) = relative(root, &path) else {
            continue;
        };
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            // Skip VCS metadata always, and prune directories whose whole subtree
            // is excluded (e.g. `target/**`).
            if rel == ".git" || is_excluded(&format!("{rel}/"), exclude) {
                continue;
            }
            walk_dir(root, &path, exclude, out)?;
        } else if file_type.is_file() {
            push_if_kept(out, &rel, exclude);
        }
    }
    Ok(())
}

fn has_glob_meta(path: &str) -> bool {
    path.contains(['*', '?', '['])
}

fn glob_literal_prefix(pattern: &str) -> PathBuf {
    let mut prefix = PathBuf::new();
    for segment in pattern.split('/') {
        if segment.is_empty() || has_glob_meta(segment) {
            break;
        }
        prefix.push(segment);
    }
    prefix
}

fn push_if_kept(out: &mut Vec<String>, rel: &str, exclude: &[Glob]) {
    if !is_excluded(rel, exclude) {
        out.push(rel.to_owned());
    }
}

fn relative(root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    Some(rel.to_string_lossy().replace('\\', "/"))
}

/// Normalize an explicit file path to the repo-relative `/` form used by rules
/// and exact exceptions (§FS-003-exceptions.3, §FS-004-check-audit.1).
pub fn normalize_repo_path(root: &Path, raw: &str) -> io::Result<String> {
    let raw = raw.replace('\\', "/");
    let path = Path::new(&raw);
    if path.is_absolute()
        || path
            .components()
            .any(|component| component == Component::ParentDir)
    {
        let root = root.canonicalize()?;
        let full = if path.is_absolute() {
            path.to_path_buf()
        } else {
            root.join(path)
        }
        .canonicalize()?;
        return relative(&root, &full).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("path `{raw}` is outside the repository"),
            )
        });
    }

    clean_relative_path(path).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("path `{raw}` is not a repo-relative file path"),
        )
    })
}

fn clean_relative_path(path: &Path) -> Option<String> {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => out.push(part),
            Component::ParentDir => {
                out.pop();
            }
            Component::Prefix(_) | Component::RootDir => return None,
        }
    }
    if out.as_os_str().is_empty() {
        None
    } else {
        Some(out.to_string_lossy().replace('\\', "/"))
    }
}

/// The git-staged file set for `check --staged` (§FS-004-check-audit.1). Deleted
/// paths are dropped: there is nothing left to measure. Returns the paths with
/// `[scan].exclude` applied.
pub fn staged_files(root: &Path, scan: &Scan) -> io::Result<Vec<String>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["diff", "--cached", "--name-only", "--diff-filter=ACMR"])
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other("git diff --cached failed"));
    }

    let exclude = compile_globs(&scan.exclude);
    let mut out: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !is_excluded(line, &exclude))
        .map(str::to_owned)
        .collect();
    out.sort();
    out.dedup();
    Ok(out)
}

fn filter_gitignored(root: &Path, paths: &mut Vec<String>) {
    if paths.is_empty() {
        return;
    }
    let mut child = Command::new("git");
    child
        .arg("-C")
        .arg(root)
        .args(["check-ignore", "--stdin"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());

    let Ok(mut handle) = child.spawn() else {
        return; // git unavailable: best-effort, keep the exclusion-only result.
    };
    if let Some(mut stdin) = handle.stdin.take() {
        use io::Write;
        let _ = stdin.write_all(paths.join("\n").as_bytes());
    }
    let Ok(output) = handle.wait_with_output() else {
        return;
    };
    // exit 0 = some ignored, 1 = none ignored; anything else: leave list as is.
    if !matches!(output.status.code(), Some(0 | 1)) {
        return;
    }
    let ignored: std::collections::HashSet<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_owned)
        .collect();
    paths.retain(|path| !ignored.contains(path));
}

/// Measure one repo-relative file. UTF-8 files are line-classified; others are
/// measured by bytes only. Token counts come from the opt-in external command
/// (§DA-001-token-external-command).
pub fn measure_file(root: &Path, rel: &str, tokens: &Tokens) -> io::Result<FileMeasurement> {
    let bytes = fs::read(root.join(rel))?;
    let mut measurement = measure_content(rel, &bytes);
    if tokens.enabled
        && let Some(count) = run_token_command(root, &tokens.command, rel)?
    {
        measurement = measurement.with_tokens(count);
    }
    Ok(measurement)
}

/// Measure a staged blob for `check --staged` (§FS-004-check-audit.1).
pub fn measure_staged_file(root: &Path, rel: &str, tokens: &Tokens) -> io::Result<FileMeasurement> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .arg("show")
        .arg(format!(":{rel}"))
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!("git show failed for {rel}")));
    }
    let mut measurement = measure_content(rel, &output.stdout);
    if tokens.enabled
        && let Some(count) =
            run_token_command_for_staged_bytes(root, &tokens.command, rel, &output.stdout)?
    {
        measurement = measurement.with_tokens(count);
    }
    Ok(measurement)
}

fn measure_content(rel: &str, bytes: &[u8]) -> FileMeasurement {
    match std::str::from_utf8(bytes) {
        Ok(text) => measure_text(rel, text),
        Err(_) => measure_bytes(rel, bytes),
    }
}

/// Run `[tokens].command` with `{path}` substituted and parse one integer
/// (§FS-001-config.7).
fn run_token_command(root: &Path, command: &[String], rel: &str) -> io::Result<Option<u64>> {
    run_token_command_with_path(root, command, rel, rel)
}

fn run_token_command_for_staged_bytes(
    root: &Path,
    command: &[String],
    rel: &str,
    bytes: &[u8],
) -> io::Result<Option<u64>> {
    if command.is_empty() {
        return Ok(None);
    }
    let path = staged_temp_path(rel);
    fs::write(&path, bytes)?;
    let path_arg = path.to_string_lossy().into_owned();
    let result = run_token_command_with_path(root, command, &path_arg, rel);
    let _ = fs::remove_file(path);
    result
}

fn staged_temp_path(rel: &str) -> PathBuf {
    let sanitized: String = rel
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect();
    std::env::temp_dir().join(format!("fissile-staged-{}-{sanitized}", std::process::id()))
}

fn run_token_command_with_path(
    root: &Path,
    command: &[String],
    path_arg: &str,
    rel: &str,
) -> io::Result<Option<u64>> {
    let Some((program, rest)) = command.split_first() else {
        return Ok(None);
    };
    let args: Vec<String> = rest
        .iter()
        .map(|arg| arg.replace("{path}", path_arg))
        .collect();
    let output = Command::new(program)
        .args(&args)
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!("token command failed for {rel}")));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.split_whitespace()
        .next()
        .and_then(|token| token.parse::<u64>().ok())
        .map(|count| Ok(Some(count)))
        .unwrap_or_else(|| {
            Err(io::Error::other(format!(
                "token command did not print an integer for {rel}"
            )))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn globs(patterns: &[&str]) -> Vec<Glob> {
        patterns.iter().map(|p| Glob::new(*p)).collect()
    }

    fn temp_root() -> std::path::PathBuf {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("fissile-scan-{}-{n}", std::process::id()))
    }

    #[test]
    fn exclusion_matches_subtrees_and_extensions() {
        let exclude = globs(&["target/**", "**/*.lock"]);
        assert!(is_excluded("target/debug/app", &exclude));
        assert!(is_excluded("Cargo.lock", &exclude));
        assert!(!is_excluded("src/lib.rs", &exclude));
    }

    #[test]
    fn directory_prune_test_matches_doublestar_excludes() {
        let exclude = globs(&["target/**"]);
        assert!(is_excluded("target/", &exclude));
        assert!(!is_excluded("src/", &exclude));
    }

    #[test]
    fn walk_scope_expands_include_globs() {
        let root = temp_root();
        fs::create_dir_all(root.join("src/nested")).unwrap();
        fs::write(root.join("src/lib.rs"), "fn lib() {}\n").unwrap();
        fs::write(root.join("src/nested/mod.rs"), "fn nested() {}\n").unwrap();
        fs::write(root.join("src/readme.md"), "# src\n").unwrap();

        let paths = walk_scope(
            &root,
            &Scan {
                include: vec!["src/**/*.rs".to_owned()],
                exclude: Vec::new(),
                respect_gitignore: false,
            },
        )
        .expect("walks scope");

        assert_eq!(paths, vec!["src/lib.rs", "src/nested/mod.rs"]);
        let _ = fs::remove_dir_all(root);
    }
}
