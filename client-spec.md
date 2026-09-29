# Client Specification v1.0

> **Status:** Draft — source of truth for client implementation tasks.
> **Scope:** This document describes the client-side application, its architecture, behavior, and contract with the server. Server behavior is defined by the Server Specification v1.0.
> **Stack:** Coralite 1.0.0-rc.5 · Wire CoreCrypto 10.5.2 (WASM) · Mediabunny · Capacitor · Tauri
> **Design language:** Privacy-first messenger. Adopts proven social-interaction patterns; visual density follows modern Western conventions. Not a clone of any single product.

Amendments are tracked at §24. Any change to this document requires a documented amendment.

---

## 1. Product Summary

A self-hosted, end-to-end encrypted group messaging client. The server is an untrusted delivery service that never sees plaintext, keys, or meaningful metadata. The client holds all plaintext, all keys, and all MLS state. Encryption, media processing, and transcription run locally.

| Layer | Technology |
|---|---|
| Framework | Coralite 1.0.0-rc.5 |
| Runtime | Node.js ≥ 22.22.2, ESM |
| MLS engine | Wire CoreCrypto 10.5.2 (WASM, bundled) |
| Attachment crypto | Web Crypto API (AES-256-GCM, HKDF, SHA-256) |
| Media processing | Mediabunny (WebCodecs-first, ffmpeg.wasm fallback) |
| Transcription | Moonshine Tiny / Small, Whisper Tiny (ONNX Runtime Web) |
| Real-time | pusher-js (Sockudo, Pusher Protocol 7) |
| Search | Fuse.js (lazy-loaded) |
| Mobile shell | Capacitor (iOS, Android) |
| Desktop shell | Tauri |
| Storage | SQLite (native on mobile/desktop, WASM+OPFS on web) |
| CSS | Tailwind utilities + Coralite scoped `<style>` |
| Icons | Solar icon set (tree-shaken) |
| Emoji picker | `emoji-picker-element` |
| Waveform | `wavesurfer.js` |
| QR codes | `qr-code-styling` |

---

## 2. Scope

### 2.1 In Scope — V1

- Two-page SPA: `index.html` (auth gate), `app.html` (messenger shell)
- Three-panel desktop layout; stacked mobile layout; tablet two-panel layout
- Room-based conversation model (1:1 = 2-member room; no contact list)
- Text messaging with Markdown formatting
- Message editing (15-minute window, signed edit chain)
- Reactions (silent — no notification, no read receipt, no unread badge)
- @mentions with autocomplete and mute override
- Disappearing messages (24h / 7d / 90d)
- Read receipts (aggregate count, mutual setting)
- Typing indicators (via client events)
- Message deletion (tombstone via server endpoint)
- Encrypted attachments (C2SP chunked AES-256-GCM)
- Range-request streaming and seeking
- Presigned URL handling (S3 backends)
- P2P fallback for files exceeding room limits (WebRTC data channels)
- Media processing pipeline (image, video, audio)
- Client-side poster extraction for video
- Client-side thumbnail generation for media
- Metadata stripping for all processed media
- Local voice message transcription (Moonshine, Whisper)
- Voice messages with interactive waveform player
- Stickers (bundled + custom + combined) with entrance transitions
- Emoji picker
- Client-generated link previews (with optional server proxy for CORS-blocked URLs)
- In-thread search (Fuse.js, per-room index)
- Global message search
- Media, Documents, Links curation views (filter + search)
- Room creation, member management, invite links + QR
- Room settings (off-canvas right drawer on desktop; full-screen push on mobile)
- Per-room, per-user theming (theme, wallpaper, bubble style)
- Manual room ordering (drag-and-drop)
- Biometric unlock (Face ID, Touch ID, Windows Hello)
- Recovery code flow (break-glass account recovery)
- Device approval for new devices
- Multi-device read state (via `private-user-{user_id}` channel)
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

### 2.2 Server V2 Requirements (Client V1 Dependencies)

The following are required by the client's V1 spec and must be provided by the server team's next build phase:

- `private-user-{user_id}` channel (multi-device read state, device revocation, room order sync)
- `read.sync` and `room_order.sync` events
- `pending_mls_adds` table and coordination (mirrors `pending_mls_removes`)
- `PATCH /rooms/:id` with `metadata` field (room metadata JSON update)
- Key transparency log + auditor signatures in identity key responses
- Link preview proxy (opt-in) with SSRF guard
- Message editing endpoint and storage
- Reactions table and endpoints
- Threading via `reply_to` (already in spec)
- `POST /users/me/avatar` endpoint
- Call signaling events (WebRTC signaling, TURN credentials endpoint)
- Member list pagination
- Retention change preview endpoint

### 2.3 Out of Scope

- Multi-page routing (settings, admin, room settings are views, not routes)
- Contact/friend list (rooms only)
- Server-side key escrow
- Plaintext export of message contents
- Username changes
- Federation between servers
- Multi-tenancy
- Multiple accounts on one device
- Analytics or crash reporting (privacy-first)
- Message forwarding (users can copy/paste)
- Message pinning
- Chat screenshot (LINE-style export)
- GIF search (external API)
- User status / away message
- Office document inline rendering
- Native keyboard sticker access

---

## 3. Design Principles

1. **Clear primary task** — the messenger is chat-first. Every view serves conversation.
2. **Privacy-first** — the server is untrusted. Security indicators are visible and unambiguous. Metadata leaks are documented, not hidden.
3. **Behavior over ornament** — animation and decoration serve function. Decoration is opt-in.
4. **Consistency through position** — navigation and actions appear in predictable places. Users never relearn.
5. **Asymmetric cultural adoption** — adopt proven social-interaction patterns (read receipts, stickers, mention guarantees) while keeping visual density closer to modern Western conventions. Do not inherit culture-specific design choices uncritically.
6. **Honest defaults** — the app is transparent about what it can and cannot do. No fake previews, no unverifiable claims.
7. **Ask when needed** — permissions and prompts appear at the moment of relevance, not at first launch.

---

## 4. Architecture

### 4.1 Pages

The application has **two Coralite pages**:

| Page | File | Purpose |
|---|---|---|
| Auth Gate | `src/pages/index.html` | Login, registration, recovery, ALTCHA. Redirects to the SPA on success. |
| Messenger | `src/pages/app.html` | Loads the SPA shell. All subsequent navigation is client-side. |

There is no separate settings, admin, profile, or room-settings page. Those are views or overlays within `app.html`.

### 4.2 SPA Routing

Client-side router (History API). The route determines which view renders into Panel 3 (desktop) or the full-screen outlet (mobile).

| Route | View |
|---|---|
| `/chat/:roomId` | `view-chat` |
| `/chat/:roomId/settings` | `view-room-settings` |
| `/chat/:roomId/message/:msgId` | `view-chat`, scrolled to message |
| `/media` | `view-media` |
| `/documents` | `view-documents` |
| `/links` | `view-links` |
| `/calls` | `view-calls` |
| `/settings` | `view-settings` |
| `/settings/:section` | `view-settings`, section expanded |
| `/profile/:userId` | `view-profile` |
| `/join/:token` | `view-join-room` |

Unknown paths route to the chat list.

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
│   │   ├── primitives/       # ui-* single-responsibility
│   │   ├── composed/         # profiles, bubbles, rows
│   │   ├── containers/       # list/thread composers
│   │   ├── views/            # SPA route views
│   │   └── shell/            # messenger-shell, hosts
│   ├── plugins/              # factory → definePlugin
│   ├── lib/
│   │   ├── mls/              # CoreCrypto wrapper
│   │   ├── crypto/           # C2SP, HKDF, Web Crypto
│   │   ├── media/            # Mediabunny pipeline
│   │   ├── transcription/    # Model loading + inference
│   │   ├── api/              # Fetch wrapper
│   │   ├── p2p/              # WebRTC data channels
│   │   ├── db/               # SQLite repositories
│   │   ├── blobs/            # Blob filesystem
│   │   ├── store/            # Global state
│   │   ├── actions/          # Orchestration functions
│   │   ├── events/           # Socket event handlers
│   │   ├── validation/       # Input schemas
│   │   └── utils/
│   ├── styles/
│   │   ├── tokens.css
│   │   └── utilities.css
│   └── public/
│       ├── stickers/         # Bundled sticker packs
│       ├── wallpapers/       # Preset wallpapers
│       ├── sounds/           # Notification sounds
│       └── icons/            # Solar sprite or tree-shaken imports
├── package.json
├── tailwind.config.js
└── migrations/               # SQLite migrations
```

### 4.4 Plugin Architecture

Plugins are **factory functions** returning `definePlugin` results. Values from `coralite.config.js` flow through the factory closure.

```js
// src/plugins/socket-plugin.js
import { definePlugin } from 'coralite'

export default (options) => definePlugin({
  name: 'socket-plugin',
  client: {
    config: options,
    context: (pluginContext) => (instanceContext) => ({
      useSocket: (handlers) => { /* ... */ }
    })
  }
})
```

```js
// coralite.config.js
import socketPlugin from './src/plugins/socket-plugin.js'
import cryptoPlugin from './src/plugins/crypto-plugin.js'
import statePlugin from './src/plugins/state-plugin.js'

