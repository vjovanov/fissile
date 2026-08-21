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
use fissile::exception::{self, AddOptions};
use fissile::exceptions::{Kind, MatchKind};
use fissile::init::{self, AgentTargets, HookMode, InitOptions};
use fissile::measure::{self, MeasureOptions};
use fissile::retune::{self, RetuneOptions};
use fissile::{Severity, Unit};

const USAGE: &str = "\
usage: fissile <command> [options]

fissile keeps files small so readers — human and agent — spend less to
understand them, without letting a budget damage the design. Every rule has two
limits: soft warns, hard fails the commit. Run `fissile check --staged` before
calling work done, and split what it names along a seam the code already has.
Its findings carry the rest: what to split, how, and how to record a file that
cannot be split without making the design worse.

commands:
  init [<path>]        install config, registries, and agent instructions
  check [<paths>...]   enforce file budgets on a file set or the scan scope
  measure <paths>...   report what fissile counts, and the headroom left
  audit                inventory the whole repo against its budgets
  exception add        record a justified oversized-file exception
  exception retune     move the ceiling an exception already records

run `fissile <command> --help` for command options
`--version`/`-V` prints the version";

const INIT_USAGE: &str = "\
usage: fissile init [<path>] [--name <name>] [--config <path>] [--exceptions]
                    [--hook] [--no-hook] [--force] [--dry-run] [--agents-md]
                    [--claude] [--gemini] [--copilot] [--cursor] [--windsurf] [--zed]

examples:
  fissile init --exceptions
  fissile init . --agents-md --claude";

const CHECK_USAGE: &str = "\
usage: fissile check [<paths>...] [--staged] [--config <path>]
                     [--format text|json] [--no-color]

examples:
  fissile check --staged
  fissile check src/lib.rs --format json";

const MEASURE_USAGE: &str = "\
usage: fissile measure <paths>... [--staged] [--config <path>]
                       [--format text|json] [--no-color]

Reports each file's measured size, the limits that apply, any accepted ceiling,
and the distance to whichever of those binds first. Unlike `check` it answers
for files that are passing, and it never fails a build.

examples:
  fissile measure src/lib.rs
  fissile measure --staged --format json";

const AUDIT_USAGE: &str = "\
usage: fissile audit [--config <path>] [--format text|json] [--top <N>]
                     [--stale-exceptions] [--rule-coverage] [--no-color]

examples:
  fissile audit --top 10
  fissile audit --stale-exceptions --rule-coverage";

const EXCEPTION_USAGE: &str = "\
usage: fissile exception <add|retune> <path> [options]

  add     record a justified oversized-file exception
  retune  move the ceiling an entry already records, up or down

--kind says what an added entry's --reason has to establish. Describing the
file does not:
  structural  splitting is illegal — name the constraint. Never expires.
  deferred    a boundary is missing — name it and what must exist first, and
              give --until the condition that retires the entry.

examples:
  fissile exception add src/big.rs --severity hard --rule source --kind deferred --reason \"...\" --until \"the parser module lands\"
  fissile exception retune src/big.rs --severity hard --rule source

run `fissile exception <add|retune> --help` for the full options";

const EXCEPTION_ADD_USAGE: &str = "\
usage: fissile exception add <path> --severity soft|hard --rule <id>
                 --kind structural|deferred --reason <text> [--until <text>]
                 [--config <path>] [--match exact|glob] [--title <text>]
                 [--owner <text>] [--issue <text>] [--force] [--dry-run]
                 [--max <N> --unit bytes|lines|tokens]

--kind says what --reason has to establish. Describing the file does not:
  structural  splitting is illegal — name the constraint. Never expires.
  deferred    a boundary is missing — name it and what must exist first, and
              give --until the condition that retires the entry.

examples:
  fissile exception add src/big.rs --severity hard --rule source --kind deferred --reason \"...\" --until \"the parser module lands\"
  fissile exception add \"tests/fixtures/**\" --match glob --severity soft --rule fixtures --max 300000 --unit bytes --kind structural --reason \"...\"";

const EXCEPTION_RETUNE_USAGE: &str = "\
usage: fissile exception retune <path> --severity soft|hard --rule <id>
                 [--max <N> --unit bytes|lines|tokens]
                 [--config <path>] [--match exact|glob] [--dry-run]

