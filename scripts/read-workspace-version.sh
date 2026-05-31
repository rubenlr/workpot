#!/usr/bin/env bash
# Print the release version from crates/workpot-cli/Cargo.toml [package].version.
set -euo pipefail

FILE="${1:-crates/workpot-cli/Cargo.toml}"

read_version() {
  awk -F ' *= *' '
    /^\[package\]$/ { in_pkg = 1; next }
    /^\[/ { in_pkg = 0 }
    in_pkg && $1 == "version" {
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
