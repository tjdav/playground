Client Specification v2.0

Status: Final — source of truth for client implementation tasks.
Server basis: Server Specification v3.0.
Supersedes: Client Specification v1.0.
Stack: Coralite 1.0.0-rc.5 · Wire CoreCrypto 10.5.2 · Mediabunny · Supertonic · Moonshine / Whisper · Capacitor · Tauri · @atoll/bot
Design language: Privacy-first messenger. Adopts proven social-interaction patterns; visual density follows modern Western conventions.

Design decisions made during drafting are recorded at §24.

---

1. Product Summary

A self-hosted, end-to-end encrypted group messaging client. The server is an untrusted delivery service that never sees plaintext, keys, or plaintext user identifiers. The client holds all plaintext, all keys, and all MLS state. Encryption, media processing, transcription, and speech synthesis run locally.

Layer Technology
Framework Coralite 1.0.0-rc.5
Config loader coralite-scripts
Runtime Node.js ≥ 22.22.2, ESM
MLS engine Wire CoreCrypto 10.5.2 (WASM, bundled)
Attachment crypto Web Crypto API (AES-256-GCM, HKDF, SHA-256)
Username lookup OPRF (client-side blinding), lookup_token derivation
Media processing Mediabunny (WebCodecs-first, ffmpeg.wasm fallback)
STT Moonshine Tiny / Small, Whisper Tiny (ONNX Runtime Web)
TTS Supertonic 3 (ONNX Runtime Web)
Real-time pusher-js (Sockudo, Pusher Protocol 7)
Search Fuse.js (lazy-loaded)
Storage SQLite (native on mobile/desktop, WASM+OPFS on web)
CSS Tailwind utilities + Coralite scoped <style>
Icons Solar icon set (tree-shaken)
Emoji picker emoji-picker-element
Waveform wavesurfer.js
QR codes qr-code-styling
Bots @atoll/bot (parallel package to @atoll/extend)

1.1 v2.0 architectural changes

1. Token split. The client derives lookup_token from the OPRF output and sends it to the server. The raw token stays in memory and never leaves the device.
2. Encrypted preferences. All preference values are encrypted client-side. The server stores opaque ciphertext.
3. Bot infrastructure. Bots are first-class identities with capability-scoped grants. Write-only bots cryptographically cannot read room content. The bot runtime is a separate Node.js process.
4. Composer commands. A unified command registry with a dedicated slash palette.
5. Declarative settings. Extensions and bots declare settings fields; the client renders them in existing chrome.
6. Local messages. A new message visibility level for output only the user sees.
7. Kiko system voice. A scripted system voice with three avatar states, three accessibility modes, and full i18n.
8. Username generation. A two-mode registration flow with generated names and a mandatory save screen.
9. Encrypted call signaling. Call signaling moves to the user channel with ephemeral ECDH key agreement.
10. Encrypted session signaling. Session signaling uses a signed, encrypted envelope with associated data binding.
11. Pending MLS remove. New events and honest handling of the two-member residual.
12. Publisher key persistence. Publisher keys are stored server-side in room_publisher_keys and verified client-side.

---

2. Scope

2.1 In Scope

Everything in v1.0 §2.1, plus:

· lookup_token derivation and use at registration, login, recovery, and lookup.
· Encrypted preference values (all of them, not just secrets).
· Bot infrastructure: @atoll/bot package, capability grants, three derived modes, publisher key, command encryption, bot settings.
· Composer / trigger with a dedicated slash command palette.
· Declarative settings fields for extensions and bots.
· Local messages (self-messages) with persistence, dismissal, and local search.
· Kiko system voice: three states, three modes, kiko.* locale namespace.
· Generated username flow with theme picker, candidate selection, and mandatory save screen.
· /whoami core command.
· Call signaling on user channels with ephemeral ECDH and call key rotation.
· Session signaling envelope with signature, associated data, and honest confidentiality statement.
· Pending MLS remove handling with the two-member residual documented.
· Member list integration for member-tier bots.
· Call join/leave endpoints.
· Encrypted session metadata and signal keys derived from RMK with domain separation.
· Publisher key publication on epoch transitions, with client-side verification.

2.2 Presence and Co-Presence

Unchanged from v1.0 §2.2, with one addition:

The connected: true|false field on GET /rooms/:id/bots is a server-to-bot liveness indicator. It is a server-to-bot connection status, not user presence.

· It is present in the response only when the authenticated caller is the bot's owner_user_id. The server enforces this; the client does not filter.
· It is scoped to the bot's own channel.
· It is ephemeral, in-memory, never logged, never synced.
· It is never shown in the member list.
· It is displayed only in the owned-bots settings page, as a small dot: green for connected, grey for disconnected, with the timestamp of the last refresh. A (stale) marker appears if the last fetch is older than 30 seconds.

2.3 Out of Scope

Unchanged from v1.0 §2.3, plus:

· ctx.whisper — deferred to v3.1.
· User-scoped file settings — restricted to room scope in v3.0.
· Runtime extension installation.
· Local LLM Kiko — deferred to post-v3.0.
· Bots as binaries — shared as source.
· Hosting the bot runtime in the client. The bot runtime is a separate Node.js process (see §27).

2.4 Breaking Changes from v1.0

Change v1.0 v2.0
Wire field for OPRF username_token lookup_token (still 86-char base64url, 64 bytes)
Client-readable preference values Plaintext Encrypted with preferences_key
Call signaling channel private-room-{room_id} private-user-{user_id}
Call signaling payload signal_type + payload plaintext envelope (encrypted, signed)
Call participation Implicit Explicit join/leave endpoints
Session signaling payload signal_type + payload plaintext envelope (signed, AES-256-GCM)
MLS pending remove Single event mls.remove_stale + mls.remove_confirmed
Composer Text only / trigger + slash palette
Extension settings Component-based Declarative (component escape hatch retained)
Local messages Absent First-class
System voice Plain text Kiko (three states, three modes)
Username registration Chosen only Chosen or generated
Member list Users only Users + member-tier bots
Bot identity Absent @atoll/bot package; headless Node.js runtime
Commands Absent Core, extension, session, bot
Bot settings encryption Absent X25519 ECDH to bot_command_pubkey
Publisher keys In-memory server-side Persisted server-side; verified client-side

---

3. Design Principles

Unchanged from v1.0 §3, with three additions:

10. The server routes; it does not remember. Session occupancy and call participation are in-memory only. The client never displays what the server does not have.
11. Capabilities are declared; grants are a subset; modes are derived. The user sees what a bot can do, grants a subset, and never sees an internal mode.
12. Secrets are write-only. Once set, a secret is never readable by any client. It is set and replaced, never revealed.

---

4. Architecture

4.1 Pages

Unchanged from v1.0 §4.1.

4.2 In-Page Routing

Unchanged from v1.0 §4.2, plus one addition:

Bot grant routes. Deep links to a bot grant confirmation use ?rail=chat&detail=bot-grant&id=<roomId>&botId=<botId>.

4.3 Directory Structure

Unchanged from v1.0 §4.3, plus:

```
packages/
├── extend/                     @atoll/extend — extension SDK
└── bot/                        @atoll/bot — bot SDK (separate Node.js package)
    ├── src/
    │   ├── define-bot.js
    │   ├── define-command.js
    │   ├── define-setting.js
    │   ├── define-trigger.js
    │   ├── runtime/
    │   ├── transport/
    │   └── index.js
    └── package.json
```

And in the app:

```
src/
├── extensions/
│   ├── chat/
│   ├── media/
│   ├── ... (unchanged)
│   └── bots/                   # bot management UI
├── lib/
│   ├── bot/                    # client-side bot helpers (command encryption only)
│   ├── commands/               # slash command registry + palette
│   ├── local-messages/         # local message storage + rendering
│   └── kiko/                   # Kiko strings, avatar states, mode
```

lib/bot/ contains only the client-side helpers: command encryption when a user invokes a bot command, bot settings encryption with ephemeral X25519, and the token handoff UI. The bot runtime itself is a separate Node.js process using @atoll/bot and is not bundled with the client.

4.4 Plugin Architecture

Unchanged from v1.0 §4.4, plus:

