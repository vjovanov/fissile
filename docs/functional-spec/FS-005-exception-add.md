# FS-005-exception-add: fissile exception add writes structured exception entries

`fissile exception add` is the supported way to add entries to the soft and hard
exception registries. Users should not need to hand-edit registry TOML for the
common case of accepting a current overflow.

## 1. Command

```text
fissile exception add <path> --severity soft|hard --rule <id>
                      --reason <text> --until <text>
                      [--config <path>] [--match exact|glob]
                      [--id <id>] [--title <text>] [--owner <text>]
                      [--issue <text>] [--replaces <id>]
                      [--max <N> --unit bytes|lines|tokens]
                      [--dry-run]
```

`--severity` chooses the configured registry: `soft` writes to
`[exceptions].soft_registry`; `hard` writes to `[exceptions].hard_registry`.
`--rule` may be repeated to create one exception for multiple same-unit rules.
`--reason` and `--until` are required so every accepted oversized file has a
reviewable rationale and a retirement condition.

`--match` defaults to `exact`. `glob` is allowed only when `<path>` contains a
glob metacharacter. The command never creates `[scan].exclude` entries; accepted
oversized files remain under `fissile` measurement.

## 2. Accepted Size

When `--max` is omitted, `fissile` measures `<path>` using the selected rule unit
and writes the current measurement as `max_accepted`. This makes the generated
exception a ceiling, not an open-ended waiver: if the file grows later, the
finding appears again.

When `--max` is present, `--unit` is required. The unit must match every selected
rule. `--max` must be at least the selected soft or hard limit for the chosen
severity and at least the current measurement for exact-path entries.

For `--match glob`, `--max` and `--unit` are required because there is no single
file measurement to infer.

## 3. Generated Entry

The command appends one `[[exceptions]]` table to the selected registry:

```toml
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
This fixture is intentionally large because it mirrors production parser
incidents while the generator is still planned.
"""
```

If `--id` is omitted, `fissile` derives a slug from `<path>` and picks the next
unused `EX-NNN-...` ID across both registries. The entry records no date — the
commit that adds it carries that — and optional flags are omitted when absent.

If the target registry does not exist, `fissile` creates it with:

```toml
fissile_exceptions_version = 1
```

Existing registry comments and entry order are preserved. New entries append at
the end so reviews see exactly what changed.

## 4. Validation

Before writing, `fissile` validates the effective config, both exception
registries, and the new entry using §FS-003-exceptions. The command fails without
modifying files when:

- the selected rule does not exist;
- selected rules use different units;
- the generated ID already exists;
- another exception in the same severity registry already matches the same
  `(path, rule, unit)` condition;
- `--max` would make the exception invalid or smaller than the current exact-path
  measurement;
- the registry contains unrelated schema errors.

`--dry-run` prints the TOML entry that would be appended and the registry path it
would update. It does not modify the filesystem.
