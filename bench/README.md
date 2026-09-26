# Benchmark Relay
This relay serves as a baseline control (Track C) for evaluating transport delivery mechanisms of MLS ciphertexts. It is a minimal Axum WebSocket relay that holds incoming connections in memory and rebroadcasts incoming binary frames to all clients subscribed to the same conversation.

## Requirements
- Rust edition 2021
- Cargo

## Building and Running
To build the project in release mode:
```bash
cd bench
cargo build --release
```

To run the project:
```bash
./target/release/relay
```
The relay binds to `127.0.0.1:8080`. You can configure the port by setting the `PORT` environment variable (e.g., `PORT=8081 ./target/release/relay`).

## Generator Usage
The generator is a load testing tool designed to test the relay and compute performance metrics.

### Building and Running the Generator
The generator is built with the workspace:
```bash
cd bench
cargo build --release
```

Ensure the relay is running, then run the generator.

### Scenarios
Here are five example invocations for common scenarios:

1. **Baseline** (10 receivers, 1 KB, 1 msg/s, 60s):
```bash
./target/release/generator \
  --transport ws \
  --url ws://127.0.0.1:8080 \
  --conversation baseline \
  --receivers 10 \
  --messages 60 \
  --payload-bytes 1024 \
  --rate 1 \
  --warmup-secs 5 \
  --output baseline.jsonl
```

2. **Small group** (50 receivers, 1 KB, 10 msg/s, 120s):
```bash
./target/release/generator \
  --transport ws \
  --url ws://127.0.0.1:8080 \
  --conversation small_group \
  --receivers 50 \
  --messages 1200 \
  --payload-bytes 1024 \
  --rate 10 \
  --warmup-secs 5 \
  --output small_group.jsonl
```

3. **Large group** (500 receivers, 1 KB, 10 msg/s, 120s):
```bash
./target/release/generator \
  --transport ws \
  --url ws://127.0.0.1:8080 \
  --conversation large_group \
  --receivers 500 \
  --messages 1200 \
  --payload-bytes 1024 \
  --rate 10 \
  --warmup-secs 5 \
  --output large_group.jsonl
```

4. **Welcome burst** (50 receivers, 32 KB, 1 msg/s for 10s, 60s total):
```bash
./target/release/generator \
  --transport ws \
  --url ws://127.0.0.1:8080 \
  --conversation welcome_burst \
  --receivers 50 \
  --messages 10 \
  --payload-bytes 32768 \
  --rate 1 \
  --warmup-secs 5 \
  --output welcome_burst.jsonl
```

5. **Mixed** (100 receivers, mixed payload sizes, 10 msg/s, 300s):
*(Note: Mixed payload logic is currently out of scope and requires future extensions. A placeholder uniform run is shown below).*
```bash
./target/release/generator \
  --transport ws \
  --url ws://127.0.0.1:8080 \
  --conversation mixed \
  --receivers 100 \
  --messages 3000 \
  --payload-bytes 1024 \
  --rate 10 \
  --warmup-secs 5 \
  --output mixed.jsonl
```

### Analyzing Metrics
After running the generator, you can parse the JSONL output using the provided script:
```bash
python3 scripts/analyze.py results.jsonl
```

## API Endpoints

### `GET /health`
A health check endpoint that returns HTTP 200 with the body `ok`.
```bash
curl -s -o /dev/null -w "%{http_code}\n" http://127.0.0.1:8080/health
```

### `GET /ws/{conversation_id}`
Establishes a WebSocket upgrade for the given conversation ID. Any binary messages sent to this connection are fanned out to all other WebSocket connections open for this conversation ID.

## Smoke-Test

### Option A: `websocat`
Using two terminals, connect to the same conversation.

Terminal 1:
```bash
websocat -b ws://127.0.0.1:8080/ws/test
```
Terminal 2:
```bash
websocat -b ws://127.0.0.1:8080/ws/test
```

Type any text into Terminal 1 (which websocat will send) or use a file/pipe to send binary payload. The binary payload will be fanned out and will appear in Terminal 2. If you connect a third terminal to `ws://127.0.0.1:8080/ws/other`, you will not receive messages sent on `/ws/test`.

### Option B: Node.js (ws)

```javascript
const WebSocket = require('ws');

const ws1 = new WebSocket('ws://127.0.0.1:8080/ws/test');
const ws2 = new WebSocket('ws://127.0.0.1:8080/ws/test');

ws2.on('message', function message(data) {
  console.log('ws2 received: %s', data);
});

ws1.on('open', function open() {
  ws1.send(Buffer.from('hello from ws1'));
});
```
## PocketBase Setup