· botPlugin({ commandEncryption: true, settingsEncryption: true }) — client-side command encryption and bot settings encryption. Does not host a bot runtime.
· commandPlugin({ registry }) — slash command registry and palette.
· kikoPlugin({ defaultMode: 'kiko' }) — system voice mode, avatar states, locale bridge.

4.5 State Management

Unchanged from v1.0 §4.5, plus these shell state keys:

Key Shape Notes
kikoMode 'kiko' \| 'plain' \| 'off' —
localMessages Record<roomId, LocalMessage[]> —
activeBots Record<roomId, Bot[]> Cached bot list for owned bots, refreshed on demand. Not a runtime.
botGrants Record<botId, { roomId, mode, scopes }[]> Cached room-bot grants. Server is the source of truth.

Extensions and bots continue to use their own namespaced state slice.

4.6 Cross-Component Communication

Unchanged from v1.0 §4.6.

4.7 Boot Sequence

```
1. index.html renders (auth gate)
2. app.html loads
3. Coralite shell renders
4. state-plugin initializes
5. router-plugin initializes
6. extension-plugin registers all extensions
7. Read session token from secure storage
8. Initialize client-side bot helpers:
   load owned-bots cache, prepare command encryption keys. No runtime starts.
9. Open SQLite, run migrations
10. Hydrate from SQLite:
    ├─ Rooms, members, room order
    ├─ Unread counts, drafts, preferences
    ├─ Cached display names, device names, nicknames
    ├─ Session metadata per room
    ├─ Starred items
    ├─ Cached read-aloud settings
    ├─ Extension storage (namespaced)
    ├─ Extension preferences (mirrored locally)
    ├─ Local messages
    ├─ Bot grants cache, owned-bots cache
    ├─ Kiko mode
    └─ Sync cursors (per-room + user-scoped)
11. User-scoped state sync:
    GET /users/me/sync?since_seq=<last_user_seq>
    Apply read_state, user_preferences, device_state, starred_items, bot_settings
    Store new max_seq
12. Derive OPRF token + lookup_token (memory only)
13. Derive preferences_key, display_name_key, device_name_key (memory only)
14. CoreCrypto initialization
15. Welcome poll:
    GET /welcomes
    ├─ If any pending welcomes, process them, fetch messages from welcome epoch.
    └─ Schedule next poll at +60s if any pending MLS add exists.
16. Room delta sync (since cursor)
17. Render UI with hydrated data
18. Fetch capabilities
    └─ Read sessions_enabled, session_types[], model hosting, extension_proxy_enabled
19. Connect WebSocket (pusher-js)
20. Subscribe to private-user-{user_id} and all private-room-{room_id}
21. Background: key package replenishment, welcome polling,
    pending removes, thumbnail generation, TTS/STT model idle prefetch
```

Failure modes unchanged from v1.0 §4.7, with additions:

Step Failure Behavior
8 Bot helper init fails Bots show as offline. Owner sees a diagnostic local message.
12 lookup_token derivation fails Fatal. Redirect to auth gate.
13 preferences_key derivation fails Preferences fall back to device-local until resolved.
15 Welcome fetch fails Retry with backoff. App works from local state.

4.8 Sync & Offline

Unchanged from v1.0 §4.8, plus:

· Bot settings are part of the operator's sync response. Applied atomically with the rest of the sync payload. A partial application does not advance the cursor.
· Local messages are never synced. Device-local only.
· Read counts are rendered from the message record (read_by_count on GET /rooms/:id/messages), updated live by read.count events.

4.9 Component Authoring Rules

Unchanged from v1.0 §4.9.

---

5. Design System

Unchanged from v1.0 §5.

---

6. Layout

Unchanged from v1.0 §6, with two additions:

6.1 Member list with bots

The member list renders users and member-tier bots. Bots carry a "Bot" chip next to the display name. Bots do not have a role selector. Removing a bot queues a pending_mls_remove; the bot disappears from the list immediately.

6.2 Slash palette placement

· Desktop: drop-up above the composer, left-anchored, width capped at 480px.
· Mobile: bottom sheet above the keyboard, full width minus safe area insets.
· Tablet: drop-up, same as desktop.

The palette never covers the composer input. See §12.

---

7. Component Inventory

Unchanged from v1.0 §7, plus these additions.

7.1 Primitives (additions)

Component Responsibility Key attributes
ui-bot-chip "Bot" chip next to bot names —
ui-kiko-avatar Kiko avatar state: 'default' \| 'concerned' \| 'error'

7.2 Composed (additions)

Component Responsibility
slash-command-palette Slash command autocomplete. See §12.
slash-command-row One row in the palette
slash-command-group-header Sticky group header
local-message Local message rendering
local-message-card Structured card variant for /status, /help, /whoami
bot-row Bot entry in a room settings list
bot-grant-card Install-time capability checklist
bot-settings-card Rendered declarative settings for a bot
setting-field One field, any type
kiko-line Kiko's framing line

7.3 Containers (additions)

Component Responsibility
bot-list Bots in a room
bot-grant-flow Install wizard
local-message-list Local messages in a room
command-help-card Rendered /help
command-status-card Rendered /status
command-whoami-card Rendered /whoami

7.4 Shell

Unchanged.

7.5 Views

Updated mapping. The bots view is new.

Extension Routes
core.chat chats, chat, room-settings
core.media media, media-viewer
core.documents documents, document
core.links links, link
core.calls calls, call
core.settings settings, settings-section
core.sessions sessions, session
core.bots bots, bot-detail, bot-grant
core.profile profile
core.join join
core.admin admin

---

8. Crypto & E2EE

8.1 Three Encryption Layers

Layer Purpose Implementation
MLS Message encryption, group state Wire CoreCrypto 10.5.2
C2SP Attachment encryption Web Crypto API
OPRF Username lookup, display-name/device-name/preferences encryption Client-side OPRF blinding + HKDF

8.2 Room Metadata Key (RMK)

Unchanged, plus new derivations:

```
room_metadata_key    = HKDF(RMK, "room-metadata-v1", 32)
session_metadata_key = HKDF(RMK, "session-metadata-v1", 32)
session_signal_key   = HKDF-Expand(RMK, "session-signal-v1" || session_id, 32)
```

8.3 OPRF-Based Lookup

Purpose: the server must not hold plaintext usernames or display names, and must not hold the raw OPRF output.

```
Given: username, server key k, hash-to-curve H

Client:
  1. h         = H(username)
  2. r         = random blinding factor
  3. blinded   = h * r
  4. sends { blinded } to POST /oprf/blind

Server:
  5. evaluated = blinded ^ k
  6. returns { evaluated }

Client:
  7. token         = OprfClient::finalize(username, evaluated)   // 64 bytes, memory only
  8. lookup_token  = HKDF-Expand(token, "username-lookup-v1", 64)
```

Wire format. lookup_token is 64 bytes, encoded as 86-character unpadded base64url. It is sent to the server in place of the raw token. The server stores it in users.username_token (column name unchanged).

Derivation details:

· Hash: SHA-256.
· HKDF-Expand directly (no HKDF-Extract). token is the SHA-512 output of finalize, already a uniform PRK.
· Info string: exactly "username-lookup-v1", UTF-8, no null terminator.
· Output: 64 bytes.

Token retention. Both token and lookup_token are cached in memory for the session. Neither is persisted at rest. Both are re-derived on login.

Key rotation. If the OPRF key rotates, both values change. The client must re-register. Documented.

8.4 Derived Keys

```
display_name_key  = HKDF-Expand(token, "display-name-encryption-v1", 32)
device_name_key   = HKDF-Expand(token, "device-name-encryption-v1",  32)
preferences_key   = HKDF-Expand(token, "preferences-encryption-v1",  32)
```

All derive from token, not lookup_token. All are memory-only. All use 32-byte output directly.

8.5 OPRF Lookup — Operator Enumeration

The token split closes the display-name derivation path but does not close enumeration. An operator with the OPRF key can still compute lookup_token for any candidate username. Rate limiting is the enumeration defense, not cryptography. The client spec states this plainly.

8.6 C2SP Attachment Encryption

Unchanged from v1.0 §8.4.

8.7 MLS Client Behavior

Updated with pending remove handling.

