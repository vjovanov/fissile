# FS-003-exceptions: oversized files are accepted through configured registries

Exception registries are TOML documents that record every file or glob accepted
above a configured limit. Version 1 uses two registries with configurable paths:

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
fissile_exceptions_version = 1

[[exceptions]]
id = "EX-001-generated-parser-fixture"
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

`fissile_exceptions_version` is required and must be `1`. Unknown keys are
errors. The `id` uses the `EX-` prefix and is local to `fissile`; exception files
are parsed only according to this functional spec.

## 2. Fields

Required fields:

- `id`: stable local exception ID with the `EX-` prefix;
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
- `issue`: tracker URL or ID;
- `replaces`: prior exception ID when splitting or renaming entries.

There is no `created` field: the date an exception was added is recorded by the
commit that added it, so duplicating it in the entry would only invite drift.

Unknown fields are errors in version 1 so typos cannot silently weaken the
registry.

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
`kind`/`until` agreement above is checked only on entries that declare a `kind`.
Registries written before the field existed therefore keep loading unchanged;
`fissile exception add` always writes the field explicitly
(§FS-005-exception-add.3).

`indefinite` is matched case-insensitively after trimming, so `Indefinite` and
`indefinite ` are the same value.

## 3. Matching

`match = "exact"` compares `path` to the repo-relative normalized path.
`match = "glob"` uses the same glob engine as config rules. An exception applies
only when the path matcher, the `rules` field, the registry severity, and
`max_accepted` match the overflow. `max_accepted.unit` uses the matched rule's
unit and `max_accepted.value` must be greater than or equal to the rule limit for
the registry severity. A soft-registry entry silences only soft findings at or
below its accepted maximum. A hard-registry entry silences only hard findings at
or below its accepted maximum. If the measured value is higher than
`max_accepted.value`, `fissile` reports the overflow again. If a hard finding is
silenced and no matching soft exception exists, `fissile` may still emit the soft
finding so agents can keep minimizing accepted human debt.

When more than one exception in the same severity registry matches the same
overflow, `fissile` reports a schema error. One accepted oversized condition at
one severity should have one rationale. A single exception entry may list
multiple rules only when all listed rules use the same unit.

## 4. Validation

`fissile` validates both registries before evaluating overflows:

- every exception ID is unique across both registries;
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

The validator does not require the target file to exist during `check --staged`
because a staged deletion may make the path temporarily absent. Whole-repo
`audit --stale-exceptions` performs the stale-path inventory.

## 5. Output

An overflow silenced by an exception emits no default finding for that severity.
In verbose audit output, `fissile` includes the exception ID and severity so a
reviewer can resolve the rationale:

```text
tests/fixtures/parser/large-corpus.json: hard exception EX-001-generated-parser-fixture (accepted up to 300000 bytes)
```

JSON output carries the same ID as `exception_id` and the same ceiling as
`exception_max`.

`audit` also counts the registries by kind (§FS-004-check-audit.2), so a reader
sees accepted-permanently and carrying-debt as two numbers rather than one
undifferentiated total.

## 6. Adding Entries

`fissile exception add` (§FS-005-exception-add) is the supported command for
adding entries. It measures exact-path files, chooses the configured soft or hard
registry, writes `max_accepted`, and validates the result before modifying the
registry.
