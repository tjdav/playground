# Server Specification v1.0

> **Status:** Stable — source of truth for all Jules tasks.
> **Scope:** This document describes the **server-side contract only**. Client implementation is out of scope and is covered by a separate client specification.
> **Stack:** Sockudo + Axum + SQLite + OPAQUE + ALTCHA + S3 (or filesystem)
> **Deployment:** Single VPS, Docker-based, self-hosted. Two containers: `server` (Axum + static SPA hosting) and `sockudo`. TLS terminates at a reverse proxy (Coolify/Traefik or Caddy).

Amendments are tracked at §16. Any change to this document requires a documented amendment and a corresponding task update.

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
| Bot protection | ALTCHA Proof-of-Work v2 via `altcha` 0.2.0 |
| Attachment encryption | C2SP chunked encryption (`c2sp.org/chunked-encryption`) |
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
- Message deletion (tombstone via server endpoint; no editing)
- Encrypted attachments using C2SP chunked encryption
- Attachment range requests for streaming and seeking
- Presigned URLs for direct S3 fetch (bypassing the API server)
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
- User-scoped channel events (`private-user-{user_id}`)
- Presence channels (`presence-room-{room_id}`)

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
- MP4 fast-start enforcement (client responsibility)
- Multi-range HTTP requests
- Range-restricted presigned URLs
- Real-time presence (online/offline indicators). The `sessions.last_seen_at` column is for session management, not user-facing presence. V1 clients do not display presence indicators.

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
| `owner` | Kick members, delete room, promote/demote moderators (Discord mode only), transfer ownership, delete any message |
| `moderator` | Kick members (Discord mode only), delete any message (Discord mode only) |
| `member` | Send messages, upload attachments, leave room, delete own messages |

### 3.3 Moderation Modes

- **Messenger mode (default):** Only room owner can kick. Moderators cannot exist. Only the sender can delete their own messages.
- **Discord mode (optional):** Room owner can promote members to moderators. Moderators can kick and delete any message.

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

### 4.4 Effective Limits Exposure

`GET /rooms/:id` returns the resolved effective limits:

```json
{
  "effective_max_file_size_bytes": 104857600,
  "effective_message_retention_days": 0
}
```

Clients use these to pre-flight attachment uploads and avoid the 413 error path.

### 4.5 Cleanup Jobs

All cleanup jobs run on a shared hourly scheduler.

| Job | Retention | Notes |
|---|---|---|
| Session cleanup | 30 days after expiry or revocation | Preserves active and recently-expired sessions |
| Rate limit table | 24 hours | Longest rate window is daily |
| Audit log | `AUDIT_RETENTION_DAYS` (default 90) | Compliance-aligned default |
| Attachment pruning | Three-tier effective retention | Deletes row and blob |
| Welcome expiry | 7 days | Stale welcomes deleted |
| Message retention | Per-room, falling to instance default | Deletes messages from DB regardless of `deleted_at` |
| Registration state | 5-minute TTL | In-memory, opportunistic purge |
| Login state | 5-minute TTL | In-memory, opportunistic purge |

**Message retention and deletion interplay:** A message with `deleted_at` set is pruned by the retention job on the same schedule as an undeleted message. Deletion does not accelerate pruning.

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
| `RATE_INVITE_CREATE_HOURLY` | `50` | Invites created per user per hour |
| `RATE_INVITE_CREATE_DAILY` | `200` | Invites created per user per day |
| `RATE_INVITE_REDEEM_PER_MIN` | `10` | Redemption attempts per IP per minute |
| `RATE_KP_CLAIM_PER_MIN` | `30` | KeyPackage claims per user per minute |
| `RATE_KP_CLAIM_HOURLY` | `200` | KeyPackage claims per user per hour |
| `RATE_LOGIN_PER_MIN` | `10` | Login attempts per IP per minute |
| `RATE_LOGIN_LOCKOUT_MIN` | `15` | Lockout duration after threshold |
| `RATE_PRESIGN_PER_MIN` | `60` | Presigned URL generation per user per minute |

**Rate limit keys:**

| Variant | Key format | Window(s) |
|---|---|---|
| `InviteCreate` | `invite_create:{user_id}:{hour\|day}:{boundary}` | Hourly, Daily |
| `InviteRedeem` | `invite_redeem:{ip}:min:{boundary}` | Per minute |
| `KpClaim` | `kp_claim:{user_id}:{minute\|hour}:{boundary}` | Per minute, Hourly |
| `Login` | `login:{ip}:min:{boundary}` | Per minute |
| `DataExport` | `data_export:{user_id}:{boundary}` | `EXPORT_RATE_LIMIT_HOURS` |
| `Presign` | `presign:{user_id}:min:{boundary}` | Per minute |

Rate limit state is stored in a SQLite table and pruned hourly (entries older than 24 hours).

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

The multi-seed mechanism is reserved for V2.

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
| `BACKUP_INCLUDE_ATTACHMENTS` | `false` | Include filesystem blobs. Ignored when `STORAGE_BACKEND=s3`. |

**Backup encryption key:** Derived from the same root secret as the OPRF seed via HKDF:

```
root_secret = random 32 bytes, stored at OPAQUE_OPRF_KEY_PATH
oprf_seed   = HKDF(root_secret, info="opaque-oprf-v1")
backup_key  = HKDF(root_secret, info="backup-encryption-v1")
```

**Restore procedure:** CLI only. `server restore --from <path> --confirm`.

**S3 backend note:** Attachment blobs on S3 are not backed up by the server. Rely on S3's durability and versioning. If `STORAGE_BACKEND=s3` and `BACKUP_INCLUDE_ATTACHMENTS=true`, the server fails startup with a clear error — S3 blobs cannot be snapshotted by this mechanism.

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

**Push suppression:** Per-device. Suppress push for a device with an active WebSocket connection in the last 30 seconds, confirmed via session heartbeat.

**Push payload schema:** All push notifications use the following JSON envelope. The `encrypted_payload` field contains an MLS-encrypted message body that the client decrypts locally. The server never sees plaintext.

```json
{
  "type": "message" | "call" | "member_change" | "welcome",
  "room_id": "<room_id>",
  "sender_user_id": "<user_id>",
  "encrypted_payload": "<base64 MLS ciphertext>",
  "notification_id": "<uuid>",
  "priority": "high" | "normal",
  "collapse_key": "<room_id>",
  "timestamp": "<ISO 8601>"
}
```

**Platform mapping:**