Key package lifecycle unchanged.

Group lifecycle:

· Create — unchanged.
· Join — unchanged.
· Leave — the client marks the room as left immediately and deletes local MLS state. It does not wait for mls.remove_confirmed. The pending remove is queued server-side; the client does not track it.
· The client does not publish a Remove proposal for itself. It marks the room as left, deletes local MLS state, and relies on the server's pending_mls_removes mechanism. Self-removal completes when an online member generates the Remove commit or when the pending remove goes stale.

Two-member residual. In a two-member room where one leaves and the other never connects, the removal has no guaranteed completion. The client MUST NOT promise the user that removal will complete. The room disappears from the departing client's list immediately. No "leaving…" state. No pending indicator.

Rejoin. Rejoining cancels the pending remove server-side. The client discards old MLS state and processes a fresh Welcome. The old state is never reused.

Owner leave. A room owner cannot leave until ownership is transferred. Exception: if the owner is the sole member, "leave" is a room deletion.

Welcome polling. The client fetches pending welcomes via GET /welcomes:

· On boot.
· After login.
· Every 60 seconds while any pending MLS add exists.

Each welcome is deleted locally after successful processing. mls.welcome_ready is non-durable; the event is a ping, the endpoint is the payload.

8.8 Key Transparency

Updated for bots.

For users:

1. Fetch identity keys with tree_head, auditor_signatures, inclusion_proof.
2. Verify the server signature, each auditor signature, and the inclusion proof.
3. Mark the contact as transparency-verified.

For bots:

1. Fetch the bot's bot_command_pubkey and identity_pubkey with the same proof via GET /kt/bot/:id.
2. Verify against the KT entry.
3. Only then encrypt commands to bot_command_pubkey.

Historical verification: after a bot rotates keys, use GET /kt/bot/:id/history and the bot_key_leaf_index field on the message to fetch the correct historical key.

Failure cases as v1.0 §8.6.

kt.snapshot handling: the event is non-durable. On receipt, mark cached verifications stale and schedule a background re-verification pass (debounced, 5 minutes). On foreground activation, fetch GET /kt/snapshot as a fallback, throttled to at most one fetch per 5 minutes. Skip if a kt.snapshot event was received within the throttle window.

8.9 Safety Numbers

Unchanged.

8.10 Call Signaling Encryption

Scope. Per-room, per-call. Forward secrecy via ephemeral ECDH.

Design:

1. Each participant generates an ephemeral X25519 keypair on joining the call.
2. The participant sends the ephemeral public key, signed with their MLS identity key, via POST /rooms/:id/calls/:call_id/signal. The server relays it on each target's private-user-{target_user_id} channel as a call.signal event.
3. The signature is verified against the signer's identity_pubkey, fetched from the member list and verified against the KT log.
4. The initiator generates a random 32-byte call key.
5. The initiator encrypts the call key to each participant's ephemeral public key (X25519 ECDH → HKDF-SHA256 → AES-256-GCM).
6. All signaling is encrypted with the call key.

Encryption parameters:

Field Value
Key agreement X25519
KDF HKDF-Expand, SHA-256
Info string "call-signaling-v1"
Encryption AES-256-GCM
Nonce 12 bytes, random per message
Associated data call_id \|\| sender_user_id \|\| target_user_id
Wire format nonce \|\| ciphertext \|\| tag, base64url unpadded

Envelope plaintext:

```json
{
  "signal_type": "offer" | "answer" | "ice" | "call_key" | "ephemeral_key" | "leave",
  "payload": { ... }
}
```

signal_type is inside the envelope. The server does not see it.

Rotation. Rotate on join and on leave. The participant with the lowest user_id among remaining participants generates the new call key and distributes it. No rotation on re-negotiation (video on/off, mute, screen share).

Wire event:

```json
{
  "call_id": "...",
  "sender_user_id": "...",
  "sender_client_id": "...",
  "target_client_id": "...",
  "envelope": "<base64url>"
}
```

The server delivers to all of the target user's devices via private-user-{target_user_id}. The client filters by target_client_id. A device whose client_id does not match discards the event silently.

Identity verification. Cache from member list; verify against KT log on first use per call, per participant.

Ordering. The client buffers ICE candidates until the offer/answer is processed. The server does not guarantee ordering.

Rate limit. RATE_CALL_SIGNAL_PER_MIN=120 per user per call.

8.11 Session Signaling Encryption

Scope. Per-session. Shared key among all participants.

Key derivation:

```
session_signal_key = HKDF-Expand(RMK, "session-signal-v1" || session_id, 32)
```

· Hash: SHA-256.
· Info: "session-signal-v1" followed by the UTF-8 session_id, no separator, no null terminator.
· Output: 32 bytes.

Envelope plaintext:

```json
{
  "session_id": "...",
  "sender_user_id": "...",
  "sender_client_id": "...",
  "signal_type": "...",
  "payload": { ... }
}
```

Encryption:

Field Value
Encryption AES-256-GCM
Nonce 12 bytes, random per message
Associated data session_id \|\| sender_user_id \|\| sender_client_id \|\| (target_client_id or "")
Signature Ed25519 over nonce \|\| ciphertext \|\| tag, using the sender's MLS identity private key
Wire format signature (64) \|\| nonce (12) \|\| ciphertext \|\| tag (16)

This is the session signaling pattern. It differs from the length-prefixed pattern used for bot messages and publisher keys (Server §8.10). Fixed-length fields (nonce, tag) bracket the variable-length ciphertext, so no length prefix is needed. This is the only context that uses this pattern.

Sender verification. Fetch identity_pubkey from the member list, verify against the KT log on first use per session, cache for the session duration.

Extension interface. Extensions see { signal_type, payload, sender_user_id, sender_client_id }. target_client_id is used by the infrastructure for routing.

Confidentiality statement (spec-level, must be reproduced in the UI where relevant):

Session signaling keys are derivable by all room members. Confidentiality from non-participants depends on the server's delivery gating. A non-participant who obtains an envelope by other means can decrypt it. Signaling is not confidential from other participants.

Rate limit. RATE_SESSION_SIGNAL_PER_MIN=120 per user per session.

8.12 Preferences Encryption

Key derivation:

```
preferences_key = HKDF-Expand(token, "preferences-encryption-v1", 32)
```

Encryption:

Field Value
Encryption AES-256-GCM
Nonce 12 bytes, random per value
Associated data Full preference key, UTF-8 (including ext:, _system:, room: prefixes)
Wire format nonce \|\| ciphertext \|\| tag, base64url unpadded

Size limits:

· Plaintext effective limit: ~96 KB.
· Encrypted limit: 128 KB (server-enforced).
· Overhead: 12 (nonce) + 16 (tag) = 28 bytes.

The client enforces the plaintext limit before encryption. Exceeding it fails with a clear error.

Local mirror. Plaintext in SQLite, encrypted at rest by the device key.

Search. Client-side only.

8.13 Bot Settings Encryption

Settings use X25519 ECDH to bot_command_pubkey. The client does not hold the bot's identity private key and cannot derive a shared key from it.

Flow for PATCH /users/me/bots/:bot_id/settings/:key:

1. Fetch bot_command_pubkey from the KT log and verify it.
2. Generate an ephemeral X25519 keypair.
3. shared = ECDH(ephemeral_private, bot_command_pubkey).
4. key = HKDF-Expand(shared, "bot-settings-v1", 32) (SHA-256).
5. Encrypt the setting value with AES-256-GCM under key.
6. Send { is_secret, ephemeral_pubkey, value_encrypted_bot, value_encrypted_client? }.
7. The bot decrypts with its bot_command_private_key and the ephemeral_pubkey.

Non-secret settings:

· value_encrypted_client uses the operator's preferences_key (standard preferences pattern, no ECDH). Client-readable on every device.
· value_encrypted_bot uses X25519 ECDH. Bot-readable.

Secret settings:

· value_encrypted_bot only, using X25519 ECDH.
· No client-readable copy. No local plaintext mirror.

UI consequences:

· No _meta cache of secret values.
· No autocomplete of secret values.
· No reveal or copy action.

---

9. Media Pipeline

Unchanged from v1.0 §9.

---

10. Attachments

