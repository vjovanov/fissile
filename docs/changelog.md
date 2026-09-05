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

- §FS-004-check-audit.2: `fissile audit --only <section>[,<section>]` prints the
  named sections of the text report and nothing else, so tuning config or
  pruning the registry stops paying for a findings block the reader is not
  looking at. The names are the seven top-level keys of
  `schema/audit.schema.json` — `findings`, `silenced`, `exceptions`, `top`,
  `stale`, `loose`, `coverage` — and they render in that order whatever order
  they were named in. Naming a section is the request to compute it, so `--only
  coverage` needs no `--rule-coverage`; `--only top` still needs `--top <N>`,
  which carries a count no default could stand in for. Selection reaches the
  screen and nothing else: exit status is computed from the whole run, so a
  standing hard overflow still exits non-zero under `--only coverage`. An
  unknown or empty name is a usage error naming the valid set, and `--only`
  with `--format json` is one too — `findings`, `silenced` and `exceptions` are
  `required` in the schema, so a filtered object would not validate. `audit`
  with no `--only` prints what it printed before, and the JSON surface is
  unchanged. Resolves #15. (PR #63)
- §GOAL-004-token-thrift.1, §FS-006-cli.2: `fissile audit --help` names the JSON
  route, `fissile audit --format json --rule-coverage | jq .coverage`, and says
  that `--format json` is the agent surface. The goals document has designated
  it one since §GOAL-004-token-thrift was written; the screen's two examples
  were both text and said so nowhere. (PR #63)

### Changed

- §FS-004-check-audit.2: `fissile::audit::AuditOptions` gains a public
  `only: Option<Vec<Section>>` field, and `fissile::audit` gains the public
  `Section` enum and the `SECTIONS` array that fixes its canonical order. A
  library caller constructing the options with a struct literal must initialize
  the new field, normally with `None`, which is the whole report. A 0.x source
  break, so the minor number moves. (PR #63)

- §AR-001-ci.2: CI's `grund check` job pins `grund` 0.13.0, up from 0.12.3, and
  the agent entrypoint's grund managed block moves from v7 to v8 to match.
  0.13.0 turns an unindexed declaration into a `grund check` error instead of a
  warning and stops accepting the `prefix` spelling of `[[kinds]] kind`; this
  repository already satisfied both, so pinning the new release found nothing
  else to fix. (PR #64)

## 2. [0.8.3] — 2026-09-05

### Added

- §FS-001-config.8, §FS-002-init.2: the config's home is
  `.agent-grounds/fissile.toml`. Every command that discovers a config looks
  there first and falls back to `.agents/fissile.toml`, `fissile init` writes the
  new home, and `--config` is unchanged. `.agents/` is by convention where agent
  *instructions* live, and sandboxed agent runtimes mount it read-only, so a tool
  that has to maintain its own config could not write the one file it owns
  (§DF-012-config-home). A repository that already has a config at the old path
  keeps it byte-for-byte and gets no second one written at the new home, because
  a generated default there would take precedence over the project's own rules on
  the very next run. Sibling issues carry the same move in `grund`, `rhei` and
  `ephor`; `.agents/skills` is out of scope everywhere. Resolves #61. (PR #62)

- §FS-001-config.3.4, §FS-010-limits.3, §FS-010-limits.4: a `[[rules]]` entry
  may declare `exclude` globs that remove a path from that rule before overlap
  resolution while leaving every other unit and rule eligible. This is distinct
  from `[scan].exclude`, which removes a path from the whole scan, and from an
  exception, which accepts an overflow of a rule that still applies. `limits`
  prints non-empty exclusions after `include` in text and JSON and omits empty
  lists, so the addition stays compatible with existing version-1 configs and
  output. Generated exhaustive configs spell the empty default. (PR #59)
- §FS-004-check-audit.1: each file detail of a `check` or `audit` finding names
  the ceiling a `fissile exception add` with no `--max` would record for that
  file — `(budget 550; an exception here would accept 700)` for a line rule, a
  parenthesis of its own where the unit carries no budget clause — and the JSON
  record carries the same number as `exception_would_accept`. The measurement
  was the only number on screen, so a caller reaching for an exception copied it
  into `--max`, which is written exactly as stated
  (§DF-010-stated-ceilings-are-exact.1); the entry then had no headroom and the
  next unrelated edit failed the gate. The number is the one the command already
  computes (§DF-006-quantized-ceilings.1), said at the moment the caller chooses
  between the two forms. It is withheld where that plain call would be refused —
  a soft ceiling reaching the rule's hard limit for a file still under it — and
  a finding does not read the exception registries, so it withholds from the
  deferred-hard-twin case `add` would accept. Resolves #49. (PR #58)
- §FS-010-limits: new `fissile limits` prints every rule the tree configures —
  id, include patterns, unit, and the soft and hard limits each declares — in
  config declaration order, with `--config`, `--format text|json` and
  `--no-color` as `audit` takes them. Nothing printed a repository's rule
  inventory before: `check` and `audit` report findings, so a passing tree said
  nothing, and `measure` answers only for paths a caller already named, so the
  numbers were copied into prose that nothing checks. The JSON form is an object
  keyed `rules` and carries each rule's `priority`, message ids and line-counting
  policy, published as `schema/limits.schema.json`, so a documented limit can be
  generated or verified rather than maintained by hand. It loads the config and
  not the exception registries, so it answers with exit `0` in a tree whose
  registries `check` and `audit` refuse. Resolves #46. (PR #56)
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
- §FS-009-exception-remove: new `fissile exception remove <path> --severity
  soft|hard --rule <id>` deletes one exception entry, addressing it exactly as
  `exception retune` does and supporting `--config`, `--match` and `--dry-run`.
  It refuses to delete an entry that is still silencing a finding, and it is the
  one command that loads a registry the rule check rejects — so an entry whose
  ceiling a raised limit left below that limit, which aborted `check`, `audit`,
  `measure` and the hook before they measured anything, is now removable without
  hand-editing TOML. `retune`'s min-limit refusal, `check`'s stale-entry
  guidance and `audit`'s "silences nothing" line all name the new command.
  (PR #53)

- §FS-003-exceptions.7, §FS-004-check-audit.2: `fissile audit
  --stale-exceptions` now reports an exact-path entry whose ceiling sits exactly
  on the file it accepts, in the `loose ceilings:` section, with its advice
  prefixed `no headroom`. Such an entry silences the finding today and stops on
  the next unrelated commit, and nothing named it: the one-step test that
  excuses a freshly quantized ceiling excused a spent one too. The advice is the
  first of four cases that applies, and each is a call the named command
  performs — removal where the file no longer crosses the limit; the stated form
  and a range where the step would land a soft ceiling on the hard limit, or the
  hard registry where that range is empty; the stated form carrying the step's
  next multiple where the measurement is already one; and otherwise `retune to`
  that multiple. Every `loose` JSON record gains `no_headroom`, `0` or `1`, and
  `stated_range` may now carry `min` alone, so `max_excluded` leaves its
  `required` list in `schema/audit.schema.json`. Loose entries, the section
  heading, and the exit codes are unchanged. Resolves #48. (PR #57)

### Changed

- §FS-001-config.8.1: `fissile::cli::Loaded` gains a public `source:
  ConfigSource` field, and `audit`, `measure`, `limits` and the three
  `exception` command `Run` types each gain a public `notes: Vec<String>` —
  what discovery owes stderr, carried out to the surface that owns it.
  `fissile::init::Report` gains `config: PathBuf`, the config the run wrote or
  found, and `deprecation: Option<&'static str>`. A library caller constructing
  any of these with a struct literal must initialize the new field.
  `Config::load(root, explicit)` keeps its signature; the search order is
  reported through the additive `Config::discover`. A 0.x source break, so the
  minor number moves. (PR #62)

- §FS-001-config.3.4: `fissile::config::RuleSpec` gains a public
  `exclude: Vec<String>` field. A library caller constructing the public struct
  directly must initialize it, normally with `Vec::new()`; parsed configs
  default it automatically. A 0.x source break, so the minor number moves.
  (PR #59)
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

- §FS-010-limits.4: `fissile::json::Json` gains a `Bool` variant, for the two
  line-counting flags `limits --format json` emits. The enum is not
  `#[non_exhaustive]`, so an exhaustive external `match` on it stops compiling:
  add the new arm, or a wildcard. Same minor bump. (PR #56)

- §AR-001-ci: every declaration is listed in its folder's index. `grund check`
  warned that thirteen declarations were absent from their index README and that
  the warning becomes an error in grund 0.13.0; the two decision folders that had
  no index at all now have one. (PR #51)

- §AR-001-ci.8.2: after a release, main opens the next patch as `X.Y.Z-dev`
  instead of keeping the version it just published. A build from main previously
  reported the tag it was already ahead of, so a merged-but-uninstalled fix was
  indistinguishable from an installed one. (PR #50)

### Deprecated

- §FS-001-config.8.2: `.agents/fissile.toml` is deprecated as the config path.
  It is still discovered, one step behind `.agent-grounds/fissile.toml`, and it
  changes no exit code — but every run that reads it prints one warning line on
  stderr naming the move, and a run that finds a config at both paths says which
  one it ignored. Nothing breaks on upgrade; a repository migrates by moving the
  file. (PR #62)

### Fixed

- §FS-009-exception-remove.2: soft `exception remove` can delete an orphaned
  `shadows = "hard"` entry after its hard twin is removed, while ordinary
  registry loading stays strict and continues to reject orphan shadows.
  Resolves #55. (PR #60)
- §FS-004-check-audit.2: `audit` now reports exception entry totals alongside
  distinct path-expression totals, so soft/hard twins and repeated globs are
  not presented as additional files. (PR #47)
- §FS-005-exception-add.2: `exception add --max` now names the step's next
  multiple in its normal and its `--dry-run` result, as `exception retune --max`
  already did — the round number the measured form would have written, named and
  never applied. A ceiling stated at the day's measurement has no headroom, and
  the silence let four such ceilings ship before unrelated growth of 6 and 38
  lines in two other files failed CI. The suggestion is withheld exactly where
  the command would refuse the number, so it never offers a ceiling the next
  call would reject. Registry contents are unchanged. Resolves #45. (PR #54)

## 3. Older releases

- [0.8.2](changelog/0.8.2.md) — 2026-08-31: - §FS-001-config.0.1: the built-in defaults budget a Markdown file by how it is read.
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
