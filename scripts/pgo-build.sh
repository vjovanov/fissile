#!/usr/bin/env bash
set -euo pipefail

# Profile-guided-optimization build for the current fissile crate.
#
# fissile is currently a library core, not a CLI binary. This script therefore
# trains on the release test workload and rebuilds the release library artifact
# with the merged profile. When a CLI target lands, extend the training loop to
# run the hot subcommands before the final rebuild.

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
RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-Cprofile-generate=$pgo_dir_rustc" \
  cargo test --release --locked --workspace --all-targets

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
