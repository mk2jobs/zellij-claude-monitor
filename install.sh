#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PLUGIN_DIR="$HOME/.config/zellij/plugins"

echo "Building zellij-claude-monitor WASM plugin..."
cd "$SCRIPT_DIR"
cargo build --release

echo "Installing plugin..."
mkdir -p "$PLUGIN_DIR"

WASM_SRC="target/wasm32-wasip1/release/zellij-claude-monitor.wasm"
WASM_DST="$PLUGIN_DIR/zellij-claude-monitor.wasm"
LAYOUT_DIR="$HOME/.config/zellij/layouts"
LAYOUT_SRC="$SCRIPT_DIR/layouts/claude.kdl"

cp "$WASM_SRC" "$WASM_DST"
cp scripts/monitor-data.py "$PLUGIN_DIR/monitor-data.py"
cp scripts/statusline.py "$PLUGIN_DIR/statusline.py"

if [ -f "$LAYOUT_SRC" ]; then
    mkdir -p "$LAYOUT_DIR"
    cp "$LAYOUT_SRC" "$LAYOUT_DIR/claude.kdl"
    echo "  - claude.kdl → $LAYOUT_DIR/"
fi

# cleanup old plugin name
rm -f "$PLUGIN_DIR/claude_dashboard.wasm"

echo "Installed to $PLUGIN_DIR/"
echo "  - zellij-claude-monitor.wasm"
echo "  - monitor-data.py"
echo "  - statusline.py"
echo "Done!"
