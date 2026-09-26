#!/bin/bash

set -e

# Change to repo root
cd "$(dirname "$0")/../.."
REPO_ROOT=$(pwd)

# Trap INT and TERM for cleanup
cleanup() {
    echo "Caught signal, cleaning up..."
    if [ -n "$RELAY_PID" ] && kill -0 "$RELAY_PID" 2>/dev/null; then
        kill "$RELAY_PID"
        wait "$RELAY_PID" 2>/dev/null || true
    fi
    if [ -n "$COLLECTOR_PID" ] && kill -0 "$COLLECTOR_PID" 2>/dev/null; then
        kill "$COLLECTOR_PID"
        wait "$COLLECTOR_PID" 2>/dev/null || true
    fi
    docker compose -f bench/pocketbase/docker-compose.yml down >/dev/null 2>&1 || true
    docker compose -f bench/sockudo/docker-compose.yml down >/dev/null 2>&1 || true
    exit 1
}
trap cleanup INT TERM

FORCE=0
RESET=0

for arg in "$@"; do
    case $arg in
        --force) FORCE=1 ;;
        --reset) RESET=1 ;;
        --reset=yes) RESET=1; FORCE_RESET=1 ;;
        *) ;;
    esac
done

if [ "$RESET" -eq 1 ]; then
    if [ -z "$FORCE_RESET" ]; then
        read -p "Resetting Docker volumes. All accumulated data will be lost. Continue? [y/N] " confirm
        if [[ ! "$confirm" =~ ^[Yy]$ ]]; then
            echo "Aborting."
            exit 1
        fi
    fi
    docker compose -f bench/pocketbase/docker-compose.yml down -v
    docker compose -f bench/sockudo/docker-compose.yml down -v
fi

# Run preflight checks
bash bench/scripts/preflight.sh || exit 1

mkdir -p bench/results

# Parse skip env vars
IFS=',' read -ra SKIP_T <<< "${SKIP_TRANSPORTS:-}"
IFS=',' read -ra SKIP_S <<< "${SKIP_SCENARIOS:-}"

skip_transport() {
    local t=$1
    for i in "${SKIP_T[@]}"; do
        if [ "$i" == "$t" ]; then
            return 0
        fi
    done
    return 1
}

skip_scenario() {
    local s=$1
    for i in "${SKIP_S[@]}"; do
        if [ "$i" == "$s" ]; then
            return 0
        fi
    done
    return 1
}

update_manifest() {
  local transport="$1" scenario="$2" status="$3"
  local manifest="bench/results/run-manifest.json"
  local tmp="${manifest}.tmp"

  if [ ! -f "$manifest" ]; then
    echo '{}' > "$manifest"
  fi

  jq --arg t "$transport" --arg s "$scenario" --arg st "$status" \
    '.[$t] = (.[$t] // {}) | .[$t][$s] = $st' \
    "$manifest" > "$tmp" && mv "$tmp" "$manifest"
}

declare -A SCENARIOS=(
    ["baseline"]="--receivers 10 --payload-bytes 1024 --rate 1 --messages 60 --warmup-secs 5"
    ["small-group"]="--receivers 50 --payload-bytes 1024 --rate 10 --messages 1200 --warmup-secs 10"
    ["large-group"]="--receivers 500 --payload-bytes 1024 --rate 10 --messages 1200 --warmup-secs 15"
    ["welcome-burst"]="--receivers 50 --payload-bytes 32768 --rate 1 --messages 60 --warmup-secs 5"
    ["mixed"]="--receivers 100 --payload-bytes 1024 --rate 10 --messages 3000 --warmup-secs 10 --payload-mix 1024:70,8192:20,32768:10"
)

TRANSPORTS=("ws" "sse" "sockudo")
SCENARIO_KEYS=("baseline" "small-group" "large-group" "welcome-burst" "mixed")

