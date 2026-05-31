#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
INSTALLER="${ROOT}/scripts/install.sh"

ASSERT_VERSION_MATCH=false
if [[ "${1:-}" == "--assert-version-match" ]]; then
  ASSERT_VERSION_MATCH=true
fi

fail() {
  printf 'FAIL: %s\n' "$*" >&2
  exit 1
}

pass() {
  printf 'PASS: %s\n' "$*"
}

assert_file() {
  local path="$1"
  [[ -e "$path" ]] || fail "expected file to exist: $path"
}

assert_missing() {
  local path="$1"
  [[ ! -e "$path" ]] || fail "expected file to be absent: $path"
}

assert_contains() {
  local haystack="$1"
  local needle="$2"
  [[ "$haystack" == *"$needle"* ]] || fail "expected output to contain '$needle'"
}

create_release_fixture() {
  local fixture_dir="$1"
  local version="$2"
  local checksum_mode="${3:-good}"

  local assets_dir="${fixture_dir}/assets"
  mkdir -p "${assets_dir}/cli" "${assets_dir}/app-root/Workpot.app/Contents/MacOS"

  cat > "${assets_dir}/cli/workpot" <<EOF
#!/usr/bin/env bash
if [[ "\${1:-}" == "--version" ]]; then
  echo "${version}"
  exit 0
fi
echo "workpot smoke binary ${version}"
EOF
  chmod +x "${assets_dir}/cli/workpot"

  tar -C "${assets_dir}/cli" -czf "${assets_dir}/workpot-macos-aarch64.tar.gz" workpot
  shasum -a 256 "${assets_dir}/workpot-macos-aarch64.tar.gz" > "${assets_dir}/workpot-macos-aarch64.tar.gz.sha256"

  cat > "${assets_dir}/app-root/Workpot.app/Contents/MacOS/Workpot" <<EOF
#!/usr/bin/env bash
echo "Workpot app ${version}"
EOF
  chmod +x "${assets_dir}/app-root/Workpot.app/Contents/MacOS/Workpot"

  hdiutil create \
    -quiet \
    -volname "Workpot-${version}" \
    -srcfolder "${assets_dir}/app-root" \
    -ov \
    -format UDZO \
    "${assets_dir}/Workpot-${version}-aarch64.dmg" >/dev/null
  shasum -a 256 "${assets_dir}/Workpot-${version}-aarch64.dmg" > "${assets_dir}/Workpot-${version}-aarch64.dmg.sha256"

  if [[ "$checksum_mode" == "bad-cli-checksum" ]]; then
    echo "0000000000000000000000000000000000000000000000000000000000000000  workpot-macos-aarch64.tar.gz" \
      > "${assets_dir}/workpot-macos-aarch64.tar.gz.sha256"
  fi

  local release_json="${fixture_dir}/release.json"
  cat > "$release_json" <<EOF
{
  "tag_name": "v${version}",
  "assets": [
    {
      "name": "workpot-macos-aarch64.tar.gz",
      "browser_download_url": "file://${assets_dir}/workpot-macos-aarch64.tar.gz"
    },
    {
      "name": "workpot-macos-aarch64.tar.gz.sha256",
      "browser_download_url": "file://${assets_dir}/workpot-macos-aarch64.tar.gz.sha256"
    },
    {
      "name": "Workpot-${version}-aarch64.dmg",
      "browser_download_url": "file://${assets_dir}/Workpot-${version}-aarch64.dmg"
    },
    {
      "name": "Workpot-${version}-aarch64.dmg.sha256",
      "browser_download_url": "file://${assets_dir}/Workpot-${version}-aarch64.dmg.sha256"
    }
  ]
}
EOF

  printf '%s\n' "$release_json"
}

create_global_mocks() {
  local mock_bin="$1"
  local global_root="$2"
  mkdir -p "$mock_bin"

  cat > "${mock_bin}/sudo" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
exec "$@"
EOF

  cat > "${mock_bin}/mkdir" <<EOF
#!/usr/bin/env bash
set -euo pipefail
args=()
for arg in "\$@"; do
  case "\$arg" in
    /usr/local/bin*) arg="${global_root}/usr/local/bin\${arg#/usr/local/bin}" ;;
    /Applications*) arg="${global_root}/Applications\${arg#/Applications}" ;;
  esac
  args+=("\$arg")
done
exec /bin/mkdir "\${args[@]}"
EOF

  cat > "${mock_bin}/install" <<EOF
#!/usr/bin/env bash
set -euo pipefail
args=()
for arg in "\$@"; do
  case "\$arg" in
    /usr/local/bin*) arg="${global_root}/usr/local/bin\${arg#/usr/local/bin}" ;;
    /Applications*) arg="${global_root}/Applications\${arg#/Applications}" ;;
  esac
  args+=("\$arg")
done
exec /usr/bin/install "\${args[@]}"
EOF

  cat > "${mock_bin}/cp" <<EOF
#!/usr/bin/env bash
set -euo pipefail
args=()
for arg in "\$@"; do
  case "\$arg" in
    /usr/local/bin*) arg="${global_root}/usr/local/bin\${arg#/usr/local/bin}" ;;
    /Applications*) arg="${global_root}/Applications\${arg#/Applications}" ;;
  esac
  args+=("\$arg")
done
exec /bin/cp "\${args[@]}"
EOF

  cat > "${mock_bin}/rm" <<EOF
