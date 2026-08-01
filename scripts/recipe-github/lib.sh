#!/usr/bin/env bash
# Shared helpers for scripts/recipe-github/*
# shellcheck shell=bash

recipe_github_root() {
  local here
  here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  cd "${here}/../.." && pwd
}

recipe_github_require_cmd() {
  local c
  for c in "$@"; do
    if ! command -v "$c" >/dev/null 2>&1; then
      echo "recipe-github: required command not found: $c" >&2
      return 1
    fi
  done
}

recipe_github_require_repo() {
  if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    echo "recipe-github: not inside a git work tree" >&2
    return 1
  fi
}

recipe_github_require_clean() {
  if [[ -n "$(git status --porcelain)" ]]; then
    echo "recipe-github: working tree is dirty; commit or stash first" >&2
    return 1
  fi
}

# JSON-escape a string to stdout (no surrounding quotes)
recipe_github_json_escape() {
  local s=${1-}
  python3 -c 'import json,sys; print(json.dumps(sys.argv[1])[1:-1])' "$s"
}

# Print a JSON string value (with quotes)
recipe_github_json_string() {
  python3 -c 'import json,sys; print(json.dumps(sys.argv[1]))' "${1-}"
}

# Join bash array as JSON string array
recipe_github_json_string_array() {
  python3 -c 'import json,sys; print(json.dumps(sys.argv[1:]))' "$@"
}
