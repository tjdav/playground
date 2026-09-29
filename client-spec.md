# Client Specification v1.0

> **Status:** Final — source of truth for client implementation tasks.
> **Scope:** This document describes the client-side application, its architecture, behavior, and contract with the server. Server behavior is defined by the Server Specification v1.0 and its amendment package.
> **Stack:** Coralite 1.0.0-rc.5 (static site generator) · Wire CoreCrypto 10.5.2 (WASM) · Mediabunny · Capacitor · Tauri
> **Design language:** Privacy-first messenger. Adopts proven social-interaction patterns; visual density follows modern Western conventions. Not a clone of any single product.

Amendments are tracked at §24. Any change to this document requires a documented amendment.

---

## 1. Product Summary

A self-hosted, end-to-end encrypted group messaging client. The server is an untrusted delivery service that never sees plaintext, keys, or plaintext user identifiers. The client holds all plaintext, all keys, and all MLS state. Encryption, media processing, and transcription run locally.

| Layer | Technology |
|---|---|
| Framework | Coralite 1.0.0-rc.5 (static site generator) |
| Config loader | `coralite-scripts` |
| Runtime | Node.js ≥ 22.22.2, ESM |
| MLS engine | Wire CoreCrypto 10.5.2 (WASM, bundled) |
| Attachment crypto | Web Crypto API (AES-256-GCM, HKDF, SHA-256) |
| Username lookup | OPRF (server-side `voprf`, client-side OPRF blinding) |
| Media processing | Mediabunny (WebCodecs-first, ffmpeg.wasm fallback) |
| Transcription | Moonshine Tiny / Small, Whisper Tiny (ONNX Runtime Web) |
| Real-time | pusher-js (Sockudo, Pusher Protocol 7) |
| Search | Fuse.js (lazy-loaded) |
| Mobile shell | Capacitor (iOS, Android) |
| Desktop shell | Tauri |
| Storage | SQLite (native on mobile/desktop, WASM+OPFS on web) |
| CSS | Tailwind utilities + Coralite scoped `<style>` |
| Icons | Solar icon set (tree-shaken via `@solar-icons/static`) |
| Emoji picker | `emoji-picker-element` |
| Waveform | `wavesurfer.js` |
| QR codes | `qr-code-styling` |

---

## 2. Scope

### 2.1 In Scope — V1

- Two static pages: `index.html` (auth gate), `app.html` (messenger shell)
- Three-panel desktop layout; stacked mobile layout; tablet two-panel layout
- Room-based conversation model (1:1 = 2-member room; no contact list)
- OPRF-based username lookup (exact-match only)
- Text messaging with Markdown formatting
- Message editing (15-minute window, signed edit chain)
- Reactions (silent — no notification, no read receipt, no unread badge)
- @mentions with autocomplete and mute override
- Disappearing messages (24h / 7d / 90d)
- Read receipts (aggregate count, mutual setting, synced across devices)
- Typing indicators (via client events)
- Message deletion (tombstone via server endpoint)
- Encrypted attachments (C2SP chunked AES-256-GCM)
- Range-request streaming and seeking
- Presigned URL handling (S3 backends)
- P2P fallback for files exceeding room limits (WebRTC data channels)
- Media processing pipeline (image, video, audio)
- Client-side poster and thumbnail generation
- Metadata stripping for all processed media
- Local voice message transcription (Moonshine, Whisper)
- Voice messages with interactive waveform player
- Stickers (bundled + custom + combined) with entrance transitions
- Emoji picker
- Client-generated link previews (with optional server proxy)
- In-thread search (Fuse.js, per-room index)
- Global message search
- Media, Documents, Links curation views (filter + search)
- Room creation, member management, invite links + QR
- Room settings (off-canvas right drawer on desktop; full-screen push on mobile)
- Per-room, per-user theming (theme, wallpaper, bubble style)
- Manual room ordering (drag-and-drop, synced across devices)
- Biometric unlock (Face ID, Touch ID, Windows Hello)
- Recovery code flow (server-generated, server-hashed)
- Device linking for new devices
- Multi-device read state sync
- Key transparency automatic verification
- Calls UI (pre-call preview, in-call screen, history, PiP)
- Push notifications (Web Push, APNs, FCM, UnifiedPush)
- Notification actions (reply, mark as read)
- Offline-first: readable cache, outgoing message queue
- Boot, sync, and delta sync
- Blocked users management
- Privacy screen (app switcher blur)
- App lock timeout
- Deep linking (universal links, custom scheme, push)
- Accessibility (WCAG 2.1 AA)
- Internationalization infrastructure (English V1)

### 2.2 Server Requirements (Client V1 Dependencies)

The following are required by the client's V1 spec and are defined in the server amendment package:

- OPRF-based username lookup (`POST /oprf/blind`, OPRF token storage)
- Encrypted display names (`encrypted_display`)
- `private-user-{user_id}` channel with `user_seq` sequence
- `GET /users/me/sync?since_seq=X` state sync endpoint
- `read.sync` and `room_order.sync` via REST writes
- Device names via user-scoped sync (server stores ciphertext)
- `pending_mls_adds` coordination
- `PATCH /rooms/:id` with `metadata` field
- Key transparency log + auditor signatures
- Link preview proxy (opt-in)
- Message editing endpoint
- Reactions table and endpoints
- Threading via `reply_to`
- `POST /users/me/avatar`
- Call signaling events + TURN credentials
- Member list pagination
- Retention change preview
- Invite metadata preview
- Transfer ownership consent
- Separate recovery flow (`POST /auth/recover/start` + `/finish`)
- Schema naming cleanup

### 2.3 Out of Scope

- SPA routing between pages (each page is a static HTML document)
- Contact/friend list (rooms only)
- Server-side key escrow
- Plaintext export of message contents
- Username changes
- Federation between servers
- Multi-tenancy
- Multiple accounts on one device
- Analytics or crash reporting
- Message forwarding
- Message pinning
- Chat screenshot export
- GIF search (external API)
- User status / away message
- Presence (online/offline indicators)
- Office document inline rendering
- Native keyboard sticker access

---

## 3. Design Principles

1. **Clear primary task** — the messenger is chat-first. Every view serves conversation.
2. **Privacy-first** — the server is untrusted. It holds no plaintext user identifiers, no plaintext messages, and no plaintext room metadata. Security indicators are visible and unambiguous.
3. **Behavior over ornament** — animation and decoration serve function. Decoration is opt-in.
4. **Consistency through position** — navigation and actions appear in predictable places.
5. **Asymmetric cultural adoption** — adopt proven social-interaction patterns (read receipts, stickers, mention guarantees) while keeping visual density closer to modern Western conventions.
6. **Honest defaults** — no fake previews, no unverifiable claims.
7. **Ask when needed** — permissions and prompts appear at the moment of relevance.

---

## 4. Architecture

### 4.1 Pages

The application is a **static site** built by Coralite. It has two pages:

| Page | File | Purpose |
|---|---|---|
| Auth Gate | `src/pages/index.html` | Login, registration, recovery. |
| Messenger | `src/pages/app.html` | The messenger shell. All subsequent view navigation is in-page. |

Both pages are static HTML served by Axum's `CLIENT_STATIC_DIR`. There is **no client-side routing between pages**. Navigation from the auth gate to the messenger is a full page load:

```js
// After successful login, session token is stored, then:
window.location.href = '/app.html'
```

Each page is a plain HTML document that uses components via their `<template id>` tags.

### 4.2 In-Page Routing

Within `app.html`, the application uses a **router plugin** to manage view state. The router synchronizes in-page view state with the browser's URL (query parameters) and History API.

**URL format:**

```
/app.html?view=chat&id=r_abc123
/app.html?view=chat&id=r_abc123&messageId=m_xyz789
/app.html?view=media
/app.html?view=settings
/app.html?view=profile&id=u_abc123
```

**State keys synced to the URL:**

| Key | Values |
|---|---|
| `currentAppView` | `chats`, `chat`, `media`, `documents`, `links`, `calls`, `settings`, `profile`, `join` |
| `activeSelectionId` | Room ID, user ID, message ID (context-dependent) |
| `activeSelectionType` | `room`, `user`, `message` |

**View mapping:**

