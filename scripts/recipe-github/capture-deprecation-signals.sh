#!/usr/bin/env bash
# Capture deprecated API symbols (methods/functions/classes) in our dependency usage.
# Not package-level "this dependency is deprecated" notices.
# stdout: JSON { signals, symbols, deprecated, notes, count }
# exit 0 unless setup fatal (always soft on tool noise)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "${SCRIPT_DIR}/lib.sh"

ROOT="$(recipe_github_root)"
cd "$ROOT"

recipe_github_require_cmd git python3 || exit 1
recipe_github_require_repo || exit 1

TMP="$(mktemp)"
RAW="$(mktemp)"
trap 'rm -f "$TMP" "$RAW"' EXIT
: >"$RAW"

# Rust: clippy/rustc deprecated API diagnostics (bounded)
if command -v cargo >/dev/null 2>&1; then
  set +e
  cargo clippy --workspace --all-targets --message-format=short 2>"$TMP" >/dev/null
  set -e
  grep -iE 'deprecated|deprecation' "$TMP" 2>/dev/null | head -n 80 >>"$RAW" || true
fi

# JS/TS: tsc lines that mention deprecated APIs (non-fatal; skip package-manager noise)
if [[ -f package.json ]] && command -v pnpm >/dev/null 2>&1; then
  set +e
  pnpm exec tsc --noEmit --pretty false 2>"$TMP" >/dev/null
  set -e
  grep -iE 'deprecated|deprecation' "$TMP" 2>/dev/null \
    | grep -viE 'deprecated subdependenc|deprecated package' \
    | head -n 40 >>"$RAW" || true
fi

signals=()
while IFS= read -r line; do
  [[ -z "$line" ]] && continue
  signals+=("$line")
done <"$RAW"

merge_subject="$(git log -1 --pretty=%s 2>/dev/null || echo "")"

parse_out="$(
  python3 - "$RAW" <<'PY'
import json, re, sys
from pathlib import Path

text = Path(sys.argv[1]).read_text(errors="replace")
symbols = []
seen = set()
patterns = [
    re.compile(
        r"deprecated\s+(?:function|method|associated function|struct|enum|trait|type|constant|macro)\s+`([^`]+)`",
        re.I,
    ),
    re.compile(r"`([^`]+)`\s+is deprecated", re.I),
    re.compile(r"['\"]([A-Za-z_][\w.]*)['\"]\s+is deprecated", re.I),
    re.compile(r"deprecated\s+(?:API|symbol)?\s*[`'\"]([A-Za-z_][\w./:]*)[`'\"]", re.I),
]
for line in text.splitlines():
    low = line.lower()
    if "deprecated package" in low or "deprecated subdependenc" in low:
        continue
    if "deprecat" not in low:
        continue
    for pat in patterns:
        for m in pat.finditer(line):
            sym = m.group(1).strip()
            if not sym or sym in seen:
                continue
            if re.fullmatch(r"[\d.]+", sym):
                continue
            seen.add(sym)
            symbols.append(sym)
print(json.dumps(symbols))
PY
)"

deprecated="$(python3 -c 'import json,sys; print(len(json.loads(sys.argv[1])))' "$parse_out")"
sig_json="$(recipe_github_json_string_array "${signals[@]+"${signals[@]}"}")"
notes="merge_subject=$(recipe_github_json_escape "$merge_subject"); scan=API symbols (methods/functions/classes) from clippy/tsc; excludes package-deprecation notices"

printf '{"action":"capture-deprecation-signals","signals":%s,"symbols":%s,"deprecated":%s,"notes":%s,"count":%s}\n' \
  "$sig_json" \
  "$parse_out" \
  "$deprecated" \
  "$(recipe_github_json_string "$notes")" \
  "$deprecated"
exit 0
