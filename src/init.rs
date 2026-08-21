//! `fissile init` — install config, exception registries, and the managed agent
//! block, fully populated at their defaults (§FS-002-init, §DF-002-explicit-config).
//! Project-owned files are never overwritten.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::managed::Block;

/// The fully-populated starter config written by `init` (§DF-002-explicit-config).
pub const DEFAULT_CONFIG: &str = include_str!("templates/fissile.default.toml");

/// Starter soft (agent) exception registry (§FS-003-exceptions).
pub const DEFAULT_SOFT_REGISTRY: &str = include_str!("templates/soft-exceptions.toml");

/// Starter hard (human) exception registry (§FS-003-exceptions).
pub const DEFAULT_HARD_REGISTRY: &str = include_str!("templates/hard-exceptions.toml");

/// The canonical managed agent-instruction block, markers included
/// (§FS-002-init.4).
pub const MANAGED_BLOCK: &str = include_str!("templates/agents-block.md");

const BLOCK_BEGIN: &str = "<!-- BEGIN FISSILE MANAGED BLOCK -->";
const BLOCK_END: &str = "<!-- END FISSILE MANAGED BLOCK -->";
/// The block's own heading, which states its version and — in a v1 or v2 file,
/// written before the markers existed — was its only boundary (§FS-002-init.4).
const BLOCK_HEADING: &str = "## Keeping Files Small With fissile (v";
const SUPPORTED_BLOCK_VERSION: u32 = 3;

/// How `init` finds, versions, and rewrites the agent block (§FS-002-init.4).
fn agent_block() -> Block<'static> {
    Block {
        begin_prefix: BLOCK_BEGIN,
        end_prefix: BLOCK_END,
        version: SUPPORTED_BLOCK_VERSION,
        body: MANAGED_BLOCK.trim_end(),
        version_heading: Some(BLOCK_HEADING),
    }
}

/// Which agent entrypoint families to write (§FS-002-init.3).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AgentTargets {
    pub agents_md: bool,
    pub claude: bool,
    pub gemini: bool,
    pub copilot: bool,
    pub cursor: bool,
    pub windsurf: bool,
    pub zed: bool,
}

impl AgentTargets {
    fn any(&self) -> bool {
        self.agents_md
            || self.claude
            || self.gemini
            || self.copilot
            || self.cursor
            || self.windsurf
            || self.zed
    }

    /// Entrypoint files requested by explicit flags, relative to the repo root.
    fn explicit_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if self.agents_md {
            paths.push(PathBuf::from("AGENTS.md"));
        }
        if self.claude {
            paths.push(PathBuf::from("CLAUDE.md"));
        }
        if self.gemini {
            paths.push(PathBuf::from("GEMINI.md"));
        }
        if self.copilot {
            paths.push(PathBuf::from(".github/copilot-instructions.md"));
        }
        if self.cursor {
            paths.push(PathBuf::from(".cursor/rules/fissile.mdc"));
        }
        if self.windsurf {
            paths.push(PathBuf::from(".windsurfrules"));
        }
        if self.zed {
            paths.push(PathBuf::from(".rules"));
        }
        paths
    }
}

/// Whether `init` installs the managed pre-commit hook (§FS-002-init.6).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HookMode {
    /// Install when the target is a git repository; skip silently otherwise.
    #[default]
    Auto,
    /// Always install; error when the target is not a git repository (`--hook`).
    Always,
    /// Never install (`--no-hook`).
    Never,
}

/// Inputs to an `init` run (§FS-002-init.1).
#[derive(Clone, Debug)]
pub struct InitOptions {
    pub root: PathBuf,
    pub config_path: PathBuf,
    /// Project name for a freshly created `AGENTS.md` heading; defaults to the
    /// target directory basename (§FS-002-init.1).
    pub name: Option<String>,
    pub exceptions: bool,
    pub force: bool,
    pub dry_run: bool,
    pub agents: AgentTargets,
    /// Pre-commit hook install policy (§FS-002-init.6).
    pub hook: HookMode,
}

impl InitOptions {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            config_path: PathBuf::from(".agents/fissile.toml"),
            name: None,
            exceptions: false,
            force: false,
            dry_run: false,
            agents: AgentTargets::default(),
            hook: HookMode::Auto,
        }
    }
}

