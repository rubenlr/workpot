#!/usr/bin/env bash
# Aggregate trigger-site counts for the report `trigger_sites` column.
# NOT lines-of-code. Each deprecated call/usage migrated = 1; each build-break
# site fixed = 1. Same symbol at two call sites → 2.
#
# Usage:
#   measure-trigger-sites.sh [--deprecation-sites N] [--build-sites N]
# Defaults: both 0.
# stdout: JSON { action, deprecation_sites, build_sites, trigger_sites }
# exit 0 success; 1 usage/fatal
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "${SCRIPT_DIR}/lib.sh"

ROOT="$(recipe_github_root)"
cd "$ROOT"

deprecation_sites=0
build_sites=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --deprecation-sites)
      deprecation_sites="${2:-}"
      shift 2
      ;;
    --build-sites)
      build_sites="${2:-}"
      shift 2
      ;;
    -h|--help)
      echo "usage: measure-trigger-sites.sh [--deprecation-sites N] [--build-sites N]" >&2
      exit 1
      ;;
    *)
      echo "usage: measure-trigger-sites.sh [--deprecation-sites N] [--build-sites N]" >&2
      exit 1
      ;;
  esac
done

if ! [[ "$deprecation_sites" =~ ^[0-9]+$ && "$build_sites" =~ ^[0-9]+$ ]]; then
  printf '{"action":"measure-trigger-sites","deprecation_sites":0,"build_sites":0,"trigger_sites":0,"escalate":"sites must be non-negative integers"}\n'
  exit 1
fi

trigger_sites=$((deprecation_sites + build_sites))

printf '{"action":"measure-trigger-sites","deprecation_sites":%s,"build_sites":%s,"trigger_sites":%s,"escalate":null}\n' \
  "$deprecation_sites" "$build_sites" "$trigger_sites"
exit 0