Unchanged from v1.0 §10, plus one row:

Operation Scope Permission
Bot avatar Room-scoped grant required for file settings Bot owner

---

11. Timeline (Message Thread)

Unchanged from v1.0 §11, with these additions.

11.1 Bot Message Rendering

· Bot messages render with the bot's avatar (fileId preferred, emoji fallback, default glyph).
· Display name is plaintext. No encryption.
· A "Bot" chip follows the display name.
· Deleted bots render as <name> (deleted).
· Bot messages are subject to the same edit window and delete rules as user messages, within the bot's granted capabilities.
· Read receipt: rendered from read_by_count on the message record returned by GET /rooms/:id/messages, updated live by read.count events.

11.2 Local Message Rendering

· Accent-tinted bubble.
· Right-aligned, faded.
· Label: "Only you can see this".
· Speaker attribution per §29.
· Persisted until dismissed.
· Searchable in local search.
· Excluded from room counts, unread state, and mention totals.

11.3 Local Message Dismissal

· Swipe left (mobile).
· Hover action (desktop).
· Long-press (both).
· Bulk dismiss: "Clear local messages in this room" in the room menu.
· No auto-expiry.
· Encrypted at rest by the device key.

11.4 Whisper — Deferred

Not in v2.0. The visibility level is documented in §29 for reference. No UI, no storage, no encryption target.

---

12. Composer

12.1 Controls

```
[＋] [😊] [🔊] [    Message...    ] [🎤/➤]
```

Unchanged from v1.0 §12.1.

12.2–12.6

Unchanged from v1.0 §12.2 through §12.6.

12.7 Extension Slots on the Composer

Unchanged, with one note: extensions continue to interact via chat.composer.* slots, but no longer touch crypto directly. The infrastructure handles encryption when the extension's output is a room_message.

12.8 Slash Command Trigger

Rules:

· / at position 0 opens the palette.
· The identifier matches ^[a-z][a-z0-9-]*.
· Filtering begins as soon as the first character after / is typed.
· A space as the first character after / closes the palette; the text is not a command.
· // is an escape and renders as /. Not a command.
· A leading space before / disables the trigger.

Command history:

· Per-room.
· Capped at 50 entries.
· Persisted locally.
· Accessed with Arrow Up in an empty composer.
· Distinct from the palette.

12.9 Slash Command Palette

Dedicated component: <slash-command-palette>.

Row anatomy:

```
┌─────────────────────────────────────────────┐
│ /help                                core   │
│ List every available command                │
└─────────────────────────────────────────────┘
```

Element Required
Name Yes
Description No
Source tag (core / ext / bot / session) Yes
Arg hint No
Subcommand indicator (›) Conditional

Rows without descriptions render name only.

Grouping:

```
CORE
  /help
  /status
  /theme
  /leave
  /whoami
  /commands

THIS ROOM'S EXTENSIONS
  /roll
  /poll

THIS ROOM'S BOTS
  /github
  /rss
```

Group headers are sticky. Empty groups are omitted. Order: Core, Extensions, Bots, Sessions. Alphabetical within groups.

States:

State Render
Open, unfiltered All commands, grouped
Filtering Rows filtered by name and description
No matches "No commands match" + suggestion to try /help
Subcommand Rows replaced with subcommands
Dismissed Palette closes; text remains

Interactions:

Input Behavior
Arrow up/down Move selection
Arrow right / Enter on parent Drill into subcommands
Arrow left / Backspace at empty Return to parent
Enter on leaf Insert + trailing space, close
Tab Same as Enter
Escape Close, keep text
Space as first char after / Close, text is not a command

Filtering:

· Prefix match on name preferred.
· Substring on description.
· Ranking: exact name > name prefix > name substring > description substring.
· Case-insensitive.
· No fuzzy matching.

Selection memory: Per-room, per-session. Cleared on input reset.

Accessibility:

· listbox with aria-activedescendant.
· Rows are option.
· Group headers are presentation.
· Screen reader announces name, source, description.

Non-goals:

· Not a general autocomplete for @ mentions.
· Not extensible directly by extensions.
· Not themeable per extension.
· Not a history browser.

12.10 Core Commands

Command Purpose
/help List every available command, grouped by source
/commands Alias for /help
/theme Switch light/dark/auto
/status Connection, encryption, sync, outbox, storage, versions
/leave Leave the current room
/whoami Username, user_id, truncated lookup_token prefix

/help is the load-bearing discovery surface.

12.11 /status Rendering

Rendered as a local message with a structured card. Snapshot at invocation. Copyable as plain text.

```
CONNECTION
  Status          Connected
  Since           14m 22s ago
  Reconnects      0

ENCRYPTION
  MLS epoch       42
  Pending adds    0
  Pending removes 1
  Key packages    18 / 20

SYNC
  User cursor     seq 12,488 (2m ago)
  Room cursor     seq 3,102 (live)
  Last delta      14s ago

OUTBOX
  Queued          0
  Failed          0

STORAGE
  SQLite          142 MB
  Blob cache      1.8 GB

VERSIONS
  App             2.0.0
  Server          3.0.0
  CoreCrypto      10.5.2
```

Room-scoped fields show — if no room is selected.

12.12 /whoami Rendering

```
Username        coral-dolphin-lagoon-4729
User ID         u_abc123
Token prefix    a1b2c3d4
```

Footnote (shown in the card, not as Kiko):

The token prefix is stable for your account and useful for support. It has the same enumeration properties as your username.

12.13 /leave Behavior

Same rules as the leave button. Owner must transfer ownership first, unless sole member (then the room is deleted).

12.14 /theme Behavior

Cycles light → dark → auto → light.

---

13. Rooms

Unchanged from v1.0 §13, with these additions.

13.1 Member List with Bots

GET /rooms/:id/members returns a merged array:

```json
{
  "members": [
    { "type": "user", "user_id": "u_...", "role": "member", "joined_at": "..." },
    { "type": "bot", "bot_id": "b_...", "mode": "write_only" | "observer" | "member",
      "display_name": "GitHub Bot", "avatar_file_id": "...", "joined_at": "..." }
  ],
  "next_cursor": "..."
}
```

The client renders bots with the "Bot" chip. Bots have no role selector. Removing a member bot queues a pending_mls_remove.

The field is mode, with values write_only, observer, member. The derived mode is computed from granted scopes and is not displayed to the user. It is used internally for routing only.

13.2 Bot Grant Flow

Entry: Room Settings → "Bots" section → "Add bot."

Step 1 — Select a bot. List of installed bots. Each shows label and avatar.

Step 2 — Capability checklist. The declared capabilities from GET /bots/:id. Each is a checkbox with a plain-language label. The client enforces the scope-dependency rules from §27.2:

· Checking post_reaction auto-checks read_content.
· Unchecking read_content unchecks post_reaction, edit_message, delete_message.

The derived mode is computed client-side from the checked set. The mode is not shown. The client sends the granted scope set.

Step 3 — Confirm. POST /rooms/:id/bots with the granted scopes. Server re-derives and validates. On success: bot.added on the room channel.

For member mode, the server queues pending_mls_adds. The standard MLS add flow proceeds.

13.3 Username Lookup

Unchanged from v1.0 §13.4, with one change: the client sends lookup_token instead of username_token.

13.4 Room Metadata JSON

Unchanged.

13.5 Room Avatar Upload

Unchanged.

13.6 Room List Ordering

Unchanged.

13.7 Conversation Row Content

Unchanged, with one addition:

Session indicator (headphones icon + count) unchanged. Bot indicator: if a room has a bot grant, the room header shows a small bot glyph with count. It is not a presence indicator; it just shows that a bot is granted.

13.8 Leaving and Deletion

Updated:

· Leave as a user — room disappears immediately. No pending state. No "leaving…" spinner.
· Owner leave — rejected unless ownership transferred. Sole member leave is a delete.
· Multi-device leave — leave is user-scoped. All devices leave together via session.revoked on the user channel.
· Rejoin — cancels any pending remove. Fresh Welcome. Client discards old MLS state.

---

14. Curation Views

Unchanged from v1.0 §14.

---

15. Calls and Sessions

15.1 Call Model

Unchanged.

