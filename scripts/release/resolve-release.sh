#!/usr/bin/env bash
set -euo pipefail

# Resolve the release version, tag, and mode for release.yml (§AR-001-ci.8),
# appending version/tag/publish_crates/create_github_release/create_tag to
# $GITHUB_OUTPUT. Arguments carry the workflow event context.
event="${1:?usage: resolve-release.sh <event> <ref_type> <ref_name> <version-input> <publish-input> <release-input>}"
ref_type="${2:?}"
ref_name="${3:?}"
version_input="${4:-}"
publish_input="${5:-true}"
release_input="${6:-true}"

if [ "$ref_type" = "tag" ]; then
  tag="$ref_name"
  version="${tag#v}"
  if [ "$event" = "workflow_dispatch" ] && [ -n "$version_input" ] && [ "$version_input" != "$version" ]; then
    echo "error: workflow ref ${tag} does not match requested v${version_input}" >&2
    exit 1
  fi
else
  version="$version_input"
  tag="v${version}"
fi

manifest_ref="HEAD"
if git ls-remote --exit-code --tags origin "refs/tags/${tag}" >/dev/null 2>&1; then
  git fetch --force origin "refs/tags/${tag}:refs/tags/${tag}"
  manifest_ref="$tag"
fi
bash "$(dirname "$0")/verify-version.sh" "$version" "$manifest_ref"

if [ "$event" = "push" ] && [ "$ref_type" = "tag" ]; then
  publish_crates="true"
  create_github_release="true"
  create_tag="false"
else
  publish_crates="$publish_input"
  create_github_release="$release_input"
  if [ "$ref_type" = "tag" ]; then
    create_tag="false"
  elif [ "$publish_crates" = "true" ] || [ "$create_github_release" = "true" ]; then
    create_tag="true"
  else
    create_tag="false"
  fi
fi

out="${GITHUB_OUTPUT:-/dev/stdout}"
{
  echo "version=$version"
  echo "tag=$tag"
  echo "publish_crates=$publish_crates"
  echo "create_github_release=$create_github_release"
  echo "create_tag=$create_tag"
} >> "$out"
echo "release ${tag}: publish_crates=${publish_crates} create_github_release=${create_github_release} create_tag=${create_tag}"