export default defineConfig({
  plugins: [
    statePlugin({ initialState: {} }),
    socketPlugin({
      socketUrl: process.env.SOCKUDO_URL,
      apiBase: process.env.API_BASE_URL,
      appKey: process.env.SOCKUDO_APP_KEY
    }),
    cryptoPlugin({
      wasmPath: '/wasm/corecrypto.wasm',
      targetKeyPackageCount: 20,
      keyPackageLowWatermark: 5
    })
  ]
})
```

**Two-phase resolver pattern:** `client.context` is a curried function. The first call runs once as a singleton; the returned function runs per component instance. Expensive setup (loading WASM, opening keystores, creating indexes) happens in the outer phase. Per-instance concerns (subscriptions, cleanup handlers) happen in the inner phase.

### 4.5 State Management

Global state is managed by the **global store plugin** (a Coralite plugin). The pattern is a keyed pub/sub store:

- A singleton state object initialized with defaults and merged with `options.initialState`.
- A `listeners` Map keyed by property name, each holding a Set of callbacks.
- A **Proxy** over the state object that intercepts `subscribe(key, handler)`, `set(key, value)`, and regular property access.
- Subscription cleanup via `instanceContext.signal` on component unmount.

**State keys:**

| Key | Shape |
|---|---|
| `currentUser` | `User \| null` |
| `capabilities` | `Capabilities` |
| `rooms` | `Record<roomId, Room>` |
| `roomOrder` | `string[]` |
| `selectedRoomId` | `string \| null` |
| `messages:{roomId}` | `Message[]` (dynamic key per room) |
| `members:{roomId}` | `Member[]` |
| `typing:{roomId}` | `TypingUser[]` |
| `outbox` | `OutboxEntry[]` |
| `unreadCounts` | `Record<roomId, number>` |
| `activeCall` | `Call \| null` |
| `callHistory` | `Call[]` |
| `settings` | `Record<string, unknown>` |
| `ui` | `{ activeRail, modal, toasts, offline }` |

**Fine-grained updates** come from key-specific subscriptions. A component rendering room A subscribes to `messages:roomA`. A message in room B updates `messages:roomB`, which A's subscribers never see.

**Source of truth:** SQLite. The store is an in-memory view. If the store is wiped, it rebuilds from SQLite.

### 4.6 Boot Sequence

```
1. Render app shell (splash / empty layout)          < 100ms
2. Read session token from secure storage
   ├─ None → route to Auth Gate (index.html)
   └─ Present → continue
3. Open SQLite, run migrations
4. Hydrate from SQLite:
   ├─ Rooms, members, room order
   ├─ Unread counts, drafts, preferences
   ├─ Last-viewed room's recent messages
   └─ Sync cursors
5. CoreCrypto initialization
   ├─ Load device secret from secure storage
   ├─ Derive entropy seed
   ├─ Open keystore
   └─ mlsInit()
6. Render UI with hydrated data
7. Fetch capabilities (GET /api/v1/capabilities)
8. Connect WebSocket (pusher-js)
9. Delta sync per room (since cursor)
10. Background: key package replenishment, welcome polling,
    pending removes, thumbnail generation
```

**Failure modes:**

| Step | Failure | Behavior |
|---|---|---|
| 2 | Corrupt token | Clear storage, route to Auth Gate |
| 3–4 | SQLite unavailable | Show degradation notice; operate in memory-only mode |
| 5 | Keystore corrupted | Wipe keystore, re-init; mark all rooms `mls_status='error'` |
| 8 | Socket fails | Retry with backoff; app remains usable from cache |
| 9 | Delta sync fails | Mark rooms as "stale"; retry |

### 4.7 Sync & Offline

**Readable offline:** All cached messages, media, and room metadata remain readable. The chat list and threads function normally.

**Outbox queue:** Messages composed while offline are persisted in a SQLite outbox. Each entry holds the plaintext, the room ID, a temp ID, and a state (`queued`, `sending`, `failed`).

**Queue processing:** On reconnect, the client processes the outbox FIFO. Each message is encrypted and submitted. On success, the temp message is reconciled with the server-assigned ID. On failure after retries, the message is marked `failed` with a Retry/Delete action.

**No global offline banner.** Feedback is inline on messages and via one-shot toasts. Attempts to perform server-required actions (create room, generate invite, start P2P) surface a toast: *"This needs a connection. Try again when you're back online."*

**Delta sync:** Per-room cursor `(epoch, seq)`. Both initial and delta sync responses are ordered ascending; the client renders directly without reordering.

---

## 5. Design System

### 5.1 Naming Conventions

Components follow a canonical vocabulary. Where a term is shared across messengers (profile, room), we adopt it. Where it's product-specific (Talk, Keep), we use neutral terms.

| Term | Component | Reason |
|---|---|---|
| **Profile** | `ui-profile` | Not a simple avatar — supports decoration, sub-profiles, status |
| **Chat** | `view-chat` | Universal term |
| **Room** | `room-*` | Canonical container |
| **Thread** | `message-thread` | The message list |

### 5.2 CSS Strategy

| Layer | Tool | Scope |
|---|---|---|
| Design tokens | CSS custom properties (`:root`) | Global |
| Global utilities | Tailwind | Entire app |
| Component styles | Coralite scoped `<style>` | Per-component |

Tailwind provides layout utilities (`flex`, `grid`, `gap-*`), spacing, responsive prefixes, and text truncation. Components own structure and variant styling. Coralite's scoped styles prevent leakage without Shadow DOM, so Tailwind utilities work inside components.

### 5.3 Theming

**Global theme:** light / dark / auto. Token swap under `[data-theme="dark"]`.

**Accent color:** TBD. Placeholder blue (`#3B82F6`) used until brand decision. All components reference `var(--accent-*)`, so the change is a single-block edit.

**Per-room, per-user theming:** Each room container can override a small subset of tokens (bubble colors, background, wallpaper, bubble style). Themes are stored in `room_preferences` in SQLite (local only). They are not shared with other room members.

**Component variant pattern:**

```html
<ui-profile size="md" shape="circle" state="online"></ui-profile>
<ui-button variant="primary" size="md" disabled></ui-button>
<chat-bubble variant="outgoing" status="read"></chat-bubble>
```

Variants map to scoped CSS modifier classes. No separate components for variants.

### 5.4 Icons

Icons come from the **Solar icon set**, tree-shaken via `@solar-icons/static`. No sprite.

The `ui-icon` component resolves a **canonical name** and a **state** to a concrete icon via the `icon-plugin`:

```html
<ui-icon name="chat" state="active"></ui-icon>
```

The plugin maps states to Solar styles (`linear` → default, `bold` → active, `bold-duotone` → emphasis). The mapping is overridable per-instance with a `style` attribute.

**No emoji are used as icons.** All icons come from Solar.

### 5.5 Motion

Motion uses tokens from the design system:

| Token | Value | Use |
|---|---|---|
| `--duration-fast` | 100ms | Micro-interactions (button press) |
| `--duration-normal` | 200ms | Standard transitions |
| `--duration-slow` | 300ms | Larger surfaces (drawer slide) |
| `--ease-standard` | `cubic-bezier(0.2, 0, 0, 1)` | Default |
| `--ease-decelerate` | `cubic-bezier(0.05, 0.7, 0.1, 1)` | Content entering |
| `--ease-accelerate` | `cubic-bezier(0.3, 0, 0.8, 0.15)` | Content exiting |
| `--ease-spring` | `cubic-bezier(0.34, 1.56, 0.64, 1)` | Playful overshoot |

All motion respects `prefers-reduced-motion: reduce` — durations collapse to 0ms.

---

## 6. Layout

### 6.1 Three-Panel Desktop (≥ 1024px)

```
┌──────┬──────────────┬──────────────────────────────┐
│ Rail │ List Panel   │ Detail Panel                 │
│ 64px │ 320–400px    │ flex                         │
└──────┴──────────────┴──────────────────────────────┘
```

The layout follows an **explorer pattern** (Finder, Outlook): selecting an item in Panel 2 highlights it and renders its detail in Panel 3. The rail switches the "location," which changes what Panel 2 lists.

**Rail items:** Chat, Media, Documents, Links, Calls, Settings.

**Detail panel content by location:**

| Location | Panel 2 | Panel 3 |
|---|---|---|
| Chat | Conversation list | Selected thread |
| Media | Media grid | Selected media viewer |
| Documents | Document list | Selected document |
| Links | Link list | Selected link preview |
| Calls | Call history | Selected call detail |
| Settings | Section list | Section detail |

### 6.2 Tablet (768–1023px)

Rail collapses to a bottom nav. Panels 2 and 3 remain side by side. Room settings open as a full-screen push within Panel 3.

### 6.3 Mobile (< 768px)

Single column. Bottom navigation with four items: **Chat, Media, Calls, Profile**.

The Media tab hosts an internal segmented control: **Photos / Docs / Links**.

### 6.4 Floating Glass Chrome

Chat header and composer are absolute overlays with `backdrop-filter: blur(20px) saturate(180%)`. The message thread scrolls underneath.

```
chat-view              position: relative; overflow: hidden
├── chat-header        position: absolute; top: 0; z-index: 10
├── chat-scroll        position: absolute; inset: 0; overflow-y: auto
│   └── chat-messages  padding-top: var(--header-height)
│                      padding-bottom: var(--composer-height)
├── scroll-to-bottom   position: absolute; bottom: calc(--composer-height + 16px)
└── chat-composer      position: absolute; bottom: 0; z-index: 10
```

**Dynamic offsets.** Composer height is tracked via `ResizeObserver`. Header height includes `env(safe-area-inset-top)`; composer includes `env(safe-area-inset-bottom)`.

**Scroll edge cue.** A `data-scrolled` attribute adds a subtle border when content is behind the glass.

**Fallback.** `@supports not (backdrop-filter: blur(20px))` and a low-end device check (`navigator.hardwareConcurrency < 4 || navigator.deviceMemory < 2`) disable the glass effect and use a solid background.

### 6.5 Safe Areas and Status Bar

The app uses **edge-to-edge** rendering. Safe area insets are provided by:

- **Capacitor:** `capacitor-edge-to-edge` or `@aashu-dubey/capacitor-statusbar-safe-area`
- **Tauri:** `tauri-plugin-safe-area-insets-css`
- **Web:** `env(safe-area-inset-*)` CSS environment variables

Both plugins expose consistent CSS variables. Components don't branch on platform. Root viewport height uses `100dvh`.

### 6.6 Navigation Patterns

Three affordances, three purposes:

| Affordance | Purpose | When |
|---|---|---|
| **Back arrow** (←) | Nested navigation | Returning to a parent view in a stack |
| **Close** (✕) | Modal dismissal | Dismissing a full-screen modal overlay |
| **OK / Cancel** | Forced decision | Dialogs requiring an explicit choice |

**Platform split:**

- **Desktop:** The explorer pattern presents everything simultaneously. Back arrows don't appear — there's nothing to go "back" to. Media viewers open embedded in Panel 3 when the rail location is browse-oriented; they open as full-screen modal overlays (with ✕) when the rail location is Chat.
- **Mobile:** Everything is full-screen. Navigation is a stack. Back arrows are used for all drill-downs.

**Action placement:**

