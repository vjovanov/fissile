//! Managed pre-commit hook install for `fissile init` (§FS-002-init.6): a
//! marker-delimited block inside `.git/hooks/pre-commit` that composes with hooks
//! a project already maintains, refreshed in place like the agent block (§FS-002-init.4).

use std::fs;
use std::io;
use std::path::Path;

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

/// The hook path under a repo root.
fn hook_path(root: &Path) -> std::path::PathBuf {
    root.join(".git/hooks/pre-commit")
}

/// Whether `<root>/.git` is a directory we can install a hook into. Automatic
/// mode skips when this is false; `--hook` turns the same condition into an error
/// (§FS-002-init.6).
pub fn is_git_repo(root: &Path) -> bool {
    root.join(".git").is_dir()
}

/// Install or refresh the managed pre-commit hook (§FS-002-init.6). The caller
/// has already decided that a hook should be installed.
pub fn install(root: &Path, dry_run: bool) -> Result<Outcome, InitError> {
    let path = hook_path(root);
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
    Block {
        begin_prefix: BEGIN_PREFIX,
        end_prefix: END_PREFIX,
        version: SUPPORTED_VERSION,
        body: BLOCK,
        // A shell file has no heading to state a version on, so the marker
        // carries it — the shape `conda init` writes (§FS-002-init.6).
        version_heading: None,
    }
    .apply(existing, path)
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

    #[test]
    fn rejects_newer_block_version() {
        let existing = "#!/bin/sh\n# >>> fissile managed block (v2) >>>\nfuture\n# <<< fissile managed block (v2) <<<\n";
        let error = apply_block(existing, Path::new("pre-commit")).expect_err("v2 unsupported");
        assert!(matches!(
            error,
            InitError::UnsupportedBlock { version: 2, .. }
        ));
    }
}