15.2 Pre-Call Preview

Unchanged.

15.3 Incoming Call

Unchanged.

15.4 In-Call Screen

Unchanged layout, plus:

· Call join: POST /rooms/:id/calls/:call_id/join with { client_id }.
· Call leave: POST /rooms/:id/calls/:call_id/leave with { client_id }.
· On join, the client generates an ephemeral X25519 keypair and sends the public key, signed, via POST /rooms/:id/calls/:call_id/signal. The server relays on private-user-{target_user_id}.
· Signaling via POST /rooms/:id/calls/:call_id/signal with the envelope from §8.10.
· On leave, the client removes its ephemeral private key and call key.

15.5 Minimized Call

Unchanged.

15.6 Call History

Call history is fetched via GET /rooms/:id/calls?limit=&cursor=. Visibility: all room members. Retention: CALL_RECORD_RETENTION_DAYS.

On call.started, the client appends to local history. On call.ended, the client updates the entry with ended_at and duration_seconds. On room open, the client reconciles from the REST fetch.

Per-participant data is not displayed. Call history remains profile, room, type, direction, duration, timestamp.

15.7 Sessions

Unchanged from v1.0 §15.7.

15.8 Session Client Behavior

Updated for the envelope.

· Send: POST /rooms/:id/sessions/:session_id/signal with { sender_client_id, target_client_id?, envelope }.
· Envelope construction and verification per §8.11.
· Roster fetch gated on participation.
· Heartbeat every 15s.
· Leave with { client_id }.
· Room deletion: tear down media on session.deleted.

15.9 Sessions — Prohibitions

Unchanged from v1.0 §15.9.

15.10 Session Rate Limits

Unchanged.

15.11 Sessions Feature Disabled

Unchanged.

15.12 Other Session Types

Unchanged, with a note: session type declaration includes metadata and signaling schemas that describe the decrypted payload shapes. The infrastructure enforces them.

15.13 Session Confidentiality Statement

The client must surface the statement from §8.11 wherever a user is deciding whether to trust session signaling. Placement:

· The session create sheet, in a collapsible "How this works" disclosure.
· A persistent ? icon in the in-session bar that opens the same disclosure.

Content (spec-level, exact text):

Session signaling is encrypted so the server cannot read it. It is not confidential from other participants — anyone in this session can decrypt any signal. If you would not share something with everyone in this session, do not send it through session signaling.

No other placement. Not in settings. Not in a help article.

---

16. Auth

16.1 Auth Gate

Unchanged.

16.2 Login View

Flow updated:

1. Blind the typed username → POST /oprf/blind → unblind → token.
2. Derive lookup_token.
3. POST /auth/login/start with { lookup_token, opaque_client_auth_state }.
4. OPAQUE AKE locally.
5. POST /auth/login/finish with { lookup_token, ke3 }.
6. Store session token.
7. Cache token and lookup_token in memory.
8. Redirect to /app.html.

16.3 Register View

Two-mode registration.

Mode 1 — Choose your own.

Fields:

· Invite code (8-char Crockford Base32).
· ALTCHA widget.
· Username.
· Display name.
· Password.
· "Create account."

Validation:

· Minimum length: 6 characters.
· Dictionary check against top 100,000 common names and words.
· Warning, not block. Dismissible.
· No entropy enforcement.
· No complexity rules.

Mode 2 — Generate one.

1. Theme picker (ocean, forest).
2. Five generated candidates (four words + four digits, hyphenated, lowercase).
3. Select one or reroll.
4. Confirm.

Example: coral-dolphin-lagoon-4729.

Save screen (mandatory).

After generating, the user sees a "save this" screen. "Next" is disabled until:

1. The user taps Copy or Download.
2. The user types the last two words back.

Content:

· Username displayed prominently.
· Copy to clipboard button.
· Download as .txt button.
· Confirmation input.

The generated username is shown permanently in Settings → Account → Profile.

Registration flow (both modes):

1. Blind username → token → lookup_token.
2. Derive display_name_key.
3. Encrypt display name.
4. OPAQUE registration locally.
5. POST /auth/register/finish with { lookup_token, encrypted_display, opaque_record, identity_pubkey, altcha }.
6. Server returns session_token, user_id, recovery codes (plaintext, shown once).
7. Display recovery codes. Download mandatory. Confirmation step: type the 3rd code back.
8. Cache token and lookup_token in memory.
9. Redirect to /app.html.

16.4 Recovery View

Flow updated:

1. Blind username → token → lookup_token.
2. POST /auth/recover/start with { recovery_code, lookup_token }.
3. Server verifies code. Returns OPAQUE registration challenge.
4. Client runs OPAQUE registration with the new password.
5. POST /auth/recover/finish with { lookup_token, recovery_session, opaque_record, encrypted_display, identity_pubkey }.
6. Server replaces OPAQUE record, consumes code, revokes all existing sessions, issues new session.
7. Client stores new session, redirects to /app.html.

Other devices receive session.revoked and clear local state.

16.5 Biometric Unlock

Unchanged.

16.6 Device Linking

Unchanged, plus one note: device names are encrypted with device_name_key derived from token (not lookup_token).

16.7 Passkeys (V2 Seam)

Unchanged.

16.8 Session Expiry

Unchanged.

16.9 Multi-Device Session Semantics

Unchanged.

16.10 Multi-Device Read State Sync

Unchanged.

---

17. Settings

17.1 Account

Row Control
Profile Navigate → display name, avatar, username (read-only), generated indicator
Password Navigate → change password
Recovery codes Navigate → status, regenerate
Devices Navigate → list, revoke
Biometric unlock Toggle
Passkeys Navigate → V2 placeholder
Starred items Navigate → list

Display name update: re-encrypt with display_name_key, fresh nonce. PATCH /users/me { encrypted_display }.

17.2 Privacy

Unchanged from v1.0, with one addition:

Row Default
Session confidentiality notice Shown in session create sheet

17.3 Notifications

Unchanged. Bot messages respect the same categories. Bot DMs do not exist in v3.0.

17.4 Chats

Unchanged, plus:

Row Default
Command history Per-room, 50 entries

17.5 Media & Storage

Unchanged.

17.6 Transcription (STT)

Unchanged.

17.7 Read Aloud (TTS)

Unchanged.

17.8 Calls

Unchanged.

17.9 Appearance

Unchanged.

17.10 Extensions

Updated:

Row Purpose
Installed extensions List with enable/disable toggle
Network permissions Per-extension origins
Session types Declared session types
Commands Declared commands

17.11 Bots

New section.

Row Purpose
Owned bots List of bots created by this user
Bot settings Per-bot: non-secret settings (readable, editable), secrets (masked, "Set" + "Replace")
Bot avatar Upload / remove (owner only)
Bot tokens Issue / revoke
Delete bot Type-to-confirm

Bot settings renderer is the same declarative field renderer used for extensions.

Encryption. Bot settings writes use the X25519 ECDH flow from §8.13. The PATCH request includes ephemeral_pubkey. The non-secret value_encrypted_client uses the operator's preferences_key. The secret value_encrypted_bot has no client-readable copy on any device.

Secret UI — normative. Secret settings are write-only. The client MUST NOT:

· Store a plaintext mirror of any secret value.
· Autocomplete secret values.
· Reveal secret values.
· Copy secret values to the clipboard.
· Log secret values.

The client MAY:

· Display the field as set (•••••••• with a "Set" label).
· Offer a "Replace" action that opens a secure input.
· Show the timestamp of the last set.

The plaintext exists only while the user is entering or replacing it. It is discarded from memory when the input is committed or dismissed. A feature that reveals, copies, or caches a secret value is a spec violation.

Token handoff:

1. Owner taps "Issue token" in bot settings.
2. Client calls POST /bots/:id/tokens.
3. Server returns { token_id, token }. The token is shown once.
4. Client displays a copy-to-clipboard action and a "Download .env" action.
5. Owner pastes the token into the bot process environment (ATOL_BOT_TOKEN).
6. Client does not persist the plaintext token beyond the display moment.
7. Token revocation: DELETE /bots/:id/tokens/:token_id from the client.

17.12 Starred Items

Unchanged.

17.13 Administration

Unchanged.

