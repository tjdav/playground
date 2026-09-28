
# Server Specification v1.0

> **Status:** Frozen — source of truth for all Jules tasks.
> **Scope:** This document describes the **server-side contract only**. Client implementation is out of scope and will be covered by a separate client specification.
> **Stack:** Sockudo + Axum + SQLite + OPAQUE + ALTCHA + S3 (or filesystem)
> **Deployment:** Single VPS, Docker-based, self-hosted. Two containers: `server` (Axum + static SPA hosting) and `sockudo`. TLS terminates at a reverse proxy (Coolify/Traefik or Caddy).

Any change to this document requires an explicit revision and a corresponding task update.

---

## 1. Product Summary

A self-hosted, end-to-end encrypted group messaging system with MLS-grade forward secrecy and post-compromise security. The server is an untrusted delivery service that never sees plaintext, keys, or meaningful metadata.

| Layer | Technology |
|---|---|
| Delivery | Sockudo (WebSocket, Pusher v7, Protocol V2 history) |
| API | Axum (Rust) |
| Database | SQLite (embedded in the Axum process) |
| Blob storage | S3 (default) or filesystem (fallback) |
| MLS engine | Wire CoreCrypto 10.5.2 (client-side only) |
| Auth | OPAQUE (aPAKE) via `opaque-ke` 4.0.1 |
| Bot protection | ALTCHA Proof-of-Work v2 via `altcha` 0.2.0 |
| S3 client | `rust-s3` |
| Static SPA hosting | Axum `ServeDir` with SPA fallback |
| TLS termination | External reverse proxy (Traefik via Coolify, or Caddy) |

The client is delivered as a separate package (`client/`) and is not covered by this specification.

---

## 2. Scope

### 2.1 In Scope — V1

- Invite-only registration with configurable codes
- OPAQUE authentication
- ALTCHA proof-of-work bot protection on registration
- Multi-device support (default 10 devices per user)
- Rooms as the unit of conversation (1:1 = 2-member room)
- Text messaging with read receipts and typing indicators
- Message deletion (tombstone; no editing)
- Encrypted attachments with chunked AES-GCM and byte-range streaming
- Push notifications (Web Push, APNs, FCM, UnifiedPush)
- Safety number verification (soft warning, non-blocking)
- Recovery via multi-use one-time code list
- Admin and user web UIs (both first-class)
- Automatic SQLite + Sockudo backups
- Three-tier resource limits (server / instance / entity)
- Display name separate from immutable username
- Server-side cleanup jobs
- GDPR data subject rights (access, erasure, portability)
- Static SPA hosting from the same origin as the API
- HTTPS enforcement via reverse proxy with HSTS

### 2.2 In Scope — V2

- Voice and video calling (WebRTC + TURN, P2P mesh)
- Per-call participant limit (configurable)
- Optional SFU for large calls
- Message editing
- Reactions
- Threading
- Multi-seed OPRF rotation

### 2.3 Out of Scope

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

---

## 3. Roles and Permissions

### 3.1 Global Roles

| Role | Level | Permissions |
|---|---|---|
| `owner` | 100 | `*` |
| `admin` | 80 | `user.manage`, `invite.unlimited`, `config.edit`, `room.force_delete`, `backup.manage` |
| `inviter` | 50 | `invite.limited` |
| `member` | 10 | `room.create`, `room.join`, `message.send` |

### 3.2 Room Roles

| Role | Permissions |
|---|---|
| `owner` | Kick members, delete room, promote/demote moderators (Discord mode only), transfer ownership |
| `moderator` | Kick members (Discord mode only) |
| `member` | Send messages, upload attachments, leave room |

### 3.3 Moderation Modes

- **Messenger mode (default):** Only room owner can kick. Moderators cannot exist.
- **Discord mode (optional):** Room owner can promote members to moderators. Moderators can kick.

Toggled per-instance via `MODERATION_MODE`. Live and applies to all rooms retroactively.

### 3.4 Enforcement Order

Every API call checks:
1. Global permission for the action
2. Room role for the target room (if applicable)

### 3.5 Room Ownership Transfer

- Owner can explicitly transfer ownership to any other member.
- If owner leaves without transferring, ownership passes to the longest-tenured moderator, or to the longest-tenured member.
- If the room has no other members, it is deleted.
- Instance admins can force-transfer ownership.

---

## 4. Resource Limits

### 4.1 Three-Tier Model

```
Server hard limit (env var, deploy-time)
    ↓ cannot be exceeded
Instance limit (admin UI, stored in DB)
    ↓ cannot be exceeded
Per-user / per-room override
```

Effective limit = `MIN(override, instance, server)`.

### 4.2 Default Limits

| Key | Server hard max | Instance default | Instance range |
|---|---|---|---|
| `file_size_bytes` | 104857600 (100 MB) | 104857600 | 1 MB – 100 MB |
| `room_size` | 1000 | 100 | 2 – 1000 |
| `rooms_per_user` | 500 | 50 | 1 – 500 |
| `devices_per_user` | 20 | 10 | 1 – 20 |
| `keypackages_per_device` | 50 | 20 | 5 – 50 |
| `message_size_bytes` | 65536 (64 KB) | 16384 (16 KB) | 256 B – 64 KB |
| `attachment_retention_days` | 365 | 0 (forever) | 0 – 365 |
| `call_max_participants` | 50 | 8 | 2 – 50 |

### 4.3 Per-Entity Overrides