/// What `init` did to one file (§FS-002-init.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Wrote,
    Appended,
    Updated,
    Exists,
}

impl Action {
    fn prefix(self, dry_run: bool) -> &'static str {
        match (self, dry_run) {
            (Action::Wrote, false) => "wrote",
            (Action::Wrote, true) => "would-write",
            (Action::Appended, false) => "appended",
            (Action::Appended, true) => "would-append",
            (Action::Updated, false) => "updated",
            (Action::Updated, true) => "would-update",
            (Action::Exists, _) => "exists",
        }
    }

    pub(crate) fn changed(self) -> bool {
        !matches!(self, Action::Exists)
    }
}

/// One reported path and what happened to it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub path: PathBuf,
    pub action: Action,
}

/// The full result of an `init` run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    pub outcomes: Vec<Outcome>,
    pub dry_run: bool,
    /// Automatic hook install found no git repository (§FS-002-init.6); the
    /// `next:` block must not promise a hook that was not written (§FS-002-init.5).
    pub hook_skipped_not_git: bool,
    /// Agent entrypoints this run touched, in reported order (§FS-002-init.3).
    /// The `next:` block names one of these rather than a fixed filename, so it
    /// cannot point at a file that was never written (§FS-002-init.5).
    pub entrypoints: Vec<PathBuf>,
}

impl Report {
    /// Whether the run changed anything; drives the `next:` block (§FS-002-init.5).
    pub fn changed_anything(&self) -> bool {
        self.outcomes.iter().any(|outcome| outcome.action.changed())
    }

    /// Render the per-path report lines plus an optional `next:` block.
    pub fn render(&self) -> String {
        let mut lines = Vec::new();
        for outcome in &self.outcomes {
            lines.push(format!(
                "{} {}",
                outcome.action.prefix(self.dry_run),
                outcome.path.display()
            ));
        }
        if self.changed_anything() {
            let hook_step = if self.hook_skipped_not_git {
                NEXT_HOOK_STEP_NO_GIT
            } else {
                NEXT_HOOK_STEP
            };
            let mut block = format!(
                "next:\n\
                 1. Review .agents/fissile.toml: the source rule budgets common code extensions; \
                 add this repo's languages or tune the limits.\n\
                 2. {hook_step}\n\
                 3. Run fissile audit once and add justified exceptions with fissile exception add."
            );
            // Omitted rather than invented when no entrypoint was written
            // (§FS-002-init.5).
            if let Some(entrypoint) = self.entrypoints.first() {
                block.push_str(&format!(
                    "\nsee {} for what agents are told; the findings carry the rest.",
                    entrypoint.display()
                ));
            }
            lines.push(block);
        }
        lines.join("\n")
    }
}

const NEXT_HOOK_STEP: &str =
    "Commit a change to see the pre-commit hook run fissile check --staged.";
const NEXT_HOOK_STEP_NO_GIT: &str = "Run git init && fissile init to install the pre-commit hook, \
    or wire fissile check --staged into your commit flow.";

/// A failure during `init`.
#[derive(Debug)]
pub enum InitError {
    Io(io::Error),
    /// `None` when the block declares no version this build can read: a later
    /// generation renamed the heading that carries it (§FS-002-init.4).
    UnsupportedBlock {
        path: PathBuf,
        version: Option<u32>,
    },
    NotAGitRepo {
        root: PathBuf,
    },
}

impl std::fmt::Display for InitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InitError::Io(error) => write!(f, "{error}"),
            InitError::UnsupportedBlock {
                path,
                version: Some(version),
            } => write!(
                f,
                "{} has an unsupported managed block version v{version}; this build writes v{SUPPORTED_BLOCK_VERSION}",
                path.display()
            ),
            InitError::UnsupportedBlock {
                path,
                version: None,
            } => write!(
                f,
                "{} has a managed block that declares no version this build can read; this build writes v{SUPPORTED_BLOCK_VERSION}",
                path.display()
            ),
            InitError::NotAGitRepo { root } => write!(
                f,
                "{} is not a git repository; cannot install the pre-commit hook",
                root.display()
            ),
        }
    }
}

impl std::error::Error for InitError {}

impl From<io::Error> for InitError {
    fn from(error: io::Error) -> Self {
        InitError::Io(error)
    }
}