| Position | Reserved for |
|---|---|
| Top-left | Navigation (back arrow or X) |
| Top-center | Title (room name, view name, filename) |
| Top-right | Contextual actions (overflow menu, primary action) |

---

## 7. Component Inventory

### 7.1 Primitives

| Component | Responsibility | Key attributes |
|---|---|---|
| `ui-profile` | User/group profile image with states and decoration | `size`, `shape`, `src`, `state`, `fallback` |
| `ui-profile-group` | Composite of 2–4 profiles; `+N` for larger | `max`, `size` |
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
| `ui-timestamp` | Relative or absolute time | `datetime`, `format` |
| `ui-icon` | Solar icon wrapper | `name`, `state`, `size`, `color` |
| `ui-sheet` | Bottom sheet / dialog | `variant`, `open` |
| `ui-toast` | Transient message | `variant`, `duration` |
| `ui-tooltip` | Hover/focus hint | `placement` |
| `ui-disclosure` | Collapsed/expandable section | `label`, `open` |
| `ui-context-menu` | Trigger-adaptive menu | `items`, `trigger` |

### 7.2 Composed

| Component | Composed of | Responsibility |
|---|---|---|
| `chat-bubble` | Shape + slot | Message container with variant |
| `chat-bubble-group` | `ui-profile` + `chat-bubble` × N | Consecutive messages from one sender |
| `message-text` | — | Markdown-rendered body |
| `message-attachment` | — | Image/file/audio preview |
| `message-reaction` | `ui-chip` | Reaction with count |
| `message-meta` | `ui-timestamp` + receipt icon | Time + status |
| `message-waveform` | `wavesurfer.js` | Interactive voice player |
| `message-transcript` | — | Transcript display under waveform |
| `message-tombstone` | — | Deleted message placeholder |
| `typing-dots` | — | Animated typing indicator |
| `list-row` | `ui-profile` + text + trailing slot | Generic list row |
| `conversation-row` | `list-row` | Conversation list item |
| `media-thumbnail` | — | Grid thumbnail with selection |
| `link-card` | favicon + title + domain | Shared link preview |
| `document-row` | file-type icon + name + meta | Shared file list item |
| `file-icon` | `ui-icon` + category color | File type icon |
| `nav-item` | icon + label | Rail or bottom-nav item |
| `form-field` | label + control + hint/error | Wraps `ui-input`/`ui-textarea` |
| `search-field` | `ui-input` + icon | Search input |
| `section-header` | — | Group label in lists |
| `empty-state` | icon + copy + action slot | Empty list state |
| `safety-number` | — | MLS safety number display |
| `call-participant` | `ui-profile` + status | Single call participant tile |

### 7.3 Containers

| Component | Responsibility |
|---|---|
| `conversation-list` | Virtualized list of `conversation-row` |
| `message-thread` | Virtualized list of `chat-bubble-group` (TanStack Virtual, `anchorTo: 'end'`) |
| `message-composer` | Input + attach + send + emoji + voice recorder |
| `media-grid` | Virtualized 3-column flush grid |
| `document-list` | Virtualized list group |
| `link-list` | Virtualized list group |
| `settings-list` | List of settings rows |
| `auth-form` | Auth gate form |
| `media-viewer` | Full-screen slideshow viewer |
| `link-preview` | Full link preview with actions |
| `document-preview` | File preview with actions |
| `call-participant-grid` | Grid/focus layout for calls |
| `message-reply-preview` | Reply bar above composer |
| `sticker-picker` | Bottom sheet with tabs, grid, search |

### 7.4 Shell

| Component | Responsibility |
|---|---|
| `messenger-shell` | Root layout: rail/bottom nav + panels + overlay hosts |
| `modal-host` | Renders active overlay from SPA store |
| `toast-host` | Renders toast queue |
| `call-overlay-host` | Renders minimized and active call overlays |
| `notification-banner-host` | Renders in-app notification banners |

### 7.5 Views

| View | Contains |
|---|---|
| `view-chats` | `conversation-list` |
| `view-chat` | `message-thread` + `message-composer` |
| `view-media` | Filter bar + `media-grid` |
| `view-documents` | Filter bar + `document-list` |
| `view-links` | Filter bar + `link-list` |
| `view-calls` | `call-history-list` |
| `view-settings` | Section list + detail sub-views |
| `view-profile` | Profile detail with `safety-number` |
| `view-room-settings` | Off-canvas drawer (desktop) / push (mobile) |
| `view-join-room` | Invite link confirmation |
| `view-admin` | Administration section |

---

## 8. Crypto & E2EE

### 8.1 Two Layers

| Layer | Purpose | Implementation |
|---|---|---|
| **MLS** | Message encryption, group state, key packages, welcomes, commits | Wire CoreCrypto 10.5.2 (WASM, bundled) |
| **C2SP chunked AES-256-GCM** | Attachment blob encryption, range-based decryption | Web Crypto API |

MLS handles protocol-level E2EE. C2SP handles bulk content with random access. The manifest (file_id, key, salt, etc.) travels inside the MLS message; the blob itself is a C2SP ciphertext.

### 8.2 MLS Client Behavior

**CoreCrypto initialization:**

```
1. Load device_secret from platform secure storage
   (generate 32 random bytes on first launch if absent)
2. entropy_seed = HKDF-Expand(device_secret, info="mls-entropy-v1", length=32)
3. Open the encrypted keystore (managed by CoreCrypto)
4. CoreCrypto.init(client_id, keystore, entropy_seed)
5. CoreCrypto.mlsInit()
```

**Key package lifecycle:**

- Target: 20 unconsumed key packages per device.
- Low watermark: 5.
- Replenish on login and on app launch.
- Rotate every 30 days.

**Group lifecycle:**

- **Create:** `mlsCreateConversation(room_id)`, then batch-add members in a single commit.
- **Join:** Poll for welcome (`GET /welcomes`), process via `mlsProcessWelcomeMessage`, consume, fetch messages from welcome epoch.
- **Leave:** Server queues `pending_mls_remove`; another online member generates the Remove commit; the leaving client deletes local group state.

**Message encryption/decryption:**

- Encryption wraps the JSON payload in an MLS message, POSTs to `/rooms/:id/messages`.
- Decryption uses the message's epoch; if the message is from an older epoch, previous-epoch keys are used; if newer, the client queues and syncs.

**Epoch transitions:**

- The composer enters a **"Securing…"** state during transitions. Outgoing messages are queued and flushed once the new epoch settles.
- Concurrent commits are handled by CoreCrypto's deterministic tie-break; stale commits trigger re-attempts.

### 8.3 C2SP Attachment Encryption

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

**Padding:** Plaintext is padded to the nearest bucket from `ATTACHMENT_BUCKET_SIZES` **before** encryption. The padded plaintext is chunked as a single C2SP message producing one short final chunk.

**Range translation:**

```
start_chunk = p_start / 16384
end_chunk   = p_end   / 16384
encrypted_start = 56 + start_chunk * (16384 + 16)
encrypted_end   = 56 + (end_chunk + 1) * (16384 + 16) - 1
```

### 8.4 Key Transparency

The client consumes key transparency proofs from the server. The server's log structure and auditor deployment are defined by the server spec.

**Client responsibility:**

1. Fetch identity keys with `tree_head`, `auditor_signatures`, and `inclusion_proof`.
2. Verify the server signature, each auditor signature, and the inclusion proof locally.
3. Mark the contact as **transparency-verified** on success.
4. Display a green checkmark; no user action required in the common case.

**Failure cases:**

| Case | Display |
|---|---|
| Inclusion proof fails | Prominent warning; not verified |
| Auditor signatures missing | "Verified by server only" |
| Auditor signatures invalid | Warning; unverified |
| Network unavailable | "Verification pending" |

**Manual safety numbers** remain available under an "Advanced" section for users who want out-of-band verification. They are no longer the primary verification path.

### 8.5 Safety Numbers

Safety numbers are computed locally from MLS identity keys. They are displayed in the profile view and in the room profile. Verification is soft (advisory); the server's `SAFETY_NUMBER_MODE` is surfaced.

When a contact's identity key changes, the client shows a warning banner in shared rooms. The user can verify manually or dismiss.

---

## 9. Media Pipeline

### 9.1 Format Matrix

| Type | Primary | Fallback A | Fallback B |
|---|---|---|---|
| Video | WebM / VP9 + Opus | WebM / VP8 + Opus | MP4 / H.264 + AAC |
| Audio | Opus (Ogg or WebM) | AAC (M4A) | — |
| Image | AVIF | WebP | JPEG |

**Quality presets (global, default Balanced):**

| Preset | Image | Video | Audio |
|---|---|---|---|
| High | AVIF q80, max 2560px | VP9, 1080p, ~4 Mbps | Opus 96 kbps |
| Balanced | AVIF q70, max 1920px | VP9, 720p, ~2 Mbps | Opus 64 kbps |
| Small | AVIF q60, max 1280px | VP9, 480p, ~1 Mbps | Opus 48 kbps |

### 9.2 Transcoding

Mediabunny handles demux, decode, encode, resize, rotate, crop, and thumbnail extraction. GPU path uses WebCodecs (`VideoEncoder`/`VideoDecoder`, `OffscreenCanvas`). CPU fallback is `ffmpeg.wasm`, loaded lazily.

**Worker pool:** Concurrency cap of 2 (1 on low-end devices). Jobs above 500 MB are processed alone. Each job exposes an `AbortController`.

**Failed transcode:** Fall back to passthrough — the file is uploaded unprocessed as a generic attachment. A subtle notice appears on the chip.

### 9.3 Thumbnail Generation

During processing, the client generates a **128×128 WebP thumbnail** alongside the full attachment. The thumbnail is encrypted separately (same key/salt/context) and uploaded as a companion blob. The manifest carries `thumbnail_file_id`.

**Fallback:** If thumbnail generation fails, the recipient generates one after decrypting the full attachment and caches it locally.

### 9.4 Padding

Padding operates on the **plaintext** before encryption. The client:

1. Selects the smallest bucket that can contain the plaintext.
2. Computes `L_padded` from the bucket size (`T`):
   ```
   M = T - 72
   q = floor(M / 16400)
   r = M - 16400 * q
   if r < 16384: L_padded = 16384 * q + r
   else: skip to next bucket (unreachable)
   ```
