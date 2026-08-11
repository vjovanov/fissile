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

## 2. [0.2.0] — 2026-08-11

### Added

- §FS-001-config.3: rules take `soft_message` and `hard_message`, so a warning
  and a block can say different things (§DF-003-severity-guidance). `message`
  still sets both, and every declared threshold must resolve one — a `soft`
  limit with no guidance for it is now a schema error rather than a rule that
  reports with nothing to say.

### Changed

- §FS-004-check-audit.1: text findings are grouped, one block per
  `(severity, rule, rendered guidance)` — guidance printed once, then the files
  it applies to, largest first, hard blocks before soft. A repo-wide run over a
  dozen files under one rule used to repeat the same remediation line a dozen
  times (§GOAL-004-token-thrift). Guidance wraps at a fixed 78 columns and keeps
  newlines written into the message, so a configured paragraph reads as one. The
  agent-facing shape in §GOAL-006-graded-limits.2 now spans the block header and
  the per-file line; `--format json` is unchanged, still one flat record per
  finding.
- §FS-001-config.4: the shipped default messages no longer cite
  §GOAL-008-remediation-messages or any other fissile ID. `init` copies these
  into another repository, where an ID from fissile's own docs resolves nowhere
  and describes fissile's architecture rather than the reader's
  (§DF-003-severity-guidance.1). The defaults now split per severity, name the
  escape hatch each severity has — a recorded soft exception, or asking a human
  before a hard one — and say what *not* to do to fit a budget. The generated
  config marks where a project adds a citation into its own docs.
- §FS-002-init.4: the managed agent block (still v1) teaches the grouped output
  and the two severities as *should split* / *must split*, including that
  `--severity hard` is a human's call. Re-run `fissile init` to refresh it.

### Fixed

- §FS-002-init.5: the `next:` block's closing line now names an agent
  entrypoint the run actually handled instead of always saying `AGENTS.md`.
  Automatic mode updates whichever entrypoints already exist and only falls
  back to `AGENTS.md` when none do, so in a repository carrying `CLAUDE.md` the
  last line a new adopter read pointed at a file that was not there. Resolves
  #3.

- §FS-001-config.2: `respect_gitignore` now prunes the walk instead of
  filtering its result, so an ignored directory is never descended into.
  `audit` previously traversed ignored subtrees in full and discarded them
  afterwards, which only stayed cheap when the subtree also matched an
  `[scan].exclude` glob. On a repository with a gitignored 850 MB
  `scratchpad/`, `audit` went from over two minutes to 0.04 s; the emitted
  findings are byte-identical, since the pruned contents were discarded either
  way. `git check-ignore` is now batched one call per tree level rather than
  one per path, so invocations track depth, not directory count. Resolves #1.

## 3. Older releases

- [0.1.0](changelog/0.1.0.md) — 2026-08-11: The first release: the commit-time file-size gate, its adoption tooling, and the evidence chain behind them (§GND-001-fissile).
<!-- Populated by `prepare_changelog_release.py prepare` when a release ships. -->
