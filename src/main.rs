//! `fissile` CLI entry point. Hand-rolled argument parsing keeps the dependency
//! tree auditable (§GOAL-002-tiny-footprint); it dispatches `init`, `check`,
//! `audit`, and `exception add` (§FS-002-init, §FS-004-check-audit, §FS-005-exception-add).

use std::path::PathBuf;
use std::process::ExitCode;
use std::slice::Iter;

use fissile::audit::{self, AuditOptions};
use fissile::check::{self, CheckOptions};
use fissile::cli::Format;
use fissile::exception::{self, AddOptions};
use fissile::exceptions::MatchKind;
use fissile::init::{self, AgentTargets, InitOptions};
use fissile::{Severity, Unit};

const USAGE: &str = "\
usage: fissile <command> [options]

commands:
  init [<path>]        install config, registries, and agent instructions
  check [<paths>...]   enforce file budgets on a file set or the scan scope
  audit                inventory the whole repo against its budgets
  exception add <path> record a justified oversized-file exception

run `fissile <command> --help` for command options";

const INIT_USAGE: &str = "\
usage: fissile init [<path>] [--name <name>] [--config <path>] [--exceptions]
                    [--force] [--dry-run] [--agents-md] [--claude] [--gemini]
                    [--copilot] [--cursor] [--windsurf] [--zed]";

const CHECK_USAGE: &str = "\
usage: fissile check [<paths>...] [--staged] [--config <path>]
                     [--format text|json] [--no-color]";

const AUDIT_USAGE: &str = "\
usage: fissile audit [--config <path>] [--format text|json] [--top <N>]
                     [--stale-exceptions] [--rule-coverage] [--no-color]";

