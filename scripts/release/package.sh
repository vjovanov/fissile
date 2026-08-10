#!/usr/bin/env bash
set -euo pipefail

# Self-check, size-guard, and package one release binary (§AR-001-ci.8).
#
# The binary must answer `fissile --version` with the version being released
# (§FS-006-cli.3) and fit the size ceiling (§AR-001-ci.7,
# §GOAL-002-tiny-footprint.3). The result is dist/<base> archived next to a
# .sha256, with `archive=<name>` appended to $GITHUB_OUTPUT.
version="${1:?usage: package.sh <version> [<target-triple>]}"
target="${2:-$(rustc -vV | awk '/^host:/ { print $2 }')}"

bin="./target/release/fissile"
case "$target" in
  *windows*) bin="${bin}.exe" ;;
esac

out="$("$bin" --version)"
if [ "$out" != "fissile $version" ]; then
  echo "error: binary answered '$out', expected 'fissile $version' (§FS-006-cli.3)" >&2
  exit 1
fi

case "$target" in
  *windows*) ;;
  *) strip "$bin" ;;
esac
max_bytes=$((8 * 1024 * 1024))
size=$(wc -c < "$bin")
echo "fissile (${target}): ${size} bytes (ceiling ${max_bytes})"
if [ "$size" -gt "$max_bytes" ]; then
  echo "::error::fissile binary ${size} bytes exceeds ${max_bytes}-byte ceiling" >&2
  exit 1
fi

base="fissile-${version}-${target}"
dir="dist/${base}"
mkdir -p "$dir"
cp "$bin" "$dir/"
cp README.md LICENSE "$dir/"

case "$target" in
  *windows*)
    archive="${base}.zip"
    powershell -NoProfile -Command \
      "Compress-Archive -Path 'dist/${base}' -DestinationPath '${archive}' -Force"
    ;;
  *)
    archive="${base}.tar.gz"
    tar -czf "$archive" -C dist "$base"
    ;;
esac

if command -v sha256sum >/dev/null 2>&1; then
  sha256sum "$archive" > "${archive}.sha256"
else
  shasum -a 256 "$archive" > "${archive}.sha256"
fi

echo "archive=$archive" >> "${GITHUB_OUTPUT:-/dev/stdout}"
