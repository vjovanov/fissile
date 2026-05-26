// §AR-002-instruction-benchmarks: instruction-counting benches cover the hot library paths.
//
// The real benchmark body is gated behind the `bench` feature. Normal
// `cargo test --all-targets` and `cargo build --all-targets` compile this no-op
// target without pulling in Valgrind-only benchmark machinery.

#[cfg(feature = "bench")]
use fissile::{
    Budget, Checker, MessageTemplate, Rule, Selector, Unit, measure_bytes, measure_text,
};
#[cfg(feature = "bench")]
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
#[cfg(feature = "bench")]
use std::hint::black_box;

#[cfg(feature = "bench")]
fn rust_checker() -> Checker {
    Checker::new(vec![Rule::new(
        "rust-modules",
        Selector::Extension("rs".to_owned()),
        Budget::new(Unit::Lines, Some(200), Some(400)),
        MessageTemplate::new(
            "split-rust-module",
            "Move cohesive helpers from {path} into the nearest owned module.",
        )
        .with_architecture_ref("§GOAL-008-architecture-aware-messages")
        .with_action("split a helper or extract a submodule"),
    )])
    .expect("benchmark checker is valid")
}

#[cfg(feature = "bench")]
fn small_source() -> String {
    "fn helper() {}\n".repeat(250)
}

#[cfg(feature = "bench")]
fn large_batch() -> Vec<fissile::FileMeasurement> {
    (0..10_000)
        .map(|index| {
            let line_count = if index % 10 == 0 { 450 } else { 40 };
            let text = "fn helper() {}\n".repeat(line_count);
            measure_text(format!("src/module_{index:05}.rs"), &text)
        })
        .collect()
}

#[cfg(feature = "bench")]
#[library_benchmark]
#[bench::small_text(setup = small_source)]
fn measure_small_text(text: String) -> fissile::FileMeasurement {
    measure_text(black_box("src/lib.rs"), black_box(&text))
}

#[cfg(feature = "bench")]
#[library_benchmark]
fn check_single_overflow() -> usize {
    let checker = rust_checker();
    let file = measure_text("src/lib.rs", &"fn helper() {}\n".repeat(450));
    checker
        .check(black_box(&file))
        .expect("check succeeds")
        .len()
}

#[cfg(feature = "bench")]
#[library_benchmark]
#[bench::ten_thousand_files(setup = large_batch)]
fn check_large_batch(files: Vec<fissile::FileMeasurement>) -> usize {
    let checker = rust_checker();
    files
        .iter()
        .map(|file| {
            checker
                .check(black_box(file))
                .expect("check succeeds")
                .len()
        })
        .sum()
}

#[cfg(feature = "bench")]
#[library_benchmark]
fn measure_binary_bytes() -> fissile::FileMeasurement {
    let bytes = vec![0_u8; 1024 * 1024];
    measure_bytes(black_box("fixtures/blob.bin"), black_box(&bytes))
}

#[cfg(feature = "bench")]
library_benchmark_group!(
    name = core;
    benchmarks = measure_small_text, check_single_overflow, check_large_batch, measure_binary_bytes
);

#[cfg(feature = "bench")]
main!(library_benchmark_groups = core);

#[cfg(not(feature = "bench"))]
fn main() {}
