#!/usr/bin/env bash
# Capture compact deprecation-related signals for the deprecation-migrate agent.
# Does not fail the pipeline on warnings — always exit 0 unless setup fatal.
# stdout: JSON { signals: [...], notes: "..." }
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "${SCRIPT_DIR}/lib.sh"

ROOT="$(recipe_github_root)"
cd "$ROOT"

recipe_github_require_cmd git || exit 1
recipe_github_require_repo || exit 1

TMP="$(mktemp)"
signals=()

# Clippy / rustc deprecated mentions (bounded, non-fatal)
if command -v cargo >/dev/null 2>&1; then
  set +e
  cargo clippy --workspace --all-targets --message-format=short 2>"$TMP" >/dev/null
  set -e
  while IFS= read -r line; do
    [[ -z "$line" ]] && continue
    signals+=("$line")
  done < <(grep -iE 'deprecated|deprecation' "$TMP" 2>/dev/null | head -n 40 || true)
fi

# Recent merge subject for agent context
merge_subject="$(git log -1 --pretty=%s 2>/dev/null || echo "")"

# Changed manifests in last commit (hint which ecosystem)
changed="$(git diff-tree --no-commit-id --name-only -r HEAD 2>/dev/null | head -n 30 || true)"

rm -f "$TMP"

sig_json="$(recipe_github_json_string_array "${signals[@]+"${signals[@]}"}")"
notes="merge_subject=$(recipe_github_json_escape "$merge_subject"); scan=clippy deprecated lines (max 40); manifests_touched may include: $(echo "$changed" | tr '\n' ' ' | head -c 300)"

printf '{"action":"capture-deprecation-signals","signals":%s,"notes":%s,"count":%s}\n' \
  "$sig_json" \
  "$(recipe_github_json_string "$notes")" \
  "${#signals[@]}"
exit 0