/// Run `init` against the filesystem.
pub fn run(options: &InitOptions) -> Result<Report, InitError> {
    let mut outcomes = Vec::new();

    // 1. Config — written when absent, never overwritten (§FS-002-init.2).
    outcomes.push(write_new_file(
        &options.root.join(&options.config_path),
        DEFAULT_CONFIG,
        options.dry_run,
    )?);

    // 2. Exception registries, only with --exceptions (§FS-002-init.2). The
    //    paths come from the generated config so they stay in lockstep with it.
    if options.exceptions {
        let config = crate::config::Config::built_in();
        outcomes.push(write_new_file(
            &options.root.join(&config.exceptions.soft_registry),
            DEFAULT_SOFT_REGISTRY,
            options.dry_run,
        )?);
        outcomes.push(write_new_file(
            &options.root.join(&config.exceptions.hard_registry),
            DEFAULT_HARD_REGISTRY,
            options.dry_run,
        )?);
    }

    // 3. Agent entrypoints and managed blocks (§FS-002-init.3).
    let name = project_name(options);
    let mut entrypoints = Vec::new();
    for relative in resolve_entrypoints(&options.root, &options.agents) {
        outcomes.push(write_managed_block(
            &options.root.join(&relative),
            &name,
            options.dry_run,
        )?);
        entrypoints.push(relative);
    }

    // 4. Managed pre-commit hook (§FS-002-init.6). Automatic mode installs only
    //    inside a git repo; `--hook` forces it; `--no-hook` opts out.
    let mut hook_skipped_not_git = false;
    match options.hook {
        HookMode::Always if !crate::hook::is_git_repo(&options.root) => {
            return Err(InitError::NotAGitRepo {
                root: options.root.clone(),
            });
        }
        HookMode::Always => {
            outcomes.push(crate::hook::install(&options.root, options.dry_run)?);
        }
        HookMode::Auto if crate::hook::is_git_repo(&options.root) => {
            outcomes.push(crate::hook::install(&options.root, options.dry_run)?);
        }
        HookMode::Auto => hook_skipped_not_git = true,
        HookMode::Never => {}
    }

    Ok(Report {
        outcomes,
        dry_run: options.dry_run,
        hook_skipped_not_git,
        entrypoints,
    })
}

/// The project name for a fresh `AGENTS.md` heading: the `--name` value, else the
/// target directory basename (§FS-002-init.1).
fn project_name(options: &InitOptions) -> String {
    if let Some(name) = &options.name {
        return name.clone();
    }
    options
        .root
        .canonicalize()
        .ok()
        .as_deref()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .unwrap_or_else(|| "project".to_owned())
}

fn is_agents_md(path: &Path) -> bool {
    path.file_name().and_then(|name| name.to_str()) == Some("AGENTS.md")
}

/// Decide which entrypoint files to touch (§FS-002-init.3).
fn resolve_entrypoints(root: &Path, agents: &AgentTargets) -> Vec<PathBuf> {
    if agents.any() {
        return agents.explicit_paths();
    }

    // Automatic mode: update known existing entrypoints; if none exist, fall
    // back to the canonical AGENTS.md.
    const KNOWN: &[&str] = &[
        "AGENTS.md",
        "AGENTS.override.md",
        "CLAUDE.md",
        ".claude/CLAUDE.md",
        "GEMINI.md",
        ".github/copilot-instructions.md",
        ".cursor/rules/fissile.mdc",
        ".cursorrules",
        ".windsurfrules",
        ".rules",
    ];

    let mut paths: Vec<PathBuf> = KNOWN
        .iter()
        .map(PathBuf::from)
        .filter(|relative| root.join(relative).exists())
        .collect();

    // Workspace-triggered aliases: create when the tool's directory exists.
    for (dir, entry) in [
        (".claude", ".claude/CLAUDE.md"),
        (".gemini", "GEMINI.md"),
        (".cursor", ".cursor/rules/fissile.mdc"),
        (".zed", ".rules"),
    ] {
        let entry = PathBuf::from(entry);
        if root.join(dir).is_dir() && !paths.contains(&entry) {
            paths.push(entry);
        }
    }

    if paths.is_empty() {
        paths.push(PathBuf::from("AGENTS.md"));
    }
    paths
}

