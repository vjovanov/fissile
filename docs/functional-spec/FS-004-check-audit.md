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

### 1.1 The hint line

A `check` run that reports at least one finding closes with one `hint:` line
naming `fissile measure`:

```text
hint: fissile measure <path>... reports size and headroom for the files you split into.
```

The finding already carries the offending file's measurement and the limit it
crossed, so the hint is not about that file. It is about the ones the split
moves code *into*, whose headroom decides where the seam can go and which no
other tool computes (§FS-007-measure). One line, once per run, never per file,
and only when something was reported — a clean run stays exactly `ok`.

This line and §1.2 are the two things `check` prints that are not findings. Both
exist because the instructions they carry left the managed agent block
(§DF-007-instructions-at-the-error-site.2), and both are bounded to a single
line so that §GOAL-004-token-thrift still holds for a run that reports many
files.

### 1.2 The commit-gate epilogue

`check --staged` that ends in a standing hard overflow closes with:

```text
commit blocked by fissile. Split the file, or ask a human for a reviewed hard
exception. Bypassing with --no-verify leaves the overflow for review or CI.
```

Only `--staged` prints it, because only `--staged` is a commit: the same
findings from a CI run or a manual `fissile check src/` are not blocking
anything a caller is about to bypass. It says the one thing the finding's own
guidance cannot, since a project rewrites that guidance (§DF-003-severity-guidance.1)
and it is the wrong voice regardless — `--no-verify` is reached for by a caller
who has just decided the gate is in the way.

### 1.3 Dangling exceptions

`check` reports every `match = "exact"` exception entry whose path does not
exist on disk, in a block of its own:

```text
stale: 1 exception accepts a file that is not there [registry: docs/file-size-agent-exceptions.toml]
  The file moved or was deleted, so the entry silences nothing. Remove it, or
  point it at the path the file moved to.
    src/domain/order.rs [soft, rule: rust-source]
```

Entries are reported under `[exceptions].stale`: `warn` reports them, `error`
also fails the run, `ignore` says nothing (§FS-003-exceptions.4).

`check` reports only what its file set proves. Under `--staged` it sees the
staged paths alone, so an entry matching nothing in that set is not thereby
stale — but an exact path that is not on the filesystem is gone whatever the
file set was. Globs are not judged here at all: a glob matching no file today is
a question about the scan scope, which is `audit`'s inventory to answer
(§FS-004-check-audit.2), not a fact a commit hook can establish.

JSON output emits one record per overflow with at least:

- `path`
- `unit`
- `actual`
- `limit`
- `severity`
- `rule_id`
- `message_id`
- `message`
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
- Default audit also counts the exception registries by kind
  (§FS-003-exceptions.2.1) — how many files are accepted permanently versus how
  many carry debt someone has to retire:

  ```text
  exceptions:
    structural (never expires): 3
    deferred (carrying debt): 32
  ```

  The section is omitted from text output when both registries are empty, so a
  repository with no exceptions pays nothing for it. JSON always carries the
  object, because a consumer should not have to distinguish "no exceptions" from
  "this build does not report them".
- `--top <N>` reports the largest measured files per unit, after exclusions, even
  when they are under limit. Where a rule measures the file in that unit, the
  value is the one that rule counts — its line policy decides what a line is
  (§FS-001-config.3.1) — so a `--top` number and a finding never disagree about
  one file. Every other scanned file still ranks, under the default line policy:
  `--top` is the adoption surface, and a repository whose rules do not yet reach
  its largest file is the repository that most needs to be told about it.
- `--stale-exceptions` reports every exception entry whose path or glob matches
  no scanned file, each named by its registry and its `path` (§FS-003-exceptions.4)
  — the list spans both registries, and the same path can be stale in each. It
  reports **loose** entries in the same pass: an exact-path entry whose ceiling
  stands more than one bump step above the file it accepts
  (§FS-003-exceptions.7), with the ceiling `fissile exception retune` would write
  in its place. Stale means the entry accepts a file that is gone; loose means it
  accepts far more of a file that is still there.

  ```text
  loose ceilings:
    docs/file-size-agent-exceptions.toml: src/domain/order.rs accepts 650 lines, now 421 — retune to 500
  ```

  An entry whose file no longer crosses the limit at all silences nothing, so
  the line says that instead of naming a lower ceiling: removing the entry, not
  retuning it, is the remedy.
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
