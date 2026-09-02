//! `fissile` CLI entry point (§FS-006-cli): hand-rolled parsing, tiny
//! dependency tree (§GOAL-002-tiny-footprint, §DA-003-single-static-binary);
//! dispatches the commands (§FS-002-init, §FS-004-check-audit, §FS-005-exception-add).

use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;
use std::slice::Iter;

use fissile::audit::{self, AuditOptions};
use fissile::check::{self, CheckOptions};
use fissile::cli::Format;
use fissile::exception::{self, AddOptions, Rationale};
use fissile::exceptions::{Kind, MatchKind};
use fissile::init::{self, AgentTargets, HookMode, InitOptions};
use fissile::measure::{self, MeasureOptions};
use fissile::remove::{self, RemoveOptions};
use fissile::retune::{self, RetuneOptions};
use fissile::{Severity, Unit};

mod usage;

use usage::{
    AUDIT_USAGE, CHECK_USAGE, EXCEPTION_ADD_USAGE, EXCEPTION_REMOVE_USAGE, EXCEPTION_RETUNE_USAGE,
    EXCEPTION_USAGE, INIT_USAGE, MEASURE_USAGE, USAGE,
};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("init") => run_init(&args[1..]),
        Some("check") => run_check(&args[1..]),
        Some("measure") => run_measure(&args[1..]),
        Some("audit") => run_audit(&args[1..]),
        Some("exception") => run_exception(&args[1..]),
        Some("--help" | "-h") | None => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        // One stable line, no banner (§FS-006-cli.3).
        Some("--version" | "-V") => {
            println!("fissile {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("fissile: unknown command `{other}`\n\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

/// Pull the value of a flag that takes an argument, or report a usage error.
fn value(iter: &mut Iter<String>, flag: &str) -> Result<String, String> {
    iter.next()
        .cloned()
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn parse_format(raw: &str) -> Result<Format, String> {
    match raw {
        "text" => Ok(Format::Text),
        "json" => Ok(Format::Json),
        other => Err(format!("unknown format `{other}` (expected text or json)")),
    }
}

fn run_init(args: &[String]) -> ExitCode {
    let mut options = InitOptions::new(".");
    let mut agents = AgentTargets::default();
    let mut iter = args.iter();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                println!("{INIT_USAGE}");
                return ExitCode::SUCCESS;
            }
            "--name" => match value(&mut iter, "--name") {
                Ok(name) => options.name = Some(name),
                Err(message) => return usage_fail("init", &message, INIT_USAGE),
            },
            "--config" => match value(&mut iter, "--config") {
                Ok(path) => options.config_path = PathBuf::from(path),
                Err(message) => return usage_fail("init", &message, INIT_USAGE),
            },
            "--exceptions" => options.exceptions = true,
            "--hook" => {
                if options.hook == HookMode::Never {
                    return usage_fail("init", "--hook conflicts with --no-hook", INIT_USAGE);
                }
                options.hook = HookMode::Always;
            }
            "--no-hook" => {
                if options.hook == HookMode::Always {
                    return usage_fail("init", "--no-hook conflicts with --hook", INIT_USAGE);
                }
                options.hook = HookMode::Never;
            }
            "--force" => options.force = true,
            "--dry-run" => options.dry_run = true,
            "--agents-md" => agents.agents_md = true,
            "--claude" => agents.claude = true,
            "--gemini" => agents.gemini = true,
            "--copilot" => agents.copilot = true,
            "--cursor" => agents.cursor = true,
            "--windsurf" => agents.windsurf = true,
            "--zed" => agents.zed = true,
            other if other.starts_with('-') => {
                return usage_fail("init", &format!("unknown option `{other}`"), INIT_USAGE);
            }
            other => options.root = PathBuf::from(other),
        }
    }

    options.agents = agents;
    let dry_run = options.dry_run;
    match init::run(&options) {
        Ok(report) => {
            eprintln!("{}", report.render());
            // A dry run is the one way to read the instructions without writing
            // a file, so it prints them — on stdout, keeping the planned writes
            // on stderr separable (§FS-002-init.4).
            if dry_run {
                println!("{}", init::MANAGED_BLOCK.trim_end());
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("fissile init: {error}");
            ExitCode::FAILURE
        }
    }
}

/// The flags `check` and `measure` share: one file-set selection and one set of
/// output controls, parsed once so the two commands cannot drift on what
/// `--staged` or `--format` mean (§FS-007-measure.1, §FS-006-cli.1).
#[derive(Default)]
struct FileSetArgs {
    config_path: Option<PathBuf>,
    staged: bool,
    format: Option<Format>,
    no_color: bool,
    paths: Vec<String>,
}

/// `Ok(None)` means `--help` was handled and the command is done.
fn parse_file_set(
    command: &str,
    usage: &str,
    args: &[String],
) -> Result<Option<FileSetArgs>, ExitCode> {
    let mut parsed = FileSetArgs::default();
    let mut iter = args.iter();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                println!("{usage}");
                return Ok(None);
            }
            "--staged" => parsed.staged = true,
            "--no-color" => parsed.no_color = true,
            "--config" => match value(&mut iter, "--config") {
                Ok(path) => parsed.config_path = Some(PathBuf::from(path)),
                Err(message) => return Err(usage_fail(command, &message, usage)),
            },
            "--format" => match value(&mut iter, "--format").and_then(|raw| parse_format(&raw)) {
                Ok(format) => parsed.format = Some(format),
                Err(message) => return Err(usage_fail(command, &message, usage)),
            },
            other if other.starts_with('-') => {
                return Err(usage_fail(
                    command,
                    &format!("unknown option `{other}`"),
                    usage,
                ));
            }
            other => parsed.paths.push(other.to_owned()),
        }
    }
    Ok(Some(parsed))
}

