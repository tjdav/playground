Server Specification v2.0

Status: Stable — source of truth for all V2 implementation tasks.
Supersedes: Server Specification v1.0 (amended through §16.12).
Scope: Server-side contract only. Client implementation is out of scope and is covered by Client Specification v1.0.
Stack: Sockudo + Axum + SQLite + OPAQUE (opaque-ke 4.0.1) + VOPRF (voprf 0.5.0) + ALTCHA + S3 (or filesystem)
Deployment: Single VPS, Docker-based, self-hosted. Two containers: server (Axum + static SPA hosting) and sockudo. TLS terminates at a reverse proxy.

This document is a breaking revision. Nothing has been deployed; no migration path is required. Where V1 sections are unchanged, they are carried forward verbatim and marked. Where V2 changes them, the change is stated normatively.

Amendments to this document are tracked at §16. The V1 amendment log (§16.1–§16.12 of V1) is preserved in §17 for historical reference.

---

1. Product Summary

A self-hosted, end-to-end encrypted group messaging system with MLS-grade forward secrecy and post-compromise security. The server is an untrusted delivery service that never sees plaintext, keys, or meaningful metadata.

Layer Technology
Delivery Sockudo (WebSocket, Pusher v7)
API Axum (Rust)
Database SQLite (embedded in the Axum process)
Blob storage S3 (default) or filesystem (fallback)
S3 client rust-s3
MLS engine Wire CoreCrypto 10.5.2 (client-side only)
Auth OPAQUE (aPAKE) via opaque-ke 4.0.1
Username lookup VOPRF (Ristretto255-SHA512) via voprf 0.5.0
Bot protection ALTCHA Proof-of-Work v2 via altcha 0.2.0
Attachment encryption C2SP chunked encryption (c2sp.org/chunked-encryption)
Static SPA hosting Axum ServeDir with SPA fallback
Model hosting Instance-local, external shared origin, or proxy-with-cache
TLS termination External reverse proxy (Traefik via Coolify, or Caddy)

V2 architectural changes:

1. The server no longer stores plaintext usernames, display names, or device names. Identity is anchored in an OPRF token.
2. User-scoped state (read state, room order, device names, starred items, preferences) is durable and sequence-synced across a user’s devices.
3. Message editing, reactions, threading, key transparency, link preview proxy, call signaling, sessions, model hosting, starred items, and generic preferences are added.
4. Ambient presence remains refused. Call-scoped co-presence within calls and sessions is permitted and is disclosed only to participants. It is never persisted.

---

2. Scope

2.1 In Scope — V2

Everything in V1 §2.1, plus:

· OPRF-based identity. Usernames, display names, and device names are opaque to the server.
· Recovery flow. Server-generated recovery codes; OPAQUE re-registration.
· User-scoped channels. private-user-{user_id} with durable, sequence-synced state.
· Multi-device sync. Read state, room order, device names, starred items, and preferences propagate across devices via user_seq.
· Generic preferences endpoint. Client-defined key/value preferences synced per user.
· Starred items. User-scoped stars for attachments, messages, and links.
· Message editing. Signed edit chain, edit_window_seconds enforced server-side.
· Reactions. Table, endpoints, aggregation, silent delivery.
· Threading. reply_to on messages.
· Room metadata updates. PATCH /rooms/:id with encrypted metadata.
· Room avatars. POST /users/me/avatar.
· Member pagination. Cursor-based pagination on GET /rooms/:id/members.
· Retention change preview. POST /rooms/:id/retention/preview.
· MLS adds coordination. pending_mls_adds.
· Key transparency. Append-only log with inclusion proofs and auditor signatures.
· Link preview proxy. Opt-in, SSRF-guarded, blind to URLs.
· Call signaling. WebRTC signaling events; TURN credential endpoint.
· Sessions. Persistent, named, room-scoped real-time spaces with in-memory occupancy. Voice hangouts are one session type; watch-together, games, whiteboards, and other types are supported by the same mechanism.
· Model hosting. STT and TTS models, with local, external, and proxy hosting modes.
· Admin surfaces for new V2 features.
· Batched test execution model. See §5.30.

2.2 Presence and Co-Presence

Ambient presence is refused. The server does not track or expose who is online, when they were last seen, or what they are doing across the system. There are no presence channels. sessions.last_seen_at is not user-facing and is coarsened in any admin view. The client spec’s “Last seen” setting is removed.

Call-scoped co-presence is permitted. Within a call or a session, the server knows in memory which room members are participating, in order to route media. This knowledge is disclosed only to participants of the same call or session. It is held in memory only and is never written to any persistent store or external system. Leaving a call or session removes the participant completely; no record remains.

2.3 Out of Scope — V2

· Ambient presence. No online/offline indicators, no presence channels, no last-seen exposure.
· Federation between servers
· Multi-tenancy
· Server-side key escrow
· Plaintext metadata on server
· Email or OAuth registration
· Anonymous accounts
· Plaintext message export
· Username changes
· Third-party CAPTCHA services
· Client-side data deletion
· Forcing peers to delete local copies
· Consent management UI
· TLS termination inside the Axum process
· ACME client inside the server
· MP4 fast-start enforcement
· Multi-range HTTP requests
· Range-restricted presigned URLs
· Message forwarding
· Message pinning
· GIF search
· Multi-node deployments. In-memory call and session occupancy requires a single Axum process. Multi-node requires a shared coordination layer and is out of scope for V2.
· Shared SPA hosting across instances. The SPA bundle is version-coupled to the server. Operators may place a CDN in front of CLIENT_STATIC_DIR for the same version, but the spec does not provide a cross-version shared origin.
· Server-side periodic work. Fetching RSS, polling external APIs, scheduled tasks. These belong to the server spec directly, as discrete endpoints, and are out of scope here.
· Room events for extension sync without a session. Fire-and-forget broadcasts to room members outside of a session are a future addition. See §16.23 for the forward-looking note.

2.4 Breaking Changes from V1

Change V1 V2
users.username Plaintext Removed. Replaced by users.username_token.
users.username_hash Plaintext hash Removed.
users.display_name Plaintext Removed. Replaced by users.encrypted_display.
devices.name Plaintext Removed. Device names are user-scoped sync state.
rooms.name_encrypted TEXT column Replaced by rooms.metadata (opaque encrypted JSON).
room_messages.reply_to Absent Added.
room_messages.edit_of, edit_sequence, edited_at Absent Added.
Reactions Absent New table.
Starred items Absent New table.
Generic preferences Absent New endpoints over user_preferences.
Key transparency Absent New tables + endpoints.
Call signaling Absent New events + endpoint.
Hangouts Absent Renamed to sessions with extension_id and session_type.
Model hosting Absent New endpoints + three hosting modes.
client-read client event Present Removed. Replaced by POST /users/me/read-state.
GET /users/me/sync Absent New.
POST /oprf/blind Absent New.
POST /auth/recover/start, /finish Absent New.
Push payload sender_user_id Present Removed. Replaced by sender_ref.
OPRF key rotation N/A Immutable for account lifetime; compromise-only rotation.
hangouts_enabled capability N/A Replaced by sessions_enabled.
hangout_max_participants capability N/A Replaced by session_types[].

---

3. Roles and Permissions

3.1 Global Roles

Role Level Permissions
owner 100 *
admin 80 user.manage, invite.unlimited, config.edit, room.force_delete, backup.manage
inviter 50 invite.limited
member 10 room.create, room.join, message.send

3.2 Room Roles

Role Permissions
owner Kick, delete room, promote/demote (Discord mode), transfer ownership, delete any message, edit metadata, change retention, set disappearing timer, create/delete sessions
moderator Kick (Discord mode), delete any message (Discord mode), create/delete sessions (Discord mode)
member Send messages, upload attachments, leave room, delete own messages, add reactions, edit own messages (within edit_window_seconds), create invites, join sessions, create sessions (Messenger mode)

3.3 Moderation Modes

Unchanged from V1 §3.3. In Discord mode, session creation and deletion are restricted to moderator+.

3.4 Enforcement Order

Unchanged from V1 §3.4.

3.5 Room Ownership Transfer

Unchanged from V1 §3.5.

---

4. Resource Limits

4.1 Three-Tier Model

Unchanged from V1 §4.1.

4.2 Default Limits

Key Server hard max Instance default Instance range
file_size_bytes 104857600 104857600 1 MB – 100 MB
room_size 1000 100 2 – 1000
rooms_per_user 500 50 1 – 500
devices_per_user 20 10 1 – 20
keypackages_per_device 50 20 5 – 50
message_size_bytes 65536 16384 256 B – 64 KB
attachment_retention_days 365 0 (forever) 0 – 365
call_max_participants 50 8 2 – 50
reactions_per_message 50 50 1 – 50
edit_window_seconds 86400 900 60 – 86400
sync_event_retention_days 365 90 30 – 365
key_transparency_retention_days 3650 3650 365 – 3650
sessions_per_room 25 10 1 – 25
session_max_participants 50 12 2 – 50
starred_items_per_user 100000 10000 100 – 100000

session_max_participants: conservative default of 12. For a mesh call without an SFU, 25 participants means 24 outbound and 24 inbound media streams per client. Operators who want larger sessions can raise this up to SERVER_MAX_SESSION_PARTICIPANTS and accept the operational cost. Per-type limits are declared in SESSION_TYPES.toml; see §5.26.1.

