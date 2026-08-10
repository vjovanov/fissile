# FS-006-cli: the top-level command line is small, self-describing, and versioned

`fissile` is one binary with four commands: `init`, `check`, `audit`, and
`exception add` (§FS-002-init, §FS-004-check-audit, §FS-005-exception-add).
Everything above the commands — dispatch, help, version — is specified here so
the top level cannot drift between surfaces.

## 1. Dispatch

`fissile <command> [options]` dispatches on the first argument. An unknown
command prints `fissile: unknown command \`<arg>\`` followed by the usage screen
on stderr and exits `2`. No arguments at all prints the usage screen on stdout
and exits `0`.

## 2. Help

`--help`/`-h` at the top level prints the usage screen; after a command it
prints that command's usage with compact examples. Every help screen fits in 24
lines (§GOAL-003-friendly-output.1) and the bound is enforced by a test.

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
