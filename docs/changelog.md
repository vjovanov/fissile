# Changelog

Records every notable change to `fissile`. Versions follow semver; the
**latest release is inline** in this file, and **older releases live
one-per-file under `docs/changelog/`** so a reader — human or agent — only
loads the history they ask for (§GOAL-004-token-thrift). Each entry cites the
FS/AR/GOAL/DA IDs it touches, so the changelog is part of the grounded tree.

Schema-version bumps are called out explicitly: `fissile_config_version`
(§FS-001-config.1), the exception registry version (§FS-003-exceptions.1), and
the managed block versions written by `init` (§FS-002-init.4). A bump to any of
these is a breaking change for the consumer and must appear under **Changed**
with a migration note.

## 1. Conventions

- **Sections per release:** `Added`, `Changed`, `Deprecated`, `Removed`,
  `Fixed`, `Security` — the Keep-a-Changelog set; omit any with no entries. A
  large entry (a first release, most of all) may add narrative subsection
  headings when the standard six would bury the structure.
- **Entry style:** one bullet per change, present tense, leading with the
  affected ID, e.g. `§FS-004-check-audit.5: skip unmeasurable paths instead of
  aborting`.
- **Progressive discovery:** only **Unreleased** and the most recent release
  are inline. When a release ships, `scripts/prepare_changelog_release.py
  prepare <version>` promotes Unreleased, archives the previous inline release
  to `docs/changelog/<version>.md`, and links it under
  [§3 Older releases](#3-older-releases). The release workflow reads the
  published notes back with the same script (§AR-001-ci.8).

## Unreleased

### Added

- §FS-004-check-audit.1.1: a run that reports a finding closes with one `hint:`
  line naming `fissile measure`. The finding already carries the offending
  file's size; the hint is about the files a split moves code *into*, whose
  headroom decides where the seam can go.
- §FS-004-check-audit.1.2: `check --staged` that ends in a standing hard
  overflow closes with a commit-gate epilogue — the commit is blocked, a
  reviewed hard exception is the other way through, and `--no-verify` only moves
  the overflow into the branch. Only `--staged` prints it: a CI or manual run is
  not blocking anything a caller is about to bypass.
- §FS-004-check-audit.1.3: `check` reports exact-path exception entries whose
  file is not on disk, under `[exceptions].stale`. `check --staged` sees a
  partial file set, so it reports only what a partial view proves — an absent
  exact path. The glob and scan-scope inventory stays in
  `audit --stale-exceptions`.
- §FS-005-exception-add.4: `--reason` that says nothing beyond the finding's own
  facts now warns. It never refuses: the test catches only a reason that is
  entirely restatement, and rejecting a terse honest claim would teach callers
  to pad it.

### Changed

- §FS-002-init.4: the managed agent block is now **v3**, and delimited by
  `<!-- >>> fissile managed block (v3) >>> -->` / `<!-- <<< ... <<< -->` markers
  like the hook block, with the version in the marker rather than the heading.
  The block is five lines rather than thirty-five: the instructions moved to the
  surfaces that raise each question (§DF-007-instructions-at-the-error-site).
  **Migration: re-run `fissile init` to upgrade.** A v1 or v2 heading-only block
  is recognized and replaced in place, so a repository upgrades rather than
  growing a second block. Since the span is now the markers and not "everything
  up to the next H1 or H2", a heading a user writes directly beneath the block is
  outside it and survives a refresh.
- §FS-005-exception-add.4: `exception add --severity hard` is refused when
  standard input is not a terminal, naming the soft-severity route and `--force`
  (§DF-008-hard-severity-needs-a-terminal). **Migration: scripted hard adds need
  `--force`.** A hard exception is the only way past a stop-the-line gate, and
  §DF-003-severity-guidance.1 already held that it is not an agent's to grant
  itself; until now nothing enforced it.
- §FS-006-cli.2: the usage screen closes by pointing at `fissile check --staged`
  rather than at `fissile init --dry-run` for the full instructions. The
  complete answer is a run, not a document: the finding names the file, the
  limit, this repository's remediation, and the command that records an
  exception.
- §FS-002-init.5: the `next:` block closes with `see <path> for what agents are
  told; the findings carry the rest.`

## 2. [0.6.0] — 2026-08-21

### Added

- §FS-007-measure: new `fissile measure <paths>... | --staged` reports what
  fissile counts for a file — the value, the rule, the limits, any accepted
  ceiling, and the signed room left before whichever binds first. The room is
  spendable: a limit fires *at* the limit and a ceiling silences *at* the
  ceiling, so `0 to hard` means the next line fails while `0 to hard-accepted`
  is a file `check` calls `ok`. It answers for files that are passing, which
  `check` never did, and always exits `0`. The JSON shape is published as
  `schema/measure.schema.json`.
- §FS-008-exception-retune: new `fissile exception retune` moves the ceiling an
  entry already records, up or down and at either severity, leaving `reason`,
  `kind`, and `until` untouched. It rewrites one `max_accepted` line in place —
  reading the registry as TOML, so prose in a `reason` names no entry, and
  keeping the line endings the file is stored with — so the diff is the single
  decision that changed. The entry is addressed by its matcher, not by a path
  the matcher happens to cover: an address that only overlaps an entry, or
  spans two, is refused with the address to use instead. Lowering stops at the
  rule limit, where the remedy is to remove the entry rather than retune it.
- §FS-001-config.5: new `[exceptions.bump]` table — `lines = 100`,
  `bytes = 4096`, `tokens = 1000` — quantizing every ceiling `fissile` writes
  (§DF-006-quantized-ceilings). Set a unit to `1` for the previous
  exact-measurement behavior.
- §FS-003-exceptions.7: `audit --stale-exceptions` also reports **loose**
  ceilings — an exact-path entry standing more than one bump step above the file
  it accepts — with the value `exception retune` would write in its place. The
  JSON record carries `severity` and `limit` alongside them, so a consumer can
  reproduce the text line without matching a registry filename against the
  config.

### Changed

- §FS-002-init.4: the managed agent block is now **v2**, teaching `measure`,
  `retune`, and the loose-ceiling sweep. **Migration: re-run `fissile init` to
  upgrade.** `init` replaces a v1 block in place and preserves the bytes around
  it; a repository that does not re-run it keeps its v1 text and only misses the
  new guidance.
- §FS-001-config.1: `[exceptions.bump]` is additive within
  `fissile_config_version = 1`, so a config that declares it is **rejected by
  fissile 0.5.0 and earlier** with `unknown field \`bump\``, which names no
  version floor. A repository adopting the key must raise its pinned `fissile`
  version — including in any pre-commit hook or CI install step — at the same
  commit.
- §FS-005-exception-add.2: `max_accepted` is now the measurement quantized up to
  the bump step rather than the measurement itself. Existing registries are
  unaffected — quantization governs what the commands write, never what a
  registry may hold (§FS-003-exceptions.4).
- §FS-005-exception-add.4: the refusal when an entry already exists reports the
  recorded ceiling beside the file's current measurement and names
  `fissile exception retune`. It previously read "already accepts <path>", which
  was false at the very moment `check` was reporting that file.
- §FS-006-cli.1: five commands rather than four, and `exception add` and
  `exception retune` each carry their own one-screen usage under a shared
  `exception` dispatch screen.

### Fixed

- §FS-004-check-audit.2: `audit --top` ranked files by raw physical lines while
  findings reported the rule's counted value, so one tool reported two numbers
  for one file. `--top` now reports what the effective rule counts where a rule
  measures the file, and the default line policy everywhere else — so the
  largest file in a repository still ranks when no rule reaches it yet.

## 3. Older releases

- [0.5.0](changelog/0.5.0.md) — 2026-08-12: - §FS-003-exceptions.1: the exception registry schema is now `fissile_exceptions_version = 2`.
- [0.4.0](changelog/0.4.0.md) — 2026-08-11: - §FS-003-exceptions.2.1: exception entries carry `kind = "structural" | "deferred"`, which fixes what `reason` must establish — the architectural constraint that makes the split illegal, or the boundary that is missing and what has to exist first (§DF-004-exception-kind).
- [0.3.0](changelog/0.3.0.md) — 2026-08-11: - §FS-006-cli.2: the usage screen opens with a short paragraph — what `fissile` is for, the two tiers, the `check --staged` habit, and the rule that a budget is never met by damaging the design — closing with a pointer to `fissile init --dry-run` for the full agent instructions.
- [0.2.0](changelog/0.2.0.md) — 2026-08-11: - §FS-001-config.3: rules take `soft_message` and `hard_message`, so a warning and a block can say different things (§DF-003-severity-guidance).
- [0.1.0](changelog/0.1.0.md) — 2026-08-11: The first release: the commit-time file-size gate, its adoption tooling, and the evidence chain behind them (§GND-001-fissile).
<!-- Populated by `prepare_changelog_release.py prepare` when a release ships. -->
