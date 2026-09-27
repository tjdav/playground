# Application Specification v1.0

> **Status:** Frozen — source of truth for all Jules tasks.
> **Stack:** Sockudo + Axum + SQLite + Wire CoreCrypto + Coralite + Tauri
> **Deployment:** Single VPS, Docker-based, self-hosted.

Any change to this document requires an explicit revision and a corresponding task update. Frozen means: no feature additions, no schema changes, no API surface changes without a spec revision and a documented migration path.

---

## 1. Product Summary

A self-hosted, end-to-end encrypted group messaging system with MLS-grade forward secrecy and post-compromise security. Clients run in browsers (WASM), on mobile (Capacitor), and on desktop (Tauri). The server is an untrusted delivery service that never sees plaintext, keys, or meaningful metadata.

| Layer | Technology |
|---|---|
| Delivery | Sockudo (WebSocket, Pusher v7, Protocol V2 history) |
| API | Axum (Rust) |
| Database | SQLite (embedded in the Axum process) |
| Blob storage | Filesystem or S3 (content-addressed) |
| MLS engine | Wire CoreCrypto 10.5.2 |
| Auth | OPAQUE (aPAKE) via `opaque-ke` 4.0.1 |
| Bot protection | ALTCHA Proof-of-Work v2 via `altcha` 0.2.0 |
| Frontend | Coralite |
| Desktop | Tauri |

---

## 2. Scope

### 2.1 In Scope — V1

- Invite-only registration with configurable codes
- OPAQUE authentication
- ALTCHA proof-of-work bot protection on registration
- Multi-device support (default 10 devices per user)
- Rooms as the unit of conversation (1:1 = 2-member room)
- MLS group key agreement via Wire CoreCrypto
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
- Third-party CAPTCHA services (reCAPTCHA, hCaptcha, Turnstile)

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

---

## 5. Environment Variables

Follows Coolify conventions. All variables are optional unless marked **required**. Defaults are applied at startup.

### 5.1 Application

```env
APP_ENV=production
APP_URL=https://chat.example.com
APP_NAME=Encrypted Chat
LOG_LEVEL=info
```

| Variable | Default | Notes |
|---|---|---|
| `APP_ENV` | `production` | `production` or `development` |
| `APP_URL` | — | **Required.** Public URL of the deployment. |
| `APP_NAME` | `Encrypted Chat` | Display name |
| `LOG_LEVEL` | `info` | `trace`, `debug`, `info`, `warn`, `error` |

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

### 5.7 CORS and CSRF

```env
CORS_ALLOWED_ORIGINS=
CSRF_PROTECTION=none
```

| Variable | Default | Notes |
|---|---|---|
| `CORS_ALLOWED_ORIGINS` | empty (dev: `*`) | Comma-separated list of allowed origins |
| `CSRF_PROTECTION` | `none` | `none`, `header`, `double-submit` |

In production, `CORS_ALLOWED_ORIGINS` must be set. In development, it defaults to `*` with a startup warning.

CSRF protection is unnecessary for Bearer-token auth. The `CSRF_PROTECTION` variable exists to support future cookie-based sessions. V1 uses `none`.

The server rejects requests with an `Origin` header not in the allowed list, even for non-CORS requests. Origin mismatches are logged at `warn` level.

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
| `ALTCHA_ALGORITHM` | `PBKDF2/SHA-256` | KDF algorithm. Use only algorithms supported by the `altcha` crate. |
| `ALTCHA_COST` | `5000` | KDF iteration count. Higher = more client work, same server cost. |

**Crate:** `altcha` 0.2.0 or later. Version 0.1.0 is not permitted.

**Effort mode:** Deterministic only. The HMAC signature secret is set on both challenge creation and verification. Probabilistic mode and deterministic mode without the HMAC secret are not permitted. This restriction exists because of the July 2026 ALTCHA advisory that affected the fallback verification branch.

**Purpose:** ALTCHA provides bot resistance on registration endpoints without third-party services, user tracking, or image puzzles. It is a validation layer that runs before the OPAQUE handshake. It does not modify the OPAQUE protocol.

**Protection scope:** ALTCHA protects `POST /api/v1/auth/register/start` and `POST /api/v1/auth/register/finish`. It does not protect login (Phase 3b), invite redemption (Phase 5), or any other endpoint.

**Client integration:** The frontend embeds the `<altcha-widget>` web component and points it at `GET /api/v1/auth/register/challenge`. The widget solves the challenge in a Web Worker and populates a hidden `altcha` input with the base64-encoded JSON payload.