3. Appends zero bytes to reach `L_padded`.
4. Chunks the padded plaintext.
5. Encrypts each chunk.

### 9.5 Metadata Stripping

All processed media is stripped of metadata (EXIF, GPS, device info, XMP, IPTC, ICC, ID3). Documents are not modified.

### 9.6 Fast-Start MP4

For MP4 files, the client validates that the `moov` atom is at the start. Files where `moov` follows `mdat` are re-muxed with `mp4box.js`. If re-muxing fails, the file is transcoded to WebM/VP9.

### 9.7 Progress and Cancel

Per-file progress is reported inline in the message thread:

| State | UI |
|---|---|
| `queued` | Clock icon |
| `transcoding` | Progress ring, cancel button |
| `encrypting` | Brief |
| `uploading` | Progress bar, cancel |
| `p2p_pending` | Waiting for recipient selection |
| `p2p_transferring` | Per-recipient sub-rows |
| `failed` | Retry or remove |

Long uploads surface a Live Activity on iOS and an ongoing notification on Android.

---

## 10. Attachments

### 10.1 Upload Flow

```
1. User attaches file(s)
2. Probe with Mediabunny
3. Transcode per preset (worker pool)
4. Extract poster frame (video)
5. Generate 128×128 thumbnail
6. Strip metadata
7. Compute padded size; pad plaintext to bucket
8. Chunk per C2SP
9. Derive key, nonce, commitment
10. Encrypt chunks
11. Compute SHA-256 of padded ciphertext (file_id)
12. Upload ciphertext (POST /rooms/:id/attachments)
13. Send MLS message with manifest
```

### 10.2 Download Flow

- **Files ≤ 1 MB:** Single GET.
- **Files > 1 MB:** Lazy chunk fetch.
- **Media (audio/video):** Always lazy.

Presigned URLs are used when `capabilities.storage_presign_supported` is true. URLs are not cached; on expiry, a fresh URL is requested and the download resumes.

### 10.3 P2P Fallback

When a file exceeds `effective_max_file_size_bytes` for the room:

1. A **member picker modal** appears.
2. WebRTC data channels are established (one per recipient).
3. The file transfers directly, encrypted with an ephemeral key per recipient.
4. Chunk-level acknowledgment enables resume on connection drop.
5. Cancel discards partial data on both sides.

**Online-only.** If the recipient is offline, the send fails with a clear message.

### 10.4 Deletion

| Operation | Scope | Permission |
|---|---|---|
| **Delete for me** | Local only | Anyone |
| **Unsend** | All members | Sender, within 24h (server-enforced) |
| **Remove from room** | All members | Owner (or moderator in Discord mode) |

**Blob deletion:** Triggered by unsend/remove. Server prunes when no other references exist.

### 10.5 Attachment Icon Mapping

File extensions map to six **canonical categories**, each with a color. The canonical name resolves to a Solar icon via the icon plugin.

| Category | Color | Extensions |
|---|---|---|
| `document` | Red | pdf, doc, docx, odt, rtf, pages |
| `spreadsheet` | Green | xls, xlsx, csv, ods, numbers |
| `presentation` | Orange | ppt, pptx, odp, key |
| `text` | Blue | txt, md, json, js, ts, py, rs, go, html, css, log |
| `archive` | Amber | zip, rar, 7z, gz, tar, bz2 |
| `design` | Purple | fig, sketch, xd, psd, ai |
| `generic` | Grey | everything else |

**Resolution order:** MIME type → extension → generic.

---

## 11. Timeline (Message Thread)

### 11.1 Bubble Layout

| Aspect | Own messages | Others' messages |
|---|---|---|
| Alignment | Right | Left |
| Avatar | Omitted | `ui-profile` on left, bottom-aligned |
| Sender name | N/A | Above the bubble on first message of a group (group chats only) |
| Bubble color | `var(--bubble-outgoing)` | `var(--bubble-incoming)` |
| Timestamp | Inside bubble, bottom-right | Inside bubble, bottom-left |
| Read receipt | "Read" / "Read N" next to timestamp | — |

Bubble max width: `min(75%, 600px)` on desktop. Corner radius: `18px` default; `4px` on the tail corner and inner group corners.

### 11.2 Sending States

| State | Icon |
|---|---|
| Queued | Clock |
| Sending | Diagonal upward-left arrow (↖) |
| Sent | No icon (timestamp only) |
| Read | "Read" / "Read N" |
| Failed | Circular arrow (↻); tap to retry |

### 11.3 Message Grouping

Consecutive messages from the same sender within 5 minutes are grouped: avatar and sender name appear only on the first message; timestamp on the last; inner corners tighten.

### 11.4 Date Separators

Centered pill-style separators on calendar day change: "Today", "Yesterday", "March 5, 2026". A sticky date header floats at the top of the viewport.

### 11.5 New Messages Divider

A horizontal line at the last-read position. Shown when opening a room with unread messages; hidden once scrolled past.

### 11.6 Scroll Behavior

- **On open:** Scroll to bottom, instant.
- **New incoming while pinned:** Auto-follow if within 80px of the bottom.
- **New incoming while scrolled up:** Do not follow; increment unseen count.
- **Own send:** Always scroll to bottom.
- **Scroll-to-bottom button:** Floating, appears when scrolled up; shows unseen count.
- **Infinite scroll:** Trigger at ~200px from top.
- **Prepend anchoring:** Viewport stays fixed on the message being read (TanStack Virtual `anchorTo: 'end'`).
- **Stable keys:** Message IDs (not indices).

### 11.7 Message Context Menu

Long-press (mobile) or right-click / hover bar (desktop) opens the message context menu.

**Universal actions:**

- Copy
- Reply
- React (opens quick-reaction strip)
- Select (enters multi-select mode)
- Delete for me

**Sender-only:**

- Unsend (within 24h)

**Conditional:**

- Edit (own messages, within 15-minute window)
- Show original (if edited)
- Delete (Discord mode: owner/moderator)

**Hover bar (desktop only):** A compact strip appears on the inner side of the bubble with four buttons: react, reply, edit, overflow. Right-click opens the full menu directly.

**Browser context menu suppression:** `preventDefault()` is scoped to `.chat-bubble` elements only.

### 11.8 Message Editing

**Affordance:** Long-press → "Edit" (mobile); hover bar → edit icon, or right-click → "Edit" (desktop).

**Edit mode:** The composer morphs — a banner shows "Editing" with a dismiss button, the input is pre-filled, and the send button becomes a checkmark.

**Window:** 15 minutes from the original send time. Enforced client-side and server-side.

**Edits:** Unlimited within the window. Each edit is a new signed message with `reply_to` set and `sequence` incrementing.

**Indicator:** "Edited" label next to the timestamp inside the bubble.

**History:** "Show original" opens a sheet showing the full edit chain with timestamps. New joiners see only versions they can decrypt; missing versions show "not available."

**Media messages:** Text/caption only. Attachments cannot be changed.

### 11.9 Voice Messages

**Recording flow (WhatsApp-style):**

| Gesture | Result |
|---|---|
| Press and hold mic | Recording starts; release to send |
| Slide left while holding | Cancel |
| Tap mic (without holding) | Hands-free mode |
| Tap send | Send |
| Tap trash | Discard |
| Tap play | Preview before sending |

**Composer button:** Dynamic — mic when empty, send when text present. First-use tooltip: *"Tap the mic to record, or start typing to send."*

**In-thread display:** `message-waveform` with `wavesurfer.js`:

- Play/pause
- Interactive seek
- Duration (current / total)
- Speed control (0.5×, 1.0×, 1.5×, 2.0×)
- Transcription button

**Peaks:** A compact float array (~100–200 values) is included in the payload for instant waveform rendering.

**Max duration:** 30 minutes.

### 11.10 Transcription

Local, in-browser, Web Worker, WebGPU-first with WASM fallback.

**Models:**

| Tier | Model | Size | Languages |
|---|---|---|---|
| Fast (default) | Moonshine Tiny | ~26 MB | 6 |
| Accurate | Moonshine Small | ~123 MB | EN-focused |
| Multilingual | Whisper Tiny | ~40 MB | 99 |

**Source:** Server instance, via a base URL in capabilities. Versioned paths. Cached via Cache API.

**Per-message override:** The transcript shows the model used, with a dropdown to re-transcribe.

**Settings:** Default model, auto-transcribe toggle, show-below-voice toggle, downloaded models list.

**Transcript source:** Generated locally per recipient. The sender does not attach transcripts.

### 11.11 Stickers and Emoji

**Emoji picker:** `emoji-picker-element`. Web component, ~12.5kB.

**Stickers:**

| Tier | Source | Delivery |
|---|---|---|
| Bundled | Ships with the app | Static assets |
| Shared pack | Encrypted attachment | Cached locally |
| Custom | User-created from camera roll | Processed as media |

**Pack manifest:** JSON with `id`, `name`, `publisher`, `version`, `tray_image`, and `stickers[]`. Each sticker has `id`, `format` (`webp` or `lottie`), `file`, `width`, `height`, `emojis`, `keywords`, and optional `duration_ms`.

**Animated stickers:** Lottie JSON, rendered via `dotlottie-web` with expressions disabled.

**Message payload:** `{ "type": "sticker", "pack_id": "...", "sticker_id": "...", "entrance": "pop" }`.

**Entrance transitions:** An in-place animation on first view. Options: `none`, `pop`, `drop`, `slide`, `crash`, `fade`. Sender can override via long-press in the picker. Plays only on first view of a new message; history scroll does not animate. Respects `prefers-reduced-motion`.

**Replay:** Tapping a sticker replays its entrance transition.

**Sticker combination:** Long-press a sticker in the picker → "Combine" → multi-select up to 6 → drag, resize, rotate → send. The combined sticker is rendered to a canvas and sent as an image attachment.

**Sticker suggestions:** A keyword-to-sticker mapping runs locally as the user types, showing a small strip of suggested stickers above the composer.

### 11.12 Link Previews

Generated client-side by the sender. The preview (title, description, site name, favicon, and optionally an image) is embedded in the message payload.

**Fetch:** Debounced (500ms) after URL detection. 3-second timeout. If the fetch fails (CORS, timeout, no metadata), the client falls back to a minimal card showing only the domain and URL.

