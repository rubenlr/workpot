#!/usr/bin/env bash
# Full local quality gate: just fmt → just pre → just test (fail-fast).
# stdout: JSON with per-step status; on failure includes log_tail
# exit 0 all pass; 2 verify failed; 1 setup fatal
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "${SCRIPT_DIR}/lib.sh"

ROOT="$(recipe_github_root)"
cd "$ROOT"

recipe_github_require_cmd just git || exit 1
recipe_github_require_repo || exit 1

LOG_DIR="$(mktemp -d "${TMPDIR:-/tmp}/recipe-github-verify.XXXXXX")"
cleanup() { rm -rf "$LOG_DIR"; }
trap cleanup EXIT

fmt_status="pending"
pre_status="pending"
test_status="pending"

emit_fail() {
  local name="$1"
  local log="$2"
  local tail
  tail="$(tail -n 80 "$log" | python3 -c 'import json,sys; print(json.dumps(sys.stdin.read()))')"
  printf '{"action":"verify-full","verify":{"fmt":"%s","pre":"%s","test":"%s"},"failed_step":%s,"log_tail":%s}\n' \
    "$fmt_status" "$pre_status" "$test_status" \
    "$(recipe_github_json_string "$name")" "$tail"
}

run_step() {
  local name="$1"
  shift
  local log="${LOG_DIR}/${name}.log"
  set +e
  "$@" >"$log" 2>&1
  local rc=$?
  set -e
  echo "$rc:$log"
}

result="$(run_step fmt just fmt)"
rc="${result%%:*}"
log="${result#*:}"
if [[ "$rc" != "0" ]]; then
  fmt_status="fail"
  emit_fail fmt "$log"
  exit 2
fi
fmt_status="pass"

result="$(run_step pre just pre)"
rc="${result%%:*}"
log="${result#*:}"
if [[ "$rc" != "0" ]]; then
  pre_status="fail"
  emit_fail pre "$log"
  exit 2
fi
pre_status="pass"

result="$(run_step test just test)"
rc="${result%%:*}"
log="${result#*:}"
if [[ "$rc" != "0" ]]; then
  test_status="fail"
  emit_fail test "$log"
  exit 2
fi
test_status="pass"

printf '{"action":"verify-full","verify":{"fmt":"pass","pre":"pass","test":"pass"},"failed_step":null,"log_tail":null}\n'
exit 0
