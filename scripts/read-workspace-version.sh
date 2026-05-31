#!/usr/bin/env bash
# Print the release version from the repo-root version file.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION_FILE="${1:-$ROOT/version}"

if [[ ! -f "$VERSION_FILE" ]]; then
  echo "version file not found: $VERSION_FILE" >&2
  exit 1
fi

tr -d '[:space:]' <"$VERSION_FILE"
