# fissile

`fissile` is a Rust library for keeping repository files small on every commit.
It checks measured files against configured size budgets and returns structured
overflow findings with short, architecture-aware messages that a project can
customize per rule.

The crate is currently the library core. A CLI can build on top of it to provide
pre-commit and audit workflows.

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
