#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

FRONTEND_DIR="$SCRIPT_DIR/frontend"
CONFIG_FILE="$FRONTEND_DIR/src/lib/config.ts"

BACKEND_PORT=8082
FRONTEND_PORT=5173

cleanup() {
    echo ""
    echo "Shutting down..."

    kill $FRONTEND_PID 2>/dev/null || true
    kill $TUNNEL_BACK_PID 2>/dev/null || true
    kill $TUNNEL_FRONT_PID 2>/dev/null || true
    docker compose down 2>/dev/null || true

    cat > "$CONFIG_FILE" << 'EOF'
export const BACKEND_URL = 'http://localhost:8082';
export const WS_URL = `ws://localhost:8082/ws`;
EOF

    echo "Done."
}
trap cleanup EXIT INT TERM

echo "==> Starting backend containers..."
docker compose up -d

echo "==> Starting backend tunnel..."
BACK_LOG=$(mktemp)
cloudflared tunnel --url http://localhost:$BACKEND_PORT 2>"$BACK_LOG" &
TUNNEL_BACK_PID=$!

echo "   Waiting for backend tunnel URL..."
BACKEND_URL=""
for i in $(seq 1 60); do
    BACKEND_URL=$(grep -oP 'https://[a-z0-9\-]+\.trycloudflare\.com' "$BACK_LOG" | head -1 || true)
    [ -n "$BACKEND_URL" ] && break
    sleep 1
done

if [ -z "$BACKEND_URL" ]; then
    echo "Failed to get backend tunnel URL"
    exit 1
fi

WS_URL="${BACKEND_URL/https:/wss:}/ws"

echo "==> Patching frontend config.ts"
cat > "$CONFIG_FILE" << EOF
export const BACKEND_URL = '${BACKEND_URL}';
export const WS_URL = \`${WS_URL}\`;
EOF

echo "==> Starting frontend dev server..."
cd "$FRONTEND_DIR"
bun install
bun run dev &
FRONTEND_PID=$!

echo "   Waiting for frontend to start..."
sleep 5

echo "==> Starting frontend tunnel..."
FRONT_LOG=$(mktemp)
cloudflared tunnel --url http://localhost:$FRONTEND_PORT 2>"$FRONT_LOG" &
TUNNEL_FRONT_PID=$!

echo "   Waiting for frontend tunnel URL..."
FRONTEND_URL=""
for i in $(seq 1 60); do
    FRONTEND_URL=$(grep -oP 'https://[a-z0-9\-]+\.trycloudflare\.com' "$FRONT_LOG" | head -1 || true)
    [ -n "$FRONTEND_URL" ] && break
    sleep 1
done

if [ -z "$FRONTEND_URL" ]; then
    echo "Failed to get frontend tunnel URL"
    exit 1
fi

cd "$SCRIPT_DIR"

echo ""
echo "============================================"
echo "  y-chat is live — share this link:"
echo ""
echo "  $FRONTEND_URL"
echo ""
echo "============================================"
echo ""
echo "Press Ctrl+C to stop everything"

wait $FRONTEND_PID