17.14 Kiko

New section, under Accessibility.

Row Default
Kiko mode kiko (auto-detects screen reader → plain)
Override Kiko mode User can change at any time

Three modes:

· kiko — Kiko speaks. Avatar + framing line.
· plain — same content, no character. Speaker is "System".
· off — system messages collapse to inline text.

Preference key: kiko_mode with values "kiko" | "plain" | "off". Synced via user_preferences (encrypted).

17.15 Sign-Out and Account Deletion

Updated:

· Log out: session cleared, local data preserved.
· Log out and clear data: everything cleared, including bot grants cache, owned-bots cache, and bot settings mirror.
· Delete account: warning, type username to confirm, server anonymizes, local wipe, redirect.

Owner's bots: on account deletion, bots are deleted. Their messages render as (deleted bot).

---

18. Notifications

Unchanged from v1.0 §18, plus:

· Bot messages notify like user messages, subject to room mute and mention rules.
· Local messages never notify.
· Session events never notify.
· Call events continue to notify (existing behavior).

---

19. Cross-Platform

Unchanged from v1.0 §19.

---

20. Empty States

Unchanged, plus:

Surface Variant Copy
Bots in a room empty "No bots in this room. Add one to get started."
Owned bots empty "You don't own any bots yet."
Local messages empty "No local messages in this room."
Command history empty (No UI — Arrow Up does nothing)

---

21. Error Handling

21.1–21.6

Unchanged.

21.7 MLS Errors

Updated:

· Out-of-sync epoch → "Syncing encryption state…".
· Missing welcome → poll every 5s; after 5 min, "Waiting for a room member to add you".
· Pending MLS remove → no UI. The room disappears on leave. No "leaving…" state.
· Keystore corruption → wipe, mark rooms error, offer rejoin.
· Welcome fetch (GET /welcomes) fails → retry with backoff. App works from local state.

21.8 Session Errors

Unchanged.

21.9 Read-Aloud Errors

Unchanged.

21.10 Extension Errors

Unchanged.

21.11 Bot Errors

Error Behavior
Bot offline past command TTL Command silently dropped. Client may show "bot did not respond" if the user opened a tracking UI.
Bot throws on handler Logged locally. User sees "The bot could not process this command."
Bot paused by runtime Owner sees a local message. Other users see nothing.
Command encryption failure Client-side error. "Could not send command."
KT verification failure Command rejected. "Could not verify the bot's key."
Grant update to member Standard MLS add flow. No user-visible state.
Grant update away from member Standard MLS remove flow. Bot disappears from member list immediately.
bot.deleted received Evict bot from owned-bots cache. Evict all grants for the bot. Bot settings mirror removed. Messages render as (deleted bot) on next view.
bot.created received Add to owned-bots cache.

21.12 Interaction Edge Cases

Unchanged, plus:

Scenario Behavior
Local message in a room the user leaves Local messages remain in local storage; the room shows as left.
Bot removed, then re-added New grant. Old messages retained with sender_bot_id.
Bot deleted, messages retained Messages render as (deleted bot).
Secret set, then device restore from backup Secret is not in any backup. Operator must re-enter.

---

22. Plugin Inventory

Updated:

Plugin Options Purpose
extension-plugin extensions Unchanged.
state-plugin initialState Unchanged.
router-plugin — Unchanged.
socket-plugin socketUrl, apiBase, appKey Unchanged.
crypto-plugin wasmPath, thresholds Unchanged.
oprf-plugin apiBase Derives token + lookup_token.
media-plugin presets, workerPoolSize Unchanged.
transcription-plugin STT config Unchanged.
speech-plugin TTS config Unchanged.
push-plugin vapidKey, apnsConfig, fcmConfig Unchanged.
storage-plugin dbName, blobRoot Unchanged.
i18n-plugin locales, defaultLocale Unchanged.
theme-plugin presets, accent Unchanged.
call-plugin iceServers, transport Includes ephemeral ECDH handling.
session-plugin iceServers, heartbeatInterval Includes envelope encryption/signing.
p2p-plugin iceServers, chunkSize Unchanged.
icon-plugin styleMap, registry Unchanged.
notification-plugin defaultSound Unchanged.
search-plugin threshold, debounceMs Also indexes local_messages.
bot-plugin commandEncryption, settingsEncryption Client-side command encryption and bot settings encryption. Does not host a bot runtime.
command-plugin registry Slash command registry and palette.
kiko-plugin defaultMode System voice mode, avatar states, locale bridge.

---

23. Appendices

23.1 SQLite Schema

Updated domains and tables. Additions:

```sql
-- Local messages
CREATE TABLE local_messages (
    id           TEXT PRIMARY KEY,
    room_id      TEXT NOT NULL,
    speaker      TEXT NOT NULL,   -- 'core' | 'extension:<id>' | 'bot:<id>'
    content      TEXT NOT NULL,
    created_at   INTEGER NOT NULL,
    dismissed_at INTEGER,
    searchable   INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX idx_local_messages_room ON local_messages(room_id, created_at);
CREATE INDEX idx_local_messages_undismissed ON local_messages(room_id)
    WHERE dismissed_at IS NULL;

-- Bot grants cache (server is source of truth; local for offline)
CREATE TABLE bot_grants (
    room_id     TEXT NOT NULL,
    bot_id      TEXT NOT NULL,
    mode        TEXT NOT NULL CHECK(mode IN ('write_only', 'observer', 'member')),
    scopes      TEXT NOT NULL,   -- JSON array
    granted_at  INTEGER NOT NULL,
    PRIMARY KEY (room_id, bot_id)
);

-- Owned bots cache
CREATE TABLE bots (
    bot_id          TEXT PRIMARY KEY,
    label           TEXT NOT NULL,
    avatar_file_id  TEXT,
    emoji           TEXT,
    owner_user_id   TEXT NOT NULL,
    capabilities    TEXT NOT NULL,   -- JSON array
    updated_at      INTEGER NOT NULL
);

-- Command history
CREATE TABLE command_history (
    room_id     TEXT NOT NULL,
    command     TEXT NOT NULL,
    args        TEXT,
    used_at     INTEGER NOT NULL,
    PRIMARY KEY (room_id, used_at)
);
CREATE INDEX idx_command_history_room ON command_history(room_id, used_at DESC);

-- Bot settings local mirror (non-secret only)
CREATE TABLE bot_settings_client (
    bot_id          TEXT NOT NULL,
    key             TEXT NOT NULL,   -- includes room:{room_id}: prefix when room-scoped
    is_secret       INTEGER NOT NULL,
    value_plaintext TEXT,            -- null when is_secret = 1
    user_seq        INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    PRIMARY KEY (bot_id, key)
);

-- Room publisher keys local cache
CREATE TABLE room_publisher_keys (
    room_id              TEXT NOT NULL,
    epoch                INTEGER NOT NULL,
    publisher_public_key BLOB NOT NULL,
    signer_user_id       TEXT NOT NULL,
    signature            BLOB NOT NULL,
    verified_at          INTEGER,
    PRIMARY KEY (room_id, epoch)
);
```

Room-scoped bot settings use the room:{room_id}: prefix in the key column. No scope_room_id column exists.

Every other table is unchanged. _meta gains:

· _meta.last_user_seq — also covers bot_settings, bot.created, and bot.deleted changes.
· _meta.kiko_mode — mirrored locally for fast read.

Preferences: local mirror is plaintext, encrypted at rest by the device key. Server stores ciphertext.

23.2 Design Tokens

Unchanged. Kiko tokens are aliases of existing accents.

23.3 Event Catalog

Room channel — private-room-{room_id}. Best-effort delivery. Clients reconcile via REST.

