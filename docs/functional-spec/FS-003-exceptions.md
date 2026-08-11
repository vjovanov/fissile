# FS-003-exceptions: oversized files are accepted through configured registries

Exception registries are TOML documents that record every file or glob accepted
above a configured limit. Version 2 uses two registries with configurable paths:

- the soft registry, default `docs/file-size-agent-exceptions.toml`, accepts
  agent-facing soft-limit warning debt;
- the hard registry, default `docs/file-size-human-exceptions.toml`, accepts
  human-reviewed hard-limit blocking debt.

The hard registry is the only hard-limit escape hatch. Both registries are typed
data plus reviewable rationale: a reviewer or agent can read why the file is
large, and `fissile` can parse which path and rule are waived. The registry file,
not a field inside the entry, determines whether an entry waives soft or hard
findings. Each entry also records the largest accepted measurement, so an
exception starts reporting again if the file keeps growing.

## 1. File Shape

The file is a versioned TOML document:

```toml
fissile_exceptions_version = 2

[[exceptions]]
title = "generated parser fixture"
path = "tests/fixtures/parser/large-corpus.json"
match = "exact"
rules = ["fixtures"]
kind = "deferred"
max_accepted = { value = 300000, unit = "bytes" }
until = "the fixture generator lands"
owner = "parser"
reason = """
Missing boundary: a generator that reproduces this corpus from the incident
descriptions. Until one exists the corpus can only be split by hand, and a hand
split loses the incident-to-case mapping the fixture exists to preserve.
"""
```

`fissile_exceptions_version` is required and must be `2`; version 2 removed the
`id` and `replaces` keys version 1 carried (§2.2). Unknown keys are errors. An
entry has no name of its own: it is identified by the registry it lives in and
what it accepts (§DF-005-exception-identity). Exception files are parsed only
according to this functional spec.

## 2. Fields

Required fields:

- `path`: repo-relative path or glob being accepted;
- `match`: `exact` or `glob`;
- `rules`: array of rule IDs, or `["*"]` for every matching rule;
- `max_accepted.value`: largest measurement this exception accepts;
- `max_accepted.unit`: `bytes`, `lines`, or `tokens`;
- `until`: review condition, date, or `indefinite`;
- `reason`: non-empty rationale, establishing what §2.1 requires of the entry's
  kind.

Optional fields:

- `kind`: `structural` or `deferred` (§2.1); an entry without one reads as
  `deferred`;
- `title`: short human-readable label;
- `owner`: team, person, or component responsible for retiring the exception;
- `issue`: tracker URL or ID.

There is no `created` field: the date an exception was added is recorded by the
commit that added it, so duplicating it in the entry would only invite drift.

Unknown fields are errors so typos cannot silently weaken the registry. That is
also what makes a leftover `id` a named error rather than a silent no-op (§2.2).

### 2.1 Kind, And What A Reason Must Establish

An accepted oversized file is one of two things, and the entry says which
(§DF-004-exception-kind). The kind decides what the `reason` has to claim and
what `until` may say; neither kind is answered by describing the file's contents,
which is what the finding already reported.

- `kind = "structural"` — an architectural constraint makes the split illegal.
  The reason names that constraint and what would break if the file were split
  anyway. Nothing retires the entry, so `until` is `indefinite`; any other value
  is a schema error.
- `kind = "deferred"` — no such constraint exists; a boundary is missing. The
  reason names the missing boundary and what has to exist before the split is
  possible. `until` carries the retirement condition and may not be `indefinite`.

An entry that omits `kind` is read as `deferred` for reporting, and the
`kind`/`until` agreement above is checked only on entries that declare a `kind`
— so an entry carried over from a registry written before the field existed
still loads. `fissile exception add` always writes the field explicitly
(§FS-005-exception-add.3).

`indefinite` is matched case-insensitively after trimming, so `Indefinite` and
`indefinite ` are the same value.

### 2.2 What Version 2 Removed, And How To Migrate

A version-1 entry carried a required `id` (`EX-NNN-slug`) and an optional
`replaces` naming another entry's id. Version 2 removes both: what identifies an
entry is the registry it lives in and the condition it accepts, and a second name
for that is a name that can be wrong (§DF-005-exception-identity).

The removal is a break, not a tolerated leftover. A registry declaring version 1
is rejected, and a version-2 registry that still carries `id` fails on the
unknown key (§2) — so no file is left holding a field that silently means
nothing. Migrating one registry is two edits:

1. set `fissile_exceptions_version = 2`;
2. delete every `id` and `replaces` line.

