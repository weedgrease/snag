#!/usr/bin/env bash
set -euo pipefail

REPO="weedgrease/snag"
BINARY="snag"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

info() { printf "\033[1;34m==>\033[0m %s\n" "$1"; }
success() { printf "\033[1;32m==>\033[0m %s\n" "$1"; }
error() { printf "\033[1;31m==>\033[0m %s\n" "$1" >&2; exit 1; }

# Detect OS
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
case "$OS" in
    linux)  OS_TARGET="unknown-linux-gnu" ;;
    darwin) OS_TARGET="apple-darwin" ;;
    *)      error "Unsupported operating system: $OS" ;;
esac

# Detect architecture
ARCH="$(uname -m)"
case "$ARCH" in
    x86_64|amd64)   ARCH_TARGET="x86_64" ;;
    aarch64|arm64)   ARCH_TARGET="aarch64" ;;
    *)               error "Unsupported architecture: $ARCH" ;;
esac

TARGET="${ARCH_TARGET}-${OS_TARGET}"
info "Detected platform: $TARGET"

# Get latest release version
info "Fetching latest release..."
VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep '"tag_name"' \
    | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')

if [ -z "$VERSION" ]; then
    error "Could not determine latest version. Check https://github.com/${REPO}/releases"
fi

info "Latest version: $VERSION"

# Download and extract
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

ARCHIVE="${BINARY}-${TARGET}.tar.gz"
URL="https://github.com/${REPO}/releases/download/${VERSION}/${ARCHIVE}"

info "Downloading ${ARCHIVE}..."
if ! curl -fsSL "$URL" -o "${TMPDIR}/${ARCHIVE}"; then
    error "Download failed. Binary may not be available for $TARGET."
fi

tar xzf "${TMPDIR}/${ARCHIVE}" -C "$TMPDIR"

if [ ! -f "${TMPDIR}/${BINARY}" ]; then
    error "Binary not found in archive. Expected: ${BINARY}"
fi

chmod +x "${TMPDIR}/${BINARY}"

# Install
info "Installing to ${INSTALL_DIR}/${BINARY}..."
if [ -w "$INSTALL_DIR" ]; then
    mv "${TMPDIR}/${BINARY}" "${INSTALL_DIR}/${BINARY}"
else
    info "Requesting sudo access to install to ${INSTALL_DIR}..."
    sudo mv "${TMPDIR}/${BINARY}" "${INSTALL_DIR}/${BINARY}"
fi

# Verify
if command -v "$BINARY" &>/dev/null; then
    success "snag ${VERSION} installed successfully!"
    info "Run 'snag' to launch the TUI."
else
    success "Installed to ${INSTALL_DIR}/${BINARY}"
    info "Make sure ${INSTALL_DIR} is in your PATH."
fi
