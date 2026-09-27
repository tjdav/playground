
# Server Specification v1.0

> **Status:** Frozen — source of truth for all Jules tasks.
> **Scope:** This document describes the **server-side contract only**. Client implementation is out of scope and will be covered by a separate client specification.
> **Stack:** Sockudo + Axum + SQLite + OPAQUE + ALTCHA
> **Deployment:** Single VPS, Docker-based, self-hosted. Two containers: `server` (Axum + static SPA hosting) and `sockudo`. TLS terminates at a reverse proxy (Coolify/Traefik or Caddy).

Any change to this document requires an explicit revision and a corresponding task update. Frozen means: no feature additions, no schema changes, no API surface changes without a spec revision and a documented migration path.

---

## 1. Product Summary

A self-hosted, end-to-end encrypted group messaging system with MLS-grade forward secrecy and post-compromise security. The server is an untrusted delivery service that never sees plaintext, keys, or meaningful metadata.

| Layer | Technology |
|---|---|
| Delivery | Sockudo (WebSocket, Pusher v7, Protocol V2 history) |
| API | Axum (Rust) |
| Database | SQLite (embedded in the Axum process) |
| Blob storage | Filesystem or S3 (content-addressed) |
| Auth | OPAQUE (aPAKE) via `opaque-ke` 4.0.1 |
| Bot protection | ALTCHA Proof-of-Work v2 via `altcha` 0.2.0 |
| MLS engine | Wire CoreCrypto 10.5.2 (client-side only) |
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
- Encrypted attachments with configurable size limits
- Push notifications (Web Push, APNs, FCM, UnifiedPush)
- Safety number verification (soft warning, non-blocking)
- Recovery via multi-use one-time code list
- Admin and user web UIs (both first-class)
- Automatic SQLite + Sockudo backups
- Three-tier resource limits (server / instance / entity)
- Display name separate from immutable username
- Server-side cleanup jobs (sessions, rate limits, audit log, attachments, welcomes)
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
- Multi-seed OPRF rotation (dependent on `opaque-ke` library support)

### 2.3 Out of Scope

- Federation between servers
- Multi-tenancy
- Server-side key escrow
- Plaintext metadata on server
- Email or OAuth registration
- Anonymous accounts
- Plaintext message export
- Username changes (usernames are immutable)
- Third-party CAPTCHA services
- Client-side data deletion (client responsibility)
- Forcing peers to delete local message copies (impossible in E2EE)
- Consent management UI (client responsibility)
- TLS termination inside the Axum process
- ACME/Let's Encrypt client inside the server

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

Toggled per-instance via `MODERATION_MODE`. The setting is live and applies to all rooms retroactively. The `moderation_override` column exists in the `rooms` table for future per-room exceptions but is unused in V1.

### 3.4 Enforcement Order

Every API call checks:
1. Global permission for the action
2. Room role for the target room (if applicable)

Effective privilege is the union.

### 3.5 Room Ownership Transfer

- Owner can explicitly transfer ownership to any other room member.
- If owner leaves without transferring, ownership passes to the longest-tenured moderator, or to the longest-tenured member if no moderators exist.
- If the room has no other members, it is deleted.
- Instance admins can force-transfer ownership via the admin API.

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
| Session cleanup | `expires_at < now() - 30 days` OR `revoked_at < now() - 30 days` | Preserves active and recently-expired sessions |
| Rate limit table | Entries older than 24 hours | Longest rate window is daily |
| Audit log | 90 days (configurable via `AUDIT_RETENTION_DAYS`) | Compliance-aligned default |
| Attachment pruning | Per-room retention, falling to instance default, then server hard max | Respects three-tier limits |
| Welcome expiry | 7 days | Stale welcomes deleted; room admin must re-issue |
| Message retention | Per-room `retention_days`, falling to instance default | Deletes messages from Sockudo history and DB references |
| Registration state | 5-minute TTL, purged opportunistically on insert | In-memory only, lost on restart |
| Login state | 5-minute TTL, purged opportunistically on insert | In-memory only, lost on restart |

---

## 5. Environment Variables