| Platform | Envelope |
|---|---|
| Web Push | Payload is the JSON, sent as-is with `Content-Encoding: aes128gcm` |
| APNs | `aps.alert.title` = sender display name, `aps.alert.body` = "New message", custom keys carry the JSON |
| FCM | `notification.title` and `notification.body` set, `data` field carries the JSON |

**Metadata note:** The `sender_user_id` is plaintext in the push payload. This is necessary for the client to look up the sender's display name. It leaks who is messaging whom. Documented in §14.6.

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
| `S3_PRESIGN_TTL_SECONDS` | `600` | Default presigned URL lifetime (10 minutes) |

**S3 crate:** `rust-s3`. Selected over `aws-sdk-s3` for binary size (~445 KB vs ~14 MB) and simpler configuration.

**Backend selection:** At startup, the server initializes the storage backend based on `STORAGE_BACKEND`. Missing required variables cause startup failure with a clear error.

**Filesystem layout:** Blobs are stored at `{STORAGE_FS_PATH}/attachments/{id[0..2]}/{id[2..4]}/{id}` where `id` is a 64-character hex string. Two-level sharding prevents directory bloat.

**S3 key layout:** Blobs are stored at `attachments/{id[0..2]}/{id[2..4]}/{id}`. The same sharding applies.

**Key validation:** Storage keys are validated against `^attachments/[0-9a-f]{2}/[0-9a-f]{2}/[0-9a-f]{64}$` before any operation. Malformed keys are rejected with `StorageError::InvalidKey`.

**Presign support:**

| Backend | Presign | Notes |
|---|---|---|
| `fs` | Not supported | Presign endpoint returns `501 presign_not_supported`. Clients use proxied `GET` with `Range`. |
| `s3` | Supported | Presign endpoint returns a signed URL. Clients fetch ranges directly from S3. |

**Presign TTL clamping:**

- **Minimum:** 30 seconds
- **Maximum:** `2 × S3_PRESIGN_TTL_SECONDS`
- **Default:** `S3_PRESIGN_TTL_SECONDS`

**Presigned URL scope:** S3 presigned URLs grant access to the **entire object**, not a specific byte range. The client sends a `Range` header when fetching from S3 to retrieve only the bytes it needs.

### 5.13 Attachment Format

```env
ATTACHMENT_CHUNK_SIZE=16384
ATTACHMENT_BUCKET_SIZES=65536,524288,4194304,33554432,268435456
```

| Variable | Default | Notes |
|---|---|---|
| `ATTACHMENT_CHUNK_SIZE` | `16384` | Fixed by the C2SP specification. **Not configurable.** |
| `ATTACHMENT_BUCKET_SIZES` | `65536,524288,4194304,33554432,268435456` | Padded size buckets for upload |

**Chunk size is a protocol constant.** The C2SP chunked encryption specification fixes the chunk size at 16 KiB. It is not an application-selectable parameter. Do not change this value.

**Bucket sizes:** Comma-separated list. Must be strictly increasing. Each value must be a multiple of `ATTACHMENT_CHUNK_SIZE`. The largest bucket MUST be sufficient to contain `SERVER_MAX_FILE_SIZE_BYTES` plus worst-case padding overhead. If `SERVER_MAX_FILE_SIZE_BYTES` is raised above 255 MiB, an additional bucket MUST be appended.

**Bucket capacity:** The maximum paddable plaintext length per bucket is `L_max(T) = 16384 × floor((T − 72) / 16400) + 16383`, provided `(T − 72) mod 16400 < 16384`. If the modulo falls in `[16384, 16399]`, the bucket cannot be reached exactly and the client must advance to the next bucket.

| Target bucket `T` | `L_max(T)` | Human-readable |
|---|---|---|
| 65,536 | 65,416 | ~63.9 KiB |
| 524,288 | 523,720 | ~511.4 KiB |
| 4,194,304 | 4,190,152 | ~4.0 MiB |
| 33,554,432 | 33,521,640 | ~32.0 MiB |
| 268,435,456 | 268,173,496 | ~255.8 MiB |

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
SOCKUDO_ENABLE_CLIENT_EVENTS=true
```

External WebSocket URL is derived from `APP_URL`:

```
wss_url = APP_URL.replace("https://", "wss://") + "/realtime"
```

**Channel taxonomy:**

| Channel | Type | Purpose |
|---|---|---|
| `private-room-{room_id}` | Private | Room events, client events |
| `private-user-{user_id}` | Private | User-scoped events — V2 |

No presence channels in V1. No public channels.

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

Attachments use **C2SP chunked encryption** instantiated with AES-256-GCM and SHA-256, with the protocol-mandated chunk size of 16 KiB. The reference specification is `https://c2sp.org/chunked-encryption`.

#### Header (56 bytes)

```
salt         (24 bytes, random per file)
commitment   (32 bytes, HKDF-derived)
```

The header is prepended to the ciphertext. It is not encrypted.

#### Key and nonce derivation

Given an input key `K` (32 random bytes, generated per file), the salt, the AEAD name, and the context:

```
info = "c2sp.org/chunked-encryption@v1+"
    || "AEAD_AES_256_GCM"
    || 0x00
    || salt
    || context

key_material = HKDF-Expand(
    prk    = K,
    info   = info,
    length = 32 + 12 + 32
)

file_key   = key_material[0..32]   (32 bytes, AES-256 key)
base_nonce = key_material[32..44]  (12 bytes)
commitment = key_material[44..76]  (32 bytes)
```

**Info field layout notes (verified against the C2SP specification, 2026-09-28):**

- `"c2sp.org/chunked-encryption@v1+"` is a 31-byte ASCII string. The `+` is literal. There is **no** `0x00` separator between the `+` and the AEAD name.
- The AEAD name follows immediately. For AES-256-GCM, it is the 17-byte string `"AEAD_AES_256_GCM"`.
- A single `0x00` byte separates the AEAD name from the salt.
- The salt is 24 raw bytes. It is not encoded.
- The context follows the salt directly. There is no `0x00` separator between the salt and the context.

#### Context

```
context = "attachment" || 0x00 || room_id
```

| Offset | Length | Content |
|---|---|---|
| 0 | 10 | `"attachment"` (ASCII bytes) |
| 10 | 1 | `0x00` (domain separator) |
| 11 | variable | `room_id` (UTF-8) |

The context binds the encryption to a specific room. A file cannot be copied between rooms without re-encryption. This contains the blast radius of a client-side key reuse bug.

#### Chunks

Plaintext is split into chunks of `chunk_size` (16384 bytes). The final chunk may be shorter than `chunk_size`; it may also be empty if the plaintext size is an exact multiple of `chunk_size`.

For chunk index `i` (0-based):

