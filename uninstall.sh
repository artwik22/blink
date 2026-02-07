#!/bin/bash

# Uninstallation script for Blink
# This script removes blink from your system

set -e

echo "Uninstalling Blink..."

# Check and remove from /usr/local/bin
if [ -f "/usr/local/bin/blink" ]; then
    if command -v sudo &> /dev/null; then
        echo "Removing blink from /usr/local/bin (requires sudo)..."
        sudo rm -f /usr/local/bin/blink
        echo "Removed /usr/local/bin/blink"
    else
        echo "Warning: Cannot remove /usr/local/bin/blink without sudo"
    fi
fi

# Check and remove from ~/.local/bin
LOCAL_BIN="$HOME/.local/bin"
if [ -f "$LOCAL_BIN/blink" ]; then
    echo "Removing blink from $LOCAL_BIN..."
    rm -f "$LOCAL_BIN/blink"
    echo "Removed $LOCAL_BIN/blink"
fi

echo ""
echo "Blink has been uninstalled successfully!"
