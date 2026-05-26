# AR-002-instruction-benchmarks: instruction-counting benchmarks for hot library paths

`fissile` uses the same performance measurement shape as `grund`: an
instruction-counting `cargo bench` harness under Callgrind. Wall-clock targets
in §GOAL-001-fast-feedback remain the product budget, but instruction counts are
the CI regression meter because they are stable on shared runners.

## 1. What is benched

The benchmark harness lives at `benches/instructions.rs` and is gated behind the
`bench` Cargo feature. It measures the hot library paths that a pre-commit CLI
or embedding application will run repeatedly:

- measuring source text into byte and line counts;
- checking one file that crosses a hard rule;
- checking a synthetic 10k-file batch with deterministic overflow density;
- measuring a binary byte buffer without line or token work.

The regular build and test matrix compiles the bench target without the feature,
which keeps Valgrind and `iai-callgrind` out of normal development feedback.

## 2. Why instruction counts

Callgrind reports CPU instructions for the same binary and same input. That
number does not depend on runner load, scheduling, or neighboring CI jobs, so it
can fail a pull request on a real regression without becoming a flaky stopwatch.
The wall-clock smoke guard in §AR-001-ci.4 remains as the catastrophic blow-up
backstop; the instruction-count harness is the precise meter.

## 3. How it runs

Locally, run:

```sh
cargo bench --features bench --bench instructions
```

This requires Valgrind and `iai-callgrind-runner` on `PATH`. In CI, the benchmark
job installs those tools, records a base-branch baseline for pull requests, and
fails the pull request if instruction count grows by more than 5%.

## 4. Relationship to goals

This harness is the measurable part of §GOAL-001-fast-feedback. The goal names
the performance budgets and the need to catch regressions; this architecture
document pins the measurement tool, the benchmark inputs, and the CI gate.