**Multiple links:** Only the first URL gets a preview. Others render as inline text links.

**Opt-in server proxy:** For URLs blocked by CORS, an optional server-side proxy (per-instance, opt-in) can fetch and return metadata. The proxy uses `agent-fetch` for SSRF protection: DNS resolution, IP validation, DNS pinning, and redirect re-validation. No logging, no per-request attribution. The response is encrypted with a Content Key the client generates, so the proxy is blind to the URL.

### 11.13 Reply / Quote

- **Trigger:** Swipe right on mobile; hover bar or right-click → Reply on desktop.
- **Preview bar** above the composer shows the quoted sender and snippet.
- **Quoted snippet** appears inside the reply bubble, tappable to jump to the original.
- **Payload:** `reply_to: "<message_id>"`.

### 11.14 @Mentions

**Composer autocomplete:** Typing `@` opens a popover listing room members. Filtering is prefix-based on display name and username. Selection is required — typed mentions without selection are plain text.

**Payload:**

```json
{
  "type": "text",
  "text": "Hey @Alex how are you?",
  "mentions": [
    { "user_id": "u_abc123", "offset": 4, "length": 5 }
  ]
}
```

**Rendering:** Accent color pill, tappable to profile. Mentions of the current user get additional emphasis (left border accent on the bubble).

**Notification override:** Mentions override mute but not "Nothing." The server sends a generic push to all room subscribers; the client decrypts locally and decides whether to display. On iOS, a Notification Service Extension handles the decryption; on Android, a `FirebaseMessagingService` subclass; on Web, the service worker's push event.

**@everyone / @here:** Not in V1.

### 11.15 Markdown Formatting

Supported: bold, italic, strikethrough, inline code, links, bullet lists, numbered lists, block quotes. Rendered client-side.

### 11.16 Message Tombstone

When a message is deleted, the bubble is replaced in place with a muted pill: "This message was deleted." The tombstone retains the original alignment but shows no avatar or reactions.

### 11.17 In-Thread Search

- **Entry point:** Magnifying glass in the chat header.
- **Overlay:** Slides down from the top.
- **Scope:** Current room only.
- **Library:** Fuse.js, lazy-loaded on first search.
- **Index:** Built from the decrypted message store; cached per room; incrementally updated.
- **Debounce:** 300ms.
- **Result grouping:** By date.
- **Highlight color:** `#FFE066`.
- **Jump to message:** Scroll + 2s yellow flash + "Back to search" affordance.
- **Date search:** Calendar sheet with bold dates.
- **Desktop shortcuts:** `Ctrl/Cmd+F`, `Enter`/`Shift+Enter`, `Esc`.
- **Large rooms:** `FuseWorker` above 5,000 messages.

### 11.18 Global Search

A search field in the chat list header. Searches **both room names and message content across all rooms**. Results group by room. Tapping a message result jumps to that message in the room.

### 11.19 Disappearing Messages

Room-level timer stored in the encrypted `metadata` JSON:

```json
{
  "schema_version": 1,
  "name": "Design Team",
  "avatar_file_id": "...",
  "disappearing_timer": 604800
}
```

**Options:** Off (`null`), 24h (86400), 7d (604800), 90d (7776000).

**Who sets it:** Room owner only.

**Applies to:** New messages only. Existing messages keep their original expiration (or no expiration).

**Timer start:** At send time. The sender computes `expires_at = now + duration` and includes it in the encrypted payload.

**Icon:** Clock on the bubble; tap shows the remaining time.

**Cleanup:** Local-only. A periodic task queries `SELECT id FROM messages WHERE expires_at <= now`, deletes those rows, and removes associated attachments from the local cache if no other message references them.

**Interactions:**

- **Edits** inherit the original's `expires_at`. Editing does not extend the timer.
- **Replies** to an expiring message show `[message expired]` in the quoted snippet after expiration.
- **Curation views** reflect the removal on next render.
- **Search indexes** are updated when messages are deleted.

**Server involvement:** None. The timer travels in the encrypted metadata.

---

## 12. Composer

### 12.1 Controls

| Element | Action |
|---|---|
| `[＋]` | Attachment picker |
| `[😊]` | Emoji / sticker picker |
| Text input | Auto-growing textarea |
| `[🎤/➤]` | Dynamic: mic when empty, send when text present |

### 12.2 Reply Preview

A bar above the composer, part of the glass overlay. Composer height updates via `ResizeObserver`.

### 12.3 Attachment Preview

Between pick and send, a preview screen shows the processed result with thumbnail, filename, size, quality override, caption input, remove, and reorder.

### 12.4 Enter Key Behavior

| Platform | Enter | Shift+Enter |
|---|---|---|
| Desktop | Send | Newline |
| Mobile | Newline | — |
| Tablet (physical keyboard) | Send | Newline |

### 12.5 Focus Behavior

After sending, focus remains in the input. The virtual keyboard stays open on mobile. The user dismisses manually via swipe-down on the message list.

### 12.6 Multi-Attachment Layout

| Count | Layout |
|---|---|
| 1 image | Full-width |
| 2 images | Two equal columns |
| 3 images | Left spans full height; right has two stacked |
| 4 images | 2×2 grid |
| 5+ images | 2×2 grid with `+N` overlay |

Files render as stacked cards below the image grid. Caption renders last. The grid is a single message unit — long-press on any tile opens the context menu for the whole message.

---

## 13. Rooms

### 13.1 Room Creation

1. Add members (username lookup or invite link).
2. For groups (3+ members), a collapsed "Customize room" disclosure exposes name and avatar.
3. Tap **Create room**.

For 1:1 (exactly 2 members), the disclosure is not shown. The room uses the other user's display name and profile.

**Derived name** (when not customized): first two display names, then `+ N`. Cap: 2 names.

**Auto-nudge:** At 5+ members, the disclosure auto-expands with the name field focused.

**Default avatar:** `ui-profile-group` composite.

### 13.2 Member Management

- Add member via username lookup (exact match) or invite link.
- Remove member: owner (or moderator in Discord mode).
- Promote/demote: owner (Discord mode only).

### 13.3 Invite Links and QR

- Room-specific invite tokens.
- Default expiration: 7 days.
- Display: QR (via `qr-code-styling`), copy link, OS share sheet.
- Regenerate: invalidates the current link and issues a new one.
- Scanning: camera (Capacitor) or paste (desktop).

### 13.4 Username Lookup

- Exact match only.
- On no-match, a hint card explains: case-sensitive, no extra spaces, full username not display name.
- Fallback: "Invite via link instead."
- Rate-limit feedback: *"Too many search attempts. Please wait a moment and try again."*

### 13.5 Room Settings

**Display pattern:**

- **Desktop (≥1024px):** Off-canvas **right drawer** within Panel 3. Width: min(480px, 40% of Panel 3). No backdrop scrim. Dismiss: X, Escape, click outside.
- **Tablet and Mobile:** Full-screen push from the right.

**Sections:**

| Section | Permission | Notes |
|---|---|---|
| Profile (avatar, name) | Owner | Avatar stored as encrypted attachment referenced in `metadata` JSON |
| Members | All | Drill-down to member list |
| Invites | All | Create, view, revoke |
| Notifications | Per-user | All / Mentions only / Nothing; mute; sound |
| Chat Theme | Per-user | Theme, wallpaper, bubble style |
| Media & Storage | Per-user | Auto-download, storage used, clear cache |
| Retention & Limits | Owner | Days, max file size; effective limits shown read-only |
| Moderation | Owner (Discord mode) | Messenger / Discord |
| Danger Zone | Owner | Transfer ownership, delete room |
| Leave Room | All | Confirmation; warns if last member |

**Drill-down:** Tapping a section replaces the drawer's content with the section detail, with a back arrow in the header.

**Destructive actions:** Delete room and transfer ownership require type-to-confirm. Kick and leave use single-confirmation dialogs.

### 13.6 Room Metadata JSON

The server's `rooms.metadata` field holds an encrypted JSON blob:

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

Null `name` means "derive from members." Null `avatar_file_id` means "use composite." Unknown fields are ignored.

### 13.7 Room Avatar Upload

1. Tap avatar in Room Settings → Profile (owner only).
2. Source sheet: Camera / Photo library / Remove photo.
3. Circular crop (1:1 aspect ratio, drag, pinch/scroll to zoom).
4. Multi-size preview (room list, header, profile view).
5. Upload as encrypted attachment; update metadata JSON; broadcast `room.updated`.

**Constraints:** 20 MB max source, 128×128 min, JPEG/PNG/WebP/AVIF/HEIC. Output 512×512 AVIF q80.

### 13.8 Room List Ordering

**Default:** Fixed by join date. New rooms append to the bottom. Activity does NOT reorder the list.

**Manual reordering:** Activated from the room list header menu. A drag handle appears on each row. Drag to reorder. Keyboard alternative: `Alt+↑` / `Alt+↓`. "Reset to default" reverts to join date.

**Order persists** locally and syncs across devices via `room_order.sync` on `private-user-{user_id}`.

**Filters** (not sort modes):

| Filter | Behavior |
|---|---|
| Unread only | Show only rooms with unread messages |
| Mentions only | Show only rooms with unread mentions |
| Muted rooms | Show only muted rooms |

Reorder mode is disabled while filters are active.

### 13.9 Conversation Row Content

| Position | Content |
|---|---|
| Left | `ui-profile` (composite for groups) |
| Top line | Room name (bold if unread) |
| Bottom line | Last message preview, or typing indicator, or draft |
| Top right | Timestamp |
| Bottom right | Unread badge or mute icon |

**Indicators (only one at a time, prioritized):**

| Indicator | Meaning |
|---|---|
| Red `@` pill | Unread mentions in a muted room |
| Green pill with count | Unread messages |
| Green dot | Manually marked unread |
| Grey pill with count | Unread in a muted room |
| Bell-slash | Room is muted |
| Pencil | Draft exists |

**Message preview rules:**

| Last message | Preview |
|---|---|
| Text | `Sender: message text` (or `You: ...` for own) |
| Image | `Sender: 📷 Photo` |
| Video | `Sender: 📹 Video` |
| Voice | `Sender: 🎤 0:15` |
| Document | `Sender: 📎 filename.pdf` |
| Sticker | `Sender: [Sticker]` |
| Tombstone | Falls back to previous non-tombstoned message |
| System message | Falls back to previous message |