Nothing else about an entry changes. The version error names both edits, because
every adopter meets it once, on upgrade, and a message that only states the
version leaves the remedy to be guessed (§GOAL-003-friendly-output.1). An
unmigrated registry breaks both rules at once — it declares version 1 *and*
carries `id` keys — and the version error is the one reported, because it is the
one that names the whole fix.

`fissile exception add` writes version-2 registries and never writes either key
(§FS-005-exception-add.3).

## 3. Matching

`match = "exact"` compares `path` to the repo-relative normalized path.
`match = "glob"` uses the same glob engine as config rules. An exception applies
only when the path matcher, the `rules` field, the registry severity, and
`max_accepted` match the overflow. `max_accepted.unit` uses the matched rule's
unit and `max_accepted.value` must be greater than or equal to the rule limit for
the registry severity. A soft-registry entry silences only soft findings at or
below its accepted maximum. A hard-registry entry silences only hard findings at
or below its accepted maximum. If the measured value is higher than
`max_accepted.value`, `fissile` reports the overflow again.

A hard finding that still stands — no entry matched, or the file grew past its
ceiling — suppresses the soft finding on its own (§GOAL-006-graded-limits.1).
When a hard finding is *silenced*, the accepting entry's `kind` (§2.1) decides
what happens to the soft finding for the same overflow:

- `kind = "deferred"` — the soft finding is still emitted, so agents keep
  minimizing accepted human debt. An entry that declares no kind reads as
  `deferred` (§2.1) and behaves this way.
- `kind = "structural"` — the soft finding is silenced too. Splitting the file is
  illegal, so the warning asks for work nobody may do and no amount of work can
  clear it. A structural hard entry ends the evaluation of that overflow: the
  soft registry is not consulted, and a soft entry that would have matched the
  same overflow silences nothing while the hard entry is doing it.

The rule reads the hard entry's kind only. A `kind` in the soft registry says
what its own reason must establish (§2.1) and changes nothing about matching.

The rule reaches only as far as a hard finding does. A file below the hard limit
produces no hard finding, so the hard registry is never consulted for it and the
soft warning stands on its own, structural constraint or not. Accepting that
warning is the soft registry's job — with `kind = "structural"` there too, when
the same constraint is what makes it permanent. A soft entry paired with a
structural hard one is therefore dormant, not dead: it takes over exactly where
the hard entry stops applying.

When more than one exception in the same severity registry matches the same
overflow, `fissile` reports a schema error. One accepted oversized condition at
one severity should have one rationale. A single exception entry may list
multiple rules only when all listed rules use the same unit.

## 4. Validation

`fissile` validates both registries before evaluating overflows:

- every required field is present once;
- every listed rule ID exists, unless `rules = ["*"]`;
- `max_accepted.value` is a positive integer;
- `max_accepted.unit` is `bytes`, `lines`, or `tokens`;
- `max_accepted.unit` matches every rule the entry can silence;
- `max_accepted.value` is at least the corresponding soft or hard rule limit;
- `reason` is not empty after trimming whitespace;
- a declared `kind` agrees with `until` (§2.1): `structural` requires
  `indefinite`, `deferred` forbids it;
- every matched path is inside the scan scope unless stale handling is disabled;
- every stale entry follows `[exceptions].stale`: `warn`, `error`, or `ignore`.

Every diagnostic about a single entry leads with the registry file and the
entry's `path`, because that pair is the line the reader has to edit
(§DF-005-exception-identity):

```text
docs/file-size-human-exceptions.toml: src/orders.rs has an empty reason
```

The registry file is part of the identifier: the same path may appear in both
registries, making two different claims at two different severities.

The validator does not require the target file to exist during `check --staged`
because a staged deletion may make the path temporarily absent. Whole-repo
`audit --stale-exceptions` performs the stale-path inventory.

## 5. Output

An overflow silenced by an exception emits no default finding for that severity.
In verbose audit output, `fissile` includes the severity and the accepted ceiling
so a reviewer can find the entry — in the registry the severity names, under the
path already on the line:

```text
tests/fixtures/parser/large-corpus.json: hard exception (accepted up to 300000 bytes)
```

JSON output carries the same ceiling as `exception_max`.

`audit` also counts the registries by kind (§FS-004-check-audit.2), so a reader
sees accepted-permanently and carrying-debt as two numbers rather than one
undifferentiated total.

## 6. Adding Entries

`fissile exception add` (§FS-005-exception-add) is the supported command for
adding entries. It measures exact-path files, chooses the configured soft or hard
registry, writes `max_accepted`, and validates the result before modifying the
registry.
