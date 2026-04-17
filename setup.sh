#!/bin/bash
# lux setup script for Fedora
set -euo pipefail

echo "=== lux setup ==="

# Check Fedora
if ! command -v dnf &>/dev/null; then
    echo "Error: dnf not found. This script is for Fedora."
    exit 1
fi

# Install Rust if needed
if ! command -v cargo &>/dev/null; then
    echo "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    # shellcheck disable=SC1091
    source "$HOME/.cargo/env"
fi

# Install ollama if needed
if ! command -v ollama &>/dev/null; then
    echo "Installing ollama..."
    curl -fsSL https://ollama.com/install.sh | sh
fi

# Pull fine-tuned model
echo "Pulling henrywang/lux model..."
ollama pull henrywang/lux

# Build lux
echo "Building lux..."
cargo build --release

# Symlink to ~/.local/bin
mkdir -p "$HOME/.local/bin"
ln -sf "$(pwd)/target/release/lux" "$HOME/.local/bin/lux"

echo ""
echo "=== Setup complete ==="
echo "Run 'lux' to start (make sure ~/.local/bin is in your PATH)"
