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

## 2. [0.1.0] — 2026-08-11

The first release: the commit-time file-size gate, its adoption tooling, and
the evidence chain behind them (§GND-001-fissile).

### Added

- §FS-001-config: one versioned TOML document (`.agents/fissile.toml`) with
  per-glob rules in three units (bytes, lines, tokens), soft/hard limits,
  per-rule line-counting policy, scan scope, and project-owned remediation
  messages (§GOAL-005-configurable, §GOAL-008-remediation-messages). A built-in
  default budgets common source extensions wherever they live.
- §FS-002-init: `fissile init` writes the starter config, optional exception
  registries, versioned managed blocks in agent instruction files, and the
  managed pre-commit hook — idempotently, preserving user content, with
  `--dry-run`. Outside a git repository the `next:` block points at the repair
  instead of promising a hook that was not installed (§FS-002-init.5).
- §FS-004-check-audit: `check` (staged set, explicit paths, or scan scope;
  hard blocks, soft warns) and `audit` (`--top`, `--stale-exceptions`,
  `--rule-coverage`). Staged checks measure the index blob, not the worktree.
  Text findings lead with the path; `--format json` emits the byte-stable
  record shape validated against the schemas under `schema/`
  (§GOAL-003-friendly-output, §GOAL-004-token-thrift).
- §FS-004-check-audit.5: file-level failures skip the path with a stderr line
  that names it and force exit `2`, instead of aborting the run; non-UTF-8
  content measures physical lines from raw bytes (§FS-001-config.3.1); config
  diagnostics name their document (§FS-001-config.1); a staged check outside a
  repository says so.
- §FS-003-exceptions / §FS-005-exception-add: two structured registries — soft
  debt recorded by agents, hard debt reviewed by humans — where every entry
  carries a reason, an accepted maximum that re-triggers when outgrown, and
  stale detection (§GOAL-007-justified-exceptions). `fissile exception add`
  writes entries from the command line.
- §FS-006-cli: `--version`/`-V` prints one stable line; every help screen fits
  in 24 lines, enforced by a test.
- §AR-001-ci: three-platform CI with instruction-count regression gates
  (§AR-002-instruction-benchmarks, §DA-002-instruction-count-benchmarks), a
  performance smoke guard, grounding checks, a binary-size ceiling
  (§GOAL-002-tiny-footprint.3), and PGO pre-release builds.
- §AR-001-ci.8: the release workflow — PGO binaries for six targets (manylinux
  Linux x86_64/aarch64, macOS Intel/Apple silicon, Windows x86_64/aarch64),
  each self-checked against `fissile --version`, published to crates.io and a
  GitHub release with per-artifact SHA-256 sums; `auto-bump.yml` and
  `release-minor.yml` prepare versions but never publish by themselves.
- E2E: a fixture-driven harness drives the real binary with one case per
  documented behavior (§E2E-001-check-clean through §E2E-013-init-no-git).
- MIT license, `rust-version = 1.85`, and a crate package trimmed to the
  library, binary, benches, and schemas (§GOAL-002-tiny-footprint).

## 3. Older releases

<!-- Populated by `prepare_changelog_release.py prepare` when a release ships. -->
