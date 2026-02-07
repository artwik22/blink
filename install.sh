#!/bin/bash

# Installation script for Blink
# This script builds and installs blink to your system

set -e

# Check if Rust toolchain is configured
if ! rustc --version &> /dev/null; then
    echo "Rust toolchain not configured. Setting up default stable toolchain..."
    rustup default stable
fi

echo "Building Blink..."
cargo build --release

if [ ! -f "target/release/index" ]; then
    echo "Error: Build failed or binary not found"
    exit 1
fi

# Try to install to system directory (requires sudo)
if command -v sudo &> /dev/null; then
    echo "Installing blink to /usr/local/bin (requires sudo)..."
    sudo cp target/release/index /usr/local/bin/blink
    sudo chmod +x /usr/local/bin/blink
    echo "Blink installed successfully to /usr/local/bin"
    echo "You can now run 'blink' from anywhere in your terminal!"
else
    # Fallback to user's local bin directory
    LOCAL_BIN="$HOME/.local/bin"
    mkdir -p "$LOCAL_BIN"
    echo "Installing blink to $LOCAL_BIN..."
    cp target/release/index "$LOCAL_BIN/blink"
    chmod +x "$LOCAL_BIN/blink"
    echo "Blink installed successfully to $LOCAL_BIN"
    
    # Check if ~/.local/bin is in PATH
    if [[ ":$PATH:" != *":$LOCAL_BIN:"* ]]; then
        echo ""
        echo "Warning: $LOCAL_BIN is not in your PATH"
        echo "Add this line to your ~/.bashrc or ~/.zshrc:"
        echo "export PATH=\"\$HOME/.local/bin:\$PATH\""
        echo ""
        echo "Then run: source ~/.bashrc  (or source ~/.zshrc)"
    else
        echo "You can now run 'blink' from anywhere in your terminal!"
    fi
fi
