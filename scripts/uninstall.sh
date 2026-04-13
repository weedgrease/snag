#!/usr/bin/env bash
set -euo pipefail

BINARY="snag"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

info() { printf "\033[1;34m==>\033[0m %s\n" "$1"; }
success() { printf "\033[1;32m==>\033[0m %s\n" "$1"; }
warn() { printf "\033[1;33m==>\033[0m %s\n" "$1"; }

CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/snag"
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/snag"

echo ""
echo "This will remove snag and optionally its data."
echo ""

# Remove binary
BINARY_PATH="${INSTALL_DIR}/${BINARY}"
if [ -f "$BINARY_PATH" ]; then
    info "Removing binary: ${BINARY_PATH}"
    if [ -w "$INSTALL_DIR" ]; then
        rm -f "$BINARY_PATH"
    else
        sudo rm -f "$BINARY_PATH"
    fi
    success "Binary removed."
else
    warn "Binary not found at ${BINARY_PATH}"
fi

# Remove config and data
echo ""
read -rp "Remove config and data? This deletes alerts, results, and credentials. [y/N] " REMOVE_DATA

if [[ "$REMOVE_DATA" =~ ^[Yy]$ ]]; then
    if [ -d "$CONFIG_DIR" ]; then
        info "Removing config: ${CONFIG_DIR}"
        rm -rf "$CONFIG_DIR"
    fi
    if [ -d "$DATA_DIR" ]; then
        info "Removing data: ${DATA_DIR}"
        rm -rf "$DATA_DIR"
    fi
    success "Config and data removed."
else
    info "Keeping config at ${CONFIG_DIR}"
    info "Keeping data at ${DATA_DIR}"
fi

echo ""
success "snag has been uninstalled."
