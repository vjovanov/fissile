# FS-003-exceptions: oversized files are accepted through a cited registry

The exception registry is a markdown file, default
`docs/file-size-exceptions.md`, that records every file or glob accepted above a
configured limit. It is the only hard-limit escape hatch. The registry is
reviewable prose plus structured fields: a reviewer can read why the file is
large, and `fissile` can parse which path, rule, and limit are waived.

## 1. File Shape

The file starts with a normal title:

```markdown
# File Size Exceptions
```

Each exception is a second-level grund declaration:

```markdown
## EX-001-generated-parser-fixture: generated parser fixture

`tests/fixtures/parser/large-corpus.json` is intentionally large because it is a
golden corpus copied from production parser incidents. Retire this exception when
the fixture can be generated deterministically inside the test or split by parser
feature without losing incident coverage.

- **Path:** `tests/fixtures/parser/large-corpus.json`
- **Match:** exact
- **Rules:** `fixtures`
- **Limit waived:** hard
- **Until:** review after parser fixture generator lands
- **Owner:** `parser`
- **Created:** `2026-05-26`
```

The ID prefix defaults to `EX` and is configured by `[exceptions].kind` in
§FS-001-config. The declaration body must contain at least one prose paragraph
before the structured fields. Empty rationales are parse errors.

## 2. Fields

Required fields:

- `Path`: the repo-relative path or glob being accepted;
- `Match`: `exact` or `glob`;
- `Rules`: comma-separated rule IDs, or `*` for every matching rule;
- `Limit waived`: `soft`, `hard`, or `both`;
- `Until`: review condition, date, or `indefinite`.

Optional fields:

- `Owner`: team, person, or component responsible for retiring the exception;
- `Created`: ISO date when the exception was added;
- `Issue`: tracker URL or ID;
- `Replaces`: prior exception ID when splitting or renaming entries.

Unknown fields are errors in version 1 so typos cannot silently weaken the
registry.

## 3. Matching

`Match: exact` compares `Path` to the repo-relative normalized path. `Match:
glob` uses the same glob engine as config rules. An exception applies only when
both the path matcher and the `Rules` field match the overflow rule. If `Limit
waived` is `hard`, a soft warning still appears when the file is below the hard
limit but above the soft limit; `both` silences both tiers.

When more than one exception matches the same overflow, `fissile` reports a
schema error. One accepted oversized condition should have one rationale.

## 4. Validation

`fissile` validates the registry before evaluating overflows:

- every exception ID is unique;
- every required field is present once;
- every listed rule ID exists, unless `Rules: *`;
- every `§` citation in the rationale resolves under `grund check` when the repo
  uses grund;
- every matched path is inside the scan scope unless stale handling is disabled;
- every stale entry follows `[exceptions].stale`: `warn`, `error`, or `ignore`.

The validator does not require the target file to exist during `check --staged`
because a staged deletion may make the path temporarily absent. Whole-repo
`audit --stale-exceptions` performs the stale-path inventory.

## 5. Output

An overflow silenced by an exception emits no default finding. In verbose audit
output, `fissile` includes the exception ID so a reviewer can resolve the
rationale:

```text
tests/fixtures/parser/large-corpus.json: exception EX-001-generated-parser-fixture
```

JSON output carries the same ID as `exception_id`.

An example registry lives at `examples/file-size-exceptions.md`.