**Rooms:** `max_file_size_bytes`, `message_retention_days`
**Users:** `max_file_size_bytes`

All nullable. `NULL` means "use instance default."

### 4.4 Cleanup Jobs

All cleanup jobs run on a shared hourly scheduler.

| Job | Retention | Notes |
|---|---|---|
| Session cleanup | 30 days after expiry or revocation | Preserves active and recently-expired sessions |
| Rate limit table | 24 hours | Longest rate window is daily |
| Audit log | `AUDIT_RETENTION_DAYS` (default 90) | Compliance-aligned default |
| Attachment pruning | Three-tier effective retention | Deletes row and blob |
| Welcome expiry | 7 days | Stale welcomes deleted |
| Message retention | Per-room, falling to instance default | Deletes messages from Sockudo history and DB |
| Registration state | 5-minute TTL | In-memory, opportunistic purge |
| Login state | 5-minute TTL | In-memory, opportunistic purge |

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

| Variable | Default | Notes |
|---|---|---|
| `APP_ENV` | `production` | `production` or `development` |
| `APP_URL` | — | **Required.** Must begin with `https://` in production. |
| `APP_NAME` | `Encrypted Chat` | Display name |
| `LOG_LEVEL` | `info` | `trace`, `debug`, `info`, `warn`, `error` |
| `CLIENT_STATIC_DIR` | — | Optional. When set, Axum serves the SPA from this directory. |