**Secret persistence:** The HMAC secret is stored in `instance_config`. Loss of the secret invalidates in-flight challenges only — no persistent state depends on it. Challenge rotation is safe at any time.

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
```

| Variable | Default | Notes |
|---|---|---|
| `INVITE_DEFAULT_USES` | `1` | `0` = unlimited |
| `INVITE_EXPIRY_DAYS` | `0` | `0` = never expires |
| `INVITE_CODE_LENGTH` | `8` | Base32 length |

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
| `AUDIT_RETENTION_DAYS` | `90` | Entries older than this are pruned daily |

### 5.16 Sockudo

```env
SOCKUDO_URL=http://sockudo:6001
SOCKUDO_APP_ID=chat
SOCKUDO_APP_KEY=auto
SOCKUDO_APP_SECRET=auto
```

| Variable | Default | Notes |
|---|---|---|
| `SOCKUDO_URL` | — | **Required.** Sockudo HTTP API base |
| `SOCKUDO_APP_ID` | `chat` | App identifier |
| `SOCKUDO_APP_KEY` | `auto` | Generated if `auto`, stored in `instance_config` |
| `SOCKUDO_APP_SECRET` | `auto` | Generated if `auto`, stored in `instance_config` |

If the credentials are missing from `instance_config` but Sockudo is reachable, the server refuses to start and logs an error directing the operator to complete setup.

---

## 6. Client Storage Model

### 6.1 Native (iOS, Android, Tauri)

- **Database:** SQLCipher with key from Secure Enclave (iOS), Keystore (Android), or OS keychain (Tauri).
- **MLS state:** Stored in SQLCipher. CoreCrypto manages its own encrypted state.
- **Identity key:** Wrapped by hardware-backed keystore.

### 6.2 Web (Browser)

- **Database:** WASM SQLite (`wa-sqlite` or `@sqlite.org/sqlite-wasm`), unencrypted at the SQL layer.
- **Encryption model:** Application-layer. All sensitive blobs (identity key, MLS state, wrapped recovery key, message plaintext cache) are encrypted with WebCrypto before write.
- **CoreCrypto database key:** Derived from the OPAQUE session key via HKDF. Cached in memory for the session lifetime. Never persisted.
- **Persistence:** OPFS preferred, IndexedDB fallback.
- **Rationale:** SQLCipher does not compile to WASM. Application-layer encryption provides equivalent security because everything sensitive is already ciphertext before touching disk.

### 6.3 Local Key Derivation (Web)

```
session_key = OPAQUE session key (32 bytes, from login)
db_key      = HKDF-SHA256(session_key, salt="mls-db-v1", info="corecrypto-db")
```

`db_key` is held in a JavaScript `Uint8Array`, cleared with `.fill(0)` on logout or session expiry.

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
    disabled_at         DATETIME
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
    id          TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id   TEXT REFERENCES devices(id) ON DELETE CASCADE,
    token_hash  TEXT NOT NULL UNIQUE,
    created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at  DATETIME NOT NULL,
    revoked_at  DATETIME
);
```

**Username:** immutable, unique, 3–32 characters, alphanumeric plus underscore and dash. Used as the lookup key for room membership.

**Display name:** mutable, 1–64 characters, any printable Unicode. Server-visible (so push notifications can include it). If empty or `NULL`, clients render `username`.

**`opaque_registration`:** serialized `ServerRegistration<DefaultCipherSuite>` produced by the OPAQUE registration handshake. Binary format bound to the `opaque-ke` version and cipher suite documented in §5.8.

**`profile_blob`:** E2E encrypted. Contains bio, avatar URL, and any other private profile metadata. The server never sees its contents.

### 7.2 Invites

```sql
CREATE TABLE server_invites (
    id            TEXT PRIMARY KEY,
    code          TEXT NOT NULL UNIQUE,
    created_by    TEXT NOT NULL REFERENCES users(id),
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
```

Welcomes expire 7 days after creation.

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

**Platform separation:** Web, iOS, Android, and desktop are separate rows. Each uses a different transport.

- **Web rows** populate `endpoint`, `p256dh`, `auth`, `browser_id`. `push_token` is `NULL`.
- **iOS/Android rows** populate `push_token`. `endpoint`, `p256dh`, `auth`, `browser_id` are `NULL`.
- **Desktop rows** may use a native OS token or IPC channel.

**Per-browser instance:** A single device can have multiple web subscriptions (Chrome, Firefox, Safari). Each is a separate row keyed on `(user_id, browser_id)`.

**Cleanup:** Subscriptions where `revoked_at IS NOT NULL AND revoked_at < now() - 30 days` are deleted by the hourly cleanup job.

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

