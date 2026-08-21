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

## 2. [0.7.0] — 2026-08-21

### Added

- §FS-004-check-audit.1.1: a run that reports a finding adds one `hint:` line
  naming `fissile measure`, beneath the findings it is about. The finding
  already carries the offending file's size; the hint is about the files a split
  moves code *into*, whose headroom decides where the seam can go.
- §FS-004-check-audit.1.2: a `check --staged` run that exits non-zero closes
  with a commit-gate epilogue naming what blocked it — a split and a reviewed
  hard exception for an overflow, the registry for a dead entry, and the path to
  fix or unstage when a staged file could not be measured at all
  (§FS-004-check-audit.5) — and in every case that `--no-verify` only moves the
  problem into the branch. Every reason the run has to fail has an epilogue, and
  the two are decided together: a blocked commit whose output reads as advisory
  is the failure mode this closes.
  Only `--staged` prints it: a CI or manual run is not blocking anything a
  caller is about to bypass.
- §FS-004-check-audit.1.3: `check` reports exact-path exception entries its own
  run proves have outlived their file, under `[exceptions].stale` — a staged
  deletion or rename under `--staged`, and — on a full scan — an entry matching
  nothing in the inventory with no file at its path. Absence from the working
  tree is deliberately not the test on its own: a generated file a build has not
  written, or one deleted without staging the deletion, has outlived no entry.
  Neither has one the scan scope excludes or git ignores, which is missing from
  the inventory while sitting exactly where the entry says. The glob and scan-scope inventory
  stays in `audit --stale-exceptions`.
  **Migration: `[exceptions].stale = "error"` now fails `check`, not only
  `audit --stale-exceptions`.** Through the pre-commit hook that blocks the
  commit. Repositories that set `error` to make a CI audit strict, and do not
  want it at commit time, should set `warn`.
- §FS-005-exception-add.4: `--reason` that says nothing beyond the finding's own
  facts now warns. It never refuses: the test catches only a reason that is
  entirely restatement, and rejecting a terse honest claim would teach callers
  to pad it.

### Changed

- §FS-002-init.4: the managed agent block is now **v3**, and delimited by
  `<!-- BEGIN FISSILE MANAGED BLOCK -->` / `<!-- END FISSILE MANAGED BLOCK -->`.
  The heading keeps the version, as it always has; the markers say who owns the
  span. The block is five lines rather than thirty-five: the instructions moved
  to the surfaces that raise each question
  (§DF-007-instructions-at-the-error-site). **Migration: re-run `fissile init`
  to upgrade.** A v1 or v2 unmarked block is recognized and replaced in place,
  so a repository upgrades rather than growing a second block. Since the span is
  now the markers and not "everything up to the next H1 or H2", a heading a user
  writes directly beneath the block is outside it and survives a refresh. A
  delimited block whose heading states no version this build can read is refused
  rather than overwritten: the markers carry no version, so there is nothing to
  fall back to, and assuming "current" would silently downgrade a newer block.
- §FS-005-exception-add.4: `exception add --severity hard` is refused when
  standard input is not a terminal, offering this call with `--severity soft` —
  every other flag, `--kind` and `--reason` included, carried through so the
  command runs as printed — and naming `--force`
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
- §FS-003-exceptions.4: `[exceptions].stale` is documented as governing the
  subject rather than one command, and the generated config says so. The
  validator's own rule is stated plainly: an entry is well-formed whether or not
  its path is on disk today.

## 3. Older releases

- [0.6.0](changelog/0.6.0.md) — 2026-08-21: - §FS-007-measure: new `fissile measure <paths>...
- [0.5.0](changelog/0.5.0.md) — 2026-08-12: - §FS-003-exceptions.1: the exception registry schema is now `fissile_exceptions_version = 2`.
- [0.4.0](changelog/0.4.0.md) — 2026-08-11: - §FS-003-exceptions.2.1: exception entries carry `kind = "structural" | "deferred"`, which fixes what `reason` must establish — the architectural constraint that makes the split illegal, or the boundary that is missing and what has to exist first (§DF-004-exception-kind).
- [0.3.0](changelog/0.3.0.md) — 2026-08-11: - §FS-006-cli.2: the usage screen opens with a short paragraph — what `fissile` is for, the two tiers, the `check --staged` habit, and the rule that a budget is never met by damaging the design — closing with a pointer to `fissile init --dry-run` for the full agent instructions.
- [0.2.0](changelog/0.2.0.md) — 2026-08-11: - §FS-001-config.3: rules take `soft_message` and `hard_message`, so a warning and a block can say different things (§DF-003-severity-guidance).
- [0.1.0](changelog/0.1.0.md) — 2026-08-11: The first release: the commit-time file-size gate, its adoption tooling, and the evidence chain behind them (§GND-001-fissile).
<!-- Populated by `prepare_changelog_release.py prepare` when a release ships. -->