Event Payload
message.new { id, room_id, sender_type, sender_id, sender_client_id, epoch, seq, content_type, reply_to, bot_key_leaf_index?, created_at }
message.edited { id, edit_of, edit_sequence, room_id, sender_type, sender_id, created_at }
message.deleted { id, room_id }
reaction.added { id, room_id, message_id, sender_user_id, reaction, created_at }
reaction.removed { id, room_id, message_id }
room.updated { room_id, metadata?, retention_days?, max_file_size_bytes? }
room.member_added { room_id, user_id, role, joined_at }
room.member_removed { room_id, user_id }
epoch.updated { room_id, epoch, sequence }
mls.add_pending { room_id, target_user_id?, target_bot_id?, client_ids }
mls.remove_stale { room_id, target_user_id?, target_bot_id?, queued_at, stale_since }
mls.remove_confirmed { room_id, target_user_id?, target_bot_id?, confirmed_at }
call.started { call_id, room_id, initiator_id, started_at }
call.ended { call_id, room_id, ended_at, duration_seconds }
session.created { room_id, session_id, extension_id, session_type, created_by, created_at }
session.updated { room_id, session_id, metadata?, metadata_version?, position?, changed }
session.deleted { room_id, session_id, extension_id }
session.occupancy { room_id, session_id, participant_count }
room.publisher_key_updated { room_id, epoch, publisher_public_key, signer_user_id, signature }
read.count { room_id, message_id, read_by_count }
bot.added { room_id, bot_id, mode, scopes, granted_by }
bot.revoked { room_id, bot_id }
bot.updated { room_id, bot_id, scopes?, avatar_file_id?, changed }
bot.deleted { room_id, bot_id, deleted_at }

Notes:

· message.new for bot messages includes bot_key_leaf_index (present iff sender_type = "bot"; absent on user messages). The field is the KT log leaf index of the bot's signing key at publish time.
· call.ended.duration_seconds is an integer, in seconds, truncated toward zero. Always present. Always non-null. Computed server-side.
· room.publisher_key_published is removed. Merged into room.publisher_key_updated.
· read.count is non-durable. Read counts are reconciled from read_by_count on GET /rooms/:id/messages.
· Whisper messages (target_user_ids non-null) are delivered on each recipient's private-user-{user_id} channel and on the sender's own private-user-{sender_user_id} channel as a message.new event. The payload is identical to the room-channel message.new, with target_user_ids included. The sender's sending device filters its own event by comparing sender_client_id against its own client_id. Other devices of the sender apply the event for read-state consistency. No whisper is published on the room channel. The server strips target_user_ids from any room-channel payload as a defensive measure.

User channel — private-user-{user_id}. Durable events caught up via GET /users/me/sync?since_seq=N.

Event Payload Durable
read.sync { room_id, last_read_message_id, user_seq } Yes
room_order.sync { room_ids, user_seq } Yes
device.added { device_id, platform, added_at, user_seq } Yes
device.revoked { device_id, reason, user_seq } Yes
device.name_updated { device_id, encrypted_device_name, user_seq } Yes
session.revoked { session_id, reason } No
user.updated { user_id, profile_version, user_seq } Yes
call.signal { call_id, sender_user_id, sender_client_id, target_client_id, envelope } No
session.signal { room_id, session_id, sender_user_id, sender_client_id, target_client_id?, envelope } No
starred_item.added { item_id, item_type, room_id, user_seq } Yes
starred_item.removed { item_id, item_type, user_seq } Yes
preference.updated { key, user_seq } Yes
bot_settings.updated { bot_id, key, user_seq } Yes
bot.created { bot_id, display_name, created_at } Yes
bot.deleted { bot_id, deleted_at } Yes
mls.remove_confirmed { room_id, target_user_id?, target_bot_id?, confirmed_at, user_seq } Yes
mls.welcome_ready { room_id, welcome_id } No
room.publisher_key_updated { room_id, epoch, publisher_public_key, signer_user_id, signature } No
kt.snapshot { tree_size, root_hash, created_at } No
bot.paused { bot_id, reason } No
account.disabled { reason } Yes
account.deleted {} Yes
room.transfer_initiated { room_id, from_user_id, transfer_id } Yes
room.transfer_accepted { room_id, from_user_id, to_user_id, accepted_at } Yes
room.transfer_cancelled { room_id, transfer_id, reason } Yes

Notes:

· room.transfer_initiated is delivered on private-user-{recipient_user_id}.
· room.transfer_accepted is delivered on both private-user-{from_user_id} and private-user-{to_user_id}.
· room.transfer_cancelled is delivered on private-user-{recipient_user_id} and on private-user-{owner_user_id} for the owner's own multi-device sync.
· bot.paused is delivered only to the bot's owner_user_id. Not on the bot channel (the bot is paused and cannot process events).
· kt.snapshot is non-durable. Clients fetch GET /kt/snapshot on foreground activation as a fallback, throttled to at most one fetch per 5 minutes.

Bot channel — private-bot-{bot_id}.

Event Payload
bot.command_invoked { command_id, room_id, sender_user_id, sender_client_id, ciphertext }
bot.settings_updated { bot_id, keys_changed, user_seq }
bot.grant_updated { bot_id, room_id, old_mode, new_mode, scopes, changed }
bot.updated { bot_id, avatar_file_id?, changed }
bot.keys_rotated { bot_id, rotated_at }
room.publisher_key_updated { room_id, epoch, publisher_public_key, signer_user_id, signature }

Notes:

· command_name is inside the ciphertext, not a payload field.
· bot.paused is not on this channel. See user channel.
· room_id is absent on bot.updated for the bot channel.

Client events (Sockudo, ephemeral):

Event Payload
client-typing.start { user_id }
client-typing.stop { user_id }

Removed: client-read (read state via REST). client-mls-request (MLS add coordinated via mls.add_pending).

23.4 Server Assumptions

The client implements against Server Specification v3.0. All items previously listed as blocking have been resolved and incorporated into v3.0. No blocking items remain.

---

24. Design Decisions Log

Existing entries from v1.0 §24 (1–30) are retained. New decisions:

# Date Decision
31 2026-10-04 lookup_token split. Client derives both token and lookup_token. token memory-only. lookup_token sent to server.
32 2026-10-04 Call signaling on user channels. Ephemeral ECDH. Call key rotation on join/leave.
33 2026-10-04 Session signal envelope: signature (64) ‖ nonce (12) ‖ ciphertext ‖ tag (16). Shared key. Honest confidentiality statement.
34 2026-10-04 All preference values encrypted. Plaintext limit ~96 KB, ciphertext limit 128 KB. room_order moved to dedicated server table.
35 2026-10-04 Pending MLS remove: room disappears on leave. mls.remove_stale for remaining members. Two-member residual documented.
36 2026-10-04 Bots are first-class identities. No tier field. Mode derived from granted capabilities.
37 2026-10-04 Capability vocabulary: 8 scopes with dependency rules. Author declares ceiling. Room owner grants subset.
38 2026-10-04 Bot avatar: { fileId?, emoji? }. File primary. Emoji fallback. POST /bots/:id/avatar.
39 2026-10-04 Bot settings: double-encrypt for non-secret. Bot-only for secret. No client-readable copy of secrets. No local plaintext mirror.
40 2026-10-04 Publisher key derivation: MLS-Exporter("atoll", "publisher-key-v1", 32). Signed publishes. Verified by bot and clients.
41 2026-10-04 Composer commands: unified registry. Core (6), extension, session, bot. Dedicated <slash-command-palette> component.
42 2026-10-04 Local messages: first-class. Persisted locally. Searchable. No auto-expiry. Never transmitted.
43 2026-10-04 Kiko system voice: 3 states, 3 modes, kiko.* locale namespace. Scripted only in v2.0.
44 2026-10-04 Username generation: two-mode registration. 6-char min for chosen. 7,776-word lists. Ocean + forest. Mandatory save screen.
45 2026-10-04 /whoami included. Core command count: 6.
46 2026-10-04 Bot runtime is an external Node.js process. Client hosts only command and settings encryption, install UI, token handoff, and rendering.
47 2026-10-04 Bot settings encryption uses X25519 ECDH to bot_command_pubkey.
48 2026-10-04 Room-scoped bot settings use the room:{room_id}: key prefix. No server-side scope column.
49 2026-10-04 Call history fetched via GET /rooms/:id/calls.
50 2026-10-04 Publisher keys persist server-side in room_publisher_keys.
51 2026-10-04 Session signaling signing is the exception to length-prefixed encoding.
52 2026-10-04 Event catalog frozen. Both specs reference the canonical table.
53 2026-10-04 bot_key_leaf_index is present on message.new iff sender_type = "bot". Absent on user messages.
54 2026-10-04 call.ended.duration_seconds is integer seconds, truncated toward zero. Always present.
55 2026-10-04 connected field is server-enforced owner-only. No client-side filter.
56 2026-10-04 Whisper edit, delete, and reaction events route to user channels only, same as the original message.new.
57 2026-10-04 Read counts render from read_by_count on the message record, updated live by read.count events.
58 2026-10-04 kt.snapshot is non-durable; GET /kt/snapshot is the authoritative fallback.
59 2026-10-04 Self-remove proposal removed. Leave relies on pending_mls_removes only.

