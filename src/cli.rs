//! Shared command plumbing for `check`, `audit`, and `exception`
//! (§FS-004-check-audit, §FS-005-exception-add): load the effective config,
//! build the checker, and load+validate both exception registries.

use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use crate::Checker;
use crate::config::{Color, Config, ConfigError, Format as ConfigFormat};
use crate::exceptions::{ExceptionError, Registries};
use crate::report::EvalError;

/// Output format for a finding stream (§FS-001-config.6).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Format {
    #[default]
    Text,
    Json,
}

impl From<ConfigFormat> for Format {
    fn from(format: ConfigFormat) -> Self {
        match format {
            ConfigFormat::Text => Format::Text,
            ConfigFormat::Json => Format::Json,
        }
    }
}

/// Whether text output should be ANSI-colored (§FS-001-config.6). JSON is never
/// colored; `--no-color` and `color = "never"` force plain; `"auto"` colors only
/// when stdout is a terminal.
pub fn use_color(color: Color, no_color: bool, format: Format) -> bool {
    if no_color || format == Format::Json {
        return false;
    }
    match color {
        Color::Always => true,
        Color::Never => false,
        Color::Auto => std::io::stdout().is_terminal(),
    }
}

/// The effective config, checker, and registries for one command invocation.
pub struct Loaded {
    pub config: Config,
    pub checker: Checker,
    pub registries: Registries,
    pub root: PathBuf,
    pub soft_registry: PathBuf,
    pub hard_registry: PathBuf,
}

/// Load and validate everything a `check`/`audit`/`exception` run needs.
pub fn load(root: &Path, config_path: Option<&Path>) -> Result<Loaded, CommandError> {
    let config = Config::load(root, config_path)?;
    let checker = config.to_checker()?;

    let soft_registry = PathBuf::from(&config.exceptions.soft_registry);
    let hard_registry = PathBuf::from(&config.exceptions.hard_registry);
    let soft_text = read_optional(&root.join(&soft_registry))?;
    let hard_text = read_optional(&root.join(&hard_registry))?;
    let registries = Registries::load(soft_text.as_deref(), hard_text.as_deref())?;
    registries.validate_against(checker.rules())?;

    Ok(Loaded {
        config,
        checker,
        registries,
        root: root.to_path_buf(),
        soft_registry,
        hard_registry,
    })
}

/// Read a file, mapping a missing file to `None` (§FS-003-exceptions: an absent
/// registry is an empty registry).
pub fn read_optional(path: &Path) -> Result<Option<String>, io::Error> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// Any failure that aborts a command before producing findings. These map to a
/// usage/schema exit code; standing findings are reported separately.
#[derive(Debug)]
pub enum CommandError {
    Config(ConfigError),
    Exceptions(ExceptionError),
    Eval(EvalError),
    Io(io::Error),
    Usage(String),
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandError::Config(error) => write!(f, "{error}"),
            CommandError::Exceptions(error) => write!(f, "{error}"),
            CommandError::Eval(error) => write!(f, "{error}"),
            CommandError::Io(error) => write!(f, "{error}"),
            CommandError::Usage(message) => write!(f, "{message}"),
        }
    }
}

impl Error for CommandError {}

impl From<ConfigError> for CommandError {
    fn from(error: ConfigError) -> Self {
        CommandError::Config(error)
    }
}

impl From<ExceptionError> for CommandError {
    fn from(error: ExceptionError) -> Self {
        CommandError::Exceptions(error)
    }
}

impl From<EvalError> for CommandError {
    fn from(error: EvalError) -> Self {
        CommandError::Eval(error)
    }
}

impl From<io::Error> for CommandError {
    fn from(error: io::Error) -> Self {
        CommandError::Io(error)
    }
}