Follows Coolify conventions. All variables are optional unless marked **required**. Defaults are applied at startup.

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
| `APP_URL` | — | **Required.** Public URL of the deployment. Must begin with `https://` when `APP_ENV=production`. Server refuses to start otherwise. |
| `APP_NAME` | `Encrypted Chat` | Display name |
| `LOG_LEVEL` | `info` | `trace`, `debug`, `info`, `warn`, `error` |
| `CLIENT_STATIC_DIR` | — | Optional. When set, Axum serves the SPA from this directory with an SPA fallback. When unset, static serving is disabled (development mode where the Coralite dev server runs separately). |

**Startup validation:** In production, `APP_URL` must start with `https://`. The server exits with a fatal error if the scheme is `http://`. In development, `http://` is permitted.

### 5.2 Server

```env
SERVER_BIND=0.0.0.0:8080
SERVER_WORKERS=4
```

| Variable | Default | Notes |
|---|---|---|
| `SERVER_BIND` | `0.0.0.0:8080` | Axum bind address |
| `SERVER_WORKERS` | CPU count | Tokio worker threads |

### 5.3 Database

```env
DB_PATH=/data/app.db
DB_BUSY_TIMEOUT_MS=5000
```

| Variable | Default | Notes |
|---|---|---|
| `DB_PATH` | `/data/app.db` | SQLite file path |
| `DB_BUSY_TIMEOUT_MS` | `5000` | SQLite `busy_timeout` PRAGMA |

### 5.4 Sessions

```env
SESSION_EXPIRY_DAYS=30
SESSION_SLIDING=true
```

| Variable | Default | Notes |
|---|---|---|
| `SESSION_EXPIRY_DAYS` | `30` | Absolute expiry |
| `SESSION_SLIDING` | `true` | Activity refreshes expiry |

### 5.5 Server Hard Limits

Absolute ceilings. Nothing can exceed them at runtime.

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
| `CORS_ALLOWED_ORIGINS` | empty (dev: `*`) | Comma-separated list of allowed origins |
| `CSRF_PROTECTION` | `none` | `none`, `header`, `double-submit` |
| `TRUST_PROXY` | `false` | Trust `X-Forwarded-Proto`, `X-Forwarded-For`, `X-Real-IP`. Set to `true` behind a reverse proxy. |
| `HSTS_MAX_AGE` | `31536000` | HSTS max-age in seconds (1 year). Set to `0` to disable HSTS. |
| `HSTS_INCLUDE_SUBDOMAINS` | `true` | Only disable if the domain is shared with non-HTTPS services. |

**CORS:** In production, `CORS_ALLOWED_ORIGINS` must be set. In development, it defaults to `*` with a startup warning.

**CSRF:** Bearer-token auth does not require CSRF protection. The `CSRF_PROTECTION` variable exists to support future cookie-based sessions. V1 uses `none`.

The server rejects requests with an `Origin` header not in the allowed list, even for non-CORS requests. Origin mismatches are logged at `warn` level.

**Trust proxy:** When `TRUST_PROXY=false`, the server ignores all forwarding headers. This is safe when the server is not behind a proxy, but produces wrong redirects when it is. When `TRUST_PROXY=true`, the server trusts `X-Forwarded-Proto` to determine the client's scheme and `X-Forwarded-For` for IP-based rate limiting.

**HSTS:** Only sent on HTTPS responses. When `HSTS_MAX_AGE=0`, the header is omitted entirely. The `preload` directive is never set — preloading requires submitting the domain to a browser-managed list, which is inappropriate for self-hosted deployments.

### 5.8 OPAQUE

```env
OPAQUE_OPRF_KEY_PATH=/data/oprf.key
```

| Variable | Default | Notes |
|---|---|---|
| `OPAQUE_OPRF_KEY_PATH` | `/data/oprf.key` | Path to the persisted `ServerSetup` (OPRF seed + server static keypair) |

**Crate:** `opaque-ke` 4.0.1, feature `argon2`.

**Rust minimum:** 1.85.

**Cipher suite:** Defined in `server/src/opaque.rs` as `DefaultCipherSuite`. The suite composes:

| Component | Value |
|---|---|
| OPRF group | `Ristretto255` |
| Key exchange | `TripleDh<Ristretto255, Sha512>` |
| Key stretching function (KSF) | Argon2 (via `opaque_ke::ksf::Argon2`) |

The cipher suite is bound to every stored user registration record. Changing the suite, the crate major version, or the KSF parameters invalidates all existing user registrations. Do not change without a spec revision and documented migration path.

