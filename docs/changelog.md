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
with a migration note. A change that breaks a library caller's source is called
out the same way and names the release it forces: the crate publishes a `[lib]`,
and at 0.x semver puts the minor number in charge of it.

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

- §FS-003-exceptions.2.3, §FS-005-exception-add.1.1: a soft entry may declare
  `shadows = "hard"` and inherit its `reason` and `until` from the hard entry at
  the same address, and `fissile exception add --shadows-hard` writes one. A
  deferred hard entry leaves the soft finding standing (§FS-003-exceptions.3),
  so every deferred hard acceptance needs a soft twin, and that twin had to
  restate an argument and a retirement condition it does not own. The pairing is
  now checked at load: exactly one hard entry must answer the twin's address, so
  deleting the original fails the load until the twin goes too. `shadows` is
  forbidden alongside `reason` or `until` and in the hard registry, and
  `--shadows-hard` refuses `--severity hard`, the other three flags, and a
  missing hard entry. `max_accepted` stays required and local, and the pair may
  differ. No registry version moves; existing registries are unaffected.
  Resolves #16. (PR #52)

### Changed

- §FS-005-exception-add.1.1: `fissile::exception::AddOptions` replaces its public
  `kind`, `reason`, and `until` fields with one `rationale: Rationale`, so a
  shadowing call is a value the type admits rather than three fields left blank.
  A library caller's struct literal stops compiling. Wrap the three it used to
  pass in `Rationale::Stated { kind, reason, until }`, or write
  `Rationale::ShadowsHard`. A 0.x source break, so the minor number moves.
  (PR #52)

- §FS-003-exceptions.4: `fissile::exceptions::ExceptionError` gains four variants
  for the `shadows` refusals, and `EmptyReason`/`EmptyUntil` gain a `severity`
  field — the soft-registry wording offers `shadows = "hard"` as the way out and
  the hard-registry wording cannot. The enum is not `#[non_exhaustive]`, so an
  exhaustive external `match` on it stops compiling: add the new arms, or a
  wildcard. Whether the enum should become `#[non_exhaustive]` is left open
  rather than settled here. Same minor bump. (PR #52)

- §FS-003-exceptions.4: an entry that states no `reason` or no `until` now reads
  `states no reason` rather than `has an empty reason`. Both fields became
  optional in the schema so a shadowing entry can omit them, which makes an
  absent field and a blank one one defect; the old wording was true of only one
  of them. (PR #52)

- §AR-001-ci: every declaration is listed in its folder's index. `grund check`
  warned that thirteen declarations were absent from their index README and that
  the warning becomes an error in grund 0.13.0; the two decision folders that had
  no index at all now have one. (PR #51)

- §AR-001-ci.8.2: after a release, main opens the next patch as `X.Y.Z-dev`
  instead of keeping the version it just published. A build from main previously
  reported the tag it was already ahead of, so a merged-but-uninstalled fix was
  indistinguishable from an installed one. (PR #50)

### Fixed

- §FS-004-check-audit.2: `audit` now reports exception entry totals alongside
  distinct path-expression totals, so soft/hard twins and repeated globs are
  not presented as additional files. (PR #47)

## 2. [0.8.2] — 2026-08-31

### Changed

- §FS-001-config.0.1: the built-in defaults budget a Markdown file by how it is
  read. The flat `markdown-docs` rule (`**/*.md`, 250/500) splits into
  `citable-spec` (`**/*.md`, soft 750 / hard 2000) — a document opened when
  needed and, with stable section IDs, read a section at a time, so its length
  is not charged on every read the way a source file's is — and `entrypoint`
  (the §FS-002-init.3 family plus `skills/**/*.md`, soft 250 / hard 500), which
  is loaded whole into every agent session. An exact filename and a rooted glob
  outrank `**/*.md` on the §FS-001-config.3.2 specificity order, so neither
  needs a `priority`. No schema version moves; a repository that already has a
  config owns it and is unaffected. This repository's own `spec-docs` rule
  follows, `CLAUDE.md` moves to a new `entrypoints` rule, and the deferred
  exception that let `FS-001-config.md` sit at its natural size is removed —
  a 300-line ceiling under a 750-line limit is a run-level error, not merely
  redundant. Resolves #27 and #41. (PR #42)
- Testing: adopted grund's two non-citable test homes. The e2e corpus moved
  from `e2e/cases` to `tests/e2e/cases` (harness at `tests/e2e/main.rs`), the
  cross-part proofs that lived beside it in `tests/` moved into
  `tests/integration/`, and every `grund.toml` `[[kinds]]` block's deprecated
  `prefix` key is now `kind`. Resolves #39. (PR #40)

## 3. Older releases

- [0.8.1](changelog/0.8.1.md) — 2026-08-30: - §FS-002-init.5: `init::Report` carries one `HookStatus` — `Installed`, `SkippedNotGit`, `SkippedByFlag` — in place of the `hook_skipped_not_git` boolean, so the hook step 2 reports is a value every path has to answer for instead of a flag that can be left unset.
- [0.8.0](changelog/0.8.0.md) — 2026-08-26: - §FS-005-exception-add.2, §FS-008-exception-retune.1: a ceiling stated with `--max` is written as stated; the `[exceptions.bump]` step rounds only what the command measured (§DF-010-stated-ceilings-are-exact).
- [0.7.1](changelog/0.7.1.md) — 2026-08-24: - §FS-002-init.3: `AGENTS.md` is the one entrypoint that holds the managed block, and every other one `init` touches is a **symbolic link** to it (§DF-009-one-file-agents-read).
- [0.7.0](changelog/0.7.0.md) — 2026-08-21: - §FS-004-check-audit.1.1: a run that reports a finding adds one `hint:` line naming `fissile measure`, beneath the findings it is about.
- [0.6.0](changelog/0.6.0.md) — 2026-08-21: - §FS-007-measure: new `fissile measure <paths>...
- [0.5.0](changelog/0.5.0.md) — 2026-08-12: - §FS-003-exceptions.1: the exception registry schema is now `fissile_exceptions_version = 2`.
- [0.4.0](changelog/0.4.0.md) — 2026-08-11: - §FS-003-exceptions.2.1: exception entries carry `kind = "structural" | "deferred"`, which fixes what `reason` must establish — the architectural constraint that makes the split illegal, or the boundary that is missing and what has to exist first (§DF-004-exception-kind).
- [0.3.0](changelog/0.3.0.md) — 2026-08-11: - §FS-006-cli.2: the usage screen opens with a short paragraph — what `fissile` is for, the two tiers, the `check --staged` habit, and the rule that a budget is never met by damaging the design — closing with a pointer to `fissile init --dry-run` for the full agent instructions.
- [0.2.0](changelog/0.2.0.md) — 2026-08-11: - §FS-001-config.3: rules take `soft_message` and `hard_message`, so a warning and a block can say different things (§DF-003-severity-guidance).
- [0.1.0](changelog/0.1.0.md) — 2026-08-11: The first release: the commit-time file-size gate, its adoption tooling, and the evidence chain behind them (§GND-001-fissile).
<!-- Populated by `prepare_changelog_release.py prepare` when a release ships. -->
