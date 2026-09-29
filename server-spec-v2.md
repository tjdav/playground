# Server Specification v2.0

> **Status:** Stable — source of truth for all V2 implementation tasks.
> **Supersedes:** Server Specification v1.0 (amended through §16.12).
> **Scope:** Server-side contract only. Client implementation is out of scope and is covered by Client Specification v1.0.
> **Stack:** Sockudo + Axum + SQLite + OPAQUE (`opaque-ke` 4.0.1) + VOPRF (`voprf`) + ALTCHA + S3 (or filesystem)
> **Deployment:** Single VPS, Docker-based, self-hosted. Two containers: `server` (Axum + static SPA hosting) and `sockudo`. TLS terminates at a reverse proxy.

This document is a **breaking revision**. Nothing has been deployed; no migration path is required. Where V1 sections are unchanged, they are carried forward verbatim and marked. Where V2 changes them, the change is stated normatively.

Amendments to this document are tracked at §16. The V1 amendment log (§16.1–§16.12 of V1) is preserved in §17 for historical reference.

---

## 1. Product Summary

A self-hosted, end-to-end encrypted group messaging system with MLS-grade forward secrecy and post-compromise security. The server is an untrusted delivery service that never sees plaintext, keys, or meaningful metadata.

| Layer | Technology |
|---|---|
| Delivery | Sockudo (WebSocket, Pusher v7) |
| API | Axum (Rust) |
| Database | SQLite (embedded in the Axum process) |
| Blob storage | S3 (default) or filesystem (fallback) |
| S3 client | `rust-s3` |
| MLS engine | Wire CoreCrypto 10.5.2 (client-side only) |
| Auth | OPAQUE (aPAKE) via `opaque-ke` 4.0.1 |
| Username lookup | VOPRF (Ristretto255-SHA512) via `voprf` |
| Bot protection | ALTCHA Proof-of-Work v2 via `altcha` 0.2.0 |
| Attachment encryption | C2SP chunked encryption (`c2sp.org/chunked-encryption`) |
| Static SPA hosting | Axum `ServeDir` with SPA fallback |
| TLS termination | External reverse proxy (Traefik via Coolify, or Caddy) |

**V2 architectural change:** the server no longer stores plaintext usernames, display names, or device names. Identity is anchored in an OPRF token. User-scoped state (read state, room order, device names) is durable and sequence-synced across a user's devices.

---

## 2. Scope

### 2.1 In Scope — V2

Everything in V1 §2.1, plus:

- **OPRF-based identity.** Usernames, display names, and device names are opaque to the server.
- **Recovery flow.** Server-generated recovery codes; OPAQUE re-registration.
- **User-scoped channels.** `private-user-{user_id}` with durable, sequence-synced state.
- **Multi-device sync.** Read state, room order, device names propagate across devices via `user_seq`.
- **Message editing.** 15-minute window, signed edit chain, encrypted history.
- **Reactions.** Table, endpoints, aggregation, silent delivery.
- **Threading.** `reply_to` on messages.
- **Room metadata updates.** `PATCH /rooms/:id` with encrypted metadata, atomic with `room.updated`.
- **Room avatars.** `POST /users/me/avatar`; avatar as encrypted attachment.
- **Member pagination.** Cursor-based pagination on `GET /rooms/:id/members`.
- **Retention change preview.** `POST /rooms/:id/retention/preview`.
- **MLS adds coordination.** `pending_mls_adds` table mirroring `pending_mls_removes`.
- **Key transparency.** Append-only log with inclusion proofs and auditor signatures.
- **Link preview proxy.** Opt-in, SSRF-guarded, blind to URLs.
- **Call signaling.** WebRTC signaling events on the room channel; TURN credential endpoint.
- **Admin surfaces** for new V2 features.

### 2.2 Out of Scope — V2

- **Presence.** No online/offline indicators. No presence channels. `sessions.last_seen_at` is not user-facing. The client spec’s “Last seen” setting (§17.2) is removed.
- Federation between servers
- Multi-tenancy
- Server-side key escrow
- Plaintext metadata on server
- Email or OAuth registration
- Anonymous accounts
- Plaintext message export
- Username changes
- Third-party CAPTCHA services
- Client-side data deletion
- Forcing peers to delete local copies
- Consent management UI
- TLS termination inside the Axum process
- ACME client inside the server
- MP4 fast-start enforcement
- Multi-range HTTP requests
- Range-restricted presigned URLs
- **Message forwarding** (client re-encrypts and re-uploads)
- **Message pinning**
- **GIF search**

### 2.3 Breaking Changes from V1

| Change | V1 | V2 |
|---|---|---|
| `users.username` | Plaintext | Removed. Replaced by `users.username_token`. |
| `users.username_hash` | Plaintext hash | Removed. |
| `users.display_name` | Plaintext | Removed. Replaced by `users.encrypted_display`. |
| `devices.name` | Plaintext | Removed. Device names are user-scoped sync state. |
| `rooms.name_encrypted` | `TEXT` column | Replaced by `rooms.metadata` (opaque encrypted JSON blob). |
| Room metadata | `name_encrypted`, `retention_days`, `max_file_size_bytes` as separate columns | Encrypted JSON blob in `rooms.metadata`, plus unencrypted server-visible fields (see §7.3). |
| `room_messages.reply_to` | Absent | Added. |
| `room_messages.edited_at` | Absent | Added. |
| `room_messages.edit_chain` | Absent | Added. |
| Reactions | Absent | New table. |
| Key transparency | Absent | New tables + endpoints. |
| Call signaling | Absent | New events + endpoint. |
| `client-read` client event | Present | Removed. Replaced by `POST /users/me/read-state`. |
| `GET /users/me/sync` | Absent | New. |
| `POST /oprf/blind` | Absent | New. |
| `POST /auth/recover/start`, `/finish` | Absent | New. |
| OPRF key rotation | N/A | Immutable for account lifetime; compromise-only rotation. |

---

## 3. Roles and Permissions

Unchanged from V1 §3, with the following additions.

### 3.1 Room Roles

| Role | Permissions |
|---|---|
| `owner` | Kick, delete room, promote/demote (Discord mode), transfer ownership, delete any message, edit metadata, change retention, set disappearing timer |
| `moderator` | Kick (Discord mode), delete any message (Discord mode) |
| `member` | Send messages, upload attachments, leave room, delete own messages, add reactions, edit own messages (15-min window), create invites |

### 3.2 Enforcement Order

Unchanged from V1 §3.4.

### 3.3 Room Ownership Transfer

Unchanged from V1 §3.5.

---

## 4. Resource Limits

### 4.1 Three-Tier Model

Unchanged from V1 §4.1.

### 4.2 Default Limits

| Key | Server hard max | Instance default | Instance range |
|---|---|---|---|
| `file_size_bytes` | 104857600 | 104857600 | 1 MB – 100 MB |
| `room_size` | 1000 | 100 | 2 – 1000 |
| `rooms_per_user` | 500 | 50 | 1 – 500 |
| `devices_per_user` | 20 | 10 | 1 – 20 |
| `keypackages_per_device` | 50 | 20 | 5 – 50 |
| `message_size_bytes` | 65536 | 16384 | 256 B – 64 KB |
| `attachment_retention_days` | 365 | 0 (forever) | 0 – 365 |
| `call_max_participants` | 50 | 8 | 2 – 50 |
| `reactions_per_message` | 50 | 50 | 1 – 50 |
| `edit_window_seconds` | 86400 | 900 | 60 – 86400 |
| `sync_event_retention_days` | 365 | 90 | 30 – 365 |
| `key_transparency_retention_days` | 3650 | 3650 | 365 – 3650 |

**`edit_window_seconds`:** client spec §11.8 specifies 15 minutes. The server enforces this as the default and maximum unless the instance overrides downward. The server never allows an edit beyond the instance’s `edit_window_seconds`.

**`sync_event_retention_days`:** how long per-user state rows are retained after their last `user_seq` bump. Rows older than this window may be pruned. A device whose cursor is older than the retention window receives `full_resync_required` (see §6.7).

### 4.3 Per-Entity Overrides

Unchanged from V1 §4.3.

### 4.4 Effective Limits Exposure

Unchanged from V1 §4.4. `GET /rooms/:id` continues to return `effective_max_file_size_bytes` and `effective_message_retention_days`.

### 4.5 Cleanup Jobs

All jobs run on the shared hourly scheduler. V1 jobs are retained. New V2 jobs:

| Job | Retention | Notes |
|---|---|---|
| Sync state pruning | `sync_event_retention_days` | Deletes user-scoped state rows whose `updated_at` is older than the retention window and whose `user_seq` is below the current max. Devices are not deleted; only stale rows. |
| Push subscription expiry | 90 days of inactivity | Revokes and deletes `push_subscriptions` with `last_used_at < now - 90d`. |
| Key transparency retention | `key_transparency_retention_days` | Retains log entries for the configured window. Proofs older than the window are returned as “no longer available” with a snapshot fallback. |
| Recovery code consumption | Immediate | Consumed codes are marked, not deleted, for audit. |
| Call state cleanup | 24 hours after call end | Deletes `call_sessions` rows and any dangling participants. |

---

## 5. Environment Variables

Follows Coolify conventions. All variables are optional unless marked **required**.

### 5.1 Application

```env
APP_ENV=production
APP_URL=https://chat.example.com
APP_NAME=Encrypted Chat
LOG_LEVEL=info
CLIENT_STATIC_DIR=/app/client
```

**Log policy.** The server writes structured logs to `stdout` and `stderr` only. It does not write log files. Container runtime captures and rotates stdout. The server does not store request logs persistently. No IP addresses, URLs, or user identifiers are written by the server to any persistent store. Retention of stdout is the operator’s responsibility and must be documented in the operator’s privacy policy.

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

### 5.4 Sessions

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
```

| Variable | Default | Notes |
|---|---|---|
| `RATE_OPRF_BLIND_PER_MIN` | `30` | Per IP. Rate limit on the public `/oprf/blind` endpoint. |
| `RATE_OPRF_BLIND_PER_HOUR` | `300` | Per IP. Second window to bound burst abuse. |
| `RATE_RECOVER_START_PER_MIN` | `5` | Per IP. Recovery attempts are expensive and rare. |
| `RATE_RECOVER_START_PER_HOUR` | `20` | Per IP. |
| `RATE_LOOKUP_PER_MIN` | `30` | Per authenticated user. |
| `RATE_EDIT_PER_MIN` | `30` | Per user. |
| `RATE_REACTION_PER_MIN` | `60` | Per user. |
| `RATE_LINK_PREVIEW_PER_MIN` | `10` | Per user. Opt-in proxy. |
| `RATE_TURN_CREDENTIALS_PER_MIN` | `10` | Per user. |

**Rate limit keys:**

| Variant | Key format | Window(s) |
|---|---|---|
| `InviteCreate` | `invite_create:{user_id}:{hour\|day}:{boundary}` | Hourly, Daily |
| `InviteRedeem` | `invite_redeem:{ip}:min:{boundary}` | Per minute |
| `KpClaim` | `kp_claim:{user_id}:{minute\|hour}:{boundary}` | Per minute, Hourly |
| `Login` | `login:{ip}:min:{boundary}` | Per minute |
| `DataExport` | `data_export:{user_id}:{boundary}` | `EXPORT_RATE_LIMIT_HOURS` |
| `Presign` | `presign:{user_id}:min:{boundary}` | Per minute |
| `OprfBlind` | `oprf_blind:{ip}:{min\|hour}:{boundary}` | Per minute, Per hour |
| `RecoverStart` | `recover_start:{ip}:{min\|hour}:{boundary}` | Per minute, Per hour |
| `Lookup` | `lookup:{user_id}:min:{boundary}` | Per minute |
| `Edit` | `edit:{user_id}:min:{boundary}` | Per minute |
| `Reaction` | `reaction:{user_id}:min:{boundary}` | Per minute |
| `LinkPreview` | `link_preview:{user_id}:min:{boundary}` | Per minute |
| `TurnCredentials` | `turn_credentials:{user_id}:min:{boundary}` | Per minute |

Rate limit state is stored in a SQLite table and pruned hourly (entries older than 24 hours).

### 5.7 Transport Security

Unchanged from V1 §5.7.

### 5.8 OPAQUE and OPRF

```env
OPAQUE_OPRF_KEY_PATH=/data/oprf.key
USERNAME_OPRF_ENABLED=true
```

| Variable | Default | Notes |
|---|---|---|
| `OPAQUE_OPRF_KEY_PATH` | `/data/oprf.key` | Path to the persisted `ServerSetup` and root secret. |
| `USERNAME_OPRF_ENABLED` | `true` | V2 always enables username OPRF. Kept for forward compatibility. |

**Cipher suite (OPAQUE):** `DefaultCipherSuite` — Ristretto255, `TripleDh<Ristretto255, Sha512>`, Argon2 KSF. Bound to every stored user registration. Do not change without a spec revision.

**OPRF construction (username lookup):** `voprf` crate, Ristretto255-SHA512, base (non-verifiable) mode.

**Key derivation from root secret:**

```
root_secret   = random 32 bytes (persisted at OPAQUE_OPRF_KEY_PATH)
oprf_seed     = HKDF(root_secret, info="opaque-oprf-v1")        -- OPAQUE internal OPRF
username_key  = HKDF(root_secret, info="username-oprf-v1")      -- username OPRF key k
backup_key    = HKDF(root_secret, info="backup-encryption-v1")  -- backups
```

**`root_secret` rotation policy.** `root_secret` is immutable for the lifetime of the deployment. Rotation is a catastrophic, operator-initiated operation requiring full user re-registration. There is no routine rotation. Documented in §12 (Security Boundaries) and §16 (Amendment 13).

**Server setup persistence:** serialized to `OPAQUE_OPRF_KEY_PATH` on first startup, `0600` on Unix. Loss invalidates all user registrations, all backups, and all username tokens.

### 5.9 Backups

Unchanged from V1 §5.9, with one addition:

```
backup_key = HKDF(root_secret, info="backup-encryption-v1")
```

The backup includes the OPRF key, the ALTCHA HMAC secret, the VAPID keys, and all database contents. Attachment blobs are included only if `BACKUP_INCLUDE_ATTACHMENTS=true` and `STORAGE_BACKEND=fs`.

### 5.10 Push Notifications

Unchanged from V1 §5.10, with one removal:

**Push payload no longer includes `sender_user_id` in plaintext.** The push envelope carries `room_id` and an opaque `sender_ref`, which is the sender’s `username_token` truncated to 16 bytes and base64-encoded. Clients look up the sender’s display name from local state or the user channel. This removes a documented metadata leak from V1 §14.6.

### 5.11 ALTCHA

Unchanged from V1 §5.11.

### 5.12 Storage Backend

Unchanged from V1 §5.12, plus:

- The `presign` endpoint remains as specified in V1 §8.6.3.
- Presign TTL clamping remains `[30, 2 × S3_PRESIGN_TTL_SECONDS]`.

### 5.13 Attachment Format

Unchanged from V1 §5.13. The C2SP chunk size is `16384` and is not configurable.

### 5.14 Moderation

Unchanged from V1 §5.14.

### 5.15 Invite Defaults

Unchanged from V1 §5.15.

### 5.16 Safety Numbers

Unchanged from V1 §5.16.

### 5.17 Audit Log

```env
AUDIT_RETENTION_DAYS=90
```

**Metadata constraint.** `audit_log.metadata` MUST NOT contain message content, IP addresses, URLs, or user-identifying data beyond `user_id` and `room_id`. This is enforced by code review; violations are treated as bugs and fixed in patch releases.

### 5.18 Cleanup Scheduler

Unchanged from V1 §5.18.

### 5.19 Sockudo

```env
SOCKUDO_URL=http://sockudo:6001
SOCKUDO_APP_ID=chat
SOCKUDO_APP_KEY=auto
SOCKUDO_APP_SECRET=auto
SOCKUDO_ENABLE_CLIENT_EVENTS=true
```

**Channel taxonomy (V2):**

| Channel | Type | Purpose |
|---|---|---|
| `private-room-{room_id}` | Private | Room events, client events (typing) |
| `private-user-{user_id}` | Private | User-scoped durable events |

**No presence channels.** Presence is out of scope for V2.

**Client events accepted:**

| Event | Channel | Payload |
|---|---|---|
| `client-typing.start` | `private-room-{room_id}` | `{ user_id }` |
| `client-typing.stop` | `private-room-{room_id}` | `{ user_id }` |

**Removed:** `client-read` is no longer accepted. Read state is written via `POST /users/me/read-state`.

### 5.20 GDPR

Unchanged from V1 §5.20.

### 5.21 TLS Termination

Unchanged from V1 §5.21.

### 5.22 Key Transparency

```env
KEY_TRANSPARENCY_ENABLED=true
KEY_TRANSPARENCY_LOG_PATH=/data/kt-log
KEY_TRANSPARENCY_AUDITOR_KEYS=
```

| Variable | Default | Notes |
|---|---|---|
| `KEY_TRANSPARENCY_ENABLED` | `true` | V2 always enables KT. |
| `KEY_TRANSPARENCY_LOG_PATH` | `/data/kt-log` | Append-only log directory. |
| `KEY_TRANSPARENCY_AUDITOR_KEYS` | empty | Comma-separated list of auditor public keys. |

### 5.23 Link Preview Proxy

```env
LINK_PREVIEW_PROXY_ENABLED=false
LINK_PREVIEW_PROXY_TIMEOUT_SECONDS=5
LINK_PREVIEW_PROXY_MAX_BYTES=1048576
```

| Variable | Default | Notes |
|---|---|---|
| `LINK_PREVIEW_PROXY_ENABLED` | `false` | Opt-in. Operator must enable. |
| `LINK_PREVIEW_PROXY_TIMEOUT_SECONDS` | `5` | Per-request timeout. |
| `LINK_PREVIEW_PROXY_MAX_BYTES` | `1048576` | Max response body size. |

### 5.24 Calls

```env
CALLING_ENABLED=false
TURN_URL=
TURN_SHARED_SECRET=
TURN_TTL_SECONDS=600
CALL_MAX_PARTICIPANTS=8
```

| Variable | Default | Notes |
|---|---|---|
| `CALLING_ENABLED` | `false` | Operator must enable. Advertised in `/capabilities`. |
| `TURN_URL` | empty | TURN server URL. Required for calls. |
| `TURN_SHARED_SECRET` | empty | HMAC secret for TURN credential generation. |
| `TURN_TTL_SECONDS` | `600` | TURN credential TTL. |
| `CALL_MAX_PARTICIPANTS` | `8` | Per-call participant cap. |

### 5.25 Username OPRF

```env
OPRF_BLIND_ENABLED=true
```

`/oprf/blind` is always enabled in V2. The variable is retained for future disabling during incident response.

---

## 6. Client Interface Contract

### 6.1 Authentication

- `Authorization: Bearer <session_token>` on every authenticated request.
- Tokens are 43-character base64url strings.
- **Unchanged from V1 §6.1.**

### 6.2 OPAQUE Handshake

- Registration: `start` then `finish`. Both within 5 minutes.
- Login: `start` then `finish`. Both within 5 minutes.
- **Recovery:** `POST /auth/recover/start` then `POST /auth/recover/finish`. Both within 10 minutes.

### 6.3 ALTCHA Payload

Unchanged from V1 §6.3.

### 6.4 MLS Message Envelope

Unchanged from V1 §6.4.

### 6.5 Attachments — Encryption Format

Unchanged from V1 §6.5.

### 6.6 Attachments — Upload

Unchanged from V1 §6.6.

### 6.7 Attachments — Streaming and Seeking

Unchanged from V1 §6.7.

### 6.8 Attachments — Presigned URLs

Unchanged from V1 §6.8.

### 6.9 Attachments — MP4 Streaming

Unchanged from V1 §6.9.

### 6.10 Attachments — Forwarding

Unchanged from V1 §6.10.

### 6.11 Range Request Format

Unchanged from V1 §6.11.

### 6.12 Room Membership

Unchanged from V1 §6.12.

### 6.13 Push Subscriptions

Unchanged from V1 §6.13.

### 6.14 Safety Numbers

Unchanged from V1 §6.14.

### 6.15 WebSocket Connection

Unchanged from V1 §6.15.

### 6.16 Client Events

Only two client events are accepted: `client-typing.start` and `client-typing.stop`. `client-read` is removed. Client events are ephemeral. The server never persists them.

### 6.17 Client Capability Requirements

Unchanged from V1 §6.17.

### 6.18 CoreCrypto Initialization

Unchanged from V1 §6.18.

### 6.19 OPRF Blinding

The client MUST compute the username token before registration, login, and lookup:

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
  7. token     = evaluated / r^k = h^k
```