**Server setup persistence:** The `ServerSetup` is serialized to the file at `OPAQUE_OPRF_KEY_PATH` on first startup and reused on every subsequent startup. The file must have `0600` permissions on Unix. Loss of this file invalidates all existing user registrations — there is no recovery path.

**OPRF seed rotation:** The OPRF seed is a long-lived secret. It is not rotated on a schedule. Rotation occurs only on suspected compromise or cryptographic migration, and requires forced re-registration of all users. There is no graceful rotation path in V1.

The multi-seed mechanism from the 2026 OPAQUE-with-OPRF-key-rotation paper is reserved for V2, contingent on `opaque-ke` exposing multi-seed support. `opaque-ke` 4.0.1 added remote OPRF seed support, which is a prerequisite for that work.

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
| `BACKUP_ENABLED` | `true` | Master switch |
| `BACKUP_PATH` | `/data/backups` | Backup destination |
| `BACKUP_INTERVAL_HOURS` | `24` | Snapshot interval |
| `BACKUP_RETENTION_COUNT` | `30` | Snapshots to retain |
| `BACKUP_INCLUDE_ATTACHMENTS` | `false` | Include encrypted blobs |

**Backup encryption key:** Derived from the same root secret as the OPRF seed via HKDF with a distinct `info` string:

```
root_secret = random 32 bytes, stored at OPAQUE_OPRF_KEY_PATH
oprf_seed   = HKDF(root_secret, info="opaque-oprf-v1")
backup_key  = HKDF(root_secret, info="backup-encryption-v1")
```

One root secret, two derived keys. Compromise of the backup key does not compromise the OPRF, and vice versa. Rotation of the root secret requires re-registration of all users and re-encryption of all backups.

**Restore procedure:** CLI only. `server restore --from <path> --confirm`. The API can list backups and trigger new ones but cannot restore them. Restore requires shell access and explicit confirmation.

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

| Variable | Default | Notes |
|---|---|---|
| `PUSH_ENABLED` | `true` | Master switch |
| `PUSH_VAPID_PUBLIC_KEY` | `auto` | Generated on first startup |
| `PUSH_VAPID_PRIVATE_KEY` | `auto` | Generated on first startup |
| `PUSH_APNS_KEY` | — | Required to enable iOS push |
| `PUSH_APNS_KEY_ID` | — | Required to enable iOS push |
| `PUSH_APNS_TEAM_ID` | — | Required to enable iOS push |
| `PUSH_FCM_SERVICE_ACCOUNT_JSON` | — | Required to enable Android push |
| `PUSH_GATEWAY_URL` | — | UnifiedPush-compatible endpoint |

**VAPID key management:** VAPID keys are generated on first startup and stored in `instance_config`. They are included in every backup. If regenerated (via admin API only), all existing `push_subscriptions` are marked revoked, an audit event is logged, and clients re-register on next session. There is no silent recovery.

**Push suppression:** Before sending a push, the server checks whether the target device has a recent live session for the target conversation. If it does, the push is suppressed for that device. This is per-device, not per-user — a user with phone and laptop receives phone push even if the laptop is active.

### 5.11 ALTCHA

```env
ALTCHA_ENABLED=true
ALTCHA_HMAC_SECRET=auto
ALTCHA_ALGORITHM=PBKDF2/SHA-256
ALTCHA_COST=5000
```

| Variable | Default | Notes |
|---|---|---|
| `ALTCHA_ENABLED` | `true` | Master switch. When `false`, the challenge endpoint returns 404 and registration skips validation. |
| `ALTCHA_HMAC_SECRET` | `auto` | Generated on first startup and stored in `instance_config` under `altcha_hmac_secret`. |
| `ALTCHA_ALGORITHM` | `PBKDF2/SHA-256` | KDF algorithm. |
| `ALTCHA_COST` | `5000` | KDF iteration count. |

**Crate:** `altcha` 0.2.0 or later. Version 0.1.0 is not permitted.

**Effort mode:** Deterministic only. The HMAC signature secret is set on both challenge creation and verification. Probabilistic mode and deterministic mode without the HMAC secret are not permitted. This restriction exists because of the July 2026 ALTCHA advisory that affected the fallback verification branch.

**Purpose:** ALTCHA provides bot resistance on registration endpoints without third-party services, user tracking, or image puzzles. It is a validation layer that runs before the OPAQUE handshake. It does not modify the OPAQUE protocol.

