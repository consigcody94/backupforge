#!/bin/bash
# Quick start script for BackupForge Dashboard

set -e

echo "╔════════════════════════════════════════════════════════════╗"
echo "║         BackupForge Dashboard - Quick Start                ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

# Check for Rust
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust not found. Installing..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
fi

echo "✓ Rust found: $(rustc --version)"
echo ""

# Create storage directory
STORAGE_DIR="${BACKUPFORGE_STORAGE:-$HOME/.local/share/backupforge/storage}"
mkdir -p "$STORAGE_DIR"
echo "✓ Storage directory: $STORAGE_DIR"
echo ""

# Build the server
echo "Building BackupForge server..."
echo "(This may take a few minutes on first run)"
echo ""

cargo build --release --bin backupforge-server

if [ $? -eq 0 ]; then
    echo ""
    echo "✅ Build successful!"
    echo ""
else
    echo "❌ Build failed. Please check the errors above."
    exit 1
fi

# Start the server
PORT="${PORT:-8080}"
echo "╔════════════════════════════════════════════════════════════╗"
echo "║  Starting BackupForge Dashboard on http://localhost:$PORT  ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""
echo "Dashboard:  http://localhost:$PORT"
echo "API:        http://localhost:$PORT/api"
echo "Health:     http://localhost:$PORT/health"
echo ""
echo "Press Ctrl+C to stop the server"
echo ""

export BACKUPFORGE_STORAGE="$STORAGE_DIR"
exec ./target/release/backupforge-server