**Typing indicator** replaces the preview: `Alex is typing...`.

### 13.10 Leaving and Deletion

- **Leave room** (all members): Confirmation; warns if last member.
- **Delete room** (owner): Type-to-confirm. Other members receive `room.deleted`, their local room is removed from the conversation list, and a system message appears in the thread: "This room was deleted by [owner]."
- **Rejoining after leaving:** Fresh view from rejoin forward. Historical messages from before the leave are not re-delivered (MLS forward secrecy).

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
| Promote/demote | ✅ | ❌ | ❌ |

---

## 14. Curation Views

### 14.1 Media Grid

- 3-column flush grid.
- Grouped by chat with `section-header`.
- Tap a thumbnail → `media-viewer`.

### 14.2 Documents and Links

- List groups using `list-row`, grouped by chat.
- Documents show file-type icon, filename, size, sender, date.
- Links show favicon, title, domain, sender, date.

### 14.3 Filter and Search

**Filter chips** are hidden by default. A filter icon in the header opens a bottom sheet (mobile) or popover (desktop) with:

- Chat filter
- Date range
- Sender filter
- Media type (Photos / Videos) for Media view only

Active filters appear as dismissible pills in the header.

**Search bar** is always visible in Docs and Links; hidden behind a search icon in Media.

**Search index:** Fuse.js, lazy-loaded on first search. Indexed from the full metadata array in SQLite (not the rendered slice). Incrementally updated.

**Search fields:**

| View | Fields |
|---|---|
| Media | filename, sender, chat (date as filter) |
| Documents | filename, sender, chat |
| Links | title, domain, sender, chat |

Threshold: 0.35. `includeMatches: true` for highlighting.

### 14.4 Jump to Chat

Opens `/chat/:roomId/message/:msgId`. The client navigates, scrolls to the message, and highlights it for 2 seconds. If the message is tombstoned, a system notice appears.

### 14.5 Media Viewer

**Scoping:** The viewer receives a source list and an index. Chat-scoped and curation-scoped invocations produce different source lists.

| Opened from | Source list |
|---|---|
| Chat thread | All media in the current room, chronological |
| Curation view | The currently filtered list (crosses rooms if unfiltered) |

**Scrolling:** CSS Scroll Snap (`scroll-snap-type: x mandatory`). GPU-accelerated.

**Morph:** View Transitions API for thumbnail-to-viewer morph (progressive enhancement).

**Navigation:**

- Left/right arrows on desktop (`::scroll-button()` where supported, JS fallback).
- Swipe on mobile.
- Keyboard: `←` / `→` for slides, `+` / `-` / `0` for zoom, `Esc` to close, `Space` to play/pause video.

**Zoom and pan (images):**

- Pinch, double-tap, scroll wheel.
- Min 1×, max 5×.
- Drag to pan when zoomed; drag at 1× navigates.
- Zoom resets per slide.

**Swipe-to-dismiss:**

- Swipe down dismisses at 1×; when zoomed, swipe down pans.
- Escape (desktop) or click outside dismisses.
- Back button (mobile) dismisses.

**Action bar (top, glass chrome):**

| Action | Availability |
|---|---|
| Save | Always |
| Share | Always |
| Copy | Always |
| Jump to message | Chat-scoped: scrolls. Curation-scoped: navigates and scrolls. |
| Delete | Only if the current user uploaded the attachment |
| Info | Always |

**Thumbnail strip:** Bottom of the viewer. Virtualized (±20 items). Two-way sync with the main slideshow via `scrollIntoView` and `scrollTo`. Thumbnails are decrypted and cached locally (128×128 WebP, 3–8 KB each). Generation is lazy and runs in a Web Worker with concurrency based on device profile.

**Video:**

- `<video>` element in a slide.
- `IntersectionObserver` pauses playback when the slide scrolls out of view (mandatory — prevents simultaneous playback).
- Poster frame shows while loading.
- Auto-play respects the `Auto-download media` setting.

**Prefetching:** Active slide ±2 for decryption. Thumbnails ±20.

**Closing behavior:**

| Trigger | Effect |
|---|---|
| Swipe down | Dismiss |
| Tap outside | Dismiss |
| Escape (desktop) | Dismiss |
| Share | Opens OS share sheet; viewer stays open |
| Save | Saves; toast confirms; viewer stays open |
| Jump to message | Closes viewer; navigates |
| Delete | Confirmation; viewer moves to next item or closes |

### 14.6 Document Preview

| Category | Preview |
|---|---|
| PDF | Inline viewer via `<embed>` or `<iframe>` on a blob URL |
| Image-as-doc | Rendered as an image |
| Video-as-doc | Rendered as a video |
| Audio-as-doc | Rendered as an audio player |
| Text | Plain text render, monospace |
| Code | Syntax-highlighted via `highlight.js` |
| Office | Not previewed; file card with "Open in app" |
| Archives | Not previewed; file card with "Open in app" |
| Unknown | Not previewed; file card with "Open in app" |

**File card:**

```
┌─────────────────────────────────────────┐
│  ┌───┐  Q3-report.pdf                   │
│  │PDF│  2.4 MB · PDF                    │
│  └───┘                    [ Download ]  │
└─────────────────────────────────────────┘
```

**"Open in app":** On mobile, uses `@capacitor/filesystem` and `@capacitor/share`. On desktop, `openPath()` from `tauri-plugin-shell`. On web, a download link.

### 14.7 Link Preview Card

```
┌─────────────────────────────────────────┐
│  ┌─────┐                                │
│  │     │  The Verge                      │
│  │ IMG │  Apple announces new...         │
│  └─────┘  theverge.com                   │
└─────────────────────────────────────────┘
```

Single tappable block opening the URL in the system browser.

---

## 15. Calls (V1 UI)

### 15.1 Model

Calls are room-scoped. A 1:1 call is a 2-member room. A group call is a room with 3+ members.

### 15.2 Pre-Call Preview

```
┌─────────────────────────────────────────┐
│  ←   Call                          [✕]  │
│                                         │
│         ┌───────────────────┐           │
│         │   [Camera preview │           │
│         │    or profile]    │           │
│         └───────────────────┘           │
│                                         │
│         Alex, Sam + 2                   │
│         Design Team                     │
│                                         │
│  [🎤 Mic: On]  [📷 Camera: Off]        │
│                                         │
│  [      Start Voice Call      ]         │
│  [      Start Video Call      ]         │
└─────────────────────────────────────────┘
```

Default: voice, camera off, mic on. Background blur optional.

### 15.3 Incoming Call

- **Full-screen:** When the app is in the foreground.
- **Banner:** When in another app (iOS via CallKit, Android via full-screen intent).
- **Actions:** Decline, Accept, Message, More.

### 15.4 In-Call Screen

**Voice 1:1:** Profile-centric layout. Controls: speaker, mute, switch to video, minimize, end.

**Voice group:** Grid (up to 9 profiles) or list view with mic/video status. Active speaker highlighted with a green ring.

**Video 1:1:** Remote video full-screen; self-view as draggable PiP. Controls: speaker, mute, flip camera, minimize, end.

**Video group:** Grid (up to 6) or focus view with filmstrip. Active speaker auto-detected; tap to pin.

### 15.5 Minimized Call

Picture-in-picture floating window, draggable. On desktop, the call can dock into a resizable pane within Panel 3.

### 15.6 Call History

Calls tab in the rail (and bottom nav on mobile). Rows show profile, room name, call type, direction (incoming / outgoing / missed), duration, timestamp. Tap to re-call.

**Group calls over the participant cap:** The call starts with the first N who accept. Additional members see: "This call is full."

**Call history and disappearing messages:** Call history is not affected by the room's disappearing timer.

**Room deletion during a call:** The call ends with `reason: "room_deleted"`. Call history entries cascade away.

---

## 16. Auth

### 16.1 Auth Gate

Single page (`index.html`) with three views: Login, Register, Recovery.

### 16.2 Login View

- Username field
- Password field with show/hide
- "Log in" primary button
- "Log in with Face ID" (only if biometric enrolled)
- "Register with invite code →"
- "Lost access? Use recovery →"

### 16.3 Register View

1. Invite code field (8-char Crockford Base32, auto-formatted).
2. ALTCHA widget (invisible; fetches challenge, solves PoW).
3. Username field.
4. Display name field.
5. Password field with strength indicator.
6. "Create account" button.

After successful registration:

1. Recovery codes displayed.
2. **Download is mandatory** — the "Next" button is disabled until downloaded (or shared on mobile).
3. MLS key packages generated and uploaded.
4. Enter SPA.

### 16.4 Recovery View

- Recovery code field.
- New password field.
- "Restore access" button.

Recovery codes are the break-glass mechanism: full account recovery when password is lost or all devices are lost. They are **not** used for adding new devices.

### 16.5 Biometric Unlock

- Root of trust remains OPAQUE + password.
- Biometric gates access to the locally stored OPAQUE `RegistrationRecord`.
- Enrollment offered once after a successful password login.
- Unenrollment in Settings → Account → Security.
- Auto-invalidated on password change.

**Screen lock timeout:** Settings → Privacy → Screen lock → After immediately / 1 min / 5 min / 30 min. Default: 1 minute.

### 16.6 Device Linking

Adding a new device when an existing session is active:

1. On the existing device, generate a device linking code (QR or short code).
2. On the new device, enter or scan the code.
3. The new device performs an OPAQUE login.
4. The new device uploads its MLS key package.
5. MLS state syncs as the new device processes pending welcomes.

### 16.7 Passkeys (V2 Seam)

Not shipped in V1. The login view's action list is data-driven. The Settings → Account → Security section lists "Login methods" with Password and Biometric. Passkey is added as a third row in V2.

### 16.8 Session Expiry

| Expiry | Behavior |
|---|---|
| Auth token expired | Global API interceptor redirects to Auth Gate with message |
| OPAQUE registration session (5 min) | Inline error, form resets |
| OPAQUE login session (5 min) | Inline error, form resets |

### 16.9 Multi-Device Session Semantics

