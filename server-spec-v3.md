# Server Specification v3.0.3

**Status:** Final — source of truth for all v3.0 implementation tasks.
**Supersedes:** Server Specification v3.0.2, which superseded v3.0.1, which superseded v3.0. This is a clean break from v2.0. No v2.0 content is carried forward by reference; every section is restated in v3.0 terms.
**Scope:** Server-side contract only. Client implementation is out of scope and is covered by Client Specification v2.0.
**Stack:** Sockudo + Axum + SQLite + OPAQUE (opaque-ke 4.0.1) + VOPRF (voprf 0.5.0) + ALTCHA + S3 (or filesystem).
**Deployment:** Single VPS, Docker-based, self-hosted. Two containers: server (Axum + static SPA hosting) and sockudo. TLS terminates at a reverse proxy.

No migration path is required. Nothing has been deployed.

---

## 1. Product Summary

A self-hosted, end-to-end encrypted group messaging system with MLS-grade forward secrecy and post-compromise security. The server is a relay: it stores and forwards ciphertext, and never holds decryption keys.

### 1.1 What the server sees

- Room membership.
- Message sender (a stable user ID or bot ID).
- Message timing.
- Message size.
- Attachment size.
- Reaction attribution and timing.
- Call records: initiator, start time, end time, duration. Not per-participant attendance.
- Session existence and participant count (disclosed to all room members).
- Push notification room ID and timing.
- A one-way derived lookup token per account.
- Preference ciphertext (unreadable).
- Bot account metadata (display name, avatar, scopes, KT entry, declarations).
- Bot connection status (ephemeral, in-memory, owner-only).
- Publisher public keys per room per epoch (public).
- Read counts per message (aggregate, per message).
- Generated Sockudo app key (public) and app secret (never exposed).

### 1.2 What the server does not see

- Message content.
- Attachment content.
- Decryption keys.
- Plaintext usernames.
- Plaintext display names.
- Plaintext device names.
- Plaintext room metadata.
- Plaintext session metadata.
- Call signaling content.
- Session signaling content.
- Preference values.
- Bot settings values.
- Local message content.
- Secret values.
- Bot command plaintext.
- Extension proxy URLs, request bodies, response bodies.
- Sockudo app secret.

### 1.3 What the server can enumerate

Low-entropy usernames. The operator holds the OPRF key k. The lookup token is deterministic for a given (username, k). The operator can compute tokens for candidate usernames offline and compare them against stored values. This is mathematical, not implementation. High-entropy or generated usernames are not enumerable.

### 1.4 What the server does not persist

- Session occupancy.
- Call participation.
- Bot connection status.
- Observer stream cursors.
- Session signal payloads.
- Call signal payloads.
- Local messages.

### 1.5 Technology layers

| Layer | Technology |
|---|---|
| Delivery | Sockudo (WebSocket, Pusher v7) |
| API | Axum (Rust) |
| Database | SQLite (embedded in the Axum process) |
| Blob storage | S3 (default) or filesystem (fallback) |
| S3 client | rust-s3 |
| MLS engine | Wire CoreCrypto 10.5.2 (client-side only) |
| Auth | OPAQUE (aPAKE) via opaque-ke 4.0.1 |
| Username lookup | VOPRF (Ristretto255-SHA512) via voprf 0.5.0 |
| Bot protection | ALTCHA Proof-of-Work v2 via altcha 0.2.0 |
| Attachment encryption | C2SP chunked encryption (c2sp.org/chunked-encryption) |
| Static SPA hosting | Axum ServeDir with SPA fallback |
| Model hosting | Instance-local, external shared origin, or proxy-with-cache |
| Extension proxy | SSRF-guarded egress gateway for client extensions |
| TLS termination | External reverse proxy (Traefik via Coolify, or Caddy) |

### 1.6 Architectural principles

1. The server does not store plaintext usernames, display names, or device names. Identity is anchored in a one-way derived lookup token.
2. User-scoped state (read state, room order, device names, starred items, preferences, bot settings) is durable and sequence-synced across a user's devices.
3. Message editing, reactions, threading, whisper messages, key transparency, link preview proxy, call signaling, sessions, model hosting, starred items, generic preferences, an extension proxy, headless bots, and a declarative settings/commands surface are included.
4. Ambient presence is refused. Call-scoped and session-scoped co-presence is permitted, disclosed only to participants, and never persisted. Bot connection status is a server-to-bot liveness indicator, not user presence.
5. Extension and bot payloads that are encrypted by the client are opaque to the server. The server defines only the wire shape.
6. Extensions and bots do not implement cryptographic primitives. The client runtime provides typed APIs. This is a client-side architectural constraint, stated in the server spec so the wire contract is stable.
7. Bots are headless clients. They run on the operator's machine, not on the server, and not inside a user's client. Tier guarantees are cryptographic, not policy.
8. Sockudo handles transport. The server authenticates subscriptions; it does not route WebSocket traffic itself.

---

## 2. Scope

### 2.1 In Scope

- OPRF-based identity. Usernames, display names, and device names are opaque to the server.
- Recovery flow. Server-generated recovery codes; OPAQUE re-registration.
- User-scoped channels. `private-user-{user_id}` with durable, sequence-synced state.
- Bot-scoped channels. `private-bot-{bot_id}` with durable command delivery and lifecycle events.
- Multi-device sync. Read state, room order, device names, starred items, preferences, bot settings.
- Generic preferences endpoint. Client-defined key/value preferences synced per user, encrypted client-side.
- Starred items.
- Message editing.
- Reactions.
- Threading.
- Whisper messages.
- Room metadata updates.
- Room avatars.
- Member pagination. Merged user and bot entries.
- Retention change preview.
- MLS adds coordination. `pending_mls_adds` with user and bot targets.
- MLS removes coordination. `pending_mls_removes` with user and bot targets, stale signal, durable confirmation.
- Key transparency. Append-only log with inclusion proofs and auditor signatures. Extended with bot identity keys and command keys. Historical verification via `bot_key_leaf_index`. User and bot endpoints.
- Link preview proxy. Opt-in, SSRF-guarded, blind to URLs.
- Extension proxy. Opt-in, SSRF-guarded egress gateway. Blind to URLs, request bodies, response bodies, and credentials.
- Call signaling. WebRTC signaling on user channels, gated on call participation, encrypted with per-call ephemeral ECDH.
- TURN credential endpoint.
- Call history. `GET /rooms/:id/calls` for room members.
- Sessions. Persistent, named, room-scoped real-time spaces with in-memory occupancy and encrypted signaling envelopes.
- Model hosting. STT and TTS models, with local, external, and proxy hosting modes.
- Headless bots. Capability-driven mode derivation, publisher key, command key, signed publishes, avatar upload, key rotation, declarative commands and settings, command results, owner-directed local messages.
- Declarative settings and commands. Server-side preferences contract and bot command routing.
- Sockudo subscription authentication. Server-issued HMAC signatures for channel subscriptions.
- Admin surfaces for all features.
- Batched test execution model.

### 2.2 Presence and Co-Presence

Ambient presence is refused. The server does not track or expose who is online, when they were last seen, or what they are doing across the system. There are no presence channels. `sessions.last_seen_at` (the HTTP sessions table, §7.1) is not user-facing and is coarsened in any admin view.

Call-scoped and session-scoped co-presence is permitted. Within a call or session, the server knows in memory which room members are participating, in order to route media. This knowledge is disclosed only to participants of the same call or session. It is held in memory only and is never written to any persistent store or external system. Leaving a call or session removes the participant completely; no record remains.

Session occupancy counts are disclosed to all room members. Non-participants see that a session exists and how many participants it has. Over time, this reveals activity patterns. Identities are not disclosed. Clients and operators must disclose this in their privacy documentation.

Bot connection status is disclosed only to the bot's owner. It is a server-to-bot liveness indicator, not user presence. It is held in memory only and is never written to any persistent store.

Call IP addresses are exposed between WebRTC peers. ICE candidates include host addresses. This is inherent to WebRTC and is not mediated by the server. Operators and clients must disclose this in their privacy documentation.

### 2.3 Out of Scope

- Ambient presence.
- Federation between servers.
- Multi-tenancy.
- Server-side key escrow.
- Plaintext metadata on server.
- Email or OAuth registration.
- Anonymous accounts.
- Plaintext message export.
- Username changes.
- Third-party CAPTCHA services.
- Client-side data deletion.
- Forcing peers to delete local copies.
- Consent management UI.
- TLS termination inside the Axum process.
- ACME client inside the server.
- MP4 fast-start enforcement.
- Multi-range HTTP requests.
- Range-restricted presigned URLs.
- Message forwarding.
- Message pinning.
- GIF search.
- Multi-node deployments.
- Shared SPA hosting across instances.
- Server-side periodic work beyond the shared hourly scheduler.
- Room events for extension sync without a session.
- Streaming through the extension proxy.
- Extension proxy response caching.
- Bot marketplace or discovery.
- Bot runtime hosted on the server.
- Bot runtime hosted inside a user's client.
- Runtime extension or bot installation.
- Sealed sender.
- SGX-based contact discovery.
- Threshold OPRF.
- Relay-only mode for WebRTC media.
- Conversational Kiko (client-side, post-v3.0).
- Sockudo app key / app secret rotation.

### 2.4 Differences From v2.0

| Area | v2.0 | v3.0 |
|---|---|---|
| Identity column | `users.username_token` = OPRF output | One-way derived lookup token |
| Display/device name keys | Derived from stored token | Derived from client-only token |
| Preferences | Plaintext JSON | Client-encrypted ciphertext |
| Room order | JSON in user_preferences | Dedicated `user_room_order` table |
| Call signaling channel | Room channel | User channel, gated on participation |
| Call signaling encryption | None | Per-call ephemeral ECDH |
| `call_participants` table | Present (24h retention) | Removed. In-memory only. |
| `mls.remove_confirmed` channel | Room channel only | Room + user channel (durable) |
| `pending_mls_removes` table | Absent | Present with stale signal |
| Session signaling | Plaintext signal_type + payload | Opaque envelope (encrypted, signed) |
| Publisher key derivation | Not specified | `MLS-Exporter("atoll", "publisher-key-v1", 32)` |
| Publisher key persistence | In-memory | Persisted in `room_publisher_keys` |
| Publisher key publish | Not specified | Signed by member identity key; `POST /rooms/:id/publisher-key` |
| Bots | Absent | Full capability-driven model |
| Bot tier | Absent | Removed. Mode derived from granted capabilities. |
| Bot runtime | Absent | Headless Node.js process on operator's machine |
| Bot message signature | Absent | Ed25519 over length-prefixed payload |
| Bot key rotation | Absent | `PATCH /bots/:id/keys` |
| Bot settings encryption | Absent | X25519 ECDH to `bot_command_pubkey` |
| Bot declarations | Absent | `commands` and `settings` stored server-side |
| Bot command results | Absent | Ephemeral X25519 per command; `bot.command_result` |
| Bot owner local messages | Absent | `bot.local_message` via `POST /bots/me/messages` |
| Settings/commands | Extension-local | Declarative; server defines sync contract |
| KT log | User identity keys | Extended with bot identity and command keys; user and bot endpoints |
| Whisper messages | Absent | `target_user_ids` on messages |
| Read counts | Absent | `read_by_count` on message fetch; live event |
| Call history | Absent | `GET /rooms/:id/calls` |
| Sockudo connection params | Capabilities `websocket_url`, `sockudo_app_key` | Capabilities `sockudo_url`, `sockudo_app_key`, `sockudo_auth_endpoint`; server-issued subscription auth |
| Security boundary table | Contained disproven claims | Corrected; enumeration, social graph, occupancy, IP exposure |
| Product summary | "Never sees meaningful metadata" | Explicit list of what the server sees |

---

## 3. Roles and Permissions

### 3.1 Global Roles

| Role | Level | Permissions |
|---|---|---|
| owner | 100 | `*` |
| admin | 80 | `user.manage, invite.unlimited, config.edit, room.force_delete, backup.manage` |
| inviter | 50 | `invite.limited` |
| member | 10 | `room.create, room.join, message.send` |

### 3.2 Room Roles

| Role | Permissions |
|---|---|
| owner | Kick, delete room, promote/demote (Discord mode), transfer ownership, delete any message, edit metadata, change retention, set disappearing timer, create/delete sessions, grant/revoke bots |
| moderator | Kick (Discord mode), delete any message (Discord mode), create/delete sessions (Discord mode), grant/revoke bots (Discord mode) |
| member | Send messages, upload attachments, leave room, delete own messages, add reactions, edit own messages (within `edit_window_seconds`), create invites, join sessions, create sessions (Messenger mode) |

### 3.3 Moderation Modes

Discord mode: moderator+ for session creation, session deletion, bot grants, and kicks. Messenger mode: any member may create sessions; only owner may grant bots.

### 3.4 Enforcement Order

Server role → room role → resource-specific permission.

### 3.5 Room Ownership Transfer

Owner must transfer ownership before leaving, unless they are the sole member (in which case the room is deleted). `POST /rooms/:id/transfer-ownership` initiates; the recipient must accept. Until accepted, the transfer is pending and the owner may cancel.

Events: `room.transfer_initiated` on the recipient's user channel; `room.transfer_accepted` on both parties' user channels and on the room channel; `room.transfer_cancelled` on the recipient's user channel and on the owner's own user channel for multi-device sync.

---

## 4. Resource Limits

### 4.1 Three-Tier Model

Server hard max → instance default → per-entity override.

### 4.2 Default Limits

| Key | Server hard max | Instance default | Instance range |
|---|---|---|---|
| file_size_bytes | 104857600 | 104857600 | 1 MB – 100 MB |
| room_size | 1000 | 100 | 2 – 1000 |
| rooms_per_user | 500 | 50 | 1 – 500 |
| devices_per_user | 20 | 10 | 1 – 20 |
| keypackages_per_device | 50 | 20 | 5 – 50 |
| message_size_bytes | 65536 | 16384 | 256 B – 64 KB |
| attachment_retention_days | 365 | 0 (forever) | 0 – 365 |
| call_max_participants | 50 | 8 | 2 – 50 |
| reactions_per_message | 50 | 50 | 1 – 50 |
| edit_window_seconds | 86400 | 900 | 60 – 86400 |
| sync_event_retention_days | 365 | 90 | 30 – 365 |
| key_transparency_retention_days | 3650 | 3650 | 365 – 3650 |
| sessions_per_room | 25 | 10 | 1 – 25 |
| session_max_participants | 50 | 12 | 2 – 50 |
| starred_items_per_user | 100000 | 10000 | 100 – 100000 |
| bots_per_room | 25 | 10 | 1 – 25 |
| bots_per_user | 100 | 50 | 1 – 100 |
| pending_mls_removes_per_room | 500 | 100 | 10 – 500 |
| pending_mls_removes_per_user | 100 | 100 | 10 – 100 |
| preferences_value_bytes_encrypted | 524288 | 131072 | 65536 – 524288 |
| call_record_retention_days | 3650 | 365 | 0 – 3650 |

### 4.3 Per-Entity Overrides

`rooms.sessions_per_room`, `rooms.bots_per_room`, `rooms.pending_mls_removes_per_room`, `users.max_file_size_bytes`.

### 4.4 Effective Limits Exposure

`GET /rooms/:id` returns `effective_max_file_size_bytes` and `effective_message_retention_days`.

### 4.5 Cleanup Jobs

All jobs run on the shared hourly scheduler.

| Job | Retention | Notes |
|---|---|---|
| Session cleanup | 30 days after expiry | HTTP sessions (§7.1) |
| Rate limit table | 24 hours | — |
| Audit log | `AUDIT_RETENTION_DAYS` (90) | — |
| Attachment pruning | Effective retention | — |
| Welcome expiry | 7 days | — |
| Message retention | Per-room, falling to instance default | — |
| Registration state | 5-minute TTL | — |
| Login state | 5-minute TTL | — |
| Sync state pruning | `sync_event_retention_days` | Deletes rows older than the window whose `user_seq` is below the current max |
| Push subscription expiry | 90 days of inactivity | — |
| Key transparency retention | `key_transparency_retention_days` | — |
| Recovery code consumption | Immediate | Consumed codes marked, not deleted |
| Pending MLS adds expiry | 7 days | Stale adds pruned; clients may re-add |
| Pending MLS removes stale | `PENDING_MLS_REMOVE_TIMEOUT_DAYS` (30) | Sets `stale_at` and publishes `mls.remove_stale` **once** per row. Fires only for rows where `remove_confirmed_at IS NULL` and `cancelled_at IS NULL`. Does not re-fire. |
| Pending MLS removes consumed | 30 days after consumed | Tombstone pruning |
| Bot token pruning | 90 days of inactivity | Revokes and deletes tokens |
| Bot command TTL | `BOT_COMMAND_TTL_HOURS` (24) | Marks expired commands. Does not apply once `acked_at` is set. |
| Bot request log pruning | 24 hours | Idempotency records |
| Revoked bot grant tombstones | 30 days | — |
| Room session metadata pruning (optional) | 90 days of zero occupancy | Disabled by default |
| Extension proxy bandwidth accounting | 24 hours | — |
| Call record retention | `CALL_RECORD_RETENTION_DAYS` (365) | Deletes `call_sessions` rows with `ended_at < now - retention`. No cascade; there is no `call_participants` table. |

No cleanup job is required for session occupancy, call participation, bot connection status, publisher keys, observer stream cursors, or local messages. These are in-memory, client-only, or persisted with their own lifecycle.

---

## 5. Environment Variables

### 5.1 Application

```env
APP_ENV=production
APP_URL=https://chat.example.com
APP_NAME=Encrypted Chat
LOG_LEVEL=info
CLIENT_STATIC_DIR=/app/client
```

Log policy: the server writes structured logs to stdout and stderr only. It does not write log files. Session occupancy, call participation, bot connection status, publisher keys, observer stream contents, call signaling payloads, session signal payloads, and extension proxy URLs/bodies are never logged — not to stdout, not to stderr, not to metrics, not to tracing.

### 5.2 Server

```env
SERVER_BIND=0.0.0.0:8080
SERVER_WORKERS=4
```

### 5.3 Database

```env
DB_PATH=/data/app.db
DB_BUSY_TIMEOUT_MS=5000
```

### 5.4 HTTP Sessions

```env
SESSION_EXPIRY_DAYS=30
SESSION_SLIDING=true
```

### 5.5 Server Hard Limits

```env
SERVER_MAX_FILE_SIZE_BYTES=104857600
SERVER_MAX_ROOM_SIZE=1000
SERVER_MAX_ROOMS_PER_USER=500
SERVER_MAX_DEVICES_PER_USER=20
SERVER_MAX_KEYPACKAGES_PER_DEVICE=50
SERVER_MAX_MESSAGE_SIZE_BYTES=65536
SERVER_MAX_ATTACHMENT_RETENTION_DAYS=365
SERVER_MAX_EDIT_WINDOW_SECONDS=86400
SERVER_MAX_REACTIONS_PER_MESSAGE=50
SERVER_MAX_SESSIONS_PER_ROOM=25
SERVER_MAX_SESSION_PARTICIPANTS=50
SERVER_MAX_STARRED_ITEMS_PER_USER=100000
SERVER_MAX_BOTS_PER_ROOM=25
SERVER_MAX_BOTS_PER_USER=100
SERVER_MAX_PENDING_MLS_REMOVES_PER_ROOM=500
SERVER_MAX_PREFS_VALUE_BYTES_ENCRYPTED=524288
```

