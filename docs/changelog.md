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

## 2. [0.8.0] — 2026-08-26

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
  ceiling at or above the rule's hard limit is refused — whether the step landed
  there or `--max` did — unless the hard registry holds a *deferred* entry at the
  same address. Only a deferred twin keeps the soft finding alive above the limit
  (§FS-003-exceptions.3); a structural one ends evaluation there, which leaves
  the soft ceiling as dead as one with no twin at all. A glob is held to the same
  rule: it measures nothing, so no class of files can claim the exemption a
  single file already past the limit has. That file does keep its soft route:
  that entry is the record of debt §DF-008-hard-severity-needs-a-terminal offers
  a script in place of the hard entry it may not write. The hard finding fires at
  that limit and suppresses the soft one, so such an entry never fires; under a
  350/500-line rule with the default step the measured form wrote exactly that
  for every file over 400 lines. The refusal prints the form that succeeds —
  this call with `--max <N> --unit <unit>` and the valid range, or the
  hard-severity `exception add` carrying that same ceiling
  (§DF-010-stated-ceilings-are-exact.2). The range clears every rule the entry
  lists, and the severity gate never answers a refused ceiling by repeating it,
  so no two refusals send a caller in a circle
  (§DF-007-instructions-at-the-error-site).
- §FS-004-check-audit.2: a loose soft entry whose step lands on the hard limit
  is reported with the stated form to retune it, rather than a value the command
  would refuse. Its JSON `retune_to` is `null` and the new `stated_range`
  (`{"min": N, "max_excluded": M}`) carries the range the text line prints;
  exactly one of the two is ever set.
  **Migration:** a consumer of `audit --format json` that reads `retune_to` as
  always-an-integer should read `stated_range` when it is `null`.

## 3. Older releases

- [0.7.1](changelog/0.7.1.md) — 2026-08-24: - §FS-002-init.3: `AGENTS.md` is the one entrypoint that holds the managed block, and every other one `init` touches is a **symbolic link** to it (§DF-009-one-file-agents-read).
- [0.7.0](changelog/0.7.0.md) — 2026-08-21: - §FS-004-check-audit.1.1: a run that reports a finding adds one `hint:` line naming `fissile measure`, beneath the findings it is about.
- [0.6.0](changelog/0.6.0.md) — 2026-08-21: - §FS-007-measure: new `fissile measure <paths>...
- [0.5.0](changelog/0.5.0.md) — 2026-08-12: - §FS-003-exceptions.1: the exception registry schema is now `fissile_exceptions_version = 2`.
- [0.4.0](changelog/0.4.0.md) — 2026-08-11: - §FS-003-exceptions.2.1: exception entries carry `kind = "structural" | "deferred"`, which fixes what `reason` must establish — the architectural constraint that makes the split illegal, or the boundary that is missing and what has to exist first (§DF-004-exception-kind).
- [0.3.0](changelog/0.3.0.md) — 2026-08-11: - §FS-006-cli.2: the usage screen opens with a short paragraph — what `fissile` is for, the two tiers, the `check --staged` habit, and the rule that a budget is never met by damaging the design — closing with a pointer to `fissile init --dry-run` for the full agent instructions.
- [0.2.0](changelog/0.2.0.md) — 2026-08-11: - §FS-001-config.3: rules take `soft_message` and `hard_message`, so a warning and a block can say different things (§DF-003-severity-guidance).
- [0.1.0](changelog/0.1.0.md) — 2026-08-11: The first release: the commit-time file-size gate, its adoption tooling, and the evidence chain behind them (§GND-001-fissile).
<!-- Populated by `prepare_changelog_release.py prepare` when a release ships. -->