#!/usr/bin/env bash
set -euo pipefail
args=()
for arg in "\$@"; do
  case "\$arg" in
    /usr/local/bin*) arg="${global_root}/usr/local/bin\${arg#/usr/local/bin}" ;;
    /Applications*) arg="${global_root}/Applications\${arg#/Applications}" ;;
  esac
  args+=("\$arg")
done
exec /bin/rm "\${args[@]}"
EOF

  chmod +x "${mock_bin}/sudo" "${mock_bin}/mkdir" "${mock_bin}/install" "${mock_bin}/cp" "${mock_bin}/rm"
}

run_installer() {
  local home_dir="$1"
  local release_json_file="$2"
  shift 2
  HOME="$home_dir" WORKPOT_RELEASE_JSON="$(cat "$release_json_file")" bash "$INSTALLER" "$@"
}

test_default_install() {
  local base="$1"
  local version="$2"
  local home_dir="${base}/home-default"
  local fixture="${base}/fixture-default"
  mkdir -p "${home_dir}"
  local release_json
  release_json="$(create_release_fixture "$fixture" "$version")"

  run_installer "$home_dir" "$release_json"

  local cli="${home_dir}/.local/bin/workpot"
  local tray="${home_dir}/Applications/Workpot.app"
  assert_file "$cli"
  assert_file "$tray"

  if [[ "$ASSERT_VERSION_MATCH" == true ]]; then
    local installed_version
    installed_version="$("$cli" --version)"
    [[ "$installed_version" == "$version" ]] || fail "version mismatch: expected $version, got $installed_version"
  fi

  pass "default installs cli+tray"
}

test_only_cli() {
  local base="$1"
  local version="$2"
  local home_dir="${base}/home-cli"
  local fixture="${base}/fixture-cli"
  mkdir -p "${home_dir}"
  local release_json
  release_json="$(create_release_fixture "$fixture" "$version")"

  run_installer "$home_dir" "$release_json" --only-cli

  assert_file "${home_dir}/.local/bin/workpot"
  assert_missing "${home_dir}/Applications/Workpot.app"
  pass "--only-cli installs cli only"
}

test_only_tray() {
  local base="$1"
  local version="$2"
  local home_dir="${base}/home-tray"
  local fixture="${base}/fixture-tray"
  mkdir -p "${home_dir}"
  local release_json
  release_json="$(create_release_fixture "$fixture" "$version")"

  run_installer "$home_dir" "$release_json" --only-tray

  assert_missing "${home_dir}/.local/bin/workpot"
  assert_file "${home_dir}/Applications/Workpot.app"
  pass "--only-tray installs tray only"
}

test_global_install() {
  local base="$1"
  local version="$2"
  local fixture="${base}/fixture-global"
  local fake_home="${base}/home-global"
  local global_root="${base}/global-root"
  local mock_bin="${base}/mock-bin"
  mkdir -p "$fake_home" "$global_root"
  local release_json
  release_json="$(create_release_fixture "$fixture" "$version")"
  create_global_mocks "$mock_bin" "$global_root"

  local output
  output="$(
    PATH="${mock_bin}:$PATH" \
    HOME="$fake_home" \
    WORKPOT_RELEASE_JSON="$(cat "$release_json")" \
    bash "$INSTALLER" --global --only-cli
  )"

  assert_file "${global_root}/usr/local/bin/workpot"
  assert_contains "$output" "/usr/local/bin/workpot"
  pass "--global uses global cli path"
}

test_conflicting_only_flags() {
  local base="$1"
  local home_dir="${base}/home-conflict"
  local fixture="${base}/fixture-conflict"
  mkdir -p "$home_dir"
  local release_json
  release_json="$(create_release_fixture "$fixture" "9.9.9-smoke")"

  set +e
  HOME="$home_dir" WORKPOT_RELEASE_JSON="$(cat "$release_json")" \
    bash "$INSTALLER" --only-cli --only-tray >/dev/null 2>&1
  local exit_code=$?
  set -e

  [[ "$exit_code" -ne 0 ]] || fail "conflicting --only-* flags should fail non-zero"
  pass "conflicting --only-cli and --only-tray are rejected"
}

test_checksum_failure() {
  local base="$1"
  local version="$2"
  local home_dir="${base}/home-bad-checksum"
  local fixture="${base}/fixture-bad-checksum"
  mkdir -p "$home_dir"
  local release_json
  release_json="$(create_release_fixture "$fixture" "$version" "bad-cli-checksum")"

  set +e
  HOME="$home_dir" WORKPOT_RELEASE_JSON="$(cat "$release_json")" bash "$INSTALLER" --only-cli >/dev/null 2>&1
  local exit_code=$?
  set -e

  [[ "$exit_code" -ne 0 ]] || fail "checksum mismatch should fail non-zero"
  assert_missing "${home_dir}/.local/bin/workpot"
  pass "checksum mismatch fails closed"
}

main() {
  [[ -x "$INSTALLER" ]] || fail "installer not executable: $INSTALLER"

  workspace=""
  workspace="$(mktemp -d)"
  trap 'rm -rf "$workspace"' EXIT

  local version="9.9.9-smoke"
  test_default_install "$workspace" "$version"
  test_only_cli "$workspace" "$version"
  test_only_tray "$workspace" "$version"
  test_global_install "$workspace" "$version"
  test_conflicting_only_flags "$workspace"
  test_checksum_failure "$workspace" "$version"

  pass "all install smoke checks passed"
}

main "$@"