| Event | Effect |
|---|---|
| Log out on Device A | Only Device A is signed out |
| Revoke Device B from Device A | Device B is signed out remotely (via `session.revoked` event) |
| Delete account from Device A | All devices signed out, account anonymized |
| Change password from Device A | All other devices signed out |

---

## 17. Settings

A single view (`view-settings`), rendered in Panel 3 (desktop) or as a full-screen push (mobile).

A **settings search bar** at the top filters across all rows using Fuse.js.

### 17.1 Account

| Row | Control |
|---|---|
| Profile | Navigate → display name, profile image, username (read-only) |
| Password | Navigate → change password |
| Recovery codes | Navigate → status, regenerate |
| Devices | Navigate → list, revoke |
| Biometric unlock | Toggle (if supported) |
| Passkeys | Navigate → V2 placeholder |

### 17.2 Privacy

| Row | Default |
|---|---|
| Read receipts | On |
| Typing indicators | On |
| Last seen | Everyone / Rooms only / Nobody |
| Message preview in notifications | On |
| Screen lock | Off |
| Screen lock timeout | 1 minute |
| Blocked users | Navigate → list |
| Emergency wipe | Configure trigger; no confirmation when triggered |

### 17.3 Notifications

| Row | Default |
|---|---|
| Push notifications | On |
| Sound | On |
| Vibration | On |
| Per-room overrides | Navigate → list |
| Muted rooms | Navigate → list |

### 17.4 Chats

| Row | Default |
|---|---|
| Font size | Medium |
| Default chat theme | Default |
| Per-chat themes | Navigate → list |
| Enter to send | On (desktop only) |
| Voice message auto-play | Off |
| Voice playback speed | 1.0× |
| Draft messages | On |

### 17.5 Media & Storage

| Row | Default |
|---|---|
| Storage usage | Value |
| Clear cached media | Action |
| Auto-download media | Wi-Fi only |
| Auto-download documents | Never |
| Transcode quality | Balanced |
| Strip metadata | Always on (read-only) |
| Max upload size | Value from capabilities |

### 17.6 Transcription

| Row | Default |
|---|---|
| Auto-transcribe voice messages | Off |
| Show transcript below voice message | On |
| Default model | Moonshine Tiny |
| Downloaded models | Navigate → list, delete |
| Transcription language | Auto-detect |

### 17.7 Calls

| Row | Default |
|---|---|
| Default call mode | Voice |
| Call ringtone | Default |
| Video call quality | Auto |
| Show calls tab | On |

### 17.8 Appearance

| Row | Default |
|---|---|
| Theme | System |
| Accent color | TBD |
| Wallpaper | Default |
| Reduced motion | Follows OS |
| Sticker animations | First view only / Always / Never |
| Language | System |

### 17.9 About

| Row |
|---|
| Version |
| Terms of service |
| Privacy policy |
| Open source licenses |
| Log out |
| Log out and clear data |
| Delete account |

### 17.10 Administration

Rendered only if `capabilities.admin` is present.

| Row |
|---|
| Users |
| Invites |
| Rooms |
| Server status |
| Audit log |
| Server config |
| Resources (storage, users, bandwidth, rooms) |
| Upgrade (only if `capabilities.plans` present) |

### 17.11 Sign-Out and Account Deletion

**Log out:** Session token cleared, local data preserved (encrypted at rest), push subscription revoked.

**Log out and clear data:** Everything cleared, including SQLite and MLS keystore.

**Pending state handling:** Warn if there are unsent messages, drafts, in-progress uploads, or active calls.

**Account deletion:** Warning screen → type username to confirm → server processes → local data wiped → Auth Gate.

---

## 18. Notifications

### 18.1 In-App Banner

| Aspect | Decision |
|---|---|
| Position | Top of detail panel (desktop), top of screen below status bar (mobile) |
| Content | Profile, room name, preview line, timestamp, dismiss |
| Preview | Respects the Privacy setting |
| Actions | Tap to navigate; swipe up or X to dismiss; auto-dismiss after 5s |
| Suppression | Same room, muted, backgrounded, in-call, full-screen viewer, own messages |
| Mute exception | @mentions override mute |
| Stacking | One at a time; newest replaces; no queue |
| Platform | Custom in-app banner on all platforms (privacy-first; no OS notification database storage) |
| No inline reply | The banner is a nudge, not a composer |

### 18.2 Push Notifications

**Case matrix:**

| App state | WebSocket | Mechanism |
|---|---|---|
| Foreground, same room open | Connected | No notification |
| Foreground, different room | Connected | In-app banner |
| Foreground, in call | Connected | In-app banner (reduced prominence) |
| Backgrounded (< 30s) | Connected | No push yet |
| Backgrounded (> 30s) | Disconnected | Push notification |
| Lock screen | Disconnected | Push notification |
| App killed | Disconnected | Push notification |

**Permission flow:**

1. **Pre-prompt:** In-app soft prompt appears when: 24h + first message sent + first message received. One-shot; after two declines, never shown again.
2. **OS prompt:** Triggered by "Enable notifications."
3. **iOS:** Provisional notifications adopted — no OS prompt initially; user promotes via "Keep" on a notification.
4. **Post-denial:** No re-prompt. Settings shows state with "Open Settings."
5. **Token re-registration:** On launch, on push failure, on permission change.

**Push actions:**

| Action | iOS | Android | Web |
|---|---|---|---|
| Reply | ✅ inline text | ✅ inline text | ❌ |
| Mark as read | ✅ | ✅ | ✅ |
| Tap | ✅ deep link | ✅ deep link | ✅ deep link |

**No Mute action** — muting requires a duration; belongs in the app.

**Reply architecture:**

1. NSE (iOS) / FirebaseMessagingService (Android) decrypts the message using locally cached MLS state.
2. Notification displayed with a Reply action.
3. User taps Reply, types, taps Send.
4. OS launches the app in the background.
5. App encrypts with MLS, POSTs to `/rooms/:id/messages`, updates local state.
6. Notification dismissed.

**Screen lock interaction:** If Screen lock is on, Reply prompts biometric first. The reply is pre-scoped to the room.

**Notification categories (iOS) / channels (Android):**

| Category | Actions |
|---|---|
| `MESSAGE` | Reply, Mark as read |
| `MESSAGE_MUTED` | Reply |
| `MENTION` | Reply, Mark as read |
| `CALL` | Accept, Decline |
| `MEMBER_CHANGE` | (none) |

**DND:** Respected. Calls break through via critical channel / full-screen intent.

**Web Push:** Mark as read and Tap only. No inline reply.

---

## 19. Cross-Platform

### 19.1 Capacitor (iOS, Android)

- Push via `@capacitor/push-notifications`
- Secure storage via `@capacitor/preferences` + platform keychain
- File picking via `@capacitor/camera` / `@capacitor/filesystem`
- Status bar via `capacitor-edge-to-edge`
- Biometric via `@capacitor/biometrics`
- QR scanning via a camera-based scanner plugin

### 19.2 Tauri (Desktop)

- Push via `tauri-plugin-notification` (local notifications; remote push may not ship in V1)
- Secure storage via `tauri-plugin-stronghold` or OS keychain
- Status bar and safe areas via `tauri-plugin-safe-area-insets-css`
- Biometric via `tauri-plugin-biometric`
- Deep link via `tauri-plugin-deep-link`

### 19.3 Shared CSS

Both platforms expose the same CSS variables:

```css
:root {
  --safe-area-inset-top: env(safe-area-inset-top, 0px);
  --safe-area-inset-bottom: env(safe-area-inset-bottom, 0px);
}
```

Components don't branch on platform.

### 19.4 Privacy Screen

When the app backgrounds, the OS captures a snapshot for the app switcher. This snapshot is blurred on all platforms.

| Platform | Mechanism |
|---|---|
| iOS | Blur overlay in `applicationWillResignActive` |
| Android 13+ | `setRecentsScreenshotEnabled(false)` |
| Android 12 and below | Not blocked (would require `FLAG_SECURE` which also blocks screenshots) |

User screenshots remain allowed everywhere.

### 19.5 Deep Linking

**Route table:** Same as SPA routes (§4.2).

**Pending deep link slot:** In-memory only; cleared after processing. Invite tokens use `sessionStorage` for auth-flow survival.

**Processing order:** Boot → auth → sync start → route → clear.

**Push notification mapping:** `type` → route.

**Platform configuration:**

| Platform | Universal links | Custom scheme |
|---|---|---|
| iOS | `apple-app-site-association`, Associated Domains | `CFBundleURLTypes` |
| Android | `assetlinks.json`, `intent-filter` | `intent-filter` |
| Tauri | `Info.plist` (macOS), registry (Windows) | `tauri.conf.json` schemes |
| Web | Routes only | — |

**Edge cases:**

| Scenario | Behavior |
|---|---|
| Link to a room the user isn't a member of | Error: "You're not a member of this room" |
| Link to a tombstoned message | Navigate and show the tombstone |
| Link to a deleted room | Error: "This room no longer exists" |
| Link with an expired invite token | The join flow shows "This invite link has expired" |
| Two deep links in rapid succession | Second replaces first |
| Deep link while a call is active | Call overlay takes priority; route deferred |
| Deep link during P2P transfer | Transfer continues; link navigates |

---

## 20. Empty States

Three variants: **empty** (no data), **no results** (filter match fails), **error** (load failed).

| Surface | Variant | Copy | Actions |
|---|---|---|---|
| Room list (new account) | empty | "No chats yet. Create a room or join with an invite link to start talking." | Create room / Join with invite |
| Room list (search) | no-results | "No results for 'query'" | Clear search |
| Room list (error) | error | "Couldn't load your chats" | Retry |
| Chat thread (new room) | empty | "This is the beginning of the room. Send a message to say hello." | — |
| Chat thread (MLS pending) | empty | "Setting up encryption…" | — |
| Media | empty | "No media yet. Photos and videos shared in your chats will appear here." | — |
| Documents | empty | "No documents yet." | — |
| Links | empty | "No links yet." | — |
| Curation (filtered) | no-results | "No [media/docs/links] match your filters." | Clear filters |
| Curation (error) | error | "Couldn't load [media/docs/links]" | Retry |
| Calls | empty | "No calls yet. Call a room and your history will appear here." | — |
| Blocked users | empty | "No blocked users" | — |
| Downloaded models | empty | "No models downloaded" | — |
| Custom stickers | empty | "No custom stickers yet" | Create sticker |
| In-thread search | empty | "Nothing to search yet" | — |