starred_items_per_user: bounds the initial sync response. A power user with 100,000 stars is unusual; the hard max protects against a runaway client.

4.3 Per-Entity Overrides

Unchanged from V1 §4.3, plus rooms.sessions_per_room override (owner-set, bounded by instance).

4.4 Effective Limits Exposure

Unchanged from V1 §4.4. GET /rooms/:id continues to return effective_max_file_size_bytes and effective_message_retention_days.

4.5 Cleanup Jobs

All jobs run on the shared hourly scheduler.

Job Retention Notes
Session cleanup 30 days after expiry or revocation V1.
Rate limit table 24 hours V1.
Audit log AUDIT_RETENTION_DAYS (90) V1.
Attachment pruning Three-tier effective retention V1.
Welcome expiry 7 days V1.
Message retention Per-room, falling to instance default V1.
Registration state 5-minute TTL V1.
Login state 5-minute TTL V1.
Sync state pruning sync_event_retention_days V2. Deletes user-scoped state rows older than the window whose user_seq is below the current max.
Push subscription expiry 90 days of inactivity V2. Revokes and deletes push_subscriptions with last_used_at < now - 90d.
Key transparency retention key_transparency_retention_days V2.
Recovery code consumption Immediate V2. Consumed codes are marked, not deleted, for audit.
Call state cleanup 24 hours after call end V2. Deletes call_sessions rows and any dangling participants.
Session metadata pruning (optional) 90 days of zero occupancy, disabled by default V2. A session is a persistent space. Do not enable without operator intent.

No cleanup job is required for session occupancy. Occupancy is in-memory and self-cleaning. There is nothing to prune.

No cleanup job is required for model files. Models are immutable and content-addressed by version.

---

5. Environment Variables

Follows Coolify conventions. All variables are optional unless marked required.

5.1 Application

```env
APP_ENV=production
APP_URL=https://chat.example.com
APP_NAME=Encrypted Chat
LOG_LEVEL=info
CLIENT_STATIC_DIR=/app/client
```

Log policy. The server writes structured logs to stdout and stderr only. It does not write log files. Container runtime captures and rotates stdout. The server does not store request logs persistently. No IP addresses, URLs, or user identifiers are written by the server to any persistent store. Retention of stdout is the operator’s responsibility and must be documented in the operator’s privacy policy.

Session occupancy is never logged. Not to stdout, not to stderr, not to any metrics pipeline, not to any tracing system. See §12.

SPA deployment note. Operators with multiple instances of the same server version MAY place a CDN in front of CLIENT_STATIC_DIR to reduce bandwidth. This is a deployment choice, not a server feature. The SPA bundle is version-coupled to the server; do not serve an SPA bundle from a different version than the server behind it.

5.2 Server

```env
SERVER_BIND=0.0.0.0:8080
SERVER_WORKERS=4
```

5.3 Database

```env
DB_PATH=/data/app.db
DB_BUSY_TIMEOUT_MS=5000
```

5.4 Sessions (HTTP Sessions)

```env
SESSION_EXPIRY_DAYS=30
SESSION_SLIDING=true
```

5.5 Server Hard Limits

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
```

5.6 Rate Limits

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
RATE_MODEL_DOWNLOAD_PER_MIN=30
```

Variable Default Notes
RATE_OPRF_BLIND_PER_MIN 30 Per IP. /oprf/blind.
RATE_OPRF_BLIND_PER_HOUR 300 Per IP.
RATE_RECOVER_START_PER_MIN 5 Per IP.
RATE_RECOVER_START_PER_HOUR 20 Per IP.
RATE_LOOKUP_PER_MIN 30 Per authenticated user.
RATE_EDIT_PER_MIN 30 Per user.
RATE_REACTION_PER_MIN 60 Per user.
RATE_LINK_PREVIEW_PER_MIN 10 Per user.
RATE_TURN_CREDENTIALS_PER_MIN 10 Per user.
RATE_SESSION_CREATE_HOURLY 20 Per user.
RATE_SESSION_CREATE_DAILY 100 Per user.
RATE_SESSION_JOIN_PER_MIN 30 Per user.
RATE_SESSION_HEARTBEAT_PER_MIN 10 Per user, per session. Normal usage is 4/min at 15s intervals.
RATE_MODEL_DOWNLOAD_PER_MIN 30 Per IP. Model downloads only.

Rate limit keys:

Variant Key format Window(s)
InviteCreate invite_create:{user_id}:{hour\|day}:{boundary} Hourly, Daily
InviteRedeem invite_redeem:{ip}:min:{boundary} Per minute
KpClaim kp_claim:{user_id}:{minute\|hour}:{boundary} Per minute, Hourly
Login login:{ip}:min:{boundary} Per minute
DataExport data_export:{user_id}:{boundary} EXPORT_RATE_LIMIT_HOURS
Presign presign:{user_id}:min:{boundary} Per minute
OprfBlind oprf_blind:{ip}:{min\|hour}:{boundary} Per minute, Per hour
RecoverStart recover_start:{ip}:{min\|hour}:{boundary} Per minute, Per hour
Lookup lookup:{user_id}:min:{boundary} Per minute
Edit edit:{user_id}:min:{boundary} Per minute
Reaction reaction:{user_id}:min:{boundary} Per minute
LinkPreview link_preview:{user_id}:min:{boundary} Per minute
TurnCredentials turn_credentials:{user_id}:min:{boundary} Per minute
SessionCreate session_create:{user_id}:{hour\|day}:{boundary} Hourly, Daily
SessionJoin session_join:{user_id}:min:{boundary} Per minute
SessionHeartbeat session_heartbeat:{user_id}:{session_id}:min:{boundary} Per minute
ModelDownload model_download:{ip}:min:{boundary} Per minute

Rate limit state is stored in a SQLite table and pruned hourly.

5.7 Transport Security

Unchanged from V1 §5.7.

5.8 OPAQUE and OPRF

```env
OPAQUE_OPRF_KEY_PATH=/data/oprf.key
USERNAME_OPRF_ENABLED=true
```

Variable Default Notes
OPAQUE_OPRF_KEY_PATH /data/oprf.key Path to the persisted serialized ServerSetup (128 bytes). Not a raw seed.
USERNAME_OPRF_ENABLED true V2 always enables username OPRF.

Cipher suite (OPAQUE): DefaultCipherSuite — Ristretto255, TripleDh<Ristretto255, Sha512>, Argon2 KSF.

OPRF construction (username lookup): voprf 0.5.0 crate, Ristretto255-SHA512, base (non-verifiable) mode.

Key derivation from the ServerSetup file (empirically verified by V-C, 2026-09-30):

```
oprf_key_bytes  = contents of OPAQUE_OPRF_KEY_PATH
                  (serialized ServerSetup, 128 bytes)
root_secret     = SHA-256(oprf_key_bytes)
username_key    = HKDF-Expand(root_secret, info="username-oprf-v1", length=32)
backup_key      = HKDF-Expand(root_secret, info="backup-encryption-v1", length=32)
```

Why not a raw seed. opaque-ke 4.0.1 cannot reconstruct ServerSetup deterministically from a 32-byte seed. The file therefore stores the serialized ServerSetup struct directly. The root_secret is derived from its bytes via SHA-256 and used as the HKDF PRK for all downstream keys. This preserves V1’s file format and requires no migration.

The voprf server is constructed as:

```rust
voprf::OprfServer::<Ristretto255Sha512>::new_from_seed(
    &username_key,
    b"username-oprf-v1",
)
```

The second argument is a domain separator used internally by the crate. It is deliberately identical to the HKDF info string and is not a bug.

root_secret rotation policy. root_secret is immutable for the lifetime of the deployment. Rotation is a catastrophic, operator-initiated operation requiring full user re-registration. There is no routine rotation.

Server setup persistence. The serialized ServerSetup is written to OPAQUE_OPRF_KEY_PATH on first startup with 0600 permissions on Unix.

Compatibility note. The key file format is identical to V1’s. Existing V1 files at the same path continue to work without modification. This is the only V1 storage format that V2 does not change.

5.9 Backups

Unchanged from V1 §5.9, with additions:

· Backups include the OPRF key, the ALTCHA HMAC secret, the VAPID keys, the key transparency log directory, and all database contents.
· Attachment blobs are included only if BACKUP_INCLUDE_ATTACHMENTS=true and STORAGE_BACKEND=fs.
· Model files are included only if BACKUP_INCLUDE_MODELS=true. Default false — models are large and immutable, and are recoverable from a shared origin if one is configured.

5.10 Push Notifications

Unchanged from V1 §5.10, with one change: push payload no longer includes sender_user_id in plaintext. The envelope carries room_id and an opaque sender_ref, which is the sender’s username_token truncated to 16 bytes and base64-encoded.

5.11 ALTCHA

Unchanged from V1 §5.11.

5.12 Storage Backend

Unchanged from V1 §5.12.

5.13 Attachment Format

Unchanged from V1 §5.13. C2SP chunk size is 16384 and is not configurable.

5.14 Moderation

Unchanged from V1 §5.14.

5.15 Invite Defaults

Unchanged from V1 §5.15.

5.16 Safety Numbers

Unchanged from V1 §5.16.

5.17 Audit Log

```env
AUDIT_RETENTION_DAYS=90
```

Metadata constraint. audit_log.metadata MUST NOT contain message content, IP addresses, URLs, or user-identifying data beyond user_id and room_id. This is enforced by code review; violations are treated as bugs.

