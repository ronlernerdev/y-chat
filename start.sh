#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

cleanup() {
    echo ""
    echo "Shutting down..."
    kill $TUNNEL_PID 2>/dev/null || true
    docker compose down
    echo "Bye!"
}
trap cleanup EXIT INT TERM

echo "==> Starting docker containers..."
docker compose up -d --build

echo "==> Starting cloudflared tunnel..."
TUNNEL_LOG=$(mktemp)
cloudflared tunnel --url http://localhost:8082 2>"$TUNNEL_LOG" &
TUNNEL_PID=$!

echo "   Waiting for tunnel URL..."
TUNNEL_URL=""
for i in $(seq 1 60); do
    TUNNEL_URL=$(grep -oP 'https://[a-z0-9\-]+\.trycloudflare\.com' "$TUNNEL_LOG" 2>/dev/null | head -1 || true)
    if [ -n "$TUNNEL_URL" ]; then
        break
    fi
    sleep 1
done

if [ -z "$TUNNEL_URL" ]; then
    echo "ERROR: could not get tunnel URL after 60s"
    cat "$TUNNEL_LOG"
    exit 1
fi

echo ""
echo "============================================"
echo "  y-chat is live — share this link:"
echo ""
echo "  $TUNNEL_URL"
echo ""
echo "============================================"
echo ""
echo "Press Ctrl+C to stop everything"

wait $TUNNEL_PID
