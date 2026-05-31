# FS-004-check-audit: fissile check and audit enforce file budgets

`fissile check` and `fissile audit` are the user-visible enforcement surfaces
for the library core. `check` is the commit-time gate; `audit` is the whole-repo
inventory and migration tool. Both use the same effective config, rule
resolution, exclusions, messages, and exception registries.

## 1. Check

```text
fissile check [--staged] [--config <path>] [--format text|json] [--no-color]
```

`check --staged` receives the file set from git and applies `[scan].exclude`.
Without `--staged`, `check` evaluates the paths passed by the caller or the
configured scan scope. A soft overflow exits `0` unless a matching soft
exception applies; a hard overflow exits non-zero unless a matching hard
exception applies. Severity is not configurable. This is the stable
CI/pre-commit contract: the same config must produce the same pass/fail result
locally and remotely (§GOAL-003-friendly-output).

Text output for an overflow has a compact finding line plus, when configured, a
single rendered guidance line:

```text
src/domain/order.rs: 612 lines > 550 lines [hard, rule: rust-source, message: split-rust-module]
  Split src/domain/order.rs: move cohesive helpers into a sibling module before adding more code.
```

JSON output emits one record per overflow with at least:

- `path`
- `unit`
- `actual`
- `limit`
- `severity`
- `rule_id`
- `message_id`
- `message`
- `exception_id`, when applicable in verbose output
- `exception_max`, when an exception is reported in verbose output

When no findings are emitted, text output prints exactly `ok`; JSON output emits
no success envelope.

## 2. Audit

```text
fissile audit [--config <path>] [--format text|json] [--top <N>]
              [--stale-exceptions] [--rule-coverage]
```

`audit` walks the configured scan scope and reports the current repository
state. It is for adoption and maintenance, not just pass/fail.

- Default audit reports current soft and hard overflows.
- `--top <N>` reports the largest measured files per unit, after exclusions,
  even when they are under limit.
- `--stale-exceptions` reports exception entries whose path or glob matches no
  scanned file.
- `--rule-coverage` reports which rules matched zero files, which files matched
  only built-in catch-all rules, and which rule/message pairs are unused.

`audit` exits non-zero for hard overflows and schema errors. Soft-only findings
exit `0`. Stale exceptions follow `[exceptions].stale`: `warn`, `error`, or
`ignore`.

## 3. Default Large-File Guard

The built-in config includes a simple byte-size guard over all non-excluded
files. It is intentionally boring: it catches accidental blobs and platform-host
problems before they reach review. Projects should tune or replace it with
named, project-specific rules once they know their layout.

This guard does not replace line or token budgets. A file may be checked by one
effective byte rule and one effective line rule at the same time (§FS-001-config.3.2).

## 4. Named Budget Entries

Findings always name the matched rule. The intended config style is a list of
named budget entries, similar to bundle-size tools but applied to source layout:

```toml
[[rules]]
id = "api-docs"
include = ["docs/api/**/*.md"]
unit = "lines"
soft = 500
hard = 900
message = "split-api-doc"
```

Names must be stable because exception entries, JSON consumers, and agent
guidance all key off them.
