# integration

Integration tests prove the How: that the parts fit as designed. This is the
home of the non-citable `integration` kind — none of these files carries an
ID, and `[citations.integration]` in `grund.toml` says a test here should cite
the `§AR` point whose structure it exercises. A test belongs here when its
subject spans more than one part of fissile rather than a single module; a
claim about one function is a unit test beside it, and black-box proof of a
spec point is an `E2E` case under `tests/e2e/`.

fissile is a single published package, not a workspace, so each file here is
wired into `Cargo.toml` with its own `[[test]]` block (Cargo does not
auto-discover `.rs` files nested under `tests/`) rather than being a
workspace member the way grund's own integration suite is.

- `commands.rs` — the library command surfaces `check`, `audit`, and
  `exception add` against the real config and registry types
  (§FS-004-check-audit, §FS-005-exception-add).
- `json_schema.rs` — the published JSON schema and the bytes `fissile`
  actually emits stay in lockstep; a new or renamed field that is not
  reflected in `schema/` fails here (§GOAL-003-friendly-output.1,
  §GOAL-004-token-thrift.1).
- `agent_loop.rs` — the agent-minimize loop end to end: a soft warning names a
  byte-stable finding shape, and shrinking the file it named clears the
  warning on the next run (§GOAL-006-graded-limits.2,
  §GOAL-006-graded-limits.4).
- `config_fixtures.rs` — the checked-in config fixtures load under the real
  schema (§FS-001-config, §DF-002-explicit-config).
- `help.rs` — `--help` stays a one-screen surface and every subcommand carries
  an example (§GOAL-003-friendly-output.3).
