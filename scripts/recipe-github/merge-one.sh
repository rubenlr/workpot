#!/usr/bin/env bash
# Attempt git merge --no-ff of origin/<branch> into HEAD.
# Usage: merge-one.sh <short-branch-name>
# exit 0: merge committed or already ancestor; 2: conflicts; 1: fatal
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "${SCRIPT_DIR}/lib.sh"

ROOT="$(recipe_github_root)"
cd "$ROOT"

BRANCH="${1:-}"
if [[ -z "$BRANCH" ]]; then
  echo "usage: merge-one.sh <dependabot-branch-short-name>" >&2
  exit 1
fi

recipe_github_require_cmd git || exit 1
recipe_github_require_repo || exit 1

if [[ -n "$(git status --porcelain)" ]]; then
  printf '{"action":"merge-one","branch":%s,"merge":"fatal","conflict_paths":[],"escalate":"dirty working tree"}\n' \
    "$(recipe_github_json_string "$BRANCH")"
  exit 1
fi

if ! git rev-parse --verify "origin/${BRANCH}" >/dev/null 2>&1; then
  printf '{"action":"merge-one","branch":%s,"merge":"fatal","conflict_paths":[],"escalate":"missing remote ref"}\n' \
    "$(recipe_github_json_string "$BRANCH")"
  exit 1
fi

if git merge-base --is-ancestor "origin/${BRANCH}" HEAD 2>/dev/null; then
  sha="$(git rev-parse HEAD)"
  printf '{"action":"merge-one","branch":%s,"merge":"already","conflict_paths":[],"commits":{"merge":%s,"deprecations":null},"escalate":null}\n' \
    "$(recipe_github_json_string "$BRANCH")" "$(recipe_github_json_string "$sha")"
  exit 0
fi

set +e
git merge --no-ff --no-edit "origin/${BRANCH}" 2>/tmp/recipe-github-merge-err.$$
merge_rc=$?
set -e

if [[ $merge_rc -eq 0 ]]; then
  sha="$(git rev-parse HEAD)"
  printf '{"action":"merge-one","branch":%s,"merge":"ok","conflict_paths":[],"commits":{"merge":%s,"deprecations":null},"escalate":null}\n' \
    "$(recipe_github_json_string "$BRANCH")" "$(recipe_github_json_string "$sha")"
  rm -f /tmp/recipe-github-merge-err.$$
  exit 0
fi

mapfile -t conflict_paths < <(git diff --name-only --diff-filter=U 2>/dev/null || true)
if [[ ${#conflict_paths[@]} -eq 0 ]]; then
  err="$(tr '\n' ' ' </tmp/recipe-github-merge-err.$$ 2>/dev/null | head -c 200 || true)"
  rm -f /tmp/recipe-github-merge-err.$$
  git merge --abort 2>/dev/null || true
  printf '{"action":"merge-one","branch":%s,"merge":"fatal","conflict_paths":[],"escalate":%s}\n' \
    "$(recipe_github_json_string "$BRANCH")" "$(recipe_github_json_string "${err:-merge failed}")"
  exit 1
fi

rm -f /tmp/recipe-github-merge-err.$$
paths_json="$(recipe_github_json_string_array "${conflict_paths[@]}")"
printf '{"action":"merge-one","branch":%s,"merge":"conflicts","conflict_paths":%s,"escalate":null}\n' \
  "$(recipe_github_json_string "$BRANCH")" "$paths_json"
exit 2