`instance_config` stores VAPID keys, Sockudo credentials, the ALTCHA HMAC secret, and any other auto-generated per-instance secrets.

### 7.8 State Outside the Database

Not all persistent state lives in SQLite. The following files must be preserved across restarts and included in backups:

| Path | Purpose | Notes |
|---|---|---|
| `OPAQUE_OPRF_KEY_PATH` | OPAQUE `ServerSetup` and root backup secret | Required. Loss invalidates all user registrations and all backups. |
| `BACKUP_PATH` | Backup snapshots | Populated by the backup scheduler |
| Blob storage root | Encrypted attachment blobs | Content-addressed; filesystem or S3 |

The OPAQUE `ServerSetup` file must have `0600` permissions on Unix. It must be readable only by the server process.

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

`error` is a stable machine-readable code. `message` is human-readable. `details` is optional and used for field-level validation errors.

### 8.1 Public

| Method | Path | Purpose |
|---|---|---|
| GET | `/health` | Liveness. Returns 200 if process is alive. Never touches DB. |
| GET | `/ready` | Readiness. Checks DB connectivity, OPRF key loaded, Sockudo reachable. |
| GET | `/capabilities` | Feature + VAPID public key + ALTCHA status advertisement |
| GET | `/auth/register/challenge` | ALTCHA challenge for registration. Returns 404 when `ALTCHA_ENABLED=false`. |
| POST | `/auth/register/start` | OPAQUE registration start. Requires ALTCHA payload when enabled. |
| POST | `/auth/register/finish` | OPAQUE registration finish. Requires ALTCHA payload when enabled. Requires invite code if users exist. |
| POST | `/auth/login/start` | OPAQUE login start |
| POST | `/auth/login/finish` | OPAQUE login finish → session token |
| POST | `/invites/redeem` | Validate and consume a server invite |

**`GET /capabilities` response includes:**

```json
{
  "version": "0.1.0",
  "calling": false,
  "push_vapid_public_key": null,
  "altcha": {
    "enabled": true,
    "algorithm": "PBKDF2/SHA-256",
    "cost": 5000
  },
  "safety_number_mode": "warn",
  "moderation_mode": "messenger"
}
```

### 8.2 User

| Method | Path | Purpose |
|---|---|---|
| POST | `/auth/logout` | Revoke session |
| GET | `/users/me` | Current user (includes `display_name`) |
| PATCH | `/users/me` | Update `display_name` and/or `profile_blob`. Rejects `username`. |
| POST | `/users/lookup` | Exact username_hash lookup |
| GET | `/users/me/devices` | List devices |
| DELETE | `/users/me/devices/:id` | Revoke device (cascades: sessions, KeyPackages, MLS Remove) |
| GET | `/users/me/sessions` | List sessions |
| DELETE | `/users/me/sessions/:id` | Revoke session |
| POST | `/users/me/push-subscriptions` | Register a push subscription |
| DELETE | `/users/me/push-subscriptions/:id` | Revoke a push subscription |

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
| POST | `/admin/vapid/rotate` | Rotate VAPID keys (revokes all push subscriptions) |
| POST | `/admin/altcha/rotate` | Rotate ALTCHA HMAC secret |

### 8.4 Rooms

| Method | Path | Purpose |
|---|---|---|
| POST | `/rooms` | Create room |
| GET | `/rooms` | List my rooms |
| GET | `/rooms/:id` | Room metadata |
| DELETE | `/rooms/:id` | Delete room (owner only) |
| POST | `/rooms/:id/leave` | Leave room |
| POST | `/rooms/:id/transfer` | Transfer ownership to another member |
| GET | `/rooms/:id/members` | List members |
| POST | `/rooms/:id/members` | Add member (MLS Add + Welcome) |
| DELETE | `/rooms/:id/members/:uid` | Kick |
| POST | `/rooms/:id/members/:uid/promote` | Promote to moderator (Discord mode) |
| POST | `/rooms/:id/members/:uid/demote` | Demote |
| POST | `/rooms/:id/invites` | Create room invite |
| GET | `/rooms/:id/invites` | List room invites |
| DELETE | `/rooms/:id/invites/:code` | Revoke room invite |

### 8.5 MLS & Messaging

| Method | Path | Purpose |
|---|---|---|
| POST | `/keypackages` | Upload KeyPackage batch |
| GET | `/keypackages/count` | Count unconsumed packages |
| POST | `/keypackages/claim` | Atomically claim a user's KeyPackage |
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

The server binary exposes these subcommands in addition to running the HTTP server:

