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

### Changed

- Testing: adopted grund's two non-citable test homes. The e2e corpus moved
  from `e2e/cases` to `tests/e2e/cases` (harness at `tests/e2e/main.rs`), the
  cross-part proofs that lived beside it in `tests/` moved into
  `tests/integration/`, and every `grund.toml` `[[kinds]]` block's deprecated
  `prefix` key is now `kind`. Resolves #39. (PR #40)

## 2. [0.8.1] — 2026-08-30

### Changed

- §FS-002-init.5: `init::Report` carries one `HookStatus` — `Installed`,
  `SkippedNotGit`, `SkippedByFlag` — in place of the `hook_skipped_not_git`
  boolean, so the hook step 2 reports is a value every path has to answer for
  instead of a flag that can be left unset. This breaks a library caller's
  source, so the release carrying it is 0.9.0, not 0.8.1; no config, registry,
  or managed-block version moves, and the CLI output is unchanged apart from
  the fix below. **Migration:** reading the field becomes `report.hook ==
  HookStatus::SkippedNotGit`, but its inverse does not — the
  `!report.hook_skipped_not_git` idiom meant "a hook is installed", and that
  false branch has split in two, so the installed test is now `report.hook ==
  HookStatus::Installed`. A caller constructing a `Report` sets `hook`. (PR #35)

### Fixed

- §FS-002-init.6, §FS-002-init.5: `fissile init` recognizes a linked git
  worktree as a repository instead of telling the reader to `git init` inside
  one. Automatic mode wrote no hook and reported the not-a-git-repository step,
  and `--hook` errored, because `<root>/.git` is a file there, not a directory;
  `init` now reads that file's `gitdir:` pointer (and its `commondir`, when
  present) to find the repository and installs the hook into its shared
  `hooks/pre-commit`, the same file every worktree of that repository reads.
  Resolves #36. (PR #38)
- §GOAL-006-graded-limits.1, §GOAL-006-graded-limits.2,
  §FS-004-check-audit.1: budget findings now fire strictly above their limits
  and line findings name the counting basis and budget (PR #37)
- §FS-002-init.5, §FS-002-init.6: `fissile init --no-hook` no longer tells the
  reader to `Commit a change to see the pre-commit hook run fissile check
  --staged` after installing no hook. Step 2 of the `next:` block was picked
  from a boolean that only the automatic not-a-git-repository skip ever set, so
  the one flag whose purpose is to decline the hook fell through to the promise.
  Step 2 now reports the hook the run leaves in `.git/hooks/pre-commit`: with
  none there it names the flag and the wiring left to do, and a hook an earlier
  run installed still earns the commit invitation. Resolves #13. (PR #35)

## 3. Older releases

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