fn run_check(args: &[String]) -> ExitCode {
    let parsed = match parse_file_set("check", CHECK_USAGE, args) {
        Ok(Some(parsed)) => parsed,
        Ok(None) => return ExitCode::SUCCESS,
        Err(code) => return code,
    };
    let options = CheckOptions {
        root: PathBuf::from("."),
        config_path: parsed.config_path,
        staged: parsed.staged,
        format: parsed.format,
        no_color: parsed.no_color,
        paths: parsed.paths,
    };

    match check::run(&options) {
        Ok(run) => {
            // Under `--format json` stdout is holding a machine shape, so the
            // run's account of a registry it can no longer read comes out here
            // rather than being lost (§FS-004-check-audit.5).
            for note in &run.notes {
                eprintln!("{note}");
            }
            finish_run("check", &run.output, run.failed, &run.errors)
        }
        Err(error) => {
            eprintln!("fissile check: {error}");
            ExitCode::from(2)
        }
    }
}

fn run_measure(args: &[String]) -> ExitCode {
    let parsed = match parse_file_set("measure", MEASURE_USAGE, args) {
        Ok(Some(parsed)) => parsed,
        Ok(None) => return ExitCode::SUCCESS,
        Err(code) => return code,
    };
    let options = MeasureOptions {
        root: PathBuf::from("."),
        config_path: parsed.config_path,
        staged: parsed.staged,
        format: parsed.format,
        no_color: parsed.no_color,
        paths: parsed.paths,
    };

    match measure::run(&options) {
        // Never `failed`: measuring is inspection, and a file over a hard limit
        // is an answer rather than a verdict (§FS-007-measure.1).
        Ok(run) => finish_run("measure", &run.output, false, &run.errors),
        Err(error) => {
            eprintln!("fissile measure: {error}");
            ExitCode::from(2)
        }
    }
}