/// Write a file only if absent; report `exists` otherwise (§FS-002-init.2).
fn write_new_file(path: &Path, contents: &str, dry_run: bool) -> Result<Outcome, InitError> {
    if path.exists() {
        return Ok(Outcome {
            path: path.to_path_buf(),
            action: Action::Exists,
        });
    }
    if !dry_run {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, contents)?;
    }
    Ok(Outcome {
        path: path.to_path_buf(),
        action: Action::Wrote,
    })
}

/// Append, replace, or leave the managed block in an entrypoint (§FS-002-init.4).
fn write_managed_block(path: &Path, name: &str, dry_run: bool) -> Result<Outcome, InitError> {
    let existing = match fs::read_to_string(path) {
        Ok(text) => Some(text),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };

    let (new_contents, action) = match existing {
        // A fresh canonical AGENTS.md gets an unmanaged project H1 above the
        // block; companion entrypoints are block-only (§FS-002-init.4).
        None if is_agents_md(path) => (
            format!("# {name}\n\n{}\n", MANAGED_BLOCK.trim_end()),
            Action::Wrote,
        ),
        None => (format!("{}\n", MANAGED_BLOCK.trim_end()), Action::Wrote),
        Some(existing) => apply_managed_block(&existing, path)?,
    };

    if action.changed() && !dry_run {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, &new_contents)?;
    }

    Ok(Outcome {
        path: path.to_path_buf(),
        action,
    })
}

