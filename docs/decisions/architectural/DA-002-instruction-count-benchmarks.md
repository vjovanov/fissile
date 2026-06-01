# DA-002-instruction-count-benchmarks: CI meters performance in instructions, not wall-clock.

`fissile` makes a hard speed promise on the hook path (§GOAL-001-fast-feedback). The
regression gate that defends it in CI counts CPU instructions with
`iai-callgrind`/Valgrind, not elapsed time.

## 1. Decision

The benchmark job (§AR-002-instruction-benchmarks, run by §AR-001-ci.5) measures the
hot library paths in instructions and fails a pull request that regresses beyond a
fixed percentage against the base branch. Wall-clock is still guarded, but only as a
coarse catastrophe net: a release-mode 10k-file smoke test under a generous timeout
(§AR-001-ci.4) that catches accidental quadratic blow-ups without pretending to
measure small deltas.

## 2. Why

Shared CI runners have noisy, contended CPUs; wall-clock variance there is large
enough to swamp the single-digit-percent regressions worth catching. A precise gate
on a noisy signal either flaps (false failures erode trust until the gate is muted)
or is set so loose it catches nothing. Instruction counts are deterministic for a
given binary and input, so the same code yields the same number run to run and
machine to machine. That makes a tight threshold meaningful: a real change in work
done shows up; scheduler noise does not.

The cost — Valgrind is slow and Linux-centric — is acceptable because the gate runs
once per PR on one platform, while correctness tests still run across the full OS
matrix (§AR-001-ci).

## 3. Consequences

- The benchmark job is Linux-only and installs Valgrind plus the
  `iai-callgrind-runner`; the cross-platform matrix does not depend on it.
- Absolute timing targets (the sub-50 ms hook, sub-500 ms 10k-file scan in
  §GOAL-001-fast-feedback.1) stay *human-verified* on a real machine; CI does not
  assert them in milliseconds. The instruction gate is the proxy that a change did
  not quietly add work, and the smoke timeout is the proxy that nothing went
  catastrophically wrong.
- A deliberate, justified instruction increase requires moving the saved baseline —
  a visible, reviewable action rather than a silent drift.
