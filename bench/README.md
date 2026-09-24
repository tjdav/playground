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