**Protection scope:** ALTCHA protects `POST /api/v1/auth/register/start` and `POST /api/v1/auth/register/finish`. It does not protect login, invite redemption, or any other endpoint.

**Secret persistence:** The HMAC secret is stored in `instance_config`. Loss of the secret invalidates in-flight challenges only — no persistent state depends on it.

### 5.12 Moderation

```env
MODERATION_MODE=messenger
```

| Variable | Default | Options |
|---|---|---|
| `MODERATION_MODE` | `messenger` | `messenger`, `discord` |

### 5.13 Invite Defaults

```env
INVITE_DEFAULT_USES=1
INVITE_EXPIRY_DAYS=0
INVITE_CODE_LENGTH=8
INVITE_LIMITED_MAX_USES=10
INVITE_LIMITED_MAX_OPEN=50
```

| Variable | Default | Notes |
|---|---|---|
| `INVITE_DEFAULT_USES` | `1` | `0` = unlimited |
| `INVITE_EXPIRY_DAYS` | `0` | `0` = never expires |
| `INVITE_CODE_LENGTH` | `8` | Crockford Base32 length |
| `INVITE_LIMITED_MAX_USES` | `10` | Max `max_uses` for `invite.limited` users |
| `INVITE_LIMITED_MAX_OPEN` | `50` | Max open invites for `invite.limited` users |

### 5.14 Safety Numbers

```env
SAFETY_NUMBER_MODE=warn
```

| Variable | Default | Options |
|---|---|---|
| `SAFETY_NUMBER_MODE` | `warn` | `warn`, `block`, `off` |

### 5.15 Audit Log

```env
AUDIT_RETENTION_DAYS=90
```

| Variable | Default | Notes |
|---|---|---|
| `AUDIT_RETENTION_DAYS` | `90` | Entries older than this are pruned daily. `0` disables pruning. |

### 5.16 Cleanup Scheduler

```env
CLEANUP_ENABLED=true
CLEANUP_INTERVAL_MINUTES=60
CLEANUP_STARTUP_DELAY_SECS=30
```

| Variable | Default | Notes |
|---|---|---|
| `CLEANUP_ENABLED` | `true` | Master switch |
| `CLEANUP_INTERVAL_MINUTES` | `60` | Cycle interval |
| `CLEANUP_STARTUP_DELAY_SECS` | `30` | Delay before first cycle |

### 5.17 Sockudo

```env
SOCKUDO_URL=http://sockudo:6001
SOCKUDO_APP_ID=chat
SOCKUDO_APP_KEY=auto
SOCKUDO_APP_SECRET=auto
```

| Variable | Default | Notes |
|---|---|---|
| `SOCKUDO_URL` | — | **Required.** Sockudo HTTP API base. Internal URL; not exposed to clients. |
| `SOCKUDO_APP_ID` | `chat` | App identifier |
| `SOCKUDO_APP_KEY` | `auto` | Generated if `auto`, stored in `instance_config` |
| `SOCKUDO_APP_SECRET` | `auto` | Generated if `auto`, stored in `instance_config` |

The **external** WebSocket URL is derived from `APP_URL`:

```
wss_url = APP_URL.replace("https://", "wss://") + "/realtime"
```

Clients connect to the external WSS URL. The reverse proxy terminates TLS and forwards plain WebSocket frames to `SOCKUDO_URL`.

### 5.18 GDPR

```env
DATA_RETENTION_DAYS=0
EXPORT_RATE_LIMIT_HOURS=24
```

| Variable | Default | Notes |
|---|---|---|
| `DATA_RETENTION_DAYS` | `0` | `0` = retain forever. When set, server-visible message metadata is purged after this window. |
| `EXPORT_RATE_LIMIT_HOURS` | `24` | Minimum interval between data export requests |

### 5.19 TLS Termination

TLS terminates at a **reverse proxy**, not in the Axum process. The server listens on plain HTTP internally and relies on the proxy to present a valid certificate to clients.

**Coolify deployments:** Traefik is the edge proxy. Certificates are provisioned automatically via Let's Encrypt. No manual TLS configuration is required.

**Manual deployments:** Caddy is the recommended proxy. A minimal `Caddyfile`:

```
chat.example.com {
    reverse_proxy server:8080
    reverse_proxy /realtime* sockudo:6001
}
```

Caddy provisions certificates automatically. Nginx and other proxies work equally well but require manual certificate management.

