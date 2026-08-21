# FS-006-cli: the top-level command line is small, self-describing, and versioned

`fissile` is one binary with five commands: `init`, `check`, `measure`, `audit`,
and `exception`, the last carrying the `add` and `retune` subcommands
(§FS-002-init, §FS-004-check-audit, §FS-007-measure, §FS-005-exception-add,
§FS-008-exception-retune). Everything above the commands — dispatch, help,
version — is specified here so the top level cannot drift between surfaces.

`check` and `measure` are separate commands over the same measurement, because
they answer different questions under different contracts: `check` is a gate that
exits non-zero on a standing hard overflow, `measure` an inspection that always
exits `0` (§FS-007-measure.3).

## 1. Dispatch

`fissile <command> [options]` dispatches on the first argument. An unknown
command prints `fissile: unknown command \`<arg>\`` followed by the usage screen
on stderr and exits `2`. No arguments at all prints the usage screen on stdout
and exits `0`.

## 2. Help

`--help`/`-h` at the top level prints the usage screen; after a command it
prints that command's usage with compact examples. Every help screen fits in 24
lines (§GOAL-003-friendly-output.1) and the bound is enforced by a test.

The top-level screen carries one short paragraph above the command list saying
what `fissile` is for and how to work with it: the two tiers, the
`check --staged` habit, and the rule that a budget is never met by damaging the
design. It closes by pointing at `fissile init --dry-run` for the full agent
instructions.

The paragraph is deliberately short, so the pointer carries the rest. A
repository that installs `fissile` may not carry the managed agent block — the
block is one `init` target among several, and an agent may be reading an
entrypoint that never received it — so the usage screen is the one surface
guaranteed to be present, and it must lead somewhere complete. A test pins its
load-bearing clauses.

## 3. Version

`fissile --version` (alias `-V`) prints exactly one line to stdout and exits `0`:

```text
fissile 0.1.0
```

The value is the crate version baked in at compile time, so a bug report, an
upgrade check, and a release self-check all read the same number the registry
shows. The line is stable output in the §GOAL-004-token-thrift sense: no
banner, no build metadata, one token-cheap line. The release pipeline asserts
it against the version being released before publishing an artifact
(§AR-001-ci.8), and an e2e case pins the exact bytes.
