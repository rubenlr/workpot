#!/usr/bin/env bash
# Print [workspace.package].version from Cargo.toml (path or "-" for stdin).
set -euo pipefail

FILE="${1:-Cargo.toml}"

read_version() {
  awk -F ' *= *' '
    /^\[workspace\.package\]$/ { in_wp = 1; next }
    /^\[/ { in_wp = 0 }
    in_wp && $1 == "version" {
      gsub(/"/, "", $2)
      print $2
      exit
    }
  '
}

if [[ "$FILE" == "-" ]]; then
  read_version
else
  read_version <"$FILE"
fi