| Command | Purpose |
|---|---|
| `server` | Run the HTTP server (default) |
| `server restore --from <path> --confirm` | Restore from backup. Offline only. Requires shell access. |
| `server migrate` | Run pending migrations and exit |
| `server rotate-vapid` | Rotate VAPID keys and exit |
| `server rotate-altcha` | Rotate ALTCHA HMAC secret and exit |
| `server rotate-oprf --confirm` | Resample the OPRF seed. Invalidates all user registrations. Requires explicit confirmation. |

---

## 10. Client Capability Matrix

| Capability | Web | iOS | Android | Desktop (Tauri) |
|---|---|---|---|---|
| Text messaging | ✅ | ✅ | ✅ | ✅ |
| Attachments | ✅ | ✅ | ✅ | ✅ |
| Push notifications | Web Push | APNs | FCM | Native |
| Hardware key store | ❌ | Secure Enclave | Keystore | OS keychain |
| Database encryption | App-layer (WebCrypto) | SQLCipher | SQLCipher | SQLCipher |
| Safety numbers | ✅ | ✅ | ✅ | ✅ |
| Multi-device | ✅ | ✅ | ✅ | ✅ |
| Recovery | ✅ | ✅ | ✅ | ✅ |
| ALTCHA widget | ✅ | ✅ | ✅ | ✅ |
| Calling (V2) | ✅ | ✅ | ✅ | ✅ |

---

## 11. Phased Build Plan

Each phase is a discrete Jules task with verifiable deliverables.

| Phase | Deliverable | Depends On |
|---|---|---|
| 1 | Axum scaffold + SQLite migrations + health + capabilities endpoints | — |
| 2 | Roles, permissions, user_roles tables + seed data | 1 |
| 3a | OPAQUE registration flow (server setup + start/finish endpoints) | 2 |
| 3a.1 | ALTCHA proof-of-work protection on registration endpoints | 3a |
| 3b | OPAQUE login flow (start/finish endpoints + identity key upload) | 3a.1 |
| 4 | Session management + device registration + device revocation cascade | 3b |
| 5 | Server invite codes (create, redeem, revoke) | 3a.1 |
| 6 | Instance config, limits API, cleanup jobs, `/ready` endpoint | 4 |
| 7 | Room CRUD + membership + ownership transfer | 5 |
| 8 | Room invites | 7 |
| 9 | KeyPackage upload, claim, quota enforcement | 7 |
| 10 | Welcome routing + MLS epoch linearization + Welcome expiry | 9 |
| 11 | Sockudo integration (publish, subscribe, history) | 10 |
| 12 | Attachment upload, download, content addressing, retention pruning | 11 |
| 13 | Admin web UI (Coralite) | 6 |
| 14 | User web UI (Coralite) | 11 |
| 15 | Push notifications (Web Push first) | 14 |
| 16 | APNs and FCM support | 15 |
| 17 | Automatic backups (SQLite + Sockudo) + restore CLI | 6 |
| 18 | Tauri desktop client | 14 |
| 19 | Capacitor iOS client | 14 |
| 20 | Capacitor Android client | 14 |

---

## 12. Security Boundaries

| Boundary | Guarantee |
|---|---|
| Server sees plaintext messages | Never |
| Server sees decryption keys | Never |
| Server sees message metadata | Minimal: room ID, sender client ID, epoch, timestamp (coarsened) |
| Server sees profile contents | Never (profile_blob is E2E encrypted) |
| Server sees display name | Yes (required for push notifications) |
| Server sees username | Yes (required for lookup) |
| Server sees attachment contents | Never (blobs are encrypted before upload) |
| Server can MITM group joins | Only if safety numbers are not verified. Soft warning mitigates. |
| Server can decrypt past messages | No (forward secrecy via MLS) |
| Server can decrypt future messages after compromise | No (post-compromise security via MLS) |
| Web client is protected against compromised server | No. Web code is host-delivered. |
| Native client is protected against compromised server | Yes. Signed binaries with independent update channel. |
| Browser memory is zero-trace | No. V8 string immutability and GC churn prevent it. |
| Native memory is zero-trace | Yes. Deterministic via `zeroize::ZeroizeOnDrop`. |
| Registration endpoints are bot-resistant | Yes. ALTCHA PoW v2 in deterministic mode. |
| Registration endpoints use third-party CAPTCHA | Never. ALTCHA is fully self-hosted. |

---

## 13. Document Status

This is the contract. Every Jules task references this document. If a task conflicts with this spec, the task is wrong and must be revised. If a feature is missing from this spec, it does not exist yet — it must be added here first, then built.

The spec is frozen for V1. New features go into V2. Bug fixes and clarifications are amendments, not revisions.