5.18 Cleanup Scheduler

Unchanged from V1 §5.18.

5.19 Sockudo

```env
SOCKUDO_URL=http://sockudo:6001
SOCKUDO_APP_ID=chat
SOCKUDO_APP_KEY=auto
SOCKUDO_APP_SECRET=auto
SOCKUDO_ENABLE_CLIENT_EVENTS=true
```

Channel taxonomy (V2):

Channel Type Purpose
private-room-{room_id} Private Room events, client events (typing only)
private-user-{user_id} Private User-scoped durable events, session signaling

No presence channels. Presence is out of scope.

Client events accepted:

Event Channel Payload
client-typing.start private-room-{room_id} { user_id }
client-typing.stop private-room-{room_id} { user_id }

Removed: client-read. Read state is written via POST /users/me/read-state.

5.20 GDPR

Unchanged from V1 §5.20.

5.21 TLS Termination

Unchanged from V1 §5.21.

5.22 Key Transparency

```env
KEY_TRANSPARENCY_ENABLED=true
KEY_TRANSPARENCY_LOG_PATH=/data/kt-log
KEY_TRANSPARENCY_AUDITOR_KEYS=
```

5.23 Link Preview Proxy

```env
LINK_PREVIEW_PROXY_ENABLED=false
LINK_PREVIEW_PROXY_TIMEOUT_SECONDS=5
LINK_PREVIEW_PROXY_MAX_BYTES=1048576
```

5.24 Calls

```env
CALLING_ENABLED=false
TURN_URL=
TURN_SHARED_SECRET=
TURN_TTL_SECONDS=600
CALL_MAX_PARTICIPANTS=8
```

5.25 Username OPRF

```env
OPRF_BLIND_ENABLED=true
```

5.26 Sessions

```env
SESSIONS_ENABLED=true
SERVER_MAX_SESSIONS_PER_ROOM=25
SERVER_MAX_SESSION_PARTICIPANTS=50
SESSION_HEARTBEAT_INTERVAL_SECONDS=15
SESSION_HEARTBEAT_TIMEOUT_SECONDS=45
SESSION_OCCUPANCY_DEBOUNCE_MS=1000
SESSION_TYPES_CONFIG_PATH=/data/session-types.toml
```

Variable Default Range Notes
SESSIONS_ENABLED true — Advertised in /capabilities.
SERVER_MAX_SESSIONS_PER_ROOM 25 1 – 25 Hard cap.
SERVER_MAX_SESSION_PARTICIPANTS 50 2 – 100 Hard cap.
SESSION_HEARTBEAT_INTERVAL_SECONDS 15 5 – 60 Client heartbeat cadence.
SESSION_HEARTBEAT_TIMEOUT_SECONDS 45 15 – 180 Stale-participant timeout. Must be ≥ 2× interval.
SESSION_OCCUPANCY_DEBOUNCE_MS 1000 0 – 5000 Minimum interval between occupancy events per session.
SESSION_TYPES_CONFIG_PATH /data/session-types.toml — Path to the session type allowlist.

5.26.1 Session Types Configuration

SESSION_TYPES.toml:

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

Rules:

· The server reads this file at startup and on SIGHUP.
· A session can only be created with a session_type present in this file.
· A session’s max_participants cannot exceed the type’s max_participants.
· A room can hold at most max_per_room sessions of a given type.
· extension_id is informational and is echoed into capabilities. The server does not validate or enforce it.
· Malformed or missing file: sessions are disabled at startup with a clear error, but the server continues running. Other features are unaffected.

Client build emission. The client build process may emit a recommended SESSION_TYPES.toml as a build artifact via atoll-chat build --emit-session-types. The emitted file is a suggestion; the server reads only the operator’s copy.

5.27 Model Hosting

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

Variable Default Notes
MODEL_HOSTING_ENABLED true Advertised in /capabilities as model_hosting_enabled.
MODEL_HOSTING_MODE local local, external, or proxy. See below.
MODEL_EXTERNAL_BASE_URL empty Required when mode is external or proxy. Must begin with https:// in production.
MODEL_STORAGE_PATH /data/models Root for all model files. Ignored in external mode.
STT_MODELS_PATH /data/models/stt Subdirectory for STT models. Ignored in external mode.
TTS_MODELS_PATH /data/models/tts Subdirectory for TTS models. Ignored in external mode.
STT_DEFAULT_MODEL moonshine-tiny Advertised as stt_default_model.
TTS_DEFAULT_MODEL supertonic-3 Advertised as tts_default_model.
MODEL_DOWNLOAD_RATE_PER_MIN 30 Per IP.
BACKUP_INCLUDE_MODELS false Include model files in backups.

Hosting modes:

Mode Behavior Use case
local Instance serves models from local filesystem. Clients fetch from the instance. Self-hosted single-instance deployments.
external Instance does not serve models. It advertises MODEL_EXTERNAL_BASE_URL in capabilities. Clients fetch directly from the shared origin. SaaS with multiple instances pointing at one model host.
proxy Instance serves models but fetches from MODEL_EXTERNAL_BASE_URL on cache miss, then caches locally. Hybrid: shared origin plus local cache.

Startup validation:

· If MODEL_HOSTING_MODE is external or proxy, MODEL_EXTERNAL_BASE_URL must be set and must begin with https:// in production. The server exits on mismatch.
· If MODEL_HOSTING_MODE is local, MODEL_EXTERNAL_BASE_URL is ignored.
· If MODEL_HOSTING_MODE is external, MODEL_STORAGE_PATH, STT_MODELS_PATH, and TTS_MODELS_PATH are ignored.

Manifest resolution:

· local and proxy: manifest read from STT_MODELS_PATH/manifest.json and TTS_MODELS_PATH/manifest.json.
· external: manifest read from MODEL_EXTERNAL_BASE_URL/manifest.json, cached in memory, refreshed on a TTL (default 1 hour) or on operator-triggered reload.

Shared origin requirements (for external and proxy mode):

The origin at MODEL_EXTERNAL_BASE_URL MUST:

· Serve over HTTPS.
· Serve with Cache-Control: public, max-age=31536000, immutable.
· Serve with Content-Type: application/octet-stream.
· Serve with ETag from the file hash.
· Support Range requests.
· Serve the manifest at /manifest.json (or at the same paths the instance would use in local mode).

The spec does not prescribe who runs the shared origin. It can be a CDN, an S3 bucket with CloudFront, a dedicated small instance, or another Atoll instance running in local mode.

Assets that are not shareable. Model hosting is the only shared-origin asset mechanism in the server specification. Static SPA assets, bundled sticker packs, wallpapers, sounds, and icons are version-coupled with the client bundle and MUST be served from the instance’s own CLIENT_STATIC_DIR. Attachments, shared sticker packs, custom stickers, thumbnails, and avatars are content-addressed and served from the attachment storage backend. They are not candidates for a shared static origin.

5.28 Generic Preferences

No environment variables. The endpoint is always enabled when authenticated.

5.29 Starred Items

No environment variables. The feature is always enabled when authenticated.

5.30 Batched Test Execution Model

The test suite is organized into domain batches so that any single batch runs in under 60 seconds. This exists because the full integration suite (55+ test binaries) exceeds common tool execution timeouts (540 s in CI sandboxes) when run unfiltered. Cargo executes test binaries sequentially, and each binary performs non-trivial setup (SQLite initialization, embedded migrations, cryptographic key generation).

The repository provides:

· server/tests/batch-manifest.toml — machine-readable batch definitions
· server/Makefile with test-<batch> targets
· server/scripts/check-test-batches.sh — orphan detection
· server/tests/test_batch_manifest.rs — test that enforces the manifest invariant
· server/TESTING.md — contributor guide

Contributor rules:

1. Run tests via make test-<batch> or make test-all. Never run unfiltered cargo test.
2. Every file in server/tests/*.rs MUST appear in exactly one batch in batch-manifest.toml. Files common.rs and test_batch_manifest.rs are excluded (they are helpers and the enforcer, respectively).
3. Adding a new test file requires updating batch-manifest.toml. If no batch fits, add a new batch and a corresponding test-<name> target to the Makefile.
4. Any task that adds a test file MUST update the manifest and verify make check-batches passes.
5. Each batch MUST complete in under 60 seconds. If a batch exceeds this, split it.

Enforcement. The test_batch_manifest test and the make check-batches script both verify the manifest invariant. Both fail on orphan files, phantom entries, or duplicate assignments.

---

6. Client Interface Contract

6.1 Authentication

· Authorization: Bearer <session_token> on every authenticated request.
· Tokens are 43-character base64url strings.

6.2 OPAQUE Handshake

· Registration: start then finish. Both within 5 minutes.
· Login: start then finish. Both within 5 minutes.
· Recovery: POST /auth/recover/start then POST /auth/recover/finish. Both within 10 minutes.

6.3 ALTCHA Payload

Unchanged from V1 §6.3.

6.4 MLS Message Envelope

Unchanged from V1 §6.4.

6.5–6.11 Attachments

Unchanged from V1 §6.5 through §6.11.

6.12 Room Membership

Unchanged from V1 §6.12.

6.13 Push Subscriptions

Unchanged from V1 §6.13.

6.14 Safety Numbers

Unchanged from V1 §6.14.

6.15 WebSocket Connection

Unchanged from V1 §6.15.

6.16 Client Events

Only two client events are accepted: client-typing.start and client-typing.stop. client-read is removed. Client events are ephemeral.

6.17 Client Capability Requirements

Unchanged from V1 §6.17.

6.18 CoreCrypto Initialization

Unchanged from V1 §6.18.

6.19 OPRF Blinding

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
  7. token     = OprfClient::finalize(username, evaluated)
```

The token is the 64-byte SHA-512 output of voprf::OprfClient::finalize() (empirically verified by V-A2, 2026-09-30), encoded as an 86-character unpadded base64url string. It is not the 32-byte group element h^k. The voprf 0.5.0 crate fuses unblinding and hashing inside finalize; the intermediate group element is not exposed by the public API.

Token construction (RFC 9497 §2.2):

```
hash_input = I2OSP(len(username), 2) || username
           || I2OSP(len(unblinded_element), 2) || unblinded_element
           || "Finalize"
token      = SHA-512(hash_input)
```

The token is deterministic for a fixed (username, server key). The client’s blinding factor does not affect the output.

The client MUST cache the token in memory for the session. The client MUST NOT persist the token at rest.

6.20 Display Name Encryption

```
material          = HKDF-Expand(token, info="display-name-encryption-v1", length=44)
display_name_key  = material[0..32]
nonce             = random 12 bytes
encrypted_display = nonce || AES-256-GCM(display_name_key, nonce, display_name)
```

Constraints:

· The plaintext display name is bounded to 256 bytes. This bounds the encrypted ciphertext to 12 + 256 + 16 = 284 bytes.
· The ciphertext is base64url encoded (unpadded) for transport and storage.
· The server never decrypts this value.

6.21 Device Name Encryption

```
material              = HKDF-Expand(token, info="device-name-encryption-v1", length=44)
device_name_key       = material[0..32]
nonce                 = random 12 bytes
encrypted_device_name = nonce || AES-256-GCM(device_name_key, nonce, device_name)
```

Same constraints as §6.20.

6.22 User-Scoped Sync Cursor

On boot, after SQLite hydrate and before subscribing to channels:

1. GET /users/me/sync?since_seq=<cursor>
2. Apply returned state rows in user_seq order.
3. Store max_seq as the new cursor.
4. Subscribe to private-user-{user_id}.
5. Apply live events as they arrive; ignore any event with user_seq <= cursor.
6. Update the cursor on every applied row.

If the server returns { "full_resync_required": true }, the client discards its cursor and refetches all current state.

6.23 Recovery

1. Prompt for recovery code and username.
2. Blind the username via POST /oprf/blind.
3. Call POST /auth/recover/start with { recovery_code, username_token }.
4. Receive a recovery session and an OPAQUE registration challenge.
5. Compute the new RegistrationRecord bound to the new password.
6. Encrypt the display name with the current token.
7. Call POST /auth/recover/finish.
8. Store the new session.

The client MUST treat session revocation on other devices as expected.

6.24 Calls

1. Fetch calling from /capabilities. If false, calls are unavailable.
2. Subscribe to private-room-{room_id} for signaling.
3. Send signaling messages via POST /rooms/:id/calls/:call_id/signal.
4. Fetch TURN credentials via POST /calls/turn-credentials before starting a call.
5. Publish call.start / call.end events.

6.25 Link Preview Proxy

1. Fetch link_preview_proxy_enabled from /capabilities.
2. When enabled and a URL is CORS-blocked, encrypt the URL with a per-request Content Key.
3. Send the encrypted URL to POST /link-preview/proxy.
4. Decrypt the response with the Content Key.

The server never sees the plaintext URL.

6.26 Sessions

1. Fetch sessions_enabled from /capabilities. If false, sessions are unavailable.
2. Fetch the session list via GET /rooms/:id/sessions.
3. Display each card with decrypted metadata and participant_count.
4. Join via POST /rooms/:id/sessions/:session_id/join, supplying a client_id.
5. Send a heartbeat every SESSION_HEARTBEAT_INTERVAL_SECONDS via POST .../heartbeat.
6. On WebSocket disconnect, do not automatically rejoin. Show the session as disconnected and offer a rejoin action.
7. Leave via POST .../leave. Any client_id of the authenticated user may be specified.
8. Initiate offers to all existing participants on join, using POST .../signal with target_client_id. Include sender_client_id, which must belong to the authenticated user.
9. Treat the roster as user-scoped: one entry per user, with a client_ids array.
10. On room deletion, treat all sessions in that room as deleted.
11. If sessions_enabled=false, existing sessions remain visible; join/signal/heartbeat return 501; delete is allowed.

Extension routing. The client reads extension_id from session.created. If the client does not have that extension installed, the event is dropped silently. A session owned by an uninstalled extension is invisible to that client. The server does not require clients to be aware of all sessions.

Graceful degradation on session types. The client cross-references its locally declared session types against session_types[] from capabilities. If a type is declared locally but absent from session_types[], the client disables that feature in the UI with a clear message (“This feature is disabled on this server”).

Client responsibility on icon attachments. If a session’s metadata references an icon attachment and that attachment is no longer available, the client falls back to the emoji variant or a default. The server does not validate the file_id.

6.27 Model Fetching

1. Fetch model_hosting_enabled from /capabilities. If false, no models are available.
2. Read stt_models_base_url and tts_models_base_url.
3. Fetch model files via GET from the advertised base URL. The server may be the instance itself (local/proxy mode) or a shared origin (external mode).
4. Cache via the Cache API. Re-download on version change.
5. Never call an external CDN not advertised in capabilities.

6.28 Starred Items

1. GET /users/me/starred-items for the initial list, with optional type, room_id, limit, cursor.
2. POST /users/me/starred-items to star.
3. DELETE /users/me/starred-items/:item_id?item_type= to unstar.
4. Apply starred_item.added and starred_item.removed events by user_seq.
5. Apply tombstones (deleted_at IS NOT NULL) by removing the item locally.

6.29 Generic Preferences

1. GET /users/me/preferences/:key to read a specific preference.
2. PATCH /users/me/preferences/:key to write a preference value.
3. DELETE /users/me/preferences/:key to remove a preference.
4. Apply preference.updated events by user_seq; the value is not in the event, so refetch from sync or from GET /users/me/preferences/:key if needed.

Reserved keys. room_order is server-defined and has its own endpoint. Keys starting with _ are reserved. All other keys are client-defined.

---

7. Data Model

7.1 Identity and Auth

```sql
CREATE TABLE users (
    id                  TEXT PRIMARY KEY,
    -- username_token: 64-byte SHA-512 output of OprfClient::finalize(),
    -- encoded as 86-character unpadded base64url. See §6.19.
    username_token      TEXT NOT NULL UNIQUE,
    -- encrypted_display: nonce (12 bytes) || AES-256-GCM(display_name),
    -- encoded as unpadded base64url. Decodes to 28–284 bytes. See §6.20.
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

username_token constraints (enforced by the server):

· Exactly 86 characters.
· Base64url alphabet (A-Z a-z 0-9 - _), no padding.
· Decodes to exactly 64 bytes.

encrypted_display constraints (enforced by the server):

· Valid base64url (unpadded).
· Decodes to between 28 and 284 bytes.
· The server never decrypts it.

On account deletion: username_token is replaced with a random 86-character base64url string. encrypted_display and profile are set to NULL. This satisfies the NOT NULL UNIQUE constraint while preventing reuse of the original token.

7.2 User-Scoped Sync

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
CREATE INDEX idx_starred_room ON starred_items(user_id, room_id, starred_at DESC);
```

room_order is stored in user_preferences with key = 'room_order'. Nicknames, read-aloud settings, and other client-defined preferences also live in user_preferences.

7.3 Invites

Unchanged from V1 §7.2.

7.4 Rooms

```sql
CREATE TABLE rooms (
    id                    TEXT PRIMARY KEY,
    owner_id              TEXT NOT NULL REFERENCES users(id),
    metadata              TEXT,
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

7.5 MLS Lifecycle

Unchanged from V1 §7.4, plus pending_mls_adds:

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

7.6 Messages and Reactions

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
    reaction           TEXT NOT NULL,
    created_at         DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at         DATETIME,
    UNIQUE (message_id, sender_user_id, sender_client_id, reaction)
);

CREATE INDEX idx_reactions_message ON reactions(message_id)
    WHERE deleted_at IS NULL;

CREATE INDEX idx_reactions_room ON reactions(room_id, created_at DESC);
```

Edit model. An edit is a new room_messages row with edit_of = <original id> and edit_sequence = <previous + 1>. The original row remains. The edit chain is reconstructed by querying edit_of = <original id> ORDER BY edit_sequence. edited_at is set on the original row on the first edit.

Edit window. The server rejects an edit if now - created_at > edit_window_seconds.

7.7 Attachments

Unchanged from V1 §7.5.

7.8 Push Subscriptions

Unchanged from V1 §7.6.

7.9 Calls and Sessions

```sql
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

CREATE TABLE sessions (
    id               TEXT PRIMARY KEY,
    room_id          TEXT NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    extension_id     TEXT NOT NULL,
    session_type     TEXT NOT NULL,
    created_by       TEXT NOT NULL REFERENCES users(id),
    metadata         TEXT,
    metadata_version INTEGER NOT NULL DEFAULT 1,
    position         INTEGER NOT NULL DEFAULT 0,
    max_participants INTEGER,
    created_at       DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at       DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_sessions_room ON sessions(room_id, position);
CREATE INDEX idx_sessions_extension ON sessions(room_id, extension_id, session_type);
```

Session occupancy is not stored. There is no session_participants table. Occupancy is held in memory only and is lost on restart.

In-memory occupancy shape (not persisted):

```
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
```

7.10 Key Transparency and Ops

```sql
CREATE TABLE key_transparency_log (
    leaf_index      INTEGER PRIMARY KEY,
    user_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    username_token  TEXT NOT NULL,
    identity_pubkey TEXT NOT NULL,
    added_at        DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
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
```

7.11 State Outside the Database

Unchanged from V1 §7.8, plus:

Path Purpose Notes
In-memory occupancy Call and session participation Never persisted. Lost on restart.
MODEL_STORAGE_PATH Model files Immutable. Optional in external mode.
SESSION_TYPES_CONFIG_PATH Session type allowlist Operator-maintained.

---

8. API Surface

All routes prefixed with /api/v1/. Auth via Authorization: Bearer <session_token> unless noted.

Error format:

```json
{
  "error": "machine_readable_code",
  "message": "Human-readable message",
  "details": {}
}
```

8.0 Static Client Hosting and HTTPS Enforcement

Unchanged from V1 §8.0.

8.1 Public

Method Path Purpose
GET /health Liveness
GET /ready Readiness
GET /capabilities Feature advertisement
GET /auth/register/challenge ALTCHA challenge
POST /auth/register/start OPAQUE registration start
POST /auth/register/finish OPAQUE registration finish
POST /auth/login/start OPAQUE login start
POST /auth/login/finish OPAQUE login finish
POST /auth/recover/start Recovery authorization
POST /auth/recover/finish Recovery credential install
POST /invites/redeem Validate and consume server invite
GET /invites/:code Public invite validation
POST /oprf/blind OPRF evaluation
POST /link-preview/proxy Link preview proxy (opt-in)
GET /models/stt/v1/:model_id/:version/:filename STT model files (local/proxy mode only)
GET /models/tts/v1/:model_id/:version/:filename TTS model files (local/proxy mode only)
GET /models/manifest.json Model manifest (local/proxy mode only)

GET /capabilities response (V2):

```json
{
  "version": "2.0.0",
  "calling": true,
  "call_max_participants": 8,
  "sessions_enabled": true,
  "max_sessions_per_room": 10,
  "max_session_participants": 50,
  "session_types": [
    { "extension_id": "core.hangouts", "type": "voice", "max_participants": 12 },
    { "extension_id": "com.example.watch-together", "type": "watch", "max_participants": 20 }
  ],
  "model_hosting_enabled": true,
  "model_hosting_mode": "local",
  "stt_models_base_url": "https://chat.example.com/models/stt/v1/",
  "stt_default_model": "moonshine-tiny",
  "tts_models_base_url": "https://chat.example.com/models/tts/v1/",
  "tts_default_model": "supertonic-3",
  "tts_models": [
    {
      "id": "supertonic-3",
      "version": 1,
      "size_bytes": 419430400,
      "languages": ["*"],
      "voices": [
        { "id": "en-US-natural", "language": "en-US", "gender": "neutral" },
        { "id": "en-GB-warm",    "language": "en-GB", "gender": "neutral" },
        { "id": "ja-JP-female",  "language": "ja-JP", "gender": "female" },
        { "id": "es-ES-neutral", "language": "es-ES", "gender": "neutral" }
      ]
    }
  ],
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
  "threading_enabled": true,
  "starred_items_per_user": 10000
}
```

8.1.1 POST /oprf/blind

Auth: None. Rate limits: RATE_OPRF_BLIND_PER_MIN, RATE_OPRF_BLIND_PER_HOUR, both per IP.

Request: { "blinded": "<base64, group element>" }
Response: { "evaluated": "<base64, group element>" }

Logging: each request increments an ephemeral counter in oprf_audit. No IP, no timing, no linkage to any user.

8.1.2 POST /link-preview/proxy

Unchanged from earlier specification.

8.1.3 GET /models/stt/v1/:model_id/:version/:filename and GET /models/tts/v1/:model_id/:version/:filename

Auth: None. Rate limit: RATE_MODEL_DOWNLOAD_PER_MIN, per IP.

Behavior: serve static model files with immutable cache headers. In external mode, this endpoint is not registered; capabilities advertise the external URL instead. In proxy mode, on cache miss, fetch from MODEL_EXTERNAL_BASE_URL, write to MODEL_STORAGE_PATH, then serve.

Headers: Cache-Control: public, max-age=31536000, immutable. Content-Type: application/octet-stream. ETag from file hash. Range requests supported.

Errors:

Error HTTP error field
Not found 404 model_not_found
Rate limited 429 rate_limited
Mode is external 404 model_hosting_external

8.1.4 GET /models/manifest.json

Auth: None.

Behavior: return the model manifest. In local and proxy mode, read from STT_MODELS_PATH/manifest.json and TTS_MODELS_PATH/manifest.json and combine. In external mode, this endpoint is not registered.

8.2 User

Method Path Purpose
POST /auth/logout Revoke session
GET /users/me Current user
PATCH /users/me Update profile
DELETE /users/me Delete account
GET /users/me/export Export data
POST /users/lookup Username token lookup
GET /users/me/devices List devices
DELETE /users/me/devices/:id Revoke device
GET /users/me/sessions List HTTP sessions
DELETE /users/me/sessions/:id Revoke HTTP session
POST /users/me/push-subscriptions Register push subscription
DELETE /users/me/push-subscriptions/:id Revoke push subscription
GET /users/me/sync Fetch user-scoped state
POST /users/me/read-state Write read state
PATCH /users/me/room-order Write room order
GET /users/me/preferences/:key Read a preference
PATCH /users/me/preferences/:key Write a preference
DELETE /users/me/preferences/:key Delete a preference
GET /users/me/starred-items List starred items
POST /users/me/starred-items Star an item
DELETE /users/me/starred-items/:item_id Unstar an item
POST /users/me/avatar Upload avatar

8.2.1 GET /users/me

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

8.2.2 PATCH /users/me

```json
{
  "encrypted_display": "<base64>",
  "profile": "<base64>"
}
```

8.2.3 POST /users/lookup

Auth: Required. Rate limit: RATE_LOOKUP_PER_MIN, per authenticated user.

Request: { "username_token": "<86-char base64url>" }

Found response (HTTP 200):

```json
{
  "user_id": "<user_id>",
  "encrypted_display": "<base64url or null>"
}
```

Not-found response (HTTP 404):

```json
{ "error": "not_found" }
```

The two responses differ in status code by design. The endpoint’s purpose is to let a client discover whether a user exists before sending them a message. Making found and not-found indistinguishable would defeat that purpose.

Enumeration mitigation. Rate limiting (RATE_LOOKUP_PER_MIN) is the defense against token enumeration. Timing padding is not required and is not applied. The username_token namespace is a 512-bit space, so brute-force enumeration of tokens is infeasible regardless of rate limits.

8.2.4 GET /users/me/sync

Query: since_seq (integer, required; 0 for full sync).

Response:

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
  "starred_items": [
    { "item_id": "a3f9...", "item_type": "attachment", "room_id": "r_abc123", "user_seq": 1288, "starred_at": "...", "deleted_at": null }
  ],
  "max_seq": 1290,
  "full_resync_required": false
}
```

8.2.5 POST /users/me/read-state

Request: { "room_id": "...", "last_read_message_id": "..." }

Behavior: verify membership; increment user_seq; upsert read_state; publish read.sync.

8.2.6 PATCH /users/me/room-order

Request: { "room_ids": ["r1", "r2", "r3"] }

Behavior: verify membership; increment user_seq; upsert user_preferences; publish room_order.sync.

8.2.7 POST /users/me/avatar

Request: multipart/form-data with a single file part. Response: 201 Created with the attachment record.

8.2.8 Preferences

GET /users/me/preferences/:key — returns { "key": "...", "value": <json>, "user_seq": N } or 404.

PATCH /users/me/preferences/:key — request body { "value": <json> }. Validates key pattern ^[a-z][a-z0-9_-]*(:[a-z0-9_-]+)*$, max 128 chars. Value size limit 64 KB. Reserved keys: room_order (has its own endpoint). Keys starting with _ are reserved.

DELETE /users/me/preferences/:key — deletes the row. Publishes preference.updated with a tombstone.

8.2.9 Starred Items

GET /users/me/starred-items — query type, room_id, limit (default 100, max 500), cursor. Returns array of starred items excluding tombstones by default; include_deleted=true returns tombstones.

POST /users/me/starred-items — request { item_id, item_type, room_id }. Enforces starred_items_per_user. Publishes starred_item.added.

DELETE /users/me/starred-items/:item_id — query item_type. Writes tombstone with new user_seq. Publishes starred_item.removed.

8.3 Admin

Unchanged from V1 §8.3, plus:

Method Path Purpose
POST /admin/oprf/rotate Rotate the username OPRF key (catastrophic)
GET /admin/key-transparency Read KT log stats
POST /admin/key-transparency/snapshot Trigger a signed snapshot
GET /admin/rooms/:id/sessions Read-only list of sessions (metadata only, no occupancy)
POST /admin/models/reload Reload the model manifest
POST /admin/session-types/reload Reload SESSION_TYPES.toml

8.4 Rooms

Unchanged from earlier specification, plus the previously specified endpoints for metadata, retention preview, and member pagination.

8.5 MLS and Messaging

Unchanged from earlier specification, plus editing, reactions, threading, and pending adds endpoints.

8.6 Attachments

Unchanged from V1 §8.6.

8.7 Calls and Sessions

Method Path Purpose
POST /calls/turn-credentials Fetch TURN credentials
POST /rooms/:id/calls/:call_id/signal Send call signaling message
POST /rooms/:id/calls/:call_id/end End call
GET /rooms/:id/sessions List sessions with live counts
POST /rooms/:id/sessions Create a session
PATCH /rooms/:id/sessions/:session_id Update metadata / position
DELETE /rooms/:id/sessions/:session_id Delete a session
POST /rooms/:id/sessions/:session_id/join Join a session
POST /rooms/:id/sessions/:session_id/leave Leave a session
POST /rooms/:id/sessions/:session_id/heartbeat Heartbeat
GET /rooms/:id/sessions/:session_id/roster Current participants (gated)
POST /rooms/:id/sessions/:session_id/signal Relay signaling

8.7.1 POST /calls/turn-credentials

Unchanged from earlier specification.

8.7.2 POST /rooms/:id/calls/:call_id/signal

Unchanged from earlier specification.

8.7.3 GET /rooms/:id/sessions

Query: extension_id, session_type optional filters.

Response:

```json
{
  "sessions": [
    {
      "id": "s_abc123",
      "extension_id": "core.hangouts",
      "session_type": "voice",
      "metadata": "<base64, opaque>",
      "metadata_version": 1,
      "position": 0,
      "participant_count": 3,
      "created_at": "<ISO 8601>"
    }
  ]
}
```

participant_count reflects live in-memory state. No identities are exposed. participant_count counts users, not clients.

8.7.4 POST /rooms/:id/sessions

Request: { "extension_id": "<id>", "session_type": "<type>", "metadata": "<base64>", "position": 0, "max_participants": 12 }

Permission: any member by default; moderator+ in Discord mode.

Behavior:

1. Verify session_type is in SESSION_TYPES.toml.
2. Verify max_participants does not exceed the type’s cap, and does not exceed SERVER_MAX_SESSION_PARTICIPANTS.
3. Verify the room has not reached the type’s max_per_room cap or SERVER_MAX_SESSIONS_PER_ROOM.
4. Assign position = max(position) + 1 if not supplied.
5. Insert row, publish session.created.

Errors:

Error HTTP error field
Unknown session type 400 unknown_session_type
Participant cap exceeded 400 participant_cap_exceeded
Room session cap reached 409 session_limit_reached
Sessions disabled 501 sessions_disabled
Rate limited 429 rate_limited

8.7.5 PATCH /rooms/:id/sessions/:session_id

Permission: creator, room owner, or moderator (Discord mode). extension_id is not enforced.

Request: { "metadata": "<base64>", "position": 0 } (both optional).

Behavior: update row; if position supplied, re-sequence other sessions in the room; publish session.updated.

8.7.6 DELETE /rooms/:id/sessions/:session_id

Permission: creator, room owner, or moderator (Discord mode).

Behavior: delete row; tear down in-memory occupancy if present; publish session.deleted.

8.7.7 POST /rooms/:id/sessions/:session_id/join

Request: { "client_id": "<client_id>" }

Response:

```json
{
  "roster": [
    { "user_id": "u_1", "client_ids": ["c_a"] },
    { "user_id": "u_2", "client_ids": ["c_b"] }
  ],
  "media_config": { "ice_servers": [ ... ] }
}
```

Behavior:

1. Verify room membership.
2. Enforce max_participants (users, not clients).
3. Create or attach to in-memory occupancy.
4. Add the participant’s client_id.
5. Publish session.occupancy with updated user-count (subject to debounce).
6. Return roster and ICE config.

Multi-device: a user may have multiple client_ids in a session. The roster shows one entry per user with all their client_ids. Count is per user.

8.7.8 POST /rooms/:id/sessions/:session_id/leave

Request: { "client_id": "<client_id>" }

Behavior: remove the client from the user’s client_ids. If it was the user’s last client, remove the user. If the occupancy is now empty, tear it down. Publish session.occupancy with updated count. No disk write.

Cross-device leave: any client_id belonging to the authenticated user may be specified, not necessarily the caller’s own.

8.7.9 POST /rooms/:id/sessions/:session_id/heartbeat

Auth: Required. Must be a current participant.

Request: { "client_id": "<client_id>" }

Response: 204 No Content.

Behavior: update last_heartbeat for the caller’s client entry. Rate-limited per user, per session. A background task marks participants stale if now - last_heartbeat > SESSION_HEARTBEAT_TIMEOUT_SECONDS and removes them.

8.7.10 GET /rooms/:id/sessions/:session_id/roster

Auth: Required. Caller must be a current participant.

Response: same shape as the roster field in join.

Errors: 403 not_a_participant if the caller is not currently in the session.

8.7.11 POST /rooms/:id/sessions/:session_id/signal

Request:

```json
{
  "sender_client_id": "<client_id>",
  "target_client_id": "<client_id>",
  "signal_type": "<extension-defined>",
  "payload": "<base64, opaque>"
}
```

Behavior:

1. Verify both caller and recipient (recipient_user_id derived from the roster) are current participants.
2. Verify sender_client_id belongs to the authenticated user.
3. Publish session.signal on private-user-{recipient_user_id}.

The server does not parse, validate, log, or store signal_type or payload. Extensions define their own signaling protocols.

Initiation pattern: the joining participant initiates offers to each existing participant. Existing participants answer. This is a client convention, not enforced by the server.

8.7.12 Room deletion cascade

On DELETE /rooms/:id:

1. Publish session.deleted for every session in the room, before the database commit.
2. Tear down in-memory occupancy for those sessions.
3. Commit the database transaction (which cascades session rows).

8.7.13 Sessions disabled mode

When SESSIONS_ENABLED=false:

· POST /rooms/:id/sessions → 501 sessions_disabled
· POST .../join → 501 sessions_disabled
· POST .../signal → 501 sessions_disabled
· POST .../heartbeat → 501 sessions_disabled
· GET /rooms/:id/sessions → available (returns metadata, count is 0)
· PATCH .../sessions/:id → available
· DELETE .../sessions/:id → available
· GET .../roster → 403 not_a_participant

8.8 Event Catalog

Room events (private-room-{room_id}):

Event Payload When
message.new { id, room_id, sender_user_id, sender_client_id, epoch, seq, content_type, reply_to, created_at } After POST /rooms/:id/messages
message.edited { id, edit_of, edit_sequence, room_id, sender_user_id, created_at } After PATCH .../messages/:msg_id
message.deleted { id, room_id } After DELETE .../messages/:msg_id
reaction.added { id, room_id, message_id, sender_user_id, reaction, created_at } After add
reaction.removed { id, room_id, message_id } After remove
room.updated { room_id, metadata?, retention_days?, max_file_size_bytes? } After PATCH /rooms/:id
room.member_added { room_id, user_id, role, joined_at } After add
room.member_removed { room_id, user_id } After kick or leave
epoch.updated { room_id, epoch, sequence } After advance
call.started { call_id, room_id, initiator_id, started_at } On call session start
call.ended { call_id, room_id, ended_at } On call end
call.signal { call_id, sender_user_id, signal_type, payload } After signal
session.created { room_id, session_id, extension_id, session_type, created_by, created_at } After create
session.updated { room_id, session_id, metadata?, position? } After PATCH
session.deleted { room_id, session_id, extension_id } After DELETE or room deletion
session.occupancy { room_id, session_id, participant_count } On join/leave/timeout, debounced

session.occupancy carries only a count. It never carries identities. Identities are exchanged through the roster endpoint and signaling path, which are gated on participation.

User events (private-user-{user_id}, durable):

Event Payload Durable
read.sync { room_id, last_read_message_id, user_seq } Yes
room_order.sync { room_ids, user_seq } Yes
device.added { device_id, platform, added_at, user_seq } Yes
device.revoked { device_id, reason, user_seq } Yes
device.name_updated { device_id, encrypted_device_name, user_seq } Yes
session.revoked { session_id, reason } No
user.updated { user_id, profile_version, user_seq } Yes
call.signal { call_id, sender_user_id, signal_type, payload } No
session.signal { room_id, session_id, sender_user_id, sender_client_id, target_client_id?, signal_type, payload } No
starred_item.added { item_id, item_type, room_id, user_seq } Yes
starred_item.removed { item_id, item_type, user_seq } Yes
preference.updated { key, user_seq } Yes
kt.snapshot { tree_size, root_hash, created_at } Yes

Delivery semantics:

· Room events: best-effort. Clients reconcile via REST.
· User events: durable via user_seq. Live push over private-user-{user_id}; catch-up via GET /users/me/sync. Non-durable user events (call.signal, session.signal) are delivered live only.
· session.occupancy is debounced to at most once per second per session. Coalesced changes publish the latest count.

---

9. CLI Subcommands

Unchanged from earlier specification, plus:

Command Purpose
server oprf rotate --confirm Resample the username OPRF key (catastrophic)
server kt snapshot Trigger a signed key transparency snapshot
server kt verify --from <index> Verify KT log integrity from an index
server models verify Verify model files against manifest hashes
server models fetch --from <url> Pre-seed models from a shared origin (proxy mode)
server session-types validate Validate SESSION_TYPES.toml without starting the server

---

10. Client Contract Matrix

Feature Server behaviour
Text messaging Relays MLS ciphertext via Sockudo; never inspects content
Message editing Stores edit chain; enforces edit_window_seconds
Message deletion Tombstones; publishes message.deleted
Reactions Aggregated; silent
Threading reply_to field
Starred items User-scoped sync; durable events
Generic preferences Client-defined keys; durable sync
Attachments — upload Content-addressed, C2SP chunked encryption
Attachments — download Range headers supported
Attachments — presign S3 only; filesystem returns 501
Identity OPRF token; no plaintext usernames
Display names AES-GCM ciphertext; key derived from OPRF token
Device names AES-GCM ciphertext; user-scoped sync
Recovery Server-generated codes; Argon2id; OPAQUE re-registration
Multi-device sync User-scoped state + user_seq cursor
User events Durable; live push + REST catch-up
Room events Best-effort; REST reconciliation
Calls Signaling on room channel; TURN via dedicated endpoint
Sessions Persistent metadata; in-memory occupancy; count-only room events; roster gated on participation; dense positions; cross-device leave permitted; opaque signal relay; extension_id advisory; operator-configured type allowlist
Model hosting Three modes: local, external, proxy. Capabilities advertise the base URL and mode.
Push notifications sender_ref replaces sender_user_id
Client events client-typing.* only
Delta sync (epoch, seq) cursor
User sync user_seq cursor via GET /users/me/sync

---

11. Phased Build Plan

V1 phases 1–20 are complete. V2 phases:

Phase Deliverable Depends On
21 OPRF identity 4a
22 Recovery 21
23 User-scoped sync foundation 21
24 User-scoped sync writes 23
25 Device model 23, 24
26 PATCH /rooms/:id with encrypted metadata 23
27 Message editing 11
28 Reactions 11
29 Threading 11
30 Member pagination 7a
31 Retention preview 7a
32 POST /users/me/avatar 12a
33 pending_mls_adds coordination 10
34 Key transparency 4a
35 Link preview proxy 4a
36 Call signaling 11
37 TURN credentials 36
38 Admin UI additions 13
39 Capabilities update 21–38
40 Sessions 36, 37
41 Model hosting (STT + TTS), three modes 14
42 Starred items 23, 24
43 Generic preferences endpoint 23

Critical path: 21 → 23 → 24 → 25. Phases 41 and 43 are independent and high-priority. Phase 40 closes the sessions work. Phase 42 is small on the server side.

Note: V2 phase 44 (Hangout clarifications) is removed. Its contents are absorbed by Amendment 35.

---

12. Security Boundaries

Boundary Guarantee
Server sees plaintext messages Never
Server sees decryption keys Never
Server sees plaintext username Never
Server sees plaintext display name Never
Server sees plaintext device name Never
Server sees attachment contents Never
Server sees attachment plaintext size Yes (metadata)
Server can decrypt past messages No
Server can decrypt future messages No
Web client protected against compromised server No
Native client protected against compromised server Yes
Registration endpoints bot-resistant Yes (ALTCHA)
Production traffic is HTTPS Yes
Attachments are streamed via range requests Yes
Attachment encryption is key-committing Yes
Presigned URLs grant access to full object Yes, but only encrypted bytes; TTL-limited
Presigned URLs are range-restricted No
Server knows which messages are deleted Yes
Ambient presence is tracked No
Presence channels exist No
Session co-presence is disclosed to room members Only as a count. Identities are disclosed only to participants.
Session occupancy is persisted Never. In-memory only.
Session occupancy is exported to any external system Never. No logs, no metrics, no tracing, no audit, no monitoring.
Session participation history is reconstructable No. No record exists.
Server restart clears session occupancy Yes, by design.
Session metadata is encrypted Yes
Session roster is gated on participation Yes
Session signaling requires both parties to be participants Yes
Session signal payloads are opaque Yes. Server relays bytes. No interpretation, logging, or storage. This is an architectural constraint, not a privacy measure.
Session extension_id is authenticated No. It is advisory. Room membership is the authorization boundary.
Session type allowlist Operator-configured via SESSION_TYPES.toml.
Model files are immutable and content-addressed Yes
Model hosting can be shared across instances Yes, via external mode
SPA bundle is version-coupled Yes (deployment concern, not a server feature)
Multi-node deployments supported No (out of scope)

Normative refusal clause:

Ambient presence is refused. The server does not track or expose who is online, when they were last seen, or what they are doing across the system.

Call-scoped co-presence is permitted. Within a call or session, the server knows in memory which room members are participating, in order to route media. This knowledge is disclosed only to participants of the same call or session. It is held in memory only and is never written to any persistent store or external system — not to the database, not to log files, not to stdout, not to stderr, not to any metrics pipeline, not to any tracing system, not to any monitoring system, not to any audit log. No server component may export occupancy state. This is a normative constraint on all server code.

The server routes a session; it does not remember it.

---

13. Backups and Disaster Recovery

Unchanged from V1 §13, with:

· Backups include the OPRF key file, the ALTCHA HMAC secret, the key transparency log directory, the session types config, and all database contents.
· Model files are included only if BACKUP_INCLUDE_MODELS=true. Default false.
· Session occupancy is not part of any backup because it is not stored.

---

14. GDPR Compliance

14.1 Data Subject Rights

Unchanged from V1 §14.1.

14.2 Account Deletion

Anonymises the user row, deletes devices, sessions, KeyPackages, push subscriptions, room memberships, recovery codes, and all user-scoped sync state (read state, preferences, device names, starred items). Queues MLS removes.

Identity columns on deletion:

· username_token — set to a random 86-character base64url string. This satisfies the NOT NULL UNIQUE constraint and prevents reuse of the original token.
· encrypted_display — set to NULL.
· profile — set to NULL.

Key transparency log: entries are retained (the log is append-only) but user_id is replaced with a random placeholder. The username_token in the KT log is retained because the log is the public audit record.

14.3 Data Export

Unchanged from V1 §14.3, plus:

· username_token is included but noted as only interpretable by the user.
· encrypted_display and encrypted_device_name are included as opaque ciphertext.
· Recovery code metadata is included; hashes are not.
· Starred items are included.
· Preferences are included.
· Session participation is not in the export because it is not stored.

14.4 Retention

Unchanged from V1 §14.4, plus sync_event_retention_days and key_transparency_retention_days. Session occupancy is not retained at all.

14.5 Tombstone Semantics

Unchanged from V1 §14.5.

14.6 Metadata Minimisation

Field Leak Mitigation
users.username_token Opaque; requires OPRF key to invert Key compromise is catastrophic and documented
users.encrypted_display Opaque ciphertext Key derived from OPRF token
devices.encrypted_device_name Opaque ciphertext Key derived from OPRF token
sessions.last_seen_at Timestamp Coarsened to hour in admin views
messages.created_at Timestamp Coarsened to minute
room_messages.edited_at Edit timing Coarsened to day in admin views
room_messages.deleted_at Deletion timing Coarsened to day in admin views
push_subscriptions.push_token Only unavoidable personal data Auto-revoked after 90 days
Push sender_ref Truncated OPRF token Requires OPRF key to correlate
audit_log.metadata JSON Never includes content, IPs, URLs
kt_log.username_token Public by design It is the KT subject
session.occupancy N/A Not persisted; never exported
Session roster N/A In-memory; gated on participation
Session signal payloads N/A Never parsed, logged, or stored
Model files N/A Immutable; not user data

14.7 Controller Obligations

Unchanged from V1 §14.7.

14.8 Audit Actions

V1 actions plus:

· oprf.rotate
· recover.start
· recover.finish
· device.name_updated
· edit.create
· reaction.create
· reaction.delete
· kt.snapshot
· call.start
· call.end
· session.create
· session.delete
· model.manifest_reload
· session_types.reload

No audit entry is written for session join, leave, heartbeat, or occupancy change. That would create exactly the log this design refuses.

---

15. Amendment Process

Unchanged from V1 §15. Factual disputes about external specifications must be resolved by citation, not negotiation. Design questions are resolved through reasoned argument and may be negotiated.

---

16. Amendments (V2)

16.1 Amendment 13 — OPRF Identity Layer — 2026-09-29

Replaced plaintext users.username, users.username_hash, and users.display_name with OPRF-based username_token and encrypted_display. Added POST /oprf/blind. Updated registration, login, and lookup. Removed devices.name; device names become user-scoped sync state.

16.2 Amendment 14 — Recovery Flow — 2026-09-29

Added POST /auth/recover/start and POST /auth/recover/finish. Recovery codes are server-generated, Argon2id-hashed, single-use by default. Recovery re-registers the OPAQUE record and revokes all existing sessions.

16.3 Amendment 15 — User-Scoped Sync — 2026-09-29

Added user_seq, read_state, user_preferences, device_names. Added GET /users/me/sync, POST /users/me/read-state, PATCH /users/me/room-order. User events durable; room events best-effort.

16.4 Amendment 16 — Room Metadata and Retention Preview — 2026-09-29

Replaced rooms.name_encrypted with rooms.metadata. Added PATCH /rooms/:id and POST /rooms/:id/retention/preview.

16.5 Amendment 17 — Message Editing and Threading — 2026-09-29

Added reply_to, edit_of, edit_sequence, edited_at. Added PATCH /rooms/:id/messages/:msg_id. Added message.edited. Enforced edit_window_seconds.

16.6 Amendment 18 — Reactions — 2026-09-29

Added reactions table and endpoints. Added reaction.added / reaction.removed. Enforced reactions_per_message.

16.7 Amendment 19 — Key Transparency — 2026-09-29

Added key_transparency_log, key_transparency_snapshots. Added auditor signature support. Added kt.snapshot event and admin endpoints.

16.8 Amendment 20 — Link Preview Proxy — 2026-09-29

Added opt-in link preview proxy with SSRF guard. URLs encrypted with request-scoped Content Key. Added POST /link-preview/proxy.

16.9 Amendment 21 — Call Signaling and TURN — 2026-09-29

Added call_sessions, call_participants. Added POST /calls/turn-credentials, POST /rooms/:id/calls/:call_id/signal, POST /rooms/:id/calls/:call_id/end. Added call.* events.

16.10 Amendment 22 — Member Pagination and Avatar Upload — 2026-09-29

Added cursor pagination to GET /rooms/:id/members. Added POST /users/me/avatar.

16.11 Amendment 23 — Removal of Presence — 2026-09-29

Presence is out of scope. No presence channels, no presence_visibility column, no live presence indicators. The client spec’s “Last seen” setting is removed.

16.12 Amendment 24 — Push Payload Metadata Reduction — 2026-09-29

Removed sender_user_id from the push payload. Replaced with sender_ref, a truncated OPRF token.

16.13 Amendment 25 — Hangouts — 2026-09-30

Status: Superseded by Amendment 35.

Added persistent, named, room-scoped voice spaces with in-memory occupancy. Superseded by Amendment 35 (Generalized Real-Time Sessions) on 2026-10-02 before implementation. The hangouts concept became the core.hangouts / voice session type under Amendment 35.

16.14 Amendment 26 — Model Hosting — 2026-09-30

Added model hosting for STT and TTS with three modes: local, external, and proxy. Added env vars, capabilities fields, endpoints, and rate limit.

Key constraints:

· local mode: instance serves from local filesystem.
· external mode: instance advertises an external base URL; clients fetch from the shared origin.
· proxy mode: instance caches on first fetch and serves locally thereafter.
· Startup validation: external and proxy require MODEL_EXTERNAL_BASE_URL; https:// required in production.
· Manifest resolution differs per mode.
· Model files are immutable and content-addressed by version.
· No other asset category is shareable across instances. SPA bundles, bundled stickers, wallpapers, sounds, icons, and favicons are version-coupled and instance-specific. Attachments and stickers use the attachment pipeline.
· Models are excluded from backups by default (BACKUP_INCLUDE_MODELS=false).

16.15 Amendment 27 — Starred Items — 2026-09-30

Added starred_items table and endpoints. Added starred_item.added and starred_item.removed events. Integrated into GET /users/me/sync. Enforced starred_items_per_user.

Key constraints:

· Composite primary key (user_id, item_id, item_type).
· room_id is NOT NULL.
· Durable events carry user_seq.
· deleted_at tombstones support unstar.

16.16 Amendment 28 — Generic Preferences Endpoint — 2026-09-30

Added GET/PATCH/DELETE /users/me/preferences/:key. Added preference.updated durable event.

Key constraints:

· Key pattern: ^[a-z][a-z0-9_-]*(:[a-z0-9_-]+)*$, max 128 chars.
· Value size limit: 64 KB.
· Reserved keys: room_order (server-defined endpoint); keys starting with _ are reserved.
· Event payload carries key and user_seq; the value is fetched from sync.

16.17 Amendment 29 — Hangout Clarifications — 2026-09-30

Status: Superseded by Amendment 35.

Folded into Amendment 25 (C1–C5) at the time. All clarifications now live in Amendment 35 with the sessions rename.

16.18 Amendment 30 — OPRF Key Derivation from Serialized ServerSetup — 2026-10-01

§5.8 key derivation formula corrected to reflect that OPAQUE_OPRF_KEY_PATH stores the serialized ServerSetup (128 bytes), not a raw 32-byte seed. The root_secret is computed as SHA-256(oprf_key_bytes). The username_key and backup_key are then derived via HKDF-Expand from root_secret.

Verification: Task V-C empirically confirmed that opaque-ke 4.0.1 cannot construct ServerSetup deterministically from a raw seed. The serialized file format is retained from V1 with no changes.

16.19 Amendment 31 — Username Token Format and Length — 2026-10-01

§6.19 and §7.1 clarified that username_token is the 64-byte SHA-512 output of OprfClient::finalize(), encoded as an 86-character unpadded base64url string. It is not the 32-byte group element h^k. The voprf 0.5.0 crate fuses unblinding and hashing inside finalize; the intermediate element is not exposed by the public API.

Verification: Task V-A2 empirically confirmed the token length and construction with the voprf 0.5.0 reference implementation.

16.20 Amendment 32 — Lookup Response Semantics — 2026-10-01

§8.2.3 corrected. Found vs. not-found responses now explicitly differ in status code (200 vs 404). Timing padding is not applied. Enumeration is mitigated by rate limiting.

Rationale: The prior text said found and not-found must be indistinguishable in shape and timing. That contradicts the endpoint’s purpose. The corrected text makes the difference explicit and documents rate limiting as the actual defense.

16.21 Amendment 33 — Identity Bounds and Deletion Behavior — 2026-10-01

§6.19, §6.20, §6.21, and §7.1 add explicit bounds:

· username_token: exactly 86 characters, base64url, decodes to 64 bytes.
· encrypted_display: decodes to 28–284 bytes.
· On account deletion, username_token is replaced with a random 86-character string.

Rationale: These constraints are enforced by the current implementation (task 21b). Documenting them ensures future tasks do not silently deviate.

16.22 Amendment 34 — Batched Test Execution Model — 2026-10-01

Added §5.30. The test suite is organized into domain batches. Contributors run tests via make test-<batch> rather than unfiltered cargo test. New test files must be registered in batch-manifest.toml. Two enforcement mechanisms prevent orphans.

Rationale: The full integration suite exceeds common tool execution timeouts. Batching is the standard workflow. Documenting it in the spec ensures future tasks reference the correct pattern.

16.23 Amendment 35 — Generalized Real-Time Sessions — 2026-10-02

Supersedes: Amendment 25 (Hangouts), Amendment 29 (Hangout Clarifications).

Change: Hangouts are generalized into sessions. A session is a room-scoped, extension-owned, ephemeral coordination space. Participants join, exchange opaque signaling messages, and leave. The server tracks who is present, in memory only, and relays messages between them. It does not interpret metadata, signaling payloads, or session purpose. Voice hangouts become one session type among many.

Key constraints:

· Occupancy is in-memory only. Never persisted. Never exported to any log, metric, tracing, monitoring, or audit system.
· session.occupancy events carry a count only; never identities.
· Roster is gated on participation.
· Heartbeat interval: 15s default. Timeout: 45s default.
· Participant cap counts users, not clients.
· Joining participant initiates offers.
· Position is dense 0..N-1 within room. Server renumbers on create, delete, and PATCH-with-position.
· Occupancy events debounced to ≤1 per second per session.
· Room deletion tears down in-memory occupancy before database commit.
· Client does not auto-rejoin after WebSocket disconnect.
· No notifications on session join.
· Sessions never appear in call history.
· target_client_id and sender_client_id are included on signal requests and event payloads. sender_client_id is validated against the authenticated user.
· Any client_id of the authenticated user may be specified on leave.
· When SESSIONS_ENABLED=false: create, join, signal, and heartbeat return 501 sessions_disabled; list, patch, delete remain available; roster returns 403 not_a_participant.
· extension_id is advisory. The server stores it, echoes it in capabilities and events, and does not validate or enforce it. PATCH and DELETE follow standard room rules: creator, room owner, or moderator (Discord mode).
· Session types are declared in an operator-maintained SESSION_TYPES.toml. The server enforces the allowlist and per-type caps.
· Client extension systems are entirely client-side. The server sees only session_type and extension_id as opaque strings.
· Signal payloads are relayed opaquely. Server-side interpretation of signal content is forbidden. This is an architectural constraint, not a privacy measure. Extensions define their own signaling protocols.

Forward-looking note: Extension-scoped fire-and-forget broadcasts to room members outside of a session (“someone is watching”) are a future addition. When drafted, the mechanism will generalize the existing client-events channel (client-typing.*), not introduce a new mechanism.

Affected tasks: Phase 40 (renamed from Hangouts to Sessions; new task session-types-config).

---

17. V1 Amendment Log (Historical)

V1 amendment V2 disposition
1 — C2SP chunked encryption Carried forward
2 — Message deletion endpoint Carried forward
3 — Event catalog and client events Revised by §16.11 (client-read removed)
4 — Delta sync cursor Carried forward
5 — Push payload schema Revised by §16.12 (sender_user_id removed)
6 — Effective limits exposure Carried forward
7 — Presence out of scope Reaffirmed by §16.11 and §16.23
8 — CoreCrypto initialization clarification Carried forward
9 — Context binding test vectors Carried forward
10 — Chunk size correction Carried forward
11 — Sync ordering fix Carried forward
12 — Padding algorithm replacement Carried forward

---

18. Document Status

This is the contract for the server side of the system. Every V2 implementation task references this document. If a task conflicts with this spec, the task is wrong and must be revised. If a feature is missing, it does not exist yet — it must be added here first, then built.

Amendments are tracked in §16. The V1 amendment log is preserved in §17 for historical reference.

End of Server Specification v2.0.