The client MUST cache the token in memory for the session. The client MUST NOT persist the token at rest.

### 6.20 Display Name Encryption

The client derives the display-name encryption key from the token:

```
material         = HKDF-Expand(token, info="display-name-encryption-v1", length=44)
display_name_key = material[0..32]
nonce            = random 12 bytes
encrypted_display = nonce || AES-256-GCM(display_name_key, nonce, display_name)
```

The client sends `encrypted_display` on registration and on display-name updates. The server stores it opaquely.

### 6.21 Device Name Encryption

The client derives the device-name encryption key from the token:

```
material        = HKDF-Expand(token, info="device-name-encryption-v1", length=44)
device_name_key = material[0..32]
nonce           = random 12 bytes
encrypted_device_name = nonce || AES-256-GCM(device_name_key, nonce, device_name)
```

The client writes `encrypted_device_name` via user-scoped sync. The server stores it opaquely.

### 6.22 User-Scoped Sync Cursor

The client maintains a `user_seq` cursor. On boot, after SQLite hydrate and before subscribing to channels:

1. `GET /users/me/sync?since_seq=<cursor>`
2. Apply returned state rows in `user_seq` order.
3. Store `max_seq` as the new cursor.
4. Subscribe to `private-user-{user_id}`.
5. Apply live events as they arrive; ignore any event with `user_seq <= cursor`.
6. Update the cursor on every applied row.

If the server returns `{ "full_resync_required": true }`, the client discards its cursor and refetches all current state (`GET /users/me/sync?since_seq=0`).

### 6.23 Recovery

The client MUST:

1. Prompt for recovery code and username.
2. Blind the username via `POST /oprf/blind`.
3. Call `POST /auth/recover/start` with `{ recovery_code, username_token }`.
4. Receive a recovery session and an OPAQUE registration challenge.
5. Compute the new `RegistrationRecord` bound to the new password.
6. Encrypt the display name with the current token.
7. Call `POST /auth/recover/finish` with the new record, `encrypted_display`, and `identity_pubkey`.
8. Store the new session.

The client MUST treat session revocation on other devices as expected. Recovery revokes all existing sessions.

### 6.24 Calls

The client MUST:

1. Fetch `calling` from `/capabilities`. If `false`, calls are unavailable.
2. Register for signaling by subscribing to `private-room-{room_id}`.
3. Send signaling messages via `POST /rooms/:id/calls/:call_id/signal`.
4. Fetch TURN credentials via `POST /calls/turn-credentials` before starting a call.
5. Publish `call.start` / `call.end` events via the room channel.

### 6.25 Link Preview Proxy

The client MUST:

1. Fetch `link_preview_proxy` from `/capabilities`. If absent or `false`, the proxy is unavailable.
2. When the proxy is enabled and a URL is CORS-blocked, encrypt the URL with a per-request Content Key.
3. Send the encrypted URL to `POST /link-preview/proxy`.
4. Decrypt the response with the Content Key.

The server never sees the plaintext URL.

---

## 7. Data Model

### 7.1 Identity and Auth

```sql
CREATE TABLE users (
    id                  TEXT PRIMARY KEY,
    username_token      TEXT NOT NULL UNIQUE,    -- OPRF output, base64
    encrypted_display   TEXT,                    -- nonce || AES-256-GCM ciphertext
    opaque_registration BLOB NOT NULL,
    identity_pubkey     TEXT NOT NULL,
    profile             TEXT,                    -- opaque encrypted JSON blob
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
    code_hash   BLOB NOT NULL,             -- Argon2id hash
    code_salt   BLOB NOT NULL,             -- per-code salt
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
-- Ephemeral counters for OPRF abuse detection. Rows older than 24h are pruned.
```

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
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    key         TEXT NOT NULL,
    value_json  TEXT NOT NULL,
    user_seq    INTEGER NOT NULL,
    updated_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (user_id, key)
);

CREATE INDEX idx_user_preferences_seq ON user_preferences(user_id, user_seq);

CREATE TABLE device_names (
    user_id              TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id            TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    encrypted_device_name TEXT NOT NULL,
    user_seq             INTEGER NOT NULL,
    updated_at           DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at           DATETIME,
    PRIMARY KEY (user_id, device_id)
);

