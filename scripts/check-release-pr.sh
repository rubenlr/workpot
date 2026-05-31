#!/usr/bin/env bash
# Validate release metadata on PRs that bump version or CHANGELOG.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

BASE_REF="${GITHUB_BASE_REF:-master}"

if ! git diff --name-only "origin/${BASE_REF}...HEAD" | grep -qxE 'version|CHANGELOG.md'; then
  echo "release-check: skip (no version or CHANGELOG.md changes)"
  exit 0
fi

HEAD_VERSION="$(bash scripts/read-workspace-version.sh)"
if [[ -z "$HEAD_VERSION" ]]; then
  echo "version file is empty" >&2
  exit 1
fi

semver_gt() {
  local a="$1" b="$2"
  [[ "$a" != "$b" ]] && [[ "$(printf '%s\n' "$a" "$b" | sort -V | tail -1)" == "$a" ]]
}

LATEST_RELEASED="$(bash scripts/latest-released-version.sh || true)"
if [[ -n "$LATEST_RELEASED" ]] && ! semver_gt "$HEAD_VERSION" "$LATEST_RELEASED"; then
  echo "version $HEAD_VERSION must be strictly greater than latest release $LATEST_RELEASED" >&2
  exit 1
fi

git fetch origin "$BASE_REF" --quiet
MASTER_VERSION="$(git show "origin/${BASE_REF}:version" 2>/dev/null | tr -d '[:space:]' || true)"
if [[ -z "$MASTER_VERSION" ]]; then
  if ! git diff --name-only "origin/${BASE_REF}...HEAD" | grep -qx 'version'; then
    echo "version file missing on origin/${BASE_REF} and not added in this PR" >&2
    exit 1
  fi
elif ! semver_gt "$HEAD_VERSION" "$MASTER_VERSION"; then
  echo "version $HEAD_VERSION must be strictly greater than origin/${BASE_REF} ($MASTER_VERSION)" >&2
  exit 1
fi

bash scripts/sync-version.sh --check

if ! awk -v ver="$HEAD_VERSION" '
  BEGIN { found = 0; bullets = 0 }
  $0 ~ "^## \\[" ver "\\]" { found = 1; next }
  found && /^## \[/ { exit }
  found && /^- / { bullets++ }
  END { exit(found && bullets > 0 ? 0 : 1) }
' CHANGELOG.md; then
  echo "CHANGELOG.md must contain ## [$HEAD_VERSION] with at least one \"- \" bullet before the next ## [" >&2
  exit 1
fi

echo "release-check: OK ($HEAD_VERSION)"
