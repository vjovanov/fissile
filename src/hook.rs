//! Managed pre-commit hook install for `fissile init` (§FS-002-init.6): a
//! marker-delimited block inside the repository's `hooks/pre-commit` that composes with
//! hooks a project already maintains, refreshed in place like the agent block (§FS-002-init.4).

use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use crate::init::{Action, InitError, Outcome};
use crate::managed::Block;

const BEGIN_PREFIX: &str = "# >>> fissile managed block (v";
const END_PREFIX: &str = "# <<< fissile managed block (v";
const SUPPORTED_VERSION: u32 = 1;
const SHEBANG: &str = "#!/bin/sh";

/// The managed body, marker lines included (§FS-002-init.6).
const BLOCK: &str = "\
# >>> fissile managed block (v1) >>>
# Managed by `fissile init`; re-run init to update. Tune budgets in fissile.toml.
fissile check --staged || exit 1
# <<< fissile managed block (v1) <<<";

/// The repository's common git directory for `root`: `<root>/.git` itself if a
/// directory, else the directory its `gitdir:` pointer names, resolved through
/// that target's own `commondir` when present. `None` if unresolvable (§FS-002-init.6).
fn common_dir(root: &Path) -> Option<PathBuf> {
    let git = root.join(".git");
    if git.is_dir() {
        return Some(git);
    }

    let pointer = fs::read_to_string(&git).ok()?;
    let target = pointer.trim().strip_prefix("gitdir:")?.trim();
    let gitdir = resolve(root, target);
    if !gitdir.is_dir() {
        return None;
    }

    match fs::read_to_string(gitdir.join("commondir")) {
        Ok(commondir) => Some(resolve(&gitdir, commondir.trim())),
        // No `commondir`: `gitdir` is a submodule's own git directory, which is
        // already the common one.
        Err(_) => Some(gitdir),
    }
}

/// `relative` resolved against `base` when it is not itself absolute, with any
/// `..` collapsed lexically (no filesystem access, so a symlinked temp dir does
/// not change the answer) — the shape a `commondir` file of `../..` needs.
fn resolve(base: &Path, relative: &str) -> PathBuf {
    let relative = Path::new(relative);
    let joined = if relative.is_absolute() {
        relative.to_path_buf()
    } else {
        base.join(relative)
    };

    let mut normalized = PathBuf::new();
    for component in joined.components() {
        match component {
            Component::ParentDir => {
                normalized.pop();
            }
            Component::CurDir => {}
            other => normalized.push(other),
        }
    }
    normalized
}

/// The hook path under a repository's common git directory.
fn hook_path(common_dir: &Path) -> PathBuf {
    common_dir.join("hooks/pre-commit")
}

/// Whether `<root>/.git` names a git repository we can install a hook into.
/// Automatic mode skips when this is false; `--hook` turns the same condition
/// into an error (§FS-002-init.6).
pub fn is_git_repo(root: &Path) -> bool {
    common_dir(root).is_some()
}

/// How `init` finds, versions, and rewrites the hook block (§FS-002-init.6).
fn hook_block() -> Block<'static> {
    Block {
        begin_prefix: BEGIN_PREFIX,
        end_prefix: END_PREFIX,
        version: SUPPORTED_VERSION,
        body: BLOCK,
        // A shell file has no heading to state a version on, so the marker
        // carries it — the shape `conda init` writes (§FS-002-init.6).
        version_heading: None,
    }
}

/// Whether the hook file already carries the managed block, whichever version
/// wrote it. `--no-hook` declines to install one; it does not remove the one a
/// previous run left, and the report has to say so (§FS-002-init.5).
pub fn is_installed(root: &Path) -> bool {
    common_dir(root).is_some_and(|dir| {
        fs::read_to_string(hook_path(&dir)).is_ok_and(|hook| hook_block().is_present(&hook))
    })
}