Moves an existing entry's ceiling, up or down, leaving its reason, kind, and
until untouched. The new value is the current measurement — or --max — rounded
up to the configured [exceptions.bump] step, so a ceiling reads as a decision
rather than as whatever the file happened to measure today.

examples:
  fissile exception retune src/big.rs --severity soft --rule source
  fissile exception retune src/big.rs --severity hard --rule source --max 900 --unit lines";

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
            // on stderr separable (§FS-002-init.4, §FS-006-cli.2).
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
        Ok(run) => finish_run("check", &run.output, run.failed, &run.errors),
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

    let options = match builder.build() {
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

/// `fissile exception retune` (§FS-008-exception-retune): the flags that address
/// an entry, plus the value its ceiling should move to.
fn run_exception_retune(args: &[String]) -> ExitCode {
    let mut options = RetuneOptions {
        root: PathBuf::from("."),
        config_path: None,
        path: String::new(),
        severity: Severity::Soft,
        rules: Vec::new(),
        match_kind: MatchKind::Exact,
        max: None,
        unit: None,
        dry_run: false,
    };
    let mut severity = None;
    let mut path = None;
    let mut iter = args.iter();

    while let Some(arg) = iter.next() {
        let result = match arg.as_str() {
            "--help" | "-h" => {
                println!("{EXCEPTION_RETUNE_USAGE}");
                return ExitCode::SUCCESS;
            }
            "--dry-run" => {
                options.dry_run = true;
                Ok(())
            }
            "--rule" => value(&mut iter, "--rule").map(|v| options.rules.push(v)),
            "--severity" => value(&mut iter, "--severity")
                .and_then(|v| parse_severity(&v).map(|v| severity = Some(v))),
            "--match" => value(&mut iter, "--match")
                .and_then(|v| parse_match(&v).map(|v| options.match_kind = v)),
            "--unit" => value(&mut iter, "--unit")
                .and_then(|v| parse_unit(&v).map(|v| options.unit = Some(v))),
            "--max" => {
                value(&mut iter, "--max").and_then(|v| parse_max(&v).map(|v| options.max = Some(v)))
            }
            "--config" => {
                value(&mut iter, "--config").map(|v| options.config_path = Some(PathBuf::from(v)))
            }
            other if other.starts_with('-') => Err(format!("unknown option `{other}`")),
            other if path.is_some() => Err(format!("only one <path> is allowed, got `{other}`")),
            other => {
                path = Some(other.to_owned());
                Ok(())
            }
        };
        if let Err(message) = result {
            return usage_fail("exception retune", &message, EXCEPTION_RETUNE_USAGE);
        }
    }

    let Some(path) = path else {
        return usage_fail(
            "exception retune",
            "a <path> is required",
            EXCEPTION_RETUNE_USAGE,
        );
    };
    let Some(severity) = severity else {
        return usage_fail(
            "exception retune",
            "--severity is required: it names the registry holding the entry",
            EXCEPTION_RETUNE_USAGE,
        );
    };
    options.path = path;
    options.severity = severity;

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

    fn build(self) -> Result<AddOptions, String> {
        Ok(AddOptions {
            root: PathBuf::from("."),
            config_path: self.config.map(PathBuf::from),
            path: self.path.ok_or("a <path> is required")?,
            severity: self.severity.ok_or("--severity is required")?,
            // Required rather than defaulted: which of the two claims the entry
            // makes is the author's call, and the error is the moment to say
            // what each one means (§DF-004-exception-kind.1).
            kind: self.kind.ok_or(
                "--kind is required: structural = an architectural constraint makes the split \
                 illegal; deferred = the boundary is simply missing and someone has to build it",
            )?,
            rules: self.rules,
            reason: self.reason.ok_or("--reason is required")?,
            until: self.until,
            match_kind: self.match_kind.unwrap_or(MatchKind::Exact),
            title: self.title,
            owner: self.owner,
            issue: self.issue,
            max: self.max,
            unit: self.unit,
            // A person adding an exception is at a terminal; an agent, a hook,
            // and CI are not (§DF-008-hard-severity-needs-a-terminal.1).
            interactive: std::io::stdin().is_terminal(),
            force: self.force,
            dry_run: self.dry_run,
        })
    }
}

fn usage_fail(command: &str, message: &str, usage: &str) -> ExitCode {
    eprintln!("fissile {command}: {message}\n\n{usage}");
    ExitCode::from(2)
}