### 5.6 Rate Limits

```env
RATE_INVITE_CREATE_HOURLY=50
RATE_INVITE_CREATE_DAILY=200
RATE_INVITE_REDEEM_PER_MIN=10
RATE_KP_CLAIM_PER_MIN=30
RATE_KP_CLAIM_HOURLY=200
RATE_LOGIN_PER_MIN=10
RATE_LOGIN_LOCKOUT_MIN=15
RATE_PRESIGN_PER_MIN=60
RATE_OPRF_BLIND_PER_MIN=30
RATE_OPRF_BLIND_PER_HOUR=300
RATE_RECOVER_START_PER_MIN=5
RATE_RECOVER_START_PER_HOUR=20
RATE_LOOKUP_PER_MIN=30
RATE_EDIT_PER_MIN=30
RATE_REACTION_PER_MIN=60
RATE_LINK_PREVIEW_PER_MIN=10
RATE_TURN_CREDENTIALS_PER_MIN=10
RATE_SESSION_CREATE_HOURLY=20
RATE_SESSION_CREATE_DAILY=100
RATE_SESSION_JOIN_PER_MIN=30
RATE_SESSION_HEARTBEAT_PER_MIN=10
RATE_SESSION_SIGNAL_PER_MIN=120
RATE_CALL_SIGNAL_PER_MIN=120
RATE_MODEL_DOWNLOAD_PER_MIN=30
RATE_BOT_MESSAGE_PER_MIN=60
RATE_BOT_COMMAND_PER_MIN=60
RATE_BOT_COMMAND_RESULT_PER_MIN=120
RATE_BOT_LOCAL_MESSAGE_PER_MIN=60
RATE_BOT_FETCH_PER_MIN=30
RATE_BOT_OBSERVER_STREAM_PER_MIN=10
RATE_BOT_TOKEN_ISSUE_PER_HOUR=20
RATE_BOT_CREATE_PER_HOUR=5
RATE_BOT_GRANT_PER_HOUR=50
RATE_BOT_KEY_ROTATE_PER_HOUR=5
RATE_SOCKUDO_AUTH_PER_MIN=300
```

Key formats (excerpt):

| Variant | Key format |
|---|---|
| OprfBlind | `oprf_blind:{ip}:{min\|hour}:{boundary}` |
| Login | `login:{ip}:min:{boundary}` |
| Lookup | `lookup:{user_id}:min:{boundary}` |
| SessionSignal | `session_signal:{user_id}:{session_id}:min:{boundary}` |
| CallSignal | `call_signal:{user_id}:{call_id}:min:{boundary}` |
| BotMessage | `bot_message:{bot_id}:{room_id}:min:{boundary}` |
| BotCommand | `bot_command:{bot_id}:{user_id}:min:{boundary}` |
| BotCommandResult | `bot_command_result:{bot_id}:{user_id}:min:{boundary}` |
| BotLocalMessage | `bot_local_message:{bot_id}:min:{boundary}` |
| BotKeyRotate | `bot_key_rotate:{bot_id}:hour:{boundary}` |
| SockudoAuth | `sockudo_auth:{identity}:min:{boundary}` — where `{identity}` is `user:{user_id}` or `bot:{bot_id}` |
| ExtensionProxy | `extension_proxy:{user_id}:{extension_id}:{minute\|hour\|day}:{boundary}` |
| ExtensionProxyTotal | `extension_proxy_total:{user_id}:{minute\|hour\|day}:{boundary}` |

Rate limit state is stored in SQLite and pruned hourly.

**Applies-to mapping:**

| Rate limit | Applies to |
|---|---|
| `RATE_BOT_FETCH_PER_MIN` | `GET /rooms/:id/publisher-key`, `GET /rooms/:id/bots`, `GET /bots/me/settings`, `GET /kt/bot/:id`, `GET /kt/bot/:id/history`, `GET /kt/user/:id`, `GET /bots/:id/tokens` |
| `RATE_BOT_OBSERVER_STREAM_PER_MIN` | Connection attempts to `GET /rooms/:id/observer-stream` |
| `RATE_BOT_MESSAGE_PER_MIN` | `POST /rooms/:id/bot-messages` |
| `RATE_BOT_COMMAND_RESULT_PER_MIN` | `POST /bots/me/messages` with `target: "invoker"` |
| `RATE_BOT_LOCAL_MESSAGE_PER_MIN` | `POST /bots/me/messages` with `target: "owner"` |
| `RATE_BOT_COMMAND_PER_MIN` | `POST /rooms/:id/bot-commands` (per bot per user) |
| `RATE_BOT_KEY_ROTATE_PER_HOUR` | `PATCH /bots/:id/keys` |
| `RATE_BOT_TOKEN_ISSUE_PER_HOUR` | `POST /bots/:id/tokens` |
| `RATE_BOT_CREATE_PER_HOUR` | `POST /bots` |
| `RATE_BOT_GRANT_PER_HOUR` | `POST /rooms/:id/bots` |
| `RATE_SOCKUDO_AUTH_PER_MIN` | `POST /sockudo/auth` |

### 5.7 Transport Security

All production traffic is HTTPS. HSTS enabled. TLS terminates at the reverse proxy.

### 5.8 OPAQUE and OPRF

```env
OPAQUE_OPRF_KEY_PATH=/data/oprf.key
USERNAME_OPRF_ENABLED=true
```

Cipher suite (OPAQUE): DefaultCipherSuite — Ristretto255, TripleDh<Ristretto255, Sha512>, Argon2 KSF.

OPRF construction (username lookup): voprf 0.5.0 crate, Ristretto255-SHA512, base (non-verifiable) mode.

Key derivation from the ServerSetup file:

```
oprf_key_bytes  = contents of OPAQUE_OPRF_KEY_PATH (serialized ServerSetup, 128 bytes)
root_secret     = SHA-256(oprf_key_bytes)
username_key    = HKDF-Expand(root_secret, info="username-oprf-v1", length=32)
backup_key      = HKDF-Expand(root_secret, info="backup-encryption-v1", length=32)
```

The voprf server is constructed as:

```rust
voprf::OprfServer::<RistrettoSha512>::new_from_seed(
    &username_key,
    b"username-oprf-v1",
)
```

Enumeration limitation. The operator holds k. Tokens are deterministic for a given (username, k). The operator can compute tokens for candidate usernames offline and compare them against stored values. This is inherent to single-key OPRF. High-entropy or generated usernames are not enumerable. Operators and clients must disclose this limitation.

Base mode rationale. Verifiable mode (VOPRF) is not used. The operator can already enumerate via the mechanism above, so the additional DLEQ proof adds complexity without meaningful protection. Base mode is deliberate.

`root_secret` rotation policy. `root_secret` is immutable for the lifetime of the deployment. Rotation is a catastrophic, operator-initiated operation requiring full user re-registration.

### 5.9 Backups

Backups include: the OPRF key, the ALTCHA HMAC secret, the VAPID keys, the KT log directory, the session types config, the extension proxy domain blocklist, the `server_config` table (including the Sockudo app secret, treated as sensitive), and all other database contents. Attachment blobs are included only if `BACKUP_INCLUDE_ATTACHMENTS=true` and `STORAGE_BACKEND=fs`. Model files are included only if `BACKUP_INCLUDE_MODELS=true`. Session occupancy, call participation, bot connection status, observer cursors, and local messages are not part of any backup because they are not stored. Publisher keys persisted in `room_publisher_keys` are part of the database and are backed up with it.

### 5.10 Push Notifications

Push payload carries `room_id` and an opaque `sender_ref`. `sender_ref` is the server-stored `username_token` (a lookup token) truncated to 16 bytes and base64url-encoded. The server derives `sender_ref`; the client does not compute it.

### 5.11 ALTCHA

ALTCHA Proof-of-Work v2 via altcha 0.2.0.

### 5.12 Storage Backend

`STORAGE_BACKEND=s3` (default) or `filesystem`. S3 uses rust-s3.

### 5.13 Attachment Format

C2SP chunked encryption. Chunk size 16384 bytes, not configurable.

### 5.14 Moderation

Discord and Messenger modes. Discord restricts session creation, bot grants, and kicks to moderator+.

### 5.15 Invite Defaults

Invites expire after 7 days by default.

### 5.16 Safety Numbers

Computed locally from MLS identity keys. `SAFETY_NUMBER_MODE` = `warn` (default), `enforce`, `off`.

### 5.17 Audit Log

```env
AUDIT_RETENTION_DAYS=90
```

`audit_log.metadata` MUST NOT contain message content, IP addresses, URLs, or user-identifying data beyond `user_id`, `bot_id`, and `room_id`.

### 5.18 Cleanup Scheduler

Shared hourly scheduler. All jobs in §4.5.

### 5.19 Sockudo

```env
SOCKUDO_URL=http://sockudo:6001
SOCKUDO_PUBLIC_URL=wss://chat.example.com/sockudo
SOCKUDO_APP_ID=chat
SOCKUDO_APP_KEY=
SOCKUDO_APP_SECRET=
SOCKUDO_AUTH_PATH=/sockudo/auth
SOCKUDO_ENABLE_CLIENT_EVENTS=true
```

Field semantics:

| Variable | Required | Exposed | Notes |
|---|---|---|---|
| `SOCKUDO_URL` | Yes | No | Internal URL used by the server to reach Sockudo. Never returned in capabilities. |
| `SOCKUDO_PUBLIC_URL` | Yes (client-reachable deployments) | Yes | The URL clients and bots connect to. Must be `ws://` or `wss://`. Production requires `wss://`. No trailing slash. |
| `SOCKUDO_APP_ID` | Yes | No | Internal identifier. |
| `SOCKUDO_APP_KEY` | No | Yes | Empty → generate on first boot, persist in `server_config`, expose in capabilities. Public by design. |
| `SOCKUDO_APP_SECRET` | No | No | Empty → generate on first boot, persist in `server_config`. Never exposed. |
| `SOCKUDO_AUTH_PATH` | No | Yes | Defaults to `/sockudo/auth`. Configurable for subpath deployments. Mounted at reverse-proxy root, not under `/api/v1/`. |
| `SOCKUDO_ENABLE_CLIENT_EVENTS` | No | No | Enables `client-typing.*` events. |

Startup validation:

- `SOCKUDO_PUBLIC_URL` scheme must be `ws://` or `wss://`. The server refuses to start on invalid scheme.
- If `SOCKUDO_PUBLIC_URL` is set but `SOCKUDO_URL` is unreachable at startup, the server logs a warning once per minute and continues running. It does not refuse to start. The reverse proxy may not be up yet.

Channel taxonomy:

| Channel | Type | Purpose |
|---|---|---|
| `private-room-{room_id}` | Private | Room events, client events (typing only) |
| `private-user-{user_id}` | Private | User-scoped durable events, call signaling, session signaling |
| `private-bot-{bot_id}` | Private | Bot commands, lifecycle events, settings updates |

Channel name pattern: `^private-(room|user|bot)-[a-zA-Z0-9_-]+$`.

No presence channels.

Client events accepted:

| Event | Channel | Payload |
|---|---|---|
| `client-typing.start` | `private-room-{room_id}` | `{ user_id }` |
| `client-typing.stop` | `private-room-{room_id}` | `{ user_id }` |

`client-read` is removed. Read state is written via `POST /users/me/read-state`.

`client-mls-request` is removed. MLS add is coordinated via `mls.add_pending` room events.

Subscription authentication:

- Endpoint: `POST /sockudo/auth` (see §8.1.6).
- Scheme: Pusher v7 HMAC-SHA256.
- Signature: `HMAC-SHA256(app_secret, socket_id + ":" + channel_name)`.
- Response: `{ "auth": "<app_key>:<hex-hmac>" }`.
- Lowercase hex, no trailing newline.
- UTF-8 encoding for all strings.

### 5.20 GDPR

See §14.

### 5.21 TLS Termination

External reverse proxy (Traefik via Coolify, or Caddy).

### 5.22 Key Transparency

```env
KEY_TRANSPARENCY_ENABLED=true
KEY_TRANSPARENCY_LOG_PATH=/data/kt-log
KEY_TRANSPARENCY_AUDITOR_KEYS=
```

Client-visible verification state (normative):

| State | Condition | Client display |
|---|---|---|
| Not independently verified | `auditor_count = 0` | "Not independently verified." |
| Awaiting signatures | `auditor_count > 0`, signatures missing | "Awaiting auditor signature." |
| Verified | `auditor_count > 0`, signatures valid | "Verified by N auditors." |

`/capabilities` advertises `key_transparency_auditor_count`.

### 5.23 Link Preview Proxy

```env
LINK_PREVIEW_PROXY_ENABLED=false
LINK_PREVIEW_PROXY_TIMEOUT_SECONDS=5
LINK_PREVIEW_PROXY_MAX_BYTES=1048576
```

### 5.24 Calls

```env
CALLING_ENABLED=false
TURN_URL=
TURN_SHARED_SECRET=
TURN_TTL_SECONDS=600
CALL_MAX_PARTICIPANTS=8
CALL_RECORD_RETENTION_DAYS=365
```

TURN is operator-provided. Without TURN, calls succeed only on non-symmetric NAT. WebRTC ICE candidates expose participant IP addresses to each other. This is inherent to WebRTC.

### 5.25 Username OPRF

```env
OPRF_BLIND_ENABLED=true
```

### 5.26 Sessions

```env
SESSIONS_ENABLED=true
SERVER_MAX_SESSIONS_PER_ROOM=25
SERVER_MAX_SESSION_PARTICIPANTS=50
SESSION_HEARTBEAT_INTERVAL_SECONDS=15
SESSION_HEARTBEAT_TIMEOUT_SECONDS=45
SESSION_OCCUPANCY_DEBOUNCE_MS=1000
SESSION_TYPES_CONFIG_PATH=/data/session-types.toml
```

#### 5.26.1 Session Types Configuration

```toml
[[session_type]]
type = "voice"
extension_id = "core.hangouts"
max_participants = 12
max_per_room = 3

[[session_type]]
type = "watch"
extension_id = "com.example.watch-together"
max_participants = 20
max_per_room = 5
```

Rules: the server reads this file at startup and on `POST /admin/session-types/reload`. A session can only be created with a `session_type` present in the file. A session's `max_participants` cannot exceed the type's cap. A room can hold at most `max_per_room` sessions of a given type. `extension_id` is informational and is echoed into capabilities. Malformed or missing file: sessions disabled at startup with a clear error, server continues running. Reload is atomic; on validation error, the previous config remains in effect and the admin endpoint returns 400.

### 5.27 Model Hosting

```env
MODEL_HOSTING_ENABLED=true
MODEL_HOSTING_MODE=local
MODEL_EXTERNAL_BASE_URL=
MODEL_STORAGE_PATH=/data/models
STT_MODELS_PATH=/data/models/stt
TTS_MODELS_PATH=/data/models/tts
STT_DEFAULT_MODEL=moonshine-tiny
TTS_DEFAULT_MODEL=supertonic-3
MODEL_DOWNLOAD_RATE_PER_MIN=30
BACKUP_INCLUDE_MODELS=false
```

Three modes: `local`, `external`, `proxy`. Startup validation as in v2.0.

### 5.28 Generic Preferences

No environment variables. Always enabled when authenticated.

### 5.29 Starred Items

No environment variables. Always enabled when authenticated.

### 5.30 Batched Test Execution Model

`server/tests/batch-manifest.toml`, `make test-<batch>`, `make check-batches`. Every file in `server/tests/*.rs` appears in exactly one batch. `common.rs` and `test_batch_manifest.rs` excluded. Each batch completes in under 60 seconds.

### 5.31 Extension Proxy

```env
EXTENSION_PROXY_ENABLED=false
EXTENSION_PROXY_MAX_REQUESTS_PER_MIN=60
EXTENSION_PROXY_MAX_REQUESTS_PER_HOUR=500
EXTENSION_PROXY_MAX_REQUESTS_PER_MIN_TOTAL=120
EXTENSION_PROXY_MAX_REQUESTS_PER_HOUR_TOTAL=1000
EXTENSION_PROXY_MAX_BANDWIDTH_PER_HOUR_BYTES=52428800
EXTENSION_PROXY_MAX_BANDWIDTH_PER_DAY_BYTES=524288000
EXTENSION_PROXY_MAX_BANDWIDTH_PER_HOUR_BYTES_TOTAL=104857600
EXTENSION_PROXY_MAX_BANDWIDTH_PER_DAY_BYTES_TOTAL=1073741824
EXTENSION_PROXY_TIMEOUT_CONNECT_SECONDS=5
EXTENSION_PROXY_TIMEOUT_READ_SECONDS=30
EXTENSION_PROXY_MAX_RESPONSE_BYTES=10485760
EXTENSION_PROXY_MAX_REQUEST_BYTES=262144
EXTENSION_PROXY_USER_AGENT=Atoll/3.0
EXTENSION_PROXY_DENY_DOMAINS=
EXTENSION_PROXY_DENY_DOMAINS_PATH=
```

Two-bucket rate limiting: per user per extension, and per user total. Domain blocklist checked after decryption, before request. No operator allowlist. Reject `Authorization` and `Cookie` headers by name.

### 5.32 Bot Infrastructure

```env
BOTS_ENABLED=true
BOT_COMMAND_TTL_HOURS=24
SERVER_MAX_BOTS_PER_ROOM=25
SERVER_MAX_BOTS_PER_USER=100
PENDING_MLS_REMOVE_TIMEOUT_DAYS=30
RATE_BOT_KEY_ROTATE_PER_HOUR=5
```

### 5.33 Preferences Encryption

```env
PREFERENCES_MAX_ENCRYPTED_BYTES=131072
```

Effective plaintext limit ≈ 96 KB.

### 5.34 Reserved

Reserved for future use.

---

## 6. Client Interface Contract

### 6.1 Authentication

`Authorization: Bearer <session_token>` on every authenticated request. Tokens are 43-character base64url strings. Bot tokens follow the same format.

The Sockudo auth endpoint (`POST /sockudo/auth`) accepts the same header. The token lookup tries the HTTP sessions table and the bot tokens table; the identity is derived from whichever table matched.

### 6.2 OPAQUE Handshake

- Registration: start then finish. Both within 5 minutes.
- Login: start then finish. Both within 5 minutes. Unknown tokens use a dummy OPAQUE registration record. Response shape and timing must be indistinguishable from a known token.
- Recovery: `POST /auth/recover/start` then `POST /auth/recover/finish`. Both within 10 minutes. Unknown tokens also use a dummy record.

### 6.3 ALTCHA Payload

ALTCHA Proof-of-Work v2 payload.

### 6.4 MLS Message Envelope

MLS message envelope as in v2.0.

