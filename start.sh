#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BACKEND_DIR="$SCRIPT_DIR/backend_v2"
FRONTEND_DIR="$SCRIPT_DIR/frontend_v3"
CONFIG_FILE="$FRONTEND_DIR/src/lib/config.ts"

DB_CONTAINER="y-postgres"
DB_USER="chatapp_user"
DB_PASS="chatapp_password"
DB_NAME="chatapp"

BACKEND_PORT=8082

cleanup() {
    echo ""
    echo "shutting down..."
    kill $BACKEND_PID 2>/dev/null || true
    kill $TUNNEL_PID 2>/dev/null || true
    docker stop $DB_CONTAINER 2>/dev/null || true
    cat > "$CONFIG_FILE" << 'EOF'
pub const BACKEND_URL: &str = "http://localhost:8082";
pub const WS_URL: &str = "ws://localhost:8082/ws";
EOF
    echo "bye"
}
trap cleanup EXIT INT TERM

echo "==> postgres"
if ! docker ps --format '{{.Names}}' | grep -q "^${DB_CONTAINER}$"; then
    if docker ps -a --format '{{.Names}}' | grep -q "^${DB_CONTAINER}$"; then
        docker start $DB_CONTAINER
    else
        docker run -d \
            --name $DB_CONTAINER \
            -e POSTGRES_USER=$DB_USER \
            -e POSTGRES_PASSWORD=$DB_PASS \
            -e POSTGRES_DB=$DB_NAME \
            -p 5432:5432 \
            postgres:16-alpine
    fi
fi

echo "   waiting for postgres..."
for i in $(seq 1 30); do
    if docker exec $DB_CONTAINER pg_isready -U $DB_USER -d $DB_NAME -q 2>/dev/null; then
        break
    fi
    sleep 1
done
echo "   postgres ready"

echo "==> cloudflared tunnel"
TUNNEL_LOG=$(mktemp)
cloudflared tunnel --url http://localhost:$BACKEND_PORT 2>"$TUNNEL_LOG" &
TUNNEL_PID=$!

echo "   waiting for tunnel URL..."
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

WS_URL="${TUNNEL_URL/https:/wss:}/ws"
echo "   tunnel: $TUNNEL_URL"

echo "==> patching config.rs"
cat > "$CONFIG_FILE" << EOF
pub const BACKEND_URL: &str = "$TUNNEL_URL";
pub const WS_URL: &str = "$WS_URL";
EOF

echo "==> building frontend (trunk build)"
cd "$FRONTEND_DIR"
trunk build --release 2>&1 | tail -5

echo "==> building and starting backend (serves frontend too)"
cd "$BACKEND_DIR"
cargo run --release 2>&1 &
BACKEND_PID=$!

echo ""
echo "============================================"
echo "  y-chat is live — share this link:"
echo ""
echo "  $TUNNEL_URL"
echo ""
echo "============================================"
echo ""
echo "ctrl+c to stop everything"

wait $BACKEND_PID
