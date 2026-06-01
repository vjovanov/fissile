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
max_accepted = { value = 300000, unit = "bytes" }
until = "review after parser fixture generator lands"
owner = "parser"
reason = """
This fixture is intentionally large because it is a golden corpus copied from
production parser incidents. Retire this exception when the fixture can be
generated deterministically inside the test or split by parser feature without
losing incident coverage.
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
- `reason`: non-empty rationale explaining why the file is accepted and what
  would let the exception be retired.

Optional fields:

- `title`: short human-readable label;
- `owner`: team, person, or component responsible for retiring the exception;
- `issue`: tracker URL or ID;
- `replaces`: prior exception ID when splitting or renaming entries.

There is no `created` field: the date an exception was added is recorded by the
commit that added it, so duplicating it in the entry would only invite drift.

Unknown fields are errors in version 1 so typos cannot silently weaken the
registry.

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

## 6. Adding Entries

`fissile exception add` (§FS-005-exception-add) is the supported command for
adding entries. It measures exact-path files, chooses the configured soft or hard
registry, writes `max_accepted`, and validates the result before modifying the
registry.
