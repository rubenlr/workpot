#!/usr/bin/env bash
set -euo pipefail

DEFAULT_REPO="rubenlr/workpot"
DEFAULT_API_BASE="https://api.github.com"

CLI_ASSET_NAME="workpot-macos-aarch64.tar.gz"
CLI_CHECKSUM_NAME="${CLI_ASSET_NAME}.sha256"

USER_CLI_PATH="${HOME}/.local/bin/workpot"      # ~/.local/bin/workpot
USER_TRAY_PATH="${HOME}/Applications/Workpot.app"  # ~/Applications/Workpot.app
GLOBAL_CLI_PATH="/usr/local/bin/workpot"
GLOBAL_TRAY_PATH="/Applications/Workpot.app"

INSTALL_CLI=true
INSTALL_TRAY=true
GLOBAL_INSTALL=false

usage() {
  cat <<'EOF'
Usage: install.sh [--only-cli | --only-tray] [--global] [--help]

Install Workpot from the latest GitHub release.

Flags:
  --only-cli   Install only the CLI
  --only-tray  Install only the tray app
  --global     Install to /usr/local/bin and /Applications
  --help       Show this help
EOF
}

log() {
  printf '%s\n' "$*"
}

err() {
  printf 'error: %s\n' "$*" >&2
}

require_cmd() {
  local cmd="$1"
  command -v "$cmd" >/dev/null 2>&1 || {
    err "required command not found: $cmd"
    exit 1
  }
}

run_with_sudo_if_needed() {
  local target="$1"
  shift
  if [[ "${EUID:-$(id -u)}" -eq 0 ]]; then
    "$@"
    return
  fi

  if [[ -e "$target" ]]; then
    if [[ -w "$target" ]]; then
      "$@"
    else
      sudo "$@"
    fi
    return
  fi

  local parent
  parent="$(dirname "$target")"
  while [[ ! -e "$parent" && "$parent" != "/" ]]; do
    parent="$(dirname "$parent")"
  done
  if [[ -w "$parent" ]]; then
    "$@"
  else
    sudo "$@"
  fi
}

parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --only-cli)
        INSTALL_CLI=true
        INSTALL_TRAY=false
        ;;
      --only-tray)
        INSTALL_CLI=false
        INSTALL_TRAY=true
        ;;
      --global)
        GLOBAL_INSTALL=true
        ;;
      --help|-h)
        usage
        exit 0
        ;;
      *)
        err "unknown flag: $1"
        usage
        exit 1
        ;;
    esac
    shift
  done
}

verify_macos() {
  if [[ "$(uname -s)" != "Darwin" ]]; then
    err "this installer currently supports macOS only"
    exit 1
  fi
}

fetch_latest_release_json() {
  local repo="${WORKPOT_REPO:-$DEFAULT_REPO}"
  local api_base="${WORKPOT_API_BASE:-$DEFAULT_API_BASE}"

  if [[ -n "${WORKPOT_RELEASE_JSON:-}" ]]; then
    printf '%s\n' "$WORKPOT_RELEASE_JSON"
    return
  fi

  curl -fsSL \
    -H "Accept: application/vnd.github+json" \
    "${api_base}/repos/${repo}/releases/latest"
}

asset_url_by_name() {
  local release_json="$1"
  local name="$2"
  printf '%s' "$release_json" | jq -r --arg name "$name" '
    .assets[] | select(.name == $name) | .browser_download_url
  ' | head -n 1
}

asset_url_by_pattern() {
  local release_json="$1"
  local pattern="$2"
  printf '%s' "$release_json" | jq -r --arg pattern "$pattern" '
    .assets[] | select(.name | test($pattern)) | .browser_download_url
  ' | head -n 1
}

download_file() {
  local url="$1"
  local output="$2"
  curl -fsSL "$url" -o "$output"
}

verify_sha256() {
  local artifact="$1"
  local checksum_file="$2"
  local expected
  local actual

  expected="$(awk 'NF {print $1; exit}' "$checksum_file")"
  if [[ -z "$expected" ]]; then
    err "checksum file is empty: $checksum_file"
    exit 1
  fi

  actual="$(shasum -a 256 "$artifact" | awk '{print $1}')"
  if [[ "$actual" != "$expected" ]]; then
    err "checksum mismatch for $(basename "$artifact")"
    err "expected: $expected"
    err "actual:   $actual"
    exit 1
  fi
}

stage_cli_binary() {
  local cli_archive="$1"
  local stage_root="$2"
  local extract_dir="${stage_root}/cli-extract"
  local staged_binary="${stage_root}/workpot"

  mkdir -p "$extract_dir"
  tar -xzf "$cli_archive" -C "$extract_dir"

  if [[ ! -f "${extract_dir}/workpot" ]]; then
    err "CLI archive does not contain workpot binary at archive root"
    exit 1
  fi

  cp "${extract_dir}/workpot" "$staged_binary"
  chmod +x "$staged_binary"
  printf '%s\n' "$staged_binary"
}

stage_tray_app() {
  local dmg_file="$1"
  local stage_root="$2"
  local mount_point="${stage_root}/dmg-mount"
  local staged_app="${stage_root}/Workpot.app"

  mkdir -p "$mount_point"
  hdiutil attach "$dmg_file" -mountpoint "$mount_point" -nobrowse -quiet
  cp -R "${mount_point}/Workpot.app" "$staged_app"
  hdiutil detach "$mount_point" -quiet

  if [[ ! -d "$staged_app" ]]; then
    err "failed to stage Workpot.app from DMG"
    exit 1
  fi

  printf '%s\n' "$staged_app"
}

