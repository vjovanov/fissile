//! `fissile` CLI entry point. Argument parsing is hand-rolled to keep the
//! dependency tree auditable (§GOAL-002-tiny-footprint); today it dispatches
//! `fissile init` (§FS-002-init). `check`/`audit`/`exception` are not yet wired.

use std::path::PathBuf;
use std::process::ExitCode;

use fissile::init::{self, AgentTargets, InitOptions};

const USAGE: &str = "\
usage: fissile <command> [options]

commands:
  init [<path>]   install config, exception registries, and agent instructions

run `fissile init --help` for init options";

const INIT_USAGE: &str = "\
usage: fissile init [<path>] [--config <path>] [--exceptions] [--force]
                    [--dry-run] [--agents-md] [--claude] [--gemini]
                    [--copilot] [--cursor] [--windsurf] [--zed]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("init") => run_init(&args[1..]),
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
            "--config" => match iter.next() {
                Some(value) => options.config_path = PathBuf::from(value),
                None => return fail("--config requires a path"),
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
            value if value.starts_with('-') => {
                return fail(&format!("unknown option `{value}`"));
            }
            value => options.root = PathBuf::from(value),
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

fn fail(message: &str) -> ExitCode {
    eprintln!("fissile init: {message}\n\n{INIT_USAGE}");
    ExitCode::from(2)
}