**Startup validation:** In production, `APP_URL` must start with `https://`. The server exits on mismatch.

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
```

| Variable | Default | Notes |
|---|---|---|
| `RATE_PRESIGN_PER_MIN` | `60` | Presigned URL generation per user per minute |

### 5.7 Transport Security

```env
CORS_ALLOWED_ORIGINS=
CSRF_PROTECTION=none
TRUST_PROXY=false
HSTS_MAX_AGE=31536000
HSTS_INCLUDE_SUBDOMAINS=true
```

| Variable | Default | Notes |
|---|---|---|
| `CORS_ALLOWED_ORIGINS` | empty (dev: `*`) | Comma-separated list |
| `CSRF_PROTECTION` | `none` | `none`, `header`, `double-submit` |
| `TRUST_PROXY` | `false` | Trust `X-Forwarded-*` headers |
| `HSTS_MAX_AGE` | `31536000` | 1 year. `0` disables |
| `HSTS_INCLUDE_SUBDOMAINS` | `true` | Disable only if domain is shared |

### 5.8 OPAQUE

```env
OPAQUE_OPRF_KEY_PATH=/data/oprf.key
```

| Variable | Default | Notes |
|---|---|---|
| `OPAQUE_OPRF_KEY_PATH` | `/data/oprf.key` | Path to the persisted `ServerSetup` |

**Crate:** `opaque-ke` 4.0.1, feature `argon2`. Rust minimum 1.85.

**Cipher suite:** `DefaultCipherSuite` in `server/src/opaque.rs`:

| Component | Value |
|---|---|
| OPRF group | `Ristretto255` |
| Key exchange | `TripleDh<Ristretto255, Sha512>` |
| KSF | Argon2 |

The cipher suite is bound to every stored user registration. Do not change without a spec revision and migration path.

**Server setup persistence:** Serialized to `OPAQUE_OPRF_KEY_PATH` on first startup. Must be `0600` on Unix. Loss invalidates all user registrations.

**OPRF seed rotation:** Long-lived secret. Rotated only on suspected compromise or cryptographic migration, requiring forced re-registration of all users.

The multi-seed mechanism is reserved for V2, contingent on `opaque-ke` exposing multi-seed support.

### 5.9 Backups

```env
BACKUP_ENABLED=true
BACKUP_PATH=/data/backups
BACKUP_INTERVAL_HOURS=24
BACKUP_RETENTION_COUNT=30
BACKUP_INCLUDE_ATTACHMENTS=false
```

| Variable | Default | Notes |
|---|---|---|
| `BACKUP_INCLUDE_ATTACHMENTS` | `false` | Include S3/filesystem blobs |

**Backup encryption key:** Derived from the same root secret as the OPRF seed via HKDF:

```
root_secret = random 32 bytes, stored at OPAQUE_OPRF_KEY_PATH
oprf_seed   = HKDF(root_secret, info="opaque-oprf-v1")
backup_key  = HKDF(root_secret, info="backup-encryption-v1")
```

**Restore procedure:** CLI only. `server restore --from <path> --confirm`.

### 5.10 Push Notifications

```env
PUSH_ENABLED=true
PUSH_VAPID_PUBLIC_KEY=auto
PUSH_VAPID_PRIVATE_KEY=auto
PUSH_APNS_KEY=
PUSH_APNS_KEY_ID=
PUSH_APNS_TEAM_ID=
PUSH_FCM_SERVICE_ACCOUNT_JSON=
PUSH_GATEWAY_URL=
```

**VAPID keys:** Generated on first startup, stored in `instance_config`, included in backups.

**Push suppression:** Per-device. Suppress push for a device with a recent live session.

### 5.11 ALTCHA

```env
ALTCHA_ENABLED=true
ALTCHA_HMAC_SECRET=auto
ALTCHA_ALGORITHM=PBKDF2/SHA-256
ALTCHA_COST=5000
```

**Crate:** `altcha` 0.2.0 or later.

**Effort mode:** Deterministic only. HMAC secret on both creation and verification. Probabilistic mode and deterministic mode without HMAC are prohibited (July 2026 advisory).

**Protection scope:** `POST /auth/register/start` and `POST /auth/register/finish`.

### 5.12 Storage Backend

```env
STORAGE_BACKEND=fs
```

**Values:** `fs` (default) or `s3`.

**Filesystem backend:**

```env
STORAGE_FS_PATH=./data/attachments
```

**S3 backend:**

```env
S3_ENDPOINT=https://s3.amazonaws.com
S3_REGION=us-east-1
S3_BUCKET=my-chat-blobs
S3_ACCESS_KEY_ID=
S3_SECRET_ACCESS_KEY=
S3_PATH_STYLE=false
S3_PRESIGN_TTL_SECONDS=600
```

| Variable | Default | Notes |
|---|---|---|
| `S3_ENDPOINT` | — | Required for S3 backend |
| `S3_REGION` | `us-east-1` | Region string |
| `S3_BUCKET` | — | Required for S3 backend |
| `S3_ACCESS_KEY_ID` | — | Required for S3 backend |
| `S3_SECRET_ACCESS_KEY` | — | Required for S3 backend |
| `S3_PATH_STYLE` | `false` | Set `true` for MinIO and most self-hosted S3-compatible services |
| `S3_PRESIGN_TTL_SECONDS` | `600` | Presigned URL lifetime (10 minutes) |

**S3 crate:** `rust-s3`. Selected over `aws-sdk-s3` for binary size (~445 KB vs ~14 MB) and simpler configuration. Streaming is not required — attachments are capped at 100 MB and buffered in memory during a single fetch.

**Backend selection:** At startup, the server initializes the storage backend based on `STORAGE_BACKEND`. Missing required variables cause startup failure with a clear error.

**Filesystem layout:** Blobs are stored at `{STORAGE_FS_PATH}/{id[0..2]}/{id[2..4]}/{id}` where `id` is a 64-character hex string. Two-level sharding prevents directory bloat.

**S3 key layout:** Blobs are stored at `attachments/{id[0..2]}/{id[2..4]}/{id}`. The same sharding applies.

### 5.13 Attachment Format

```env
ATTACHMENT_CHUNK_SIZE=65536
ATTACHMENT_BUCKET_SIZES=65536,524288,4194304,33554432
```

| Variable | Default | Notes |
|---|---|---|
| `ATTACHMENT_CHUNK_SIZE` | `65536` | Plaintext bytes per AES-GCM chunk (64 KB) |
| `ATTACHMENT_BUCKET_SIZES` | `65536,524288,4194304,33554432` | Padded size buckets for upload |

**Chunk size:** Fixed at 64 KB. Do not change without a spec revision — the value is embedded in every stored attachment's manifest.

**Bucket sizes:** Comma-separated list. Must be strictly increasing. Each value must be a multiple of `ATTACHMENT_CHUNK_SIZE` plus 16 bytes per chunk (the GCM tag overhead).

**Padding:** The client pads the encrypted blob to the nearest bucket. The server verifies the padded size matches a bucket and rejects uploads that do not.

### 5.14 Moderation

```env
MODERATION_MODE=messenger
```

### 5.15 Invite Defaults

```env
INVITE_DEFAULT_USES=1
INVITE_EXPIRY_DAYS=0
INVITE_CODE_LENGTH=8
INVITE_LIMITED_MAX_USES=10
INVITE_LIMITED_MAX_OPEN=50
ROOM_INVITE_DEFAULT_USES=1
ROOM_INVITE_CODE_LENGTH=8
```

### 5.16 Safety Numbers

```env
SAFETY_NUMBER_MODE=warn
```

### 5.17 Audit Log

```env
AUDIT_RETENTION_DAYS=90
```

### 5.18 Cleanup Scheduler

```env
CLEANUP_ENABLED=true
CLEANUP_INTERVAL_MINUTES=60
CLEANUP_STARTUP_DELAY_SECS=30
```

### 5.19 Sockudo

```env
SOCKUDO_URL=http://sockudo:6001
SOCKUDO_APP_ID=chat
SOCKUDO_APP_KEY=auto
SOCKUDO_APP_SECRET=auto
```

External WebSocket URL is derived from `APP_URL`:

```
wss_url = APP_URL.replace("https://", "wss://") + "/realtime"
```

### 5.20 GDPR

```env
DATA_RETENTION_DAYS=0
EXPORT_RATE_LIMIT_HOURS=24
```

### 5.21 TLS Termination

TLS terminates at a reverse proxy. The Axum server listens on plain HTTP internally.

**Coolify deployments:** Traefik handles certificates automatically.

**Manual deployments:** Caddy recommended.

```caddyfile
chat.example.com {
    reverse_proxy /api/* server:8080
    reverse_proxy /health server:8080
    reverse_proxy /ready server:8080
    reverse_proxy /realtime/* sockudo:6001
    reverse_proxy * server:8080
}
```

**What the application enforces:**
1. `APP_URL` must begin with `https://` in production.
2. Requests with `X-Forwarded-Proto: http` are redirected to HTTPS with 301.
3. HSTS is sent on HTTPS responses.

See §8.0 for middleware details.

---

## 6. Client Interface Contract

What the **server requires from clients**. Not an implementation guide.

### 6.1 Authentication

- `Authorization: Bearer <session_token>` on every authenticated request.
- Tokens are 43-character base64url strings.

### 6.2 OPAQUE Handshake

- Registration: `start` then `finish`. Both within 5 minutes.
- Login: `start` then `finish`. Both within 5 minutes.

### 6.3 ALTCHA Payload

- Base64-encoded JSON in the `altcha` field.
- Produced by the ALTCHA widget after solving a challenge.

### 6.4 MLS Message Envelope

- Clients encrypt/decrypt MLS locally. Server never inspects MLS state.
- Message payloads are base64-encoded MLS ciphertexts.
- Padding to fixed buckets (256 B, 1 KB, 4 KB, 16 KB) is client-side.

### 6.5 Attachments — Encryption Format

Attachments use **chunked AES-256-GCM** with the following wire format:

```
Header (23 bytes):
  version        (1 byte, currently 0x01)
  nonce_prefix   (7 bytes, random)
  chunk_size     (4 bytes, big-endian, plaintext bytes per chunk)
  chunk_count    (8 bytes, big-endian, total number of chunks)
  reserved       (3 bytes, zero)

Chunk 0:
  ciphertext_0   (up to chunk_size bytes)
  auth_tag_0     (16 bytes)

Chunk 1:
  ciphertext_1
  auth_tag_1

...
Chunk N-1:
  ciphertext_N-1
  auth_tag_N-1
  (last chunk may be shorter than chunk_size)
```

**IV derivation for chunk `i`:**

```
IV_i = nonce_prefix (7 bytes) || counter_bytes (4 bytes) || 0x00
counter_bytes = base_counter XOR i
```

Where `base_counter` is a random 32-bit value stored in the header. This lets the client derive any chunk's IV independently — a requirement for seeking.

**Additional authenticated data (AAD):**

```
AAD_i = version || chunk_count || i || is_last
```

Where `is_last` is `1` for the final chunk, `0` otherwise. This binds each chunk to its position and prevents truncation attacks.

**Manifest (embedded in the MLS message):**

```json
{
  "file_id": "<sha256 hex of the whole padded ciphertext>",
  "key": "<base64, 32 bytes>",
  "nonce_prefix": "<base64, 7 bytes>",
  "base_counter": 12345,
  "chunk_size": 65536,
  "chunk_count": 1600,
  "plaintext_size": 104857600,
  "encrypted_size": 104883216,
  "content_type": "video/mp4",
  "filename_encrypted": "<base64>"
}
```

The manifest is inside the MLS application message. The server never sees it.

### 6.6 Attachments — Padding

After chunked encryption, the client pads the ciphertext to the nearest bucket from `ATTACHMENT_BUCKET_SIZES`. The padding is added as a final chunk encrypted with the same key and an IV derived for chunk index `chunk_count`. The last chunk flag is set on the padded final chunk.

**Alternative padding:** The client may pad the plaintext before encryption so that the encrypted size naturally lands within a bucket. Either approach works; the server only checks that the total uploaded size equals a bucket value.

### 6.7 Attachments — Upload

- The client uploads the entire padded ciphertext as a single object.
- The `file_id` is the SHA-256 of the **padded ciphertext**, not the plaintext.
- The server verifies the hash before storing.

### 6.8 Attachments — Streaming and Seeking

- The client decides whether to fetch all chunks at once or lazily based on file size and MIME type.
- **Recommended threshold:** files ≤ 1 MB are fetched in a single request; files > 1 MB are fetched lazily.
- **Media (audio/video):** always fetched lazily, regardless of size.
- The client maps plaintext ranges to encrypted ranges and issues `Range` requests.
- The server translates and serves ranges without knowledge of playback position.

### 6.9 Attachments — Presigned URLs

- For S3 backends, the client fetches ranges directly from S3 using presigned URLs.
- The client requests a fresh URL per seek operation. Do not cache presigned URLs.
- Presigned URLs support `Range` headers natively.
- The client must handle URL expiry mid-download by requesting a new URL and resuming.

### 6.10 Attachments — MP4 Streaming

For MP4 files to stream and seek properly:

- The `moov` atom must be at the **beginning** of the file ("fast-start MP4").
- The client must validate this before upload and reject or re-encode files that are not fast-start.
- This is a container-format constraint, not an encryption constraint.

### 6.11 Room Membership

- Room membership is server-visible at the user level.
- MLS leaf-level membership is client-visible only.

### 6.12 Push Subscriptions

- Register via `POST /users/me/push-subscriptions`.
- `platform` must be `web`, `ios`, `android`, or `desktop`.

### 6.13 Safety Numbers

- Clients compute and display safety numbers out-of-band.
- The server never sees safety numbers.

### 6.14 WebSocket Connection

- Connect to the URL advertised in `GET /api/v1/capabilities` (`websocket_url`).
- Production: always `wss://`.

### 6.15 Client Capability Requirements

- Read `GET /api/v1/capabilities` on startup.
- Degrade gracefully if a capability is unavailable.

---

## 7. Data Model (SQLite Schema)

### 7.1 Identity & Auth

```sql
CREATE TABLE users (
    id                  TEXT PRIMARY KEY,
    username            TEXT NOT NULL,
    username_hash       TEXT NOT NULL UNIQUE,
    display_name        TEXT,
    opaque_registration BLOB NOT NULL,
    identity_pubkey     TEXT NOT NULL,
    profile_blob        TEXT,
    profile_version     INTEGER NOT NULL DEFAULT 1,
    max_file_size_bytes INTEGER,
    created_at          DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    disabled_at         DATETIME,
    deleted_at          DATETIME
);

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
    name        TEXT,
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
```

### 7.2 Invites

```sql
CREATE TABLE server_invites (
    id            TEXT PRIMARY KEY,
    code          TEXT NOT NULL UNIQUE,
    created_by    TEXT REFERENCES users(id),
    max_uses      INTEGER NOT NULL,
    current_uses  INTEGER NOT NULL DEFAULT 0,
    expires_at    DATETIME,
    revoked_at    DATETIME,
    created_at    DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE room_invites (
    id            TEXT PRIMARY KEY,
    room_id       TEXT NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    code          TEXT NOT NULL UNIQUE,
    created_by    TEXT NOT NULL REFERENCES users(id),
    max_uses      INTEGER NOT NULL,
    current_uses  INTEGER NOT NULL DEFAULT 0,
    expires_at    DATETIME,
    revoked_at    DATETIME,
    created_at    DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

### 7.3 Rooms

```sql
CREATE TABLE rooms (
    id                    TEXT PRIMARY KEY,
    owner_id              TEXT NOT NULL REFERENCES users(id),
    name_encrypted        TEXT,
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

### 7.4 MLS Lifecycle

```sql
CREATE TABLE key_packages (
    id                TEXT PRIMARY KEY,
    user_id           TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    client_id         TEXT NOT NULL,
    cipher_suite      INTEGER NOT NULL,
    key_package_data  BLOB NOT NULL,
    is_last_resort    INTEGER NOT NULL DEFAULT 0,
    consumed          INTEGER NOT NULL DEFAULT 0,
    consumed_at       DATETIME,
    created_at        DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_kp_available ON key_packages(user_id, consumed)
    WHERE consumed = 0;

CREATE TABLE welcomes (
    id                  TEXT PRIMARY KEY,
    room_id             TEXT NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    recipient_user_id   TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recipient_client_id TEXT NOT NULL,
    welcome_data        BLOB NOT NULL,
    consumed            INTEGER NOT NULL DEFAULT 0,
    created_at          DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_welcome_pending ON welcomes(recipient_user_id, consumed)
    WHERE consumed = 0;

CREATE TABLE pending_mls_removes (
    id               TEXT PRIMARY KEY,
    room_id          TEXT NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    target_user_id   TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_client_id TEXT NOT NULL,
    queued_at        DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    consumed_at      DATETIME
);

CREATE INDEX idx_pending_mls_removes_active
    ON pending_mls_removes(room_id, consumed_at)
    WHERE consumed_at IS NULL;

CREATE TABLE room_messages (
    id                        TEXT PRIMARY KEY,
    room_id                   TEXT NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    sender_user_id            TEXT NOT NULL REFERENCES users(id),
    sender_client_id          TEXT NOT NULL,
    epoch                     INTEGER NOT NULL,
    seq                       INTEGER NOT NULL,
    content_type              TEXT NOT NULL CHECK(content_type IN ('application', 'commit', 'proposal')),
    ciphertext                BLOB NOT NULL,
    created_at                DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_room_messages_room_epoch_seq
    ON room_messages(room_id, epoch, seq);

CREATE INDEX idx_room_messages_room_created
    ON room_messages(room_id, created_at DESC);

CREATE INDEX idx_room_messages_sender
    ON room_messages(sender_user_id, created_at DESC);
```

### 7.5 Attachments

```sql
CREATE TABLE attachments (
    id                TEXT PRIMARY KEY,
    room_id           TEXT NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    uploader_id       TEXT NOT NULL REFERENCES users(id),
    uploader_client_id TEXT,
    storage_backend   TEXT NOT NULL CHECK(storage_backend IN ('fs', 's3')),
    storage_key       TEXT NOT NULL,
    padded_size       INTEGER NOT NULL,
    plaintext_size    INTEGER NOT NULL,
    encrypted_size    INTEGER NOT NULL,
    chunk_size        INTEGER NOT NULL,
    chunk_count       INTEGER NOT NULL,
    nonce_prefix      TEXT NOT NULL,
    base_counter      INTEGER NOT NULL,
    content_type      TEXT NOT NULL DEFAULT 'application/octet-stream',
    created_at        DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_attachments_room_created
    ON attachments(room_id, created_at DESC);

CREATE INDEX idx_attachments_uploader_created
    ON attachments(uploader_id, created_at DESC);

CREATE INDEX idx_attachments_created
    ON attachments(created_at);
```

**Column notes:**

- `id` — SHA-256 hex of the padded ciphertext.
- `storage_backend` — `'fs'` or `'s3'`. Set at upload time based on the active backend. Enables future migrations.
- `storage_key` — path or S3 key. Derived from `id` but stored explicitly for backend flexibility.
- `padded_size` — total bytes uploaded (a bucket value).
- `plaintext_size` — original file size, before encryption.
- `encrypted_size` — `padded_size`, but semantically distinct for clarity.
- `chunk_size` — plaintext bytes per chunk (64 KB default).
- `chunk_count` — number of chunks including padding.
- `nonce_prefix` — base64 of the 7-byte prefix.
- `base_counter` — 32-bit integer used for IV derivation.
- `content_type` — client-supplied MIME type. Metadata only.

### 7.6 Push Subscriptions

```sql
CREATE TABLE push_subscriptions (
    id           TEXT PRIMARY KEY,
    user_id      TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id    TEXT REFERENCES devices(id) ON DELETE CASCADE,
    platform     TEXT NOT NULL CHECK(platform IN ('web', 'ios', 'android', 'desktop')),
    endpoint     TEXT,
    p256dh       TEXT,
    auth         TEXT,
    push_token   TEXT,
    browser_id   TEXT,
    user_agent   TEXT,
    created_at   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_used_at DATETIME,
    revoked_at   DATETIME
);

CREATE UNIQUE INDEX idx_push_endpoint ON push_subscriptions(endpoint)
    WHERE endpoint IS NOT NULL;

CREATE UNIQUE INDEX idx_push_browser ON push_subscriptions(user_id, browser_id)
    WHERE browser_id IS NOT NULL;
```

### 7.7 Instance Config & Ops

```sql
CREATE TABLE instance_limits (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_by TEXT REFERENCES users(id)
);

CREATE TABLE instance_config (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_by TEXT REFERENCES users(id)
);

CREATE TABLE audit_log (
    id          TEXT PRIMARY KEY,
    actor_id    TEXT REFERENCES users(id),
    action      TEXT NOT NULL,
    target_type TEXT,
    target_id   TEXT,
    metadata    TEXT,
    created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE backups (
    id          TEXT PRIMARY KEY,
    path        TEXT NOT NULL,
    size_bytes  INTEGER NOT NULL,
    checksum    TEXT NOT NULL,
    created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at  DATETIME
);

CREATE TABLE rate_limits (
    key          TEXT PRIMARY KEY,
    window_start DATETIME NOT NULL,
    count        INTEGER NOT NULL DEFAULT 0
);
```

`instance_config` stores VAPID keys, Sockudo credentials, ALTCHA HMAC secret, and other auto-generated per-instance secrets.

### 7.8 State Outside the Database

| Path | Purpose | Notes |
|---|---|---|
| `OPAQUE_OPRF_KEY_PATH` | OPAQUE `ServerSetup` and root backup secret | Required. Loss invalidates all user registrations and backups. |
| `BACKUP_PATH` | Backup snapshots | Populated by the backup scheduler |
| `STORAGE_FS_PATH` | Filesystem attachment blobs | Only when `STORAGE_BACKEND=fs` |
| S3 bucket | S3 attachment blobs | Only when `STORAGE_BACKEND=s3` |

The OPAQUE `ServerSetup` file must have `0600` permissions on Unix.

---

## 8. API Surface

All routes prefixed with `/api/v1/`. Auth via `Authorization: Bearer <session_token>` unless noted.

**Error format:**

```json
{
  "error": "machine_readable_code",
  "message": "Human-readable message",
  "details": {}
}
```

### 8.0 Static Client Hosting and HTTPS Enforcement

**Static SPA hosting:** When `CLIENT_STATIC_DIR` is set, Axum serves the SPA with `ServeDir` and a fallback to `index.html`. The `fallback_service` runs last.

**HTTPS enforcement:** In production, requests with `X-Forwarded-Proto: http` receive a 301 redirect. HSTS is sent on HTTPS responses. `/health` and `/ready` are exempt from redirect.

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
| POST | `/invites/redeem` | Validate and consume server invite |
| GET | `/invites/:code` | Public invite validation |

**`GET /capabilities` response:**

```json
{
  "version": "0.1.0",
  "calling": false,
  "push_vapid_public_key": null,
  "websocket_url": "wss://chat.example.com/realtime",
  "sockudo_app_key": "abc123",
  "sockudo_channel_prefix": "private-room-",
  "altcha": {
    "enabled": true,
    "algorithm": "PBKDF2/SHA-256",
    "cost": 5000
  },
  "storage_backend": "s3",
  "attachment_chunk_size": 65536,
  "attachment_bucket_sizes": [65536, 524288, 4194304, 33554432],
  "safety_number_mode": "warn",
  "moderation_mode": "messenger"
}
```

### 8.2 User

| Method | Path | Purpose |
|---|---|---|
| POST | `/auth/logout` | Revoke session |
| GET | `/users/me` | Current user |
| PATCH | `/users/me` | Update profile |
| DELETE | `/users/me` | Delete account |
| GET | `/users/me/export` | Export data |
| POST | `/users/lookup` | Username lookup |
| GET | `/users/me/devices` | List devices |
| DELETE | `/users/me/devices/:id` | Revoke device |
| GET | `/users/me/sessions` | List sessions |
| DELETE | `/users/me/sessions/:id` | Revoke session |
| POST | `/users/me/push-subscriptions` | Register push subscription |
| DELETE | `/users/me/push-subscriptions/:id` | Revoke push subscription |

### 8.3 Admin

| Method | Path | Purpose |
|---|---|---|
| GET | `/admin/users` | List users |
| POST | `/admin/users/:id/roles` | Grant role |
| DELETE | `/admin/users/:id/roles/:role` | Revoke role |
| POST | `/admin/users/:id/disable` | Disable account |
| POST | `/admin/users/:id/enable` | Enable account |
| GET | `/admin/config` | Read instance config |
| PATCH | `/admin/config` | Update config |
| GET | `/admin/limits` | Read instance limits |
| PATCH | `/admin/limits` | Update limits |
| POST | `/admin/invites` | Create server invite |
| GET | `/admin/invites` | List server invites |
| DELETE | `/admin/invites/:id` | Revoke server invite |
| GET | `/admin/audit` | Read audit log |
| GET | `/admin/backups` | List backups |
| POST | `/admin/backups` | Trigger manual backup |
| POST | `/admin/vapid/rotate` | Rotate VAPID keys |
| POST | `/admin/altcha/rotate` | Rotate ALTCHA HMAC secret |

### 8.4 Rooms

| Method | Path | Purpose |
|---|---|---|
| POST | `/rooms` | Create room |
| GET | `/rooms` | List my rooms |
| GET | `/rooms/:id` | Room metadata |
| DELETE | `/rooms/:id` | Delete room |
| POST | `/rooms/:id/leave` | Leave room |
| POST | `/rooms/:id/transfer` | Transfer ownership |
| GET | `/rooms/:id/members` | List members |
| POST | `/rooms/:id/members` | Add member |
| DELETE | `/rooms/:id/members/:uid` | Kick |
| POST | `/rooms/:id/members/:uid/promote` | Promote to moderator |
| POST | `/rooms/:id/members/:uid/demote` | Demote |
| POST | `/rooms/:id/invites` | Create room invite |
| GET | `/rooms/:id/invites` | List room invites |
| DELETE | `/rooms/:id/invites/:code` | Revoke room invite |
| POST | `/rooms/join` | Join via invite code |

### 8.5 MLS & Messaging

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
| GET | `/rooms/:id/epoch` | Current epoch and sequence |
| GET | `/rooms/:id/pending-removes` | List pending MLS removes |
| POST | `/rooms/:id/pending-removes/:id/consume` | Mark remove consumed |
| POST | `/sockudo/auth` | Sign a channel subscription |

### 8.6 Attachments

| Method | Path | Purpose |
|---|---|---|
| POST | `/rooms/:id/attachments` | Upload encrypted attachment |
| GET | `/attachments/:id` | Download (supports `Range`) |
| DELETE | `/attachments/:id` | Delete (uploader only) |
| POST | `/attachments/:id/presign` | Generate a presigned URL (S3 only) |

**Upload request:** `multipart/form-data` with a `file` part and a `claimed_id` field; or `application/octet-stream` with `X-Claimed-Id` header.

**Upload response:**

```json
{
  "id": "<sha256 hex>",
  "room_id": "<room_id>",
  "uploader_id": "<user_id>",
  "padded_size": 65536,
  "plaintext_size": 104857600,
  "encrypted_size": 104883216,
  "chunk_size": 65536,
  "chunk_count": 1600,
  "content_type": "video/mp4",
  "created_at": "<ISO 8601>"
}
```

**Download:** Standard `GET` returns the whole blob. With `Range: bytes=N-M`, returns `206 Partial Content` with the translated encrypted range. The client decrypts the chunks in that range. Headers:

```
Content-Type: application/octet-stream
Content-Length: <range length>
Content-Range: bytes N-M/total
Cache-Control: private, max-age=86400, immutable
ETag: "<id>"
X-Attachment-Content-Type: <content_type>
X-Attachment-Chunk-Size: 65536
X-Attachment-Plaintext-Size: 104857600
X-Attachment-Nonce-Prefix: <base64>
X-Attachment-Base-Counter: <int>
X-Content-Type-Options: nosniff
```

**Presign request:**

```json
{
  "range_start": 0,
  "range_end": 65535,
  "expires_in_seconds": 600
}
```

**Presign response:**

```json
{
  "url": "https://s3.example.com/...",
  "expires_at": "<ISO 8601>",
  "range_start": 0,
  "range_end": 65535
}
```

The presigned URL supports range requests. The client fetches directly from S3, bypassing the Axum server for blob traffic.

**Presign is only available when `STORAGE_BACKEND=s3`.** For filesystem, the client uses `GET /attachments/:id` with a `Range` header.

**Rate limit:** `RATE_PRESIGN_PER_MIN` per user.

---

## 9. CLI Subcommands

| Command | Purpose |
|---|---|
| `server` | Run the HTTP server (default) |
| `server restore --from <path> --confirm` | Restore from backup |
| `server migrate` | Run pending migrations and exit |
| `server rotate-vapid` | Rotate VAPID keys |
| `server rotate-altcha` | Rotate ALTCHA HMAC secret |
| `server rotate-oprf --confirm` | Resample the OPRF seed |
| `server storage migrate --from fs --to s3` | Migrate attachment blobs between backends |

---

## 10. Client Contract Matrix

| Feature | Server behaviour |
|---|---|
| Text messaging | Relays MLS ciphertext via Sockudo; never inspects content |
| Attachments | Content-addressed, chunked AES-GCM, supports range requests |
| Attachment streaming | Server supports `Range` headers on download; presigned URLs for S3 |
| Push notifications | Accepts `web`, `ios`, `android`, `desktop`; suppresses per-device |
| Multi-device | Up to `devices_per_user` (default 10) |
| Safety numbers | Advisory only; server never sees them |
| ALTCHA | Challenge endpoint on registration only |
| Calling (V2) | `calling: false` in `/capabilities` |
| Data export | ZIP of ciphertext and account metadata |
| Account deletion | Anonymises user record; cascades session, device, KeyPackage |
| WebSocket | `wss://` in production, `ws://` in development |
| HTTPS | Enforced by reverse proxy; server redirects and sends HSTS |
| Storage backend | Advertised as `storage_backend` in `/capabilities` |

---

## 11. Phased Build Plan

| Phase | Deliverable | Depends On |
|---|---|---|
| 1 | Axum scaffold + SQLite migrations + health + capabilities | — |
| 2 | Roles, permissions, user_roles tables + seed data | 1 |
| 3a | OPAQUE registration flow | 2 |
| 3a.1 | ALTCHA proof-of-work protection | 3a |
| 3b | OPAQUE login flow | 3a.1 |
| 4a | Auth middleware + session management | 3b |
| 4b | Device registration + revocation cascade | 4a |
| 5 | Server invite codes | 3a.1 |
| 6a | Instance config, limits, audit log, readiness | 4a |
| 6b | Cleanup scheduler + jobs | 6a |
| 6c | Account deletion + data export (GDPR) | 6a |
| 6d | HTTPS enforcement + static SPA hosting | 6a |
| 7a | Room lifecycle and membership | 5 |
| 7b | Room moderation | 7a |
| 8 | Room invite codes | 7a |
| 9 | KeyPackage upload, claim, quota enforcement | 7a |
| 10 | Welcome routing + MLS epoch linearization | 9 |
| 11 | Sockudo integration (publish, auth, realtime sync) | 10 |
| 12a | Attachment upload, download, delete (chunked format) | 11 |
| 12b | Range requests + presigned URLs (streaming) | 12a |
| 12c | Retention pruning + cleanup job | 12b |
| 13 | Admin web UI (Coralite) | 6a |
| 14 | User web UI (Coralite) | 12b |
| 15 | Push notifications (Web Push first) | 14 |
| 16 | APNs and FCM support | 15 |
| 17 | Automatic backups + restore CLI | 6a |
| 18 | Tauri desktop client | 14 |
| 19 | Capacitor iOS client | 14 |
| 20 | Capacitor Android client | 14 |

---

## 12. Security Boundaries

| Boundary | Guarantee |
|---|---|
| Server sees plaintext messages | Never |
| Server sees decryption keys | Never |
| Server sees message metadata | Minimal: room ID, sender client ID, epoch, coarsened timestamp |
| Server sees profile contents | Never |
| Server sees display name | Yes (for push) |
| Server sees username | Yes (for lookup) |
| Server sees attachment contents | Never |
| Server sees attachment plaintext size | Yes (metadata for range translation) |
| Server can decrypt past messages | No (forward secrecy) |
| Server can decrypt future messages | No (post-compromise security) |
| Web client protected against compromised server | No. Host-delivered code. |
| Native client protected against compromised server | Yes. Signed binaries. |
| Registration endpoints bot-resistant | Yes. ALTCHA. |
| Server can force peers to delete local copies | No. Tombstones are advisory. |
| Production traffic is HTTPS | Yes. Enforced by reverse proxy. |
| Attachments are streamed via range requests | Yes, with chunked AES-GCM. |
| Attachments are vulnerable to truncation attacks | Mitigated by per-chunk AAD with chunk_count and is_last. |

---

## 13. Backups and Disaster Recovery

- Automatic snapshots of SQLite and the OPRF key every `BACKUP_INTERVAL_HOURS`.
- Attachment blobs are included only if `BACKUP_INCLUDE_ATTACHMENTS=true`.
- Snapshots encrypted with `backup_key` derived from the root secret.
- Restore is CLI-only.
- Attachment blobs on S3 are not backed up by the server — rely on S3's own durability and versioning.

---

## 14. GDPR Compliance

### 14.1 Data Subject Rights

| Right | Article | Mechanism |
|---|---|---|
| Access | Art. 15 | `GET /users/me/export` |
| Rectification | Art. 16 | `PATCH /users/me` |
| Erasure | Art. 17 | `DELETE /users/me` |
| Restriction | Art. 18 | Account disable |
| Portability | Art. 20 | ZIP export |
| Object | Art. 21 | Account deletion |

### 14.2 Account Deletion

Anonymises the user row, deletes devices, sessions, KeyPackages, push subscriptions, room memberships, and queues MLS removes. The `users.id` is preserved for referential integrity.

### 14.3 Data Export

ZIP archive with `profile.json`, `devices.json`, `sessions.json`, `rooms.json`, `messages.json`, `attachments.json`, `audit.json`, `README.txt`. `attachments.json` lists metadata and manifest fields but not the blob (which the client can fetch separately).

### 14.4 Retention

`DATA_RETENTION_DAYS` controls message and attachment purge. `AUDIT_RETENTION_DAYS` controls the audit log.

### 14.5 Tombstone Semantics

Deleted messages are tombstoned. The server deletes its copy. Peers delete local copies at their discretion. The server cannot force peer deletion.

### 14.6 Metadata Minimisation

Timestamps coarsened where practical. No IP addresses in the database. Audit metadata excludes content.

### 14.7 Controller Obligations

Controller publishes privacy policy, provides DPA if needed, documents server location and sub-processors (Sockudo, S3).

### 14.8 Audit Actions

`user.delete`, `user.export`, `config.update`, `secret.update`, `limits.update`, `role.grant`, `role.revoke`, `invite.create`, `invite.revoke`, `device.revoke`, `bootstrap.owner`, `vapid.rotate`, `altcha.rotate`, `storage.migrate`.

---

## 15. Document Status

This is the contract for the server side of the system. Every Jules task references this document. If a task conflicts with this spec, the task is wrong and must be revised. If a feature is missing, it does not exist yet — it must be added here first, then built.

The spec is frozen for V1. New features go into V2. Bug fixes and clarifications are amendments.