**Client-facing URLs:**
- The SPA loads from `APP_URL` (HTTPS).
- API requests go to `APP_URL/api/v1/*` (HTTPS).
- WebSocket connections go to `wss://<APP_URL host>/realtime` (WSS).

The Axum server never sees the client's TLS connection directly. It sees plain HTTP from the proxy on the internal Docker network.

**What the application enforces:**
1. `APP_URL` must begin with `https://` when `APP_ENV=production`. Startup fails otherwise.
2. Requests with `X-Forwarded-Proto: http` are redirected to HTTPS with a 301 when `TRUST_PROXY=true`.
3. HSTS is sent on HTTPS responses.

See §8.0 for the middleware implementation.

---

## 6. Client Interface Contract

This section describes what the **server requires from clients**. It is not an implementation guide. Client code lives in `client/` and has its own specification.

### 6.1 Authentication

- Clients send `Authorization: Bearer <session_token>` on every authenticated request.
- The session token is obtained from `POST /api/v1/auth/login/finish`.
- Tokens are 43-character base64url strings. Clients must not parse or modify them.

### 6.2 OPAQUE Handshake Order

- Registration: `POST /auth/register/start` then `POST /auth/register/finish`.
- Login: `POST /auth/login/start` then `POST /auth/login/finish`.
- Clients must complete both rounds within 5 minutes or the correlation state expires.

### 6.3 ALTCHA Payload Format

- Clients submit a base64-encoded JSON payload in the `altcha` field.
- The payload is produced by the ALTCHA widget after solving a challenge from `GET /auth/register/challenge`.

### 6.4 MLS Message Envelope

- Clients encrypt and decrypt MLS messages locally. The server never inspects MLS state.
- Message payloads are base64-encoded MLS ciphertexts.
- Padding to fixed buckets (256 B, 1 KB, 4 KB, 16 KB) is a client responsibility.

### 6.5 Attachments

- Clients encrypt attachments before upload with a per-file ephemeral key.
- The blob is content-addressed: the SHA-256 of the ciphertext is the blob ID.
- The ephemeral key and nonce are embedded in the MLS message envelope.
- Clients verify the SHA-256 hash of the downloaded blob before decrypting.

### 6.6 Room Membership

- Room membership is server-visible at the user level (`room_members`).
- MLS leaf-level membership is client-visible only.

### 6.7 Push Subscriptions

- Clients register push subscriptions via `POST /users/me/push-subscriptions`.
- The `platform` field must be one of `web`, `ios`, `android`, `desktop`.
- Web Push subscriptions include `endpoint`, `p256dh`, `auth`, and `browser_id`.
- Native subscriptions include `push_token` only.

### 6.8 Safety Numbers

- Clients compute and display safety numbers out-of-band.
- The server never sees safety numbers.

### 6.9 WebSocket Connection

- Clients connect to the URL advertised in `GET /api/v1/capabilities` (`websocket_url`).
- In production this is always `wss://`.
- In development this may be `ws://`.

### 6.10 Client Capability Requirements

The server advertises capabilities via `GET /api/v1/capabilities`. Clients must read this on startup and degrade gracefully if a capability is unavailable.

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

**Username:** immutable, unique, 3–32 characters, alphanumeric plus underscore and dash.

**Display name:** mutable, 1–64 characters, any printable Unicode. Server-visible.

