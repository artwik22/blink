#!/bin/bash

# Update script for Blink
# This script rebuilds and reinstalls blink from the current source code

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "=== Blink Update ==="
echo ""

# Check if Rust toolchain is configured
if ! rustc --version &> /dev/null; then
    echo "Rust toolchain not configured. Setting up default stable toolchain..."
    rustup default stable
fi

# Clean previous build for fresh compilation
echo "Cleaning previous build..."
cargo clean

echo "Building Blink..."
cargo build --release

if [ ! -f "target/release/blink" ]; then
    echo "Error: Build failed or binary not found"
    exit 1
fi

echo ""
echo "Build successful!"
echo ""

# Kill running blink processes if any
if pgrep -x "blink" > /dev/null; then
    echo "Found running blink processes. Stopping them..."
    pkill -x blink || killall blink 2>/dev/null || true
    sleep 1
    # Force kill if still running
    if pgrep -x "blink" > /dev/null; then
        echo "Force stopping remaining processes..."
        pkill -9 -x blink || killall -9 blink 2>/dev/null || true
        sleep 0.5
    fi
fi

# Detect where blink is currently installed
INSTALLED_PATH=""
if [ -f "/usr/local/bin/blink" ]; then
    INSTALLED_PATH="/usr/local/bin/blink"
    INSTALL_DIR="/usr/local/bin"
    USE_SUDO=true
elif [ -f "$HOME/.local/bin/blink" ]; then
    INSTALLED_PATH="$HOME/.local/bin/blink"
    INSTALL_DIR="$HOME/.local/bin"
    USE_SUDO=false
elif [ -f "$HOME/.cargo/bin/blink" ]; then
    INSTALLED_PATH="$HOME/.cargo/bin/blink"
    INSTALL_DIR="$HOME/.cargo/bin"
    USE_SUDO=false
else
    echo "Warning: Could not find existing blink installation."
    echo "Falling back to standard installation method..."
    USE_SUDO=true
    INSTALL_DIR="/usr/local/bin"
fi

# Install the updated binary
if [ "$USE_SUDO" = true ] && command -v sudo &> /dev/null; then
    echo "Installing updated blink to $INSTALL_DIR (requires sudo)..."
    sudo cp target/release/blink "$INSTALL_DIR/blink"
    sudo chmod +x "$INSTALL_DIR/blink"
    echo "Blink updated successfully in $INSTALL_DIR"
elif [ "$USE_SUDO" = false ]; then
    echo "Installing updated blink to $INSTALL_DIR..."
    mkdir -p "$INSTALL_DIR"
    cp target/release/blink "$INSTALL_DIR/blink"
    chmod +x "$INSTALL_DIR/blink"
    echo "Blink updated successfully in $INSTALL_DIR"
else
    # Fallback: try to install to user's local bin
    LOCAL_BIN="$HOME/.local/bin"
    mkdir -p "$LOCAL_BIN"
    echo "Installing updated blink to $LOCAL_BIN..."
    cp target/release/blink "$LOCAL_BIN/blink"
    chmod +x "$LOCAL_BIN/blink"
    echo "Blink updated successfully in $LOCAL_BIN"
    
    # Check if ~/.local/bin is in PATH
    if [[ ":$PATH:" != *":$LOCAL_BIN:"* ]]; then
        echo ""
        echo "Note: $LOCAL_BIN is not in your PATH"
        echo "Add this line to your ~/.bashrc or ~/.zshrc:"
        echo "export PATH=\"\$HOME/.local/bin:\$PATH\""
    fi
fi

echo ""
echo "=== Update complete! ==="
echo "You can now run 'blink' with the updated version."
