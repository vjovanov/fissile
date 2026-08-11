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

- §FS-003-exceptions.1: the exception registry schema is now
  `fissile_exceptions_version = 2`. **Breaking: every existing registry must be
  edited before this build will read it.** Version 2 removes the `id` and
  `replaces` keys (§FS-003-exceptions.2.2, §DF-005-exception-identity), and the
  removal is a break rather than a tolerated leftover — a version-1 registry is
  refused, and a version-2 entry that kept an `id` fails on the unknown key, so
  no file is left carrying a field that silently means nothing. The version
  error names both edits instead of only the version this build supports.
  Migrating both registries is one command:

  ```sh
  sed -i -E -e '/^[[:space:]]*(id|replaces) = /d' \
    -e 's/^fissile_exceptions_version = 1$/fissile_exceptions_version = 2/' \
    docs/file-size-agent-exceptions.toml docs/file-size-human-exceptions.toml
  ```

  BSD `sed`, as on macOS, needs `sed -i ''` in place of `sed -i`. It edits by
  line, so read the diff — and anything it missed is a named error on the next
  `fissile check`, not a silent pass. Nothing else about an entry changes, and
  `fissile exception add` writes version-2 registries. Install the new binary
  *before* committing the migration: the pre-commit hook (§FS-002-init.6) runs
  whatever `fissile` is on `PATH`, and an older one refuses the migrated
  registry, blocking the very commit that fixes it.
- §FS-003-exceptions.4: a diagnostic about one entry leads with the registry file
  and the entry's `path` — `docs/file-size-human-exceptions.toml: src/orders.rs
  has an empty reason` — rather than naming an id. That pair is the line the
  reader has to edit, and it stays unambiguous when the same path appears in both
  registries. `exception add` names the blocking entry the same way when it
  refuses an overlapping one.
- §FS-004-check-audit.1: **breaking for JSON consumers.** A silenced `audit`
  record no longer carries `exception_id`, and `audit --stale-exceptions` items
  are now `{ "registry", "path" }` in place of `{ "id", "path" }` — the list
  spans both registries, so the registry is what disambiguates a path stale in
  each. The text attribution drops the id too: `src/orders.rs: hard exception
  (accepted up to 620 lines)`, and a stale line reads
  `docs/file-size-human-exceptions.toml: src/gone.rs`.

### Removed

- §FS-005-exception-add.1: `--id` and `--replaces`, with the `EX-NNN` allocator,
  the slug derivation, and the cross-registry id uniqueness check behind them. An
  entry is already identified by the registry it lives in and what it accepts —
  path matcher, rules, unit — and `fissile` already refuses two entries matching
  one overflow in one registry (§FS-003-exceptions.3), so the id was a second
  name for a tuple the tool computes, and the second name is the one that can be
  wrong (§DF-005-exception-identity).

## 2. [0.4.0] — 2026-08-11

### Added

- §FS-003-exceptions.2.1: exception entries carry `kind = "structural" |
  "deferred"`, which fixes what `reason` must establish — the architectural
  constraint that makes the split illegal, or the boundary that is missing and
  what has to exist first (§DF-004-exception-kind). `structural` never expires
  and requires `until = "indefinite"`; `deferred` requires a retirement
  condition and forbids `indefinite`. The agreement is checked only on entries
  that declare a `kind`.
- §FS-005-exception-add.1: `--kind structural|deferred` is required, and
  `--until` becomes conditional — optional for `structural`, where it defaults
  to `indefinite`, and required for `deferred`. Each rejection names the other
  kind, because that is usually the real correction.
- §FS-004-check-audit.2: `audit` reports the registries by kind — how many files
  are accepted permanently and how many carry debt — as two numbers rather than
  one total. Text omits the section when both registries are empty; JSON always
  carries `exceptions.structural` and `exceptions.deferred`.

### Changed

- §DF-003-severity-guidance.1, §FS-002-init.4: the shipped remediation messages
  and the managed agent block now say what a reason must *establish*, not just
  that one is required. "Record the debt — a written reason and a revisit
  trigger" reads as satisfied by any sentence, which is how registries fill with
  entries that restate the finding. The README example changed for the same
  reason: `"legacy order engine; splitting tracked in #142"` modelled a status
  note.
- §FS-003-exceptions.4: an entry whose `until` is empty after trimming is now a
  schema error, alongside the existing empty-`reason` check.

No registry migration is required: `fissile_exceptions_version` stays `1`, and
an entry without `kind` loads and reports as `deferred`.

## 3. Older releases

- [0.3.0](changelog/0.3.0.md) — 2026-08-11: - §FS-006-cli.2: the usage screen opens with a short paragraph — what `fissile` is for, the two tiers, the `check --staged` habit, and the rule that a budget is never met by damaging the design — closing with a pointer to `fissile init --dry-run` for the full agent instructions.
- [0.2.0](changelog/0.2.0.md) — 2026-08-11: - §FS-001-config.3: rules take `soft_message` and `hard_message`, so a warning and a block can say different things (§DF-003-severity-guidance).
- [0.1.0](changelog/0.1.0.md) — 2026-08-11: The first release: the commit-time file-size gate, its adoption tooling, and the evidence chain behind them (§GND-001-fissile).
<!-- Populated by `prepare_changelog_release.py prepare` when a release ships. -->
