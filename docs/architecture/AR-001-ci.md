# AR-001-ci: CI mirrors the local quality gate and records performance

The CI workflow is the remote form of the local development gate. Anything that
can make the crate unbuildable, untestable, unformatted, ungrounded, or too slow
must fail in CI, so contributors cannot bypass the checks by skipping local hooks
or editing through a web UI. This supports §GOAL-001-fast-feedback and
§GOAL-003-friendly-output.

## 1. Matrix

The Rust build and test matrix runs on Linux, macOS, and Windows. Each leg
installs stable Rust with `rustfmt` and `clippy`, restores Cargo caches on a
best-effort basis, checks formatting, builds all targets with warnings denied,
runs all tests, and runs clippy with warnings denied.

Cache failures are not build failures. A cold cache must still reach the actual
format/build/test/lint steps and let those steps decide pass or fail.

## 2. Grounding

The Linux leg runs `grund check` so docs, source citations, and architecture
references stay valid. This is separate from Rust compilation: `cargo` proves the
crate builds, while `grund` proves the project explanation still resolves.

## 3. Packaging

The Linux leg runs `cargo package --locked --list` as a cheap packaging sanity
check. It verifies that Cargo can assemble the crate contents under the locked
dependency graph without doing a publish.

## 4. Performance smoke guard

CI carries a cheap wall-clock backstop for §GOAL-001-fast-feedback: the matrix
runs the `large_batch_smoke` release test under a generous 30 second timeout. The
budget itself is much tighter than that; this guard is for catastrophic
regressions such as an accidental quadratic path or a repeated scan over every
file. The precise per-commit meter is the benchmark job in §5.

## 5. Benchmark job

A separate Linux-only `bench` job runs the instruction-counting harness
(§AR-002-instruction-benchmarks). The job installs Valgrind and the
`iai-callgrind-runner` version that matches the crate dependency. On pull
requests it first records a base-branch baseline, then reruns the pull request
with `--callgrind-limits=ir=5.0%`; instruction-count growth beyond that limit is
a build failure. Pushes to `main` record current counts and upload the JSON
summaries for inspection.

The benchmark body is gated behind the `bench` Cargo feature, so ordinary build
and test jobs compile a no-op bench target and never require Valgrind.