CREATE INDEX idx_device_names_seq ON device_names(user_id, user_seq);
```

**`room_order`** is stored in `user_preferences` with `key = 'room_order'` and `value_json` holding the ordered list of room IDs.

### 7.3 Invites

Unchanged from V1 §7.2, except room invite codes remain unchanged.

### 7.4 Rooms

```sql
CREATE TABLE rooms (
    id                    TEXT PRIMARY KEY,
    owner_id              TEXT NOT NULL REFERENCES users(id),
    metadata              TEXT,                         -- opaque encrypted JSON
    metadata_version      INTEGER NOT NULL DEFAULT 1,
    retention_days        INTEGER,
    max_file_size_bytes   INTEGER,
    moderation_override   TEXT,
    call_active           INTEGER NOT NULL DEFAULT 0,
    call_participants     TEXT,
    created_at            DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
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
```

**`rooms.metadata`** is an opaque encrypted JSON blob. The plaintext structure is client-defined. The client spec §13.6 defines the V1 schema for the JSON. The server does not inspect it.

**`rooms.retention_days` and `rooms.max_file_size_bytes`** remain unencrypted server-visible fields because the server enforces retention and file-size limits.

### 7.5 MLS Lifecycle

Unchanged from V1 §7.4, plus:

```sql
CREATE TABLE pending_mls_adds (
    id               TEXT PRIMARY KEY,
    room_id          TEXT NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    target_user_id   TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_client_id TEXT NOT NULL,
    key_package_id   TEXT NOT NULL REFERENCES key_packages(id),
    queued_at        DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    consumed_at      DATETIME
);

CREATE INDEX idx_pending_mls_adds_active
    ON pending_mls_adds(room_id, consumed_at)
    WHERE consumed_at IS NULL;
```

### 7.6 Messages

```sql
CREATE TABLE room_messages (
    id                        TEXT PRIMARY KEY,
    room_id                   TEXT NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    sender_user_id            TEXT NOT NULL REFERENCES users(id),
    sender_client_id          TEXT NOT NULL,
    epoch                     INTEGER NOT NULL,
    seq                       INTEGER NOT NULL,
    content_type              TEXT NOT NULL CHECK(content_type IN ('application', 'commit', 'proposal')),
    ciphertext                BLOB NOT NULL,
    reply_to                  TEXT REFERENCES room_messages(id),
    edit_of                   TEXT REFERENCES room_messages(id),
    edit_sequence             INTEGER NOT NULL DEFAULT 0,
    edited_at                 DATETIME,
    created_at                DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at                DATETIME
);

CREATE INDEX idx_room_messages_room_epoch_seq
    ON room_messages(room_id, epoch, seq);

CREATE INDEX idx_room_messages_room_created
    ON room_messages(room_id, created_at DESC);

CREATE INDEX idx_room_messages_sender
    ON room_messages(sender_user_id, created_at DESC);

CREATE INDEX idx_room_messages_room_deleted
    ON room_messages(room_id, deleted_at)
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
    reaction           TEXT NOT NULL,           -- short opaque string, e.g. emoji
    created_at         DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at         DATETIME,
    UNIQUE (message_id, sender_user_id, sender_client_id, reaction)
);

CREATE INDEX idx_reactions_message ON reactions(message_id)
    WHERE deleted_at IS NULL;

