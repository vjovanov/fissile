#!/usr/bin/env bash
set -euo pipefail

# Profile-guided-optimization build for the current fissile crate.
#
# Training has two parts, both feeding the same profile (§AR-001-ci.6): the
# release test workload, plus the `fissile` CLI hot commands (`check`, `audit`)
# run over this repository so the profile reflects the commit-time path that
# §GOAL-001-fast-feedback is ordered around. Both run instrumented, then the
# merged profile drives a final profile-use rebuild of the release artifacts.

cd "$(dirname "$0")/.."
repo="$PWD"
pgo_dir="$repo/target/pgo-data"
profdata="$pgo_dir/merged.profdata"
host="$(rustc -vV | awk '/^host:/ { print $2 }')"

rustc_path() {
  local path="$1"
  if [[ "$host" == *windows* ]] && command -v cygpath >/dev/null 2>&1; then
    cygpath -m "$path"
  else
    printf '%s\n' "$path"
  fi
}

llvm_profdata="$(find "$(rustc --print sysroot)" -type f -name 'llvm-profdata*' | head -n1)"
if [ -z "$llvm_profdata" ]; then
  echo "error: llvm-profdata not found - run: rustup component add llvm-tools-preview" >&2
  exit 1
fi

rm -rf "$pgo_dir"
mkdir -p "$pgo_dir"

pgo_dir_rustc="$(rustc_path "$pgo_dir")"
profdata_rustc="$(rustc_path "$profdata")"

echo "==> 1/3  run release tests with instrumentation (-Cprofile-generate)"
gen_flags="${RUSTFLAGS:+$RUSTFLAGS }-Cprofile-generate=$pgo_dir_rustc"
RUSTFLAGS="$gen_flags" \
  cargo test --release --locked --workspace --all-targets

echo "==> 1b/3 train the CLI hot commands on this repository"
RUSTFLAGS="$gen_flags" \
  cargo build --release --locked --bin fissile
fissile_bin="$repo/target/release/fissile"
# `check`/`audit` exit non-zero when this repo has overflows; that is expected
# training output, not a build failure, so the runs are guarded.
"$fissile_bin" audit --no-color >/dev/null 2>&1 || true
"$fissile_bin" audit --no-color --top 20 --rule-coverage --stale-exceptions >/dev/null 2>&1 || true
"$fissile_bin" audit --format json >/dev/null 2>&1 || true
"$fissile_bin" check --no-color src/*.rs >/dev/null 2>&1 || true
"$fissile_bin" check --format json src/*.rs >/dev/null 2>&1 || true

shopt -s nullglob
profraws=("$pgo_dir"/*.profraw)
if [ ${#profraws[@]} -eq 0 ]; then
  echo "error: PGO training produced no .profraw files in $pgo_dir" >&2
  exit 1
fi

echo "==> 2/3  merge profiles"
"$llvm_profdata" merge -o "$profdata" "${profraws[@]}"

echo "==> 3/3  rebuild release artifacts with profile use"
RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-Cprofile-use=$profdata_rustc" \
  cargo build --release --locked --workspace --all-targets

echo "==> done: PGO release artifacts rebuilt under target/release"
