#!/usr/bin/env bash
# Print the highest released semver (no v prefix), or empty if no v* tags exist.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

latest_tag="$(git tag -l 'v*' --sort=-v:refname | head -1 || true)"
if [[ -z "$latest_tag" ]]; then
  exit 0
fi
echo "${latest_tag#v}"
