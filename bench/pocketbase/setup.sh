#!/bin/bash
set -e

PB_ADMIN_EMAIL="${PB_ADMIN_EMAIL:-admin@example.com}"
PB_ADMIN_PASSWORD="${PB_ADMIN_PASSWORD:-changeme123}"
PB_URL="${PB_URL:-http://127.0.0.1:8090}"

echo "Waiting for PocketBase at $PB_URL/api/health..."
TIMEOUT=60
I=0
while [ $I -lt $TIMEOUT ]; do
  if curl -sf "$PB_URL/api/health" > /dev/null; then
    echo "PocketBase is up!"
    break
  fi
  sleep 1
  I=$((I+1))
done

if [ $I -eq $TIMEOUT ]; then
  echo "Error: PocketBase did not become healthy within 60 seconds."
  exit 1
fi

echo "Authenticating as admin..."
# First check if we can auth
TOKEN=$(curl -s -X POST "$PB_URL/api/collections/_superusers/auth-with-password" \
  -H "Content-Type: application/json" \
  -d "{\"identity\":\"$PB_ADMIN_EMAIL\",\"password\":\"$PB_ADMIN_PASSWORD\"}" | jq -r '.token')

if [ "$TOKEN" == "null" ] || [ -z "$TOKEN" ]; then
  echo "Failed to authenticate. Creating admin via container exec..."

  # Try to create first admin using docker exec since PocketBase 0.23+ requires a token for records endpoint
  # or requires opening the web UI to install
  CONTAINER_ID=$(docker ps -qf "ancestor=ghcr.io/muchobien/pocketbase:latest")
  if [ -n "$CONTAINER_ID" ]; then
    docker exec "$CONTAINER_ID" /usr/local/bin/pocketbase superuser upsert "$PB_ADMIN_EMAIL" "$PB_ADMIN_PASSWORD" --dir /pb_data >/dev/null 2>&1 || true
  else
    echo "Could not find pocketbase container to create admin."
  fi

  # Try auth again
  TOKEN=$(curl -s -X POST "$PB_URL/api/collections/_superusers/auth-with-password" \
    -H "Content-Type: application/json" \
    -d "{\"identity\":\"$PB_ADMIN_EMAIL\",\"password\":\"$PB_ADMIN_PASSWORD\"}" | jq -r '.token')

  if [ "$TOKEN" == "null" ] || [ -z "$TOKEN" ]; then
    # Try the old admin API for backwards compatibility
    TOKEN=$(curl -s -X POST "$PB_URL/api/admins/auth-with-password" \
      -H "Content-Type: application/json" \
      -d "{\"identity\":\"$PB_ADMIN_EMAIL\",\"password\":\"$PB_ADMIN_PASSWORD\"}" | jq -r '.token')

    if [ "$TOKEN" == "null" ] || [ -z "$TOKEN" ]; then
        echo "Error: Could not authenticate."
        exit 1
    fi
  fi
fi

echo "Checking if bench_messages collection exists..."
STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$PB_URL/api/collections/bench_messages" -H "Authorization: Bearer $TOKEN")

if [ "$STATUS" == "200" ]; then
  echo "Collection 'bench_messages' already exists. Skipping creation."
  exit 0
fi

echo "Creating 'bench_messages' collection..."

# Source of truth payload embedded in script, avoiding exact ID matches that PB rejects and separating from the schema doc.
PAYLOAD='{
  "name": "bench_messages",
  "type": "base",
  "system": false,
  "fields": [
    {
      "system": false,
      "name": "conversation",
      "type": "text",
      "required": true,
      "presentable": false,
      "unique": false,
      "options": {
        "min": null,
        "pattern": ""
      }
    },
    {
      "system": false,
      "name": "client_message_id",
      "type": "number",
      "required": true,
      "presentable": false,
      "unique": false,
      "options": {
        "min": null,
        "max": null,
        "noDecimal": false
      }
    },
    {
      "system": false,
      "name": "sender_id",
      "type": "number",
      "required": true,
      "presentable": false,
      "unique": false,
      "options": {
        "min": null,
        "max": null,
        "noDecimal": false
      }
    },
    {
      "system": false,
      "name": "payload_b64",
      "type": "json",
      "required": true,
      "presentable": false,
      "unique": false,
      "options": {
        "maxSize": 2000000
      }
    }
  ],
  "indexes": [
    "CREATE INDEX `idx_conversation` ON `bench_messages` (`conversation`)",
    "CREATE INDEX `idx_client_message_id` ON `bench_messages` (`client_message_id`)"
  ],
  "listRule": "",
  "viewRule": "",
  "createRule": "",
  "updateRule": "",
  "deleteRule": "",
  "options": {}
}'

RESULT=$(curl -s -w "\n%{http_code}" -X POST "$PB_URL/api/collections" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "$PAYLOAD")

HTTP_CODE=$(echo "$RESULT" | tail -n1)
BODY=$(echo "$RESULT" | head -n -1)

if [ "$HTTP_CODE" != "200" ]; then
  echo "Error creating collection: HTTP $HTTP_CODE"
  echo "$BODY"
  exit 1
fi

echo "Collection 'bench_messages' created successfully."