```
nonce_i = base_nonce XOR i    (i encoded big-endian as a 12-byte integer)
AAD     = empty
tag     = 16 bytes
```

Ciphertext layout:

```
header (56 bytes)
ciphertext_0 || tag_0
ciphertext_1 || tag_1
...
ciphertext_{N-1} || tag_{N-1}
```

#### Truncation resistance

A reader MUST treat a ciphertext whose final chunk is exactly `chunk_size` bytes as truncated. A well-formed ciphertext ends with a short chunk, or with an empty final chunk when the plaintext is an exact multiple of `chunk_size`.

#### Random access

To decrypt plaintext byte range `[p_start, p_end]`:

```
start_chunk = p_start / 16384
end_chunk   = p_end   / 16384

encrypted_start = 56 + start_chunk * (16384 + 16)
encrypted_end   = 56 + (end_chunk + 1) * (16384 + 16) - 1
```

Request `Range: bytes=<encrypted_start>-<encrypted_end>` from the server (or from S3 via a presigned URL). Decrypt each chunk independently using the derived nonce.

#### Key commitment

Before decrypting any chunk, the client MUST verify that the commitment derived from the input key, salt, and context matches the commitment in the header. A mismatch means the ciphertext was produced under a different key or context and MUST be rejected.

#### Manifest

The manifest travels inside the MLS application message. The server never sees it.

```json
{
  "file_id": "<sha256 hex of the padded ciphertext>",
  "key": "<base64, 32 bytes>",
  "salt": "<base64, 24 bytes>",
  "chunk_size": 16384,
  "plaintext_size": 104857600,
  "padded_size": 268435456,
  "content_type": "video/mp4",
  "filename_encrypted": "<base64>"
}
```

#### Padding algorithm

Attachments are padded to a fixed bucket size from `ATTACHMENT_BUCKET_SIZES`. Padding is performed on the **plaintext** before encryption. The padded plaintext is then chunked as a single C2SP message, producing a ciphertext with exactly one short final chunk.

**Protocol constants:**

- `chunk_size = 16384` (16 KiB, C2SP protocol constant)
- Per-chunk overhead: 16 bytes (AEAD tag)
- Header overhead: 56 bytes (24-byte salt + 32-byte commitment)
- Total C2SP overhead: `72 + 16 * num_chunks`

**Algorithm:**

Given a plaintext of length `P` and a target bucket `T`:

1. Compute the target padded plaintext length:

   ```
   M = T - 72
   q = floor(M / 16400)
   r = M - 16400 * q

   if r < 16384:
     L_padded = 16384 * q + r
   else:
     # T cannot be reached with valid C2SP chunking.
     # Skip to the next larger bucket.
     return BucketUnreachable
   ```

2. If `L_padded < P`, the bucket is too small. The client must select a larger bucket.

3. Build the padded plaintext:

   ```
   padding_zeros = L_padded - P
   padded = plaintext || zeros(padding_zeros)
   ```

4. Chunk `padded` as a single C2SP message:

   - `q` chunks of exactly `chunk_size` bytes
   - 1 final chunk of `r` bytes (may be empty when `r == 0`)

5. Encrypt each chunk with its derived nonce.

**Invariants (verified by the client before upload):**

```
assert L_padded >= P
assert 56 + L_padded + 16 * (q + 1) == T
assert r < 16384
```

**Worked example (100,000-byte plaintext, 512 KiB bucket):**

| Step | Value |
|---|---|
| `P` | 100,000 |
| `T` | 524,288 |
| `M` | 524,216 |
| `q` | 31 |
| `r` | 15,816 |
| `L_padded` | 523,720 |
| `padding_zeros` | 423,720 |
| Chunk sequence | 31 × 16,384 + 1 × 15,816 |
| Final ciphertext size | 56 + 523,720 + 16 × 32 = 524,288 ✓ |

**Exact-fit edge case:** When `(T - 72) mod 16400 ∈ [16384, 16399]`, no valid `L_padded` exists that reaches `T`. The client skips to the next larger bucket. With the default bucket list, no bucket triggers this case.

**Maximum paddable plaintext per bucket:**

| Target bucket `T` | `L_max(T)` | Human-readable |
|---|---|---|
| 65,536 | 65,416 | ~63.9 KiB |
| 524,288 | 523,720 | ~511.4 KiB |
| 4,194,304 | 4,190,152 | ~4.0 MiB |
| 33,554,432 | 33,521,640 | ~32.0 MiB |
| 268,435,456 | 268,173,496 | ~255.8 MiB |

A plaintext larger than `L_max(T)` cannot be padded to `T` and must advance to the next bucket. If no bucket accommodates the plaintext, the client rejects the upload with `BucketTooSmallError`.

**Why plaintext-level padding:** C2SP requires all non-final chunks to be exactly `chunk_size` bytes. Appending padding chunks after encryption places short chunks at non-final positions, producing invalid ciphertexts that conformant decryptors reject. Padding at the plaintext level produces a single well-formed C2SP message.

#### Storage

The `file_id` is the SHA-256 of the padded ciphertext. The server verifies the hash before storing.

#### Canonical test vectors

The following values are the output of the Rust prototype at `verification/c2sp-rust/prototype/`, cross-verified against the Go reference implementation (`filippo.io/cobblestone`). They are the canonical values for validating any implementation.

**Vector 1 — Base derivation**

```
input_key = 0x0101010101010101010101010101010101010101010101010101010101010101
salt      = 0x020202020202020202020202020202020202020202020202
aead      = "AEAD_AES_256_GCM"
room_id   = "test-room-abc123"
context   = "attachment" || 0x00 || room_id

Outputs:
  file_key   = 0x590a4fd2874a62f11c7ed624ce716f6d8eb89d6c0353b310c8180dece1bff988
  base_nonce = 0x98c30971a63703eaed863e41
  commitment = 0x4def3b6cafc9716c7a8e1311b3d10874248b4526ec59958313ef9a3c088f8524
```

**Vector 2 — Cross-room separation**

```
input_key = 0x0101010101010101010101010101010101010101010101010101010101010101
salt      = 0x020202020202020202020202020202020202020202020202
room_id   = "test-room-xyz789"
```

All derived values must differ from Vector 1.

**Vector 3 — Single chunk encryption**

Using Vector 1's `file_key` and `base_nonce`:

```
plaintext  = "The quick brown fox jumps over the lazy dog"
chunk_size = 16384
```

Ciphertext (115 bytes):

```
0x020202020202020202020202020202020202020202020202
4def3b6cafc9716c7a8e1311b3d10874248b4526ec59958313ef9a3c088f8524
50188ad22fb364d891d3715562b4b728ff40def7ac6365c0855bc52a7381a1fb
6363126b73674d3b2f7eb5cffe3578e6b204ffaae2b6f241751596
```