**Rules:**

- Shared-rooms section is **hidden entirely** when empty, not shown as an empty state.
- One primary action per state; secondary actions only when necessary.
- No tutorials, no progress indicators, no illustrations.

---

## 21. Error Handling

### 21.1 Network Layer

- **Detection:** WebSocket connection state is authoritative.
- **No global banner.** Inline message states, one-shot toasts.
- **Retry strategy:**

| Operation | Strategy |
|---|---|
| WebSocket reconnect | Exponential backoff: 1s, 2s, 4s, 8s, 16s, 30s cap |
| Failed message send | 3 attempts: 2s, 5s, 15s; then mark failed |
| Attachment upload | 2 attempts: 5s; then manual retry |
| API GET | 1 retry; then error state |
| API POST/PUT/DELETE | No auto-retry (idempotency not guaranteed) |

### 21.2 Auth Layer

| Error | UI |
|---|---|
| 401 | Redirect to Auth Gate; preserve route in pending deep link |
| `session.revoked` | Clear session and local data; toast: "This device was signed out from another device" |
| `account.disabled` | Clear session; navigate to Auth Gate; message: "Your account has been disabled" |
| `account.deleted` | Same with different copy |

### 21.3 Server Errors

| Code | UI |
|---|---|
| 400 | "Something went wrong. Please try again." |
| 403 | "You don't have permission to do that." |
| 404 | Context-specific |
| 409 | Context-specific ("Username taken", etc.) |
| 413 | "This file is too large." |
| 415 | "This file type isn't supported." |
| 429 | Toast with countdown; action disabled until reset |
| 5xx | "The server had a problem. Try again." |

### 21.4 Crypto Errors

| Error | Behavior |
|---|---|
| MLS decrypt failure | Mark "cannot decrypt"; tap for details |
| Key commitment mismatch | Reject decryption; warning: "This attachment couldn't be verified" |
| Safety number changed | Warning banner in shared rooms; "Verify" button |
| Key transparency failure | Contact marked unverified; retry option |

### 21.5 Media Errors

| Error | Behavior |
|---|---|
| Transcode failure | Passthrough as generic file; subtle notice |
| Upload failure | Auto-retry; then "Tap to retry" |
| Download failure (corrupt) | "This file couldn't be downloaded" |
| Presigned URL expired | Fetch new URL; resume |

### 21.6 Storage Errors

| Error | Behavior |
|---|---|
| SQLite quota exceeded | Warnings at 80%, blocks new media at 95% |
| SQLite write failure | Retry; fall back to in-memory mode with banner |
| OPFS unavailable (web) | Fall back to IndexedDB VFS, then in-memory |

### 21.7 MLS Errors

| Error | Behavior |
|---|---|
| Out-of-sync epoch | "Syncing encryption state…" banner |
| Missing welcome | Poll every 5s; after 5min show "Waiting for a room member to add you" |
| Pending MLS remove | Room marked "left"; toast: "You were removed from [Room]" |
| Keystore corruption | Wipe keystore; mark rooms `mls_status='error'`; offer rejoin |

### 21.8 Interaction Edge Cases

| Scenario | Behavior |
|---|---|
| Edit a message, then unsend | Tombstone replaces the latest version |
| Unsend a message, then attempt to edit | Edit option absent |
| Disappearing message edited | Edit inherits the original's `expires_at` |
| Reply to a message that expires | Quoted snippet shows `[message expired]` after expiration |
| Message arrives during MLS epoch transition | Composer shows "Securing…"; send queued |
| Same file shared in two rooms | Stored separately (encryption context binds to room_id) |

---

## 22. Plugin Inventory

| Plugin | Options | Purpose |
|---|---|---|
| `state-plugin` | `initialState` | Global keyed pub/sub store |
| `socket-plugin` | `socketUrl`, `apiBase`, `appKey`, `channelPrefix` | WebSocket connection and subscriptions |
| `crypto-plugin` | `wasmPath`, key package thresholds | MLS wrapper |
| `media-plugin` | `presets`, `workerPoolSize`, `enableFFmpegFallback` | Transcode, thumbnail, strip metadata |
| `transcription-plugin` | `models`, `modelBaseUrl`, `defaultModel` | Local speech-to-text |
| `push-plugin` | `vapidKey`, `apnsConfig`, `fcmConfig` | Push registration and handling |
| `storage-plugin` | `dbName`, `blobRoot` | SQLite + blob filesystem |
| `i18n-plugin` | `locales`, `defaultLocale` | Translation and locale formatting |
| `theme-plugin` | `presets`, `accent` | Theme application and per-room overrides |
| `call-plugin` | `iceServers`, `transport` | Call state machine and media |
| `p2p-plugin` | `iceServers`, `chunkSize` | WebRTC data channel transfers |
| `icon-plugin` | `styleMap`, `registry` | Solar icon resolution |
| `notification-plugin` | `defaultSound`, `vibration` | In-app banners, sounds, haptics |
| `deep-link-plugin` | `schemes`, `universalLinkHost` | URL routing and pending deep links |
| `search-plugin` | `threshold`, `debounceMs` | Fuse.js lifecycle and indexing |

Plugin contracts (factory signature, options, `context()` return shape) are documented per-plugin in the codebase. This table is the index.

---

## 23. Appendices

### 23.1 SQLite Schema

Defined in §23.1.1 through §23.1.23. Full schema documented in the codebase migrations. Summary of tables:

| Domain | Tables |
|---|---|
| Meta | `_migrations`, `_meta` |
| Identity | `users` |
| Rooms | `rooms`, `room_members`, `room_order` |
| Messages | `messages`, `message_versions`, `reactions` |
| Attachments | `attachments` |
| User state | `read_state`, `drafts`, `outbox`, `blocked_users`, `room_preferences` |
| Personalization | `wallpapers`, `sticker_packs`, `custom_stickers`, `blocked_sticker_packs` |
| Calls | `calls`, `call_participants` |
| Sync | `sync_state`, `processed_events` |
| MLS | `mls_rooms` |
| Settings | `settings` |

Full DDL is defined in the codebase migrations. Key principles:

- All IDs are TEXT. Client temp IDs are ULIDs.
- Timestamps are INTEGER milliseconds since Unix epoch.
- Encrypted payloads are BLOB.
- Foreign keys enforced; WAL mode.
- Forward-only migrations tracked in `_migrations`.

### 23.2 Design Tokens

Full token list in `src/styles/tokens.css`. Structure:

- **Primitives:** neutral scale, accent scale, semantic scales (success, warning, danger, info)
- **Spacing:** 4px base unit, 13 steps
- **Typography:** font families (system stack), sizes, weights, line heights, letter spacing
- **Radii:** 8 steps
- **Shadows:** 6 elevation levels
- **Motion:** durations and easings
- **Z-index:** 8 layers
- **Layout metrics:** header, composer, rail, panel widths
- **Safe areas:** 4 insets

Dark mode is a semantic-layer swap under `[data-theme="dark"]`.

Per-chat theming overrides a small subset of tokens on the chat view container.

### 23.3 Event Catalog (Server → Client)

| Event | Payload | Handler |
|---|---|---|
| `message.new` | `{ id, room_id, sender_user_id, sender_client_id, epoch, seq, content_type, created_at }` | Decrypt, append to thread |
| `message.deleted` | `{ id, room_id }` | Tombstone locally |
| `room.updated` | `{ room_id, metadata?, retention_days?, max_file_size_bytes? }` | Update room metadata |
| `room.member_added` | `{ room_id, user_id, role, joined_at }` | Update member list |
| `room.member_removed` | `{ room_id, user_id }` | Update member list |
| `epoch.updated` | `{ room_id, epoch, sequence }` | Trigger MLS epoch sync |
| `call.*` | Varies | Call state machine |
| `session.revoked` | `{ session_id, reason }` | Clear session, navigate to Auth Gate |
| `device.added` | `{ device_id, name, platform, added_at }` | Update device list |
| `device.revoked` | `{ device_id, reason }` | Update device list |
| `read.sync` | `{ room_id, last_read_message_id, timestamp }` | Update read state |
| `room_order.sync` | `{ order: string[] }` | Update room order |

### 23.4 Client Events (Client → Client via Sockudo)

| Event | Payload | Purpose |
|---|---|---|
| `client-typing.start` | `{ user_id }` | Typing indicator active |
| `client-typing.stop` | `{ user_id }` | Typing indicator inactive |
| `client-read` | `{ user_id, message_id }` | Read receipt |
| `client-mls-request` | `{ room_id }` | Request pending MLS add |

### 23.5 Error Copy Reference

Consolidated table of user-facing error strings, grouped by layer (network, auth, server, crypto, media, storage, MLS). Referenced in §21.

### 23.6 Server Spec Assumptions

This spec is written against Server Specification v1.0 (as amended). The following are **client-required server additions** awaiting the server team's next build phase:

| Assumption | Status |
|---|---|
| `private-user-{user_id}` channel (V1 required, not V2) | Pending server amendment |
| `read.sync` and `room_order.sync` events | Pending server amendment |
| `pending_mls_adds` coordination | Pending server amendment |
| `PATCH /rooms/:id` with `metadata` | Pending server amendment |
| Key transparency log + auditor signatures | Pending server amendment |
| Link preview proxy (opt-in) | Pending server amendment |
| Message editing endpoint | Pending server amendment |
| Reactions table + endpoints | Pending server amendment |
| `POST /users/me/avatar` | Pending server amendment |
| Call signaling events + TURN credentials | Pending server amendment |
| Member list pagination | Pending server amendment |
| Retention change preview | Pending server amendment |

For each pending item, the client implements against the assumed contract. If the server team's implementation differs, the client adjusts in a coordinated update.

---

## 24. Amendments

| # | Date | Change |
|---|---|---|
| — | — | Initial draft |

---

## 25. Document Status

This is the contract for the client side of the system. Every client implementation task references this document. If a task conflicts with this spec, the task is wrong and must be revised.

Amendments are tracked in §24.

---

**End of Client Specification v1.0.**