/// Compute the new file content after applying the managed block to existing
/// text. Returns the content and the action taken (§FS-002-init.4).
fn apply_managed_block(existing: &str, path: &Path) -> Result<(String, Action), InitError> {
    agent_block().apply(existing, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn default_config_is_valid_and_fully_populated() {
        // The generated config must parse and build a checker (§DF-002-explicit-config).
        let config = Config::parse(DEFAULT_CONFIG).expect("default config parses");
        config
            .to_checker()
            .expect("default config builds a checker");
        // Every default is spelled out: defaults are present, not implied.
        assert!(DEFAULT_CONFIG.contains("respect_gitignore"));
        assert!(DEFAULT_CONFIG.contains("[output]"));
        assert!(DEFAULT_CONFIG.contains("count_blank_lines"));
        assert!(DEFAULT_CONFIG.contains("count_comment_lines"));
    }

    #[test]
    fn starter_registries_parse() {
        assert!(DEFAULT_SOFT_REGISTRY.contains("fissile_exceptions_version = 2"));
        assert!(DEFAULT_HARD_REGISTRY.contains("fissile_exceptions_version = 2"));
    }

    fn report_for(entrypoints: &[&str]) -> Report {
        Report {
            outcomes: vec![Outcome {
                path: PathBuf::from(".agents/fissile.toml"),
                action: Action::Wrote,
            }],
            dry_run: false,
            hook_skipped_not_git: false,
            entrypoints: entrypoints.iter().map(PathBuf::from).collect(),
        }
    }

    /// The closing line names an entrypoint the run handled, so it cannot send
    /// the reader to a file that was never written (§FS-002-init.5).
    #[test]
    fn next_block_names_a_handled_entrypoint() {
        assert!(
            report_for(&["CLAUDE.md", ".claude/CLAUDE.md"])
                .render()
                .contains("see CLAUDE.md for what agents are told; the findings carry the rest.")
        );
        assert!(
            report_for(&["GEMINI.md"])
                .render()
                .contains("see GEMINI.md for what agents are told; the findings carry the rest.")
        );

        // Nothing to point at: the line is omitted, not invented.
        let rendered = report_for(&[]).render();
        assert!(rendered.contains("next:"));
        assert!(!rendered.contains("what agents are told"));
    }

    #[test]
    fn appends_block_to_existing_file() {
        let (result, action) =
            apply_managed_block("# My Project\n\nHello.\n", Path::new("AGENTS.md"))
                .expect("append succeeds");
        assert_eq!(action, Action::Appended);
        let opened = "# My Project\n\nHello.\n\n<!-- BEGIN FISSILE MANAGED BLOCK -->";
        assert!(result.starts_with(opened));
        assert!(result.contains("fissile check --staged"));
    }

    #[test]
    fn replaces_existing_managed_block_and_preserves_surroundings() {
        let existing = "# Project\n\n<!-- BEGIN FISSILE MANAGED BLOCK -->\n## Keeping Files Small With fissile (v3)\n\nold body\n<!-- END FISSILE MANAGED BLOCK -->\n\n## Other\n\nkeep me\n";
        let (result, action) =
            apply_managed_block(existing, Path::new("AGENTS.md")).expect("replace succeeds");
        assert_eq!(action, Action::Updated);
        assert!(result.contains("## Other\n\nkeep me"));
        assert!(!result.contains("old body"));
        assert!(result.starts_with("# Project\n"));
    }

    /// Markers carry no version, so a block whose heading this build cannot read
    /// leaves nothing to judge it by. Assuming it is current would overwrite a
    /// newer generation wholesale (§FS-002-init.4).
    #[test]
    fn rejects_a_delimited_block_that_declares_no_version() {
        let existing = "<!-- BEGIN FISSILE MANAGED BLOCK -->\n## Some Future Heading\n\nfuture body\n<!-- END FISSILE MANAGED BLOCK -->\n";
        let error = apply_managed_block(existing, Path::new("AGENTS.md"))
            .expect_err("an unreadable version is unsupported");
        assert!(matches!(
            error,
            InitError::UnsupportedBlock { version: None, .. }
        ));
    }

    #[test]
    fn rejects_newer_block_version() {
        let existing = "<!-- BEGIN FISSILE MANAGED BLOCK -->\n## Keeping Files Small With fissile (v4)\n\nfuture body\n<!-- END FISSILE MANAGED BLOCK -->\n";
        let error =
            apply_managed_block(existing, Path::new("AGENTS.md")).expect_err("v4 unsupported");
        assert!(matches!(
            error,
            InitError::UnsupportedBlock {
                version: Some(4),
                ..
            }
        ));
    }

    /// A v1/v2 block had no markers and ran to the next H1/H2. It is upgraded in
    /// place, not left beside a second block (§FS-002-init.4).
    #[test]
    fn upgrades_a_legacy_heading_block_in_place() {
        let existing = "# Project\n\n## Keeping Files Small With fissile (v2)\n\nold body\n\n## Other\n\nkeep me\n";
        let (result, action) =
            apply_managed_block(existing, Path::new("AGENTS.md")).expect("upgrade succeeds");
        assert_eq!(action, Action::Updated);
        assert!(!result.contains("old body"));
        assert!(!result.contains("(v2)"));
        let heading = "## Keeping Files Small With fissile";
        assert_eq!(result.matches(heading).count(), 1);
        assert!(result.contains("## Other\n\nkeep me"));
    }

    /// A begin marker with no end falls back to the heading rule — and the
    /// block's own heading must not end its own span (§FS-002-init.4).
    #[test]
    fn a_truncated_block_is_replaced_wholesale() {
        let existing = "<!-- BEGIN FISSILE MANAGED BLOCK -->\n## Keeping Files Small With fissile (v3)\n\nold body\n\n## Other\n\nkeep me\n";
        let (result, action) =
            apply_managed_block(existing, Path::new("AGENTS.md")).expect("replace succeeds");
        assert_eq!(action, Action::Updated);
        assert!(!result.contains("old body"));
        let heading = "## Keeping Files Small With fissile";
        assert_eq!(result.matches(heading).count(), 1);
        assert!(result.contains("## Other\n\nkeep me"));
    }

    /// The markers, not the next heading, bound the block: what a user writes
    /// under it is theirs (§FS-002-init.4).
    #[test]
    fn a_user_heading_below_the_block_survives_a_refresh() {
        let block = MANAGED_BLOCK.trim_end();
        let existing = format!("{block}\n\n### Our own note\n\nkeep me\n");
        let (result, _) =
            apply_managed_block(&existing, Path::new("AGENTS.md")).expect("refresh succeeds");
        assert!(result.contains("### Our own note\n\nkeep me"));
    }

    #[test]
    fn unchanged_block_reports_exists() {
        let existing = format!("# Project\n\n{}\n", MANAGED_BLOCK.trim_end());
        let (_, action) =
            apply_managed_block(&existing, Path::new("AGENTS.md")).expect("idempotent");
        assert_eq!(action, Action::Exists);
    }
}
