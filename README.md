# fissile

**A pre-commit guard that keeps source files small enough to stay cheap for
agents to read — without ever rewriting your code.**

Large files are an invisible tax on every AI-assisted workflow. A 4,000-line
module gets dragged into context whenever an agent needs one function inside it,
and you pay for that in tokens, latency, and attention — on every run, forever.
Reviewers see diffs, not totals, so a file that crossed a sensible size years
ago keeps gaining a line at a time and nothing pushes back at the moment the
bloat is introduced (§GND-001-fissile).

`fissile` is that feedback loop. At commit time it measures staged files against
per-repo, per-file-type budgets and flags the ones that have outgrown them. It
**only measures and reports** — it never edits your code, so *how* to split a
flagged file is always your call. Each overflow can carry a short, project-owned
message that names the local split: the destination module, the owner, the
extraction pattern.

It knows the difference between file types, because that is the whole point: a
2,000-line generated SQL dump is fine, a 2,000-line hand-written module is not,
and a checked-in PNG is usually a mistake.

## Why not an existing tool?

| Tool | What it protects |
| --- | --- |
| large-file hooks | the repo from accidental blobs |
| linter line rules | one language's local style |
| bundle-size checks | shipped artifacts |
| PR-size gates | reviewer throughput |
| **fissile** | **the source layout itself — its cost to read** |

The difference is the message. A generic "file too large" stops a bad commit;
`fissile` also names the architectural move that makes the next commit better.

## Install

```sh
cargo install --git https://github.com/vjovanov/fissile
```

One small static binary, no runtime, no Python/Node toolchain. Prebuilt
per-platform binaries are on the roadmap.

## Quickstart

```sh
fissile init            # writes .agents/fissile.toml, AGENTS.md, and the git hook
```

```text
wrote ./.agents/fissile.toml
wrote ./AGENTS.md
wrote ./.git/hooks/pre-commit
next:
1. Review .agents/fissile.toml: the source rule budgets common code extensions; add this repo's languages or tune the limits.
2. Commit a change to see the pre-commit hook run fissile check --staged.
3. Run fissile audit once and add justified exceptions with fissile exception add.
see AGENTS.md for the full workflow.
```

That is it. The installed hook runs `fissile check --staged` on every commit.
The starter config ships sensible defaults — a byte budget on everything, a
line budget on common source extensions (`.rs`, `.go`, `.py`, `.ts`, `.js`, …)
wherever they live, and a markdown budget — all editable in place.

## The two tiers

Every rule carries two limits, because one threshold is a false economy:

- **soft** — *warns, exit 0.* The signal for the agent that just grew the file:
  shrink it the way the message says, before claiming the task done.
- **hard** — *blocks, exit 1.* No override flag, no `# fissile: allow` comment.
  The only way past is a justified exception (below).

## Example output

A hard overflow fails the commit:

```text
$ fissile check src/orders.rs
src/orders.rs: 620 lines > 550 lines [hard, rule: source, message: split-source]
  Split src/orders.rs: move cohesive helpers into a sibling module before adding more code (§GOAL-008-remediation-messages).
# exit 1
```

A soft overflow warns but lets the commit through:

```text
$ fissile check src/util.rs
src/util.rs: 410 lines > 350 lines [soft, rule: source, message: split-source]
  Split src/util.rs: move cohesive helpers into a sibling module before adding more code (§GOAL-008-remediation-messages).
# exit 0
```

Every finding leads with the path (editors and agents jump straight to it),
states the measured size against the limit, and names the rule and message that
own the budget. A passing run prints exactly `ok`.

`--format json` is the agent surface — one flat record per finding:

```json
[{"path":"src/orders.rs","unit":"lines","actual":620,"limit":550,"severity":"hard","rule_id":"source","message_id":"split-source","message":"Split src/orders.rs: move cohesive helpers into a sibling module before adding more code (§GOAL-008-remediation-messages)."}]
```

The schema is published under `schema/` and validated against emitted output.

## Audit an existing repo

Adopting against a large codebase? `audit` inventories the whole repo without
blocking anyone, so you can see the surface before turning the hook on:

```text
$ fissile audit --top 5
src/orders.rs: 620 lines > 550 lines [hard, rule: source, message: split-source]
  Split src/orders.rs: move cohesive helpers into a sibling module before adding more code (§GOAL-008-remediation-messages).
src/util.rs: 410 lines > 350 lines [soft, rule: source, message: split-source]
  Split src/util.rs: move cohesive helpers into a sibling module before adding more code (§GOAL-008-remediation-messages).

top lines:
  620 src/orders.rs
  410 src/util.rs
```

Add `--stale-exceptions` to find exceptions whose file is gone, or
`--rule-coverage` to find rules and messages that match nothing.

## Justified exceptions

A file you have decided to keep large gets a written reason in a registry — not
a silent ignore comment. `exception add` appends the entry for you:

```sh
fissile exception add src/orders.rs --severity hard --rule source \
  --reason "legacy order engine; splitting tracked in #142" --until "#142 lands"
```

```toml
[[exceptions]]
id = "EX-001-orders-rs"
path = "src/orders.rs"
match = "exact"
rules = ["source"]
max_accepted = { value = 620, unit = "lines" }
until = "#142 lands"
reason = """
legacy order engine; splitting tracked in #142
"""
```

The hard block is now silenced — but only up to `max_accepted`. Grow the file
past it and the finding returns. The soft warning still nudges the agent.

## Use as a library

```rust
use fissile::{Budget, Checker, MessageTemplate, Rule, Selector, Unit, measure_text};

let checker = Checker::new(vec![Rule::new(
    "rust-modules",
    Selector::Extension("rs".into()),
    Budget::new(Unit::Lines, Some(200), Some(400)),
    MessageTemplate::new(
        "split-rust-module",
        "Move cohesive helpers from {path} into the nearest owned module.",
    ),
)])?;

let file = measure_text("src/lib.rs", "fn main() {}\n");
assert!(checker.check(&file)?.is_empty());
# Ok::<(), fissile::FissileError>(())
```

## Configuration

A single versioned TOML file at `.agents/fissile.toml` — data, not a plugin
surface. Budgets are set per extension, per glob, and per unit (`bytes`,
`lines`, or `tokens`); each rule names a message template. Full schema in
[`docs/functional-spec/FS-001-config.md`](docs/functional-spec/FS-001-config.md),
with a worked sample at [`examples/fissile.toml`](examples/fissile.toml).

## How it fits together

- **`init`** — config, exception registries, the managed `AGENTS.md` block, and
  the git hook (§FS-002-init).
- **`check`** — the commit-time gate over staged files or explicit paths
  (§FS-004-check-audit).
- **`audit`** — the whole-repo inventory and migration surface
  (§FS-004-check-audit).
- **`exception add`** — append a justified oversized-file exception
  (§FS-005-exception-add).

This repo is grounded with [`grund`](https://github.com/vjovanov/grund): the
`§ID` markers above point at the specs and goals that justify each behavior.
