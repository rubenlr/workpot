#!/usr/bin/env bash
set -euo pipefail

BANNED='reqwest|ureq|hyper|curl|oauth2|isahc|surf|awc|tokio-tungstenite|hyper-rustls'

check_crate() {
  local crate="$1"
  if cargo tree -p "$crate" 2>/dev/null | grep -vE '^#' | grep -qE "$BANNED"; then
    echo "ERROR: banned network dependency detected in $crate tree" >&2
    cargo tree -p "$crate" | grep -vE '^#' | grep -E "$BANNED" || true
    exit 1
  fi
}

check_crate workpot-core
check_crate workpot-cli
echo "OK: no banned network crates in workpot-core or workpot-cli dependency trees"