install_cli_binary() {
  local staged_binary="$1"
  local target_path="$2"
  local target_dir
  target_dir="$(dirname "$target_path")"

  run_with_sudo_if_needed "$target_dir" mkdir -p "$target_dir"
  run_with_sudo_if_needed "$target_path" install -m 0755 "$staged_binary" "$target_path"
}

install_tray_app() {
  local staged_app="$1"
  local target_app_path="$2"
  local target_parent
  target_parent="$(dirname "$target_app_path")"

  run_with_sudo_if_needed "$target_parent" mkdir -p "$target_parent"
  run_with_sudo_if_needed "$target_app_path" rm -rf "$target_app_path"
  run_with_sudo_if_needed "$target_parent" cp -R "$staged_app" "$target_app_path"
}

print_next_steps() {
  local release_tag="$1"
  local cli_path="$2"
  local tray_path="$3"

  log ""
  log "Installed from release ${release_tag}."

  if [[ "$INSTALL_CLI" == true ]]; then
    log "- CLI installed at: ${cli_path}"
    if [[ "$GLOBAL_INSTALL" == false && ":$PATH:" != *":${HOME}/.local/bin:"* ]]; then
      log "  PATH hint: add ~/.local/bin to PATH"
      log "  Example (zsh): echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.zshrc"
    fi
  fi

  if [[ "$INSTALL_TRAY" == true ]]; then
    log "- Tray installed at: ${tray_path}"
    log "  Open Workpot.app from Finder or Spotlight."
  fi
}

main() {
  parse_args "$@"
  verify_macos

  require_cmd curl
  require_cmd jq
  require_cmd shasum
  require_cmd tar
  require_cmd hdiutil

  local release_json
  release_json="$(fetch_latest_release_json)"
  local release_tag
  release_tag="$(printf '%s' "$release_json" | jq -r '.tag_name')"
  if [[ -z "$release_tag" || "$release_tag" == "null" ]]; then
    err "unable to determine latest release tag"
    exit 1
  fi

  local cli_target_path="$USER_CLI_PATH"
  local tray_target_path="$USER_TRAY_PATH"
  if [[ "$GLOBAL_INSTALL" == true ]]; then
    cli_target_path="$GLOBAL_CLI_PATH"
    tray_target_path="$GLOBAL_TRAY_PATH"
  fi

  temp_root=""
  temp_root="$(mktemp -d)"
  trap 'rm -rf "$temp_root"' EXIT

  local staged_cli=""
  local staged_tray=""

  if [[ "$INSTALL_CLI" == true ]]; then
    local cli_url
    local cli_checksum_url
    cli_url="$(asset_url_by_name "$release_json" "$CLI_ASSET_NAME")"
    cli_checksum_url="$(asset_url_by_name "$release_json" "$CLI_CHECKSUM_NAME")"
    if [[ -z "$cli_url" || -z "$cli_checksum_url" ]]; then
      err "latest release is missing CLI assets (${CLI_ASSET_NAME} + ${CLI_CHECKSUM_NAME})"
      exit 1
    fi

    local cli_archive="${temp_root}/${CLI_ASSET_NAME}"
    local cli_checksum="${temp_root}/${CLI_CHECKSUM_NAME}"
    download_file "$cli_url" "$cli_archive"
    download_file "$cli_checksum_url" "$cli_checksum"
    verify_sha256 "$cli_archive" "$cli_checksum"
    staged_cli="$(stage_cli_binary "$cli_archive" "$temp_root")"
  fi

  if [[ "$INSTALL_TRAY" == true ]]; then
    local dmg_name_pattern='^Workpot-.*-aarch64\.dmg$'
    local dmg_checksum_name_pattern='^Workpot-.*-aarch64\.dmg\.sha256$'
    local dmg_url
    local dmg_checksum_url
    dmg_url="$(asset_url_by_pattern "$release_json" "$dmg_name_pattern")"
    dmg_checksum_url="$(asset_url_by_pattern "$release_json" "$dmg_checksum_name_pattern")"
    if [[ -z "$dmg_url" || -z "$dmg_checksum_url" ]]; then
      err "latest release is missing DMG assets (Workpot-<version>-aarch64.dmg + .sha256)"
      exit 1
    fi

    local dmg_file="${temp_root}/workpot.dmg"
    local dmg_checksum_file="${temp_root}/workpot.dmg.sha256"
    download_file "$dmg_url" "$dmg_file"
    download_file "$dmg_checksum_url" "$dmg_checksum_file"
    verify_sha256 "$dmg_file" "$dmg_checksum_file"
    staged_tray="$(stage_tray_app "$dmg_file" "$temp_root")"
  fi

  # Mutate install targets only after all selected artifacts are downloaded + verified.
  if [[ "$INSTALL_CLI" == true ]]; then
    install_cli_binary "$staged_cli" "$cli_target_path"
  fi
  if [[ "$INSTALL_TRAY" == true ]]; then
    install_tray_app "$staged_tray" "$tray_target_path"
  fi

  print_next_steps "$release_tag" "$cli_target_path" "$tray_target_path"
}

main "$@"
