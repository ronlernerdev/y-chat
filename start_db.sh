#!/usr/bin/env bash
set -e

DB_CONTAINER="y-postgres"
DB_USER="chatapp_user"
DB_PASS="chatapp_password"
DB_NAME="chatapp"

cleanup() {
    echo ""
    echo "stopping postgres..."
    docker stop $DB_CONTAINER 2>/dev/null || true
    echo "done"
}
trap cleanup INT TERM

echo "==> ensuring postgres container is running"

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

echo "==> waiting for postgres to become ready"

for i in $(seq 1 30); do
    if docker exec $DB_CONTAINER pg_isready -U $DB_USER -d $DB_NAME -q 2>/dev/null; then
        echo "postgres is ready"
        exit 0
    fi
    sleep 1
done

echo "ERROR: postgres did not become ready in time"
exit 1
