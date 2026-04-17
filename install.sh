#!/bin/bash
# lux installer — downloads prebuilt binary from GitHub releases.
# For building from source, use setup.sh instead.
set -euo pipefail

REPO="henrywang/lux"
INSTALL_DIR="$HOME/.local/bin"

echo "=== lux install ==="

# Detect architecture
ARCH=$(uname -m)
case "$ARCH" in
    x86_64)  TARGET="x86_64-unknown-linux-gnu" ;;
    aarch64) TARGET="aarch64-unknown-linux-gnu" ;;
    *) echo "Error: unsupported architecture: $ARCH"; exit 1 ;;
esac

# Resolve latest version
VERSION=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
    | grep -oP '"tag_name":\s*"\K[^"]+')

if [ -z "$VERSION" ]; then
    echo "Error: could not determine latest release"
    exit 1
fi

echo "Installing lux $VERSION ($TARGET)..."

# Download and extract
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
URL="https://github.com/$REPO/releases/download/$VERSION/lux-$TARGET.tar.gz"
curl -fsSL "$URL" | tar xz -C "$TMP"

# Install
mkdir -p "$INSTALL_DIR"
mv "$TMP/lux" "$INSTALL_DIR/lux"
chmod +x "$INSTALL_DIR/lux"

# Install ollama if missing
if ! command -v ollama &>/dev/null; then
    echo "Installing ollama..."
    curl -fsSL https://ollama.com/install.sh | sh
fi

# Pull model
echo "Pulling henrywang/lux model..."
ollama pull henrywang/lux

echo ""
echo "=== Install complete ==="
echo "Binary: $INSTALL_DIR/lux"
echo "Run 'lux' to start (make sure $INSTALL_DIR is in your PATH)"
