# FS-001-config: fissile reads a versioned TOML config file

`fissile` is configured by a single TOML document. The default discovery path is
`.agents/fissile.toml`; an embedding CLI may also pass an explicit path. The
config is data, not code, so it can be read inside a pre-commit hook without
invoking a plugin system (§GOAL-002-tiny-footprint) while still making limits and
messages project-owned (§GOAL-005-configurable).

The concrete example shape is maintained in `examples/fissile.toml`.

## 0. Built-in Defaults

A repo with no config still gets a useful guard:

- scan the whole repository, honoring `exclude` and `.gitignore`, so adoption
  needs no guess about which layout a repo uses;
- exclude lockfiles, minified files, vendored directories, build output, VCS
  metadata, and common binary/media assets;
- apply a conservative byte budget to every non-excluded file;
- apply a line budget to common hand-written source extensions and to markdown,
  wherever those files live, while leaving data and generated formats to the
  byte budget (§GND-001-fissile);
- use generic messages that explain how to tune config rather than pretending to
  know the repository's architecture.

These defaults borrow the useful part of generic large-file hooks and platform
file-size guidance: catch obvious accidents immediately. They are not the product
identity. `fissile` earns its keep when projects replace the generic defaults
with named, project-specific rules and messages that can speak to local
architecture.

A hand-written config may omit any field and take its default; an omitted field
is not an error. The config that `fissile init` *generates*, however, is fully
populated — every field is written out at its default so the file is editable
without consulting this spec (§DF-002-explicit-config).

## 1. Top-level version

Every config starts with:

```toml
fissile_config_version = 1
```

Unknown major versions are a schema error. Version 1 is additive: unknown keys
inside known tables are errors, so a typo cannot silently disable a rule.

## 2. Scan scope

`[scan]` controls whole-repo audit traversal:

- `include`: root directories or globs walked by `audit`;
- `exclude`: globs ignored before measurement;
- `respect_gitignore`: whether repository ignore files participate in traversal,
  default `true`.

Pre-commit checks receive their file set from git and do not use `include`, but
they still apply `exclude` so generated assets and lockfiles stay out of the
budget system.

## 3. Rules

Rules are declared as `[[rules]]` entries. Each rule has:

- `id`: stable machine-readable rule name;
- `include`: one or more globs;
- `unit`: `bytes`, `lines`, or `tokens`;
- `soft`: optional warning threshold;
- `hard`: optional blocking threshold;
- `priority`: optional integer tie-breaker, default `0`;
- `message`: the ID of a `[[messages]]` template.

At least one of `soft` or `hard` is required. If both are present, `soft <= hard`
is required. A file above the hard limit reports only the hard overflow; the
soft overflow is implied (§GOAL-006-graded-limits).

Rule IDs are user-facing names, not incidental labels. They should read like
bundle-size entries: `rust-source`, `api-docs`, `fixtures`, `generated-rust`.
Findings include the rule ID, JSON output carries it, and exceptions can target
it. A config with anonymous or auto-numbered rules is invalid.

### 3.1 Line Counting Policy

Line rules may define what counts:

- `count_blank_lines`: boolean, default `false`;
- `count_comment_lines`: boolean, default `true`.

The defaults count lines that carry content — code and comments — but ignore
blank separator lines, so readable spacing is never what pushes a file over
budget. Counting comments by default keeps documentation honest about its review
and token cost. Projects may flip either field: set `count_blank_lines = true`
to measure raw physical file size, or `count_comment_lines = false` for a
code-only budget. The policy is per rule because generated docs, tests, and
source files often need different treatment.

### 3.2 Overlapping Rules

A file may match more than one rule. Overlap is resolved independently for each
measurement unit (`bytes`, `lines`, `tokens`), because a project may reasonably
check one file by both line count and byte count. For a given `(file, unit)`,
`fissile` selects one effective rule:

1. Higher `priority` wins.
2. If priority ties, the most-specific selector wins:
   - exact path beats glob;
   - deeper or longer glob beats broader glob;
   - extension-only beats catch-all.
3. If specificity still ties, config validation fails with an ambiguity error.

Config file order is never a tie-breaker. Reordering equivalent `[[rules]]`
entries must not change whether a repository passes.