| `view` param | Renders |
|---|---|
| `chats` | `view-chats` |
| `chat` | `view-chat` |
| `media` | `view-media` |
| `documents` | `view-documents` |
| `links` | `view-links` |
| `calls` | `view-calls` |
| `settings` | `view-settings` |
| `profile` | `view-profile` |
| `join` | `view-join-room` |

**Entry point:** The SPA entry component in `app.html` calls `router.init()` in its `client()`. The router plugin handles the `popstate` event, parses query parameters on load, and syncs state changes back to the URL.

**Deep links** (from push notifications, invite links) are query-param URLs:

```
https://app.example.com/app.html?view=chat&id=r_abc123&messageId=m_xyz789
```

### 4.3 Directory Structure

```
client/
├── coralite.config.js
├── capacitor.config.ts
├── src-tauri/
├── src/
│   ├── pages/
│   │   ├── index.html
│   │   └── app.html
│   ├── components/
│   │   ├── primitives/
│   │   ├── composed/
│   │   ├── containers/
│   │   ├── views/
│   │   └── shell/
│   ├── plugins/
│   ├── lib/
│   │   ├── mls/
│   │   ├── crypto/
│   │   ├── oprf/
│   │   ├── media/
│   │   ├── transcription/
│   │   ├── api/
│   │   ├── p2p/
│   │   ├── db/
│   │   ├── blobs/
│   │   ├── actions/
│   │   ├── events/
│   │   ├── validation/
│   │   └── utils/
│   ├── styles/
│   │   ├── tokens.css
│   │   └── utilities.css
│   └── assets/
├── migrations/
├── package.json
└── tailwind.config.js
```

**Component file naming:** Each component is a single `.html` file. Its tag name comes from the `<template id="...">` inside the file, not the filename. Components can be nested in folders. No prefix is applied.

### 4.4 Plugin Architecture

Plugins are **factory functions** returning `definePlugin` results. Values from `coralite.config.js` flow through the factory closure.

```js
// src/plugins/socket-plugin.js
import { definePlugin } from 'coralite'

export default (options) => definePlugin({
  name: 'socket',
  client: {
    config: options,
    context: (pluginContext) => (instanceContext) => ({
      useSocket: (handlers) => { /* ... */ }
    })
  }
})
```

**Config loading:**

```js
// coralite.config.js
import { defineConfig } from 'coralite-scripts'
import statePlugin from './src/plugins/state-plugin.js'
import routerPlugin from './src/plugins/router-plugin.js'
// ... other plugins

export default defineConfig({
  plugins: [
    statePlugin({ initialState: {} }),
    routerPlugin(),
    iconPlugin({ registry: /* tree-shaken icon map */ }),
    storagePlugin({ dbName: 'messenger', blobRoot: 'blobs' }),
    cryptoPlugin({ wasmPath: '/assets/corecrypto.wasm' }),
    oprfPlugin({ apiBase: process.env.API_BASE_URL }),
    socketPlugin({
      socketUrl: process.env.SOCKUDO_URL,
      apiBase: process.env.API_BASE_URL,
      appKey: process.env.SOCKUDO_APP_KEY
    }),
    pushPlugin({ vapidKey: process.env.VAPID_PUBLIC_KEY }),
    mediaPlugin({ presets: { high: {}, balanced: {}, small: {} } }),
    transcriptionPlugin({ modelBaseUrl: process.env.MODEL_BASE_URL }),
    themePlugin({ defaultTheme: 'auto' }),
    notificationPlugin({}),
    callPlugin({}),
    p2pPlugin({}),
    searchPlugin({}),
    i18nPlugin({ defaultLocale: 'en' })
  ],
  assets: [
    { src: 'src/assets/wasm/corecrypto.wasm', dest: 'assets/corecrypto.wasm' },
    { src: 'src/assets/stickers/', dest: 'assets/stickers/' },
    { src: 'src/assets/wallpapers/', dest: 'assets/wallpapers/' },
    { src: 'src/assets/sounds/', dest: 'assets/sounds/' }
  ]
})
```

**Two-phase resolver:**

- **Phase 1** — `(pluginContext) => ...`. Runs once per plugin per app load. Expensive setup. Can be `async`.
- **Phase 2** — `(instanceContext) => ({ ... })`. Runs per component instance. Cheap setup.

**Plugin name → context key.** A plugin named `'router'` becomes `client({ router })`.

**Cross-plugin access.** Plugins access each other via `instanceContext.<name>`.

**Global-singleton guard.** Expensive one-time work uses `pluginContext.__<name>_active__` for idempotency.

### 4.5 State Management

Global state is managed by the **state plugin** (`state-plugin`). Keyed pub/sub store.

**Consumption:**

```js
export default defineComponent({
  client({ globalStore }) {
    const { $state } = globalStore

    // Read
    const rooms = $state.rooms

    // Write
    $state.selectedRoomId = 'r_abc123'

    // Subscribe (auto-cleans via signal)
    $state.subscribe('rooms', (rooms) => { /* re-render */ })
  }
})
```

**State keys:**

| Key | Shape |
|---|---|
| `currentUser` | `User \| null` |
| `isAuthenticated` | `boolean` |
| `oprfToken` | `Uint8Array \| null` (in memory only) |
| `capabilities` | `Capabilities` |
| `rooms` | `Record<roomId, Room>` |
| `roomOrder` | `string[]` |
| `selectedRoomId` | `string \| null` |
| `messages:{roomId}` | `Message[]` |
| `members:{roomId}` | `Member[]` |
| `typing:{roomId}` | `TypingUser[]` |
| `outbox` | `OutboxEntry[]` |
| `unreadCounts` | `Record<roomId, number>` |
| `activeCall` | `Call \| null` |
| `callHistory` | `Call[]` |
| `settings` | `Record<string, unknown>` |
| `ui` | `{ activeRail, modal, toasts, offline }` |
| `pendingScrollToMessage` | `string \| null` |

**Source of truth:** SQLite. The store is an in-memory view.

### 4.6 Cross-Component Communication

| Relationship | Mechanism |
|---|---|
| Child → parent | `emit(name, detail)` — DOM-scoped CustomEvent, bubbles to ancestor |
| Sibling / cross-tree | `$state` key change |
| Global signal | `$state` key observed by the target component |

No global event bus.

### 4.7 Boot Sequence

```
1. index.html renders (auth gate)
   └─ No session → stays
   └─ Session → window.location.href = '/app.html'

2. app.html loads
3. Coralite shell renders
4. state-plugin initializes
5. router-plugin initializes (parses query params)
6. Read session token from secure storage
   └─ Invalid → redirect to index.html
7. Open SQLite, run migrations
8. Hydrate from SQLite:
   ├─ Rooms, members, room order
   ├─ Unread counts, drafts, preferences
   ├─ Cached display names and device names
   ├─ Last-viewed room's recent messages
   └─ Sync cursors (per-room + user-scoped)
9. User-scoped state sync:
   GET /users/me/sync?since_seq=<last_user_seq>
   Apply read_state, user_preferences, device_state
   Store new max_seq
10. CoreCrypto initialization
11. OPRF token derivation (requires username from secure storage)
12. Render UI with hydrated data
13. Fetch capabilities (GET /api/v1/capabilities)
14. Connect WebSocket (pusher-js)
15. Subscribe to private-user-{user_id} and all private-room-{room_id}
16. Delta sync per room (since cursor)
17. Background: key package replenishment, welcome polling,
    pending removes, thumbnail generation
```

**Failure modes:**

| Step | Failure | Behavior |
|---|---|---|
| 6 | Corrupt token | Clear storage, redirect |
| 7–8 | SQLite unavailable | Degradation notice; memory-only mode |
| 9 | Sync fetch fails | Retry; app works from local state |
| 10 | Keystore corrupted | Wipe, re-init; rooms marked `mls_status='error'` |
| 11 | OPRF fails | Retry; app works without display-name decryption until it succeeds |
| 14 | Socket fails | Retry with backoff; app remains usable from cache |

### 4.8 Sync & Offline

**Two cursors:**

- **Per-room** `(epoch, seq)` for message sync. Room events are best-effort WebSocket; REST delta sync is authoritative.
- **Per-user** `last_user_seq` for user-scoped state. User events are durable via `user_seq`; catch-up via `GET /users/me/sync`.

**Readable offline:** All cached messages, media, and room metadata remain readable.

**Outbox queue:** Messages composed while offline persist in SQLite. On reconnect, processed FIFO.