/// Print a run's findings and its file-level errors, then map them to the exit
/// code: errors exit 2 even without a standing hard finding, because silently
/// passing an unmeasurable file would make the gate unsound (§FS-004-check-audit.5).
fn finish_run(command: &str, output: &str, failed: bool, errors: &[String]) -> ExitCode {
    if !output.is_empty() {
        println!("{output}");
    }
    for error in errors {
        eprintln!("fissile {command}: {error}");
    }
    if !errors.is_empty() {
        ExitCode::from(2)
    } else if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn run_audit(args: &[String]) -> ExitCode {
    let mut options = AuditOptions {
        root: PathBuf::from("."),
        ..AuditOptions::default()
    };
    let mut iter = args.iter();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                println!("{AUDIT_USAGE}");
                return ExitCode::SUCCESS;
            }
            "--no-color" => options.no_color = true,
            "--stale-exceptions" => options.stale_exceptions = true,
            "--rule-coverage" => options.rule_coverage = true,
            "--config" => match value(&mut iter, "--config") {
                Ok(path) => options.config_path = Some(PathBuf::from(path)),
                Err(message) => return usage_fail("audit", &message, AUDIT_USAGE),
            },
            "--format" => match value(&mut iter, "--format").and_then(|raw| parse_format(&raw)) {
                Ok(format) => options.format = Some(format),
                Err(message) => return usage_fail("audit", &message, AUDIT_USAGE),
            },
            "--top" => match value(&mut iter, "--top").and_then(parse_count) {
                Ok(count) => options.top = Some(count),
                Err(message) => return usage_fail("audit", &message, AUDIT_USAGE),
            },
            other if other.starts_with('-') => {
                return usage_fail("audit", &format!("unknown option `{other}`"), AUDIT_USAGE);
            }
            other => {
                return usage_fail(
                    "audit",
                    &format!("unexpected argument `{other}`"),
                    AUDIT_USAGE,
                );
            }
        }
    }

    match audit::run(&options) {
        Ok(run) => finish_run("audit", &run.output, run.failed, &run.errors),
        Err(error) => {
            eprintln!("fissile audit: {error}");
            ExitCode::from(2)
        }
    }
}

fn parse_count(raw: String) -> Result<usize, String> {
    raw.parse()
        .map_err(|_| format!("`{raw}` is not a non-negative integer"))
}

fn run_exception(args: &[String]) -> ExitCode {
    match args.first().map(String::as_str) {
        Some("add") => run_exception_add(&args[1..]),
        Some("retune") => run_exception_retune(&args[1..]),
        Some("remove") => run_exception_remove(&args[1..]),
        Some("--help" | "-h") | None => {
            println!("{EXCEPTION_USAGE}");
            ExitCode::SUCCESS
        }
        Some(other) => usage_fail(
            "exception",
            &format!("unknown subcommand `{other}`"),
            EXCEPTION_USAGE,
        ),
    }
}

fn run_exception_add(args: &[String]) -> ExitCode {
    let mut builder = AddBuilder::default();
    let mut iter = args.iter();

    while let Some(arg) = iter.next() {
        let result = match arg.as_str() {
            "--help" | "-h" => {
                println!("{EXCEPTION_ADD_USAGE}");
                return ExitCode::SUCCESS;
            }
            "--dry-run" => {
                builder.dry_run = true;
                Ok(())
            }
            "--force" => {
                builder.force = true;
                Ok(())
            }
            "--shadows-hard" => {
                builder.shadows_hard = true;
                Ok(())
            }
            "--rule" => value(&mut iter, "--rule").map(|v| builder.rules.push(v)),
            "--severity" => value(&mut iter, "--severity").and_then(|v| builder.set_severity(&v)),
            "--kind" => value(&mut iter, "--kind").and_then(|v| builder.set_kind(&v)),
            "--reason" => value(&mut iter, "--reason").map(|v| builder.reason = Some(v)),
            "--until" => value(&mut iter, "--until").map(|v| builder.until = Some(v)),
            "--config" => value(&mut iter, "--config").map(|v| builder.config = Some(v)),
            "--match" => value(&mut iter, "--match").and_then(|v| builder.set_match(&v)),
            "--title" => value(&mut iter, "--title").map(|v| builder.title = Some(v)),
            "--owner" => value(&mut iter, "--owner").map(|v| builder.owner = Some(v)),
            "--issue" => value(&mut iter, "--issue").map(|v| builder.issue = Some(v)),
            "--max" => value(&mut iter, "--max")
                .and_then(|v| parse_max(&v))
                .map(|v| builder.max = Some(v)),
            "--unit" => value(&mut iter, "--unit").and_then(|v| builder.set_unit(&v)),
            other if other.starts_with('-') => Err(format!("unknown option `{other}`")),
            other => builder.set_path(other),
        };
        if let Err(message) = result {
            return usage_fail("exception add", &message, EXCEPTION_ADD_USAGE);
        }
    }

    // A person adding an exception is at a terminal; an agent, a hook, and CI
    // are not (§DF-008-hard-severity-needs-a-terminal.1).
    let options = match builder.build(std::io::stdin().is_terminal()) {
        Ok(options) => options,
        Err(message) => return usage_fail("exception add", &message, EXCEPTION_ADD_USAGE),
    };
    match exception::run(&options) {
        Ok(run) => {
            for warning in &run.warnings {
                eprintln!("fissile exception add: warning: {warning}");
            }
            println!("{}", run.output);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("fissile exception add: {error}");
            ExitCode::from(2)
        }
    }
}

