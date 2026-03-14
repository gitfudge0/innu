#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
BIN_NAME="innu"
APP_NAME="Innu"
INSTALL_BIN_DIR="${HOME}/.local/bin"
INSTALL_SHARE_DIR="${HOME}/.local/share/applications"
INSTALL_BIN_PATH="${INSTALL_BIN_DIR}/${BIN_NAME}"
INSTALL_DESKTOP_PATH="${INSTALL_SHARE_DIR}/${BIN_NAME}.desktop"

mkdir -p "${INSTALL_BIN_DIR}" "${INSTALL_SHARE_DIR}"

echo "Building ${APP_NAME}..."
cargo build --release --manifest-path "${ROOT_DIR}/Cargo.toml"

echo "Installing binary to ${INSTALL_BIN_PATH}"
install -m 0755 "${ROOT_DIR}/target/release/${BIN_NAME}" "${INSTALL_BIN_PATH}"

echo "Installing desktop entry to ${INSTALL_DESKTOP_PATH}"
cat > "${INSTALL_DESKTOP_PATH}" <<EOF
[Desktop Entry]
Type=Application
Version=1.0
Name=${APP_NAME}
Exec=${INSTALL_BIN_PATH}
Terminal=false
Categories=Network;Utility;
EOF

echo
echo "Installed ${APP_NAME}."
echo "- Binary: ${INSTALL_BIN_PATH}"
echo "- Desktop entry: ${INSTALL_DESKTOP_PATH}"
echo
echo "Make sure ${INSTALL_BIN_DIR} is on your PATH."
echo "To uninstall later, run: ${BIN_NAME} uninstall"