**Read state while offline:** Advances locally; debounced flush to `POST /users/me/read-state` (at most one POST per room per 30 seconds, plus immediate flush on room switch and app background).

**No global offline banner.** Feedback is inline on messages and via one-shot toasts.

### 4.9 Component Authoring Rules

Coralite components follow strict rules:

| Rule | Implication |
|---|---|
| **Flat template tokens only** | No expressions, function calls, dot notation, or conditionals in `<template>`. All logic moves to `getters`. |
| **No inline event handlers** | Bind events in `client()` via `refs('name').addEventListener(..., { signal })`. |
| **No array/object attributes** | Collections come via plugin context or slots. |
| **Getters destructure context** | `({ state, root, refs, slots, signal }) => ...` |
| **Style getters receive state directly** | `(state) => value`, synchronous only. |
| **`server()` is build-time only** | Stripped from browser bundles. |
| **`client()` has a serialization boundary** | No module-scope imports. Use `await import()` inside. |
| **`emit` instead of `dispatchEvent`** | Crosses custom element boundaries. |
| **Never mutate state in `observe()`** | Use event handlers or getters. |

---

## 5. Design System

### 5.1 Naming Conventions

| Term | Component |
|---|---|
| **Profile** | `ui-profile` |
| **Chat** | `view-chat` |
| **Room** | `room-*` |
| **Thread** | `message-thread` |

### 5.2 CSS Strategy

| Layer | Tool | Scope |
|---|---|---|
| Design tokens | CSS custom properties | Global |
| Global utilities | Tailwind | Entire app |
| Component styles | Coralite scoped `<style>` | Per-component |

### 5.3 Theming

**Global theme:** light / dark / auto. Token swap under `[data-theme="dark"]`.

**Accent color:** TBD. Placeholder blue (`#3B82F6`) used until brand decision.

**Per-room, per-user theming:** Room container overrides a small subset of tokens. Stored in `room_preferences` (local).

### 5.4 Icons

Solar icon set, tree-shaken. The `icon-plugin` resolves a **canonical name** and a **state** to a concrete icon. Canonical names are library-agnostic — the plugin's style map is where Solar-specific mapping lives.

### 5.5 Motion

Motion uses tokens from the design system. All motion respects `prefers-reduced-motion: reduce`.

---

## 6. Layout

### 6.1 Three-Panel Desktop (≥ 1024px)

```
┌──────┬──────────────┬──────────────────────────────┐
│ Rail │ List Panel   │ Detail Panel                 │
│ 64px │ 320–400px    │ flex                         │
└──────┴──────────────┴──────────────────────────────┘
```

Explorer pattern. Selecting an item in Panel 2 highlights it and renders detail in Panel 3.

**Rail items:** Chat, Media, Documents, Links, Calls, Settings.

### 6.2 Tablet (768–1023px)

Rail collapses to bottom nav. Panels 2 and 3 remain side by side.

### 6.3 Mobile (< 768px)

Single column. Bottom navigation with four items: **Chat, Media, Calls, Profile**.

The Media tab hosts a segmented control: **Photos / Docs / Links**.

### 6.4 Floating Glass Chrome

Chat header and composer are absolute overlays with `backdrop-filter: blur(20px) saturate(180%)`. The thread scrolls underneath. Dynamic offsets via `ResizeObserver`. Safe area insets included.

### 6.5 Safe Areas

Edge-to-edge rendering. Insets from `capacitor-edge-to-edge` (mobile), `tauri-plugin-safe-area-insets-css` (desktop), or `env(safe-area-inset-*)` (web). `100dvh` root height.

### 6.6 Navigation Patterns

| Affordance | Purpose |
|---|---|
| **Back arrow** (←) | Nested navigation (mobile only) |
| **Close** (✕) | Modal dismissal (desktop full-screen overlays) |
| **OK / Cancel** | Forced decisions |

**Desktop:** Explorer pattern. No back arrows. Media viewers open embedded in Panel 3 when browsing; full-screen overlays (with ✕) when opened from chat.

**Mobile:** Full-screen stack. Back arrows for all drill-downs.

---

## 7. Component Inventory

Component tags correspond to `<template id="...">` values. No array or object attributes.

### 7.1 Primitives

| Component | Responsibility | Key attributes |
|---|---|---|
| `ui-profile` | User/group profile image | `size`, `shape`, `src`, `state`, `fallback` |
| `ui-profile-group` | Composite of 2–4 profiles | `max`, `size` |
| `ui-badge` | Unread count or dot | `count`, `variant` |
| `ui-button` | Pill button | `variant`, `size`, `disabled`, `loading` |
| `ui-input` | Text field | `type`, `label`, `error`, `disabled` |
| `ui-textarea` | Auto-growing text area | `rows`, `maxlength` |
| `ui-switch` | Toggle | `checked`, `disabled` |
| `ui-checkbox` | Checkbox | `checked`, `indeterminate` |
| `ui-radio` | Radio | `checked`, `name`, `value` |
| `ui-spinner` | Loading indicator | `size` |
| `ui-skeleton` | Loading placeholder | `variant` |
| `ui-divider` | Rule | `orientation` |
| `ui-chip` | Tag / filter / reaction | `selected`, `removable` |
| `ui-timestamp` | Time display | `datetime`, `format` |
| `ui-icon` | Solar icon wrapper | `name`, `state`, `size`, `color` |
| `ui-sheet` | Bottom sheet / dialog | `variant`, `open` |
| `ui-toast` | Transient message | `variant`, `duration` |
| `ui-tooltip` | Hover/focus hint | `placement` |
| `ui-disclosure` | Expandable section | `label`, `open` |
| `ui-context-menu` | Trigger-adaptive menu | `trigger` |

### 7.2 Composed

| Component | Responsibility |
|---|---|
| `chat-bubble` | Message container |
| `chat-bubble-group` | Consecutive messages |
| `message-text` | Markdown body |
| `message-attachment` | Image/file/audio preview |
| `message-reaction` | Reaction with count |
| `message-meta` | Time + status |
| `message-waveform` | Voice player |
| `message-transcript` | Transcript display |
| `message-tombstone` | Deleted placeholder |
| `typing-dots` | Typing indicator |
| `list-row` | Generic list row |
| `conversation-row` | Conversation list item |
| `media-thumbnail` | Grid thumbnail |
| `link-card` | Link preview |
| `document-row` | File list item |
| `file-icon` | File type icon |
| `nav-item` | Rail or bottom-nav item |
| `form-field` | Label + control + hint |
| `search-field` | Search input |
| `section-header` | Group label |
| `empty-state` | Empty state |
| `safety-number` | Safety number display |
| `call-participant` | Call participant tile |

### 7.3 Containers

| Component | Responsibility |
|---|---|
| `conversation-list` | Virtualized list (reads `$state.rooms`) |
| `message-thread` | Virtualized message list |
| `message-composer` | Input + attach + send + emoji + voice |
| `media-grid` | 3-column flush grid |
| `document-list` | Virtualized list group |
| `link-list` | Virtualized list group |
| `settings-list` | Settings rows |
| `auth-form` | Auth gate form |
| `media-viewer` | Full-screen slideshow |
| `link-preview` | Link preview |
| `document-preview` | File preview |
| `call-participant-grid` | Call layout |
| `message-reply-preview` | Reply bar |
| `sticker-picker` | Sticker sheet |

### 7.4 Shell

| Component | Responsibility |
|---|---|
| `messenger-shell` | Root layout |
| `modal-host` | Overlay renderer |
| `toast-host` | Toast queue |
| `call-overlay-host` | Call overlays |
| `notification-banner-host` | In-app banners |

### 7.5 Views

| View | Contains |
|---|---|
| `view-chats` | `conversation-list` |
| `view-chat` | `message-thread` + `message-composer` |
| `view-media` | Filter bar + `media-grid` |
| `view-documents` | Filter bar + `document-list` |
| `view-links` | Filter bar + `link-list` |
| `view-calls` | `call-history-list` |
| `view-settings` | Section list + sub-views |
| `view-profile` | Profile detail |
| `view-room-settings` | Drawer (desktop) / push (mobile) |
| `view-join-room` | Invite confirmation |
| `view-admin` | Administration |

---

## 8. Crypto & E2EE

### 8.1 Three Encryption Layers

