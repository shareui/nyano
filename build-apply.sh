#!/usr/bin/env bash
set -euo pipefail

BINARY_NAME="nyano"
INSTALL_DIR="/usr/local/bin"
TARGET_PATH="target/release/${BINARY_NAME}"

cd "$(dirname "$0")"

owned_by_package_manager() {
 local path="$1"
 if command -v dpkg >/dev/null 2>&1 && dpkg -S "$path" >/dev/null 2>&1; then
  return 0
 fi
 if command -v rpm >/dev/null 2>&1 && rpm -qf "$path" >/dev/null 2>&1; then
  return 0
 fi
 if command -v pacman >/dev/null 2>&1 && pacman -Qo "$path" >/dev/null 2>&1; then
  return 0
 fi
 if command -v brew >/dev/null 2>&1 && brew list --formula 2>/dev/null | grep -qx "${BINARY_NAME}"; then
  return 0
 fi
 return 1
}

remove_foreign_install() {
 local existing
 existing="$(command -v "${BINARY_NAME}" 2>/dev/null)" || return 0
 if [ "${existing}" = "${INSTALL_DIR}/${BINARY_NAME}" ]; then
  return 0
 fi
 if owned_by_package_manager "${existing}"; then
  echo "Note: found ${BINARY_NAME} at ${existing}, managed by a package manager, leaving it alone"
  return 0
 fi
 echo "Removing existing ${BINARY_NAME} at ${existing}"
 if [ -w "$(dirname "${existing}")" ]; then
  rm -f -- "${existing}"
 else
  sudo rm -f -- "${existing}"
 fi
}

remove_foreign_install

echo "Building ${BINARY_NAME} (release)..."

if cargo build --release; then
 if [ ! -f "${TARGET_PATH}" ]; then
  echo "Error: build succeeded but binary not found at ${TARGET_PATH}"
  exit 1
 fi

 if [ ! -w "${INSTALL_DIR}" ]; then
  echo "Installing to ${INSTALL_DIR} (requires sudo)..."
  sudo install -m 755 "${TARGET_PATH}" "${INSTALL_DIR}/${BINARY_NAME}"
 else
  install -m 755 "${TARGET_PATH}" "${INSTALL_DIR}/${BINARY_NAME}"
 fi

 echo "Successfully!"
else
 echo "Build failed."
 exit 1
fi
