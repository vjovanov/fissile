# fissile

`fissile` does one simple thing: it helps agents keep repository files small so
they spend fewer tokens, without putting architecture or correctness at risk. It
checks measured files against configured size budgets and returns structured
overflow findings, and each finding can carry a short, project-configured
message suggesting how to split the file. It only measures and reports; it never
rewrites code, so how to split a flagged file is always the contributor's
decision (§GND-001-fissile).

The crate is both a library and a `fissile` binary. The binary currently
provides `fissile init`, which writes a fully populated starter config, the
exception registries, and a managed agent-instruction block (§FS-002-init); the
library loads and validates that config (§FS-001-config) and evaluates files
against it. The `check`, `audit`, and `exception` commands are specified but not
yet wired into the CLI.

The proposed repository config is documented in
`docs/functional-spec/FS-001-config.md`; a concrete sample lives at
`examples/fissile.toml`. The proposed `fissile init` workflow is specified in
`docs/functional-spec/FS-002-init.md`, and the exception registry formats are
specified in `docs/functional-spec/FS-003-exceptions.md`. The proposed command
for adding exceptions is specified in
`docs/functional-spec/FS-005-exception-add.md`.

```rust
use fissile::{Budget, Checker, MessageTemplate, Rule, Selector, Unit, measure_text};

fn main() -> Result<(), fissile::FissileError> {
    let checker = Checker::new(vec![Rule::new(
        "rust-modules",
        Selector::Extension("rs".into()),
        Budget::new(Unit::Lines, Some(200), Some(400)),
        MessageTemplate::new(
            "split-rust-module",
            "Move cohesive helpers from {path} into the nearest owned module (§GOAL-008-architecture-aware-messages).",
        ),
    )])?;

    let file = measure_text("src/lib.rs", "fn main() {}\n");
    let overflows = checker.check(&file)?;
    assert!(overflows.is_empty());

    Ok(())
}
```