fn parse_severity(raw: &str) -> Result<Severity, String> {
    match raw {
        "soft" => Ok(Severity::Soft),
        "hard" => Ok(Severity::Hard),
        other => Err(format!("unknown severity `{other}`")),
    }
}

fn parse_match(raw: &str) -> Result<MatchKind, String> {
    match raw {
        "exact" => Ok(MatchKind::Exact),
        "glob" => Ok(MatchKind::Glob),
        other => Err(format!("unknown match `{other}`")),
    }
}

fn parse_unit(raw: &str) -> Result<Unit, String> {
    match raw {
        "bytes" => Ok(Unit::Bytes),
        "lines" => Ok(Unit::Lines),
        "tokens" => Ok(Unit::Tokens),
        other => Err(format!("unknown unit `{other}`")),
    }
}

fn parse_max(raw: &str) -> Result<u64, String> {
    raw.parse()
        .map_err(|_| format!("--max `{raw}` is not an integer"))
}

/// The flags that address one existing entry — `<path>`, `--severity`,
/// `--rule`, `--match`, `--config`, `--dry-run` — parsed once so `retune` and
/// `remove` cannot drift on what names an entry (§DF-005-exception-identity).
/// `--max`/`--unit` belong to the one command that states a ceiling.
struct AddressArgs {
    path: Option<String>,
    severity: Option<Severity>,
    rules: Vec<String>,
    match_kind: MatchKind,
    config_path: Option<PathBuf>,
    dry_run: bool,
    max: Option<u64>,
    unit: Option<Unit>,
}

impl Default for AddressArgs {
    fn default() -> Self {
        Self {
            path: None,
            severity: None,
            rules: Vec::new(),
            // The same default `add` applies: a path with no metacharacter is
            // one file (§FS-005-exception-add.1).
            match_kind: MatchKind::Exact,
            config_path: None,
            dry_run: false,
            max: None,
            unit: None,
        }
    }
}

