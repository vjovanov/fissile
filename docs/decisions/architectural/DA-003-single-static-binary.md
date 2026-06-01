# DA-003-single-static-binary: hand-rolled CLI and JSON, no plugin surface.

A tool that nags repos to stay small must stay small itself (§GOAL-002-tiny-footprint).
`fissile` keeps its dependency tree auditable on one screen by writing the few pieces
that would otherwise pull large crates by hand, and by refusing an extension surface.

## 1. Decision

- **Argument parsing is hand-rolled** in `main.rs` rather than delegated to a
  full-featured CLI crate, so the binary carries no derive-macro or help-generation
  framework it does not need.
- **JSON is emitted by a minimal in-tree writer** that models only the value shapes
  `fissile` produces (§GOAL-002-tiny-footprint), so no reflective JSON dependency is
  linked in.
- **There is no plugin or scripting surface.** Custom behavior is expressed as config
  data — rules, globs, units, message templates — not as code loaded into the
  process (§GOAL-005-configurable, §GOAL-002-tiny-footprint.2). A project needing
  custom logic shells out to its own binary from its own pre-commit config, not from
  inside `fissile`.

## 2. Why

Each avoided dependency is weight a contributor pays in every repo that adopts the
tool, and surface a reviewer has to audit. CLI and JSON crates are convenient but
broad; the slices `fissile` actually uses are small and stable, so owning them costs
little and buys a dependency list that fits the audit promise
(§GOAL-002-tiny-footprint.3). Refusing a plugin surface is the same trade as
configuration-is-data: extensibility via embedded code would mean a runtime, a
sandbox question, and an unbounded dependency story — exactly what the footprint goal
rules out (§GOAL-002-tiny-footprint.2).

## 3. Consequences

- The crate owns a little code it could have imported: argument dispatch and a JSON
  writer. That code is covered by tests and is the price of a short `Cargo.lock`.
- Output shape and CLI ergonomics are constrained to what is cheap to maintain by
  hand. That pressure aligns with the output goals anyway: one compact record per
  finding, a one-screen help, no decorative copy (§GOAL-003-friendly-output,
  §GOAL-004-token-thrift).
- Project-specific logic lives outside `fissile`. This is a feature, not a gap: the
  tool stays one measuring binary, and orchestration belongs to the pre-commit
  config that already sequences a repo's hooks.
- The dependency count is reviewed at release time; a change that adds a heavy
  dependency must justify itself against this record.