**`deleted_at`:** Set when the user requests account deletion. The row is anonymised but retained for referential integrity.

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
```

### 7.5 Attachments

```sql
CREATE TABLE attachments (
    id           TEXT PRIMARY KEY,
    room_id      TEXT NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    uploader_id  TEXT NOT NULL REFERENCES users(id),
    padded_size  INTEGER NOT NULL,
    blob_path    TEXT NOT NULL,
    created_at   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

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

**Platform separation:** Web, iOS, Android, and desktop are separate rows.

**Per-browser instance:** A single device can have multiple web subscriptions (Chrome, Firefox, Safari).

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
| `OPAQUE_OPRF_KEY_PATH` | OPAQUE `ServerSetup` and root backup secret | Required. Loss invalidates all user registrations and all backups. |
| `BACKUP_PATH` | Backup snapshots | Populated by the backup scheduler |
| Blob storage root | Encrypted attachment blobs | Content-addressed; filesystem or S3 |

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

#### Static SPA Hosting

When `CLIENT_STATIC_DIR` is set, the Axum server serves the compiled Coralite SPA from that directory using `ServeDir` with an SPA fallback:

```rust
let spa = ServeDir::new(&config.client_static_dir)
    .not_found_service(ServeFile::new(format!("{}/index.html", config.client_static_dir)));

let app = Router::new()
    .nest("/api/v1", api_routes)
    .route("/health", get(health))
    .route("/ready", get(ready))
    .fallback_service(spa)
    .layer(middleware::from_fn_with_state(state.clone(), enforce_https))
    .with_state(state);
```

The `fallback_service` runs last. API routes and health checks are matched first. Deep links return `index.html` and the client-side router takes over.

In development (`CLIENT_STATIC_DIR` unset), static serving is disabled and the Coralite dev server runs separately on port 3000.

#### HTTPS Enforcement

When `APP_ENV=production`, a middleware layer enforces HTTPS semantics:

```rust
async fn enforce_https(
    State(config): State<Arc<Config>>,
    req: Request,
    next: Next,
) -> Response {
    let is_prod = config.app_env == "production";

    if !is_prod {
        return next.run(req).await;
    }

    // Health and readiness are exempt — orchestrators probe over plain HTTP.
    let path = req.uri().path();
    if path == "/health" || path == "/ready" {
        return next.run(req).await;
    }

    let scheme = if config.trust_proxy {
        req.headers()
            .get("x-forwarded-proto")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("https")
    } else {
        "https"  // no proxy trusted; assume proxy terminated TLS correctly
    };

    if scheme == "http" {
        let location = format!(
            "{}{}",
            config.app_url,
            req.uri().path_and_query().map(|pq| pq.as_str()).unwrap_or("/")
        );
        return Response::builder()
            .status(StatusCode::MOVED_PERMANENTLY)
            .header("Location", location)
            .body(Body::empty())
            .unwrap_or_else(|_| Response::new(Body::empty()));
    }

    let mut response = next.run(req).await;

    if config.hsts_max_age > 0 {
        let mut hsts = format!("max-age={}", config.hsts_max_age);
        if config.hsts_include_subdomains {
            hsts.push_str("; includeSubDomains");
        }
        if let Ok(value) = HeaderValue::from_str(&hsts) {
            response.headers_mut().insert(
                HeaderName::from_static("strict-transport-security"),
                value,
            );
        }
    }

    response
}
```

**Behaviour summary:**

- In development, no redirect, no HSTS.
- In production with `TRUST_PROXY=true`, requests with `X-Forwarded-Proto: http` get a 301 redirect to the HTTPS URL.
- In production with `TRUST_PROXY=false`, the server assumes the proxy is doing its job and does not redirect. A startup warning is logged if `TRUST_PROXY=false` in production.
- `/health` and `/ready` are exempt — they are probed over plain HTTP by orchestrators on the internal network.

**Reverse proxy requirement:** The reverse proxy MUST be configured to:
1. Terminate TLS with a valid certificate.
2. Set `X-Forwarded-Proto: https` on requests it forwards.
3. Proxy `/api/*` and static assets to the Axum server on port 8080.
4. Proxy `/realtime` (WebSocket) to Sockudo.

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
| POST | `/auth/login/finish` | OPAQUE login finish → session token |
| POST | `/invites/redeem` | Validate and consume a server invite |
| GET | `/invites/:code` | Public invite code validation |

**`GET /capabilities` response:**

```json
{
  "version": "0.1.0",
  "calling": false,
  "push_vapid_public_key": null,
  "websocket_url": "wss://chat.example.com/realtime",
  "altcha": {
    "enabled": true,
    "algorithm": "PBKDF2/SHA-256",
    "cost": 5000
  },
  "safety_number_mode": "warn",
  "moderation_mode": "messenger"
}
```

The `websocket_url` is derived from `APP_URL`:
- Production: `APP_URL` with `https://` replaced by `wss://`, plus `/realtime`.
- Development: `ws://localhost:6001` or the value of `SOCKUDO_PUBLIC_URL` if set.

### 8.2 User

| Method | Path | Purpose |
|---|---|---|
| POST | `/auth/logout` | Revoke session |
| GET | `/users/me` | Current user |
| PATCH | `/users/me` | Update profile |
| DELETE | `/users/me` | Delete account (GDPR Art. 17) |
| GET | `/users/me/export` | Export data (GDPR Art. 20) |
| POST | `/users/lookup` | Exact username_hash lookup |
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
| DELETE | `/rooms/:id` | Delete room (owner only) |
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

### 8.5 MLS & Messaging

| Method | Path | Purpose |
|---|---|---|
| POST | `/keypackages` | Upload KeyPackage batch |
| GET | `/keypackages/count` | Count unconsumed packages |
| POST | `/keypackages/claim` | Atomically claim a KeyPackage |
| GET | `/welcomes` | List pending welcomes |
| POST | `/welcomes/:id/consume` | Mark welcome consumed |
| POST | `/rooms/:id/messages` | Publish MLS ciphertext via Sockudo |
| GET | `/rooms/:id/epoch` | Current epoch and sequence |

### 8.6 Attachments

| Method | Path | Purpose |
|---|---|---|
| POST | `/attachments` | Upload encrypted blob |
| GET | `/attachments/:hash` | Download by content hash |
| DELETE | `/attachments/:hash` | Delete (uploader only) |

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

---

## 10. Client Contract Matrix

This matrix describes **server behaviour** for features the client relies on. Client-side implementation is not specified here.

| Feature | Server behaviour |
|---|---|
| Text messaging | Relays MLS ciphertext via Sockudo; never inspects content |
| Attachments | Content-addressed blob storage; server stores ciphertext only |
| Push notifications | Accepts subscriptions for `web`, `ios`, `android`, `desktop`; suppresses per-device based on recent live session |
| Multi-device | Supports up to `devices_per_user` (default 10); each device has an independent `client_id` |
| Safety numbers | Advisory only; server never sees them |
| ALTCHA | Challenge endpoint on registration only |
| Calling (V2) | Advertised as `calling: false` in `/capabilities` |
| Data export | Returns ZIP of ciphertext and account metadata |
| Account deletion | Anonymises user record; cascades session, device, KeyPackage, MLS Remove |
| WebSocket | Advertised as `wss://` in production, `ws://` in development |
| HTTPS | Enforced by reverse proxy; server redirects and sends HSTS |

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
| 7 | Room CRUD + membership + ownership transfer | 5 |
| 8 | Room invites | 7 |
| 9 | KeyPackage upload, claim, quota enforcement | 7 |
| 10 | Welcome routing + MLS epoch linearization | 9 |
| 11 | Sockudo integration (publish, subscribe, history) | 10 |
| 12 | Attachment upload, download, content addressing, retention pruning | 11 |
| 13 | Admin web UI (Coralite) | 6a |
| 14 | User web UI (Coralite) | 11 |
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
| Server sees profile contents | Never (profile_blob is E2E encrypted) |
| Server sees display name | Yes (required for push notifications) |
| Server sees username | Yes (required for lookup) |
| Server sees attachment contents | Never (blobs are encrypted before upload) |
| Server can MITM group joins | Only if safety numbers are not verified |
| Server can decrypt past messages | No (forward secrecy via MLS) |
| Server can decrypt future messages after compromise | No (post-compromise security via MLS) |
| Web client is protected against compromised server | No. Web code is host-delivered. |
| Native client is protected against compromised server | Yes. Signed binaries with independent update channel. |
| Registration endpoints are bot-resistant | Yes. ALTCHA PoW v2 in deterministic mode. |
| Registration endpoints use third-party CAPTCHA | Never. ALTCHA is fully self-hosted. |
| Server can force peers to delete local copies | No. Tombstones are advisory. |
| Production traffic is HTTPS | Yes. Enforced by reverse proxy with HSTS. |
| HTTP is permitted in production | No. `APP_URL` must use `https://`; server refuses to start otherwise. |
| TLS terminates inside the Axum process | No. Terminates at reverse proxy. |

---

## 13. Backups and Disaster Recovery

- Automatic snapshots of SQLite, OPRF key, and optionally attachment blobs every `BACKUP_INTERVAL_HOURS`.
- Snapshots are encrypted with `backup_key` derived from the root secret.
- Retention: `BACKUP_RETENTION_COUNT` (default 30).
- Restore is CLI-only: `server restore --from <path> --confirm`.
- The API lists backups and triggers manual backups but cannot restore them.

**Backup exclusions:** Client-side keys are never backed up. If a client's device is lost and the recovery code is lost, the data is unrecoverable. This is a documented property of E2EE, not a bug.

---

## 14. GDPR Compliance

The controller (the person running the instance) is responsible for responding to data subject requests. The software provides the technical mechanisms. Legal obligations rest with the controller.

### 14.1 Data Subject Rights

| Right | Article | Server mechanism |
|---|---|---|
| Right of access | Art. 15 | `GET /users/me/export` |
| Right to rectification | Art. 16 | `PATCH /users/me` (display name, profile) |
| Right to erasure | Art. 17 | `DELETE /users/me` |
| Right to restriction | Art. 18 | Account disable (admin endpoint) |
| Right to data portability | Art. 20 | Machine-readable ZIP export |
| Right to object | Art. 21 | Account deletion |

### 14.2 Account Deletion — `DELETE /api/v1/users/me`

**Request body:**

```json
{
  "confirm": "DELETE"
}
```

**Preconditions:**

- Session must have been created within the last 5 minutes.
- `confirm` must exactly equal `"DELETE"`.
- User must not be the last user with the `owner` role.

**Behavior:** Single transaction that:

1. Deletes all unconsumed KeyPackages.
2. Deletes all push subscriptions.
3. Queues MLS Removes for every room the user is a member of.
4. Deletes the `recovery_vault` record if it exists.
5. Deletes all devices (cascades sessions).
6. Deletes any remaining sessions without a device.
7. Anonymises the user row:
   - `username = "deleted_<random>"`
   - `username_hash = <random>`
   - `display_name = NULL`
   - `profile_blob = NULL`
   - `opaque_registration = <random 32 bytes>`
   - `identity_pubkey = ""`
   - `max_file_size_bytes = NULL`
   - `disabled_at = now()`
   - `deleted_at = now()`
8. Removes all room memberships.
9. Writes an audit entry with action `user.delete`.

Returns HTTP 204.

**Why anonymise:** `users.id` is referenced by messages, attachments, and room memberships. Hard delete cascades and destroys other users' data. Anonymisation removes personal data while preserving referential integrity.

### 14.3 Data Export — `GET /api/v1/users/me/export`

Returns a ZIP archive with:

| File | Contents |
|---|---|
| `profile.json` | Username, display name, created_at, roles |
| `devices.json` | Device list |
| `sessions.json` | Session history |
| `rooms.json` | Room membership |
| `messages.json` | Server-visible message metadata (ciphertext) |
| `audit.json` | Audit entries where the user is the actor |
| `README.txt` | Format documentation and ciphertext limitation |

**Limitation:** The export contains ciphertext, not plaintext. The server cannot decrypt.

**Rate limit:** One export per `EXPORT_RATE_LIMIT_HOURS` (default 24).

### 14.4 Retention Policy

Instance config `DATA_RETENTION_DAYS` (default `0` = forever). When set, sessions, messages, and attachments older than the window are purged.

### 14.5 Tombstone Semantics

Deleted messages are tombstoned. The server deletes its copy from Sockudo history. Peers receive a tombstone and delete their local copy if their client honours it. The server cannot force peer deletion.

### 14.6 Metadata Minimisation

| Field | Minimisation |
|---|---|
| `devices.client_id` | Random 128-bit, not a hardware identifier |
| `sessions.last_seen_at` | Coarsened to hour |
| `messages.created_at` | Coarsened to minute |
| `audit_log.metadata` | Never includes content or IPs |
| IP addresses | Not stored in schema; logs rotated |

### 14.7 Controller Obligations

The controller is responsible for:

- Publishing a privacy policy
- Providing a DPA if operating commercially
- Documenting server location and sub-processors

### 14.8 Audit Actions

| Action | Triggered by |
|---|---|
| `user.delete` | `DELETE /users/me` |
| `user.export` | `GET /users/me/export` |
| `config.update` | Admin config change |
| `secret.update` | Admin secret rotation |
| `limits.update` | Admin limits change |
| `role.grant` / `role.revoke` | Admin role change |
| `invite.create` / `invite.revoke` | Invite management |
| `device.revoke` | Device revocation |
| `bootstrap.owner` | First user registration |
| `vapid.rotate` / `altcha.rotate` | Secret rotation |

---

## 15. Document Status

This is the contract for the server side of the system. Every Jules task references this document. If a task conflicts with this spec, the task is wrong and must be revised. If a feature is missing from this spec, it does not exist yet — it must be added here first, then built.

The spec is frozen for V1. New features go into V2. Bug fixes and clarifications are amendments, not revisions.
