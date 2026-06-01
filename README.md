# fissile

`fissile` does one simple thing: it steers agents toward smaller repository
files so they spend fewer tokens, while respecting the architecture. It
checks measured files against configured size budgets and returns structured
overflow findings, and each finding can carry a short, project-configured
message suggesting how to split the file. It only measures and reports; it never
rewrites code, so how to split a flagged file is always the contributor's
decision (§GND-001-fissile).

The crate is both a library and a `fissile` binary. The binary provides:

- `fissile init`, which writes a fully populated starter config, optional
  exception registries, and managed agent-instruction blocks (§FS-002-init);
- `fissile check`, the commit-time gate over staged files, explicit paths, or
  the configured scan scope (§FS-004-check-audit);
- `fissile audit`, the whole-repo inventory and migration surface
  (§FS-004-check-audit);
- `fissile exception add`, the supported way to append justified oversized-file
  exceptions (§FS-005-exception-add).

The repository config is documented in `docs/functional-spec/FS-001-config.md`;
a concrete sample lives at `examples/fissile.toml`. The exception registry
format is specified in `docs/functional-spec/FS-003-exceptions.md`.

Common CLI flows:

```text
fissile init --exceptions
fissile check --staged
fissile audit --top 10 --stale-exceptions
fissile exception add src/big.rs --severity hard --rule rust-source \
  --reason "accepted while splitting" --until "tracked split lands"
```

```rust
use fissile::{Budget, Checker, MessageTemplate, Rule, Selector, Unit, measure_text};

fn main() -> Result<(), fissile::FissileError> {
    let checker = Checker::new(vec![Rule::new(
        "rust-modules",
        Selector::Extension("rs".into()),
        Budget::new(Unit::Lines, Some(200), Some(400)),
        MessageTemplate::new(
            "split-rust-module",
            "Move cohesive helpers from {path} into the nearest owned module (§GOAL-008-remediation-messages).",
        ),
    )])?;

    let file = measure_text("src/lib.rs", "fn main() {}\n");
    let overflows = checker.check(&file)?;
    assert!(overflows.is_empty());

    Ok(())
}
```
