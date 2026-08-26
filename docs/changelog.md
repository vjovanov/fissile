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

### Changed

- §FS-005-exception-add.2, §FS-008-exception-retune.1: a ceiling stated with
  `--max` is written as stated; the `[exceptions.bump]` step rounds only what
  the command measured (§DF-010-stated-ceilings-are-exact). `--max 480` writes
  480, not 500, and a glob entry — which always takes `--max` — records the
  policy number chosen rather than the step's next multiple (`--max 300000
  --unit bytes` wrote 303104). `retune` names that next multiple in its result
  as the round number the caller could have chosen.
  **Migration: none.** Registries are untouched. `retune --max` now moves a
  ceiling by less than one step, which is how a rounded ceiling comes back down.

### Added

- §FS-005-exception-add.4, §FS-008-exception-retune.4: a `--severity soft`
  ceiling at or above the rule's hard limit, for a file still under that limit,
  is refused — whether the step landed there or `--max` did — unless the hard
  registry holds an entry at the same address. A file already past the limit
  keeps its soft route: that entry is the record of debt
  §DF-008-hard-severity-needs-a-terminal offers a script in place of the hard
  entry it may not write. The hard finding fires at that limit and suppresses the soft one, so
  such an entry never fires; under a 350/500-line rule with the default step the
  measured form wrote exactly that for every file over 400 lines. The refusal
  prints the form that succeeds — this call with `--max <N> --unit <unit>` and
  the valid range, or the hard-severity `exception add`
  (§DF-010-stated-ceilings-are-exact.2).
- §FS-004-check-audit.2: a loose soft entry whose step lands on the hard limit
  is reported with the stated form to retune it, and its JSON `retune_to` is
  `null`, rather than a value the command would refuse.

## 2. [0.7.1] — 2026-08-24

### Changed

- §FS-002-init.3: `AGENTS.md` is the one entrypoint that holds the managed
  block, and every other one `init` touches is a **symbolic link** to it
  (§DF-009-one-file-agents-read). A repository with `AGENTS.md`, `CLAUDE.md`,
  and a `.claude/` directory carried three copies of the same five lines, and
  Claude Code reads two of them — the tool whose purpose is spending fewer
  tokens on what agents read was itself sending the block twice. Links are
  relative to the file that carries them (`../AGENTS.md` from `.claude/`), so a
  clone or a move keeps them resolving.
  **Migration: none required, and nothing is overwritten.** A companion whose
  bytes match `AGENTS.md` becomes a link to it; a project with a `CLAUDE.md` and
  no `AGENTS.md` has that file's content *become* `AGENTS.md`, since it is what
  the project already told agents. A companion holding bytes of its own is kept
  as a regular file with the block written in, and the run reports `kept` rather
  than `linked`. Where the filesystem refuses a link — Windows without Developer
  Mode — `init` writes the block into the file and says so instead of failing.

## 3. Older releases

- [0.7.0](changelog/0.7.0.md) — 2026-08-21: - §FS-004-check-audit.1.1: a run that reports a finding adds one `hint:` line naming `fissile measure`, beneath the findings it is about.
- [0.6.0](changelog/0.6.0.md) — 2026-08-21: - §FS-007-measure: new `fissile measure <paths>...
- [0.5.0](changelog/0.5.0.md) — 2026-08-12: - §FS-003-exceptions.1: the exception registry schema is now `fissile_exceptions_version = 2`.
- [0.4.0](changelog/0.4.0.md) — 2026-08-11: - §FS-003-exceptions.2.1: exception entries carry `kind = "structural" | "deferred"`, which fixes what `reason` must establish — the architectural constraint that makes the split illegal, or the boundary that is missing and what has to exist first (§DF-004-exception-kind).
- [0.3.0](changelog/0.3.0.md) — 2026-08-11: - §FS-006-cli.2: the usage screen opens with a short paragraph — what `fissile` is for, the two tiers, the `check --staged` habit, and the rule that a budget is never met by damaging the design — closing with a pointer to `fissile init --dry-run` for the full agent instructions.
- [0.2.0](changelog/0.2.0.md) — 2026-08-11: - §FS-001-config.3: rules take `soft_message` and `hard_message`, so a warning and a block can say different things (§DF-003-severity-guidance).
- [0.1.0](changelog/0.1.0.md) — 2026-08-11: The first release: the commit-time file-size gate, its adoption tooling, and the evidence chain behind them (§GND-001-fissile).
<!-- Populated by `prepare_changelog_release.py prepare` when a release ships. -->