for transport in "${TRANSPORTS[@]}"; do
    if skip_transport "$transport"; then
        echo "Skipping transport: $transport"
        continue
    fi

    echo "=== Setting up transport: $transport ==="
    TARGET_ID=""
    if [ "$transport" == "ws" ]; then
        RUST_LOG=info ./bench/target/release/relay > bench/relay.log 2>&1 &
        RELAY_PID=$!
        TARGET_ID=$RELAY_PID
        sleep 2
    elif [ "$transport" == "sse" ]; then
        docker compose -f bench/pocketbase/docker-compose.yml up -d
        TARGET_ID=$(docker compose -f bench/pocketbase/docker-compose.yml ps -q pocketbase)
        echo "Waiting for PocketBase..."
        timeout 60 bash -c 'until curl -s http://127.0.0.1:8090/api/health > /dev/null; do sleep 1; done'
        echo "Running PocketBase setup..."
        bash bench/pocketbase/setup.sh || true
    elif [ "$transport" == "sockudo" ]; then
        docker compose -f bench/sockudo/docker-compose.yml up -d
        TARGET_ID=$(docker compose -f bench/sockudo/docker-compose.yml ps -q sockudo)
        echo "Waiting for Sockudo..."
        timeout 60 bash -c 'until curl -f -s http://127.0.0.1:6001/up/bench-app > /dev/null; do sleep 1; done'
    fi

    for scenario in "${SCENARIO_KEYS[@]}"; do
        if skip_scenario "$scenario"; then
            echo "Skipping scenario: $scenario"
            continue
        fi

        OUT_JSON="bench/results/${transport}-${scenario}.jsonl"
        OUT_CSV="bench/results/${transport}-${scenario}-resources.csv"

        if [ -f "$OUT_JSON" ] && [ "$FORCE" -eq 0 ]; then
            echo "Outputs exist for ${transport}-${scenario}, skipping (use --force to override)"
            continue
        fi

        if [ "$transport" == "ws" ]; then
            > bench/relay.log
        fi
        update_manifest "$transport" "$scenario" "running"
        echo "--- Running scenario: ${scenario} ---"
        rm -f "$OUT_JSON" "$OUT_CSV"

        # Start resource collector
        bash bench/scripts/collect-resources.sh "$TARGET_ID" "$OUT_CSV" 1 &
        COLLECTOR_PID=$!

        set +e
        ARGS=${SCENARIOS[$scenario]}

        # Override URL options using env vars if provided (fallback to defaults)
        EXTRA_ARGS=""
        if [ "$transport" == "sse" ]; then
            EXTRA_ARGS="--pb-url ${PB_URL:-http://127.0.0.1:8090}"
        elif [ "$transport" == "sockudo" ]; then
            EXTRA_ARGS="--sockudo-url ${SOCKUDO_URL:-http://127.0.0.1:6001} --sockudo-ws-url ${SOCKUDO_WS_URL:-ws://127.0.0.1:6001}"
        fi

        # Convert ARGS string to array so flags like --payload-mix work correctly
        IFS=' ' read -ra ARGS_ARRAY <<< "$ARGS"

        ./bench/target/release/generator \
            --transport "$transport" \
            --conversation "${transport}-${scenario}" \
            --pb-conversation "${transport}-${scenario}" \
            --output "$OUT_JSON" \
            $EXTRA_ARGS \
            "${ARGS_ARRAY[@]}"
        GEN_EXIT=$?
        set -e

        if kill -0 "$COLLECTOR_PID" 2>/dev/null; then
            kill -TERM "$COLLECTOR_PID"
            wait "$COLLECTOR_PID" 2>/dev/null || true
        fi

        if [ $GEN_EXIT -ne 0 ]; then
            echo "ERROR: Generator failed for ${transport}-${scenario}"
            rm -f "$OUT_JSON"
            update_manifest "$transport" "$scenario" "failed: exit $GEN_EXIT"
        else
            if [ ! -f "$OUT_JSON" ] || ! grep -q '"latency_us"' "$OUT_JSON"; then
                update_manifest "$transport" "$scenario" "empty"
            else
                update_manifest "$transport" "$scenario" "complete"
            fi
        fi
        sync
    done

    echo "=== Tearing down transport: $transport ==="
    if [ "$transport" == "ws" ]; then
        kill "$RELAY_PID"
        wait "$RELAY_PID" 2>/dev/null || true
    elif [ "$transport" == "sse" ]; then
        docker compose -f bench/pocketbase/docker-compose.yml down
    elif [ "$transport" == "sockudo" ]; then
        docker compose -f bench/sockudo/docker-compose.yml down
    fi
done

# Post-run hygiene
if find . -type d -name pb_data -not -path "*/node_modules/*" -not -path "*/target/*" | grep -q .; then
  echo "WARN: pb_data/ appeared during the run. Check the compose file."
fi

echo "All runs completed successfully."