/// `Ok(None)` means `--help` was handled and the command is done. `sizing` says
/// whether this subcommand takes `--max`/`--unit`; where it does not, they are
/// unknown options rather than silently accepted ones.
fn parse_address(
    command: &str,
    usage: &str,
    args: &[String],
    sizing: bool,
) -> Result<Option<AddressArgs>, ExitCode> {
    let mut parsed = AddressArgs::default();
    let mut iter = args.iter();

    while let Some(arg) = iter.next() {
        let result =
            match arg.as_str() {
                "--help" | "-h" => {
                    println!("{usage}");
                    return Ok(None);
                }
                "--dry-run" => {
                    parsed.dry_run = true;
                    Ok(())
                }
                "--rule" => value(&mut iter, "--rule").map(|v| parsed.rules.push(v)),
                "--severity" => value(&mut iter, "--severity")
                    .and_then(|v| parse_severity(&v).map(|v| parsed.severity = Some(v))),
                "--match" => value(&mut iter, "--match")
                    .and_then(|v| parse_match(&v).map(|v| parsed.match_kind = v)),
                "--unit" if sizing => value(&mut iter, "--unit")
                    .and_then(|v| parse_unit(&v).map(|v| parsed.unit = Some(v))),
                "--max" if sizing => value(&mut iter, "--max")
                    .and_then(|v| parse_max(&v).map(|v| parsed.max = Some(v))),
                "--config" => value(&mut iter, "--config")
                    .map(|v| parsed.config_path = Some(PathBuf::from(v))),
                other if other.starts_with('-') => Err(format!("unknown option `{other}`")),
                other if parsed.path.is_some() => {
                    Err(format!("only one <path> is allowed, got `{other}`"))
                }
                other => {
                    parsed.path = Some(other.to_owned());
                    Ok(())
                }
            };
        if let Err(message) = result {
            return Err(usage_fail(command, &message, usage));
        }
    }
    Ok(Some(parsed))
}

/// The two fields an address cannot default: which registry holds the entry, and
/// which entry it is.
fn require_address(
    command: &str,
    usage: &str,
    parsed: &AddressArgs,
) -> Result<(String, Severity), ExitCode> {
    let Some(path) = parsed.path.clone() else {
        return Err(usage_fail(command, "a <path> is required", usage));
    };
    let Some(severity) = parsed.severity else {
        return Err(usage_fail(
            command,
            "--severity is required: it names the registry holding the entry",
            usage,
        ));
    };
    Ok((path, severity))
}

/// `fissile exception retune` (§FS-008-exception-retune): the flags that address
/// an entry, plus the value its ceiling should move to.
fn run_exception_retune(args: &[String]) -> ExitCode {
    let parsed = match parse_address("exception retune", EXCEPTION_RETUNE_USAGE, args, true) {
        Ok(Some(parsed)) => parsed,
        Ok(None) => return ExitCode::SUCCESS,
        Err(code) => return code,
    };
    let (path, severity) =
        match require_address("exception retune", EXCEPTION_RETUNE_USAGE, &parsed) {
            Ok(address) => address,
            Err(code) => return code,
        };
    let options = RetuneOptions {
        root: PathBuf::from("."),
        config_path: parsed.config_path,
        path,
        severity,
        rules: parsed.rules,
        match_kind: parsed.match_kind,
        max: parsed.max,
        unit: parsed.unit,
        dry_run: parsed.dry_run,
    };

    match retune::run(&options) {
        Ok(run) => {
            println!("{}", run.output);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("fissile exception retune: {error}");
            ExitCode::from(2)
        }
    }
}

/// `fissile exception remove` (§FS-009-exception-remove): the same address, and
/// no ceiling — the command states no number.
fn run_exception_remove(args: &[String]) -> ExitCode {
    let parsed = match parse_address("exception remove", EXCEPTION_REMOVE_USAGE, args, false) {
        Ok(Some(parsed)) => parsed,
        Ok(None) => return ExitCode::SUCCESS,
        Err(code) => return code,
    };
    let (path, severity) =
        match require_address("exception remove", EXCEPTION_REMOVE_USAGE, &parsed) {
            Ok(address) => address,
            Err(code) => return code,
        };
    let options = RemoveOptions {
        root: PathBuf::from("."),
        config_path: parsed.config_path,
        path,
        severity,
        rules: parsed.rules,
        match_kind: parsed.match_kind,
        dry_run: parsed.dry_run,
    };

    match remove::run(&options) {
        Ok(run) => {
            println!("{}", run.output);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("fissile exception remove: {error}");
            ExitCode::from(2)
        }
    }
}

/// Accumulates `exception add` flags before they are validated into [`AddOptions`].
#[derive(Default)]
struct AddBuilder {
    path: Option<String>,
    severity: Option<Severity>,
    kind: Option<Kind>,
    rules: Vec<String>,
    reason: Option<String>,
    until: Option<String>,
    match_kind: Option<MatchKind>,
    title: Option<String>,
    owner: Option<String>,
    issue: Option<String>,
    max: Option<u64>,
    unit: Option<Unit>,
    config: Option<String>,
    shadows_hard: bool,
    force: bool,
    dry_run: bool,
}