### 6.5–6.11 Attachments

C2SP chunked encryption, range requests, presigned URLs (S3 only).

### 6.12 Room Membership

Member list, invite flow, role changes.

### 6.13 Push Subscriptions

Web Push, APNs, FCM, UnifiedPush.

### 6.14 Safety Numbers

Computed locally from MLS identity keys.

### 6.15 WebSocket Connection

Sockudo WebSocket connection.

Connection parameters are fetched from `GET /capabilities`:
- `sockudo_url` — the WebSocket URL to connect to.
- `sockudo_app_key` — the Pusher app key.
- `sockudo_auth_endpoint` — the auth endpoint path.

Subscription auth flow (client or bot):

1. Connect to `sockudo_url` with `sockudo_app_key`.
2. For each channel, send a subscription request to Sockudo.
3. Sockudo calls `POST {sockudo_auth_endpoint}` with `{ socket_id, channel_name }`.
4. Sockudo receives `{ auth }` and validates the signature.
5. Subscription is established.

Clients do not compute HMAC signatures. The server issues them.

### 6.16 Client Events

Only `client-typing.start` and `client-typing.stop`. Client events are ephemeral.

### 6.17 Client Capability Requirements

Standard capability negotiation. The three Sockudo fields are part of the unauthenticated capabilities response; the client must fetch capabilities before attempting WebSocket connection.

### 6.18 CoreCrypto Initialization

MLS client init as in v2.0.

### 6.19 OPRF Blinding and Token Split

The client computes the username token before registration, login, and lookup:

```
Given:
  username       the plaintext username
  k              the server's OPRF key (never seen by the client)
  H              Ristretto255 hash-to-curve

Client:
  1. h         = H(username)
  2. r         = random blinding factor
  3. blinded   = h * r
  4. sends { blinded } to POST /oprf/blind

Server:
  5. evaluated = blinded ^ k
  6. returns { evaluated }

Client:
  7. token        = OprfClient::finalize(username, evaluated)   ← 64-byte SHA-512
  8. lookup_token = HKDF-Expand(token, "username-lookup-v1", 64)
```

Token format: 64-byte SHA-512 output, encoded as 86-character unpadded base64url. It is not the 32-byte group element `h^k`.

Lookup token format: 64 bytes, encoded as 86-character unpadded base64url.

Wire format: the client sends `lookup_token` in the `lookup_token` field. `token` is never sent to the server.

Storage: the server stores `lookup_token` in `users.username_token`. The column name is retained; the comment reflects that it holds a lookup token, not the raw OPRF output.

Token construction (RFC 9497 §2.2):

```
hash_input = I2OSP(len(username), 2) || username
           || I2OSP(len(unblinded_element), 2) || unblinded_element
           || "Finalize"
token      = SHA-512(hash_input)
```

The token is deterministic for a fixed (username, server_key). The client's blinding factor does not affect the output. The client caches `token` and `lookup_token` in memory for the session. Neither is persisted at rest.

HKDF parameters: SHA-256 hash, 64-byte output, info string `"username-lookup-v1"` (UTF-8, no null terminator). HKDF-Expand only.

### 6.20 Display Name Encryption

```
display_name_key  = HKDF-Expand(token, info="display-name-encryption-v1", length=32)
nonce             = random 12 bytes
encrypted_display = nonce || AES-256-GCM(display_name_key, nonce, display_name)
```

Constraints: plaintext bounded to 256 bytes; ciphertext `12 + 256 + 16 = 284` bytes; base64url unpadded. The server never decrypts.

### 6.21 Device Name Encryption

```
device_name_key       = HKDF-Expand(token, info="device-name-encryption-v1", length=32)
nonce                 = random 12 bytes
encrypted_device_name = nonce || AES-256-GCM(device_name_key, nonce, device_name)
```

Same constraints as §6.20.

### 6.22 User-Scoped Sync Cursor

On boot, after SQLite hydrate and before subscribing to channels:

1. `GET /users/me/sync?since_seq=<cursor>`
2. Apply returned state rows in `user_seq` order.
3. Store `max_seq` as the new cursor.
4. Subscribe to `private-user-{user_id}`.
5. Apply live events as they arrive; ignore any event with `user_seq <= cursor`.
6. Update the cursor on every applied row.

If the server returns `{ "full_resync_required": true }`, the client discards its cursor and refetches all current state. The sync response is applied atomically. A partial application does not advance the cursor.

### 6.23 Recovery

1. Prompt for recovery code and username.
2. Blind the username via `POST /oprf/blind`.
3. Derive `lookup_token`.
4. Call `POST /auth/recover/start` with `{ recovery_code, lookup_token }`.
5. Receive a recovery session and an OPAQUE registration challenge.
6. Compute the new RegistrationRecord bound to the new password.
7. Encrypt the display name with the current `token`.
8. Call `POST /auth/recover/finish`.
9. Store the new session.

The client MUST treat session revocation on other devices as expected.

### 6.24 Calls

1. Fetch `calling` from `/capabilities`. If false, calls are unavailable.
2. Join via `POST /rooms/:id/calls/:call_id/join` with `{ client_id }`.
3. Generate an ephemeral X25519 keypair for the call.
4. Send the ephemeral public key, signed with the client's MLS identity private key, via `POST /rooms/:id/calls/:call_id/signal`. The server relays it on the target's `private-user-{user_id}` channel as a `call.signal` event. Clients do not publish to channels directly.
5. Fetch TURN credentials via `POST /calls/turn-credentials`.
6. Verify each participant's `identity_pubkey` against the KT log before trusting their signed ephemeral key.
7. For the initiator: generate a random 32-byte call key, encrypt it to each participant's ephemeral public key (X25519 ECDH → HKDF-SHA256 → AES-256-GCM), distribute via signaling.
8. All signaling messages are AES-256-GCM under the call key. Associated data is `call_id || sender_user_id || target_user_id`. `signal_type` is inside the encrypted envelope.
9. Rotate the call key on join and leave. The participant with the lowest `user_id` among remaining participants generates the new key. No rotation on re-negotiation.
10. Leave via `POST /rooms/:id/calls/:call_id/leave` with `{ client_id }`. Any `client_id` of the authenticated user may be specified.

### 6.25 Link Preview Proxy

1. Fetch `link_preview_proxy_enabled` from `/capabilities`.
2. When enabled and a URL is CORS-blocked, encrypt the URL with a per-request Content Key.
3. Send the encrypted URL to `POST /link-preview/proxy`.
4. Decrypt the response with the Content Key.

The server never sees the plaintext URL.

### 6.26 Sessions

1. Fetch `sessions_enabled` from `/capabilities`. If false, join/signal/heartbeat return 501; list/patch/delete remain available.
2. Fetch the session list via `GET /rooms/:id/sessions`.
3. Join via `POST /rooms/:id/sessions/:session_id/join`.
4. Send a heartbeat every `SESSION_HEARTBEAT_INTERVAL_SECONDS` via `POST .../heartbeat`.
5. On WebSocket disconnect, do not automatically rejoin.
6. Leave via `POST .../leave`. Any `client_id` of the authenticated user may be specified.
7. Initiate offers to all existing participants on join, using `POST .../signal` with `target_client_id`.
8. Session signaling uses the envelope format defined in §6.26.1.

#### 6.26.1 Session Signal Envelope

- `session_signal_key = HKDF-Expand(RMK, "session-signal-v1" || session_id, 32)`, SHA-256.
- Envelope plaintext: `{ session_id, sender_user_id, sender_client_id, signal_type, payload }`.
- Associated data: `session_id || sender_user_id || sender_client_id || (target_client_id or "")`.
- Encryption: AES-256-GCM, 12-byte random nonce.
- Signature: Ed25519 over `nonce || ciphertext || tag`, using the sender's MLS identity private key.
- Wire format: `signature (64) || nonce (12) || ciphertext || tag (16)`, base64url unpadded.
- Transmit `{ sender_client_id, target_client_id, envelope }`.
- On receive: verify the sender's `identity_pubkey` against the KT log, verify the signature, decrypt, deliver typed payload to the extension.

Confidentiality limitation: session signal keys are derivable by all room members. Confidentiality from non-participants depends on the server's delivery gating. A non-participant who obtains an envelope by other means can decrypt it. Signaling is not confidential from other participants.

### 6.27 Model Fetching

As v2.0.

### 6.28 Starred Items

1. `GET /users/me/starred-items` for the initial list, with optional `type`, `room_id`, `limit`, `cursor`.
2. `POST /users/me/starred-items` to star.
3. `DELETE /users/me/starred-items/:item_id?item_type=` to unstar.
4. Apply `starred_item.added` and `starred_item.removed` events by `user_seq`.
5. Apply tombstones (`deleted_at IS NOT NULL`) by removing the item locally.

### 6.29 Generic Preferences

- `GET /users/me/preferences/:key`
- `PATCH /users/me/preferences/:key` — request body `{ "value": "<base64url>" }`.
- `DELETE /users/me/preferences/:key`

Client-side encryption: `preferences_key = HKDF-Expand(token, "preferences-encryption-v1", 32)`, SHA-256. AES-256-GCM with 12-byte random nonce. Associated data is the full preference key (UTF-8). Wire format `nonce || ciphertext || tag`, base64url unpadded.

Limits: encrypted value ≤ 128 KB. Effective plaintext ≤ ~96 KB. Client enforces the plaintext limit; server enforces the encrypted limit.

Reserved keys: `room_order` is not a preference; it has its own endpoint and table. Keys starting with `_` are reserved.

### 6.30 Extension Proxy

1. Fetch `extension_proxy_enabled` from `/capabilities`.
2. Encrypt the request with a per-request Content Key. Send `POST /extensions/proxy`.
3. Decrypt the response with the same Content Key.
4. Handle `502 upstream_response_too_large` by retrying with `transport: 'direct'`.
5. Handle `429` with `Retry-After`.
6. Handle `400 header_not_allowed` by falling back to direct fetch.

### 6.31 Bot Commands and Settings

Command flow:

1. Client fetches the bot's `bot_command_pubkey` from the KT log.
2. Client verifies the key against the KT entry. If mismatch, the client refuses to encrypt.
3. Client generates an ephemeral result keypair (`ephemeral_result_priv`, `ephemeral_result_pub`) for the result.
4. Client encrypts the command plaintext with X25519 ECDH to `bot_command_pubkey`:
   ```
   shared  = ECDH(client_ephemeral_priv, bot_command_pubkey)
   key     = HKDF-Expand(shared, "bot-command-v1", 32)
   nonce   = random 12 bytes
   ct, tag = AES-256-GCM(key, nonce, plaintext)
   ```
5. Client sends `POST /rooms/:id/bot-commands` with `{ ciphertext, ephemeral_result_pubkey, request_id }`.
6. Server relays to the bot's channel via `bot.command_invoked`, including `ephemeral_result_pubkey`.
7. Bot decrypts with `bot_command_private_key`.

Command plaintext shape (inside the ciphertext):

```json
{
  "command_name": "pr",
  "args": {
    "number": 1234
  }
}
```

Named args only. Positional args are not used.

Command results:

1. Bot executes the handler.
2. Bot generates its own ephemeral X25519 keypair (`bot_result_priv`, `bot_result_pub`).
3. Bot derives `shared = ECDH(bot_result_priv, ephemeral_result_pub)`.
4. Bot derives `key = HKDF-Expand(shared, "bot-command-result-v1", 32)`.
5. Bot encrypts the result: `nonce || AES-256-GCM(key, nonce, plaintext)`.
6. Bot sends `POST /bots/me/messages` with `target: "invoker"`, `command_id`, `result_type`, `ciphertext`, `bot_result_pubkey`, `request_id`.
7. Server relays `bot.command_result` on `private-user-{invoking_user_id}`.
8. Client decrypts with `ephemeral_result_priv`.

Client holds `ephemeral_result_priv` for 60 seconds. On timeout, discard and surface "The bot did not respond."

Bot settings use X25519 ECDH to `bot_command_pubkey`. The client does not hold the bot's identity private key and cannot derive a shared key from it.

Flow for `PATCH /users/me/bots/:bot_id/settings/:key`:

1. Client fetches `bot_command_pubkey` from the KT log and verifies it.
2. Client generates an ephemeral X25519 keypair.
3. `shared = ECDH(ephemeral_private, bot_command_pubkey)`.
4. `key = HKDF-Expand(shared, "bot-settings-v1", 32)` (SHA-256).
5. Value encrypted with AES-256-GCM under `key`.
6. Client sends `{ is_secret, value_encrypted_bot, value_encrypted_client? }`. The ephemeral public key is embedded in the wire value (see §8.8.11).
7. Bot decrypts with its `bot_command_private_key` and the embedded ephemeral pubkey.

Non-secret settings:

- `value_encrypted_client` uses the operator's `preferences_key` (standard preferences pattern, no ECDH). Client-readable on every device.
- `value_encrypted_bot` uses X25519 ECDH. Bot-readable. Wire value is `ephemeral_pubkey (32) || nonce (12) || ciphertext || tag (16)`.

Secret settings:

- `value_encrypted_bot` only, using X25519 ECDH.
- No client-readable copy. No local plaintext mirror.

Secret handling: secret plaintext is never persisted on any client. The client displays a masked "Set" state and offers only "Set" and "Replace" actions. No local plaintext mirror.

### 6.32 Encrypted Payload Opacity

Extension and bot payloads that are encrypted by the client are opaque to the server. The server does not decrypt, parse, validate, log, or inspect them. The server spec defines only the wire shape (field names, encodings, size limits). The encryption scheme is a client-side implementation detail.

### 6.33 Publisher Key Encryption

Bot messages use a publisher key derived from the current MLS epoch secret. This provides epoch-level forward secrecy, not per-message forward secrecy.

**Derivation:**

```
publisher_secret  = MLS-Exporter("atoll", "publisher-key-v1", 32)
publisher_private = HKDF-Expand(publisher_secret, "publisher-key-x25519-v1", 32)   // X25519 scalar
publisher_public  = X25519_base(publisher_private)
```

**Bot message encryption:**

```
ephemeral_priv, ephemeral_pub = X25519_keypair()
shared = ECDH(ephemeral_priv, publisher_public)
key    = HKDF-Expand(shared, "publisher-message-v1", 32)
nonce  = random 12 bytes
ct, tag = AES-256-GCM(key, nonce, plaintext)
```

Wire format for `POST /rooms/:id/bot-messages` ciphertext field:

```
ephemeral_pubkey (32) || nonce (12) || ciphertext || tag (16)
```

The server treats the ciphertext as opaque. Clients parse the structure after decryption.

**Client decryption:**

```
ephemeral_pub = ciphertext[0..32]
nonce         = ciphertext[32..44]
ct_and_tag    = ciphertext[44..]
shared        = ECDH(publisher_private, ephemeral_pub)
key           = HKDF-Expand(shared, "publisher-message-v1", 32)
plaintext     = AES-256-GCM-decrypt(key, nonce, ct_and_tag)
```

**Info strings:** `"publisher-key-x25519-v1"` and `"publisher-message-v1"` are distinct. UTF-8, no null terminator, no length prefix. SHA-256 as the HKDF hash.

**Forward secrecy limitation:** The publisher key derives from the current MLS epoch secret. It rotates on epoch transitions. Within an epoch, bot messages do not have per-message forward secrecy. If the epoch secret is compromised, all bot messages within that epoch are decryptable. This is weaker than MLS message encryption. See §12.

---

## 7. Data Model

### 7.1 Identity and Auth

