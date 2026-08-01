#!/usr/bin/env bash
# List origin/dependabot/** short branch names after fetch --prune.
# Dependabot refs are nested (e.g. origin/dependabot/npm_and_yarn/eslint-10.8.0);
# for-each-ref '*' is one path segment only — use '**'.
# stdout: JSON { "action", "branches": [...], "count": N }
# exit 0 always when listing succeeds (including empty); 1 fatal
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "${SCRIPT_DIR}/lib.sh"

ROOT="$(recipe_github_root)"
cd "$ROOT"

recipe_github_require_cmd git || exit 1
recipe_github_require_repo || exit 1

echo "recipe-github: fetching origin --prune…" >&2
if ! git fetch origin --prune 2>/dev/null; then
  # Still allow listing from existing remotes if fetch fails (offline)
  echo "recipe-github: warning: git fetch failed; listing existing remotes" >&2
fi

mapfile -t refs < <(
  git for-each-ref --format='%(refname:short)' 'refs/remotes/origin/dependabot/**' 2>/dev/null \
    | sed 's#^origin/##' \
    | sort -u
)

branches_json="$(recipe_github_json_string_array "${refs[@]+"${refs[@]}"}")"
count="${#refs[@]}"

printf '{"action":"list-dependabot-branches","branches":%s,"count":%s}\n' \
  "$branches_json" "$count"
exit 0
