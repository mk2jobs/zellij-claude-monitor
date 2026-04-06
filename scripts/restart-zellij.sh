#!/bin/bash
# Zellij 세션 재시작 + WASM 플러그인 캐시 클리어
# Usage: ./scripts/restart-zellij.sh [layout-file]

set -euo pipefail

LAYOUT="${1:-$(dirname "$0")/../layouts/claude.kdl}"
CACHE_DIR="$HOME/Library/Caches/org.Zellij-Contributors.Zellij"
PLUGIN_SRC="$(dirname "$0")/../target/wasm32-wasip1/release/zellij-claude-monitor.wasm"
PLUGIN_DST="$HOME/.config/zellij/plugins/zellij-claude-monitor.wasm"
SCRIPT_SRC="$(dirname "$0")/monitor-data.py"
SCRIPT_DST="$HOME/.config/zellij/plugins/monitor-data.py"

echo "=== Zellij Monitor 재시작 ==="

# 1. 현재 세션 이름 확인
CURRENT_SESSION=$(zellij list-sessions 2>/dev/null | grep "ATTACHED" | awk '{print $1}' | head -1)

# 2. WASM 빌드 (변경 있으면)
if [ -f "$PLUGIN_SRC" ]; then
    echo "→ WASM 배포: $(basename "$PLUGIN_SRC")"
    cp "$PLUGIN_SRC" "$PLUGIN_DST"
else
    echo "→ WASM 빌드..."
    (cd "$(dirname "$0")/.." && cargo build --release --target wasm32-wasip1)
    cp "$PLUGIN_SRC" "$PLUGIN_DST"
fi

# 3. Python 스크립트 배포
if [ -f "$SCRIPT_SRC" ]; then
    echo "→ 스크립트 배포: monitor-data.py"
    cp "$SCRIPT_SRC" "$SCRIPT_DST"
fi

# 4. 플러그인 캐시 전체 삭제
# Zellij는 세션별 UUID 디렉토리 + 버전별 컴파일 캐시를 모두 사용
if [ -d "$CACHE_DIR" ]; then
    echo "→ 플러그인 캐시 전체 삭제: $CACHE_DIR"
    rm -rf "$CACHE_DIR"
fi

# 5. 현재 세션 종료
if [ -n "$CURRENT_SESSION" ]; then
    echo "→ 현재 세션 종료: $CURRENT_SESSION"
    zellij kill-session "$CURRENT_SESSION" 2>/dev/null || true
    sleep 0.5
fi

# 6. 죽은 세션 정리
DEAD_SESSIONS=$(zellij list-sessions 2>/dev/null | grep "EXITED" | awk '{print $1}')
if [ -n "$DEAD_SESSIONS" ]; then
    echo "→ 죽은 세션 정리: $(echo "$DEAD_SESSIONS" | wc -l | tr -d ' ')개"
    zellij delete-all-sessions -y 2>/dev/null || true
fi

# 7. 새 세션 시작
echo "→ 새 세션 시작 (layout: $(basename "$LAYOUT"))"
exec zellij --layout "$LAYOUT"
