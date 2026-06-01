# e2e

End-to-end scenarios live here. Each subdirectory under `cases/` named `E2E-*` is
one scenario and is both a grund declaration (`E2E-NNN-slug.md`) and an
executable fixture driven by `tests/e2e.rs`.

A case directory holds:

- `E2E-NNN-slug.md` — the scenario declaration, citing the behavior it verifies;
- `case.toml` — the manifest: `args`, expected `exit`, optional `git`, and
  stdout/stderr/`creates` assertions;
- `repo/` — the working tree copied into a throwaway directory before the run
  (omitted when the scenario starts from an empty repo, like `init`).

The harness drives the real `fissile` binary, so every documented behavior under
`docs/functional-spec/` has at least one executable scenario.

## Scenarios

- §E2E-001-check-clean — a clean check prints `ok` and exits zero.
- §E2E-002-check-hard-blocks — a hard overflow fails the commit with a named fix.
- §E2E-003-check-soft-warns — a soft overflow warns without blocking.
- §E2E-004-check-json — the JSON surface is one flat record per finding.
- §E2E-005-exception-silences-hard — a hard exception accepts the file; the soft warning survives.
- §E2E-006-audit-inventory — audit reports overflows plus optional inventory sections.
- §E2E-007-init-installs-hook — init bootstraps config, registries, agent block, and the hook.
- §E2E-008-exception-add — exception add appends a structured registry entry.
