#!/usr/bin/env bash
# Delete remote branches on origin.
# Usage: delete-remotes.sh <short-branch> [<short-branch>...]
# stdout: JSON { deleted: [], failed: [] }
# exit 0 all deleted; 2 partial; 1 fatal (no args / no gh|git)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "${SCRIPT_DIR}/lib.sh"

ROOT="$(recipe_github_root)"
cd "$ROOT"

if [[ $# -lt 1 ]]; then
  echo "usage: delete-remotes.sh <branch> [<branch>...]" >&2
  exit 1
fi

recipe_github_require_cmd git || exit 1
recipe_github_require_repo || exit 1

deleted=()
failed=()

for b in "$@"; do
  set +e
  git push origin --delete "$b" >/tmp/recipe-github-del.$$ 2>&1
  rc=$?
  set -e
  if [[ $rc -eq 0 ]]; then
    deleted+=("$b")
  else
    failed+=("$b")
    echo "recipe-github: failed to delete origin/${b}" >&2
    tail -n 5 /tmp/recipe-github-del.$$ >&2 || true
  fi
done
rm -f /tmp/recipe-github-del.$$

del_json="$(recipe_github_json_string_array "${deleted[@]+"${deleted[@]}"}")"
fail_json="$(recipe_github_json_string_array "${failed[@]+"${failed[@]}"}")"

printf '{"action":"delete-remotes","deleted":%s,"failed":%s}\n' "$del_json" "$fail_json"

if [[ ${#failed[@]} -gt 0 ]]; then
  exit 2
fi
exit 0
