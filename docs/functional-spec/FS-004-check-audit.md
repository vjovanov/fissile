# FS-004-check-audit: fissile check and audit enforce file budgets

`fissile check` and `fissile audit` are the user-visible enforcement surfaces
for the library core. `check` is the commit-time gate; `audit` is the whole-repo
inventory and migration tool. Both use the same effective config, rule
resolution, exclusions, messages, and exception registries.

## 1. Check

```text
fissile check [<paths>...] [--staged] [--config <path>] [--format text|json] [--no-color]
```

`check --staged` receives the file set from git and applies `[scan].exclude`.
Without `--staged`, `check` evaluates the paths passed by the caller or the
configured scan scope. A soft overflow exits `0` unless a matching soft
exception applies; a hard overflow exits non-zero unless a matching hard
exception applies. Severity is not configurable. This is the stable
CI/pre-commit contract: the same config must produce the same pass/fail result
locally and remotely (§GOAL-003-friendly-output).

Text findings are grouped, one block per `(severity, rule, rendered guidance)`.
The header names the severity, the file count, the crossed limit, the rule, and
the message ID; the guidance follows once, indented two spaces; the files follow
indented four. Guidance is never repeated per file — a repo-wide run says what
to do once and then lists what it applies to (§GOAL-003-friendly-output).

```text
hard: 2 files over the 550-line budget [rule: rust-source, message: split-rust-hard]
  Must split before more code lands here: move cohesive groups of items into
  sibling modules. If you cannot see a safe split, stop and ask a human.
    src/domain/order.rs: 612 lines
    src/domain/invoice.rs: 588 lines

soft: 1 file over the 350-line budget [rule: rust-source, message: split-rust-soft]
  Should split the next time you touch it. If no split leaves the architecture
  cleaner, record it with `fissile exception add --severity soft`.
    src/domain/tax.rs: 402 lines
```

Blocks are ordered hard before soft, then by rule ID; files within a block are
ordered by measured value descending, then by path. Blocks are separated by a
blank line. A message that interpolates a per-file variable renders distinct
text per file, which by the grouping key puts each file in its own block
(§FS-001-config.4).

Guidance is wrapped at a fixed 78 columns, and newlines written into the message
are kept, so a project that configures a paragraph gets a readable block. The
width is fixed rather than read from the terminal: the same finding must be
byte-identical in a narrow terminal and in CI (§GOAL-006-graded-limits.2).

JSON output emits one record per overflow with at least:

- `path`
- `unit`
- `actual`
- `limit`
- `severity`
- `rule_id`
- `message_id`
- `message`
- `exception_id`, when applicable in audit's silenced output
- `exception_max`, when applicable in audit's silenced output

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

## 5. Errors

Failures split by scope. A **run-level** failure — an unreadable or invalid
config, an invalid exception registry, a failed `git diff --cached`, an
ambiguous rule overlap — aborts before findings and exits `2` with a single
`fissile <command>:` diagnostic on stderr. When the failing document is a file,
the diagnostic names it (`.agents/fissile.toml: config parse error: … at line
100`), and a failed git invocation appends git's own first stderr line so
"not a git repository" is visible verbatim.

A **file-level** failure — one path that cannot be read or measured (missing,
unreadable, a directory) — does not abort the run: one odd path must not hide
every other finding. The path is skipped, every other file is still measured,
findings print normally on stdout, and each skipped path adds one stderr line
that names it:

```text
fissile check: cannot measure src/gone.rs: No such file or directory (os error 2)
```

A run with file-level failures exits `2` even when no finding stands — silently
passing an unmeasurable file would make the gate unsound — and the text success
marker is withheld. JSON output never carries error records: stdout keeps the
stable findings shape (§GOAL-003-friendly-output.1) and stderr owns diagnostics.

Non-UTF-8 content is not an error: line budgets measure physical lines from raw
bytes (§FS-001-config.3.1).