const EXCEPTION_USAGE: &str = "\
usage: fissile exception add <path> --severity soft|hard --rule <id>
                 --reason <text> --until <text> [--config <path>]
                 [--match exact|glob] [--id <id>] [--title <text>]
                 [--owner <text>] [--issue <text>] [--replaces <id>]
                 [--max <N> --unit bytes|lines|tokens] [--dry-run]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("init") => run_init(&args[1..]),
        Some("check") => run_check(&args[1..]),
        Some("audit") => run_audit(&args[1..]),
        Some("exception") => run_exception(&args[1..]),
        Some("--help" | "-h") | None => {
            println!("{USAGE}");
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
    match init::run(&options) {
        Ok(report) => {
            eprintln!("{}", report.render());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("fissile init: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run_check(args: &[String]) -> ExitCode {
    let mut options = CheckOptions {
        root: PathBuf::from("."),
        config_path: None,
        staged: false,
        format: None,
        no_color: false,
        paths: Vec::new(),
    };
    let mut iter = args.iter();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                println!("{CHECK_USAGE}");
                return ExitCode::SUCCESS;
            }
            "--staged" => options.staged = true,
            "--no-color" => options.no_color = true,
            "--config" => match value(&mut iter, "--config") {
                Ok(path) => options.config_path = Some(PathBuf::from(path)),
                Err(message) => return usage_fail("check", &message, CHECK_USAGE),
            },
            "--format" => match value(&mut iter, "--format").and_then(|raw| parse_format(&raw)) {
                Ok(format) => options.format = Some(format),
                Err(message) => return usage_fail("check", &message, CHECK_USAGE),
            },
            other if other.starts_with('-') => {
                return usage_fail("check", &format!("unknown option `{other}`"), CHECK_USAGE);
            }
            other => options.paths.push(other.to_owned()),
        }
    }

    match check::run(&options) {
        Ok(run) => {
            println!("{}", run.output);
            if run.failed {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(error) => {
            eprintln!("fissile check: {error}");
            ExitCode::from(2)
        }
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
        Ok(run) => {
            println!("{}", run.output);
            if run.failed {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
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
                println!("{EXCEPTION_USAGE}");
                return ExitCode::SUCCESS;
            }
            "--dry-run" => {
                builder.dry_run = true;
                Ok(())
            }
            "--rule" => value(&mut iter, "--rule").map(|v| builder.rules.push(v)),
            "--severity" => value(&mut iter, "--severity").and_then(|v| builder.set_severity(&v)),
            "--reason" => value(&mut iter, "--reason").map(|v| builder.reason = Some(v)),
            "--until" => value(&mut iter, "--until").map(|v| builder.until = Some(v)),
            "--config" => value(&mut iter, "--config").map(|v| builder.config = Some(v)),
            "--match" => value(&mut iter, "--match").and_then(|v| builder.set_match(&v)),
            "--id" => value(&mut iter, "--id").map(|v| builder.id = Some(v)),
            "--title" => value(&mut iter, "--title").map(|v| builder.title = Some(v)),
            "--owner" => value(&mut iter, "--owner").map(|v| builder.owner = Some(v)),
            "--issue" => value(&mut iter, "--issue").map(|v| builder.issue = Some(v)),
            "--replaces" => value(&mut iter, "--replaces").map(|v| builder.replaces = Some(v)),
            "--max" => value(&mut iter, "--max")
                .and_then(|v| {
                    v.parse()
                        .map_err(|_| format!("--max `{v}` is not an integer"))
                })
                .map(|v| builder.max = Some(v)),
            "--unit" => value(&mut iter, "--unit").and_then(|v| builder.set_unit(&v)),
            other if other.starts_with('-') => Err(format!("unknown option `{other}`")),
            other => builder.set_path(other),
        };
        if let Err(message) = result {
            return usage_fail("exception add", &message, EXCEPTION_USAGE);
        }
    }

    let options = match builder.build() {
        Ok(options) => options,
        Err(message) => return usage_fail("exception add", &message, EXCEPTION_USAGE),
    };
    match exception::run(&options) {
        Ok(run) => {
            println!("{}", run.output);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("fissile exception add: {error}");
            ExitCode::from(2)
        }
    }
}

/// Accumulates `exception add` flags before they are validated into [`AddOptions`].
#[derive(Default)]
struct AddBuilder {
    path: Option<String>,
    severity: Option<Severity>,
    rules: Vec<String>,
    reason: Option<String>,
    until: Option<String>,
    match_kind: Option<MatchKind>,
    id: Option<String>,
    title: Option<String>,
    owner: Option<String>,
    issue: Option<String>,
    replaces: Option<String>,
    max: Option<u64>,
    unit: Option<Unit>,
    config: Option<String>,
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
        self.severity = Some(match raw {
            "soft" => Severity::Soft,
            "hard" => Severity::Hard,
            other => return Err(format!("unknown severity `{other}`")),
        });
        Ok(())
    }

    fn set_match(&mut self, raw: &str) -> Result<(), String> {
        self.match_kind = Some(match raw {
            "exact" => MatchKind::Exact,
            "glob" => MatchKind::Glob,
            other => return Err(format!("unknown match `{other}`")),
        });
        Ok(())
    }

    fn set_unit(&mut self, raw: &str) -> Result<(), String> {
        self.unit = Some(match raw {
            "bytes" => Unit::Bytes,
            "lines" => Unit::Lines,
            "tokens" => Unit::Tokens,
            other => return Err(format!("unknown unit `{other}`")),
        });
        Ok(())
    }

    fn build(self) -> Result<AddOptions, String> {
        Ok(AddOptions {
            root: PathBuf::from("."),
            config_path: self.config.map(PathBuf::from),
            path: self.path.ok_or("a <path> is required")?,
            severity: self.severity.ok_or("--severity is required")?,
            rules: self.rules,
            reason: self.reason.ok_or("--reason is required")?,
            until: self.until.ok_or("--until is required")?,
            match_kind: self.match_kind.unwrap_or(MatchKind::Exact),
            id: self.id,
            title: self.title,
            owner: self.owner,
            issue: self.issue,
            replaces: self.replaces,
            max: self.max,
            unit: self.unit,
            dry_run: self.dry_run,
        })
    }
}

fn usage_fail(command: &str, message: &str, usage: &str) -> ExitCode {
    eprintln!("fissile {command}: {message}\n\n{usage}");
    ExitCode::from(2)
}
