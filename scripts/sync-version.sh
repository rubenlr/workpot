#!/usr/bin/env bash
# Sync workspace manifests from repo-root version file.
# Usage: sync-version.sh [--check]
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

CHECK_ONLY=false
if [[ "${1:-}" == "--check" ]]; then
  CHECK_ONLY=true
fi

VERSION="$(bash scripts/read-workspace-version.sh)"
if [[ -z "$VERSION" ]]; then
  echo "version file is empty" >&2
  exit 1
fi

read_cargo_package_version() {
  awk -F ' *= *' '
    /^\[package\]$/ { in_pkg = 1; next }
    /^\[/ { in_pkg = 0 }
    in_pkg && $1 == "version" {
      gsub(/"/, "", $2)
      print $2
      exit
    }
  ' "$1"
}

read_json_version() {
  node -e "const j=JSON.parse(require('fs').readFileSync(process.argv[1],'utf8')); process.stdout.write(j.version||'')" "$1"
}

read_tauri_version() {
  node -e "const j=JSON.parse(require('fs').readFileSync(process.argv[1],'utf8')); process.stdout.write(j.version||'')" "$1"
}

verify_cargo_lock_versions() {
  local crate="$1"
  local expected="$2"
  local actual
  actual="$(cargo metadata --format-version 1 --no-deps 2>/dev/null | CRATE="$crate" node -e "
    const input = require('fs').readFileSync(0, 'utf8');
    const data = JSON.parse(input);
    const pkg = data.packages.find(p => p.name === process.env.CRATE);
    if (!pkg) process.exit(2);
    process.stdout.write(pkg.version);
  ")" || {
    echo "could not read cargo metadata for $crate" >&2
    exit 1
  }
  if [[ "$actual" != "$expected" ]]; then
    echo "Cargo.lock $crate version is $actual, expected $expected" >&2
    return 1
  fi
}

verify_all() {
  local failed=0

  if [[ "$(bash scripts/read-workspace-version.sh)" != "$VERSION" ]]; then
    echo "version file drift" >&2
    failed=1
  fi

  for file in crates/workpot-cli/Cargo.toml crates/workpot-core/Cargo.toml src-tauri/Cargo.toml; do
    local actual
    actual="$(read_cargo_package_version "$file")"
    if [[ "$actual" != "$VERSION" ]]; then
      echo "$file package.version is $actual, expected $VERSION" >&2
      failed=1
    fi
  done

  for file in package.json src-tauri/tauri.conf.json; do
    local actual
    actual="$(read_json_version "$file")"
    if [[ "$actual" != "$VERSION" ]]; then
      echo "$file version is $actual, expected $VERSION" >&2
      failed=1
    fi
  done

  for crate in workpot-cli workpot-core workpot-tray; do
    if ! verify_cargo_lock_versions "$crate" "$VERSION"; then
      failed=1
    fi
  done

  return "$failed"
}

update_cargo_package_version() {
  local file="$1"
  awk -v ver="$VERSION" '
    /^\[package\]$/ { in_pkg = 1; print; next }
    /^\[/ { in_pkg = 0 }
    in_pkg && /^version = / { print "version = \"" ver "\""; next }
    { print }
  ' "$file" >"$file.tmp"
  mv "$file.tmp" "$file"
}

update_workpot_core_path_dep() {
  local file="$1"
  local path_pattern="$2"
  sed -E "s|(workpot-core = \\{ path = \"${path_pattern}\", version = )\"[^\"]*\"|\\1\"${VERSION}\"|g" "$file" >"$file.tmp"
  mv "$file.tmp" "$file"
}

sync_manifests() {
  update_cargo_package_version crates/workpot-cli/Cargo.toml
  update_cargo_package_version crates/workpot-core/Cargo.toml
  update_cargo_package_version src-tauri/Cargo.toml

  update_workpot_core_path_dep crates/workpot-cli/Cargo.toml '../workpot-core'
  update_workpot_core_path_dep src-tauri/Cargo.toml '../crates/workpot-core'

  node -e "
    const fs = require('fs');
    const version = process.argv[1];
    for (const file of ['package.json', 'src-tauri/tauri.conf.json']) {
      const data = JSON.parse(fs.readFileSync(file, 'utf8'));
      data.version = version;
      fs.writeFileSync(file, JSON.stringify(data, null, 2) + '\n');
    }
  " "$VERSION"

  cargo generate-lockfile
}

if $CHECK_ONLY; then
  if verify_all; then
    echo "version sync OK ($VERSION)"
  else
    echo "version sync drift detected; run: just version" >&2
    exit 1
  fi
else
  sync_manifests
  if verify_all; then
    echo "synced version $VERSION"
  else
    echo "sync completed but verification failed" >&2
    exit 1
  fi
fi
