#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PLUGIN_DIR="$HOME/.config/zellij/plugins"

echo "Building claude-dashboard WASM plugin..."
cd "$SCRIPT_DIR"
cargo build --release

echo "Installing plugin..."
mkdir -p "$PLUGIN_DIR"
cp target/wasm32-wasip1/release/claude_dashboard.wasm "$PLUGIN_DIR/"

echo "Installed to $PLUGIN_DIR/claude_dashboard.wasm"
echo "Done!"