```sql
CREATE TABLE users (
    id                  TEXT PRIMARY KEY,
    -- username_token: stores lookup_token, not the raw OPRF finalization output.
    -- 86-character unpadded base64url, decodes to 64 bytes. See §6.19.
    username_token      TEXT NOT NULL UNIQUE,
    -- encrypted_display: nonce (12) || AES-256-GCM(display_name). 28–284 bytes decoded.
    encrypted_display   TEXT,
    opaque_registration BLOB NOT NULL,
    identity_pubkey     TEXT NOT NULL,
    profile             TEXT,
    profile_version     INTEGER NOT NULL DEFAULT 1,
    max_file_size_bytes INTEGER,
    created_at          DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    disabled_at         DATETIME,
    deleted_at          DATETIME
);

CREATE UNIQUE INDEX idx_users_username_token ON users(username_token);

CREATE TABLE recovery_codes (
    id          TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    code_hash   BLOB NOT NULL,
    code_salt   BLOB NOT NULL,
    consumed_at DATETIME,
    created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_recovery_codes_user ON recovery_codes(user_id, consumed_at)
    WHERE consumed_at IS NULL;

CREATE TABLE roles (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    level       INTEGER NOT NULL,
    permissions TEXT NOT NULL
);

CREATE TABLE user_roles (
    user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id    TEXT NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    granted_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    granted_by TEXT REFERENCES users(id),
    PRIMARY KEY (user_id, role_id)
);

CREATE TABLE devices (
    id          TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    client_id   TEXT NOT NULL UNIQUE,
    platform    TEXT NOT NULL CHECK(platform IN ('web', 'ios', 'android', 'desktop')),
    last_seen   DATETIME,
    created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE sessions (
    id           TEXT PRIMARY KEY,
    user_id      TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id    TEXT REFERENCES devices(id) ON DELETE CASCADE,
    token_hash   TEXT NOT NULL UNIQUE,
    created_at   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at   DATETIME NOT NULL,
    revoked_at   DATETIME,
    last_seen_at DATETIME
);

CREATE TABLE oprf_audit (
    id          TEXT PRIMARY KEY,
    event       TEXT NOT NULL,
    created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

`username_token` constraints: exactly 86 characters, base64url, decodes to 64 bytes. `encrypted_display`: base64url, decodes to 28–284 bytes. On account deletion, `username_token` is replaced with a random 86-character base64url string; `encrypted_display` and `profile` are set to NULL.

### 7.2 User-Scoped Sync

```sql
CREATE TABLE user_seq (
    user_id     TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    next_seq    INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE read_state (
    user_id              TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    room_id              TEXT NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    last_read_message_id TEXT,
    user_seq             INTEGER NOT NULL,
    updated_at           DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at           DATETIME,
    PRIMARY KEY (user_id, room_id)
);

CREATE INDEX idx_read_state_seq ON read_state(user_id, user_seq);

CREATE TABLE user_preferences (
    user_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    key             TEXT NOT NULL,
    value_encrypted TEXT NOT NULL,
    user_seq        INTEGER NOT NULL,
    updated_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (user_id, key)
);

CREATE INDEX idx_user_preferences_seq ON user_preferences(user_id, user_seq);

CREATE TABLE user_room_order (
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    room_id     TEXT NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    position    INTEGER NOT NULL,
    user_seq    INTEGER NOT NULL,
    updated_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (user_id, room_id)
);

CREATE INDEX idx_user_room_order_seq ON user_room_order(user_id, user_seq);

CREATE TABLE device_names (
    user_id               TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id             TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    encrypted_device_name TEXT NOT NULL,
    user_seq              INTEGER NOT NULL,
    updated_at            DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at            DATETIME,
    PRIMARY KEY (user_id, device_id)
);

CREATE INDEX idx_device_names_seq ON device_names(user_id, user_seq);

CREATE TABLE starred_items (
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    item_id     TEXT NOT NULL,
    item_type   TEXT NOT NULL CHECK(item_type IN ('attachment', 'message', 'link')),
    room_id     TEXT NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    user_seq    INTEGER NOT NULL,
    starred_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at  DATETIME,
    PRIMARY KEY (user_id, item_id, item_type)
);

CREATE INDEX idx_starred_user_seq ON starred_items(user_id, user_seq);
CREATE INDEX idx_starred_room ON starred_items(room_id, starred_at DESC);
```

### 7.3 Invites

Invites carry an inviter, an expiration, and an optional room reference. Consumption is one-time. Redeem via `POST /invites/redeem`; validate via `GET /invites/:code`.

### 7.4 Rooms

```sql
CREATE TABLE rooms (
    id                           TEXT PRIMARY KEY,
    owner_id                     TEXT NOT NULL REFERENCES users(id),
    metadata                     TEXT,
    metadata_version             INTEGER NOT NULL DEFAULT 1,
    retention_days               INTEGER,
    max_file_size_bytes          INTEGER,
    moderation_override          TEXT,
    sessions_per_room            INTEGER,
    bots_per_room                INTEGER,
    pending_mls_removes_per_room INTEGER,
    call_active                  INTEGER NOT NULL DEFAULT 0,
    call_participants            TEXT,
    created_at                   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE room_members (
    room_id     TEXT NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role        TEXT NOT NULL CHECK(role IN ('owner', 'moderator', 'member')),
    joined_at   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    joined_via  TEXT,
    PRIMARY KEY (room_id, user_id)
);

CREATE TABLE room_epochs (
    room_id                   TEXT PRIMARY KEY REFERENCES rooms(id) ON DELETE CASCADE,
    epoch                     INTEGER NOT NULL DEFAULT 0,
    sequence                  INTEGER NOT NULL DEFAULT 0,
    confirmed_transcript_hash BLOB,
    updated_at                DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE room_publisher_keys (
    room_id              TEXT PRIMARY KEY REFERENCES rooms(id) ON DELETE CASCADE,
    epoch                INTEGER NOT NULL,
    publisher_public_key BLOB NOT NULL,
    signer_user_id       TEXT NOT NULL REFERENCES users(id),
    signature            BLOB NOT NULL,
    published_at         DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

`room_publisher_keys` stores the latest publisher key per room. The public key, signature, and signer are public; nothing sensitive is stored. Persisted so that restart does not create a gap.

### 7.5 MLS Lifecycle

```sql
CREATE TABLE pending_mls_adds (
    id               TEXT PRIMARY KEY,
    room_id          TEXT NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    target_user_id   TEXT REFERENCES users(id),
    target_bot_id    TEXT REFERENCES bot_accounts(id),
    target_client_id TEXT NOT NULL,
    key_package_id   TEXT NOT NULL REFERENCES key_packages(id),
    queued_at        DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    consumed_at      DATETIME,
    CHECK (
        (target_user_id IS NOT NULL AND target_bot_id IS NULL)
        OR
        (target_user_id IS NULL AND target_bot_id IS NOT NULL)
    )
);

CREATE INDEX idx_pending_mls_adds_active
    ON pending_mls_adds(room_id, consumed_at)
    WHERE consumed_at IS NULL;

CREATE TABLE pending_mls_removes (
    id                  TEXT PRIMARY KEY,
    room_id             TEXT NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    target_user_id      TEXT REFERENCES users(id),
    target_bot_id       TEXT REFERENCES bot_accounts(id),
    queued_at           DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    stale_at            DATETIME,
    consumed_at         DATETIME,
    cancelled_at        DATETIME,
    remove_confirmed_at DATETIME,
    CHECK (
        (target_user_id IS NOT NULL AND target_bot_id IS NULL)
        OR
        (target_user_id IS NULL AND target_bot_id IS NOT NULL)
    )
);

CREATE INDEX idx_pending_mls_removes_active
    ON pending_mls_removes(room_id, consumed_at)
    WHERE consumed_at IS NULL AND cancelled_at IS NULL;
```

### 7.6 Messages and Reactions

```sql
CREATE TABLE room_messages (
    id                        TEXT PRIMARY KEY,
    room_id                   TEXT NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    sender_user_id            TEXT REFERENCES users(id),
    sender_bot_id             TEXT REFERENCES bot_accounts(id),
    sender_client_id          TEXT NOT NULL,
    -- bot_key_leaf_index: current active KT leaf at insertion, not signing-time leaf.
    -- Populated only when sender_bot_id is non-NULL.
    bot_key_leaf_index        INTEGER,
    epoch                     INTEGER NOT NULL,
    seq                       INTEGER NOT NULL,
    content_type              TEXT NOT NULL CHECK(content_type IN ('application', 'commit', 'proposal', 'bot')),
    ciphertext                BLOB,
    reply_to                  TEXT REFERENCES room_messages(id),
    target_user_ids           TEXT,
    edit_of                   TEXT REFERENCES room_messages(id),
    edit_sequence             INTEGER NOT NULL DEFAULT 0,
    edited_at                 DATETIME,
    created_at                DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at                DATETIME,
    CHECK (
        (sender_user_id IS NOT NULL AND sender_bot_id IS NULL)
        OR
        (sender_user_id IS NULL AND sender_bot_id IS NOT NULL)
    )
);

CREATE INDEX idx_room_messages_room_epoch_seq ON room_messages(room_id, epoch, seq);
CREATE INDEX idx_room_messages_room_created ON room_messages(room_id, created_at DESC);
CREATE INDEX idx_room_messages_sender_user ON room_messages(sender_user_id, created_at DESC)
    WHERE sender_user_id IS NOT NULL;
CREATE INDEX idx_room_messages_sender_bot ON room_messages(sender_bot_id, created_at DESC)
    WHERE sender_bot_id IS NOT NULL;
CREATE INDEX idx_room_messages_room_deleted ON room_messages(room_id, deleted_at)
    WHERE deleted_at IS NOT NULL;
CREATE INDEX idx_room_messages_reply_to ON room_messages(reply_to)
    WHERE reply_to IS NOT NULL;
CREATE INDEX idx_room_messages_edit_of ON room_messages(edit_of)
    WHERE edit_of IS NOT NULL;

CREATE TABLE reactions (
    id                 TEXT PRIMARY KEY,
    room_id            TEXT NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    message_id         TEXT NOT NULL REFERENCES room_messages(id) ON DELETE CASCADE,
    sender_user_id     TEXT NOT NULL REFERENCES users(id),
    sender_client_id   TEXT NOT NULL,
    reaction           TEXT NOT NULL,
    created_at         DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at         DATETIME,
    UNIQUE (message_id, sender_user_id, sender_client_id, reaction)
);

CREATE INDEX idx_reactions_message ON reactions(message_id)
    WHERE deleted_at IS NULL;
CREATE INDEX idx_reactions_room ON reactions(room_id, created_at DESC);
```

`bot_key_leaf_index` is populated only when `sender_bot_id` is non-NULL. It records the KT log leaf index of the bot's currently active `bot_identity_pubkey` at insertion time. Messages signed with rotated-out keys are rejected; see §8.8.8.

Edit model. An edit is a new `room_messages` row with `edit_of = <original>` and `edit_sequence = <previous + 1>`. The original row remains. Edits are rejected if `now - created_at > edit_window_seconds`.

Whisper messages. `target_user_ids` is a JSON array of user_ids for whispers, NULL for public messages. Whispers are delivered on each recipient's `private-user-{user_id}` channel and on the sender's own `private-user-{sender_user_id}` channel. The recipient list is not visible to non-recipients. The room channel receives no whisper event of any kind, including edits, deletes, and reactions. The server strips `target_user_ids` from all room channel payloads as a defensive measure.

Reaction retention. Reactions on a tombstoned message are retained until the parent message is pruned by retention. Clients MUST NOT display reactions on tombstoned messages. The server publishes `reaction.added`/`reaction.removed` for tombstoned messages; clients filter locally.

### 7.7 Attachments

Content-addressed blobs. C2SP chunked encryption. Manifest carries `file_id`, `thumbnail_file_id`, size, and content type.

### 7.8 Push Subscriptions

Web Push, APNs, FCM, UnifiedPush subscriptions. `last_used_at` tracked. Auto-revoked after 90 days of inactivity.

### 7.9 Calls and Sessions

```sql
CREATE TABLE call_sessions (
    id           TEXT PRIMARY KEY,
    room_id      TEXT NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    initiator_id TEXT NOT NULL REFERENCES users(id),
    started_at   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    ended_at     DATETIME
);

CREATE INDEX idx_call_sessions_room ON call_sessions(room_id, started_at DESC);

CREATE TABLE room_sessions (
    id               TEXT PRIMARY KEY,
    room_id          TEXT NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    extension_id     TEXT NOT NULL,
    session_type     TEXT NOT NULL,
    created_by       TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    metadata         TEXT,
    metadata_version INTEGER NOT NULL DEFAULT 1,
    position         INTEGER NOT NULL DEFAULT 0,
    max_participants INTEGER,
    created_at       DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at       DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_room_sessions_room ON room_sessions(room_id, position);
CREATE INDEX idx_room_sessions_extension ON room_sessions(room_id, extension_id, session_type);
```

Call attendance is not stored. No `call_participants` table. Occupancy is in memory only and is lost on restart.

Session occupancy is not stored. No `session_participants` table. Occupancy is in memory only.

In-memory shapes (not persisted):

```
call_occupancy: Map<call_id, {
  participants: Map<user_id, {
    client_ids: Set<client_id>,
    connection: <media route state>
  }>,
  call_started_at: Instant
}>

session_occupancy: Map<session_id, {
  participants: Map<user_id, {
    client_ids: Set<client_id>,
    last_heartbeat: Map<client_id, Instant>,
    connection: <media route state>
  }>,
  session_started_at: Instant,
  last_published_count: u32,
  last_published_at: Instant
}>

observer_cursors: Map<observer_id, { room_id: String, last_event_id: u64, expires_at: Instant }>

bot_connection_state: Map<bot_id, { connected: bool, last_seen_at: Instant }>
```

Publisher keys are persisted in `room_publisher_keys`. Bot connection state is ephemeral, in-memory, owner-scoped.

### 7.10 Bots

```sql
CREATE TABLE bot_accounts (
    id                  TEXT PRIMARY KEY,
    display_name        TEXT NOT NULL,
    avatar_file_id      TEXT,
    bot_identity_pubkey TEXT NOT NULL,
    bot_command_pubkey  TEXT NOT NULL,
    identity_pubkey     TEXT NOT NULL,
    owner_user_id       TEXT NOT NULL REFERENCES users(id),
    -- declarations: JSON object { schema_version, commands, settings }
    -- Server validates top-level shape only; contents are opaque.
    declarations        TEXT,
    created_at          DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    disabled_at         DATETIME,
    deleted_at          DATETIME
);

CREATE TABLE bot_tokens (
    id           TEXT PRIMARY KEY,
    bot_id       TEXT NOT NULL REFERENCES bot_accounts(id) ON DELETE CASCADE,
    token_hash   TEXT NOT NULL UNIQUE,
    created_at   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at   DATETIME,
    revoked_at   DATETIME,
    last_used_at DATETIME
);

CREATE INDEX idx_bot_tokens_bot ON bot_tokens(bot_id, revoked_at)
    WHERE revoked_at IS NULL;

CREATE TABLE bot_declared_scopes (
    bot_id TEXT NOT NULL REFERENCES bot_accounts(id) ON DELETE CASCADE,
    scope  TEXT NOT NULL CHECK(scope IN (
        'post_message',
        'post_attachment',
        'post_reaction',
        'read_commands',
        'read_metadata',
        'read_content',
        'edit_message',
        'delete_message'
    )),
    PRIMARY KEY (bot_id, scope)
);

CREATE TABLE room_bots (
    room_id    TEXT NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    bot_id     TEXT NOT NULL REFERENCES bot_accounts(id) ON DELETE CASCADE,
    mode       TEXT NOT NULL CHECK(mode IN ('write_only', 'observer', 'member')),
    granted_by TEXT NOT NULL REFERENCES users(id),
    granted_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    revoked_at DATETIME,
    PRIMARY KEY (room_id, bot_id)
);

CREATE TABLE room_bot_scopes (
    room_id TEXT NOT NULL,
    bot_id  TEXT NOT NULL,
    scope   TEXT NOT NULL CHECK(scope IN (
        'post_message',
        'post_attachment',
        'post_reaction',
        'read_commands',
        'read_metadata',
        'read_content',
        'edit_message',
        'delete_message'
    )),
    PRIMARY KEY (room_id, bot_id, scope),
    FOREIGN KEY (room_id, bot_id)
        REFERENCES room_bots(room_id, bot_id) ON DELETE CASCADE
);

CREATE INDEX idx_room_bot_scopes_lookup ON room_bot_scopes(scope, room_id);

CREATE TABLE bot_commands (
    id                        TEXT PRIMARY KEY,
    bot_id                    TEXT NOT NULL REFERENCES bot_accounts(id) ON DELETE CASCADE,
    room_id                   TEXT NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    sender_user_id            TEXT NOT NULL REFERENCES users(id),
    sender_client_id          TEXT NOT NULL,
    ciphertext                BLOB NOT NULL,
    ephemeral_result_pubkey   BLOB NOT NULL,
    created_at                DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    delivered_at              DATETIME,
    acked_at                  DATETIME,
    expired_at                DATETIME,
    result_at                 DATETIME,
    result_type               TEXT
);

CREATE INDEX idx_bot_commands_pending
    ON bot_commands(bot_id, acked_at)
    WHERE acked_at IS NULL AND expired_at IS NULL;

CREATE TABLE bot_settings (
    bot_id                 TEXT NOT NULL REFERENCES bot_accounts(id) ON DELETE CASCADE,
    key                    TEXT NOT NULL,
    is_secret              INTEGER NOT NULL CHECK(is_secret IN (0, 1)),
    value_encrypted_client TEXT,
    value_encrypted_bot    TEXT NOT NULL,
    user_seq               INTEGER NOT NULL,
    updated_at             DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (bot_id, key),
    CHECK (
        (is_secret = 1 AND value_encrypted_client IS NULL)
        OR
        (is_secret = 0 AND value_encrypted_client IS NOT NULL)
    )
);

CREATE INDEX idx_bot_settings_seq ON bot_settings(user_seq);

CREATE TABLE bot_request_log (
    bot_id        TEXT NOT NULL REFERENCES bot_accounts(id) ON DELETE CASCADE,
    request_id    TEXT NOT NULL,
    endpoint      TEXT NOT NULL,
    response_code INTEGER NOT NULL,
    created_at    DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (bot_id, request_id)
);

CREATE INDEX idx_bot_request_log_ttl ON bot_request_log(created_at);
```

`bot_settings.key` is opaque to the server. Room-scoped settings use the prefix `room:{room_id}:`. The server does not parse the prefix. The client SDK constructs prefixed keys transparently. There is no `scope_room_id` column.

`bot_settings.value_encrypted_bot` wire format: `ephemeral_pubkey (32) || nonce (12) || ciphertext || tag (16)`, base64url-encoded. The server does not parse. The bot parses after fetch.

`bot_accounts.declarations` is a JSON object with `schema_version`, `commands`, and `settings`. The server validates the top-level shape only. Contents are opaque. See §8.8.1 for the response shape.

Bot model:

- `bot_accounts` has no tier column. Mode is derived at grant time and stored on `room_bots.mode`.
- Every bot has a signing key (`bot_identity_pubkey`), a command encryption key (`bot_command_pubkey`), and an MLS identity key (`identity_pubkey`). All three are populated at install time.
- `bot_declared_scopes` is the author's declaration. It is a ceiling, not a grant. Grants must be a subset.
- `room_bots.mode` is derived: any of `read_content`, `post_reaction`, `edit_message`, `delete_message` present → member; else `read_metadata` present → observer; else `write_only`.
- `bot_settings.user_seq` is the operator's sequence, not the bot's. Bot settings appear in the operator's `GET /users/me/sync` response.
- `bot_commands.ephemeral_result_pubkey` is the X25519 pubkey the bot encrypts its result to.
- `bot_commands.result_at` and `result_type` are populated when the result is delivered.

### 7.11 Key Transparency and Ops

```sql
CREATE TABLE key_transparency_log (
    leaf_index       INTEGER PRIMARY KEY,
    user_id          TEXT REFERENCES users(id) ON DELETE CASCADE,
    bot_id           TEXT REFERENCES bot_accounts(id) ON DELETE CASCADE,
    username_token   TEXT,
    identity_pubkey  TEXT,
    command_pubkey   TEXT,
    added_at         DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK (
        (user_id IS NOT NULL AND bot_id IS NULL)
        OR
        (user_id IS NULL AND bot_id IS NOT NULL)
    )
);

CREATE INDEX idx_kt_user ON key_transparency_log(user_id);
CREATE INDEX idx_kt_bot ON key_transparency_log(bot_id)
    WHERE bot_id IS NOT NULL;
CREATE INDEX idx_kt_added ON key_transparency_log(added_at);

CREATE TABLE key_transparency_snapshots (
    id           TEXT PRIMARY KEY,
    tree_size    INTEGER NOT NULL,
    root_hash    BLOB NOT NULL,
    signature    BLOB,
    created_at   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

The `username_token` column is stored for user entries. It is excluded from all KT API responses. See §8.11 and §12.

### 7.12 Server Config

```sql
CREATE TABLE server_config (
    key         TEXT PRIMARY KEY,
    value       TEXT NOT NULL,
    is_secret   INTEGER NOT NULL DEFAULT 0 CHECK(is_secret IN (0, 1)),
    created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

Seeded at first boot with `sockudo_app_key` (`is_secret = 0`) and `sockudo_app_secret` (`is_secret = 1`) if not set by environment variables. Environment variables, when set, override the table and are not written back.

Rotation: not supported in v3.0. Both values are immutable once generated. A future amendment may add rotation; it is out of scope.

Backup: both values are included in database backups. The app secret must be treated as sensitive data in backup storage.

### 7.13 State Outside the Database

| Path | Purpose | Notes |
|---|---|---|
| In-memory call occupancy | Call participation | Never persisted. Lost on restart. |
| In-memory session occupancy | Session participation | Never persisted. Lost on restart. |
| In-memory bot connection state | Server-to-bot liveness | Never persisted. Never logged. Lost on restart. |
| In-memory observer cursors | Observer stream resume state | 5 minutes after disconnect. |
| `MODEL_STORAGE_PATH` | Model files | Immutable. Optional in external mode. |
| `SESSION_TYPES_CONFIG_PATH` | Session type allowlist | Operator-maintained. |
| `EXTENSION_PROXY_DENY_DOMAINS_PATH` | Domain blocklist | Operator-maintained. |

Publisher keys are persisted in `room_publisher_keys`. They are not in-memory.

---

## 8. API Surface

All routes prefixed with `/api/v1/` unless noted. Auth via `Authorization: Bearer <token>` unless noted. Bot tokens use the same header format.

Exception: `POST /sockudo/auth` is mounted at the reverse-proxy root, not under `/api/v1/`. Its path is determined by `SOCKUDO_AUTH_PATH`. This mirrors the Pusher protocol: the auth endpoint is a peer of the WebSocket path, not a member of the application API.

Error format:

```json
{ "error": "machine_readable_code", "message": "Human-readable message", "details": {} }
```

### 8.0 Static Client Hosting

SPA served from `CLIENT_STATIC_DIR` with SPA fallback. HTTPS enforced in production.

### 8.1 Public

| Method | Path | Purpose |
|---|---|---|
| GET | /health | Liveness |
| GET | /ready | Readiness |
| GET | /capabilities | Feature advertisement |
| GET | /auth/register/challenge | ALTCHA challenge |
| POST | /auth/register/start | OPAQUE registration start |
| POST | /auth/register/finish | OPAQUE registration finish |
| POST | /auth/login/start | OPAQUE login start |
| POST | /auth/login/finish | OPAQUE login finish |
| POST | /auth/recover/start | Recovery authorization |
| POST | /auth/recover/finish | Recovery credential install |
| POST | /invites/redeem | Validate and consume server invite |
| GET | /invites/:code | Public invite validation |
| POST | /oprf/blind | OPRF evaluation |
| POST | /link-preview/proxy | Link preview proxy (opt-in) |
| POST | /extensions/proxy | Extension proxy (opt-in, requires auth) |
| GET | /models/stt/v1/:model_id/:version/:filename | STT model files |
| GET | /models/tts/v1/:model_id/:version/:filename | TTS model files |
| GET | /models/manifest.json | Model manifest |
| POST | /sockudo/auth | Sockudo subscription authentication |

`GET /capabilities` (excerpt):

```json
{
  "version": "3.0.3",
  "sockudo_url": "wss://chat.example.com/sockudo",
  "sockudo_app_key": "a1b2c3d4e5f6g7h8",
  "sockudo_auth_endpoint": "/sockudo/auth",
  "calling": true,
  "call_max_participants": 8,
  "sessions_enabled": true,
  "max_sessions_per_room": 10,
  "max_session_participants": 50,
  "session_types": [ ... ],
  "model_hosting_enabled": true,
  "model_hosting_mode": "local",
  "stt_models_base_url": "...",
  "tts_models_base_url": "...",
  "key_transparency_enabled": true,
  "key_transparency_auditor_count": 0,
  "link_preview_proxy_enabled": false,
  "extension_proxy_enabled": false,
  "extension_proxy_max_request_bytes": 262144,
  "extension_proxy_max_response_bytes": 10485760,
  "extension_proxy_supports_streaming": false,
  "safety_number_mode": "warn",
  "moderation_mode": "messenger",
  "edit_window_seconds": 900,
  "reactions_per_message": 50,
  "sync_event_retention_days": 90,
  "threading_enabled": true,
  "starred_items_per_user": 10000,
  "bots_enabled": true,
  "bot_capabilities": [
    "post_message",
    "post_attachment",
    "post_reaction",
    "read_commands",
    "read_metadata",
    "read_content",
    "edit_message",
    "delete_message"
  ],
  "observer_stream_enabled": true,
  "bot_identity_in_kt": true,
  "bot_max_per_room": 10,
  "bot_max_per_user": 50,
  "preferences_max_encrypted_bytes": 131072,
  "publisher_key_derivation": "mls-exporter-v1"
}
```

The three `sockudo_*` fields are included in the unauthenticated response. The app key is public by Pusher design; the URL is the address clients must connect to; the auth endpoint path is public and requires a bearer token when called. Gating any of these behind authentication adds a boot-sequence dependency for no security gain. `sockudo_channel_prefix` is deliberately not exposed; channel prefixes are structural and clients hardcode them.

#### 8.1.1 POST /oprf/blind

Auth: None. Rate limits: `RATE_OPRF_BLIND_PER_MIN`, `RATE_OPRF_BLIND_PER_HOUR`, both per IP.

Request: `{ "blinded": "<base64>" }`
Response: `{ "evaluated": "<base64>" }`

#### 8.1.2 POST /link-preview/proxy

Auth: None. Rate limit: `RATE_LINK_PREVIEW_PER_MIN`, per IP.

Request: `{ "encrypted_url": "<base64>", "wrapped_content_key": "<base64, RSA-OAEP>", "nonce": "<base64, 12 bytes>" }`
Response: `{ "encrypted_metadata": "<base64>", "nonce": "<base64, 12 bytes>" }`

SSRF protection: DNS resolution with private/loopback/link-local/multicast rejection; IP pinning; redirect re-validation; HTTP(S) only; body size cap; timeout; strip all HTML.

#### 8.1.3 GET /models/stt/v1/... and GET /models/tts/v1/...

Auth: None. Rate limit: `RATE_MODEL_DOWNLOAD_PER_MIN`, per IP.

Behavior: serve static model files with immutable cache headers. In external mode, this endpoint is not registered.

Headers: `Cache-Control: public, max-age=31536000, immutable`. `Content-Type: application/octet-stream`. ETag from file hash. Range requests supported.

#### 8.1.4 GET /models/manifest.json

Auth: None. Returns the combined model manifest.

#### 8.1.5 POST /extensions/proxy

Auth: Required. `Content-Type: application/octet-stream`.

Rate limits: two buckets, both must pass. Per user per extension (`EXTENSION_PROXY_MAX_REQUESTS_PER_MIN`/`_PER_HOUR`), and per user total (`EXTENSION_PROXY_MAX_REQUESTS_PER_MIN_TOTAL`/`_PER_HOUR_TOTAL`). Bandwidth counted against both.

Request: Content Key encryption of `{ url, method, headers, body, extension_id, request_id }`.

Constraints: URL HTTPS only, private IPs rejected, max 2048 chars. Method GET/POST/HEAD. Headers allowlist: `Accept`, `Accept-Language`, `Accept-Encoding`, `Cache-Control`, `If-None-Match`, `If-Modified-Since`, `If-Range`, `Range`. `Authorization` and `Cookie` rejected outright. User-Agent forced. Body max 256 KB, POST only. `extension_id` advisory. `request_id` echoed.

Response: Content Key encryption of `{ status, headers, body, request_id }`. Response header allowlist: `content-type`, `content-length`, `etag`, `last-modified`, `cache-control`, `expires`, `date`, `vary`, `location`, `retry-after`.

Errors: `400 invalid_request`, `400 url_blocked`, `400 domain_blocked`, `400 header_not_allowed`, `400 method_not_allowed`, `413 request_too_large`, `502 upstream_response_too_large`, `504 upstream_timeout`, `429 rate_limited`/`bandwidth_limited`, `501 extension_proxy_disabled`, `500 internal`.

#### 8.1.6 POST /sockudo/auth

Auth: Required. `Authorization: Bearer <token>` — accepts a user session token or a bot token. The token lookup tries both the `sessions` and `bot_tokens` tables and determines the identity from whichever matched.

Mounted at the reverse-proxy root, not under `/api/v1/`. Path configurable via `SOCKUDO_AUTH_PATH`; default `/sockudo/auth`.

Rate limit: `RATE_SOCKUDO_AUTH_PER_MIN=300`, keyed `sockudo_auth:{identity}:min:{boundary}` where `{identity}` is `user:{user_id}` or `bot:{bot_id}`.

Request:

```json
{
  "socket_id": "<pusher socket id>",
  "channel_name": "<channel>"
}
```

Validation:

1. `socket_id` must match `^\d+\.\d+$`. Reject with `400 invalid_request` otherwise.
2. `channel_name` must match `^private-(room|user|bot)-[a-zA-Z0-9_-]+$`. Reject with `403 forbidden` otherwise.

Authorization check:

| Channel pattern | Allowed identity |
|---|---|
| `private-room-{room_id}` | User or member-tier bot with a `room_members` or `room_bots` row for `{room_id}` |
| `private-user-{user_id}` | User whose `id = {user_id}`; or the bot whose `id = {bot_id}` when the user_id matches `bot_accounts.owner_user_id` |
| `private-bot-{bot_id}` | Bot whose `id = {bot_id}`; or the user whose `id = bot_accounts.owner_user_id` |

Additional rejections:

- `bot_accounts.disabled_at IS NOT NULL` → `403 forbidden`.
- `users.disabled_at IS NOT NULL` → `403 forbidden`.
- Unknown channel prefix → `403 forbidden`. The server does not forward auth for arbitrary channel names.

Response (200):

```json
{
  "auth": "<app_key>:<hex-hmac>"
}
```

HMAC computation:

```
signature = HMAC-SHA256(app_secret, socket_id + ":" + channel_name)
auth      = app_key + ":" + hex(signature)
```

UTF-8 encoding for all strings. Hex output is lowercase. No trailing newline.

Errors: `400 invalid_request`, `401 unauthorized` (unknown, expired, or revoked token), `403 forbidden` (valid token, cannot subscribe to this channel), `429 rate_limited`, `503 service_unavailable` (database unavailable).

Behavior notes:

- The server does not proactively close existing WebSocket subscriptions when a user is removed from a room or a bot is disabled. Closing would require server-side subscription tracking that duplicates Sockudo's state. The next subscribe attempt fails. Clients handle this via `room.member_removed` events.
- The endpoint must respond in under 50ms under normal load. If the database is unavailable, `503 service_unavailable` is returned.

### 8.2 User

| Method | Path | Purpose |
|---|---|---|
| POST | /auth/logout | Revoke session |
| GET | /users/me | Current user |
| PATCH | /users/me | Update profile |
| DELETE | /users/me | Delete account |
| GET | /users/me/export | Export data |
| POST | /users/lookup | Lookup token lookup |
| GET | /users/me/devices | List devices |
| DELETE | /users/me/devices/:id | Revoke device |
| GET | /users/me/sessions | List HTTP sessions |
| DELETE | /users/me/sessions/:id | Revoke HTTP session |
| POST | /users/me/push-subscriptions | Register push subscription |
| DELETE | /users/me/push-subscriptions/:id | Revoke push subscription |
| GET | /users/me/sync | Fetch user-scoped state |
| POST | /users/me/read-state | Write read state |
| PATCH | /users/me/room-order | Write room order |
| GET | /users/me/preferences/:key | Read a preference |
| PATCH | /users/me/preferences/:key | Write a preference |
| DELETE | /users/me/preferences/:key | Delete a preference |
| GET | /users/me/starred-items | List starred items |
| POST | /users/me/starred-items | Star an item |
| DELETE | /users/me/starred-items/:item_id | Unstar an item |
| POST | /users/me/avatar | Upload avatar |

#### 8.2.1 GET /users/me

```json
{
  "id": "<user_id>",
  "username_token": "<base64url, lookup_token>",
  "encrypted_display": "<base64url>",
  "identity_pubkey": "<base64url>",
  "profile": "<base64url>",
  "profile_version": 1,
  "created_at": "<ISO 8601>"
}
```

#### 8.2.2 PATCH /users/me

Request: `{ "encrypted_display": "<base64url>", "profile": "<base64url>" }`.

#### 8.2.3 POST /users/lookup

Auth: Required. Rate limit: `RATE_LOOKUP_PER_MIN`, per authenticated user.

Request: `{ "lookup_token": "<86-char base64url>" }`

Found (200): `{ "user_id": "...", "encrypted_display": "<base64url or null>" }`
Not found (404): `{ "error": "not_found" }`

Rate limiting is the enumeration defense. Timing padding is not applied.

#### 8.2.4 GET /users/me/sync

Query: `since_seq` (integer, required; 0 for full sync).

Response:

```json
{
  "read_state": [ ... ],
  "user_preferences": [ ... ],
  "device_state": [ ... ],
  "starred_items": [ ... ],
  "bot_settings": [
    {
      "bot_id": "b_...",
      "key": "webhook_secret",
      "is_secret": true,
      "value_encrypted_client": null,
      "user_seq": 1234
    }
  ],
  "max_seq": 1240,
  "full_resync_required": false
}
```

Applied atomically. A partial application does not advance the cursor.

#### 8.2.5 POST /users/me/read-state

Request: `{ "room_id": "...", "last_read_message_id": "..." }`.

#### 8.2.6 PATCH /users/me/room-order

Request: `{ "room_ids": ["r1", "r2", "r3"] }`.

Behavior: server replaces `user_room_order` rows for the user; bumps `user_seq`; publishes `room_order.sync`. Last-writer-wins.

#### 8.2.7 POST /users/me/avatar

Request: `multipart/form-data` with a single file part. Response: 201 with the attachment record.

#### 8.2.8 Preferences

- `GET /users/me/preferences/:key` — returns `{ "key": "...", "value": "<base64url>", "user_seq": N }` or 404. Value is ciphertext.
- `PATCH /users/me/preferences/:key` — request body `{ "value": "<base64url>" }`. Key pattern validated. Encrypted size limit 128 KB. Reserved: `room_order` (own endpoint). `_` prefix reserved.
- `DELETE /users/me/preferences/:key` — deletes the row. Publishes `preference.updated` with a tombstone.

#### 8.2.9 Starred Items

- `GET /users/me/starred-items` — query `type`, `room_id`, `limit` (default 100, max 500), `cursor`.
- `POST /users/me/starred-items` — request `{ item_id, item_type, room_id }`. Enforces `starred_items_per_user`. Publishes `starred_item.added`.
- `DELETE /users/me/starred-items/:item_id` — query `item_type`. Writes tombstone. Publishes `starred_item.removed`.

### 8.3 Admin

| Method | Path | Purpose |
|---|---|---|
| POST | /admin/oprf/rotate | Rotate the username OPRF key |
| GET | /admin/key-transparency | KT log stats |
| POST | /admin/key-transparency/snapshot | Trigger a signed snapshot |
| GET | /admin/rooms/:id/sessions | Read-only session list |
| POST | /admin/models/reload | Reload model manifest |
| POST | /admin/session-types/reload | Reload session types config |
| POST | /admin/extension-proxy/reload-blocklist | Reload domain blocklist |

#### 8.3.1 POST /admin/session-types/reload

Behavior: read `SESSION_TYPES_CONFIG_PATH`, validate, atomically swap. On error, previous config remains in effect and 400 is returned.

#### 8.3.2 POST /admin/extension-proxy/reload-blocklist

Behavior: read `EXTENSION_PROXY_DENY_DOMAINS` and `EXTENSION_PROXY_DENY_DOMAINS_PATH`, validate, atomically swap. Malformed entries logged as warnings and skipped.

### 8.4 Rooms

| Method | Path | Purpose |
|---|---|---|
| POST | /rooms | Create room |
| GET | /rooms/:id | Room details |
| PATCH | /rooms/:id | Update metadata |
| DELETE | /rooms/:id | Delete room |
| GET | /rooms/:id/members | List members (merged with member-tier bots) |
| POST | /rooms/:id/members | Add member |
| DELETE | /rooms/:id/members/:user_id | Remove member |
| POST | /rooms/:id/retention/preview | Retention change preview |
| POST | /rooms/:id/transfer-ownership | Initiate ownership transfer |
| POST | /rooms/:id/transfer-ownership/accept | Accept transfer |
| POST | /rooms/:id/transfer-ownership/cancel | Cancel transfer |

`GET /rooms/:id/members` response:

```json
{
  "members": [
    { "type": "user", "user_id": "u_...", "role": "member", "joined_at": "..." },
    { "type": "bot", "bot_id": "b_...", "mode": "write_only"|"observer"|"member",
      "display_name": "GitHub Bot", "avatar_file_id": "...", "joined_at": "..." }
  ],
  "next_cursor": "..."
}
```

### 8.5 MLS and Messaging

| Method | Path | Purpose |
|---|---|---|
| GET | /rooms/:id/messages | Delta sync fetch for room messages |
| POST | /rooms/:id/messages | Send message (public or whisper) |
| PATCH | /rooms/:id/messages/:msg_id | Edit message |
| DELETE | /rooms/:id/messages/:msg_id | Delete message |
| POST | /rooms/:id/messages/:msg_id/reactions | Add reaction |
| DELETE | /rooms/:id/messages/:msg_id/reactions/:reaction_id | Remove reaction |

#### 8.5.1 GET /rooms/:id/messages

Auth: Required. Room members only.

Query parameters:
- `since_epoch` (integer, optional)
- `since_seq` (integer, optional)
- `limit` (integer, default 100, max 500)
- `cursor` (string, optional)

Response:

```json
{
  "messages": [
    {
      "id": "m_...",
      "room_id": "r_...",
      "sender_type": "user" | "bot",
      "sender_id": "u_... | b_...",
      "sender_client_id": "...",
      "epoch": 42,
      "seq": 103,
      "content_type": "application" | "commit" | "proposal" | "bot",
      "ciphertext": "<base64url>",
      "reply_to": "m_..." | null,
      "target_user_ids": ["u_..."] | null,
      "edit_of": "m_..." | null,
      "edit_sequence": 0,
      "bot_key_leaf_index": 12345 | null,
      "read_by_count": 0,
      "created_at": "...",
      "deleted_at": "..." | null
    }
  ],
  "next_cursor": "..." | null
}
```

Visibility: room members only. Whispers (`target_user_ids` non-null) are returned only to recipients and the sender.

`read_by_count` is the aggregate read receipt count for the message. The client renders it from the message record, updated live by `read.count` events.

`bot_key_leaf_index` is present iff `sender_type = "bot"`. Absent on user messages.

`POST /rooms/:id/messages` accepts an optional `target_user_ids` field. If present, the message is a whisper and is delivered on each recipient's user channel and on the sender's own user channel. If absent, the message is public and is published on the room channel.

### 8.6 Attachments

Presign, upload, download, range requests, deletion.

### 8.7 Calls and Sessions

| Method | Path | Purpose |
|---|---|---|
| GET | /welcomes | Fetch pending welcomes for the authenticated user |
| GET | /welcomes/:id | Fetch welcome bytes |
| POST | /calls/turn-credentials | Fetch TURN credentials |
| GET | /rooms/:id/calls | Call history for the room |
| POST | /rooms/:id/calls/:call_id/join | Register call participation |
| POST | /rooms/:id/calls/:call_id/leave | Deregister call participation |
| POST | /rooms/:id/calls/:call_id/signal | Send call signaling message |
| POST | /rooms/:id/calls/:call_id/end | End call |
| GET | /rooms/:id/sessions | List sessions with live counts |
| POST | /rooms/:id/sessions | Create a session |
| PATCH | /rooms/:id/sessions/:session_id | Update metadata / position |
| DELETE | /rooms/:id/sessions/:session_id | Delete a session |
| POST | /rooms/:id/sessions/:session_id/join | Join a session |
| POST | /rooms/:id/sessions/:session_id/leave | Leave a session |
| POST | /rooms/:id/sessions/:session_id/heartbeat | Heartbeat |
| GET | /rooms/:id/sessions/:session_id/roster | Current participants (gated) |
| POST | /rooms/:id/sessions/:session_id/signal | Relay signaling |

#### 8.7.1 GET /welcomes

Auth: Required.

Response:

```json
{
  "welcomes": [
    { "welcome_id": "w_...", "room_id": "r_...", "created_at": "..." }
  ]
}
```

Returns the pending MLS welcomes for the authenticated user. Used as a fallback when `mls.welcome_ready` is missed.

#### 8.7.2 GET /welcomes/:id

Auth: Required. Returns the raw welcome bytes for the specified welcome.

#### 8.7.3 POST /calls/turn-credentials

Request: `{}`.
Response: `{ "url": "...", "username": "...", "credential": "...", "ttl": 600 }`.

#### 8.7.4 GET /rooms/:id/calls

Auth: Required. Room members only.

Query parameters:
- `limit` (integer, default 50, max 200)
- `cursor` (string, optional)

Response:

```json
{
  "calls": [
    {
      "id": "c_...",
      "initiator_id": "u_...",
      "started_at": "...",
      "ended_at": "..." | null,
      "duration_seconds": 342 | null
    }
  ],
  "next_cursor": "..." | null
}
```

Visibility: all room members. Retention: `CALL_RECORD_RETENTION_DAYS`.

#### 8.7.5 POST /rooms/:id/calls/:call_id/join

Request: `{ "client_id": "<client_id>" }`.
Response: `{ "ice_servers": [ ... ] }`.

Behavior: add client to in-memory call occupancy. Return ICE config.

#### 8.7.6 POST /rooms/:id/calls/:call_id/leave

Request: `{ "client_id": "<client_id>" }`.
Response: 204.

Behavior: remove client from occupancy. Any `client_id` of the authenticated user may be specified.

#### 8.7.7 POST /rooms/:id/calls/:call_id/signal

Request:

```json
{
  "sender_client_id": "<client_id>",
  "target_user_id": "<user_id>",
  "target_client_id": "<client_id>",
  "envelope": "<base64url>"
}
```

Behavior: verify caller is in occupancy; verify `sender_client_id` belongs to caller; verify `target_client_id` is a current participant; publish `call.signal` on `private-user-{target_user_id}`; return 200 with `{ "delivered_to": 1 }`.

Rate limit: `RATE_CALL_SIGNAL_PER_MIN`, per user, per call.

#### 8.7.8–8.7.16 Sessions

Sessions use the envelope format. `POST /rooms/:id/sessions/:session_id/signal` request body:

```json
{
  "sender_client_id": "<client_id>",
  "target_client_id": "<client_id, optional>",
  "envelope": "<base64url>"
}
```

The server does not inspect the envelope. `session.occupancy` carries only a count.

### 8.8 Bots

| Method | Path | Purpose |
|---|---|---|
| POST | /bots | Create bot account |
| GET | /bots/:id | Read bot (includes declarations) |
| PATCH | /bots/:id | Update bot (including declarations) |
| DELETE | /bots/:id | Delete bot |
| PATCH | /bots/:id/keys | Rotate bot identity and command keys |
| POST | /bots/:id/avatar | Upload or replace avatar |
| DELETE | /bots/:id/avatar | Remove avatar |
| POST | /bots/:id/tokens | Issue token |
| GET | /bots/:id/tokens | List tokens |
| DELETE | /bots/:id/tokens/:token_id | Revoke token |
| GET | /rooms/:id/bots | List bots in room |
| POST | /rooms/:id/bots | Grant bot to room |
| PATCH | /rooms/:id/bots/:bot_id | Update scopes |
| DELETE | /rooms/:id/bots/:bot_id | Revoke grant |
| POST | /rooms/:id/bot-messages | Bot posts a message |
| POST | /rooms/:id/bot-commands | User sends a command to a bot |
| GET | /rooms/:id/observer-stream | Observer metadata SSE |
| GET | /bots/me/settings | Bot reads its own settings |
| GET | /users/me/bots/:bot_id/settings | Operator reads a bot's settings |
| PATCH | /users/me/bots/:bot_id/settings/:key | Operator writes a bot setting |
| DELETE | /users/me/bots/:bot_id/settings/:key | Operator deletes a bot setting |
| POST | /bots/me/messages | Bot-originated user-channel message (invoker or owner) |
| POST | /bots/me/commands/:id/ack | Bot acknowledges a command |
| GET | /rooms/:id/publisher-key | Fetch current or specific publisher key |
| POST | /rooms/:id/publisher-key | Publish publisher key for the current epoch |

#### 8.8.1 POST /bots

Auth: User session only. A bot token cannot create another bot.

Request:

```json
{
  "display_name": "GitHub Relay",
  "bot_identity_pubkey": "<base64url, Ed25519>",
  "bot_command_pubkey": "<base64url, X25519>",
  "identity_pubkey": "<base64url, Ed25519, MLS>",
  "declared_scopes": ["post_message", "post_attachment", "read_commands"],
  "declarations": {
    "schema_version": 1,
    "commands": [ ... ],
    "settings": [ ... ]
  }
}
```

`declarations` is optional at creation. It may be uploaded later via `PATCH /bots/:id`.

Response: the bot record plus an initial `bot_token` (shown once). Also publishes `bot.created` on `private-user-{owner_user_id}` (durable).

Validation:

- `declared_scopes` must all be in the standard vocabulary.
- Scope dependencies must hold (`post_reaction`, `edit_message`, `delete_message` require `read_content`).
- If `declarations` is present, the server validates the top-level shape only (`schema_version` integer, `commands` array, `settings` array). Contents are opaque.
- Rate limit: `RATE_BOT_CREATE_PER_HOUR`.

#### 8.8.2 GET /bots/:id

Auth: Required (owner session or bot token matching `:id`).

Response:

```json
{
  "id": "b_...",
  "display_name": "...",
  "avatar_file_id": "..." | null,
  "owner_user_id": "u_...",
  "bot_identity_pubkey": "...",
  "bot_command_pubkey": "...",
  "identity_pubkey": "...",
  "declared_scopes": ["post_message", "..."],
  "declarations": {
    "schema_version": 1,
    "commands": [ ... ],
    "settings": [ ... ]
  } | null,
  "created_at": "..."
}
```

`declarations` is `null` if the bot has not published any. Clients cache declarations and invalidate on `bot.updated` with `changed` including `"commands"`.

#### 8.8.3 PATCH /bots/:id

Auth: Owner session or bot token matching `:id`.

Request accepts any subset of:

```json
{
  "display_name": "...",
  "declarations": {
    "schema_version": 1,
    "commands": [ ... ],
    "settings": [ ... ]
  }
}
```

When `declarations` is present:

1. Validate top-level shape.
2. Update `bot_accounts.declarations`.
3. Publish `bot.updated` with `changed: ["commands"]` on `private-bot-{bot_id}` and on each granted room channel.

#### 8.8.4 POST /bots/:id/avatar

Request: `multipart/form-data` with a single file part. Auth: bot owner or bot token.

Behavior: same processing as `POST /users/me/avatar`. Content-addressed. Stores `avatar_file_id` on `bot_accounts`. Publishes `bot.updated` on `private-bot-{bot_id}` and on each room channel the bot is granted to, with `changed: ["avatar"]`.

Response: 201 with the attachment record.

#### 8.8.5 DELETE /bots/:id/avatar

Behavior: set `avatar_file_id = NULL` on `bot_accounts`. Publish `bot.updated` with `changed: ["avatar"]`. Blob is pruned by the cleanup job when unreferenced.

Response: 204.

#### 8.8.6 GET /rooms/:id/bots

Auth: Required. Room members.

Response:

```json
{
  "bots": [
    {
      "bot_id": "b_...",
      "display_name": "...",
      "avatar_file_id": "..." | null,
      "mode": "write_only" | "observer" | "member",
      "scopes": ["post_message", "..."],
      "connected": true | false,
      "granted_at": "...",
      "granted_by": "u_..."
    }
  ]
}
```

`connected` is a server-to-bot liveness indicator. It is `true` if the bot has at least one open WebSocket connection to Sockudo subscribed to `private-bot-{bot_id}`. It is ephemeral, in-memory only, never persisted, never logged, never synced. Fetched on demand by this endpoint only. Not user presence.

The `connected` field is included in the response **only when the authenticated caller is the bot's `owner_user_id`**. For all other callers, the field is omitted from the response.

#### 8.8.7 POST /rooms/:id/bots

Request:

```json
{
  "bot_id": "b_...",
  "scopes": ["post_message", "read_metadata"]
}
```

Behavior:

1. Verify caller has permission (owner by default; moderator in Discord mode).
2. Verify every scope is in `bot_declared_scopes` for this bot.
3. Verify scope dependencies hold.
4. Derive mode from the scope set.
5. Insert `room_bots` row with derived mode and `room_bot_scopes` rows.
6. If derived mode is `member`, queue `pending_mls_adds` for the bot.
7. Publish `bot.added` on the room channel.

Errors: `400 scope_not_declared`, `400 scope_dependency_missing`, `409 session_limit_reached` (bot limit), `429 rate_limited`.

#### 8.8.8 PATCH /rooms/:id/bots/:bot_id

Request: `{ "scopes": [...] }`.

Behavior:

1. Validate scopes as in grant.
2. Compute old mode and new mode.
3. Update `room_bots.mode` and `room_bot_scopes`.
4. Publish `bot.grant_updated` on `private-bot-{bot_id}` with `{ bot_id, room_id, old_mode, new_mode, scopes, changed }`.
5. If transition to `member`: queue `pending_mls_adds`.
6. If transition away from `member`: queue `pending_mls_removes`.
7. Publish `bot.updated` on the room channel with `{ room_id, bot_id, scopes, avatar_file_id, changed }`.

#### 8.8.9 DELETE /rooms/:id/bots/:bot_id

Behavior: revoke grant. If mode was `member`, queue `pending_mls_removes`. Publish `bot.revoked` on the room channel.

#### 8.8.10 POST /rooms/:id/bot-messages

Auth: Bot token matching `bot_id`.

Request:

```json
{
  "epoch": 42,
  "ciphertext": "<base64url>",
  "content_type": "bot",
  "signature": "<base64url, Ed25519>",
  "request_id": "<idempotency key>"
}
```

`signature` is Ed25519 over the length-prefixed concatenation defined in §8.10:

```
u32_be(len(room_id)) || room_id
|| epoch_u64_be
|| u32_be(len(content_type)) || content_type
|| u32_be(len(ciphertext)) || ciphertext
```

Signed with the bot's `bot_identity_private_key`. Verified by the server against the **currently active** `bot_identity_pubkey` from `bot_accounts`. Messages signed with rotated-out keys are rejected with `400 signature_invalid`.

`request_id` is optional. If provided, the server uses `bot_request_log` to deduplicate. A duplicate request_id for the same endpoint returns the original response. A different endpoint with the same request_id returns `409 request_id_conflict`.

Behavior: verify bot grant has `post_message`; verify epoch within window (current + 3 past); verify signature; insert `room_messages` row with `sender_bot_id`, `content_type = "bot"`, and `bot_key_leaf_index` set to the current active KT leaf for the bot; publish `message.new`.

Errors: `403 bot_not_granted`, `400 scope_missing`, `400 epoch_out_of_window`, `400 signature_invalid`, `409 request_id_conflict`, `429 rate_limited`.

#### 8.8.11 POST /rooms/:id/bot-commands

Request:

```json
{
  "bot_id": "b_...",
  "ciphertext": "<base64url>",
  "ephemeral_result_pubkey": "<base64url, X25519>",
  "request_id": "<idempotency key>"
}
```

`ephemeral_result_pubkey` is the client's ephemeral X25519 public key for the command result. See §6.31.

Behavior: verify bot grant has `read_commands`; store in `bot_commands` with `ephemeral_result_pubkey`; publish `bot.command_invoked` on `private-bot-{bot_id}` with `{ command_id, room_id, sender_user_id, sender_client_id, ciphertext, ephemeral_result_pubkey }`.

Rate limit: `RATE_BOT_COMMAND_PER_MIN`, per bot per user.

#### 8.8.12 POST /bots/me/messages

Auth: Bot token.

Request:

```json
{
  "target": "invoker" | "owner",
  "target_user_id": "<user_id>",
  "command_id": "<cmd_id>",
  "result_type": "local_message" | "toast" | "panel" | "none",
  "ciphertext": "<base64url>",
  "bot_result_pubkey": "<base64url, X25519>",
  "request_id": "<idempotency key>"
}
```

Field requirements:

| Field | target: "invoker" | target: "owner" |
|---|---|---|
| `target_user_id` | Required; must match `bot_commands.sender_user_id` for `command_id` | Absent; derived from `bot_accounts.owner_user_id` |
| `command_id` | Required | Absent |
| `result_type` | Required | Required |
| `ciphertext` | Required unless `result_type = "none"` | Required |
| `bot_result_pubkey` | Required unless `result_type = "none"` | Required |
| `request_id` | Optional | Optional |

Behavior:

For `target: "invoker"`:
1. Verify caller is the bot identified by the token.
2. Verify `command_id` exists in `bot_commands` and belongs to this bot.
3. Verify `acked_at IS NOT NULL` (the command must have been acked before a result is delivered).
4. If `result_type = "none"`: mark command completed with no result. Return 204.
5. Otherwise: publish `bot.command_result` on `private-user-{sender_user_id}` with `{ command_id, bot_id, room_id, result_type, ciphertext, bot_result_pubkey }`.
6. Mark the command completed: set `result_at = now()`, `result_type`.

For `target: "owner"`:
1. Verify caller is the bot.
2. Publish `bot.local_message` on `private-user-{owner_user_id}` with `{ bot_id, result_type, ciphertext, bot_result_pubkey }`.
3. Return 204.

Rate limits:
- `RATE_BOT_COMMAND_RESULT_PER_MIN` for `target: "invoker"`.
- `RATE_BOT_LOCAL_MESSAGE_PER_MIN` for `target: "owner"`.

#### 8.8.13 GET /rooms/:id/observer-stream

Auth: bot token with `read_metadata` scope.

Response: `text/event-stream`.

Event filter (server-side): `message.new` with `{ id, sender_user_id, sender_client_id, created_at, size_bytes }`; `message.edited` with `{ id, edit_of, editor_user_id, edited_at }`; `message.deleted` with `{ id }`; `room.member_added`, `room.member_removed`, `room.updated` (non-encrypted fields).

Never sent: ciphertext, session events, call events, epoch events, key packages, welcomes, `session.occupancy`, `session.signal`, `call.signal`.

Backpressure: 60-second buffer, drop oldest, send `dropped { count }`. Reconnection via `Last-Event-ID`. Cursor retained 5 minutes after disconnect.

#### 8.8.14 Bot settings

`GET /bots/me/settings` — bot returns `{ key, value_encrypted_bot, is_secret, user_seq }[]`. The `value_encrypted_bot` field is a single base64url string encoding `ephemeral_pubkey (32) || nonce (12) || ciphertext || tag (16)`. The bot parses the structure after fetch. The server does not parse.

`GET /users/me/bots/:bot_id/settings` — operator returns `{ key, value_encrypted_client, is_secret, user_seq }[]`; `value_encrypted_client` NULL for secrets.

`PATCH /users/me/bots/:bot_id/settings/:key` — request body:

```json
{
  "is_secret": false,
  "value_encrypted_bot": "<base64url>",
  "value_encrypted_client": "<base64url>"
}
```

`value_encrypted_bot` embeds the ephemeral X25519 pubkey. See §6.31 for the construction. The `ephemeral_pubkey` is not a separate request field.

Validation:

| `is_secret` | `value_encrypted_bot` | `value_encrypted_client` | Result |
|---|---|---|---|
| false | present | present | accept |
| false | present | absent | 400 `client_ciphertext_required` |
| true | present | absent | accept |
| true | present | present | 400 `client_ciphertext_forbidden` |

`DELETE /users/me/bots/:bot_id/settings/:key` — 204.

#### 8.8.15 GET /rooms/:id/publisher-key

Auth: Required. Room members.

Query: `epoch` (integer, optional).

Behavior:
- With `?epoch=N`: return the key for that specific epoch if in `room_publisher_keys`; else 404 `publisher_key_unavailable`.
- Without `?epoch`: return the current epoch's key; else 404 `publisher_key_unavailable`.

Response:

```json
{
  "room_id": "r_...",
  "epoch": 42,
  "publisher_public_key": "<base64url>",
  "signer_user_id": "u_...",
  "signature": "<base64url>"
}
```

#### 8.8.16 POST /rooms/:id/publisher-key

Auth: Required. Room member (human member only; bots are excluded).

Request:

```json
{
  "epoch": 42,
  "publisher_public_key": "<base64url>",
  "signature": "<base64url, Ed25519>"
}
```

`signature` is Ed25519 over the length-prefixed concatenation defined in §8.10:

```
u32_be(len(room_id)) || room_id
|| epoch_u64_be
|| publisher_public_key_raw_32_bytes
```

Signed with the caller's MLS identity private key. Verified against the caller's `identity_pubkey`.

Behavior:

1. Verify caller is a current `room_members` row. If not: 403 `not_a_member`.
2. Verify the signature. If invalid: 400 `signature_invalid`.
3. First-publish-wins per epoch. If a valid publication for `(room_id, epoch)` exists: 409 `epoch_already_published`.
4. Upsert the row into `room_publisher_keys`.
5. Publish `room.publisher_key_updated` on the room channel, on the signer's own user channel, and on each granted bot's channel.

Bots (write-only, observer, member) are excluded from publishing.

#### 8.8.17 PATCH /bots/:id/keys

Auth: bot owner session **or** bot token where `bot_token.bot_id == :id`.

Admin roles cannot rotate bot keys; they may disable the bot.

Rate limit: `RATE_BOT_KEY_ROTATE_PER_HOUR=5` per bot.

Request:

```json
{
  "bot_identity_pubkey": "<base64url, Ed25519>",
  "bot_command_pubkey": "<base64url, X25519>",
  "identity_pubkey": "<base64url, Ed25519, MLS>"
}
```

All three fields required. Partial rotation → 400 `partial_rotation_not_supported`.

Behavior:

1. Verify caller is the bot owner or the bot itself.
2. Append a new leaf to the KT log with the new keys.
3. Update `bot_accounts` with the new public keys.
4. Publish `bot.keys_rotated` on `private-bot-{bot_id}` with `{ bot_id, rotated_at }` (non-durable), and on `private-user-{owner_user_id}` with `{ bot_id, rotated_at, user_seq }` (durable).
5. Existing bot tokens remain valid. The runtime reloads keys and reconnects.

Response: 200 with the updated bot record.

#### 8.8.18 POST /bots/:id/tokens

Auth: Bot owner session or bot token matching `:id`.

Rate limit: `RATE_BOT_TOKEN_ISSUE_PER_HOUR=20`.

Response:

```json
{
  "token_id": "t_...",
  "token": "<43-char base64url>",
  "created_at": "..."
}
```

The token value is shown once. It is not recoverable.

#### 8.8.19 GET /bots/:id/tokens

Auth: Bot owner session or bot token matching `:id`.

Rate limit: `RATE_BOT_FETCH_PER_MIN`.

Response:

```json
{
  "tokens": [
    {
      "token_id": "t_...",
      "created_at": "...",
      "last_used_at": "..." | null,
      "revoked_at": "..." | null
    }
  ]
}
```

No token value is returned.

#### 8.8.20 DELETE /bots/:id/tokens/:token_id

Auth: Bot owner session or bot token matching `:id`. Marks `revoked_at`. Returns 204.

#### 8.8.21 POST /bots/me/commands/:id/ack

Auth: Bot token. Bot acknowledges a command. Marks `acked_at`. Once acked, TTL does not apply. Returns 204.

### 8.9 Event Catalog

**Room channel** — `private-room-{room_id}`. Best-effort delivery. Clients reconcile via REST.

| Event | Payload | When |
|---|---|---|
| `message.new` | `{ id, room_id, sender_type, sender_id, sender_client_id, epoch, seq, content_type, reply_to, bot_key_leaf_index?, created_at }` | After `POST /rooms/:id/messages` (public) or `POST /rooms/:id/bot-messages` |
| `message.edited` | `{ id, edit_of, edit_sequence, room_id, sender_type, sender_id, created_at }` | After `PATCH .../messages/:msg_id` |
| `message.deleted` | `{ id, room_id }` | After `DELETE .../messages/:msg_id` |
| `reaction.added` | `{ id, room_id, message_id, sender_user_id, reaction, created_at }` | After add |
| `reaction.removed` | `{ id, room_id, message_id }` | After remove |
| `room.updated` | `{ room_id, metadata?, retention_days?, max_file_size_bytes? }` | After `PATCH /rooms/:id` |
| `room.member_added` | `{ room_id, user_id, role, joined_at }` | After add or accept |
| `room.member_removed` | `{ room_id, user_id }` | After kick or leave |
| `epoch.updated` | `{ room_id, epoch, sequence }` | After MLS epoch advance |
| `mls.add_pending` | `{ room_id, target_user_id?, target_bot_id?, client_ids }` | After `POST /rooms/:id/members` or bot grant to member mode |
| `mls.remove_stale` | `{ room_id, target_user_id?, target_bot_id?, queued_at, stale_since }` | After `PENDING_MLS_REMOVE_TIMEOUT_DAYS` elapses with no commit. Fires once per stale row. |
| `mls.remove_confirmed` | `{ room_id, target_user_id?, target_bot_id?, confirmed_at }` | After a member generates the Remove commit |
| `call.started` | `{ call_id, room_id, initiator_id, started_at }` | On call start |
| `call.ended` | `{ call_id, room_id, ended_at, duration_seconds }` | On call end |
| `session.created` | `{ room_id, session_id, extension_id, session_type, created_by, created_at }` | After session create |
| `session.updated` | `{ room_id, session_id, metadata?, metadata_version?, position?, changed }` | After `PATCH .../sessions/:id` |
| `session.deleted` | `{ room_id, session_id, extension_id }` | After session delete or room deletion |
| `session.occupancy` | `{ room_id, session_id, participant_count }` | On join/leave/timeout, debounced ≤1/sec/session |
| `room.publisher_key_updated` | `{ room_id, epoch, publisher_public_key, signer_user_id, signature }` | After publisher key publication is accepted |
| `read.count` | `{ room_id, message_id, read_by_count }` | After read state advance, silent, non-durable |
| `bot.added` | `{ room_id, bot_id, mode, scopes, granted_by }` | After bot grant |
| `bot.revoked` | `{ room_id, bot_id }` | After bot revocation |
| `bot.updated` | `{ room_id, bot_id, scopes?, avatar_file_id?, changed }` | After bot scope, avatar, or declarations change |
| `bot.deleted` | `{ room_id, bot_id, deleted_at }` | After bot account deletion, one event per room with a grant |

**Notes (room channel):**

- `message.new` for bot messages includes `bot_key_leaf_index`. The field is present iff `sender_type = "bot"`. On user messages, the field is absent.
- `call.ended.duration_seconds` is an integer, in seconds, truncated toward zero. Always present. Non-null.
- `room.publisher_key_published` is removed. Merged into `room.publisher_key_updated`.
- Whisper messages (`target_user_ids` non-null) are delivered on each recipient's `private-user-{user_id}` channel and on the sender's own `private-user-{sender_user_id}` channel as a `message.new` event. The payload is identical to the room-channel `message.new`, with `target_user_ids` included. The sender's sending device filters its own event by comparing `sender_client_id` against its own `client_id`. Other devices of the sender apply the event for read-state consistency. No whisper is published on the room channel.
- Events for whisper messages — `message.new`, `message.edited`, `message.deleted`, `reaction.added`, `reaction.removed` — are delivered on the same user channels as the original `message.new`. The room channel receives no whisper event of any kind.
- The server strips `target_user_ids` from any payload published on the room channel. This is defensive.
- `bot.updated` `changed` array may include `"scopes"`, `"avatar"`, `"commands"`. When `"commands"` is present, `scopes` and `avatar_file_id` are omitted; the client invalidates its declarations cache.

**User channel** — `private-user-{user_id}`. Durable events caught up via `GET /users/me/sync?since_seq=N`.

| Event | Payload | Durable |
|---|---|---|
| `read.sync` | `{ room_id, last_read_message_id, user_seq }` | Yes |
| `room_order.sync` | `{ room_ids, user_seq }` | Yes |
| `device.added` | `{ device_id, platform, added_at, user_seq }` | Yes |
| `device.revoked` | `{ device_id, reason, user_seq }` | Yes |
| `device.name_updated` | `{ device_id, encrypted_device_name, user_seq }` | Yes |
| `session.revoked` | `{ session_id, reason }` | No |
| `user.updated` | `{ user_id, profile_version, user_seq }` | Yes |
| `call.signal` | `{ call_id, sender_user_id, sender_client_id, target_client_id, envelope }` | No |
| `session.signal` | `{ room_id, session_id, sender_user_id, sender_client_id, target_client_id?, envelope }` | No |
| `starred_item.added` | `{ item_id, item_type, room_id, user_seq }` | Yes |
| `starred_item.removed` | `{ item_id, item_type, user_seq }` | Yes |
| `preference.updated` | `{ key, user_seq }` | Yes |
| `bot_settings.updated` | `{ bot_id, key, user_seq }` | Yes |
| `bot.created` | `{ bot_id, display_name, created_at }` | Yes |
| `bot.deleted` | `{ bot_id, deleted_at }` | Yes |
| `bot.keys_rotated` | `{ bot_id, rotated_at, user_seq }` | Yes |
| `bot.command_result` | `{ command_id, bot_id, room_id, result_type, ciphertext, bot_result_pubkey }` | No |
| `bot.local_message` | `{ bot_id, result_type, ciphertext, bot_result_pubkey }` | No |
| `mls.remove_confirmed` | `{ room_id, target_user_id?, target_bot_id?, confirmed_at, user_seq }` | Yes |
| `mls.welcome_ready` | `{ room_id, welcome_id }` | No |
| `room.publisher_key_updated` | `{ room_id, epoch, publisher_public_key, signer_user_id, signature }` | No |
| `kt.snapshot` | `{ tree_size, root_hash, created_at }` | No |
| `bot.paused` | `{ bot_id, reason }` | No |
| `account.disabled` | `{ reason }` | Yes |
| `account.deleted` | `{}` | Yes |
| `room.transfer_initiated` | `{ room_id, from_user_id, transfer_id }` | Yes |
| `room.transfer_accepted` | `{ room_id, from_user_id, to_user_id, accepted_at }` | Yes |
| `room.transfer_cancelled` | `{ room_id, transfer_id, reason }` | Yes |

**Notes (user channel):**

- `room.transfer_initiated` is delivered on `private-user-{recipient_user_id}`.
- `room.transfer_accepted` is delivered on both `private-user-{from_user_id}` and `private-user-{to_user_id}`.
- `room.transfer_cancelled` is delivered on `private-user-{recipient_user_id}` and on `private-user-{owner_user_id}` for the owner's own multi-device sync.
- `bot.paused` is delivered only to the bot's `owner_user_id`. Not on the bot channel (the bot is paused and cannot process events).
- `bot.keys_rotated` on the user channel is for the owner. The bot-channel variant is non-durable; this one is durable.
- `bot.command_result` is delivered to the invoking user's channel. Live-only. If the client is offline, the result is lost. The client may refetch command state, or the bot may use `ctx.reply()` for durable output.
- `bot.local_message` is delivered to the owner's channel. Live-only. Owner-scoped diagnostics.
- `kt.snapshot` is non-durable. Clients fetch `GET /kt/snapshot` on foreground activation as a fallback, throttled to at most one fetch per 5 minutes.

**Bot channel** — `private-bot-{bot_id}`.

| Event | Payload | Durable |
|---|---|---|
| `bot.command_invoked` | `{ command_id, room_id, sender_user_id, sender_client_id, ciphertext, ephemeral_result_pubkey }` | Yes |
| `bot.settings_updated` | `{ bot_id, keys_changed, user_seq }` | Yes |
| `bot.grant_updated` | `{ bot_id, room_id, old_mode, new_mode, scopes, changed }` | No |
| `bot.updated` | `{ bot_id, avatar_file_id?, changed }` | No |
| `bot.keys_rotated` | `{ bot_id, rotated_at }` | No |
| `room.publisher_key_updated` | `{ room_id, epoch, publisher_public_key, signer_user_id, signature }` | No |

**Notes (bot channel):**

- `command_name` is inside the ciphertext, not a payload field.
- `ephemeral_result_pubkey` is always present on `bot.command_invoked`. It is the X25519 pubkey the bot encrypts its result to.
- `bot.paused` is **not** on this channel. Delivered on the owner's user channel.
- `room_id` is absent on `bot.updated` for the bot channel; the channel is already bot-scoped.

**Client events** (Sockudo, ephemeral):

| Event | Channel | Payload |
|---|---|---|
| `client-typing.start` | `private-room-{room_id}` | `{ user_id }` |
| `client-typing.stop` | `private-room-{room_id}` | `{ user_id }` |

`client-read` is removed. `client-mls-request` is removed.

### 8.10 Signing Encoding

All Ed25519 signatures in the system use the following encoding.

**Rule:** Variable-length fields are prefixed with their length as a 4-byte big-endian unsigned integer. Fixed-length fields are concatenated raw. Field ordering is specified per context. **All length prefixes and integer fields are big-endian. There are no little-endian fields in any signing context.**

| Field type | Encoding |
|---|---|
| Variable-length (strings, blobs) | `u32_be(length) \|\| bytes` |
| Fixed-length (u64, [u8;32], [u8;64]) | raw bytes, big-endian |
| Field ordering | Per context |

**Context: Bot message**

```
signed = u32_be(len(room_id)) || room_id_utf8
      || epoch_u64_be
      || u32_be(len(content_type)) || content_type_utf8
      || u32_be(len(ciphertext)) || ciphertext_bytes
```

Signed with `bot_identity_private_key`. Verified with the bot's currently active `bot_identity_pubkey`.

**Context: Publisher key publication**

```
signed = u32_be(len(room_id)) || room_id_utf8
      || epoch_u64_be
      || publisher_public_key_raw_32_bytes
```

Signed with the member's MLS identity private key. Verified with the member's `identity_pubkey`. `publisher_public_key` is raw X25519 bytes (32), not base64url text.

**Context: Session signaling (exception)**

```
signed = nonce (12 bytes)
      || ciphertext (variable, length implicit from envelope size minus 12 minus 16 minus 64)
      || tag (16 bytes)
```

No length prefixes. Nonce and tag are fixed-length and bracket the variable-length field, so the ciphertext length is unambiguous. This is the only context that uses this pattern.

### 8.11 Key Transparency Endpoints

| Method | Path | Purpose |
|---|---|---|
| GET | /kt/user/:id | Current user keys with inclusion proof |
| GET | /kt/bot/:id | Current bot keys with inclusion proof |
| GET | /kt/bot/:id/history | All bot key history |
| GET | /kt/snapshot | Current KT snapshot |

#### 8.11.1 GET /kt/user/:id

Auth: Required. Any authenticated user or bot may fetch any user's KT entry.

Response:

```json
{
  "user_id": "u_...",
  "leaf_index": 1234,
  "identity_pubkey": "<base64url, Ed25519>",
  "inclusion_proof": [ ... ],
  "tree_head": { ... },
  "auditor_signatures": [ ... ]
}
```

**Omitted:** `username_token`. It is stored in the KT log but is not needed for signature verification, and exposing it would create an additional enumeration surface.

#### 8.11.2 GET /kt/bot/:id

Auth: Required.

Response:

```json
{
  "bot_id": "b_...",
  "leaf_index": 12345,
  "bot_identity_pubkey": "...",
  "bot_command_pubkey": "...",
  "identity_pubkey": "...",
  "inclusion_proof": [ ... ],
  "tree_head": { ... },
  "auditor_signatures": [ ... ]
}
```

#### 8.11.3 GET /kt/bot/:id/history

Auth: Required.

Response:

```json
{
  "bot_id": "b_...",
  "entries": [
    { "leaf_index": 100, "bot_identity_pubkey": "...", "bot_command_pubkey": "...", "identity_pubkey": "...", "added_at": "..." },
    { "leaf_index": 12345, "bot_identity_pubkey": "...", "bot_command_pubkey": "...", "identity_pubkey": "...", "added_at": "..." }
  ]
}
```

#### 8.11.4 GET /kt/snapshot

Auth: Required.

Response:

```json
{
  "tree_size": 12345,
  "root_hash": "<base64url>",
  "created_at": "..."
}
```

Returned on demand. Used as the fallback for `kt.snapshot` events, which are non-durable.

---

## 9. CLI Subcommands

| Command | Purpose |
|---|---|
| `server oprf rotate --confirm` | Resample the username OPRF key |
| `server kt snapshot` | Trigger a signed KT snapshot |
| `server kt verify --from <index>` | Verify KT log integrity |
| `server models verify` | Verify model files against manifest hashes |
| `server models fetch --from <url>` | Pre-seed models from a shared origin |
| `server session-types validate` | Validate `SESSION_TYPES.toml` |
| `server extension-proxy validate-blocklist` | Validate the domain blocklist |
| `server bots validate` | Validate bot runtime connectivity |
| `server config show` | Print the `server_config` table (secrets masked) |

---

## 10. Client Contract Matrix

| Feature | Server behaviour |
|---|---|
| Text messaging | Relays MLS ciphertext; never inspects content |
| Message editing | Stores edit chain; enforces `edit_window_seconds` |
| Message deletion | Tombstones; publishes `message.deleted` |
| Reactions | Aggregated; silent; retained until parent pruning |
| Threading | `reply_to` field |
| Whisper messages | `target_user_ids`; delivered on user channels only; sender's own user channel included for multi-device sync |
| Starred items | User-scoped sync; durable events |
| Generic preferences | Client-defined keys; all values encrypted |
| Bot settings | Two ciphertexts for non-secrets (client + bot via X25519 ECDH); one for secrets (bot only) |
| Attachments — upload | Content-addressed, C2SP chunked encryption |
| Attachments — download | Range headers supported |
| Attachments — presign | S3 only; filesystem returns 501 |
| Identity | Lookup token; no plaintext usernames |
| Display names | AES-GCM ciphertext; key derived from client-only token (32-byte HKDF) |
| Device names | AES-GCM ciphertext; user-scoped sync; 32-byte HKDF |
| Recovery | Server-generated codes; Argon2id; OPAQUE re-registration |
| Multi-device sync | User-scoped state + `user_seq` cursor |
| User events | Durable; live push + REST catch-up |
| Room events | Best-effort; REST reconciliation via `GET /rooms/:id/messages` |
| Calls | Signaling on user channels, gated on participation, ephemeral ECDH; TURN via dedicated endpoint; history via `GET /rooms/:id/calls` |
| Sessions | Persistent metadata; in-memory occupancy; count-only room events; roster gated on participation; envelope-based signaling |
| Bots | Headless Node.js clients; capability-driven mode; publisher key from MLS epoch; command key in KT; signed publishes; server relays opaque |
| Bot avatar | Content-addressed; `POST /bots/:id/avatar` |
| Bot key rotation | `PATCH /bots/:id/keys`; KT append; `bot.keys_rotated` on bot and owner channels |
| Bot connection status | `connected` field on `GET /rooms/:id/bots`; owner-only; ephemeral |
| Bot declarations | `declarations` field on `GET /bots/:id`; uploaded via `POST /bots` or `PATCH /bots/:id` |
| Bot command results | Ephemeral X25519 per command; `bot.command_result` on invoker's user channel |
| Bot owner local messages | `bot.local_message` via `POST /bots/me/messages` |
| Model hosting | Three modes: local, external, proxy |
| Link preview proxy | Opt-in. OG metadata only. No auth. |
| Extension proxy | Opt-in. Requires auth. SSRF-guarded. Blind to URLs, bodies, credentials. |
| Push notifications | `sender_ref` replaces `sender_user_id` |
| Client events | `client-typing.*` only |
| Delta sync | `(epoch, seq)` cursor; REST fetch is authoritative |
| User sync | `user_seq` cursor via `GET /users/me/sync` |
| Encrypted payload opacity | Server does not decrypt, parse, validate, log, or inspect any client-encrypted payload |
| Publisher keys | Persisted in `room_publisher_keys`; signed publishes; client-side verification |
| Read counts | Aggregate per message; attached to message fetch; live event non-durable |
| Call history | `GET /rooms/:id/calls`; all room members |
| Sockudo connection | `sockudo_url`, `sockudo_app_key`, `sockudo_auth_endpoint` in capabilities; server issues subscription auth |

---

## 11. Phased Build Plan

| Phase | Deliverable | Depends On |
|---|---|---|
| 1 | OPRF identity with token split | — |
| 2 | Recovery | 1 |
| 3 | User-scoped sync foundation | 1 |
| 4 | User-scoped sync writes | 3 |
| 5 | Device model | 3, 4 |
| 6 | Room metadata with encrypted blob | 3 |
| 7 | Message editing | — |
| 8 | Reactions | — |
| 9 | Threading | — |
| 10 | Whisper messages | 7 |
| 11 | Member pagination + bot merge | — |
| 12 | Retention preview | — |
| 13 | Avatar upload | — |
| 14 | `pending_mls_adds` coordination (user and bot targets) | — |
| 15 | `pending_mls_removes` coordination + timeout | 14 |
| 16 | Key transparency with bot keys | 1 |
| 17 | Link preview proxy | — |
| 18 | Call signaling (user channel, ephemeral ECDH) | — |
| 19 | Call join/leave endpoints | 18 |
| 20 | TURN credentials | 18 |
| 21 | Sessions (metadata, occupancy, envelopes) | 18, 19 |
| 22 | Model hosting (STT + TTS), three modes | — |
| 23 | Starred items | 3, 4 |
| 24 | Generic preferences (encrypted) | 3, 4 |
| 25 | `room_order` moved to dedicated table | 4 |
| 26 | Extension proxy | 17 |
| 27 | Bot account model (capabilities, declared scopes, avatar) | 18, 19 |
| 28 | Bot publisher key relay (signed, persisted) | 27 |
| 29 | Bot command key + KT extension | 16, 27 |
| 30 | Bot settings encryption (X25519 ECDH) | 24, 27 |
| 31 | Observer stream | 27 |
| 32 | Commands and declarative settings contract | 27 |
| 33 | Documentation corrections (enumeration, occupancy, social graph, IP exposure) | 1–32 |
| 34 | Capabilities update | 1–33 |
| 35 | Message fetch endpoint (`GET /rooms/:id/messages`) | 3 |
| 36 | Welcome fetch endpoints (`GET /welcomes`) | 1 |
| 37 | Call history endpoint (`GET /rooms/:id/calls`) | 18 |
| 38 | Bot key rotation (`PATCH /bots/:id/keys`) | 27, 29 |
| 39 | Read count reconciliation | 3, 7 |
| 40 | Multi-device bot events (`bot.created`, `bot.deleted`) | 27 |
| 41 | Sockudo capabilities fields + `POST /sockudo/auth` | — |
| 42 | `server_config` table + auto-generation | 41 |
| 43 | Bot declarations (`declarations` field, `PATCH /bots/:id`) | 27, 32 |
| 44 | Bot command results (`POST /bots/me/messages` invoker target, `bot.command_result`) | 27, 29 |
| 45 | Bot owner local messages (`POST /bots/me/messages` owner target, `bot.local_message`) | 44 |
| 46 | Idempotency (`bot_request_log`, `request_id`) | 27 |
| 47 | User KT endpoint (`GET /kt/user/:id`) | 16 |

Critical path: 1 → 3 → 4 → 5. Phase 33 closes the adversarial review.

---

## 12. Security Boundaries

| Boundary | Guarantee |
|---|---|
| Server sees plaintext messages | Never |
| Server sees decryption keys | Never |
| Server sees plaintext username | Never |
| Server sees plaintext display name | Never |
| Server sees plaintext device name | Never |
| Server sees attachment contents | Never |
| Server sees attachment plaintext size | Yes (metadata) |
| Server can decrypt past messages | No |
| Server can decrypt future messages | No |
| Server can enumerate low-entropy usernames | Yes, offline, undetectable |
| Server can derive display name key from database | No |
| Server can derive device name key from database | No |
| Server can derive preferences key from database | No |
| Server sees the social graph | Yes. Room membership, sender, timing. |
| Server sees message timing | Yes |
| Server sees message size | Yes |
| Server sees sender identity | Yes |
| Sealed sender | No |
| Server can swap a bot's `bot_command_pubkey` undetected | No. KT-verified. |
| Server can swap a bot's `bot_identity_pubkey` undetected | No. KT-verified. |
| Server can swap a bot's publisher key undetected | No. Signed by member; client verifies against own MLS derivation. |
| Server validates publisher key correctness | No. It relays and enforces first-publish-wins. Client comparison is the security boundary. |
| Removed member can derive future publisher keys | No. MLS epoch secret rotates. |
| Bot message forward secrecy within epoch | Not provided. Epoch-level only. If the epoch secret is compromised, all bot messages within that epoch are decryptable. Weaker than MLS message encryption. |
| Write-only bot holds room keys | Never |
| Write-only bot sees message content | Never |
| Observer bot sees ciphertext | Never |
| Observer bot sees room social graph | Yes. Disclosed at install. |
| Bot messages are signed | Yes. Server verifies signature over opaque bytes; does not decrypt. |
| Bot key rotation | Appends to KT log. Previous keys retained. Historical messages remain verifiable via `bot_key_leaf_index`. Messages signed with rotated-out keys are rejected. |
| Bot connected status | Ephemeral, in-memory, never logged, shown only to bot owner. |
| Session occupancy is persisted | Never. In-memory only. |
| Session occupancy is disclosed to room members | Yes, as a count. |
| Session identity is disclosed to non-participants | No |
| Session signal payloads are opaque to the server | Yes |
| Session signal forgery is possible by another participant | No |
| Session signaling confidential from other participants | No. Shared key. |
| Call participation is persisted | Never. In-memory only. |
| Call signaling is gated on participation | Yes |
| Call signaling is encrypted | Yes. Per-call ephemeral ECDH. |
| Call signaling has forward secrecy | Yes |
| Call participant IPs are exposed to each other | Yes. Inherent to WebRTC. |
| Call record (initiator, start, end, duration) is persisted | Yes. Retention-controlled. |
| Call history is visible to room members | Yes. Documented disclosure. |
| Server sees plaintext room metadata | Never |
| Server sees plaintext session metadata | Never |
| Server sees preference values | Never |
| Server sees secret values | Never |
| Server sees local message content | Never |
| Server sees extension/bot encrypted payloads | Never. Opaque. |
| Server logs session occupancy | Never |
| Server logs call participation | Never |
| Server logs bot connection status | Never |
| Server logs publisher keys | Never |
| Server logs observer stream contents | Never |
| Server logs extension proxy URLs, bodies, responses | Never |
| Server logs call signaling payloads | Never |
| Server logs session signal payloads | Never |
| Push notification reveals room_id and timing | Yes. Structurally necessary. |
| Key transparency is independently verified | Only when auditors are configured. Default: not independently verified. |
| KT API exposes `username_token` | Never. Stored in the KT log but excluded from all API responses. |
| Extension proxy rejects Authorization and Cookie by header name | Yes. Custom-header credential forwarding is possible; build-time install is the control. |
| Extension proxy forwards user IP addresses | No |
| Extension proxy streams responses | No |
| Extension proxy caches responses | No |
| Model files are immutable and content-addressed | Yes |
| SPA bundle is version-coupled | Yes |
| Multi-node deployments supported | No |
| Publisher keys persisted server-side | Yes. Public key, signature, signer are public; nothing sensitive. |
| Room channel carries `target_user_ids` | Never. Stripped defensively. |
| Whisper event routing | User channels only. Room channel receives nothing for whispers. |
| `read.count` durable | No. Attached to message fetch as `read_by_count`. |
| Sockudo app key public | Yes. By Pusher design. Identifies the app, not the client. |
| Sockudo app secret exposed | Never. Stored in `server_config`, `is_secret = 1`. |
| Sockudo auth endpoint is an HMAC oracle | No. Channel name pattern is validated; unknown prefixes rejected with 403. |
| Sockudo auth accepts disabled accounts | No. `users.disabled_at` / `bot_accounts.disabled_at` set → 403. |
| Sockudo subscription closed on removal | No. The next subscribe attempt fails. Design decision, documented. |

**Normative refusal clause:**

Ambient presence is refused. The server does not track or expose who is online, when they were last seen, or what they are doing across the system.

Call-scoped co-presence is permitted. Within a call or session, the server knows in memory which room members are participating, in order to route media. This knowledge is disclosed only to participants of the same call or session. It is held in memory only and is never written to any persistent store or external system — not to the database, not to log files, not to stdout, not to stderr, not to any metrics pipeline, not to any tracing system, not to any monitoring system, not to any audit log. No server component may export occupancy state. This is a normative constraint on all server code.

Bot connection status is permitted. It is a server-to-bot liveness indicator, scoped to the bot's own channel, visible only to the bot's owner. It is held in memory only. It is never written to any persistent store or external system. It is never logged. It is never synced.

The extension proxy is blind. The server decrypts URLs, request bodies, and response bodies in memory only, uses them to fulfill the request, and discards them. No URL, no header value, no body byte is written to any persistent store, log file, stdout, stderr, metrics pipeline, tracing system, or monitoring system. Only aggregated request counts per user per extension per hour are retained for audit.

Encrypted extension and bot payloads are opaque to the server. The server does not decrypt, parse, validate, log, or inspect them. The server defines only the wire shape. The encryption scheme is a client-side implementation detail.

Publisher keys are public. The server stores and relays them. The server does not verify that a publisher key matches any client's MLS state; it enforces first-publish-wins per epoch. Clients verify each other's publications against their own MLS derivation. This is the security boundary.

Room channel events never carry `target_user_ids`. This is enforced defensively. A routing bug must not be able to leak a whisper's recipient list to the room.

The Sockudo auth endpoint is not a generic HMAC oracle. It validates the channel name pattern and the caller's right to the channel. Unknown prefixes are rejected. The signing key never leaves the server.

---

## 13. Backups and Disaster Recovery

Backups include: the OPRF key file, the ALTCHA HMAC secret, the VAPID keys, the KT log directory, the session types config, the extension proxy domain blocklist, the `server_config` table (including the Sockudo app secret, treated as sensitive), and all other database contents. Model files included only if `BACKUP_INCLUDE_MODELS=true`. Session occupancy, call participation, bot connection status, observer cursors, and local messages are not part of any backup because they are not stored. Publisher keys persisted in `room_publisher_keys` are part of the database and are backed up with it.

Restore order: OPRF key file → database → KT log directory → session types config → blocklist. Session occupancy and call participation begin empty on restart. Bot connection status begins empty on restart. Publisher keys are restored from the database; members republish only on epoch transition.

---

## 14. GDPR Compliance

### 14.1 Data Subject Rights

Access, correction, deletion, portability, restriction. Exercised via `GET /users/me/export` and `DELETE /users/me`.

### 14.2 Account Deletion

Anonymises the user row, deletes devices, HTTP sessions, KeyPackages, push subscriptions, room memberships, recovery codes, and all user-scoped sync state (read state, preferences, room order, device names, starred items, bot settings). Queues MLS removes. Clears extension proxy rate-limit buckets. Publishes `account.deleted` on the user's own channel.

Identity columns on deletion: `username_token` set to a random 86-character base64url string; `encrypted_display` NULL; `profile` NULL.

Room session records cascaded if `created_by` is the deleted user. Key transparency entries retained; `user_id` replaced with a random placeholder. Bot ownership: on account deletion, bots are deleted. Their messages render as `(deleted bot)`. `bot.deleted` is published on each room channel with a grant and on the owner's own user channel.

### 14.3 Data Export

Structural metadata exported. Ciphertext exported as opaque. Recovery code metadata exported (hashes not). Starred items exported. Preferences exported as ciphertext. Bot settings exported as ciphertext. Bot declarations exported. Bot command history metadata exported (no ciphertext). Session participation not exported. Created sessions exported as metadata (no encrypted session metadata). Call history exported as metadata (initiator, start, end, duration).

### 14.4 Retention

`sync_event_retention_days` applies to sync state. `key_transparency_retention_days` applies to KT entries. `CALL_RECORD_RETENTION_DAYS` applies to call records. Session occupancy and call participation are not retained at all. Bot commands expire after `BOT_COMMAND_TTL_HOURS` unless acked. `bot_request_log` is retained 24 hours.

### 14.5 Tombstone Semantics

Deleted rows are marked, not physically removed, until the retention window expires. Tombstones carry `user_seq` and are propagated via sync.

### 14.6 Metadata Minimisation

| Field | Leak | Mitigation |
|---|---|---|
| `users.username_token` | Opaque lookup token | Requires OPRF key to invert. Operator enumeration documented. |
| `users.encrypted_display` | Opaque ciphertext | Key derived from client-only token. |
| `devices.encrypted_device_name` | Opaque ciphertext | Key derived from client-only token. |
| `user_preferences.value_encrypted` | Opaque ciphertext | Key derived from client-only token. |
| `bot_settings.value_encrypted_*` | Opaque ciphertext | Client-encrypted; bot ciphertext uses X25519 ECDH. |
| `bot_accounts.declarations` | Opaque JSON | Command names and setting keys are visible to the server. Not encrypted. Disclosed. |
| `sessions.last_seen_at` | Timestamp (HTTP sessions) | Coarsened to hour in admin views. |
| `messages.created_at` | Timestamp | Coarsened to minute. |
| `room_messages.edited_at` | Edit timing | Coarsened to day in admin views. |
| `room_messages.deleted_at` | Deletion timing | Coarsened to day in admin views. |
| `reactions.sender_user_id` | Attribution | Retained until parent pruning. Documented. |
| `push_subscriptions.push_token` | Unavoidable personal data | Auto-revoked after 90 days. |
| Push `sender_ref` | Truncated lookup token | One-way derived. |
| `audit_log.metadata` | JSON | Never includes content, IPs, URLs. |
| `kt_log.username_token` | Public by design | Stored in KT log; excluded from all KT API responses. |
| `session.occupancy` | N/A | Not persisted; count only. |
| Session roster | N/A | In-memory; gated on participation. |
| Session signal payloads | N/A | Opaque; encrypted and signed by client. |
| Call signal payloads | N/A | Opaque; encrypted by client. |
| `delivered_to` | N/A | Returned to caller; not logged. |
| Extension proxy URLs | N/A | Decrypted in memory only; never logged. |
| Extension proxy request/response bodies | N/A | Decrypted in memory only; never logged. |
| Extension proxy credentials | N/A | `Authorization` and `Cookie` rejected by header name. |
| Extension proxy domain counts | N/A | Only aggregated hourly counts. |
| Model files | N/A | Immutable; not user data. |
| Bot command payloads | N/A | Opaque; server relays. |
| Bot command results | N/A | Opaque; server relays. |
| Bot messages | N/A | Opaque ciphertext; `sender_bot_id` is public. |
| Bot display names | Plaintext | Bots are not people. Deliberate. |
| Bot avatar | Content-addressed | Public by design. |
| Bot declared scopes | Plaintext | Public. Used for grant validation. |
| Publisher keys | Public | Persisted. Public key, signature, signer are public. |
| Bot connection status | N/A | Ephemeral, in-memory, owner-only. |
| Call history | Metadata | Initiator, start, end, duration. Visible to room members. |
| `read_by_count` | Aggregate | Per message. No user identity. |
| `sockudo_app_key` | Public | By Pusher design. |
| `sockudo_app_secret` | Secret | `is_secret = 1` in `server_config`. Included in backups as sensitive. |

### 14.7 Controller Obligations

The controller's privacy policy must document: the username enumeration limitation, the session occupancy count disclosure, the call IP exposure, the call history visibility, the bot connection status disclosure to owners, the extension proxy existence, and what the server does and does not log. It must also state which metadata categories the server sees and which it does not.

### 14.8 Audit Actions

- `oprf.rotate`
- `recover.start`
- `recover.finish`
- `device.name_updated`
- `edit.create`
- `reaction.create`
- `reaction.delete`
- `kt.snapshot`
- `call.start`
- `call.end`
- `session.create`
- `session.delete`
- `model.manifest_reload`
- `session_types.reload`
- `extension.proxy_request` (aggregated hourly count only)
- `extension_proxy.reload_blocklist`
- `bot.create`
- `bot.grant`
- `bot.revoke`
- `bot.token_revoke`
- `bot.pause`
- `bot.resume`
- `bot.message`
- `bot.command_ack`
- `bot.key_rotate`
- `bot.declaration_update`
- `sockudo.config_generate`

No audit entry is written for session join, leave, heartbeat, signal, occupancy change, call join, call leave, call signal, publisher key publish, extension proxy request, or Sockudo auth. That would create exactly the log this design refuses.

---

## 15. Version History

| Version | Date | Notes |
|---|---|---|
| 1.0 | — | Historical baseline. |
| 2.0 | — | Introduced OPRF identity, user-scoped sync, editing, reactions, threading, key transparency, link preview proxy, extension proxy, calls, sessions, model hosting, starred items, generic preferences, batched test model. |
| 3.0 | — | Clean break. Corrected all adversarial review findings. Token split. Encrypted preferences. Call signaling on user channels with ephemeral ECDH. Pending MLS remove timeout. Removal of `call_participants`. Session signal envelope. Whisper messages. Full bot infrastructure with capability-driven mode, signed publisher key publishes, command encryption, settings encryption, avatar endpoints. Encrypted payload opacity principle. Corrected product summary and security boundary table. |
| 3.0.1 | 2026-10-04 | Coordination patch. Corrected HKDF lengths for display/device names (32, not 44). Bot settings use X25519 ECDH. Added `bot_key_leaf_index`. Added `room_publisher_keys` table. Added `GET /rooms/:id/messages`, `GET /rooms/:id/calls`, `PATCH /bots/:id/keys`, `GET /kt/bot/:id`, `GET /kt/bot/:id/history`. Added `CALL_RECORD_RETENTION_DAYS`. Added `connected` field. Added bot message signature. Added signing encoding §8.10. Merged publisher key events. Full event catalog reconciliation. Whisper delivery includes sender's own user channel. |
| 3.0.2 | 2026-10-04 | Final coordination patch. `connected` field restricted to owner-only. `read_by_count` attached to message fetch. Added `GET /welcomes` and `GET /welcomes/:id`. Added `room.transfer_accepted`, `bot.deleted`, `bot.created` events. `kt.snapshot` marked non-durable; added `GET /kt/snapshot`. `mls.remove_stale` fires once per row. `target_user_ids` stripped from all room channel payloads. `room.transfer_cancelled` channels corrected. Bot message signing and publisher key signing use length-prefixed encoding with big-endian integers. Event catalog frozen. |
| 3.0.3 | 2026-10-06 | Sockudo connection and subscription authentication. Added `SOCKUDO_PUBLIC_URL`, `SOCKUDO_AUTH_PATH`; `SOCKUDO_APP_KEY` and `SOCKUDO_APP_SECRET` now auto-generate and persist. Added `server_config` table. Added `sockudo_url`, `sockudo_app_key`, `sockudo_auth_endpoint` to capabilities. Added `POST /sockudo/auth` (Pusher v7 HMAC, channel pattern validation, disabled-account rejection, rate limit 300/min). Added `GET /kt/user/:id`; `username_token` excluded from all KT API responses. Bot coordination: added `POST /bots/me/messages` (invoker + owner targets), `GET /bots/:id/tokens`; removed `POST /bots/me/commands/:id/result`; `POST /rooms/:id/bot-commands` gains `ephemeral_result_pubkey` and `request_id`; `POST /rooms/:id/bot-messages` gains `request_id`; `PATCH /bots/:id/keys` accepts bot token or owner session; `GET /bots/:id` returns `declarations`; `bot.command_invoked` gains `ephemeral_result_pubkey`; `bot.keys_rotated` added on owner user channel (durable); added `bot.command_result` and `bot.local_message`; added `bot_request_log` table; `bot_commands` gains `ephemeral_result_pubkey`, `result_at`, `result_type`; `bot_accounts` gains `declarations`; added §6.33 Publisher Key Encryption; added `RATE_BOT_COMMAND_RESULT_PER_MIN`, `RATE_BOT_LOCAL_MESSAGE_PER_MIN`, `RATE_SOCKUDO_AUTH_PER_MIN`. Bot message forward secrecy within epoch documented as not provided. |

No amendments are used. v3.0 is a clean document. Future changes are new versions.

---

## 16. Document Status

This is the contract for the server side of the system. Every v3.0 implementation task references this document. If a task conflicts with this spec, the task is wrong and must be revised. If a feature is missing, it does not exist yet — it must be added here first, then built.

The event catalog in §8.9 is frozen. The signing encoding in §8.10 is frozen. The publisher key encryption scheme in §6.33 is frozen. Any change requires a new version.

**End of Server Specification v3.0.3.**
