#!/bin/bash
set -e

echo "Building WASM plugin..."
cargo build --release --target wasm32-wasip1

echo "Deploying to ~/.config/zellij/plugins/"
cp target/wasm32-wasip1/release/zellij-claude-monitor.wasm ~/.config/zellij/plugins/

echo "Deploying layout to ~/.config/zellij/layouts/"
cp layouts/claude.kdl ~/.config/zellij/layouts/claude.kdl

echo "Done."
