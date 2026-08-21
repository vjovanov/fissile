//! Agent entrypoints as links to one file (§DF-009-one-file-agents-read).
//! `AGENTS.md` holds the managed block; every other entrypoint `init` touches
//! links to it, so no two copies can drift apart (§FS-002-init.3).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The one entrypoint that holds real bytes (§FS-002-init.3).
pub const CANONICAL: &str = "AGENTS.md";

pub fn is_canonical(path: &Path) -> bool {
    path.file_name().and_then(|name| name.to_str()) == Some(CANONICAL)
}

/// The link target for a companion at `relative`: `AGENTS.md` beside the root,
/// `../AGENTS.md` from `.claude/`, and so on. Relative so that cloning or
/// moving the tree keeps it resolving (§FS-002-init.3).
pub fn target_from(relative: &Path) -> PathBuf {
    let depth = relative
        .parent()
        .map_or(0, |parent| parent.components().count());
    let mut target = PathBuf::new();
    for _ in 0..depth {
        target.push("..");
    }
    target.push(CANONICAL);
    target
}

/// What one companion entrypoint is to become (§FS-002-init.3).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Plan {
    /// Point it at the canonical file: nothing is lost by doing so.
    Link,
    /// It already points there.
    Linked,
    /// It holds bytes of its own. Keep the file and write the block into it.
    Keep,
}

/// Decide every companion's fate, and adopt a lone one, **before** the block is
/// written to `AGENTS.md`. Both answers depend on bytes the write would change:
/// a copy stops matching the moment the canonical file is updated, and a rename
/// laid over a freshly written `AGENTS.md` would take the block with it.
pub fn plan(root: &Path, companions: &[PathBuf], dry_run: bool) -> io::Result<Vec<Plan>> {
    let canonical = root.join(CANONICAL);
    let existing = fs::read_to_string(&canonical).ok();

    let mut plans = Vec::with_capacity(companions.len());
    for relative in companions {
        plans.push(classify(root, relative, existing.as_deref())?);
    }

    // A lone companion's bytes *are* what the project already told agents, so
    // they become the canonical file. Only when it is the only one: two files
    // with content of their own may disagree on purpose (§FS-002-init.3).
    if existing.is_none() {
        let owners: Vec<usize> = plans
            .iter()
            .enumerate()
            .filter(|(_, plan)| **plan == Plan::Keep)
            .map(|(index, _)| index)
            .collect();
        if let [only] = owners[..] {
            if !dry_run {
                fs::rename(root.join(&companions[only]), &canonical)?;
            }
            plans[only] = Plan::Link;
        }
    }

    Ok(plans)
}

fn classify(root: &Path, relative: &Path, canonical: Option<&str>) -> io::Result<Plan> {
    let path = root.join(relative);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Plan::Link),
        Err(error) => return Err(error),
    };

    if metadata.file_type().is_symlink() {
        // Ours already, or someone else's — a link we did not make is not ours
        // to redirect, so it is left exactly as it is.
        let target = target_from(relative);
        return Ok(if fs::read_link(&path).is_ok_and(|found| found == target) {
            Plan::Linked
        } else {
            Plan::Keep
        });
    }

    // A byte-identical copy loses nothing by becoming a link to the original.
    let mine = fs::read_to_string(&path)?;
    Ok(match canonical {
        Some(canonical) if canonical == mine => Plan::Link,
        _ => Plan::Keep,
    })
}

/// Lay the link down. `Ok(false)` means the filesystem refused — Windows without
/// Developer Mode — and the caller writes the block into a regular file instead,
/// because a duplicated block beats no instructions (§FS-002-init.3).
pub fn write_link(root: &Path, relative: &Path, dry_run: bool) -> io::Result<bool> {
    if dry_run {
        return Ok(true);
    }
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    // Replacing a copy: its bytes are already safe in the canonical file.
    let _ = fs::remove_file(&path);
    Ok(symlink(&target_from(relative), &path).is_ok())
}

#[cfg(unix)]
fn symlink(target: &Path, path: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(target, path)
}

#[cfg(windows)]
fn symlink(target: &Path, path: &Path) -> io::Result<()> {
    std::os::windows::fs::symlink_file(target, path)
}

#[cfg(not(any(unix, windows)))]
fn symlink(_target: &Path, _path: &Path) -> io::Result<()> {
    Err(io::Error::other("symbolic links are unsupported here"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A link is relative to the file that carries it, so the tree can move
    /// (§FS-002-init.3).
    #[test]
    fn a_nested_entrypoint_climbs_back_to_the_root() {
        assert_eq!(
            target_from(Path::new("CLAUDE.md")),
            PathBuf::from("AGENTS.md")
        );
        assert_eq!(
            target_from(Path::new(".claude/CLAUDE.md")),
            PathBuf::from("../AGENTS.md")
        );
        assert_eq!(
            target_from(Path::new(".cursor/rules/fissile.mdc")),
            PathBuf::from("../../AGENTS.md")
        );
    }

    #[test]
    fn the_canonical_file_is_recognized_by_name() {
        assert!(is_canonical(Path::new("AGENTS.md")));
        assert!(is_canonical(Path::new("/tmp/project/AGENTS.md")));
        assert!(!is_canonical(Path::new("CLAUDE.md")));
        assert!(!is_canonical(Path::new(".claude/CLAUDE.md")));
    }

    /// Two companions with bytes of their own may disagree on purpose, so
    /// neither is adopted and both are kept (§FS-002-init.3).
    #[test]
    fn adoption_needs_exactly_one_owner() {
        let root = temp_dir("adopt-two");
        fs::write(root.join("CLAUDE.md"), "house rules\n").unwrap();
        fs::write(root.join("GEMINI.md"), "different rules\n").unwrap();

        let companions = [PathBuf::from("CLAUDE.md"), PathBuf::from("GEMINI.md")];
        let plans = plan(&root, &companions, false).unwrap();

        assert_eq!(plans, vec![Plan::Keep, Plan::Keep]);
        assert!(!root.join(CANONICAL).exists());
    }

    /// One companion and no canonical file: its bytes become `AGENTS.md`, and
    /// nothing is lost (§FS-002-init.3).
    #[test]
    fn a_lone_companion_becomes_the_canonical_file() {
        let root = temp_dir("adopt-one");
        fs::write(root.join("CLAUDE.md"), "house rules\n").unwrap();

        let companions = [PathBuf::from("CLAUDE.md")];
        let plans = plan(&root, &companions, false).unwrap();

        assert_eq!(plans, vec![Plan::Link]);
        assert_eq!(
            fs::read_to_string(root.join(CANONICAL)).unwrap(),
            "house rules\n"
        );
    }

    /// A copy of the canonical file is a link waiting to happen; a file that
    /// says something else is not (§FS-002-init.3).
    #[test]
    fn a_copy_links_and_a_different_file_is_kept() {
        let root = temp_dir("adopt-copy");
        fs::write(root.join(CANONICAL), "the truth\n").unwrap();
        fs::write(root.join("CLAUDE.md"), "the truth\n").unwrap();
        fs::write(root.join("GEMINI.md"), "something else\n").unwrap();

        let companions = [PathBuf::from("CLAUDE.md"), PathBuf::from("GEMINI.md")];
        let plans = plan(&root, &companions, false).unwrap();

        assert_eq!(plans, vec![Plan::Link, Plan::Keep]);
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fissile-entrypoint-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