| Layer | Purpose | Implementation |
|---|---|---|
| **MLS** | Message encryption, group state, key packages, welcomes, commits | Wire CoreCrypto 10.5.2 (WASM, bundled) |
| **C2SP chunked AES-256-GCM** | Attachment blob encryption, range-based decryption | Web Crypto API |
| **OPRF token** | Username lookup, display-name and device-name encryption | Client-side OPRF blinding |

### 8.2 OPRF-Based Lookup

**Purpose:** The server must not hold plaintext usernames or display names.

**Construction:**

```
Given:
  username       The plaintext username
  k              The server's OPRF key (derived from root_secret)
  H              Ristretto255 hash-to-curve

Client:
  1. h         = H(username)
  2. r         = random blinding factor
  3. blinded   = h * r
  4. sends { blinded } to server

Server:
  5. blinded_oprf = blinded ^ k
  6. returns { blinded_oprf } to client

Client:
  7. token = blinded_oprf / r^k = h^k
```

**Token usage:**

```
display_name_key = HKDF(token, info="display-name-encryption-v1", length=32)
device_name_key  = HKDF(token, info="device-name-encryption-v1",  length=32)
```

Domain separation ensures a compromise of one derived key doesn't compromise the other.

**Client caches the token in memory for the session.** Never stored at rest. Re-derived on login (requires username from secure storage; biometric login retrieves it automatically).

**Token stability:** `k` is immutable for the account lifetime. The token is stable across devices and sessions.

### 8.3 MLS Client Behavior

**CoreCrypto initialization:**

```
1. Load device_secret from platform secure storage
   (generate 32 random bytes on first launch if absent)
2. entropy_seed = HKDF-Expand(device_secret, info="mls-entropy-v1", length=32)
3. Open the encrypted keystore
4. CoreCrypto.init(client_id, keystore, entropy_seed)
5. CoreCrypto.mlsInit()
```

**Key package lifecycle:**

- Target: 20 unconsumed key packages per device.
- Low watermark: 5.
- Replenish on login and on app launch.
- Rotate every 30 days.

**Group lifecycle:**

- **Create:** `mlsCreateConversation(room_id)`, batch-add members in one commit.
- **Join:** Poll for welcome (`GET /welcomes`), `mlsProcessWelcomeMessage`, consume, fetch messages from welcome epoch.
- **Leave:** Server queues `pending_mls_remove`; another online member generates the Remove commit; leaving client deletes local group state.

**Message encryption/decryption:** Envelope wrapped in MLS message, POSTed to `/rooms/:id/messages`. Decryption uses the message's epoch; older epochs use previous-epoch keys; newer epochs queue and sync.

**Epoch transitions:** Composer enters "Securing…" state. Outgoing messages queued and flushed once the new epoch settles.

### 8.4 C2SP Attachment Encryption

Per Server Spec §6.5. Summary:

```
1. input_key = random 32 bytes
2. salt = random 24 bytes
3. context = "attachment" || 0x00 || room_id
4. info = "c2sp.org/chunked-encryption@v1+" || "AEAD_AES_256_GCM" || 0x00 || salt || context
5. key_material = HKDF-Expand(input_key, info, 76)
6. file_key = key_material[0..32]
   base_nonce = key_material[32..44]
   commitment = key_material[44..76]
7. header = salt || commitment
8. For chunk i: nonce_i = base_nonce XOR i
9. Ciphertext = header || encrypted_chunks
```

**Padding:** Plaintext padded to the nearest bucket from `ATTACHMENT_BUCKET_SIZES` **before** encryption. The padded plaintext is chunked as a single C2SP message producing one short final chunk.

**Range translation:**

```
start_chunk = p_start / 16384
end_chunk   = p_end   / 16384
encrypted_start = 56 + start_chunk * (16384 + 16)
encrypted_end   = 56 + (end_chunk + 1) * (16384 + 16) - 1
```

### 8.5 Key Transparency

The client consumes key transparency proofs from the server:

1. Fetch identity keys with `tree_head`, `auditor_signatures`, and `inclusion_proof`.
2. Verify the server signature, each auditor signature, and the inclusion proof locally.
3. Mark the contact as **transparency-verified** on success.
4. Display a green checkmark; no user action required.

**Failure cases:**

| Case | Display |
|---|---|
| Inclusion proof fails | Warning; not verified |
| Auditor signatures missing | "Verified by server only" |
| Auditor signatures invalid | Warning; unverified |
| Network unavailable | "Verification pending" |

**Manual safety numbers** remain available under an "Advanced" section.

---

## 9. Media Pipeline

### 9.1 Format Matrix

| Type | Primary | Fallback A | Fallback B |
|---|---|---|---|
| Video | WebM / VP9 + Opus | WebM / VP8 + Opus | MP4 / H.264 + AAC |
| Audio | Opus (Ogg or WebM) | AAC (M4A) | — |
| Image | AVIF | WebP | JPEG |

**Quality presets (default Balanced):**

| Preset | Image | Video | Audio |
|---|---|---|---|
| High | AVIF q80, max 2560px | VP9, 1080p, ~4 Mbps | Opus 96 kbps |
| Balanced | AVIF q70, max 1920px | VP9, 720p, ~2 Mbps | Opus 64 kbps |
| Small | AVIF q60, max 1280px | VP9, 480p, ~1 Mbps | Opus 48 kbps |

### 9.2 Transcoding

Mediabunny handles demux, decode, encode, resize, rotate, crop, thumbnail extraction. GPU path uses WebCodecs. CPU fallback is `ffmpeg.wasm`.

**Worker pool:** Concurrency cap of 2 (1 on low-end). Jobs above 500 MB processed alone. Each job exposes an `AbortController`.

**Failed transcode:** Passthrough — the file is uploaded unprocessed.

### 9.3 Thumbnail and Poster Generation

Client generates a **128×128 WebP thumbnail** alongside the full attachment. Encrypted separately (same key/salt/context), uploaded as a companion blob. Manifest carries `thumbnail_file_id`.

For video, a **poster frame** is also extracted.

### 9.4 Padding

Padding operates on the **plaintext** before encryption:

```
Given target bucket T:
  M = T - 72
  q = floor(M / 16400)
  r = M - 16400 * q
  if r < 16384: L_padded = 16384 * q + r
  else: skip to next bucket
```

Append zero bytes to reach `L_padded`. Chunk. Encrypt.

### 9.5 Metadata Stripping

All processed media is stripped of metadata (EXIF, GPS, XMP, IPTC, ICC, ID3). Documents are not modified.

### 9.6 Fast-Start MP4

MP4 files validated for `moov` at the start. Files with `moov` after `mdat` are re-muxed with `mp4box.js`.

### 9.7 Progress and Cancel

| State | UI |
|---|---|
| `queued` | Clock icon |
| `transcoding` | Progress ring, cancel |
| `encrypting` | Brief |
| `uploading` | Progress bar, cancel |
| `p2p_pending` | Recipient selection |
| `p2p_transferring` | Per-recipient sub-rows |
| `failed` | Retry or remove |

Long uploads surface a Live Activity on iOS and an ongoing notification on Android.

---

## 10. Attachments

### 10.1 Upload Flow

```
1. User attaches file(s)
2. Probe with Mediabunny
3. Transcode per preset
4. Extract poster (video)
5. Generate thumbnail
6. Strip metadata
7. Pad plaintext to bucket
8. Chunk per C2SP
9. Derive key, nonce, commitment
10. Encrypt chunks
11. Compute SHA-256 (file_id)
12. Upload ciphertext
13. Send MLS message with manifest
```

### 10.2 Download Flow

- **Files ≤ 1 MB:** Single GET.
- **Files > 1 MB:** Lazy chunk fetch.
- **Media:** Always lazy.

Presigned URLs used when `capabilities.storage_presign_supported`. Not cached; on expiry, fresh URL + resume.

### 10.3 P2P Fallback

When a file exceeds `effective_max_file_size_bytes`:

1. Member picker modal.
2. WebRTC data channels (one per recipient).
3. Transfer with ephemeral key per recipient.
4. Chunk-level ack + resume.
5. Cancel discards on both sides.

Online-only.

### 10.4 Deletion

| Operation | Scope | Permission |
|---|---|---|
| Delete for me | Local | Anyone |
| Unsend | All members | Sender, ≤24h |
| Remove from room | All members | Owner / moderator |

### 10.5 Attachment Icon Mapping

Six canonical categories, each with a color:

| Category | Color | Extensions |
|---|---|---|
| `document` | Red | pdf, doc, docx, odt, rtf, pages |
| `spreadsheet` | Green | xls, xlsx, csv, ods, numbers |
| `presentation` | Orange | ppt, pptx, odp, key |
| `text` | Blue | txt, md, json, js, ts, py, rs, go, html, css, log |
| `archive` | Amber | zip, rar, 7z, gz, tar, bz2 |
| `design` | Purple | fig, sketch, xd, psd, ai |
| `generic` | Grey | everything else |

Resolution: MIME → extension → generic.

---

## 11. Timeline (Message Thread)

### 11.1 Bubble Layout

| Aspect | Own | Others' |
|---|---|---|
| Alignment | Right | Left |
| Avatar | Omitted | `ui-profile`, bottom-aligned |
| Sender name | N/A | Above bubble, first of group |
| Bubble color | `var(--bubble-outgoing)` | `var(--bubble-incoming)` |
| Timestamp | Inside, bottom-right | Inside, bottom-left |
| Read receipt | "Read" / "Read N" | — |

Max width: `min(75%, 600px)`. Corner radius: `18px` default; `4px` on tail and inner group corners.

### 11.2 Sending States

| State | Icon |
|---|---|
| Queued | Clock |
| Sending | Diagonal arrow (↖) |
| Sent | Timestamp only |
| Read | "Read" / "Read N" |
| Failed | Circular arrow (↻) |

### 11.3 Message Grouping

Consecutive messages from same sender within 5 minutes grouped.

### 11.4 Date Separators

Centered pills on day change. Sticky date header.

### 11.5 New Messages Divider

Line at last-read position.

### 11.6 Scroll Behavior

- Open → scroll to bottom
- New incoming while pinned → follow
- New incoming while scrolled up → increment unseen count
- Own send → always scroll
- Scroll-to-bottom button (floating, shows unseen count)
- Infinite scroll trigger at ~200px from top
- Prepend anchoring (TanStack Virtual `anchorTo: 'end'`)
- Stable keys: message IDs

### 11.7 Message Context Menu

Long-press / right-click / hover bar.

**Universal:** Copy, Reply, React, Select, Delete for me.
**Sender-only:** Unsend (≤24h).
**Conditional:** Edit, Show original, Delete (Discord).

Browser context menu suppressed on `.chat-bubble` only.

### 11.8 Message Editing

15-minute window. Signed edit chain. "Edited" label. "Show original" sheet.

### 11.9 Voice Messages

Press-hold or tap-to-lock. Wavesurfer display. Peaks float array in payload. Max 30 min.

### 11.10 Transcription

Local, Web Worker, WebGPU-first. Moonshine Tiny (default), Moonshine Small, Whisper Tiny.

### 11.11 Stickers and Emoji

Bundled / shared / custom. Pack manifest JSON. Lottie for animated. Entrance transitions (`none`, `pop`, `drop`, `slide`, `crash`, `fade`). Combination tool. Suggestions.

### 11.12 Link Previews

Client-generated. Debounced (500ms), 3s timeout. Opt-in server proxy for CORS-blocked.

### 11.13 Reply / Quote

Swipe (mobile) or hover bar (desktop). Preview bar above composer.

### 11.14 @Mentions

Autocomplete. `mentions` array in payload with `user_id`, `offset`, `length`. Mute override (except "Nothing"). Client filters after decryption.

### 11.15 Markdown Formatting

Bold, italic, strikethrough, inline code, links, lists, block quotes.

### 11.16 Message Tombstone

Muted pill replacing the bubble.

### 11.17 In-Thread Search

Magnifying glass in header. Fuse.js per-room. Highlight `#FFE066`.

### 11.18 Global Search

Chat list header. Searches room names + message content. Results grouped by room.

### 11.19 Disappearing Messages

Room-level timer in encrypted metadata. Options: Off, 24h, 7d, 90d. Owner-only. Applies to new messages. Local cleanup.

---

## 12. Composer

### 12.1 Controls

| Element | Action |
|---|---|
| `[＋]` | Attachment picker |
| `[😊]` | Emoji / sticker picker |
| Text input | Auto-growing textarea |
| `[🎤/➤]` | Dynamic mic/send |

### 12.2 Reply Preview

Bar above composer, part of glass overlay.

### 12.3 Attachment Preview

Between pick and send. Thumbnail, filename, size, quality override, caption, remove, reorder.

### 12.4 Enter Key Behavior

| Platform | Enter | Shift+Enter |
|---|---|---|
| Desktop | Send | Newline |
| Mobile | Newline | — |
| Tablet + physical keyboard | Send | Newline |

### 12.5 Focus Behavior

After send, focus stays in input. Virtual keyboard stays open on mobile.

### 12.6 Multi-Attachment Layout

| Count | Layout |
|---|---|
| 1 | Full-width |
| 2 | Two columns |
| 3 | Left full-height, right stacked |
| 4 | 2×2 |
| 5+ | 2×2 with `+N` |

Files as stacked cards. Caption last.

---

## 13. Rooms

### 13.1 Room Creation

1. Add members (username lookup or invite link).
2. For groups (3+ members), collapsed "Customize room" disclosure.
3. Tap Create room.

For 1:1, no disclosure.

### 13.2 Member Management

**Add member via username lookup:**

Full cycle documented in §13.4.

**Member list display:**

Member rows show profile image and display name. Display names come from a resolution chain:

1. Cached display name (from previous decryption or message payload)
2. Message payload (sender's display name is inside MLS-encrypted messages)
3. `encrypted_display` from member list response (only if token is cached for that user)
4. Fallback: `user_id` or "Member" placeholder

**Practical consequence:** Member lists show placeholders until a message is received from that member. Once the first message arrives, the display name resolves. The client spec acknowledges this UX limitation.

### 13.3 Invite Links and QR

Room-specific tokens. Default 7-day expiration. QR via `qr-code-styling`.

### 13.4 Username Lookup

**Full UX cycle:**

**Phase 1 — User taps "Add member"**

Sheet opens with a single Username field.

**Phase 2 — Lookup (invisible)**

| Step | Where | What happens |
|---|---|---|
| 1 | Client | Blinds the username with a random factor |
| 2 | Client → Server | `POST /oprf/blind` |
| 3 | Server | OPRF evaluation with `k` |
| 4 | Client | Unblinds → token |
| 5 | Client → Server | `POST /users/lookup { username_token }` |
| 6 | Server | Matches token, returns `{ user_id, encrypted_display }` |
| 7 | Client | Derives `display_name_key` from token |
| 8 | Client | Decrypts display name |

Server saw two opaque values. Never saw `alex_92` or "Alex."

**Rate limiting:** OPRF endpoint rate-limited. Client surfaces: *"Too many attempts. Please wait a moment and try again."*

**Phase 3 — Confirmation card**

On success: profile image + display name + username + "Add to room" button.

On failure: hint card with case-sensitivity, no-spaces, full-username reminders. Fallback: "Invite via link instead."

**Phase 4 — Adding to room**

| Step | What happens |
|---|---|
| 1 | `POST /rooms/:id/members { user_id }` |
| 2 | Server adds to `room_members`, inserts `pending_mls_adds` for each of B's devices |
| 3 | Server publishes `mls.add_pending` on room channel |
| 4 | Any online member generates Add commit + Welcome |
| 5 | Server publishes `mls.welcome_ready` to B |
| 6 | Server publishes `room.member_added` on room channel |

System message: **"You added Alex."**

**Phase 5 — Recipient side**

Online: receives `mls.welcome_ready`, fetches welcome, processes, fetches messages from welcome epoch, room appears.

Offline: push notification. On open, processes pending welcome.

**Phase 6 — Post-add**

Display name cached locally. Future messages carry sender's display name inside encrypted payload.

**Failure modes:**

| Failure | Recovery |
|---|---|
| Username doesn't exist | Retry or invite link |
| Rate-limited | Wait and retry |
| Already a member | Toast |
| Room full | Toast |
| MLS add fails | Retry; `client-mls-request` after 5min |

### 13.5 Room Settings

**Desktop:** Off-canvas right drawer in Panel 3.
**Tablet / Mobile:** Full-screen push.

**Sections:**

| Section | Permission |
|---|---|
| Profile | Owner |
| Members | All |
| Invites | All |
| Notifications | Per-user |
| Chat Theme | Per-user |
| Media & Storage | Per-user |
| Retention & Limits | Owner |
| Moderation | Owner (Discord) |
| Danger Zone | Owner |
| Leave Room | All |

### 13.6 Room Metadata JSON

```json
{
  "schema_version": 1,
  "name": "Design Team",
  "avatar_file_id": "<sha256>",
  "description": null,
  "disappearing_timer": null,
  "created_by": "u_abc123"
}
```

### 13.7 Room Avatar Upload

Source sheet → circular crop → preview → upload.

Constraints: 20 MB max, 128×128 min, JPEG/PNG/WebP/AVIF/HEIC. Output 512×512 AVIF q80.

### 13.8 Room List Ordering

**Default:** Fixed by join date. New rooms append.

**Manual reordering:** Drag-and-drop via menu activation. `Alt+↑`/`Alt+↓` keyboard alternative. Reset to default.

**Sync:** `PATCH /users/me/room-order { room_ids }`. Other devices receive `room_order.sync` on `private-user-{user_id}`.

**Filters:** Unread only, Mentions only, Muted rooms.

### 13.9 Conversation Row Content

**Indicators (prioritized):** Red `@` pill → green pill with count → green dot → grey pill → bell-slash → pencil.

**Message preview rules:** Text, image, video, voice, document, sticker, tombstone (falls back), system message (falls back).

**Typing indicator** replaces the preview.

### 13.10 Leaving and Deletion

**Leave room:** Confirmation; warns if last member.
**Delete room (owner):** Type-to-confirm. Other members receive `room.deleted`, room removed locally, system message: "This room was deleted by [owner]."

### 13.11 Member Roles

| Action | Owner | Moderator (Discord) | Member |
|---|---|---|---|
| Send messages | ✅ | ✅ | ✅ |
| Upload attachments | ✅ | ✅ | ✅ |
| Add members | ✅ | ✅ | ✅ |
| Create invites | ✅ | ✅ | ✅ |
| Edit room metadata | ✅ | ❌ | ❌ |
| Change retention | ✅ | ❌ | ❌ |
| Kick members | ✅ | ✅ | ❌ |
| Delete any message | ✅ | ✅ | ❌ |
| Delete own message | ✅ | ✅ | ✅ |
| Transfer ownership | ✅ | ❌ | ❌ |
| Delete room | ✅ | ❌ | ❌ |
| Change disappearing timer | ✅ | ❌ | ❌ |

---

## 14. Curation Views

### 14.1 Media Grid

3-column flush grid, grouped by chat.

### 14.2 Documents and Links

List groups using `list-row`.

### 14.3 Filter and Search

Filter chips hidden by default. Search via Fuse.js.

### 14.4 Jump to Chat

Opens `/app.html?view=chat&id=:roomId&messageId=:msgId`.

### 14.5 Media Viewer

**Scoping:**

| Opened from | Source list |
|---|---|
| Chat thread | All media in the room |
| Curation view | Filtered list |

**Scrolling:** CSS Scroll Snap.

**Morph:** View Transitions API.

**Navigation:** Arrows, swipe, keyboard.

**Zoom and pan:** Pinch, double-tap, scroll wheel.

**Swipe-to-dismiss:** Vertical swipe at 1×.

**Action bar (top):** Save, Share, Copy, Jump to message, Delete, Info.

**Thumbnail strip:** Bottom, virtualized (±20 items), two-way sync.

**Video:** `IntersectionObserver` pause-on-scroll-out.

**Prefetching:** Active slide ±2 for decryption.

### 14.6 Document Preview

PDF inline; image/video/audio as media; text/code rendered; Office/archives file card.

### 14.7 Link Preview Card

Single tappable block.

---

## 15. Calls (V1 UI)

### 15.1 Model

Room-scoped. 1:1 = 2-member room.

### 15.2 Pre-Call Preview

Default: voice, camera off, mic on.

### 15.3 Incoming Call

Full-screen or banner. Actions: Decline, Accept, Message, More.

### 15.4 In-Call Screen

Voice 1:1 → profile-centric.
Voice group → grid (9) or list.
Video 1:1 → full remote, self PiP.
Video group → grid (6) or focus view.

### 15.5 Minimized Call

PiP floating window.

### 15.6 Call History

Calls tab. Rows show profile, room, type, direction, duration, timestamp.

---

## 16. Auth

### 16.1 Auth Gate

Single page (`index.html`). Three views: Login, Register, Recovery.

### 16.2 Login View

- Username field
- Password field
- "Log in with Face ID" (if enrolled)
- "Register with invite code →"
- "Lost access? Use recovery →"

**Login flow (client):**

1. Blind the typed username → `POST /oprf/blind` → unblind → token.
2. `POST /auth/login/start { username_token, opaque_client_auth_state }`.
3. OPAQUE AKE locally.
4. `POST /auth/login/finish { username_token, ke3 }`.
5. Store session token.
6. Cache OPRF token in memory.
7. `window.location.href = '/app.html'`.

### 16.3 Register View

1. Invite code (8-char Crockford Base32).
2. ALTCHA widget.
3. Username.
4. Display name.
5. Password.
6. "Create account".

**Registration flow (client):**

1. Blind the username → `POST /oprf/blind` → unblind → token.
2. Derive `display_name_key = HKDF(token, "display-name-encryption-v1", 32)`.
3. Encrypt display name with `display_name_key`.
4. OPAQUE registration locally.
5. `POST /auth/register/finish { username_token, encrypted_display, opaque_record, identity_pubkey, altcha }`.
6. Server returns:
   - `session_token`
   - `user_id`
   - **Recovery codes (plaintext, shown once)**
7. Display recovery codes. **Download is mandatory** — Next disabled until downloaded.
8. Confirmation: type the 3rd code back (matches Signal pattern).
9. Client caches the OPRF token.
10. `window.location.href = '/app.html'`.

**Server-generated recovery codes:** The server generates the plaintext list, returns it once, stores only Argon2id hashes with per-code salt. The client never generates codes.

### 16.4 Recovery View

**Separate flow, not a login-mode variant.**

```
┌─────────────────────────────────────────┐
│  ←  Recover access                      │
│                                         │
│  Username                               │
│  [ alex_92                    ]         │
│                                         │
│  Recovery code                          │
│  [ XXXX-XXXX-XXXX-XXXX        ]         │
│                                         │
│  New password                           │
│  [ **********                 ]         │
│                                         │
│  [      Restore access      ]           │
└─────────────────────────────────────────┘
```

**Recovery flow (client):**

1. Blind username → OPRF → token.
2. `POST /auth/recover/start { recovery_code, username_token }`.
3. Server looks up user by token, verifies code (Argon2id, constant-time).
4. Server returns OPAQUE registration challenge.
5. Client runs OPAQUE registration with new password.
6. `POST /auth/recover/finish { username_token, recovery_session, opaque_record, encrypted_display, identity_pubkey }`.
7. Server replaces OPAQUE record, consumes recovery code, **revokes all existing sessions**, issues new session.
8. Client stores new session, redirects to `/app.html`.

**Server behavior:**

- Recovery codes hashed with Argon2id + per-code salt, stored as `{ salt, hash }`.
- Constant-time comparison.
- Recovery code consumed (single-use or decrement) on success.
- Session revocation is mandatory.

**Other devices:** Receive `session.revoked` → clear session and local data → redirect to `index.html`.

**Recovery code display screen (initial registration):**

- Codes are shown once, must be downloaded.
- Confirmation step requires typing back one of the codes.
- Explicit warning: "These codes cannot be recovered if lost."
- Regeneration is only available from an authenticated session (Settings → Account → Recovery codes).

### 16.5 Biometric Unlock

- Root of trust remains OPAQUE + password.
- Biometric gates access to the stored OPAQUE `RegistrationRecord`.
- Enrollment offered once after a successful password login.
- Unenrollment in Settings → Account → Security.
- Auto-invalidated on password change.

**Screen lock timeout:** Settings → Privacy → Screen lock → Immediately / 1 min / 5 min / 30 min. Default: 1 minute.

**Biometric login flow:** Biometric unlocks the OPAQUE record → username retrieved from local storage → OPRF blinding → token → login proceeds. User types nothing.

### 16.6 Device Linking

**Adding a new device when an existing session is active:**

1. Existing device generates a device linking code (QR or short code).
2. New device enters or scans the code.
3. New device performs OPAQUE login with password.
4. New device uploads its MLS key package.
5. New device fetches user-scoped sync state: `GET /users/me/sync?since_seq=0`.
6. Device names and other user-scoped state populate.

**Device names:**

- Server stores `encrypted_device_name` as opaque ciphertext in user-scoped sync state.
- Client encrypts with `device_name_key = HKDF(token, "device-name-encryption-v1", 32)`.
- Cross-device: `device.name_updated` event on `private-user-{user_id}`.

### 16.7 Passkeys (V2 Seam)

Not shipped in V1. Data-driven action list.

### 16.8 Session Expiry

Auth token expired → redirect to `index.html`.
OPAQUE session expired → inline error.

### 16.9 Multi-Device Session Semantics

| Event | Effect |
|---|---|
| Log out on A | Only A |
| Revoke B from A | B via `session.revoked` |
| Delete account | All devices |
| Change password | All other devices |
| Complete recovery | All other devices |

### 16.10 Multi-Device Read State Sync

- Client publishes read state via `POST /users/me/read-state { room_id, last_read_message_id }`.
- Server assigns `user_seq`, publishes `read.sync` on `private-user-{user_id}`.
- Client stores `last_user_seq` in `_meta`.
- On boot, `GET /users/me/sync?since_seq=<cursor>` fetches missed state.
- **Debounced:** at most one POST per room per 30 seconds, plus immediate flush on room switch and app background.

---

## 17. Settings

Single view (`view-settings`). Rendered in Panel 3 (desktop) or full-screen push (mobile). Search bar at the top.

### 17.1 Account

| Row | Control |
|---|---|
| Profile | Navigate → display name (encrypted), profile image, username (read-only) |
| Password | Navigate → change password |
| Recovery codes | Navigate → status, regenerate (requires authenticated session) |
| Devices | Navigate → list, revoke |
| Biometric unlock | Toggle |
| Passkeys | Navigate → V2 placeholder |

**Display name update flow:**

1. User enters new display name.
2. Client re-encrypts with the cached `display_name_key` and a fresh nonce.
3. `PATCH /users/me { encrypted_display }`.

**Device names:** Stored locally and synced across devices via user-scoped state. Each device's name is encrypted with `device_name_key` before sync.

### 17.2 Privacy

| Row | Default |
|---|---|
| Read receipts | On |
| Typing indicators | On |
| Message preview in notifications | On |
| Screen lock | Off |
| Screen lock timeout | 1 minute |
| Blocked users | Navigate → list |
| Emergency wipe | Configure trigger |

**Removed:** "Last seen" row. Presence is out of scope.

### 17.3 Notifications

Push notifications, Sound, Vibration, Per-room overrides, Muted rooms.

### 17.4 Chats

Font size, Default chat theme, Per-chat themes, Enter to send, Voice message auto-play, Voice playback speed, Draft messages.

### 17.5 Media & Storage

Storage usage, Clear cached media, Auto-download media/docs, Transcode quality, Strip metadata, Max upload size.

### 17.6 Transcription

Auto-transcribe, Show transcript, Default model, Downloaded models, Transcription language.

### 17.7 Calls

Default call mode, Call ringtone, Video call quality, Show calls tab.

### 17.8 Appearance

Theme, Accent color, Wallpaper, Reduced motion, Sticker animations, Language.

### 17.9 About

Version, Terms, Privacy, Licenses, Log out, Log out and clear data, Delete account.

### 17.10 Administration

Rendered only if `capabilities.admin`.

### 17.11 Sign-Out and Account Deletion

**Log out:** Session cleared, local data preserved (encrypted at rest).

**Log out and clear data:** Everything cleared.

**Account deletion:** Warning screen → type username to confirm → server anonymizes → local data wiped → Auth Gate.

**Recovery triggers session revocation on all other devices.** The affected devices handle it via the existing `session.revoked` path.

---

## 18. Notifications

### 18.1 In-App Banner

Top of detail panel (desktop) / top of screen below status bar (mobile). One at a time. Auto-dismiss after 5s. Mute exception for mentions.

### 18.2 Push Notifications

**Case matrix:**

| App state | WebSocket | Mechanism |
|---|---|---|
| Foreground, same room | Connected | No notification |
| Foreground, different room | Connected | In-app banner |
| Backgrounded (< 30s) | Connected | No push yet |
| Backgrounded (> 30s) | Disconnected | Push notification |
| Lock screen | Disconnected | Push notification |
| App killed | Disconnected | Push notification |

**Permission flow:** Pre-prompt (once, after 24h + activity) → OS prompt. iOS provisional notifications adopted. No re-prompt after denial.

**Push actions:**

| Action | iOS | Android | Web |
|---|---|---|---|
| Reply | ✅ inline | ✅ inline | ❌ |
| Mark as read | ✅ | ✅ | ✅ |
| Tap | ✅ deep link | ✅ deep link | ✅ deep link |

**Reply architecture:** NSE decrypts, app process encrypts and sends.

**Screen lock interaction:** Reply prompts biometric if Screen lock is on.

**Notification categories:**

| Category | Actions |
|---|---|
| `MESSAGE` | Reply, Mark as read |
| `MESSAGE_MUTED` | Reply |
| `MENTION` | Reply, Mark as read |
| `CALL` | Accept, Decline |
| `MEMBER_CHANGE` | (none) |

---

## 19. Cross-Platform

### 19.1 Capacitor

Push via `@capacitor/push-notifications`. Secure storage via keychain. Status bar via `capacitor-edge-to-edge`. Biometric via `@capacitor/biometrics`. QR via camera plugin.

### 19.2 Tauri

Push via `tauri-plugin-notification`. Secure storage via `tauri-plugin-stronghold`. Safe areas via `tauri-plugin-safe-area-insets-css`. Biometric via `tauri-plugin-biometric`. Deep link via `tauri-plugin-deep-link`.

### 19.3 Shared CSS

Same CSS variables across platforms. No platform branching.

### 19.4 Privacy Screen

App switcher snapshot blurred.

| Platform | Mechanism |
|---|---|
| iOS | Blur overlay in `applicationWillResignActive` |
| Android 13+ | `setRecentsScreenshotEnabled(false)` |
| Android 12 and below | Not blocked |

User screenshots remain allowed.

### 19.5 Deep Linking

**URL format:** `https://app.example.com/app.html?view=chat&id=r_abc123&messageId=m_xyz789`

**Pending slot:** In-memory only. Invite tokens use `sessionStorage` for auth-flow survival.

**Processing order:** Boot → auth → sync start → route → clear.

**Platform config:** Universal links (iOS, Android), custom scheme (all).

---

## 20. Empty States

Three variants: **empty**, **no results**, **error**.

| Surface | Variant | Copy |
|---|---|---|
| Room list (new) | empty | "No chats yet. Create a room or join with an invite link." |
| Room list (search) | no-results | "No results for 'query'" |
| Room list (error) | error | "Couldn't load your chats" |
| Chat thread (new) | empty | "This is the beginning of the room." |
| Chat thread (MLS pending) | empty | "Setting up encryption…" |
| Media | empty | "No media yet." |
| Documents | empty | "No documents yet." |
| Links | empty | "No links yet." |
| Curation (filtered) | no-results | "No [media/docs/links] match your filters." |
| Calls | empty | "No calls yet." |
| Blocked users | empty | "No blocked users" |
| Downloaded models | empty | "No models downloaded" |
| Custom stickers | empty | "No custom stickers yet" |
| In-thread search | empty | "Nothing to search yet" |

**Rules:** Shared-rooms section hidden when empty. One primary action per state.

---

## 21. Error Handling

### 21.1 Network Layer

WebSocket connection state authoritative. No global banner. Retry strategy:

| Operation | Strategy |
|---|---|
| WebSocket reconnect | 1s, 2s, 4s, 8s, 16s, 30s cap |
| Failed message send | 3 attempts: 2s, 5s, 15s |
| Attachment upload | 2 attempts: 5s |
| API GET | 1 retry |
| API POST/PUT/DELETE | No auto-retry |

### 21.2 Auth Layer

| Error | UI |
|---|---|
| 401 | Redirect to index.html; preserve route |
| `session.revoked` | Clear session and data; redirect |
| `account.disabled` | Clear session; navigate to Auth Gate |
| `account.deleted` | Same with different copy |
| Recovery-triggered session revocation | Same as `session.revoked` |

### 21.3 Server Errors

| Code | UI |
|---|---|
| 400 | "Something went wrong." |
| 403 | "You don't have permission." |
| 404 | Context-specific |
| 409 | Context-specific |
| 413 | "This file is too large." |
| 415 | "This file type isn't supported." |
| 429 | Toast with countdown |
| 5xx | "The server had a problem." |

### 21.4 Crypto Errors

| Error | Behavior |
|---|---|
| MLS decrypt failure | Mark "cannot decrypt" |
| Key commitment mismatch | Reject; warning |
| Safety number changed | Warning banner |
| Key transparency failure | Unverified |
| OPRF failure | Retry; app works without display-name decryption until it succeeds |

### 21.5 Media Errors

Transcode failure → passthrough. Upload failure → retry. Download failure (corrupt) → error. Presigned URL expired → fetch new URL.

### 21.6 Storage Errors

Quota warnings at 80%, blocks at 95%. Write failure → in-memory mode with banner.

### 21.7 MLS Errors

Out-of-sync epoch → "Syncing encryption state…". Missing welcome → poll every 5s; after 5min, "Waiting for a room member to add you". Pending MLS remove → room marked "left". Keystore corruption → wipe, mark rooms error, offer rejoin.

### 21.8 Interaction Edge Cases

| Scenario | Behavior |
|---|---|
| Edit, then unsend | Tombstone replaces latest version |
| Unsend, then edit | Edit absent |
| Disappearing message edited | Edit inherits `expires_at` |
| Reply to expiring message | `[message expired]` after expiration |
| Message during MLS transition | Composer "Securing…"; send queued |
| Same file in two rooms | Stored separately |
| Member list without cached display name | `user_id` placeholder until first message |

---

## 22. Plugin Inventory

| Plugin | Options | Purpose |
|---|---|---|
| `state-plugin` | `initialState` | Global keyed pub/sub store |
| `router-plugin` | — | History API + query-param routing |
| `socket-plugin` | `socketUrl`, `apiBase`, `appKey` | WebSocket connection |
| `crypto-plugin` | `wasmPath`, thresholds | MLS wrapper |
| `oprf-plugin` | `apiBase` | OPRF blinding + token derivation |
| `media-plugin` | `presets`, `workerPoolSize` | Transcode, thumbnail, strip |
| `transcription-plugin` | `models`, `modelBaseUrl`, `defaultModel` | Local speech-to-text |
| `push-plugin` | `vapidKey`, `apnsConfig`, `fcmConfig` | Push registration |
| `storage-plugin` | `dbName`, `blobRoot` | SQLite + blob filesystem |
| `i18n-plugin` | `locales`, `defaultLocale` | Translation |
| `theme-plugin` | `presets`, `accent` | Theme application |
| `call-plugin` | `iceServers`, `transport` | Call state machine |
| `p2p-plugin` | `iceServers`, `chunkSize` | WebRTC data channels |
| `icon-plugin` | `styleMap`, `registry` | Solar icon resolution |
| `notification-plugin` | `defaultSound` | In-app banners |
| `search-plugin` | `threshold`, `debounceMs` | Fuse.js lifecycle |

No event bus. Cross-component communication uses `emit` and `$state` key subscriptions.

---

## 23. Appendices

### 23.1 SQLite Schema

| Domain | Tables |
|---|---|
| Meta | `_migrations`, `_meta` (includes `last_user_seq`, `oprf_token_cache_key`) |
| Identity | `users` (cached display names keyed by user_id) |
| Rooms | `rooms`, `room_members`, `room_order` |
| Messages | `messages`, `message_versions`, `reactions` |
| Attachments | `attachments` |
| User state | `read_state`, `drafts`, `outbox`, `blocked_users`, `room_preferences`, `device_names` |
| Personalization | `wallpapers`, `sticker_packs`, `custom_stickers`, `blocked_sticker_packs` |
| Calls | `calls`, `call_participants` |
| Sync | `sync_state` (per-room epoch/seq), `processed_events` |
| MLS | `mls_rooms` |
| Settings | `settings` |

**Principles:** Flat columns for queryable fields; JSON blobs for opaque metadata. IDs are TEXT. Timestamps are INTEGER ms. Encrypted payloads are BLOB. WAL mode. Forward-only migrations.

**New in V1.1:**

- `_meta.last_user_seq` — integer cursor for user-scoped sync
- `device_names` table — locally cached, decrypted device names (encrypted at rest with SQLCipher)
- `users` — cache of decrypted display names keyed by `user_id`

### 23.2 Design Tokens

Full token list in `src/styles/tokens.css`. Structure: primitives, spacing, typography, radii, shadows, motion, z-index, layout metrics, safe areas. Dark mode swaps the semantic layer.

### 23.3 Event Catalog

**Room channel (`private-room-{room_id}`):**

| Event | Payload |
|---|---|
| `message.new` | `{ id, room_id, sender_user_id, sender_client_id, epoch, seq, content_type, created_at }` |
| `message.deleted` | `{ id, room_id }` |
| `room.updated` | `{ room_id, metadata?, retention_days?, max_file_size_bytes? }` |
| `room.member_added` | `{ room_id, user_id, role }` |
| `room.member_removed` | `{ room_id, user_id }` |
| `epoch.updated` | `{ room_id, epoch, sequence }` |
| `mls.add_pending` | `{ room_id, target_user_id, client_ids }` |
| `read.count` | `{ room_id, message_id, read_by_count }` |
| `reaction.added` | `{ room_id, message_id, user_id, emoji }` |
| `reaction.removed` | `{ room_id, message_id, user_id, emoji }` |
| `message.edited` | `{ id, room_id, reply_to, editor_user_id, edited_at }` |

**User channel (`private-user-{user_id}`):**

| Event | Payload | Durable |
|---|---|---|
| `read.sync` | `{ room_id, last_read_message_id, user_seq }` | Yes |
| `room_order.sync` | `{ order: string[], user_seq }` | Yes |
| `device.added` | `{ device_id, user_seq }` | Yes |
| `device.revoked` | `{ device_id, reason }` | Yes |
| `device.name_updated` | `{ device_id, encrypted_name, user_seq }` | Yes |
| `session.revoked` | `{ session_id, reason }` | Yes |
| `account.disabled` | `{ reason }` | Yes |
| `account.deleted` | `{}` | Yes |
| `mls.welcome_ready` | `{ room_id, welcome_id }` | No |
| `room.transfer_initiated` | `{ room_id, from_user_id, transfer_id }` | Yes |
| `room.transfer_cancelled` | `{ room_id, transfer_id, reason }` | Yes |

**Client events (Sockudo-published, ephemeral):**

| Event | Payload | Purpose |
|---|---|---|
| `client-typing.start` | `{ user_id }` | Typing indicator |
| `client-typing.stop` | `{ user_id }` | Typing indicator |
| `client-mls-request` | `{ room_id }` | Prompt pending MLS add |

`client-read` has been removed — read state is written via REST (`POST /users/me/read-state`).

### 23.4 Client Events

See table above. **Note:** `client-read` is deprecated. Read state writes go through `POST /users/me/read-state`.

### 23.5 Error Copy Reference

Consolidated error strings, grouped by layer.

### 23.6 Server Assumptions

The client implements against the server amendment package. If implementation differs, client adjusts in a coordinated update.

**Blocking items:**

- OPRF lookup flow
- Encrypted display names
- `private-user-{user_id}` channel with `user_seq`
- `GET /users/me/sync` endpoint
- REST writes for read state and room order
- `pending_mls_adds`
- Separate recovery flow endpoints
- Server-generated recovery codes
- Key transparency log
- Call signaling and TURN
- Message editing, reactions, threading

**Breaking-change coordination:** The server ships OPRF + user-scoped sync + device model changes as one release; the client ships the matching changes as one release. No partial rollout.

---

## 24. Amendments

| # | Date | Change |
|---|---|---|
| 1 | 2026-09-29 | Initial draft. Includes OPRF-based username lookup, server-generated recovery codes, separate recovery flow, device name sync, multi-device read state, and all subsequent server amendment integrations. |

---

## 25. Document Status

This is the contract for the client side of the system. Every client implementation task references this document. If a task conflicts with this spec, the task is wrong and must be revised.

Amendments are tracked in §24.

---

**End of Client Specification v1.0.**