**Vector 4 — Truncation resistance**

Two full 16,384-byte chunks encrypted without a short final chunk (32,856 total bytes). Decryption must return a truncation error.

**Vector 5 — Empty file**

Ciphertext (72 bytes):

```
0x020202020202020202020202020202020202020202020202
4def3b6cafc9716c7a8e1311b3d10874248b4526ec59958313ef9a3c088f8524
886ac2e165d5871aac7dc442ba5d03d8
```

**Vector 6 — Padding (short real-final-chunk)**

```
plaintext  = 100,000 bytes of 0x61 ('a')
target     = 524,288
file_key   = Vector 1's file_key
salt       = Vector 1's salt
context    = "attachment" || 0x00 || "test-room-abc123"
```

Expected:

```
L_padded                     = 523,720
padding_zeros                = 423,720
ciphertext_length            = 524,288
final_chunk_plaintext_length = 15,816
```

The ciphertext MUST decrypt cleanly under the C2SP decryption rules. The trailing 423,720 bytes of the decrypted plaintext MUST be zero.

#### Rust implementation

No crate implementing C2SP chunked encryption exists on crates.io as of 2026-09-28. Implementations must build the construction directly using:

- `aes-gcm = "0.10"` for AES-256-GCM
- `hkdf = "0.12"` for HKDF-Expand-SHA256
- `sha2 = "0.10"` for SHA-256

The verification prototype at `verification/c2sp-rust/prototype/` is a reference implementation. The Phase 12a task should port this prototype into `server/src/` (client-side encryption is out of scope; the server only validates manifest fields and stores opaque bytes).

The server does **not** decrypt attachments. All cryptographic operations are client-side. The server verifies only:

- The claimed `file_id` matches the SHA-256 of the uploaded ciphertext.
- The `chunk_size` matches the C2SP constant (16384).
- The `chunk_count` is consistent with `padded_size`.
- The `salt` decodes to 24 bytes.
- The `commitment` decodes to 32 bytes.
- The `padded_size` matches a configured bucket.

### 6.6 Attachments — Upload

- The client uploads the entire padded ciphertext as a single object.
- The `file_id` is the SHA-256 of the **padded ciphertext**, not the plaintext.
- The server verifies the hash before storing.

### 6.7 Attachments — Streaming and Seeking

- The client decides whether to fetch all chunks at once or lazily based on file size and MIME type.
- **Recommended threshold:** files ≤ 1 MB are fetched in a single request; files > 1 MB are fetched lazily.
- **Media (audio/video):** always fetched lazily, regardless of size.
- **The client is responsible for translating plaintext ranges to encrypted ranges.** The server does not translate.

### 6.8 Attachments — Presigned URLs

- For S3 backends, the client fetches ranges directly from S3 using presigned URLs.
- The client requests a fresh URL per seek operation. **Do not cache presigned URLs.**
- Presigned URLs support `Range` headers natively.
- The client must handle URL expiry mid-download by requesting a new URL and resuming.

**Presigned URL scope:** The URL grants access to the entire object. It does **not** restrict to a specific byte range. A leaked URL grants access to encrypted bytes only — without the encryption key (which is not in the URL), the blob is useless.

### 6.9 Attachments — MP4 Streaming

For MP4 files to stream and seek properly:

- The `moov` atom must be at the **beginning** of the file ("fast-start MP4").
- The client must validate this before upload and reject or re-encode files that are not fast-start.
- This is a container-format constraint, not an encryption constraint.

The server does not enforce fast-start validation. It is a client responsibility.

### 6.10 Attachments — Forwarding

Forwarding a file from one room to another requires re-encryption. The context binds to the source room's `room_id`. A forwarded file is a new object with its own key, its own manifest, and its own lifecycle. The server cannot correlate the original and forwarded blobs (different content hashes).

### 6.11 Range Request Format

The client sends a `Range` header in the standard HTTP format:

- `Range: bytes=START-END` — inclusive range
- `Range: bytes=START-` — from START to end of file
- `Range: bytes=-SUFFIX` — last SUFFIX bytes

**Supported:**
- Single ranges only
- `bytes` unit only

**Not supported:**
- Multiple ranges → `416 Range Not Satisfiable`
- Non-bytes units → `416 Range Not Satisfiable`

**Server response:** `206 Partial Content` with `Content-Range: bytes START-END/TOTAL`.

**Clipping:** If the requested range extends beyond the file size, the server serves the available bytes and returns the actual range in `Content-Range`. This matches RFC 7233 §4.4.

#### Range batching

Clients fetch many chunks at once by issuing a single `Range` request that spans multiple chunks. This is the recommended strategy for video seeking, where a seek operation may require 50–200 consecutive chunks.

Given a plaintext range `[p_start, p_end]` that spans chunks `[k_start, k_end]`:

```
encrypted_start = 56 + k_start * (16384 + 16)
encrypted_end   = 56 + (k_end + 1) * (16384 + 16) - 1
```

Issue a single `Range: bytes=<encrypted_start>-<encrypted_end>` request. Decrypt each chunk in the returned range using its derived nonce.

**Do not batch across unbounded sizes.** The server enforces a maximum response size equal to the largest bucket size. Clients requesting ranges larger than this receive `416 Range Not Satisfiable`. Batch in units of buckets (64 KB, 512 KB, 4 MB, 32 MB, 256 MB).

### 6.12 Room Membership

- Room membership is server-visible at the user level.
- MLS leaf-level membership is client-visible only.

### 6.13 Push Subscriptions

- Register via `POST /users/me/push-subscriptions`.
- `platform` must be `web`, `ios`, `android`, or `desktop`.

### 6.14 Safety Numbers

- Clients compute and display safety numbers out-of-band.
- The server never sees safety numbers.

### 6.15 WebSocket Connection

- Connect to the URL advertised in `GET /api/v1/capabilities` (`websocket_url`).
- Production: always `wss://`.

### 6.16 Client Events

The following events are published by clients via Sockudo client events (prefix `client-`) and relayed to other subscribers of the same private channel. They are not persisted and not available in history.

| Event | Payload | Purpose |
|---|---|---|
| `client-typing.start` | `{ user_id }` | Typing indicator active |
| `client-typing.stop` | `{ user_id }` | Typing indicator inactive |
| `client-read` | `{ user_id, message_id }` | Read receipt |

**Authentication.** Client events are accepted only on channels the publishing client is authorized to subscribe to. The Sockudo auth endpoint (`POST /sockudo/auth`) enforces channel membership before signing.

