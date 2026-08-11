# FS-005-exception-add: fissile exception add writes structured exception entries

`fissile exception add` is the supported way to add entries to the soft and hard
exception registries. Users should not need to hand-edit registry TOML for the
common case of accepting a current overflow.

## 1. Command

```text
fissile exception add <path> --severity soft|hard --rule <id>
                      --kind structural|deferred --reason <text>
                      [--until <text>] [--config <path>] [--match exact|glob]
                      [--title <text>] [--owner <text>] [--issue <text>]
                      [--max <N> --unit bytes|lines|tokens]
                      [--dry-run]
```

`--severity` chooses the configured registry: `soft` writes to
`[exceptions].soft_registry`; `hard` writes to `[exceptions].hard_registry`.
`--rule` may be repeated to create one exception for multiple same-unit rules.

`--kind` and `--reason` are required so every accepted oversized file carries a
claim a reviewer can disagree with (§FS-003-exceptions.2.1,
§DF-004-exception-kind). The kind decides what the reason must establish and what
`--until` may say:

- `--kind structural` — the reason names the architectural constraint that makes
  splitting illegal, and what would break if the file were split anyway.
  `--until` is optional and defaults to `indefinite`; passing any other value is
  a usage error.
- `--kind deferred` — the reason names the boundary that is missing and what has
  to exist before the split is possible. `--until` is required and may not be
  `indefinite`.

Neither is answered by describing the file's contents; that is what the finding
already said. The command does not judge the prose, but the flags it requires
make the two questions impossible to conflate, and the error text names the
distinction at the moment the entry is written.

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

`kind` and `until` are always written, even for a `structural` entry that took
the `indefinite` default, so a registry entry never depends on a reader knowing
the command's defaults (§DF-002-explicit-config).

The entry gets no name of its own: it is identified by the registry it is written
to and what it accepts (§FS-003-exceptions.2.2, §DF-005-exception-identity), and
the command never writes the removed `id` or `replaces` keys. The entry records
no date — the commit that adds it carries that — and optional flags are omitted
when absent.

If the target registry does not exist, `fissile` creates it with:

```toml
fissile_exceptions_version = 2
```

Existing registry comments and entry order are preserved. New entries append at
the end so reviews see exactly what changed.

## 4. Validation

Before writing, `fissile` validates the effective config, both exception
registries, and the new entry using §FS-003-exceptions. The command fails without
modifying files when:

- the selected rule does not exist;
- selected rules use different units;
- `--kind` is absent, or `--until` disagrees with it (§1);
- another exception in the same severity registry already matches the same
  `(path, rule, unit)` condition — the rejection names that registry and the
  `path` of the entry already accepting it, which is the entry to edit instead;
- `--max` would make the exception invalid or smaller than the current exact-path
  measurement;
- the registry contains unrelated schema errors.

`--dry-run` prints the TOML entry that would be appended and the registry path it
would update. It does not modify the filesystem.