CREATE INDEX idx_reactions_room ON reactions(room_id, created_at DESC);
```

**Edit model.** An edit is a new `room_messages` row with `edit_of = <original id>` and `edit_sequence = <previous + 1>`. The original row remains. The edit chain is reconstructed by querying `edit_of = <original id> ORDER BY edit_sequence`. `edited_at` is set on the original row for quick display. The latest edit is the one with the highest `edit_sequence`. An edit chain is returned with any message fetch that includes the original.

**Edit window.** `edited_at` on the original row is set on the first edit. The server rejects an edit if `now - created_at > edit_window_seconds` (default 900, max 86400).

**Reply model.** `reply_to` is a reference to another `room_messages.id`. Replies are shown inside the message payload ciphertext; the `reply_to` column allows server-side filtering and delta sync.

### 7.7 Attachments

Unchanged from V1 §7.5.

### 7.8 Push Subscriptions

Unchanged from V1 §7.6.

### 7.9 Instance Config and Ops

Unchanged from V1 §7.7, plus:

```sql
CREATE TABLE key_transparency_log (
    leaf_index    INTEGER PRIMARY KEY,
    user_id       TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    username_token TEXT NOT NULL,
    identity_pubkey TEXT NOT NULL,
    added_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_kt_user ON key_transparency_log(user_id);
CREATE INDEX idx_kt_added ON key_transparency_log(added_at);

CREATE TABLE key_transparency_snapshots (
    id           TEXT PRIMARY KEY,
    tree_size    INTEGER NOT NULL,
    root_hash    BLOB NOT NULL,
    signature    BLOB,
    created_at   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE call_sessions (
    id           TEXT PRIMARY KEY,
    room_id      TEXT NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    initiator_id TEXT NOT NULL REFERENCES users(id),
    started_at   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    ended_at     DATETIME
);

CREATE INDEX idx_call_sessions_room ON call_sessions(room_id, started_at DESC);

CREATE TABLE call_participants (
    call_id     TEXT NOT NULL REFERENCES call_sessions(id) ON DELETE CASCADE,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    joined_at   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    left_at     DATETIME,
    PRIMARY KEY (call_id, user_id)
);
```

### 7.10 State Outside the Database

Unchanged from V1 §7.8.

---

## 8. API Surface

All routes prefixed with `/api/v1/`. Auth via `Authorization: Bearer <session_token>` unless noted.

**Error format (extended in V2):**

```json
{
  "error": "machine_readable_code",
  "message": "Human-readable message",
  "details": {}
}
```

### 8.0 Static Client Hosting and HTTPS Enforcement

Unchanged from V1 §8.0.

### 8.1 Public

| Method | Path | Purpose |
|---|---|---|
| GET | `/health` | Liveness |
| GET | `/ready` | Readiness |
| GET | `/capabilities` | Feature advertisement |
| GET | `/auth/register/challenge` | ALTCHA challenge |
| POST | `/auth/register/start` | OPAQUE registration start |
| POST | `/auth/register/finish` | OPAQUE registration finish |
| POST | `/auth/login/start` | OPAQUE login start |
| POST | `/auth/login/finish` | OPAQUE login finish |
| POST | `/auth/recover/start` | Recovery authorization |
| POST | `/auth/recover/finish` | Recovery credential install |
| POST | `/invites/redeem` | Validate and consume server invite |
| GET | `/invites/:code` | Public invite validation |
| POST | `/oprf/blind` | OPRF evaluation (username blinding) |
| POST | `/link-preview/proxy` | Link preview proxy (opt-in) |

**`GET /capabilities` response (V2):**

```json
{
  "version": "2.0.0",
  "calling": true,
  "call_max_participants": 8,
  "push_vapid_public_key": "...",
  "websocket_url": "wss://chat.example.com/realtime",
  "sockudo_app_key": "abc123",
  "sockudo_channel_prefix": "private-room-",
  "altcha": {
    "enabled": true,
    "algorithm": "PBKDF2/SHA-256",
    "cost": 5000
  },
  "storage_backend": "s3",
  "storage_presign_supported": true,
  "storage_presign_max_ttl_seconds": 1200,
  "attachment_format": "c2sp-chunked-aes256gcm-v1",
  "attachment_chunk_size": 16384,
  "attachment_bucket_sizes": [65536, 524288, 4194304, 33554432, 268435456],
  "attachment_accept_ranges": true,
  "username_oprf_enabled": true,
  "oprf_suite": "ristretto255-sha512",
  "key_transparency_enabled": true,
  "link_preview_proxy_enabled": false,
  "safety_number_mode": "warn",
  "moderation_mode": "messenger",
  "edit_window_seconds": 900,
  "reactions_per_message": 50,
  "sync_event_retention_days": 90,
  "threading_enabled": true
}
```

#### 8.1.1 `POST /oprf/blind`

**Auth:** None.

**Rate limits:** `RATE_OPRF_BLIND_PER_MIN`, `RATE_OPRF_BLIND_PER_HOUR`, both per IP.

**Request:**
```json
{ "blinded": "<base64, group element>" }
```

**Response:**
```json
{ "evaluated": "<base64, group element>" }
```

**Behavior:**
1. Decode `blinded`. Reject malformed input with `400 invalid_blinded`.
2. Apply the OPRF key `k` to produce `evaluated`.
3. Return `evaluated`.

**Errors:**

| Error | HTTP | `error` field |
|---|---|---|
| Malformed input | 400 | `invalid_blinded` |
| Rate limited | 429 | `rate_limited` with `details: {reset_at}` |

**Logging:** each request increments an ephemeral counter in `oprf_audit`. No IP, no timing, no linkage to any user. Counters are pruned hourly.

#### 8.1.2 `POST /link-preview/proxy`

**Auth:** None. The URL is encrypted with a per-request Content Key; the server is blind.

**Rate limits:** `RATE_LINK_PREVIEW_PER_MIN`, per IP.

**Request:**
```json
{
  "encrypted_url": "<base64, AES-GCM(ContentKey, nonce, url)>",
  "nonce": "<base64, 12 bytes>"
}
```

**Response:**
```json
{
  "encrypted_metadata": "<base64, AES-GCM(ContentKey, nonce2, metadata_json)>",
  "nonce": "<base64, 12 bytes>"
}
```

**Behavior:**
1. Decrypt `encrypted_url` using the Content Key derived from the request (Content Key delivered out-of-band is not possible; see §8.1.3 for the actual mechanism).

**Note:** the link preview proxy uses a request-scoped key. The client generates a Content Key, encrypts the URL, and sends the encrypted URL **along with a wrapped Content Key** encrypted under the server’s public key. The server unwraps the Content Key in memory only, decrypts the URL, fetches metadata, re-encrypts metadata with the same Content Key, and discards the Content Key. The server never writes the URL or Content Key to disk.

**SSRF protection:**
1. Resolve DNS. Reject private, loopback, link-local, multicast, and reserved IPs.
2. Pin the resolved IP for the request. Re-validate on every redirect.
3. Reject non-HTTP(S) schemes.
4. Cap response body size at `LINK_PREVIEW_PROXY_MAX_BYTES`.
5. Timeout at `LINK_PREVIEW_PROXY_TIMEOUT_SECONDS`.
6. Return only the following fields: `title`, `description`, `site_name`, `favicon_url`, `image_url`. Strip all HTML.

**Errors:**

| Error | HTTP | `error` field |
|---|---|---|
| Malformed request | 400 | `invalid_request` |
| URL is SSRF-blocked | 400 | `url_blocked` |
| Fetch timeout | 504 | `upstream_timeout` |
| Response too large | 413 | `response_too_large` |
| Rate limited | 429 | `rate_limited` with `details: {reset_at}` |
| Proxy disabled | 501 | `link_preview_proxy_disabled` |

### 8.2 User

| Method | Path | Purpose |
|---|---|---|
| POST | `/auth/logout` | Revoke session |
| GET | `/users/me` | Current user |
| PATCH | `/users/me` | Update profile |
| DELETE | `/users/me` | Delete account |
| GET | `/users/me/export` | Export data |
| POST | `/users/lookup` | Username token lookup |
| GET | `/users/me/devices` | List devices |
| DELETE | `/users/me/devices/:id` | Revoke device |
| GET | `/users/me/sessions` | List sessions |
| DELETE | `/users/me/sessions/:id` | Revoke session |
| POST | `/users/me/push-subscriptions` | Register push subscription |
| DELETE | `/users/me/push-subscriptions/:id` | Revoke push subscription |
| GET | `/users/me/sync` | Fetch user-scoped state |
| POST | `/users/me/read-state` | Write read state |
| PATCH | `/users/me/room-order` | Write room order |
| POST | `/users/me/avatar` | Upload avatar |

#### 8.2.1 `GET /users/me`

**Response:**
```json
{
  "id": "<user_id>",
  "username_token": "<base64>",
  "encrypted_display": "<base64>",
  "identity_pubkey": "<base64>",
  "profile": "<base64>",
  "profile_version": 1,
  "created_at": "<ISO 8601>"
}
```

#### 8.2.2 `PATCH /users/me`

**Request:**
```json
{
  "encrypted_display": "<base64>",
  "profile": "<base64>"
}
```

**Response:** `200 OK` with updated user record.

#### 8.2.3 `POST /users/lookup`

**Auth:** Required.

**Rate limit:** `RATE_LOOKUP_PER_MIN` per authenticated user.

**Request:**
```json
{ "username_token": "<base64>" }
```

**Response:**
```json
{
  "user_id": "<user_id>",
  "encrypted_display": "<base64>"
}
```

**Errors:**

| Error | HTTP | `error` field |
|---|---|---|
| Not found | 404 | `user_not_found` |
| Rate limited | 429 | `rate_limited` with `details: {reset_at}` |

**Enumeration posture:** responses to found vs. not-found MUST be indistinguishable in shape and timing. Both hit the database; both return immediately. Rate limit is per authenticated user.

#### 8.2.4 `GET /users/me/sync`

**Query:** `since_seq` (integer, required; `0` for full sync).

**Response:**
```json
{
  "read_state": [
    { "room_id": "...", "last_read_message_id": "...", "user_seq": 42, "updated_at": "...", "deleted_at": null }
  ],
  "user_preferences": [
    { "key": "room_order", "value_json": "[\"r1\",\"r2\"]", "user_seq": 43 }
  ],
  "device_state": [
    { "device_id": "...", "encrypted_device_name": "...", "user_seq": 44, "updated_at": "...", "deleted_at": null }
  ],
  "max_seq": 50,
  "full_resync_required": false
}
```

**Behavior:**
1. Read `users.next_seq` for the authenticated user.
2. Query all user-scoped state tables for rows with `user_seq > since_seq`.
3. If `since_seq` is older than the retention window (see §4.2), return `full_resync_required: true` and empty state.
4. Return rows ordered ascending by `user_seq`.

#### 8.2.5 `POST /users/me/read-state`

**Request:**
```json
{ "room_id": "...", "last_read_message_id": "..." }
```

**Response:** `200 OK` with the new `user_seq`.

**Behavior:**
1. Verify membership in `room_id`.
2. In one transaction: increment `user_seq`, upsert `read_state`, publish `read.sync` on `private-user-{user_id}`.
3. Optionally publish `read.count` on `private-room-{room_id}` with the new aggregate.

#### 8.2.6 `PATCH /users/me/room-order`

**Request:**
```json
{ "room_ids": ["r1", "r2", "r3"] }
```

**Response:** `200 OK` with the new `user_seq`.

**Behavior:**
1. Verify the list contains only rooms the user is a member of.
2. In one transaction: increment `user_seq`, upsert `user_preferences` row with `key='room_order'`, publish `room_order.sync` on `private-user-{user_id}`.

#### 8.2.7 `POST /users/me/avatar`

**Request:** `multipart/form-data` with a single `file` part (encrypted avatar image).

**Response:** `201 Created` with the attachment record (same shape as `POST /rooms/:id/attachments`).

**Behavior:**
1. Store the encrypted avatar as an attachment with `room_id = null` (avatar blobs are user-scoped, not room-scoped).
2. Update `users.profile` with the new `avatar_file_id`.
3. Publish `user.updated` on `private-user-{user_id}`.

### 8.3 Admin

Unchanged from V1 §8.3, plus:

| Method | Path | Purpose |
|---|---|---|
| POST | `/admin/oprf/rotate` | Rotate the username OPRF key (catastrophic) |
| GET | `/admin/key-transparency` | Read KT log stats |
| POST | `/admin/key-transparency/snapshot` | Trigger a signed snapshot |

### 8.4 Rooms

| Method | Path | Purpose |
|---|---|---|
| POST | `/rooms` | Create room |
| GET | `/rooms` | List my rooms |
| GET | `/rooms/:id` | Room metadata (includes effective limits) |
| PATCH | `/rooms/:id` | Update room metadata |
| DELETE | `/rooms/:id` | Delete room |
| POST | `/rooms/:id/leave` | Leave room |
| POST | `/rooms/:id/transfer` | Transfer ownership |
| GET | `/rooms/:id/members` | List members (paginated) |
| POST | `/rooms/:id/members` | Add member |
| DELETE | `/rooms/:id/members/:uid` | Kick |
| POST | `/rooms/:id/members/:uid/promote` | Promote to moderator |
| POST | `/rooms/:id/members/:uid/demote` | Demote |
| POST | `/rooms/:id/invites` | Create room invite |
| GET | `/rooms/:id/invites` | List room invites |
| DELETE | `/rooms/:id/invites/:code` | Revoke room invite |
| POST | `/rooms/join` | Join via invite code |
| POST | `/rooms/:id/retention/preview` | Preview retention change |

#### 8.4.1 `PATCH /rooms/:id`

**Auth:** Owner only.

**Request:**
```json
{
  "metadata": "<base64, opaque encrypted JSON>",
  "retention_days": 604800,
  "max_file_size_bytes": 104857600
}
```

All fields optional. `metadata` is stored opaquely. `retention_days` and `max_file_size_bytes` are validated against instance limits.

**Behavior:**
1. Verify owner.
2. In one transaction: update `rooms`, increment `metadata_version`, publish `room.updated` on `private-room-{room_id}`.
3. Return updated room record.

#### 8.4.2 `GET /rooms/:id/members`

**Query:** `cursor` (optional, opaque), `limit` (default 50, max 200).

**Response:**
```json
{
  "members": [
    {
      "user_id": "...",
      "encrypted_display": "...",
      "role": "member",
      "joined_at": "..."
    }
  ],
  "next_cursor": "opaque-or-null"
}
```

#### 8.4.3 `POST /rooms/:id/retention/preview`

**Auth:** Owner only.

**Request:**
```json
{ "retention_days": 604800 }
```

**Response:**
```json
{
  "messages_affected": 1234,
  "attachments_affected": 56,
  "earliest_affected": "<ISO 8601>",
  "latest_affected": "<ISO 8601>"
}
```

### 8.5 MLS and Messaging

| Method | Path | Purpose |
|---|---|---|
| POST | `/keypackages` | Upload KeyPackage batch |
| GET | `/keypackages/count` | Count unconsumed packages |
| POST | `/keypackages/claim` | Atomically claim a KeyPackage |
| GET | `/welcomes` | List pending welcomes |
| GET | `/welcomes/:id` | Fetch a welcome |
| POST | `/welcomes/:id/consume` | Mark welcome consumed |
| POST | `/rooms/:id/messages` | Submit MLS message |
| GET | `/rooms/:id/messages` | List messages |
| GET | `/rooms/:id/messages/:id/ciphertext` | Fetch message ciphertext |
| PATCH | `/rooms/:id/messages/:msg_id` | Edit a message |
| DELETE | `/rooms/:id/messages/:msg_id` | Delete (tombstone) a message |
| POST | `/rooms/:id/messages/:msg_id/reactions` | Add a reaction |
| DELETE | `/rooms/:id/messages/:msg_id/reactions/:reaction_id` | Remove a reaction |
| GET | `/rooms/:id/messages/:msg_id/reactions` | List reactions |
| GET | `/rooms/:id/epoch` | Current epoch and sequence |
| GET | `/rooms/:id/pending-removes` | List pending MLS removes |
| POST | `/rooms/:id/pending-removes/:id/consume` | Mark remove consumed |
| GET | `/rooms/:id/pending-adds` | List pending MLS adds |
| POST | `/rooms/:id/pending-adds/:id/consume` | Mark add consumed |
| POST | `/sockudo/auth` | Sign a channel subscription |

#### 8.5.1 Message Submission

**Request** unchanged from V1 §8.5, plus optional `reply_to`.

#### 8.5.2 Delta Sync Cursor

Unchanged from V1 §8.5.1, plus: responses include `reply_to`, `edit_of`, `edit_sequence`, `edited_at` on each message.

#### 8.5.3 `PATCH /rooms/:id/messages/:msg_id`

**Auth:** Sender only. Window enforced by `edit_window_seconds`.

**Request:**
```json
{
  "ciphertext": "<base64, new MLS application message>",
  "content_type": "application"
}
```

**Response:** `201 Created` with the new message row (which has `edit_of` and `edit_sequence` set).

**Behavior:**
1. Verify sender matches `sender_user_id` of `msg_id`.
2. Verify `now - created_at <= edit_window_seconds`.
3. Verify `msg_id` is not itself an edit (edits target the original).
4. Increment `edit_sequence` = max existing `edit_sequence` for `edit_of = msg_id` + 1.
5. Insert new `room_messages` row.
6. Set `edited_at = now` on the original if not set.
7. Publish `message.edited` on `private-room-{room_id}`.

**Errors:**

| Error | HTTP | `error` field |
|---|---|---|
| Not the sender | 403 | `forbidden` |
| Edit window passed | 403 | `edit_window_passed` |
| Message not found | 404 | `message_not_found` |
| Message deleted | 404 | `message_deleted` |
| Rate limited | 429 | `rate_limited` |

#### 8.5.4 `DELETE /rooms/:id/messages/:msg_id`

Unchanged from V1 §8.5.2. Editing a message after it is deleted is rejected with `403 forbidden`.

#### 8.5.5 Reactions

**`POST /rooms/:id/messages/:msg_id/reactions`**

**Request:**
```json
{ "reaction": "👍" }
```

**Response:** `201 Created` with the reaction row.

**Behavior:**
1. Verify membership in `room_id`.
2. Verify message exists and is not tombstoned.
3. Enforce `reactions_per_message`.
4. Upsert. On conflict (same user, same message, same reaction), return `200 OK` with the existing row.
5. Publish `reaction.added` on `private-room-{room_id}`.

**`DELETE /rooms/:id/messages/:msg_id/reactions/:reaction_id`**

Sender-only. Soft-delete via `deleted_at`. Publish `reaction.removed`.

**`GET /rooms/:id/messages/:msg_id/reactions`**

Returns the aggregated list:
```json
{
  "reactions": [
    { "reaction": "👍", "count": 3, "sender_user_ids": ["u1","u2","u3"] },
    { "reaction": "🎉", "count": 1, "sender_user_ids": ["u1"] }
  ]
}
```

#### 8.5.6 Pending MLS Adds

Mirrors V1 §8.5 for `pending_mls_removes`. Endpoints:

- `GET /rooms/:id/pending-adds`
- `POST /rooms/:id/pending-adds/:id/consume`

### 8.6 Attachments

Unchanged from V1 §8.6.

### 8.7 Calls

| Method | Path | Purpose |
|---|---|---|
| POST | `/calls/turn-credentials` | Fetch TURN credentials |
| POST | `/rooms/:id/calls/:call_id/signal` | Send signaling message |
| POST | `/rooms/:id/calls/:call_id/end` | End call |

#### 8.7.1 `POST /calls/turn-credentials`

**Auth:** Required.

**Rate limit:** `RATE_TURN_CREDENTIALS_PER_MIN` per user.

**Response:**
```json
{
  "urls": ["turn:turn.example.com:3478?transport=udp"],
  "username": "<ephemeral-username>",
  "credential": "<HMAC-signed-password>",
  "ttl_seconds": 600
}
```

**Behavior:**
1. Generate ephemeral TURN username `user_id:timestamp`.
2. Compute credential = `base64(HMAC-SHA1(TURN_SHARED_SECRET, username))`.
3. Return credentials.

**Errors:**

| Error | HTTP | `error` field |
|---|---|---|
| Calling disabled | 501 | `calling_disabled` |
| Rate limited | 429 | `rate_limited` |

#### 8.7.2 `POST /rooms/:id/calls/:call_id/signal`

**Auth:** Required. Room membership required.

**Request:**
```json
{
  "recipient_user_id": "u2",
  "signal_type": "offer" | "answer" | "ice",
  "payload": "<base64, opaque>"
}
```

**Behavior:**
1. Verify membership in `room_id`.
2. Verify the call is active (`call_sessions.ended_at IS NULL`).
3. Publish `call.signal` on `private-user-{recipient_user_id}` with the caller’s `user_id`, `call_id`, `signal_type`, and `payload`.

### 8.8 Event Catalog

**Room events** (`private-room-{room_id}`):

| Event | Payload | When |
|---|---|---|
| `message.new` | `{ id, room_id, sender_user_id, sender_client_id, epoch, seq, content_type, reply_to, created_at }` | After `POST /rooms/:id/messages` succeeds |
| `message.edited` | `{ id, edit_of, edit_sequence, room_id, sender_user_id, created_at }` | After `PATCH /rooms/:id/messages/:msg_id` succeeds |
| `message.deleted` | `{ id, room_id }` | After `DELETE /rooms/:id/messages/:msg_id` succeeds |
| `reaction.added` | `{ id, room_id, message_id, sender_user_id, reaction, created_at }` | After `POST .../reactions` succeeds |
| `reaction.removed` | `{ id, room_id, message_id }` | After `DELETE .../reactions/:id` succeeds |
| `room.updated` | `{ room_id, metadata?, retention_days?, max_file_size_bytes? }` | After `PATCH /rooms/:id` succeeds |
| `room.member_added` | `{ room_id, user_id, role, joined_at }` | After `POST /rooms/:id/members` succeeds |
| `room.member_removed` | `{ room_id, user_id }` | After kick or leave |
| `epoch.updated` | `{ room_id, epoch, sequence }` | After `room_epochs` advances |
| `call.started` | `{ call_id, room_id, initiator_id, started_at }` | When `call_sessions` row is created |
| `call.ended` | `{ call_id, room_id, ended_at }` | When `call_sessions.ended_at` is set |
| `call.signal` | `{ call_id, sender_user_id, signal_type, payload }` | After `POST .../signal` succeeds |

**User events** (`private-user-{user_id}`, durable):

| Event | Payload | When |
|---|---|---|
| `read.sync` | `{ room_id, last_read_message_id, user_seq }` | After `POST /users/me/read-state` |
| `room_order.sync` | `{ room_ids, user_seq }` | After `PATCH /users/me/room-order` |
| `device.added` | `{ device_id, platform, added_at, user_seq }` | On device registration |
| `device.revoked` | `{ device_id, reason, user_seq }` | On device revocation |
| `device.name_updated` | `{ device_id, encrypted_device_name, user_seq }` | After device-name sync write |
| `session.revoked` | `{ session_id, reason }` | On explicit revocation |
| `user.updated` | `{ user_id, profile_version, user_seq }` | After `PATCH /users/me` or avatar change |
| `call.signal` | `{ call_id, sender_user_id, signal_type, payload }` | After `POST .../signal` |
| `kt.snapshot` | `{ tree_size, root_hash, created_at }` | When a new KT snapshot is signed |

**Delivery semantics:**

- **Room events:** best-effort. Clients reconcile via REST.
- **User events:** durable via `user_seq`. Live push over `private-user-{user_id}`; catch-up via `GET /users/me/sync`.

---

## 9. CLI Subcommands

Unchanged from V1 §9, plus:

| Command | Purpose |
|---|---|
| `server oprf rotate --confirm` | Resample the username OPRF key (catastrophic) |
| `server kt snapshot` | Trigger a signed key transparency snapshot |
| `server kt verify --from <index>` | Verify KT log integrity from an index |

---

## 10. Client Contract Matrix

| Feature | Server behaviour |
|---|---|
| Text messaging | Relays MLS ciphertext via Sockudo; never inspects content |
| Message editing | Stores edit chain as `edit_of`/`edit_sequence` rows; enforces `edit_window_seconds` |
| Message deletion | Tombstones via server endpoint; publishes `message.deleted` |
| Reactions | Aggregated; `reaction.added`/`reaction.removed`; silent |
| Threading | `reply_to` field on messages |
| Attachments — upload | Content-addressed, C2SP chunked encryption, validates manifest |
| Attachments — download | Supports `Range` headers; returns `206 Partial Content` |
| Attachments — streaming | Client translates plaintext ranges; server serves encrypted bytes |
| Attachments — presign | S3 only; returns signed URLs; filesystem returns 501 |
| Attachment chunk format | 16 KiB chunks (C2SP protocol constant) |
| Attachment padding | Plaintext-level padding before encryption |
| Identity | OPRF token; server stores no plaintext usernames |
| Display names | AES-GCM ciphertext; key derived from OPRF token via HKDF |
| Device names | AES-GCM ciphertext; user-scoped sync |
| Recovery | Server-generated codes; Argon2id hashes; OPAQUE re-registration |
| Multi-device sync | User-scoped state tables + `user_seq` cursor |
| User events | Durable; live push + REST catch-up |
| Room events | Best-effort; REST reconciliation |
| Push notifications | Accepts `web`, `ios`, `android`, `desktop`; suppresses per-device |
| Multi-device | Up to `devices_per_user` (default 10) |
| Safety numbers | Advisory only; server never sees them |
| ALTCHA | Challenge endpoint on registration only |
| Calling | Signaling events on room channel; TURN credentials via dedicated endpoint |
| Data export | ZIP of ciphertext and account metadata |
| Account deletion | Anonymises user record; cascades session, device, KeyPackage |
| WebSocket | `wss://` in production, `ws://` in development |
| HTTPS | Enforced by reverse proxy; server redirects and sends HSTS |
| Storage backend | Advertised as `storage_backend` in `/capabilities` |
| Presign support | Advertised as `storage_presign_supported` in `/capabilities` |
| Key transparency | Inclusion proofs + auditor signatures in identity key responses |
| Link preview proxy | Opt-in; SSRF-guarded; URL encrypted with request-scoped Content Key |
| Client events | `client-typing.*` only |
| Delta sync | `(epoch, seq)` cursor via `since_epoch` + `since_seq` |
| User sync | `user_seq` cursor via `GET /users/me/sync` |

---

## 11. Phased Build Plan

V1 phases 1–20 are complete. V2 phases:

| Phase | Deliverable | Depends On |
|---|---|---|
| 21 | OPRF identity: `/oprf/blind`, `users.username_token`, `encrypted_display`, updated register/login/lookup | 4a |
| 22 | Recovery: `/auth/recover/start`, `/auth/recover/finish`, `recovery_codes` table | 21 |
| 23 | User-scoped sync foundation: `user_seq`, `read_state`, `user_preferences`, `device_names`, `GET /users/me/sync` | 21 |
| 24 | User-scoped sync writes: `POST /users/me/read-state`, `PATCH /users/me/room-order`, user-channel events | 23 |
| 25 | Device model: name removal, `device.added`/`device.revoked`/`device.name_updated` events, device linking | 23, 24 |
| 26 | `PATCH /rooms/:id` with encrypted metadata; `room.updated` extension | 23 |
| 27 | Message editing: `room_messages` schema, `PATCH /rooms/:id/messages/:msg_id`, `message.edited` event | 11 |
| 28 | Reactions: table, endpoints, aggregation, `reaction.added`/`reaction.removed` | 11 |
| 29 | Threading: `reply_to` in schema, sync responses, message submission | 11 |
| 30 | Member pagination | 7a |
| 31 | Retention preview: `POST /rooms/:id/retention/preview` | 7a |
| 32 | `POST /users/me/avatar` | 12a |
| 33 | `pending_mls_adds` coordination | 10 |
| 34 | Key transparency: log, snapshots, inclusion proofs, auditor signatures | 4a |
| 35 | Link preview proxy: SSRF guard, request-scoped Content Key | 4a |
| 36 | Call signaling: events, `call_sessions`/`call_participants` tables | 11 |
| 37 | TURN credentials endpoint | 36 |
| 38 | Admin UI additions | 13 |
| 39 | Capabilities update | 21–38 |

**Critical path:** 21 → 23 → 24 → 25. Everything else can proceed in parallel after 21.

---

## 12. Security Boundaries

Unchanged from V1 §12, plus:

| Boundary | Guarantee |
|---|---|
| Server sees plaintext username | Never |
| Server sees plaintext display name | Never |
| Server sees plaintext device name | Never |
| Server can correlate username to account | Only with the OPRF key |
| Server can enumerate usernames from database | No |
| Server can reverse OPRF tokens | No (without the key) |
| Server can decrypt display names | No |
| Server can decrypt device names | No |
| OPRF key is immutable | Yes (compromise-only rotation) |
| Recovery codes hashed | Yes (Argon2id) |
| Recovery revokes all sessions | Yes |
| User-scoped sync is durable | Yes (via `user_seq`) |
| Link preview proxy sees plaintext URL | No (request-scoped Content Key) |
| Key transparency log is append-only | Yes |
| Calling is opt-in | Yes (`CALLING_ENABLED`) |
| TURN credentials are ephemeral | Yes (TTL-limited) |

---

## 13. Backups and Disaster Recovery

Unchanged from V1 §13, with the addition that backups include the OPRF key file, the ALTCHA HMAC secret, and the key transparency log directory. Restoring a backup without the OPRF key file invalidates all user registrations.

---

## 14. GDPR Compliance

### 14.1 Data Subject Rights

Unchanged from V1 §14.1.

### 14.2 Account Deletion

Unchanged from V1 §14.2, plus:

- `username_token`, `encrypted_display`, `encrypted_device_name` are zeroed or removed.
- `recovery_codes` rows are deleted.
- User-scoped sync state rows (`read_state`, `user_preferences`, `device_names`) are deleted.
- Key transparency log entries are retained (append-only) but the `user_id` reference is anonymised. The log records the `username_token` and `identity_pubkey` that were published; both are already public.

### 14.3 Data Export

Unchanged from V1 §14.3, plus:

- `username_token` is included but noted as only interpretable by the user.
- `encrypted_display` and `encrypted_device_name` are included as opaque ciphertext.
- `recovery_codes` metadata (creation dates, consumption) is included; hashes are not.

### 14.4 Retention

Unchanged from V1 §14.4, plus `sync_event_retention_days` and `key_transparency_retention_days`.

### 14.5 Tombstone Semantics

Unchanged from V1 §14.5.

### 14.6 Metadata Minimisation

| Field | Leak | Mitigation |
|---|---|---|
| `users.username_token` | Opaque; requires OPRF key to invert | Key is the only reversal path; compromise is catastrophic and documented |
| `users.encrypted_display` | Opaque ciphertext | Key derived from OPRF token; server never sees it |
| `devices.encrypted_device_name` | Opaque ciphertext | Key derived from OPRF token; server never sees it |
| `sessions.last_seen_at` | Timestamp | Coarsened to hour in admin views |
| `messages.created_at` | Timestamp | Coarsened to minute |
| `room_messages.edited_at` | Server knows when edits occurred | Coarsened to day in admin views |
| `room_messages.deleted_at` | Server knows which messages were deleted | Coarsened to day in admin views |
| `push_subscriptions.push_token` | Only unavoidable personal data | Auto-revoked after 90 days of inactivity |
| Push `sender_ref` | Truncated OPRF token | Requires OPRF key to correlate |
| `audit_log.metadata` | JSON | Never includes content, IPs, URLs, or identifiers beyond `user_id` and `room_id` |
| `kt_log.username_token` | Same as `users.username_token` | Public by design (it is the KT subject) |

### 14.7 Controller Obligations

Unchanged from V1 §14.7.

### 14.8 Audit Actions

V1 actions plus:

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

---

## 15. Amendment Process

Unchanged from V1 §15.

---

## 16. Amendments (V2)

### 16.1 Amendment 13 — OPRF Identity Layer

**Date:** 2026-09-29

**Change:** Replaced plaintext `users.username`, `users.username_hash`, and `users.display_name` with OPRF-based `username_token` and `encrypted_display`. Added `POST /oprf/blind`. Updated registration, login, and lookup to use `username_token`. Removed `devices.name`; device names become user-scoped sync state.

**Rationale:** Privacy and GDPR. The server moves from holding plaintext human identifiers to holding opaque tokens.

**Dependency:** `voprf` crate (Ristretto255-SHA512). OPRF key `k` derived from `root_secret` via HKDF info `"username-oprf-v1"`. Immutable for account lifetime; compromise-only rotation.

**Sections affected:** §2.3, §5.8, §6.19, §6.20, §6.21, §7.1, §7.2, §8.1.1, §8.2, §14, §16.

**Affected phases:** 21, 22, 23, 24, 25.

### 16.2 Amendment 14 — Recovery Flow

**Date:** 2026-09-29

**Change:** Added `POST /auth/recover/start` and `POST /auth/recover/finish`. Recovery codes are server-generated, Argon2id-hashed, and single-use by default. Recovery re-registers the OPAQUE record and revokes all existing sessions.

**Sections affected:** §5.6, §6.23, §7.1, §8.1, §8.2.

**Affected phases:** 22.

### 16.3 Amendment 15 — User-Scoped Sync

**Date:** 2026-09-29

**Change:** Added `user_seq`, `read_state`, `user_preferences`, `device_names` tables. Added `GET /users/me/sync`, `POST /users/me/read-state`, `PATCH /users/me/room-order`. User events are durable; room events remain best-effort.

**Sections affected:** §4.5, §6.22, §7.2, §8.2.4–8.2.6, §8.8.

**Affected phases:** 23, 24, 25.

### 16.4 Amendment 16 — Room Metadata and Retention Preview

**Date:** 2026-09-29

**Change:** Replaced `rooms.name_encrypted` with `rooms.metadata` (opaque encrypted JSON). Added `PATCH /rooms/:id` and `POST /rooms/:id/retention/preview`. Added `metadata_version` column.

**Sections affected:** §7.4, §8.4.

**Affected phases:** 26, 31.

### 16.5 Amendment 17 — Message Editing and Threading

**Date:** 2026-09-29

**Change:** Added `reply_to`, `edit_of`, `edit_sequence`, `edited_at` columns. Added `PATCH /rooms/:id/messages/:msg_id`. Added `message.edited` event. Enforced `edit_window_seconds`.

**Sections affected:** §4.2, §7.6, §8.5.3, §8.8.

**Affected phases:** 27, 29.

### 16.6 Amendment 18 — Reactions

**Date:** 2026-09-29

**Change:** Added `reactions` table and endpoints. Added `reaction.added` and `reaction.removed` events. Enforced `reactions_per_message`.

**Sections affected:** §4.2, §7.6, §8.5.5, §8.8.

**Affected phases:** 28.

### 16.7 Amendment 19 — Key Transparency

**Date:** 2026-09-29

**Change:** Added `key_transparency_log`, `key_transparency_snapshots` tables. Added auditor signature support. Added `kt.snapshot` event and admin endpoints.

**Sections affected:** §5.22, §7.9, §8.3, §8.8.

**Affected phases:** 34.

### 16.8 Amendment 20 — Link Preview Proxy

**Date:** 2026-09-29

**Change:** Added opt-in link preview proxy with SSRF guard. URLs are encrypted with a request-scoped Content Key; the server is blind. Added `POST /link-preview/proxy`.

**Sections affected:** §5.23, §8.1.2, §8.1 (capabilities).

**Affected phases:** 35.

### 16.9 Amendment 21 — Call Signaling and TURN

**Date:** 2026-09-29

**Change:** Added `call_sessions`, `call_participants` tables. Added `POST /calls/turn-credentials`, `POST /rooms/:id/calls/:call_id/signal`, `POST /rooms/:id/calls/:call_id/end`. Added `call.*` events.

**Sections affected:** §5.24, §7.9, §8.7, §8.8.

**Affected phases:** 36, 37.

### 16.10 Amendment 22 — Member Pagination and Avatar Upload

**Date:** 2026-09-29

**Change:** Added cursor pagination to `GET /rooms/:id/members`. Added `POST /users/me/avatar`.

**Sections affected:** §8.2.7, §8.4.2.

**Affected phases:** 30, 32.

### 16.11 Amendment 23 — Removal of Presence

**Date:** 2026-09-29

**Change:** Presence is out of scope. No presence channels, no `presence_visibility` column, no live presence indicators. The client spec’s “Last seen” setting (§17.2) is removed by the client team.

**Sections affected:** §2.2, §5.19.

**Affected phases:** None.

### 16.12 Amendment 24 — Push Payload Metadata Reduction

**Date:** 2026-09-29

**Change:** Removed `sender_user_id` from the push payload. Replaced with `sender_ref`, a truncated OPRF token. This eliminates the last plaintext metadata leak in the push envelope.

**Sections affected:** §5.10, §14.6.

**Affected phases:** 15 (revision), 16 (revision).

---

## 17. V1 Amendment Log (Historical)

The V1 amendment log (§16.1–§16.12 of Server Specification v1.0) is preserved for historical reference. Those amendments are superseded by V2 where they conflict, and carried forward where they do not.

| V1 amendment | V2 disposition |
|---|---|
| 1 — C2SP chunked encryption | Carried forward |
| 2 — Message deletion endpoint | Carried forward |
| 3 — Event catalog and client events | Revised by §16.11 (client-read removed) |
| 4 — Delta sync cursor | Carried forward |
| 5 — Push payload schema | Revised by §16.12 (sender_user_id removed) |
| 6 — Effective limits exposure | Carried forward |
| 7 — Presence out of scope | Reaffirmed by §16.11 |
| 8 — CoreCrypto initialization clarification | Carried forward |
| 9 — Context binding test vectors | Carried forward |
| 10 — Chunk size correction | Carried forward |
| 11 — Sync ordering fix | Carried forward |
| 12 — Padding algorithm replacement | Carried forward |

---

## 18. Document Status

This is the contract for the server side of the system. Every V2 implementation task references this document. If a task conflicts with this spec, the task is wrong and must be revised. If a feature is missing, it does not exist yet — it must be added here first, then built.

Amendments are tracked in §16. The V1 amendment log is preserved in §17 for historical reference.

**End of Server Specification v2.0.**
