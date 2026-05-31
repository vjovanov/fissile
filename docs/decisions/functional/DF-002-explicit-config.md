# DF-002-explicit-config: The config is fully populated with every value, including defaults.

A `fissile` config declares every setting explicitly, even when the value equals
the built-in default. `fissile init` writes a complete config — every `[scan]`,
`[output]`, `[exceptions]`, and `[tokens]` field, and on each rule every line-counting
flag — at its default value, ready to edit in place. Omitting a field is still
legal and still falls back to the default; the tool does not *require* a full
config. But the config it generates and the configs it ships as examples are
exhaustive, not minimal (§GOAL-005-configurable).

## 1. Decision

The starter config written by `fissile init`, the example at `examples/fissile.toml`,
and `fissile`'s own `.agents/fissile.toml` all spell out every value the schema
accepts, set to its default unless the project has a reason to differ. A reader
never has to consult §FS-001-config to learn what a field defaults to or that a
field exists: the knob is already in the file, at its default, with the value
they would type anyway.

This is a deliberate inversion of the usual "omit anything equal to the default"
convention. Here, restating a default is not redundancy to be trimmed — it is the
point.

## 2. Why

A `fissile` config is **edited rarely and read under pressure**: someone tightening a
limit during review, or an agent reacting to an overflow mid-task. In that moment
the cost of a minimal config is a second tool you have to reach for — `--help`, the
spec, or memory of what the default was and whether the field even exists. A
fully-populated file removes that step entirely: every adjustable value is present,
named, and editable on the spot, with no hidden state to discover.

- **No extra skill to edit.** The complete file is self-documenting. You change a
  number; you do not first have to learn the schema (§GOAL-005-configurable).
- **Defaults are visible, not implied.** A reader sees exactly what the tool will do,
  including for fields they have never touched. Nothing is decided off-screen.
- **Rare edits favor completeness over brevity.** A config touched monthly is not
  helped by being short; it is helped by being unambiguous. The verbosity is paid
  once at generation and never again.

## 3. Consequences

- `fissile init` emits the full schema at defaults, not a minimal skeleton
  (§FS-002-init.2). Generated rules carry their line-counting flags explicitly even
  though each would otherwise take its default (§FS-001-config.3.1).
- `examples/fissile.toml` and `.agents/fissile.toml` are kept exhaustive. Settings
  that merely restate a default — `respect_gitignore`, the whole `[output]` block,
  the default registry paths, `[tokens]`, the per-rule `count_*` flags — are present
  on purpose and must not be trimmed as "superfluous."
- The fallback semantics are unchanged: a hand-written partial config is still valid
  and omitted fields still take their defaults (§FS-001-config.0). Completeness is a
  property of what `fissile` *generates*, not a requirement it *enforces*.
- A default that changes in a later version does not propagate to configs already on
  disk, because those configs pinned the value explicitly. This is acceptable and
  intended: a project-owned config should not shift behavior under the project's feet
  on upgrade.