The benchmark uses PocketBase as Track A (SSE for server-to-client delivery and REST for client-to-server sends). The deployment is managed via Docker Compose and uses PocketBase version 0.23+.

### Starting PocketBase
To start the PocketBase instance in the background:
```bash
docker compose -f bench/pocketbase/docker-compose.yml up -d
```

### Provisioning Collections
Before running the setup script, you can configure the admin credentials using environment variables. A template is provided in `bench/pocketbase/.env.example`:
```bash
export PB_ADMIN_EMAIL=admin@example.com
export PB_ADMIN_PASSWORD=changeme123
```

Run the setup script to initialize the admin account and provision the required `bench_messages` collection:
```bash
bash bench/pocketbase/setup.sh
```

### Verification
To verify the setup was successful, you can check that the collection exists:
```bash
curl http://127.0.0.1:8090/api/collections/bench_messages/records
```

### Stopping and Resetting
To stop the PocketBase instance:
```bash
docker compose -f bench/pocketbase/docker-compose.yml down
```

To stop the instance and completely reset the database data (useful if you need a fresh start):
```bash
docker compose -f bench/pocketbase/docker-compose.yml down -v
```

## Running the SSE Baseline

The SSE transport implementation connects to a PocketBase instance to benchmark Server-Sent Events for server-to-client delivery and REST POST for client-to-server sending. Note that because sends go over REST, the round-trip time is higher than WebSocket by design.

**Prerequisite:** The PocketBase Docker container from Task 3.1 must be running.

### Example Baseline Invocation

```bash
./target/release/generator \
  --transport sse \
  --pb-url http://127.0.0.1:8090 \
  --pb-conversation bench \
  --receivers 10 \
  --messages 100 \
  --payload-bytes 1024 \
  --rate 10 \
  --warmup-secs 2 \
  --output sse-results.jsonl
```

### Analyzing Metrics

Run the analysis script just like you would for WebSocket results:

```bash
python3 scripts/analyze.py sse-results.jsonl
```

## Sockudo Setup

The benchmark uses Sockudo as Track B, providing a high-performance Rust WebSocket server implementing the Pusher protocol.

### Starting Sockudo
To start the Sockudo instance in the background:
```bash
docker compose -f bench/sockudo/docker-compose.yml up -d
```

### Verification
To verify the setup was successful, you can run:
```bash
curl http://127.0.0.1:6001/up/bench-app
```
It should return `{"status":"ok"}`.

### App Credentials
The instance is configured with default app credentials via the `.env` template:
```
SOCKUDO_DEFAULT_APP_ID=bench-app
SOCKUDO_DEFAULT_APP_KEY=bench-key
SOCKUDO_DEFAULT_APP_SECRET=bench-secret
```
These match the defaults used in the generator.

## Running the Sockudo Baseline

The Sockudo transport implements the Pusher protocol. Note an intentional asymmetry: the WebSocket connection is subscribe-only for public channels, so all published messages go through the HTTP POST endpoint (similar to the SSE transport).

### Example Baseline Invocation

```bash
./target/release/generator \
  --transport sockudo \
  --sockudo-url http://127.0.0.1:6001 \
  --sockudo-ws-url ws://127.0.0.1:6001 \
  --sockudo-app-id bench-app \
  --sockudo-app-key bench-key \
  --sockudo-app-secret bench-secret \
  --sockudo-channel bench-channel \
  --receivers 10 \
  --messages 100 \
  --payload-bytes 1024 \
  --rate 10 \
  --warmup-secs 2 \
  --output sockudo-results.jsonl
```

### Analyzing Metrics

Run the analysis script just like you would for WebSocket or SSE results, providing side-by-side comparison capabilities if needed:

```bash
python3 scripts/analyze.py sockudo-results.jsonl
```

## Benchmark Execution

### Log Rotation
To prevent excessive disk usage, the `run-matrix.sh` runner truncates `relay.log` before each individual scenario on the WS transport track. The relay itself only emits `INFO` level logs by default, which ensures that per-message broadcasts are not logged (they require `DEBUG` level).

### Run Manifest
The runner natively maintains a JSON state file called `bench/results/run-manifest.json` across all benchmark runs. It logs the individual execution statuses of all 15 permutations (5 scenarios across 3 transports). Potential statuses include `pending`, `running`, `complete`, `empty`, and `failed: <reason>`. The summary tool `compare.py` reads this to determine completion without incorrectly marking zero-yield runs as "not run" (`N/A`).
