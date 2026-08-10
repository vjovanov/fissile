#!/usr/bin/env bash
set -euo pipefail

# Release version guard (§AR-001-ci.8): the manifest at <git-ref> and the
# pinned e2e expectation (§FS-006-cli.3) must both agree with <version> before
# anything builds.
version="${1:?usage: verify-version.sh <version> <git-ref>}"
ref="${2:?usage: verify-version.sh <version> <git-ref>}"

case "$version" in
  [0-9]*.[0-9]*.[0-9]*) ;;
  *)
    echo "error: version must look like 0.1.0, got '$version'" >&2
    exit 1
    ;;
esac

manifest_version="$(git show "${ref}:Cargo.toml" | awk '
  /^\[package\]/ { in_package = 1; next }
  /^\[/ && in_package { exit }
  in_package && /^[[:space:]]*version[[:space:]]*=/ {
    line = $0
    sub(/^[^=]*=[[:space:]]*"/, "", line)
    sub(/".*/, "", line)
    print line
    exit
  }
')"

if [ "$manifest_version" != "$version" ]; then
  echo "error: ${ref}: Cargo.toml version is ${manifest_version}, expected ${version}" >&2
  exit 1
fi

expected="stdout_equals = \"fissile ${version}\""
if ! git show "${ref}:e2e/cases/E2E-010-cli-version/case.toml" | grep -qF "$expected"; then
  echo "error: ${ref}: e2e/cases/E2E-010-cli-version/case.toml does not pin '$expected'" >&2
  exit 1
fi

echo "verified: ${ref} carries fissile ${version}"