/// Install or refresh the managed pre-commit hook (§FS-002-init.6). The caller
/// has already decided that a hook should be installed.
pub fn install(root: &Path, dry_run: bool) -> Result<Outcome, InitError> {
    let dir = common_dir(root).ok_or_else(|| InitError::NotAGitRepo {
        root: root.to_path_buf(),
    })?;
    let path = hook_path(&dir);
    let existing = match fs::read_to_string(&path) {
        Ok(text) => Some(text),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };

    let (contents, action) = match existing {
        None => (format!("{SHEBANG}\n{BLOCK}\n"), Action::Wrote),
        Some(existing) => apply_block(&existing, &path)?,
    };

    if action.changed() && !dry_run {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, &contents)?;
        make_executable(&path)?;
    }

    Ok(Outcome { path, action })
}

/// Append, replace, or leave the managed block in an existing hook file
/// (§FS-002-init.6), by the same splice the agent block uses (§FS-002-init.4).
fn apply_block(existing: &str, path: &Path) -> Result<(String, Action), InitError> {
    hook_block().apply(existing, path)
}

#[cfg(unix)]
fn make_executable(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms)
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scratch directory for one test, named after it so parallel tests never
    /// collide (§FS-002-init.6), following the project's own temp-dir pattern.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fissile-hook-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    /// A plain checkout's `.git` is a directory: today's behavior, unchanged.
    #[test]
    fn common_dir_of_a_plain_checkout_is_the_dot_git_directory() {
        let root = scratch("plain-checkout");
        fs::create_dir_all(root.join(".git")).unwrap();
        assert_eq!(common_dir(&root), Some(root.join(".git")));
        assert!(is_git_repo(&root));
    }

    /// A linked worktree's `.git` is a file naming its private gitdir, absolute
    /// (git's default), whose `commondir` names the shared one two levels up —
    /// and the pointer's line ending is `\r\n`, which the parse tolerates.
    #[test]
    fn common_dir_of_a_worktree_follows_an_absolute_gitdir_and_commondir() {
        let root = scratch("absolute-gitdir");
        let common = root.join("main-git");
        let gitdir = common.join("worktrees/branch");
        fs::create_dir_all(&gitdir).unwrap();
        fs::write(gitdir.join("commondir"), "../..\n").unwrap();

        let worktree = root.join("worktree");
        fs::create_dir_all(&worktree).unwrap();
        fs::write(
            worktree.join(".git"),
            format!("gitdir: {}\r\n", gitdir.display()),
        )
        .unwrap();

        assert_eq!(common_dir(&worktree), Some(common));
    }

    /// A `gitdir:` pointer relative to the worktree root (`worktree.useRelativePaths`),
    /// whose `commondir` is in turn relative to the gitdir it names.
    #[test]
    fn common_dir_resolves_a_relative_gitdir_pointer() {
        let root = scratch("relative-gitdir");
        let common = root.join("main/.git");
        let gitdir = common.join("worktrees/branch");
        fs::create_dir_all(&gitdir).unwrap();
        fs::write(gitdir.join("commondir"), "../..").unwrap();

        let worktree = root.join("branch");
        fs::create_dir_all(&worktree).unwrap();
        fs::write(
            worktree.join(".git"),
            "gitdir: ../main/.git/worktrees/branch\n",
        )
        .unwrap();

        assert_eq!(common_dir(&worktree), Some(common));
    }

    /// A `gitdir:` with no `commondir` file beside it — a submodule's shape —
    /// resolves to the gitdir itself rather than failing.
    #[test]
    fn common_dir_of_a_gitdir_without_commondir_is_the_gitdir_itself() {
        let root = scratch("no-commondir");
        let sub = root.join("sub");
        let gitdir = root.join("superproject-git/modules/sub");
        fs::create_dir_all(&sub).unwrap();
        fs::create_dir_all(&gitdir).unwrap();
        fs::write(sub.join(".git"), format!("gitdir: {}\n", gitdir.display())).unwrap();

        assert_eq!(common_dir(&sub), Some(gitdir));
    }

    /// A `gitdir:` pointer whose target does not exist is "not a repository" —
    /// never a panic.
    #[test]
    fn common_dir_of_a_dangling_gitdir_is_none() {
        let root = scratch("dangling-gitdir");
        fs::write(root.join(".git"), "gitdir: does-not-exist\n").unwrap();

        assert_eq!(common_dir(&root), None);
        assert!(!is_git_repo(&root));
    }

    /// No `.git` at all is not a repository.
    #[test]
    fn common_dir_with_no_dot_git_is_none() {
        let root = scratch("no-dot-git");
        assert_eq!(common_dir(&root), None);
        assert!(!is_git_repo(&root));
    }

    /// A run that installs nothing still reports the gate that is there
    /// (§FS-002-init.5), so presence is the question — not whether this build
    /// would leave the file untouched, which a stale or newer block would not.
    #[test]
    fn a_managed_block_is_seen_whatever_state_it_is_in() {
        assert!(hook_block().is_present(&format!("{SHEBANG}\n{BLOCK}\n")));
        let newer = "#!/bin/sh\n# >>> fissile managed block (v2) >>>\nfuture\n";
        assert!(hook_block().is_present(newer));
        assert!(!hook_block().is_present("#!/bin/sh\nrun-other-checks\n"));
    }

    #[test]
    fn appends_block_to_existing_hook() {
        let (result, action) =
            apply_block("#!/bin/sh\nrun-other-checks\n", Path::new("pre-commit"))
                .expect("append succeeds");
        assert_eq!(action, Action::Appended);
        assert!(result.starts_with("#!/bin/sh\nrun-other-checks\n\n# >>> fissile"));
        assert!(result.contains("fissile check --staged || exit 1"));
    }

    #[test]
    fn replaces_block_and_preserves_surroundings() {
        let existing = "#!/bin/sh\n# >>> fissile managed block (v1) >>>\nstale body\n# <<< fissile managed block (v1) <<<\ntrailing-check\n";
        let (result, action) =
            apply_block(existing, Path::new("pre-commit")).expect("replace succeeds");
        assert_eq!(action, Action::Updated);
        assert!(result.contains("trailing-check"));
        assert!(!result.contains("stale body"));
        assert!(result.starts_with("#!/bin/sh\n# >>> fissile"));
    }

    #[test]
    fn unchanged_block_reports_exists() {
        let existing = format!("{SHEBANG}\n{BLOCK}\n");
        let (_, action) = apply_block(&existing, Path::new("pre-commit")).expect("idempotent");
        assert_eq!(action, Action::Exists);
    }

    /// A begin marker whose end marker someone deleted. The heading rule would
    /// stop at the next `# ` comment and orphan the rest of the old body below
    /// the new block, in every later run; this one runs to EOF (§FS-002-init.6).
    #[test]
    fn replaces_a_truncated_block_wholesale() {
        let existing = "#!/bin/sh\n# >>> fissile managed block (v1) >>>\n# Managed by `fissile init`; re-run init to update.\nfissile check --staged || exit 1\n";
        let (result, action) =
            apply_block(existing, Path::new("pre-commit")).expect("replace succeeds");
        assert_eq!(action, Action::Updated);
        assert_eq!(result, format!("{SHEBANG}\n{BLOCK}\n"));
        assert_eq!(result.matches("fissile check --staged").count(), 1);
    }

    #[test]
    fn rejects_newer_block_version() {
        let existing = "#!/bin/sh\n# >>> fissile managed block (v2) >>>\nfuture\n# <<< fissile managed block (v2) <<<\n";
        let error = apply_block(existing, Path::new("pre-commit")).expect_err("v2 unsupported");
        assert!(matches!(
            error,
            InitError::UnsupportedBlock {
                version: Some(2),
                ..
            }
        ));
    }
}