**Payload trust.** The server does not verify that the `user_id` in the payload matches the authenticated session. Clients MUST verify the `user_id` against the channel's known members. Spoofed `user_id` values MUST be ignored.

**Rationale:** Typing indicators and read receipts are ephemeral. If a client is offline when they fire, they do not need to catch up.

### 6.17 Client Capability Requirements

- Read `GET /api/v1/capabilities` on startup.
- Degrade gracefully if a capability is unavailable.

### 6.18 CoreCrypto Initialization

The server does not provide MLS state to the client. All MLS state is client-managed.

**Entropy seed.** The client derives a 32-byte `entropySeed` from a device-specific secret stored in platform-secure storage. The derivation is implementation-defined and not part of the server contract. The server never sees the seed.

**WASM module.** The CoreCrypto WASM module is bundled with the client. The server does not serve it.

**Keystore.** The client maintains an encrypted keystore for MLS state (group state, key material, epoch history). The keystore is local-only. The server never sees its contents.

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
```

### 7.5 Attachments

```sql
CREATE TABLE attachments (
    id                 TEXT PRIMARY KEY,
    room_id            TEXT NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    uploader_id        TEXT NOT NULL REFERENCES users(id),
    uploader_client_id TEXT,
    storage_backend    TEXT NOT NULL CHECK(storage_backend IN ('fs', 's3')),
    storage_key        TEXT NOT NULL,
    padded_size        INTEGER NOT NULL,
    plaintext_size     INTEGER NOT NULL,
    encrypted_size     INTEGER NOT NULL,
    chunk_size         INTEGER NOT NULL,
    chunk_count        INTEGER NOT NULL,
    salt               TEXT NOT NULL,
    commitment         TEXT NOT NULL,
    content_type       TEXT NOT NULL DEFAULT 'application/octet-stream',
    created_at         DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
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
- `storage_backend` — `'fs'` or `'s3'`.
- `storage_key` — path or S3 key.
- `padded_size` — total uploaded bytes (a bucket value).
- `plaintext_size` — original file size.
- `encrypted_size` — same as `padded_size`; kept separate for clarity.
- `chunk_size` — always `16384`. Stored for forward compatibility.
- `chunk_count` — number of chunks including padding.
- `salt` — base64 of the 24-byte C2SP salt.
- `commitment` — base64 of the 32-byte C2SP commitment.
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
  "storage_presign_supported": true,
  "storage_presign_max_ttl_seconds": 1200,
  "attachment_format": "c2sp-chunked-aes256gcm-v1",
  "attachment_chunk_size": 16384,
  "attachment_bucket_sizes": [65536, 524288, 4194304, 33554432, 268435456],
  "attachment_accept_ranges": true,
  "safety_number_mode": "warn",
  "moderation_mode": "messenger"
}
```

| Field | Notes |
|---|---|
| `storage_backend` | `"fs"` or `"s3"` |
| `storage_presign_supported` | `true` for S3, `false` for filesystem |
| `storage_presign_max_ttl_seconds` | Maximum TTL a client may request. Equals `2 × S3_PRESIGN_TTL_SECONDS` when S3 is active. |
| `attachment_format` | Format identifier for forward compatibility |
| `attachment_chunk_size` | Always `16384` |
| `attachment_bucket_sizes` | Valid padded sizes for upload |
| `attachment_accept_ranges` | Always `true` in V1 |

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
| GET | `/rooms/:id` | Room metadata (includes effective limits) |
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

**`GET /rooms/:id` response includes:**

```json
{
  "id": "<room_id>",
  "name_encrypted": "...",
  "owner_id": "...",
  "created_at": "...",
  "effective_max_file_size_bytes": 104857600,
  "effective_message_retention_days": 0,
  "member_count": 5
}
```

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
| DELETE | `/rooms/:id/messages/:msg_id` | Delete (tombstone) a message |
| GET | `/rooms/:id/epoch` | Current epoch and sequence |
| GET | `/rooms/:id/pending-removes` | List pending MLS removes |
| POST | `/rooms/:id/pending-removes/:id/consume` | Mark remove consumed |
| POST | `/sockudo/auth` | Sign a channel subscription |

#### 8.5.1 Delta Sync Cursor

Clients tracking the last-seen position in a room use the composite cursor `(epoch, seq)`.

To fetch messages newer than a known position:

```
GET /rooms/:id/messages?since_epoch={epoch}&since_seq={seq}&limit={n}
```

The server returns messages with:

```
(epoch > since_epoch) OR (epoch = since_epoch AND seq > since_seq)
```

ordered by `(epoch, seq)` **ascending**. This matches the ordering of the `idx_room_messages_room_epoch_seq` index.

**Initial sync (no cursor):** Clients omit the query parameters. The server executes the query with `ORDER BY (epoch, seq) DESC LIMIT N`, then **reverses the result before responding**. The response body is ordered `(epoch, seq)` **ascending**, matching the delta sync response. Clients render directly from the response without reordering.

**Default and maximum limits:** Default 50, maximum 500.

**Response shape:**

```json
{
  "messages": [ ... ],
  "next_cursor": { "epoch": 42, "seq": 187 },
  "has_more": false
}
```

Each message includes a `deleted_at` field. Clients render deleted messages as tombstones.

#### 8.5.2 Message Deletion

`DELETE /rooms/:id/messages/:msg_id` tombstones a message.

**Authorization:**
- The sender can delete their own message.
- The room owner can delete any message.
- A moderator can delete any message (Discord mode only).

**Behavior:**
1. Verify authorization.
2. Verify the message exists and is not already deleted.
3. Set `deleted_at = CURRENT_TIMESTAMP`.
4. Publish `message.deleted` with payload `{ id, room_id }` to `private-room-{room_id}`.
5. Return `204 No Content`.

**Retention policy:** The row, ciphertext, and any associated attachment are retained. Retention pruning is the mechanism that eventually removes them. Deletion does not accelerate pruning.

**Ciphertext access:** `GET /rooms/:id/messages/:msg_id/ciphertext` returns `404 message_deleted` for tombstoned messages. The ciphertext is not served.

**Attachment lifetime:** Governed by retention, not by message deletion. A client that receives a tombstone removes the attachment from curation views only if no other message references it.

**GDPR export:** Deleted message metadata (including `deleted_at`) is included in the user's export. Ciphertext is excluded.

### 8.6 Attachments

| Method | Path | Purpose |
|---|---|---|
| POST | `/rooms/:id/attachments` | Upload encrypted attachment |
| GET | `/attachments/:id` | Download (supports `Range`) |
| DELETE | `/attachments/:id` | Delete (uploader only) |
| POST | `/attachments/:id/presign` | Generate a presigned URL (S3 only) |

#### 8.6.1 Upload

**Request — Multipart:** `multipart/form-data` with parts:

| Part | Type | Required |
|---|---|---|
| `file` | binary | Yes |
| `claimed_id` | text (64 hex chars) | Yes |
| `plaintext_size` | text (integer) | Yes |
| `encrypted_size` | text (integer) | Yes |
| `chunk_size` | text (integer) | Yes |
| `chunk_count` | text (integer) | Yes |
| `salt` | text (base64, 24 bytes) | Yes |
| `commitment` | text (base64, 32 bytes) | Yes |
| `content_type` | text | No (default `application/octet-stream`) |
| `uploader_client_id` | text | No |

**Request — Octet-stream:** `Content-Type: application/octet-stream` with the binary in the body and metadata in headers:

| Header | Required |
|---|---|
| `X-Claimed-Id` | Yes |
| `X-Plaintext-Size` | Yes |
| `X-Encrypted-Size` | Yes |
| `X-Chunk-Size` | Yes |
| `X-Chunk-Count` | Yes |
| `X-Salt` | Yes |
| `X-Commitment` | Yes |
| `X-Content-Type` | No |
| `X-Uploader-Client-Id` | No |

**Effective file size limit:** `MIN(room.max_file_size_bytes, instance.file_size_bytes, server_max.file_size_bytes)`. Exceeding returns 413.

**Response — 201:**

```json
{
  "id": "<sha256 hex>",
  "room_id": "<room_id>",
  "uploader_id": "<user_id>",
  "padded_size": 65536,
  "plaintext_size": 100000,
  "encrypted_size": 100016,
  "chunk_size": 16384,
  "chunk_count": 7,
  "salt": "<base64>",
  "commitment": "<base64>",
  "content_type": "video/mp4",
  "created_at": "<ISO 8601>"
}
```

**Errors:**

| Error | HTTP | `error` field |
|---|---|---|
| Not a member | 404 | `room_not_found` |
| Missing `file` | 400 | `missing_file` |
| Missing or invalid `claimed_id` | 400 | `invalid_claimed_id` |
| Invalid manifest field | 400 | `invalid_manifest` with `details: {field, reason}` |
| Hash mismatch | 400 | `hash_mismatch` with `details: {expected, computed}` |
| Invalid bucket size | 413 | `invalid_bucket_size` with `details: {received, allowed}` |
| File too large | 413 | `file_too_large` with `details: {limit, received}` |
| Duplicate ID with different owner | 409 | `id_conflict` |
| Unsupported content type | 415 | `unsupported_media_type` |
| Storage error | 500 | `internal` |

#### 8.6.2 Download

Supports the standard HTTP `Range` header for partial content. See §6.11 for supported syntax.

**Full download — Response — 200:**

```
Content-Type: application/octet-stream
Content-Length: <padded_size>
Content-Disposition: inline
Accept-Ranges: bytes
Cache-Control: private, max-age=86400, immutable
ETag: "<id>"
X-Attachment-Content-Type: <content_type>
X-Attachment-Chunk-Size: 16384
X-Attachment-Chunk-Count: <chunk_count>
X-Attachment-Plaintext-Size: <plaintext_size>
X-Attachment-Encrypted-Size: <encrypted_size>
X-Attachment-Salt: <salt>
X-Attachment-Commitment: <commitment>
X-Content-Type-Options: nosniff
```

**Range download — Response — 206:**

Same headers as full download, plus:

```
Content-Range: bytes <start>-<end>/<padded_size>
Content-Length: <end - start + 1>
```

**`If-None-Match`:** A matching ETag returns `304 Not Modified` with no body. This takes precedence over range serving.

**Unsupported or unsatisfiable ranges — Response — 416:**

```
Content-Range: bytes */<padded_size>
```

No body.

**Errors:**

| Error | HTTP | `error` field |
|---|---|---|
| Not found or not a member | 404 | `attachment_not_found` |
| Malformed range | 416 | `range_not_satisfiable` |
| Range out of bounds | 416 | `range_not_satisfiable` |
| Multiple ranges | 416 | `range_not_satisfiable` |
| Non-`bytes` unit | 416 | `range_not_satisfiable` |
| Storage read error | 500 | `internal` |

#### 8.6.3 Presign

**Request:**

```json
{
  "expires_in_seconds": 600
}
```

The field is optional. If omitted, `S3_PRESIGN_TTL_SECONDS` is used.

**Response — 200:**

```json
{
  "url": "https://s3.example.com/bucket/attachments/ab/cd/abc...?X-Amz-...",
  "expires_at": "2026-09-28T14:32:11Z"
}
```

**TTL clamping:**

- Minimum: 30 seconds
- Maximum: `2 × S3_PRESIGN_TTL_SECONDS`
- Default: `S3_PRESIGN_TTL_SECONDS`

**Rate limit:** `RATE_PRESIGN_PER_MIN` per user per minute.

**Errors:**

| Error | HTTP | `error` field |
|---|---|---|
| Not found or not a member | 404 | `attachment_not_found` |
| Presign not supported | 501 | `presign_not_supported` |
| Invalid `expires_in_seconds` | 400 | `invalid_expires_in` |
| Rate limited | 429 | `rate_limited` with `details: {reset_at}` |
| Storage error | 500 | `internal` |

#### 8.6.4 Delete

Only the uploader can delete an attachment.

**Response — 204 No Content.**

**Errors:**

| Error | HTTP | `error` field |
|---|---|---|
| Not found | 404 | `attachment_not_found` |
| Not the uploader | 403 | `forbidden` |
| Storage error (best-effort) | — | logged only |

### 8.7 Event Catalog

The server publishes the following events to `private-room-{room_id}` channels. Payloads are JSON. Message ciphertext is opaque — clients decrypt locally.

| Event | Payload | When |
|---|---|---|
| `message.new` | `{ id, room_id, sender_user_id, sender_client_id, epoch, seq, content_type, created_at }` | After `POST /rooms/:id/messages` succeeds |
| `message.deleted` | `{ id, room_id }` | After `DELETE /rooms/:id/messages/:msg_id` succeeds |
| `room.updated` | `{ room_id, name_encrypted?, retention_days?, max_file_size_bytes? }` | After room metadata changes |
| `room.member_added` | `{ room_id, user_id, role, joined_at }` | After `POST /rooms/:id/members` succeeds |
| `room.member_removed` | `{ room_id, user_id }` | After kick or leave |
| `epoch.updated` | `{ room_id, epoch, sequence }` | After `room_epochs` is advanced |

**Not published (pulled via REST instead):**

- MLS welcomes — fetched via `GET /welcomes`
- Pending MLS removes — fetched via `GET /rooms/:id/pending-removes`
- Full epoch state — fetched via `GET /rooms/:id/epoch`
- Missed messages — fetched via `GET /rooms/:id/messages` with cursor

**Rationale for the split:** MLS state changes are order-dependent and failure-prone. Pushing them over an unreliable WebSocket and hoping the client applies them correctly is worse than letting the client pull them on its own schedule, with retries.

**Delivery guarantee:** Events are best-effort. Clients MUST NOT rely on receiving every event over the socket. Authoritative state is always obtained via REST (§8.5).

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
| Message deletion | Tombstones via server endpoint; publishes `message.deleted` |
| Attachments — upload | Content-addressed, C2SP chunked encryption, validates manifest |
| Attachments — download | Supports `Range` headers; returns `206 Partial Content` |
| Attachments — streaming | Client translates plaintext ranges; server serves encrypted bytes |
| Attachments — presign | S3 only; returns signed URLs; filesystem returns 501 |
| Attachment chunk format | 16 KiB chunks (C2SP protocol constant), per-chunk derived nonces |
| Attachment padding | Plaintext-level padding before encryption; produces one short final chunk |
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
| Presign support | Advertised as `storage_presign_supported` in `/capabilities` |
| Client events | `client-typing.*`, `client-read` relayed on private channels |
| Delta sync | `(epoch, seq)` cursor via `since_epoch` + `since_seq` |

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
| 11 | Sockudo integration (publish, auth, event catalog, delta sync) | 10 |
| 12a | Attachment upload, download, delete (C2SP format) | 11 |
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
| Server sees message metadata | Minimal: room ID, sender client ID, epoch, coarsened timestamp, deletion timing |
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
| Attachments are streamed via range requests | Yes, with C2SP chunked encryption. |
| Attachments are vulnerable to truncation attacks | No. Enforced by the C2SP short-final-chunk rule. |
| Attachment encryption is key-committing | Yes. HKDF-derived commitment in the header. |
| Attachment padding is C2SP-compliant | Yes. Plaintext-level padding produces one short final chunk. |
| Presigned URLs grant access to full object | Yes — but only encrypted bytes. TTL-limited to `2 × S3_PRESIGN_TTL_SECONDS`. |
| Presigned URLs are range-restricted | No. Client adds `Range` header on fetch. |
| Server knows which messages are deleted | Yes. Documented in §14.6. |

---

## 13. Backups and Disaster Recovery

- Automatic snapshots of SQLite and the OPRF key every `BACKUP_INTERVAL_HOURS`.
- Attachment blobs on filesystem are included only if `BACKUP_INCLUDE_ATTACHMENTS=true`.
- Attachment blobs on S3 are **not** backed up by the server. Rely on S3 durability.
- Snapshots are encrypted with `backup_key` derived from the root secret.
- Restore is CLI-only: `server restore --from <path> --confirm`.
- The API lists backups and triggers manual backups but cannot restore them.

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

ZIP archive with `profile.json`, `devices.json`, `sessions.json`, `rooms.json`, `messages.json`, `attachments.json`, `audit.json`, `README.txt`.

`messages.json` includes metadata (including `deleted_at`) for all messages the user sent or received. Ciphertext for deleted messages is excluded. `attachments.json` lists metadata and manifest fields but not the blob.

### 14.4 Retention

`DATA_RETENTION_DAYS` controls message and attachment purge. `AUDIT_RETENTION_DAYS` controls the audit log.

### 14.5 Tombstone Semantics

Deleted messages are tombstoned. The server deletes its copy from the message list view but retains the row and ciphertext until retention pruning. Peers delete local copies at their discretion.

### 14.6 Metadata Minimisation

| Field | Leak | Mitigation |
|---|---|---|
| `devices.client_id` | Random 128-bit, not a hardware identifier | Keep — no mitigation needed |
| `sessions.last_seen_at` | Timestamp | Coarsened to hour in admin views |
| `messages.created_at` | Timestamp | Coarsened to minute |
| `room_messages.deleted_at` | Server knows which messages were deleted and when | Coarsened to day in admin views; per-user deletion counts not surfaced; audit log records actor |
| `audit_log.metadata` | JSON | Never includes content or IPs |
| Push `sender_user_id` | Sender identity is plaintext in push payload | Required for display name lookup; documented limitation |
| IP addresses | Not stored in schema | Logs rotated and not persisted indefinitely |

### 14.7 Controller Obligations

Controller publishes privacy policy, provides DPA if needed, documents server location and sub-processors (Sockudo, S3 provider).

### 14.8 Audit Actions

`user.delete`, `user.export`, `config.update`, `secret.update`, `limits.update`, `role.grant`, `role.revoke`, `invite.create`, `invite.revoke`, `device.revoke`, `message.delete`, `bootstrap.owner`, `vapid.rotate`, `altcha.rotate`, `storage.migrate`.

---

## 15. Amendment Process

Factual disputes about external specifications must be resolved by citation, not negotiation. When one party claims a specification says X and the other claims it says Y, the resolution is to retrieve the specification and quote the relevant passage. The party whose claim is not supported by the text accepts the correction without further debate.

Design questions — behavior, policy, trade-offs — are resolved through reasoned argument and may be negotiated.

---

## 16. Amendments

### 16.1 Amendment 1 — C2SP Chunked Encryption

**Date:** 2026-09-28

**Change:** Replaced the bespoke chunked AES-GCM format with C2SP chunked encryption (`c2sp.org/chunked-encryption`), instantiated with AES-256-GCM and SHA-256 at the protocol-mandated chunk size of 16 KiB.

**Sections affected:** §5.13, §6.5, §6.7, §6.8, §6.9, §6.10, §6.11, §7.5, §8.1, §8.6.

**Affected phases:** 12a, 12b (rewritten). Phases 1–11 unaffected.

**Chunk size correction:** The client team initially proposed 64 KiB chunks for reduced range request count. This was rejected on 2026-09-28 after empirical verification confirmed that the C2SP specification fixes the chunk size at 16 KiB as a protocol constant. See §16.10.

### 16.2 Amendment 2 — Message Deletion Endpoint

**Date:** 2026-09-28

**Change:** Added `DELETE /rooms/:id/messages/:msg_id` with tombstone semantics, Option D retention, and `message.deleted` event. Added `deleted_at` column to `room_messages`.

**Sections affected:** §3.2, §4.5, §7.4, §8.5, §8.7, §14.3, §14.5, §14.6, §14.8.

**Affected phases:** 11 (adds endpoint and event). Phases 1–10 unaffected.

### 16.3 Amendment 3 — Event Catalog and Client Events

**Date:** 2026-09-28

**Change:** Added §8.7 event catalog. Added §6.16 client events policy (typing, read receipts). Added `SOCKUDO_ENABLE_CLIENT_EVENTS` env var. Added channel taxonomy.

**Sections affected:** §2.3, §5.19, §6.16, §8.7.

**Affected phases:** 11 (event catalog). Phases 1–10 unaffected.

### 16.4 Amendment 4 — Delta Sync Cursor

**Date:** 2026-09-28

**Change:** Defined the composite cursor `(epoch, seq)` for delta sync. Added `since_seq` query parameter. Added `next_cursor` and `has_more` to response.

**Correction (2026-09-28):** Initial sync and delta sync both return ascending order. The server reverses the initial-sync result before responding. See §8.5.1.

**Sections affected:** §8.5.

**Affected phases:** 11 (endpoint update). Phases 1–10 unaffected.

### 16.5 Amendment 5 — Push Payload Schema

**Date:** 2026-09-28

**Change:** Defined the push notification JSON envelope and platform-specific mappings. Added metadata leak note for `sender_user_id`.

**Sections affected:** §5.10, §14.6.

**Affected phases:** 15, 16. Phases 1–14 unaffected.

### 16.6 Amendment 6 — Effective Limits Exposure

**Date:** 2026-09-28

**Change:** Added `effective_max_file_size_bytes` and `effective_message_retention_days` to `GET /rooms/:id`.

**Sections affected:** §4.4, §8.4.

**Affected phases:** 7a (response shape). Phases 1–6d unaffected.

### 16.7 Amendment 7 — Presence Out of Scope

**Date:** 2026-09-28

**Change:** Explicitly excluded real-time presence from V1. Added V2 concept of presence channels.

**Sections affected:** §2.2, §2.3.

**Affected phases:** None.

### 16.8 Amendment 8 — CoreCrypto Initialization Clarification

**Date:** 2026-09-28

**Change:** Added §6.18 clarifying that the server does not provide MLS state, entropy seeds, or WASM modules.

**Sections affected:** §6.18.

**Affected phases:** None.

### 16.9 Amendment 9 — Context Binding Test Vectors

**Date:** 2026-09-28

**Change:** Canonical test vectors computed by the Rust prototype at `verification/c2sp-rust/prototype/`, cross-verified against the Go reference implementation (`filippo.io/cobblestone`). Integrated into §6.5.

**Test vectors:**

- **Vector 1** — Base derivation: `file_key`, `base_nonce`, `commitment` confirmed.
- **Vector 2** — Cross-room separation: all derived values differ when `room_id` changes.
- **Vector 3** — Single chunk encryption: 115-byte ciphertext for the 43-byte plaintext.
- **Vector 4** — Truncation resistance: full-final-chunk ciphertexts are rejected.
- **Vector 5** — Empty file: 72-byte ciphertext.

**Verification outcome:** The Rust prototype's values matched the Go reference implementation byte-for-byte.

**Sections affected:** §6.5.

**Affected phases:** 12a (test fixture).

### 16.10 Amendment 10 — Chunk Size Correction and Padding Algorithm

**Date:** 2026-09-28

**Change:** Confirmed the C2SP chunk size is a protocol constant (16384 bytes), not application-selectable. Updated §5.13 and §6.5.

**Verification:** Empirical verification of the C2SP specification and the `filippo.io/cobblestone` reference implementation confirmed that the chunk size is `const ChunkSize = 16 * 1024` and is not configurable via any API parameter. The specification states: *"Padding and variable chunk sizes are not supported, to allow random access decryption with a predictable mapping of plaintext indices."* A 64 KiB variant would fail all Wycheproof test vectors for C2SP chunked encryption.

**Superseded by Amendment 12** for the padding algorithm portion. The chunk size conclusion stands.

**Sections affected:** §5.13, §6.5, §6.11.

**Affected phases:** 12a, 12b.

### 16.11 Amendment 11 — Sync Ordering Fix

**Date:** 2026-09-28

**Change:** Initial sync and delta sync both return ascending order. The server reverses the initial-sync result before responding. Clients render directly from the response without reordering.

**Sections affected:** §8.5.1.

**Affected phases:** 11.

### 16.12 Amendment 12 — Padding Algorithm Replacement and Bucket Extension

**Date:** 2026-09-28

**Status:** Revised after verification

**Verification artifacts:**

- Findings Report: Verification of Amendment 12 Attachment Padding Algorithm Flaw (2026-09-28)
- Reviewer's initial critique (2026-09-28)

**Change:** Replaced the padding algorithm in §6.5 with a verified plaintext-level algorithm. Extended `ATTACHMENT_BUCKET_SIZES` to include a 256 MiB bucket covering the full range of `SERVER_MAX_FILE_SIZE_BYTES`.

**Root cause:** The original proposal treated padding as an outer layer applied after encryption. C2SP requires all non-final chunks to be exactly `chunk_size` bytes. Appending padding chunks after a plaintext whose final chunk was short produced a sequence with a short non-final chunk, violating the C2SP rule. Every conformant decryptor rejected the resulting ciphertext.

**Empirical evidence:**

- Sweep A: 512 KiB bucket, plaintext range [0, 200,000] — original algorithm failed **200,001 / 200,001**; revised algorithm failed **0 / 200,001**.
- Sweep B: 64 KiB bucket, plaintext range [60,000, 100,000] — original failed 40,000 / 40,001; revised failed 0 within bucket capacity.
- Sweep C: 4 MiB bucket, plaintext range [60,000, 100,000] — original failed 4,001 / 4,001; revised failed 0 / 4,001.
- Go cross-verification: 20 cases spanning [0, 190,500] — all 20 failed under the original algorithm with `cipher: message authentication failed`; all 20 succeeded under the revised algorithm.

**Sections affected:** §5.13, §6.5, §8.1, §10, §12.

**Affected phases:** 12a (client-side padding algorithm), 12b (no change — range translation operates on already-padded ciphertexts).

**Test vector 6 added:** Padding case with a short real-final-chunk (100,000-byte plaintext padded to 512 KiB). Added to §6.5.

---

## 17. Document Status

This is the contract for the server side of the system. Every Jules task references this document. If a task conflicts with this spec, the task is wrong and must be revised. If a feature is missing, it does not exist yet — it must be added here first, then built.

Amendments are tracked in §16. The spec is stable for V1; new features go into V2.
