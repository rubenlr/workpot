#!/usr/bin/env bash
# List failed GitHub Actions checks for the current PR and print filtered error logs.
set -euo pipefail

PR="${1:-}"

if ! command -v gh >/dev/null 2>&1; then
  echo "gh-pr-error: gh CLI not found in PATH" >&2
  exit 1
fi

pr_args=()
if [[ -n "$PR" ]]; then
  pr_args=("$PR")
fi

if ! sha="$(gh pr view "${pr_args[@]}" --json headRefOid -q .headRefOid 2>/dev/null)"; then
  echo "gh-pr-error: no pull request for current branch (pass PR number as first arg)" >&2
  exit 1
fi

failed_checks="$(
  gh pr checks "${pr_args[@]}" --json name,state,link,bucket \
    -q '.[] | select(.bucket=="fail") | "\(.name)\t\(.state)\t\(.link)"'
)"

if [[ -z "$failed_checks" ]]; then
  echo "gh-pr-error: no failed checks on PR head ${sha}"
  exit 0
fi

echo "=== Failed checks (PR head ${sha}) ==="
while IFS=$'\t' read -r name state link; do
  printf '%s  %s\n      %s\n' "$name" "$state" "$link"
done <<<"$failed_checks"
echo

run_ids="$(
  gh run list -c "$sha" -s failure --json databaseId -q '.[].databaseId'
)"

if [[ -z "$run_ids" ]]; then
  echo "gh-pr-error: failed checks found but no failed workflow runs on ${sha}" >&2
  exit 1
fi

while read -r id; do
  [[ -z "$id" ]] && continue
  label="$(
    gh run view "$id" --json name,workflowName \
      -q '"\(.workflowName) / \(.name)"'
  )"
  echo "=== ${label} (run ${id}) ==="
  gh run view "$id" --log-failed 2>&1 || true
  echo
done <<<"$run_ids" \
  | grep -vE '^(##\[group\]|##\[endgroup\]|Post |Prepare |Restore |Cache |Set up |Run actions/|Complete job|Waiting for|Fetching |Downloading |Extracting |Resolved |job |shell: )' \
  | grep -E '(^=== |##\[error\]|error:|Error:|FAILED|failure|panic|assert|exit code [1-9]|not found|cannot find|No such file|command not found|fatal:|Caused by:|^\s*--> |npm ERR!|test result: FAILED)' \
  || true
