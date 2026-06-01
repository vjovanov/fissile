# Roadmap

How `fissile` gets from a working library core to a tool a team can drop into
any repo. Milestones are declared inline; each is a stable ID and may be cited
from anywhere in the repo. A milestone is "done" only when the goals it cites can
point at the evidence named in their **Measurable** sections — not when the code
merely exists.

Milestones:

- [§RM-001-core](roadmap.md#rm-001-core-the-library-and-cli-do-the-job) — shipped
- [§RM-002-trustworthy](roadmap.md#rm-002-trustworthy-the-specs-are-backed-by-evidence) — shipped
- [§RM-003-frictionless-adoption](roadmap.md#rm-003-frictionless-adoption-one-step-into-any-repo) — in progress
- [§RM-004-scale-and-reach](roadmap.md#rm-004-scale-and-reach-big-repos-more-surfaces)

## RM-001-core: the library and CLI do the job

The measuring engine and its four command surfaces exist and are covered by unit
and library-level integration tests.

- Config loader with per-extension, per-glob, and per-unit budgets, a built-in
  default, and a fully-populated starter document (§FS-001-config, §FS-002-init).
- `check` (staged / explicit paths / scan scope), `audit` (overflows, `--top`,
  `--stale-exceptions`, `--rule-coverage`), and `exception add` for both
  registries (§FS-004-check-audit, §FS-005-exception-add, §FS-003-exceptions).
- Text and JSON output with the byte-stable finding shape (§GOAL-003-friendly-output,
  §GOAL-004-token-thrift).
- CI on three platforms with an instruction-count regression gate and a perf
  smoke guard (§AR-001-ci, §AR-002-instruction-benchmarks).

This milestone is shipped.

## RM-002-trustworthy: the specs are backed by evidence

Every goal's **Measurable** section is written as if the executable scenarios
already exist. This milestone makes that true, so "the spec says so" and "a test
proves it" stop diverging.

- A fixture-driven e2e harness that drives the real `fissile` binary, with at
  least one case per documented behavior under `docs/functional-spec/`
  (§E2E-001-check-clean and siblings under `e2e/cases`).
- A published, committed JSON schema for the `check`/`audit` record shape, with a
  test that validates emitted output against it (§GOAL-003-friendly-output.1,
  §GOAL-004-token-thrift.1).
- A test that enforces the one-screen `--help` bound (§GOAL-003-friendly-output.3).
- Architectural decisions that goals lean on captured as `§DA-` records rather
  than argued only inside the goals (§DA-001-token-external-command and siblings).

This milestone is shipped: the e2e harness drives the real binary with a case
per documented behavior, the `check`/`audit` JSON schema is published under
`schema/` and validated against emitted output, the one-screen `--help` bound is
enforced by a test, and the leaned-on decisions are captured as `§DA-` records.

## RM-003-frictionless-adoption: one step into any repo

The headline use case is a pre-commit hook. Adoption should not require the
contributor to hand-wire git plumbing.

- `fissile init` installs and manages the pre-commit hook that runs
  `fissile check --staged`, idempotently and reversibly, like every other
  managed block it writes (§FS-002-init.6).
- A binary-size guard in CI that fails a regression past a documented threshold,
  closing the loop on the footprint promise (§GOAL-002-tiny-footprint.3).
- Token-mode end to end against an external counter, documented as opt-in so the
  default binary stays small (§DA-001-token-external-command, §GOAL-005-configurable).

This is the current focus. The managed pre-commit hook ships and is covered by
§E2E-007-init-installs-hook; the binary-size guard runs in the pre-release
workflow (§AR-001-ci.7). Prebuilt per-platform binaries (§RM-004-scale-and-reach)
remain the open step before adoption needs no Rust toolchain.

## RM-004-scale-and-reach: big repos, more surfaces

Once the contract is trustworthy and adoption is one step, widen reach.

- Parallel scan via `rayon` once the single-threaded walk stops winning on real
  repos, held to the whole-repo budget (§GOAL-001-fast-feedback.1).
- Prebuilt per-platform binaries and a `cargo-binstall` / install-script path so
  adoption needs no Rust toolchain (§GOAL-002-tiny-footprint.1).
- A thin GitHub Action wrapper so the same check runs in CI without bespoke
  glue, reusing the existing JSON surface.
