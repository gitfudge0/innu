#!/usr/bin/env bash

set -euo pipefail

REPO_OWNER="${INNU_REPO_OWNER:-gitfudge0}"
REPO_NAME="${INNU_REPO_NAME:-innu}"
DEFAULT_REF="${INNU_REF:-main}"
ARCHIVE_URL="https://github.com/${REPO_OWNER}/${REPO_NAME}/archive/refs/heads/${DEFAULT_REF}.tar.gz"
BIN_NAME="innu"
APP_NAME="Innu"
INSTALL_BIN_DIR="${HOME}/.local/bin"
INSTALL_SHARE_DIR="${HOME}/.local/share/applications"
INSTALL_BIN_PATH="${INSTALL_BIN_DIR}/${BIN_NAME}"
INSTALL_DESKTOP_PATH="${INSTALL_SHARE_DIR}/${BIN_NAME}.desktop"
SOURCE_DIR=""
TEMP_SOURCE_DIR=""

require_cmd() {
  local cmd="$1"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "Missing required command: $cmd" >&2
    exit 1
  fi
}

cleanup_tmp_dir() {
  local tmp_dir="$1"
  if [[ -n "$tmp_dir" && -d "$tmp_dir" ]]; then
    rm -rf -- "$tmp_dir"
  fi
}

is_innu_source_dir() {
  local dir="$1"
  [[ -f "$dir/Cargo.toml" ]] && grep -q '^name = "innu"' "$dir/Cargo.toml"
}

prepare_source_tree() {
  local script_dir=""

  if [[ -n "${BASH_SOURCE[0]:-}" ]]; then
    script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
    if is_innu_source_dir "$script_dir"; then
      SOURCE_DIR="$script_dir"
      return
    fi
  fi

  if is_innu_source_dir "$(pwd)"; then
    SOURCE_DIR="$(pwd)"
    return
  fi

  require_cmd curl
  require_cmd tar

  TEMP_SOURCE_DIR="$(mktemp -d)"
  trap 'cleanup_tmp_dir "$TEMP_SOURCE_DIR"' EXIT

  echo "Downloading ${APP_NAME} source from ${REPO_OWNER}/${REPO_NAME}@${DEFAULT_REF}..."
  curl -fsSL "$ARCHIVE_URL" | tar -xz -C "$TEMP_SOURCE_DIR"

  for path in "$TEMP_SOURCE_DIR"/*; do
    if [[ -d "$path" ]]; then
      SOURCE_DIR="$path"
      break
    fi
  done

  if [[ -z "$SOURCE_DIR" ]] || ! is_innu_source_dir "$SOURCE_DIR"; then
    echo "Downloaded archive did not contain a valid ${APP_NAME} source tree." >&2
    exit 1
  fi
}

install_innu() {
  local source_dir="$1"

  require_cmd cargo
  mkdir -p "$INSTALL_BIN_DIR" "$INSTALL_SHARE_DIR"

  echo "Building ${APP_NAME}..."
  cargo build --release --manifest-path "$source_dir/Cargo.toml"

  echo "Installing binary to ${INSTALL_BIN_PATH}"
  install -m 0755 "$source_dir/target/release/${BIN_NAME}" "$INSTALL_BIN_PATH"

  echo "Installing desktop entry to ${INSTALL_DESKTOP_PATH}"
  cat > "$INSTALL_DESKTOP_PATH" <<EOF
[Desktop Entry]
Type=Application
Version=1.0
Name=${APP_NAME}
Exec=${INSTALL_BIN_PATH}
Terminal=false
Categories=Network;Utility;
EOF
}

main() {
  prepare_source_tree
  install_innu "$SOURCE_DIR"

  echo
  echo "Installed ${APP_NAME}."
  echo "- Binary: ${INSTALL_BIN_PATH}"
  echo "- Desktop entry: ${INSTALL_DESKTOP_PATH}"
  echo
  echo "Launch with: ${BIN_NAME}"
  echo "Make sure ${INSTALL_BIN_DIR} is on your PATH."
  echo "This installer builds from source, so Rust and Cargo must be available."
  echo "To uninstall later, run: ${BIN_NAME} uninstall"
}

main "$@"