---

25. Document Status

This is the contract for the client side of the system. Every client implementation task references this document. If a task conflicts with this spec, the task is wrong and must be revised.

Design decisions are tracked in §24.

---

26. Extension System

Unchanged from v1.0 §26, with these additions:

26.3 Extension Object — new fields

```js
{
  // ...v1.0 fields...

  commands: [ /* CommandDecl[] */ ],   // new
  settings: [ /* SettingDecl[] */ ],   // replaces SettingsDecl's component as default
}
```

26.5 Slash Command Registration

Extensions register commands via the commands field. Each command has name, description, args, handler. Command names are namespaced by extension ID internally. Collisions with core commands are rejected at build time.

26.7 Events — unchanged, plus bot interaction

Extensions may emit bot:command events to trigger bot commands on their behalf. The infrastructure encrypts and routes.

26.8 Session Types — schema change

The metadata and signaling schemas describe the decrypted payload shape. The infrastructure enforces them. Extensions no longer touch crypto.

26.9 Preferences — all values encrypted

Extension preferences sync via the server. All values are encrypted with preferences_key before transmission. The ext:{extensionId}: prefix is preserved on the wire. Local mirror is plaintext, encrypted at rest.

Secrets remain device-local. Never synced.

26.13 Build-Time Validation — additions

Commands: name uniqueness, namespace collision with core commands, handler signature.

Settings: key uniqueness, scope validity, field type validity, file type restricted to room scope.

---

27. Bot System

27.1 Overview

Bots are headless long-lived processes that run on the operator's machine. They connect to the server as their own identity. They post to rooms and respond to commands.

A bot is not an extension. Extensions are client-side UI. Bots are server-visible identities.

The critical property: a write-only bot cryptographically cannot read room content. It never holds MLS state, never has the room's encryption material, and never receives ciphertext. Its only outbound interface is POST /rooms/:id/bot-messages, signed, which the server accepts as opaque bytes.

27.2 Capabilities

The client displays capabilities, never modes. Capabilities are declared by the author as a ceiling and granted by the room owner as a subset.

Scope Plain-language label
post_message Post messages
post_attachment Post attachments
post_reaction Add reactions
read_commands Receive commands
read_metadata Read message metadata (who sent what, when, how large)
read_content Read message content
edit_message Edit own messages
delete_message Delete own messages

Dependency rules enforced client-side in the grant UI and re-validated server-side:

· post_reaction, edit_message, delete_message require read_content.

27.3 Grant Flow

See §13.2.

27.4 Member List

See §13.1.

27.5 Bot Messages

See §11.1.

27.6 Bot Settings UI

See §17.11.

27.7 Bot Runtime

The bot runtime is a headless Node.js process managed by the operator. It is not hosted inside the client. @atoll/bot is a Node.js SDK. The client's role is limited to:

· Install UI (capability grant flow).
· Settings UI (declarative fields; secrets set/replace only).
· Command encryption when a user invokes a bot command from their device.
· Message rendering for bot messages.
· Token handoff (POST /bots/:id/tokens → display once → operator copies to the bot process).
· Publisher key publication for rooms the operator is a member of.

The client never runs bot handlers, never opens a WebSocket as the bot, never decrypts commands, and never stores bot private keys.

Authentication: the bot authenticates with its own bot_token obtained from POST /bots/:id/tokens. It opens its own WebSocket connection to Sockudo subscribed to private-bot-{bot_id}. It has no relationship to the operator's user session token.

27.8 Bot Channels

· private-bot-{bot_id} — the bot's own channel, subscribed by the bot runtime.
· private-user-{owner_user_id} — the owner's channel; used for bot.paused, bot.created, and bot.deleted.
· private-room-{room_id} — only for member-mode bots.
· Observer SSE — for observer-mode bots.

27.9 Grant Updates

When a room's grant changes, the bot receives bot.grant_updated on private-bot-{bot_id}. The client displays no user-facing state for the transition.

· If the derived mode changes to member, the server queues pending_mls_adds for the bot. The standard MLS add flow proceeds.
· If the mode changes away from member, the server queues pending_mls_removes. The bot disappears from the member list immediately.
· The bot runtime reconnects after a mode change.

27.10 Rate Limiting

Client respects:

· RATE_BOT_COMMAND_PER_MIN — 60 per bot per user.

27.11 Errors

See §21.11.

27.12 Prohibitions

· Do not display the derived mode.
· Do not promise secret recovery.
· Do not cache secret values.
· Do not auto-reconnect the bot runtime after a fatal error.
· Do not log secret values, command plaintext, or bot settings.
· Do not host the bot runtime in the client.

---

28. Commands and Settings

28.1 The Unified Registry

Settings and commands share one registry. A setting is a command that persists. A command is a setting used once. /help shows everything.

28.2 Command Taxonomy

Kind Handled by Round trip
Core Client None
Extension Client-side extension None
Session Session extension (client-side) Only as session signaling
Bot Bot runtime Full relay via POST /rooms/:id/bot-commands

28.3 Settings Fields

The declarative vocabulary: text, secret, number, boolean, select, multiselect, room-select, user-select, time-range, duration, color, file. file restricted to room scope.

28.4 Setting-to-Command Mapping

Every declared setting source generates one command with subcommands:

· /name — show card.
· /name set key value — set.
· /name get key — read.
· /name reset — reset all.
· /name reset key — reset one.

28.5 Scope Routing

· scope: 'room' → Room Settings → extension section.
· scope: 'user' → Settings → Extensions → extension.

28.6 Secret Handling

Secrets:

· Never appear in command history, message logs, or settings cards.
· Stored encrypted at rest (device key).
· Not synced.
· Set via secure input. Never revealed.

28.7 Discovery

/help groups by source. See §12.9.

28.8 Rendering

local_message is the default output. Public output requires explicit opt-in.

---

29. Kiko System Voice

29.1 Overview

Kiko is the system voice. It frames local messages. It does not pad them.

29.2 Two Kikos

· Scripted Kiko — hand-written strings, ships with the app.
· Conversational Kiko — local LLM, deferred to post-v2.0.

29.3 Three Avatar States

State Asset When
default kiko/default.svg Help, status, confirmations
concerned kiko/concerned.svg Recoverable errors
error kiko/error.svg Serious but non-fatal

Truly fatal errors (crash, keystore corruption) render plain. No Kiko.

29.4 Three Accessibility Modes

Mode Default Behavior
kiko Yes, unless screen reader detected Avatar + framing line
plain Auto-default when screen reader detected Same content, no character. Speaker is "System".
off No System messages collapse

The mode is per-user, synced via kiko_mode preference (encrypted).

Screen-reader auto-detect is not a silent default. The user can override.

29.5 Speaker Attribution

Source Speaker
Core Kiko or plain (per mode)
Extension Extension's declared avatar
Bot Bot's declared avatar

Extensions may opt in to speaking as Kiko.

29.6 Kiko in Local Messages

Kiko's shell reflects the current room's theme (accent color, bubble tint) via existing per-room theming. No additional asset loading.

29.7 i18n

All three modes have locale files. Namespace: kiko.*. Fallback to English.

29.8 Spec Language

A local message is attributed to a speaker. Core application messages are attributed to Kiko. Extension and bot messages are attributed to their declared avatar. The application speaker MAY be configured to render as Kiko, as plain text, or be hidden. Scripted speaker output MUST be deterministic and MUST NOT require a downloaded model.

---

30. Server Coordination Notes

All coordination items raised during drafting have been resolved and are incorporated into Server Specification v3.0. No open questions remain. This section is retained for historical context only.

---

End of Client Specification v2.0.