Examples:

```toml
[[rules]]
id = "docs"
include = ["docs/**/*.md"]
unit = "lines"
soft = 250
hard = 500
message = "split-doc"

[[rules]]
id = "api-docs"
include = ["docs/api/**/*.md"]
unit = "lines"
soft = 500
hard = 900
message = "split-api-doc"
```

`docs/guide.md` uses `docs`; `docs/api/openapi.md` uses `api-docs` because the
subfolder glob is more specific.

When specificity is not enough, the config must say which rule wins:

```toml
[[rules]]
id = "generated-rust"
include = ["src/**/*.gen.rs"]
unit = "lines"
soft = 1200
hard = 2000
priority = 20
message = "generated-file"

[[rules]]
id = "domain-rust"
include = ["src/domain/**/*.rs"]
unit = "lines"
soft = 350
hard = 550
message = "split-domain"
```

`src/domain/schema.gen.rs` uses `generated-rust` because its priority is higher.
Without `priority`, this overlap is ambiguous: each glob is specific in a
different dimension, and silently choosing one would make the local architecture
guidance unreliable.

### 3.3 What To Exclude Versus Except

Use `[scan].exclude` for files `fissile` should not reason about:

- vendored code;
- lockfiles;
- minified output;
- generated artifacts whose source is elsewhere;
- binary/media assets;
- build outputs and package caches.

Use the exception registries (§FS-003-exceptions) for files `fissile` should still
reason about, but that are accepted as oversized for a written reason: hand-made
fixtures, intentionally consolidated compatibility layers, generated sources
checked in for bootstrap reasons, or architectural seams that cannot yet be
split. Exclusions need no rationale because the tool does not apply. Exceptions
require rationale because the tool does apply and the repo is choosing to accept
the cost.

## 4. Messages

Messages are declared separately as `[[messages]]` entries so multiple rules can
share one remediation message. Each message has:

- `id`: stable message ID included in machine-readable findings;
- `text`: bounded template rendered for humans and agents.

A message has no separate `owner`, `destination`, or `action` fields: the
destination module, ownership boundary, and next step all live in `text`, so the
rendered guidance is a single human-readable string rather than a record the
caller must reassemble.

The supported template variables are `{path}`, `{rule}`, `{severity}`,
`{actual}`, `{limit}`, and `{unit}`. Missing message IDs are schema errors.
Grund citations are part of the message text, not a separate field, so the
rendered guidance remains the single source of human context.
Messages cannot execute code, inspect file contents, or change pass/fail
behavior (§GOAL-008-remediation-messages).

## 5. Exceptions

`[exceptions]` names the severity-specific oversized-file rationale
registries:

- `soft_registry`: TOML path for soft-limit exceptions, default
  `docs/file-size-agent-exceptions.toml`;
- `hard_registry`: TOML path for hard-limit exceptions, default
  `docs/file-size-human-exceptions.toml`;
- `stale`: `warn`, `error`, or `ignore` for entries that match no scanned file.

Soft exceptions are for agent-facing warning debt: they keep soft findings from
being repeated when the repository has deliberately accepted the current shape.
Hard exceptions are for human-reviewed blocking debt: they are the only way to
accept a hard-limit overflow without disabling the rule
(§GOAL-007-justified-exceptions). Exceptions are distinct from `[scan].exclude`,
which removes files the tool does not apply to at all. Each exception entry
records a maximum accepted measurement so the finding reappears if the file grows
again. The registry file formats are specified in §FS-003-exceptions.

## 6. Output

`[output]` sets defaults only. Invocation flags may override these values:

- `format`: `text` or `json`;
- `color`: `auto`, `always`, or `never`;
- `success`: the exact success marker for text output, default `ok`.

The machine-readable finding fields, exit-code mapping, and severity model are
not configurable (§GOAL-003-friendly-output).

## 7. Tokens

`[tokens]` is opt-in. With `enabled = false`, token rules are schema-valid but
cannot be evaluated unless the caller supplies token measurements directly. With
`enabled = true`, `command` names an external counter command. `{path}` is
substituted with the file path. The command must print one integer token count.

The default build does not bundle a tokenizer model (§GOAL-002-tiny-footprint).
