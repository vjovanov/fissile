//! `--help` stays a one-screen surface and subcommands carry examples
//! (§GOAL-003-friendly-output.3).

use std::process::Command;

/// The one-screen bound from §GOAL-003-friendly-output.3.
const MAX_HELP_LINES: usize = 24;

fn help(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_fissile"))
        .args(args)
        .output()
        .expect("fissile runs");
    assert!(output.status.success(), "help exits zero for {args:?}");
    String::from_utf8(output.stdout).expect("help is utf-8")
}

#[test]
fn top_level_help_fits_one_screen() {
    let lines = help(&["--help"]).lines().count();
    assert!(
        lines <= MAX_HELP_LINES,
        "fissile --help is {lines} lines, over the {MAX_HELP_LINES}-line budget"
    );
}

#[test]
fn subcommand_help_fits_one_screen_and_shows_examples() {
    for command in ["init", "check", "audit", "exception"] {
        let text = help(&[command, "--help"]);
        let lines = text.lines().count();
        assert!(
            lines <= MAX_HELP_LINES,
            "fissile {command} --help is {lines} lines, over the {MAX_HELP_LINES}-line budget"
        );
        assert!(
            text.contains("examples:"),
            "fissile {command} --help should include compact examples"
        );
    }
}

/// §FS-006-cli.2: the usage screen says what fissile is for, how to work with
/// it, and where the full instructions are. It is the one surface guaranteed to
/// exist in a repository whose agent entrypoint never received the block.
#[test]
fn top_level_help_states_the_workflow_and_points_at_the_instructions() {
    let text = help(&["--help"]);
    for clause in [
        "soft warns",
        "hard fails the commit",
        "fissile check --staged",
        "fissile init --dry-run",
    ] {
        assert!(
            text.contains(clause),
            "usage screen should state `{clause}`"
        );
    }
}
