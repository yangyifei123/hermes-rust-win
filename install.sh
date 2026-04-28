#!/usr/bin/env bash
# Hermes CLI installer for Linux/macOS
set -euo pipefail

REPO="nousresearch/hermes-rust-win"
BINARY="hermes"

echo "Installing Hermes CLI..."

# Detect OS and architecture
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

case "$OS" in
    linux)  PLATFORM="unknown-linux-gnu" ;;
    darwin) PLATFORM="apple-darwin" ;;
    *)      echo "Unsupported OS: $OS"; exit 1 ;;
esac

case "$ARCH" in
    x86_64|amd64)  TARGET="x86_64-$PLATFORM" ;;
    aarch64|arm64) TARGET="aarch64-$PLATFORM" ;;
    *)             echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

# Get latest release
LATEST=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | head -1 | sed -E 's/.*"([^"]+)".*/\1/')
if [ -z "$LATEST" ]; then
    echo "Could not determine latest version. Install via cargo:"
    echo "  cargo install --git https://github.com/$REPO"
    exit 1
fi

ARCHIVE="${BINARY}-${LATEST}-${TARGET}.tar.gz"
URL="https://github.com/$REPO/releases/download/${LATEST}/${ARCHIVE}"

echo "Downloading $BINARY $LATEST for $TARGET..."
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

curl -fsSL "$URL" | tar -xzf - -C "$TMPDIR"

INSTALL_DIR="${HERMES_INSTALL_DIR:-$HOME/.local/bin}"
mkdir -p "$INSTALL_DIR"

mv "$TMPDIR/$BINARY" "$INSTALL_DIR/$BINARY"
chmod +x "$INSTALL_DIR/$BINARY"

echo ""
echo "Hermes $LATEST installed to $INSTALL_DIR/$BINARY"

if ! echo "$PATH" | grep -q "$INSTALL_DIR"; then
    echo ""
    echo "Add $INSTALL_DIR to your PATH:"
    echo "  echo 'export PATH=\"$INSTALL_DIR:\$PATH\"' >> ~/.bashrc"
    echo "  source ~/.bashrc"
fi

echo ""
echo "Run 'hermes --help' to get started."