impl AddBuilder {
    fn set_path(&mut self, path: &str) -> Result<(), String> {
        if self.path.is_some() {
            return Err("only one <path> is allowed".to_owned());
        }
        self.path = Some(path.to_owned());
        Ok(())
    }

    fn set_severity(&mut self, raw: &str) -> Result<(), String> {
        self.severity = Some(parse_severity(raw)?);
        Ok(())
    }

    fn set_kind(&mut self, raw: &str) -> Result<(), String> {
        self.kind = Some(match raw {
            "structural" => Kind::Structural,
            "deferred" => Kind::Deferred,
            other => return Err(format!("unknown kind `{other}`")),
        });
        Ok(())
    }

    fn set_match(&mut self, raw: &str) -> Result<(), String> {
        self.match_kind = Some(parse_match(raw)?);
        Ok(())
    }

    fn set_unit(&mut self, raw: &str) -> Result<(), String> {
        self.unit = Some(parse_unit(raw)?);
        Ok(())
    }

    /// `interactive` is passed in rather than probed: this turns parsed flags
    /// into options and nothing else, so what it returns does not depend on how
    /// the process was launched (§DF-008-hard-severity-needs-a-terminal.1).
    fn build(self, interactive: bool) -> Result<AddOptions, String> {
        let severity = self.severity.ok_or("--severity is required")?;
        let rationale = self.rationale(severity)?;
        Ok(AddOptions {
            root: PathBuf::from("."),
            config_path: self.config.map(PathBuf::from),
            path: self.path.ok_or("a <path> is required")?,
            severity,
            rules: self.rules,
            rationale,
            match_kind: self.match_kind.unwrap_or(MatchKind::Exact),
            title: self.title,
            owner: self.owner,
            issue: self.issue,
            max: self.max,
            unit: self.unit,
            interactive,
            force: self.force,
            dry_run: self.dry_run,
        })
    }
}

impl AddBuilder {
    /// What the entry will claim: the caller's own three flags, or the pointer
    /// that replaces all three (§FS-005-exception-add.1.1). A shadowing entry
    /// forbids them rather than ignoring them — a second copy of a rationale is
    /// exactly what the flag exists to remove (§FS-003-exceptions.2.3).
    fn rationale(&self, severity: Severity) -> Result<Rationale, String> {
        if !self.shadows_hard {
            return Ok(Rationale::Stated {
                // Required rather than defaulted: which of the two claims the
                // entry makes is the author's call, and the error is the moment
                // to say what each one means (§DF-004-exception-kind.1).
                kind: self.kind.ok_or(
                    "--kind is required: structural = an architectural constraint makes the \
                     split illegal; deferred = the boundary is simply missing and someone has \
                     to build it",
                )?,
                reason: self.reason.clone().ok_or("--reason is required")?,
                until: self.until.clone(),
            });
        }
        if severity != Severity::Soft {
            return Err(
                "--shadows-hard writes the soft twin of a hard entry, so it takes \
                 --severity soft; the hard entry is the one that carries the rationale"
                    .to_owned(),
            );
        }
        for (flag, given) in [
            ("--kind", self.kind.is_some()),
            ("--reason", self.reason.is_some()),
            ("--until", self.until.is_some()),
        ] {
            if given {
                return Err(format!(
                    "--shadows-hard takes the hard entry's kind, reason, and until, so \
                     {flag} has nothing to add here — drop {flag}, or drop --shadows-hard \
                     and state all three"
                ));
            }
        }
        Ok(Rationale::ShadowsHard)
    }
}

fn usage_fail(command: &str, message: &str, usage: &str) -> ExitCode {
    eprintln!("fissile {command}: {message}\n\n{usage}");
    ExitCode::from(2)
}
