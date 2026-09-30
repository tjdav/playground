Client Specification v1.0

Status: Final — source of truth for client implementation tasks.
Server basis: Server Specification v2.0 (including Amendments 25–29).
Stack: Coralite 1.0.0-rc.5 (static site generator) · Wire CoreCrypto 10.5.2 · Mediabunny · Supertonic · Moonshine / Whisper · Capacitor · Tauri
Design language: Privacy-first messenger. Adopts proven social-interaction patterns; visual density follows modern Western conventions. Not a clone of any single product.

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
Username lookup OPRF (client-side blinding)
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

---

2. Scope

2.1 In Scope

· Two static pages: index.html (auth gate), app.html (messenger shell)
· Three-panel desktop layout; stacked mobile layout; tablet two-panel layout
· Room-based conversation model (1:1 = 2-member room; no contact list)
· OPRF-based username lookup (exact-match only)
· Text messaging with Markdown formatting
· Message editing (15-minute window, signed edit chain)
· Reactions (silent — no notification, no read receipt, no unread badge)
· @mentions with autocomplete and mute override
· Disappearing messages (24h / 7d / 90d)
· Read receipts (aggregate count, mutual setting, synced across devices)
· Typing indicators (via client events)
· Message deletion (tombstone via server endpoint)
· Encrypted attachments (C2SP chunked AES-256-GCM)
· Range-request streaming and seeking
· Presigned URL handling (S3 backends)
· P2P fallback for files exceeding room limits (WebRTC data channels)
· Media processing pipeline (image, video, audio)
· Client-side poster and thumbnail generation
· Metadata stripping for all processed media
· Local voice message transcription (Moonshine, Whisper)
· Read-aloud (TTS) — Supertonic-based local speech synthesis with sentence-level control
· Voice messages with interactive waveform player
· Stickers (bundled + custom + combined) with entrance transitions
· Emoji picker
· Client-generated link previews (with optional server proxy)
· In-thread search (Fuse.js, per-room index)
· Global message search
· Media, Documents, Links curation views (filter + search)
· Room creation, member management, invite links + QR
· Room settings (off-canvas right drawer on desktop; modal on mobile)
· Per-room, per-user theming (theme, wallpaper, bubble style)
· Manual room ordering (drag-and-drop, synced across devices)
· Per-user nicknames — local aliases for room participants; synced across devices
· Starred items — media, messages, and links marked by the user; synced across devices
· Biometric unlock (Face ID, Touch ID, Windows Hello)
· Recovery code flow (server-generated, server-hashed)
· Device linking for new devices
· Multi-device read state sync
· Key transparency automatic verification
· Hangouts — persistent room-scoped voice spaces
· Calls UI (pre-call preview, in-call screen, history, PiP)
· Push notifications (Web Push, APNs, FCM, UnifiedPush)
· Notification actions (reply, mark as read)
· Offline-first: readable cache, outgoing message queue
· Boot, sync, and delta sync
· Blocked users management
· Privacy screen (app switcher blur)
· App lock timeout
· Deep linking (universal links, custom scheme, push)
· Accessibility (WCAG 2.1 AA)
· Internationalization infrastructure (English)

2.2 Server Requirements (Client V1 Dependencies)

The following are provided by Server Specification v2.0 and its amendments:

· OPRF-based username lookup
· Encrypted display names (encrypted_display)
· private-user-{user_id} channel with user_seq
· GET /users/me/sync?since_seq=X
· REST writes for read state and room order
· Device names via user-scoped sync
· pending_mls_adds coordination
· Room metadata JSON blob via PATCH /rooms/:id
· Key transparency log + auditor signatures
· Link preview proxy (opt-in)
· Message editing, reactions, threading
· POST /users/me/avatar
· Call signaling events + TURN credentials
· Member list pagination, retention preview, invite metadata preview, transfer consent
· Separate recovery flow (POST /auth/recover/start + /finish)
· Schema naming cleanup
· Hangouts (server Amendment 25)
· Model hosting for STT and TTS (server Amendment 26)
· starred_items table + sync (server Amendment 27)
· Generic user preferences endpoint (server Amendment 28)
· Hangout clarifications (server Amendment 29, C1–C5)

2.3 Out of Scope

· SPA routing between pages (each page is a static HTML document)
· Contact/friend list (rooms only)
· Server-side key escrow
· Plaintext export of message contents
· Username changes
· Federation between servers
· Multi-tenancy
· Multiple accounts on one device
· Analytics or crash reporting
· Message forwarding
· Message pinning
· Chat screenshot export
· GIF search (external API)
· User status / away message
· Ambient presence — hangouts are the only co-presence feature
· Office document inline rendering
· Native keyboard sticker integration (platform limitation; watchlisted)

---

3. Design Principles

1. Clear primary task — chat first. Every view serves conversation.
2. Privacy-first — the server holds no plaintext user identifiers, messages, or room metadata.
3. Behavior over ornament — animation and decoration serve function.
4. Consistency through position — navigation and actions appear in predictable places.
5. Asymmetric cultural adoption — adopt proven social-interaction patterns; keep visual density closer to modern Western conventions.
6. Honest defaults — no fake previews, no unverifiable claims.
7. Ask when needed — permissions and prompts appear at the moment of relevance.
8. Accessibility as a first-class mode — read-aloud, high contrast, and reduced motion are not buried settings; they shape the design of every surface.

---

4. Architecture

4.1 Pages

Two static pages built by Coralite:

Page File Purpose
Auth Gate src/pages/index.html Login, registration, recovery.
Messenger src/pages/app.html Messenger shell. In-page navigation only.

Navigation from the auth gate to the messenger is a full page load (window.location.href = '/app.html'). No client-side routing between pages.

Each page is a plain HTML document that uses components via their <template id> tags.

4.2 In-Page Routing

Within app.html, a router plugin manages view state via query parameters.

URL format:

```
/app.html?view=chat&id=r_abc123
/app.html?view=chat&id=r_abc123&messageId=m_xyz789
/app.html?view=media
/app.html?view=link&id=<link_id>
/app.html?view=document&id=<doc_id>
/app.html?view=hangout&id=<hangout_id>
/app.html?view=settings
/app.html?view=profile&id=u_abc123
```

View mapping:

view param Renders
chats view-chats
chat view-chat
media view-media
link view-link (detail)
links view-links (list)
document view-document (detail)
documents view-documents (list)
hangout view-hangout
calls view-calls
settings view-settings
profile view-profile
join view-join-room

State keys synced to the URL:

Key Values
currentAppView The view names above
activeSelectionId Room ID, user ID, message ID (context-dependent)
activeSelectionType room, user, message

Entry point: The SPA entry component calls router.init() in its client(). The router plugin handles the popstate event, parses query parameters on load, and syncs state changes back to the URL.

Deep links (from push notifications, invite links) are query-param URLs:

```
https://app.example.com/app.html?view=chat&id=r_abc123&messageId=m_xyz789
```

4.3 Directory Structure

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
│   │   ├── primitives/       # ui-*
│   │   ├── composed/         # profiles, bubbles, rows, cards
│   │   ├── containers/       # list/thread composers
│   │   ├── views/            # route views
│   │   └── shell/            # messenger-shell, hosts
│   ├── plugins/              # factory → definePlugin
│   ├── lib/
│   │   ├── mls/              # CoreCrypto wrapper
│   │   ├── crypto/           # C2SP, HKDF, Web Crypto
│   │   ├── oprf/             # OPRF blinding
│   │   ├── media/            # Mediabunny pipeline
│   │   ├── transcription/    # STT model loading + inference
│   │   ├── speech/           # TTS model loading + synthesis
│   │   ├── hangouts/         # Hangout metadata + occupancy
│   │   ├── api/              # Fetch wrapper
│   │   ├── p2p/              # WebRTC data channels
│   │   ├── db/               # SQLite repositories
│   │   ├── blobs/            # Blob filesystem
│   │   ├── actions/          # Orchestration functions
│   │   ├── events/           # Socket event handlers
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

Component file naming: Each component is a single .html file. Its tag name comes from the <template id="..."> inside the file, not the filename. Components can be nested in folders. No prefix is applied.

4.4 Plugin Architecture

Plugins are factory functions returning definePlugin results. Values from coralite.config.js flow through the factory closure.

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

Config loading:

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
    transcriptionPlugin({ modelBaseUrl: process.env.STT_MODEL_BASE_URL }),
    speechPlugin({ modelBaseUrl: process.env.TTS_MODEL_BASE_URL }),
    themePlugin({ defaultTheme: 'auto' }),
    notificationPlugin({}),
    callPlugin({}),
    hangoutPlugin({}),
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

Two-phase resolver:

· Phase 1 — (pluginContext) => .... Runs once per plugin per app load. Expensive setup. Can be async.
· Phase 2 — (instanceContext) => ({ ... }). Runs per component instance. Cheap setup.

Plugin name → context key. A plugin named 'router' becomes client({ router }).

Cross-plugin access. Plugins access each other via instanceContext.<name>.

Global-singleton guard. Expensive one-time work uses pluginContext.__<name>_active__ for idempotency.

Asset pipeline. Assets (WASM, sticker packs, wallpapers, sounds, models) are registered in the assets array with either { pkg, path, dest } or { src, dest } form. Coralite tracks changes and rebuilds only modified assets, keeping the manifest in sync.

4.5 State Management

Global state is managed by the state plugin. Keyed pub/sub store.

Consumption:

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

State keys:

Key Shape
currentUser User \| null
isAuthenticated boolean
oprfToken Uint8Array \| null (in memory only)
capabilities Capabilities
rooms Record<roomId, Room>
roomOrder string[]
selectedRoomId string \| null
messages:{roomId} Message[]
members:{roomId} Member[]
nicknames:{roomId} Record<userId, string>
typing:{roomId} TypingUser[]
hangouts:{roomId} Hangout[]
activeHangout Hangout \| null
hangoutRoster RosterEntry[] \| null (in-memory only)
outbox OutboxEntry[]
unreadCounts Record<roomId, number>
starredItems Set<itemId>
activeCall Call \| null
callHistory Call[]
readAloudMode boolean
readAloudState { playing, messageId, sentenceIndex }
settings Record<string, unknown>
ui { activeRail, modal, toasts, offline }
pendingScrollToMessage string \| null

Source of truth: SQLite. The store is an in-memory view.

4.6 Cross-Component Communication

Relationship Mechanism
Child → parent emit(name, detail) — DOM-scoped CustomEvent, bubbles to ancestor
Sibling / cross-tree $state key change
Global signal $state key observed by target

No global event bus.

Example — jump to message from search:

```js
// search view sets the key
$state.pendingScrollToMessage = 'm_xyz789'
```

```js
// message-thread subscribes
$state.subscribe('pendingScrollToMessage', (msgId) => {
  if (msgId) scrollToMessage(msgId)
})
```

4.7 Boot Sequence

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
   ├─ Cached display names, device names, nicknames
   ├─ Hangouts per room
   ├─ Starred items
   ├─ Cached read-aloud settings
   ├─ Last-viewed room's recent messages
   └─ Sync cursors (per-room + user-scoped)
9. User-scoped state sync:
   GET /users/me/sync?since_seq=<last_user_seq>
   Apply read_state, user_preferences, device_state, starred_items
   Store new max_seq
10. CoreCrypto initialization
11. OPRF token derivation (requires username from secure storage)
12. Render UI with hydrated data
13. Fetch capabilities (GET /api/v1/capabilities)
14. Connect WebSocket (pusher-js)
15. Subscribe to private-user-{user_id} and all private-room-{room_id}
16. Delta sync per room (since cursor)
17. Background: key package replenishment, welcome polling,
    pending removes, thumbnail generation, TTS/STT model idle prefetch
```

Failure modes:

Step Failure Behavior
6 Corrupt token Clear storage, redirect
7–8 SQLite unavailable Degradation notice; memory-only mode
9 Sync fetch fails Retry; app works from local state
10 Keystore corrupted Wipe, re-init; rooms marked mls_status='error'
11 OPRF fails Retry; app works without display-name decryption until it succeeds
14 Socket fails Retry with backoff; app remains usable from cache

4.8 Sync & Offline

Two cursors:

· Per-room (epoch, seq) for message sync. Room events are best-effort WebSocket; REST delta sync is authoritative.
· Per-user last_user_seq for user-scoped state. User events are durable via user_seq; catch-up via GET /users/me/sync.

Readable offline: All cached messages, media, metadata, nicknames, starred items.

Outbox queue: Messages composed while offline persist in SQLite. On reconnect, processed FIFO. On failure after retries, marked failed with Retry/Delete actions.

Read state while offline: Advances locally; debounced flush to POST /users/me/read-state (at most one POST per room per 30 seconds, plus immediate flush on room switch and app background).

No global offline banner. Feedback is inline on messages and via one-shot toasts.

4.9 Component Authoring Rules

Coralite components follow strict rules:

Rule Implication
Flat template tokens only No expressions, function calls, dot notation, or conditionals in <template>. All logic moves to getters.
No inline event handlers Bind events in client() via refs('name').addEventListener(..., { signal }).
No array/object attributes Collections come via plugin context or slots.
Getters destructure context ({ state, root, refs, slots, signal }) => ...
Style getters receive state directly (state) => value, synchronous only.
server() is build-time only Stripped from browser bundles.
client() has a serialization boundary No module-scope imports. Use await import() inside.
emit instead of dispatchEvent Crosses custom element boundaries.
Never mutate state in observe() Use event handlers or getters.

---

5. Design System

5.1 Naming Conventions

Term Component
Profile ui-profile
Chat view-chat
Room room-*
Thread message-thread
Hangout hangout-*

5.2 CSS Strategy

Layer Tool Scope
Design tokens CSS custom properties Global
Global utilities Tailwind Entire app
Component styles Coralite scoped <style> Per-component

Tailwind provides layout utilities; components own structure and variant styling. Coralite scopes styles via compiler-injected data-style-selector attributes, allowing Tailwind utilities inside components.

5.3 Theming

Global theme: light / dark / auto. Token swap under [data-theme="dark"].

Accent color: TBD. Placeholder blue (#3B82F6) used until brand decision. All components reference var(--accent-*).

Per-room, per-user theming: Room container overrides a small subset of tokens. Stored in room_preferences (local), synced via user preferences.

Variant pattern:

```html
<ui-profile size="md" shape="circle" state="online"></ui-profile>
<ui-button variant="primary" size="md" disabled></ui-button>
```

5.4 Icons

Solar icon set, tree-shaken via @solar-icons/static. No sprite.

The icon-plugin resolves a canonical name and a state to a concrete icon:

```html
<ui-icon name="chat" state="active"></ui-icon>
```

The plugin maps states to Solar styles (linear → default, bold → active, bold-duotone → emphasis). The mapping is overridable per-instance with a style attribute. Canonical names are library-agnostic.

No emoji are used as icons.

5.5 Motion

Motion uses tokens from the design system. All motion respects prefers-reduced-motion: reduce.

---

6. Layout

6.1 Three-Panel Desktop (≥ 1024px)

```
┌──────┬──────────────┬──────────────────────────────┐
│ Rail │ List Panel   │ Detail Panel                 │
│ 64px │ 320–400px    │ flex                         │
└──────┴──────────────┴──────────────────────────────┘
```

Explorer pattern. Selecting an item in Panel 2 highlights it and renders detail in Panel 3.

Rail items: Chat, Media, Documents, Links, Calls, Settings.

6.2 Tablet (768–1023px)

Rail collapses to bottom nav. Panels 2 and 3 remain side by side.

6.3 Mobile (< 768px)

Single column. Bottom nav: Chat, Media, Calls, Profile.

Media tab hosts a segmented control: Media / Docs / Links.

The Music tab is not present. Voice messages are not curated. Shared audio files appear in Docs.

6.4 Floating Glass Chrome

Chat header and composer are absolute overlays with backdrop-filter: blur(20px) saturate(180%). The thread scrolls underneath.

Dynamic offsets. Composer height tracked via ResizeObserver. Header includes env(safe-area-inset-top); composer includes env(safe-area-inset-bottom).

Scroll edge cue. A data-scrolled attribute adds a border when content is behind the glass.

Fallback. @supports not (backdrop-filter: blur(20px)) and a low-end device check disable the glass effect.

6.5 Safe Areas

Edge-to-edge rendering. Safe area insets provided by:

· Capacitor: capacitor-edge-to-edge or @aashu-dubey/capacitor-statusbar-safe-area
· Tauri: tauri-plugin-safe-area-insets-css
· Web: env(safe-area-inset-*)

Both plugins expose consistent CSS variables. Root viewport height uses 100dvh.

6.6 Navigation Patterns

Affordance Purpose
Back arrow (←) Nested navigation (mobile)
Close (✕) Modal dismissal
OK / Cancel Forced decisions

Platform split:

· Desktop: Explorer pattern. Media, link, and document detail render in Panel 3. Room settings open as an off-canvas drawer.
· Mobile: Full-screen stack. Back arrows for drill-downs. Room settings uses X (modal, not nested destination).

Action placement:

Position Reserved for
Top-left Navigation (back arrow or X)
Top-center Title
Top-right Contextual actions (overflow menu, primary action)

List-detail navigation. Detail views opened from a list support horizontal navigation through the list:

· Mobile: horizontal swipe with scroll-snap. No visible arrows. A position indicator (12 of 47) appears in the header.
· Desktop: arrows appear on hover at the left and right edges. Keyboard ←/→ navigates. Trackpad swipe works via the same scroll-snap container.
· Gesture direction locking (8–10px threshold) prevents conflict with vertical scrolling inside the item.
· Boundary behavior: bounce (mobile) / faded arrow (desktop).

Applies to: media viewer, link detail, document detail, hangout list.

6.7 Header Consolidation

The video icon is not present in the room header. Video is selected from the pre-call preview screen.

Platform Header layout
Mobile [←] [name] [phone] [headphones] [gear]
Desktop [name] [headphones] [search] [phone] [⋯]

Hangout icon: headphones glyph (Solar headphones). Not a megaphone.

State Treatment
No hangouts No badge
Hangouts exist, none active Subtle dot
Hangouts active Count badge (total participants)

Room settings on desktop: moves into [⋯] overflow. On mobile: stays as gear icon.

---

7. Component Inventory

Component tags correspond to <template id="..."> values inside .html files. No array or object attributes are used — collections come via plugin context or slots.

7.1 Primitives

Component Responsibility Key attributes
ui-profile User/group profile size, shape, src, state, fallback
ui-profile-group Composite of 2–4 profiles max, size
ui-badge Unread count or dot count, variant
ui-button Pill button variant, size, disabled, loading
ui-input Text field type, label, error, disabled
ui-textarea Auto-growing textarea rows, maxlength
ui-switch Toggle checked, disabled
ui-checkbox Checkbox checked, indeterminate
ui-radio Radio checked, name, value
ui-spinner Loading indicator size
ui-skeleton Loading placeholder variant
ui-divider Rule orientation
ui-chip Tag / filter / reaction selected, removable
ui-timestamp Time display datetime, format
ui-icon Solar icon wrapper name, state, size, color
ui-sheet Bottom sheet / dialog variant, open
ui-toast Transient message variant, duration
ui-tooltip Hover/focus hint placement
ui-disclosure Expandable section label, open
ui-context-menu Trigger-adaptive menu trigger

7.2 Composed

Component Responsibility
chat-bubble Message container
chat-bubble-group Consecutive messages
message-text Markdown body
message-attachment Attachment preview
message-reaction Reaction chip
message-meta Time + status
message-waveform Voice player
message-audio-player Audio player with transcribe + read-aloud affordances
message-transcript Transcript display
message-tombstone Deleted placeholder
typing-dots Typing indicator
list-row Generic list row
settings-row Icon-circle settings row
conversation-row Conversation list item
media-thumbnail Grid thumbnail (with inline video preview)
link-card Link preview
document-row File list item
file-icon File type icon
nav-item Rail or bottom-nav item
form-field Label + control + hint
search-field Search input
section-header Group label
empty-state Empty state
safety-number Safety number
call-participant Call participant tile
hangout-card Hangout entry in a list
hangout-strip Active hangout bar above thread
hangout-bar In-hangout control bar
hangout-participant Participant in roster

7.3 Containers

Component Responsibility
conversation-list Virtualized list (reads $state.rooms)
message-thread Virtualized message list
message-composer Composer with read-aloud toggle
media-grid 3-column grid, photos + videos combined, date-grouped
document-list Virtualized list group
link-list Virtualized list group
settings-list Settings rows
auth-form Auth gate form
media-viewer Full-screen slideshow with list-detail navigation
link-detail Link detail view
document-detail Document detail view
hangout-list Hangouts in a room
hangout-roster Participants in a hangout
hangout-create Create/edit hangout sheet
hangout-signal-handler WebRTC signaling client
call-participant-grid Call layout
message-reply-preview Reply bar
sticker-picker Sticker sheet
read-aloud-controls Playback controls for TTS

7.4 Shell

Component Responsibility
messenger-shell Root layout
modal-host Overlay renderer
toast-host Toast queue
call-overlay-host Call overlays
notification-banner-host In-app banners

7.5 Views

View Contains
view-chats conversation-list
view-chat message-thread + message-composer
view-media Filter bar + media-grid
view-documents Filter bar + document-list
view-links Filter bar + link-list
view-link link-detail
view-document document-detail
view-hangout hangout-roster + controls
view-calls call-history-list
view-settings Section list + sub-views
view-profile Profile detail
view-room-settings Drawer (desktop) / modal (mobile)
view-join-room Invite confirmation
view-admin Administration

---

8. Crypto & E2EE

8.1 Three Encryption Layers

Layer Purpose Implementation
MLS Message encryption, group state, key packages, welcomes, commits Wire CoreCrypto 10.5.2 (WASM, bundled)
C2SP chunked AES-256-GCM Attachment blob encryption, range-based decryption Web Crypto API
OPRF token Username lookup, display-name encryption, device-name encryption, RMK delivery Client-side OPRF blinding

8.2 Room Metadata Key (RMK)

A static, room-scoped symmetric key.

Property Value
Length 32 bytes (AES-256)
Generation Random, at room creation, by the creator's client
Distribution Delivered via the MLS Welcome GroupInfo extension
Rotation None — static for the room's lifetime
Domain separation HKDF sub-keys per purpose

Sub-key derivation:

```
room_metadata_key    = HKDF(RMK, "room-metadata-v1",    32)
hangout_metadata_key = HKDF(RMK, "hangout-metadata-v1", 32)
```

Domain separation ensures that a compromise of one derived key doesn't affect the other.

8.3 OPRF-Based Lookup

Purpose: The server must not hold plaintext usernames or display names.

Construction:

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

Token usage:

```
display_name_key = HKDF(token, "display-name-encryption-v1", 32)
device_name_key  = HKDF(token, "device-name-encryption-v1",  32)
```

Domain separation ensures a compromise of one derived key doesn't compromise the other.

Client caches the token in memory for the session. Never stored at rest. Re-derived on login (requires username from secure storage; biometric login retrieves it automatically).

Token stability: k is immutable for the account lifetime. The token is stable across devices and sessions.

8.4 C2SP Attachment Encryption

Per Server Spec §6.5. Summary:

```
1. input_key = random 32 bytes
2. salt = random 24 bytes
3. context = "attachment" || 0x00 || purpose || 0x00 || room_id
4. info = "c2sp.org/chunked-encryption@v1+" || "AEAD_AES_256_GCM" || 0x00 || salt || context
5. key_material = HKDF-Expand(input_key, info, 76)
6. file_key = key_material[0..32]
   base_nonce = key_material[32..44]
   commitment = key_material[44..76]
7. header = salt || commitment
8. For chunk i: nonce_i = base_nonce XOR i
9. Ciphertext = header || encrypted_chunks
```

Purpose values (client-owned; server does not see them):

Purpose Use
"message" Regular message attachments
"room-avatar" Room avatars
"user-avatar" User profile images
"hangout-icon" Hangout icons
"sticker" Custom stickers

Padding: Plaintext is padded to the nearest bucket from ATTACHMENT_BUCKET_SIZES before encryption. The padded plaintext is chunked as a single C2SP message producing one short final chunk.

Range translation:

```
start_chunk = p_start / 16384
end_chunk   = p_end   / 16384
encrypted_start = 56 + start_chunk * (16384 + 16)
encrypted_end   = 56 + (end_chunk + 1) * (16384 + 16) - 1
```

8.5 MLS Client Behavior

CoreCrypto initialization:

```
1. Load device_secret from platform secure storage
   (generate 32 random bytes on first launch if absent)
2. entropy_seed = HKDF-Expand(device_secret, info="mls-entropy-v1", length=32)
3. Open the encrypted keystore (managed by CoreCrypto)
4. CoreCrypto.init(client_id, keystore, entropy_seed)
5. CoreCrypto.mlsInit()
```

Key package lifecycle:

· Target: 20 unconsumed key packages per device.
· Low watermark: 5.
· Replenish on login and on app launch.
· Rotate every 30 days.

Group lifecycle:

· Create: mlsCreateConversation(room_id), then batch-add members in a single commit.
· Join: Poll for welcome (GET /welcomes), process via mlsProcessWelcomeMessage, consume, fetch messages from welcome epoch.
· Leave: Server queues pending_mls_remove; another online member generates the Remove commit; the leaving client deletes local group state.

Message encryption/decryption:

· Encryption wraps the JSON payload in an MLS message, POSTs to /rooms/:id/messages.
· Decryption uses the message's epoch. Older epochs use previous-epoch keys. Newer epochs queue and sync.

Epoch transitions:

· The composer enters a "Securing…" state during transitions. Outgoing messages are queued and flushed once the new epoch settles.
· Concurrent commits handled by CoreCrypto's deterministic tie-break.

8.6 Key Transparency

The client consumes key transparency proofs from the server:

1. Fetch identity keys with tree_head, auditor_signatures, and inclusion_proof.
2. Verify the server signature, each auditor signature, and the inclusion proof locally.
3. Mark the contact as transparency-verified on success.
4. Display a green checkmark; no user action required in the common case.

Failure cases:

Case Display
Inclusion proof fails Warning; not verified
Auditor signatures missing "Verified by server only"
Auditor signatures invalid Warning; unverified
Network unavailable "Verification pending"

Manual safety numbers remain available under an "Advanced" section for users who want out-of-band verification.

8.7 Safety Numbers

Safety numbers are computed locally from MLS identity keys. Displayed in the profile view and room profile. Verification is soft; SAFETY_NUMBER_MODE is surfaced.

---

9. Media Pipeline

9.1 Format Matrix

Type Primary Fallback A Fallback B
Video WebM / VP9 + Opus WebM / VP8 + Opus MP4 / H.264 + AAC
Audio Opus (Ogg or WebM) AAC (M4A) —
Image AVIF WebP JPEG

Quality presets (default Balanced):

Preset Image Video Audio
High AVIF q80, max 2560px VP9, 1080p, ~4 Mbps Opus 96 kbps
Balanced AVIF q70, max 1920px VP9, 720p, ~2 Mbps Opus 64 kbps
Small AVIF q60, max 1280px VP9, 480p, ~1 Mbps Opus 48 kbps

9.2 Transcoding

Mediabunny handles demux, decode, encode, resize, rotate, crop, and thumbnail extraction. GPU path uses WebCodecs. CPU fallback is ffmpeg.wasm, loaded lazily.

Worker pool: Concurrency cap of 2 (1 on low-end devices). Jobs above 500 MB are processed alone. Each job exposes an AbortController.

Failed transcode: Fall back to passthrough — the file is uploaded unprocessed as a generic attachment.

9.3 Thumbnail and Poster Generation

During processing, the client generates a 128×128 WebP thumbnail alongside the full attachment. The thumbnail is encrypted separately (same key/salt/context) and uploaded as a companion blob. The manifest carries thumbnail_file_id.

For video, the client also extracts a poster frame for the thumbnail strip and preview.

9.4 Padding

Padding operates on the plaintext before encryption. The client:

1. Selects the smallest bucket T that can contain the plaintext.
2. Computes L_padded from T:
   ```
   M = T - 72
   q = floor(M / 16400)
   r = M - 16400 * q
   if r < 16384: L_padded = 16384 * q + r
   else: skip to next bucket (unreachable)
   ```
3. Appends zero bytes to reach L_padded.
4. Chunks the padded plaintext.
5. Encrypts each chunk.

9.5 Metadata Stripping

All processed media is stripped of metadata (EXIF, GPS, device info, XMP, IPTC, ICC, ID3). Documents are not modified.

9.6 Fast-Start MP4

For MP4 files, the client validates that the moov atom is at the start. Files where moov follows mdat are re-muxed with mp4box.js. If re-muxing fails, the file is transcoded to WebM/VP9.

9.7 Progress and Cancel

Per-file progress is reported inline in the message thread:

State UI
queued Clock icon
transcoding Progress ring, cancel button
encrypting Brief
uploading Progress bar, cancel
p2p_pending Waiting for recipient selection
p2p_transferring Per-recipient sub-rows
failed Retry or remove

Long uploads surface a Live Activity on iOS and an ongoing notification on Android.

---

10. Attachments

10.1 Upload Flow

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

10.2 Download Flow

· Files ≤ 1 MB: Single GET.
· Files > 1 MB: Lazy chunk fetch.
· Media (audio/video): Always lazy.

Presigned URLs are used when capabilities.storage_presign_supported is true. URLs are not cached; on expiry, a fresh URL is requested and the download resumes.

10.3 P2P Fallback

When a file exceeds effective_max_file_size_bytes for the room:

1. A member picker modal appears.
2. WebRTC data channels are established (one per recipient).
3. The file transfers directly, encrypted with an ephemeral key per recipient.
4. Chunk-level acknowledgment enables resume on connection drop.
5. Cancel discards partial data on both sides.

Online-only. If the recipient is offline, the send fails with a clear message.

10.4 Deletion

Operation Scope Permission
Delete for me Local only Anyone
Unsend All members Sender, within 24h
Remove from room All members Owner (or moderator in Discord mode)

Blob deletion: Triggered by unsend/remove. Server prunes when no other references exist.

10.5 Attachment Icon Mapping

File extensions map to seven canonical categories, each with a color. The canonical name resolves to a Solar icon via the icon plugin.

Category Color Extensions
document Red pdf, doc, docx, odt, rtf, pages
spreadsheet Green xls, xlsx, csv, ods, numbers
presentation Orange ppt, pptx, odp, key
text Blue txt, md, json, js, ts, py, rs, go, html, css, log
archive Amber zip, rar, 7z, gz, tar, bz2
design Purple fig, sketch, xd, psd, ai
audio Teal mp3, m4a, wav, ogg, opus, flac, aac, wma
generic Grey everything else

Resolution order: MIME type → extension → generic.

---

11. Timeline (Message Thread)

11.1 Bubble Layout

Aspect Own messages Others' messages
Alignment Right Left
Avatar Omitted ui-profile on left, bottom-aligned
Sender name N/A Above the bubble on first message of a group (group chats only)
Bubble color var(--bubble-outgoing) var(--bubble-incoming)
Timestamp Inside bubble, bottom-right Inside bubble, bottom-left
Read receipt "Read" / "Read N" next to timestamp —

Bubble max width: min(75%, 600px) on desktop. Corner radius: 18px default; 4px on the tail corner and inner group corners.

11.2 Sending States

State Icon
Queued Clock
Sending Diagonal upward-left arrow (↖)
Sent No icon (timestamp only)
Read "Read" / "Read N"
Failed Circular arrow (↻); tap to retry

11.3 Message Grouping

Consecutive messages from the same sender within 5 minutes are grouped: avatar and sender name appear only on the first message; timestamp on the last; inner corners tighten.

11.4 Date Separators

Centered pill-style separators on calendar day change: "Today", "Yesterday", "March 5, 2026". A sticky date header floats at the top of the viewport.

11.5 New Messages Divider

A horizontal line at the last-read position. Shown when opening a room with unread messages; hidden once scrolled past.

11.6 Scroll Behavior

· On open: Scroll to bottom, instant.
· New incoming while pinned: Auto-follow if within 80px of the bottom.
· New incoming while scrolled up: Do not follow; increment unseen count.
· Own send: Always scroll to bottom.
· Scroll-to-bottom button: Floating, appears when scrolled up; shows unseen count.
· Infinite scroll: Trigger at ~200px from top.
· Prepend anchoring: Viewport stays fixed on the message being read (TanStack Virtual anchorTo: 'end').
· Stable keys: Message IDs (not indices).

11.7 Message Context Menu

Long-press (mobile) or right-click / hover bar (desktop) opens the message context menu.

Universal actions:

· Copy
· Reply
· React
· Select (enters multi-select mode)
· Delete for me
· Read aloud

Sender-only:

· Unsend (within 24h)

Conditional:

· Edit (own messages, within 15-minute window)
· Show original (if edited)
· Delete (Discord mode: owner/moderator)

Hover bar (desktop only): A compact strip appears on the inner side of the bubble with four buttons: react, reply, edit, overflow. Right-click opens the full menu directly.

Browser context menu suppression: preventDefault() is scoped to .chat-bubble elements only.

11.8 Message Editing

Affordance: Long-press → "Edit" (mobile); hover bar → edit icon, or right-click → "Edit" (desktop).

Edit mode: The composer morphs — a banner shows "Editing" with a dismiss button, the input is pre-filled, and the send button becomes a checkmark.

Window: 15 minutes from the original send time. Enforced client-side and server-side.

Edits: Unlimited within the window. Each edit is a new signed message with reply_to set and sequence incrementing.

Indicator: "Edited" label next to the timestamp inside the bubble.

History: "Show original" opens a sheet showing the full edit chain with timestamps. New joiners see only versions they can decrypt; missing versions show "not available."

Media messages: Text/caption only. Attachments cannot be changed.

11.9 Voice Messages

Recording flow (WhatsApp-style):

Gesture Result
Press and hold mic Recording starts; release to send
Slide left while holding Cancel
Tap mic (without holding) Hands-free mode
Tap send Send
Tap trash Discard
Tap play Preview before sending

Composer button: Dynamic — mic when empty, send when text present.

In-thread display: message-audio-player with wavesurfer.js:

```
┌───────────────────────────────────────────┐
│  [▶] [🎙️▾]  ▁▂▃▅▆▅▃▂▁▂▃▅▆▅▃▂▁▂▃  0:15   │
└───────────────────────────────────────────┘
```

Element Behavior
Play button Large, prominent, primary action
Transcribe button Icon with a dropdown chevron (▾). Tapping the icon transcribes; tapping the chevron opens the picker
Waveform Interactive — tap to seek, drag to scrub
Duration Total duration idle; elapsed/total playing

Transcribed state:

```
┌───────────────────────────────────────────┐
│  [▶] [🎙️▾]  ▁▂▃▅▆▅▃▂▁▂▃▅▆▅▃▂▁▂▃  0:15   │
│                                            │
│  "Hey, just checking in about the..."     │
│                                            │
│  Fast · Transcribed                       │
└───────────────────────────────────────────┘
```

Transcription mode picker (bottom sheet mobile, popover desktop):

User-facing label Technical model Description
Fast Moonshine Tiny Best for English. Quickest.
Accurate Moonshine Small Best for English. Slower, more precise.
Multilingual Whisper Tiny Supports 99 languages.

Download states: Downloaded / Not downloaded / Downloading / Failed. Model labels use plain language — never "model."

Bad transcript detection:

Condition Response
Empty or whitespace only Suggest Multilingual or Accurate
Known silence markers ([silence], [music], [BLANK_AUDIO], [no speech], (silence)) Same
Under 3 words for audio > 10s Same
Under 5% of estimated word count Same

Suggestion UI:

```
⚠️ No speech detected
This can happen if the audio is in a different
language, or if the recording is quiet.
[ Try Multilingual ]  [ Try Accurate ]
```

Peaks: A compact float array (~100–200 values) is included in the payload for instant waveform rendering.

Max duration: 30 minutes.

11.10 Read-Aloud (TTS)

Mode toggle in composer:

```
[＋] [😊] [🔊] [  Message...  ] [🎤]
              ↑
       read-aloud toggle
```

· Grey icon when off; accent-filled when on
· Tap toggles mode
· Long-press opens options sheet (speed, voice, "Read from here")
· Persists across sessions
· On narrow screens (< 360px), consolidate [＋] and [😊] into [＋] menu

Applies to every readable bubble:

Bubble type Read aloud?
Text message Yes
Voice message transcript Yes
Caption on a media message Yes
Message with an attachment and no caption Reads the filename
Link preview Reads the title and description
System message Yes
Sticker No
Image without caption No
Tombstone Yes

Sentence-level control:

· Messages are pre-segmented into sentences client-side
· Each sentence is a tap target
· Tapping any sentence starts/jumps playback to that sentence
· Currently-speaking sentence has a subtle highlight

```
┌───────────────────────────────────────────┐
│  Hey, just checking in about the meeting. │
│  Are we still on for 3pm?                 │  ← currently speaking, highlighted
│  Let me know when you get a chance.       │
└───────────────────────────────────────────┘
```

Auto-resume:

· Position stored per-message
· Tapping the same bubble resumes from where it stopped
· Cleared on: message completion, "Restart" action, message edit, 24-hour expiry

Long-press options:

```
Read aloud                          [✕]
● Resume from where it stopped
○ Restart from beginning
○ Choose starting point
Speed  [0.75×] [1.0×] [1.25×] [1.5×] [2.0×]
Voice  [ Default ▾ ]
```

"Choose starting point" opens the message with tappable sentences.

TTS model: Supertonic 3 (~400 MB ONNX). 31 languages. WebGPU + WASM. 100% local. Downloaded on first use. Cached via Cache API. Never calls an external CDN.

Per-message action (mode off): Accessible via context menu.

Screen reader coexistence: Detect OS screen reader; offer to disable our read-aloud. If the user explicitly enables our mode, it takes precedence.

Keyboard shortcut: Ctrl/Cmd + Shift + R toggles mode from anywhere.

Settings → Accessibility → Read aloud by default: When enabled, new rooms open with read-aloud mode active.

Room Settings → Override global setting: Always / Never / Default.

11.11 Stickers and Emoji

Emoji picker: emoji-picker-element.

Stickers:

Tier Source Delivery
Bundled Ships with the app Static assets
Shared pack Encrypted attachment Cached locally
Custom User-created from camera roll Processed as media

Pack manifest: JSON with id, name, publisher, version, tray_image, and stickers[]. Each sticker has id, format (webp or lottie), file, width, height, emojis, keywords, and optional duration_ms.

Animated stickers: Lottie JSON, rendered via dotlottie-web with expressions disabled.

Message payload: { "type": "sticker", "pack_id": "...", "sticker_id": "...", "entrance": "pop" }.

Entrance transitions: none, pop, drop, slide, crash, fade. Plays on first view of a new message. Respects prefers-reduced-motion. Replay on tap.

Sticker combination: Long-press a sticker in the picker → "Combine" → multi-select up to 6 → drag, resize, rotate → send. Rendered to a canvas and sent as an image attachment.

Sticker suggestions: A keyword-to-sticker mapping runs locally as the user types.

Native keyboard sticker limitation: iOS and Android do not expose a public API for WebView-based apps to receive stickers from the system keyboard. Android's Commit Content API requires a native EditText; the WebView's <input> is not a native editor. iOS has no public API at all.

Paste-as-sticker: When an image is pasted into the composer, the client detects it and offers a choice if the image is small and square (heuristic: under 512×512, aspect ratio close to 1:1):

```
Send as:
[ Sticker ]  [ Image ]
```

· Sticker: Added to the user's custom sticker collection and sent as message_type: "sticker".
· Image: Goes through the normal attachment pipeline.

The choice is remembered for future pastes.

Education:

· First-use hint near the sticker button (once)
· Paste-aware tooltip (once per device)
· Settings → Chats → Stickers help entry

11.12 Link Previews

Generated client-side by the sender. The preview (title, description, site name, favicon, and optionally an image) is embedded in the message payload.

Fetch: Debounced (500ms) after URL detection. 3-second timeout. If the fetch fails, fall back to a minimal card showing only the domain and URL.

Multiple links: Only the first URL gets a preview.

Opt-in server proxy: For URLs blocked by CORS, an optional server-side proxy can fetch metadata. Uses agent-fetch for SSRF protection. No logging, encrypted URL relay.

11.13 Reply / Quote

· Trigger: Swipe right on mobile; hover bar or right-click → Reply on desktop.
· Preview bar above the composer shows the quoted sender and snippet.
· Quoted snippet appears inside the reply bubble, tappable to jump to the original.
· Payload: reply_to: "<message_id>".

11.14 @Mentions

Composer autocomplete: Typing @ opens a popover listing room members. Filtering is prefix-based. Selection is required — typed mentions without selection are plain text.

Payload:

```json
{
  "type": "text",
  "text": "Hey @Alex how are you?",
  "mentions": [
    { "user_id": "u_abc123", "offset": 4, "length": 5 }
  ]
}
```

Rendering: Accent color pill, tappable to profile. Mentions of the current user get additional emphasis.

Notification override: Mentions override mute but not "Nothing."

@everyone / @here: Not in V1.

11.15 Markdown Formatting

Supported: bold, italic, strikethrough, inline code, links, bullet lists, numbered lists, block quotes.

11.16 Message Tombstone

When a message is deleted, the bubble is replaced in place with a muted pill: "This message was deleted."

11.17 In-Thread Search

· Entry point: Magnifying glass in the chat header.
· Scope: Current room only.
· Library: Fuse.js, lazy-loaded.
· Highlight color: #FFE066.
· Jump to message: Scroll + 2s flash + "Back to search".

11.18 Global Search

A search field in the chat list header. Searches room names and message content. Results group by room.

11.19 Disappearing Messages

Room-level timer stored in the encrypted metadata JSON.

Options: Off, 24h, 7d, 90d.

Who sets it: Room owner only.

Applies to: New messages only.

Timer start: At send time. The sender computes expires_at and includes it in the payload.

Cleanup: Local-only periodic task queries SELECT id FROM messages WHERE expires_at <= now, deletes those rows, and removes associated attachments from the local cache if no other message references them.

Interactions:

· Edits inherit the original's expires_at.
· Replies show [message expired] after expiration.
· Curation views reflect the removal.
· Search indexes updated on delete.

Server involvement: None. The timer travels in the encrypted metadata.

11.20 Nicknames

Per-user aliases for room participants.

Where shown:

· Chat thread sender names
· Member list
· Reply preview
· Mention autocomplete
· Profile view

Not shown:

· To other members (nicknames are user-scoped)
· On the user's own messages

Edit UI: Room Settings → Customise chat → Edit nicknames. Sub-view with a text field per member.

Storage: Local in room_preferences; synced via user_preferences key nicknames:{room_id}.

Fallback: If no nickname is set, the member's display name is used.

---

12. Composer

12.1 Controls

```
[＋] [😊] [🔊] [    Message...    ] [🎤/➤]
```

Element Action
[＋] Attachment picker
[😊] Emoji / sticker picker
[🔊] Read-aloud mode toggle
Text input Auto-growing textarea
[🎤/➤] Dynamic: mic when empty, send when text present

12.2 Reply Preview

A bar above the composer, part of the glass overlay. Composer height updates via ResizeObserver.

12.3 Attachment Preview

Between pick and send, a preview screen shows the processed result with thumbnail, filename, size, quality override, caption input, remove, and reorder.

12.4 Enter Key Behavior

Platform Enter Shift+Enter
Desktop Send Newline
Mobile Newline —
Tablet (physical keyboard) Send Newline

12.5 Focus Behavior

After sending, focus remains in the input. Virtual keyboard stays open on mobile. User dismisses manually via swipe-down on the message list.

12.6 Multi-Attachment Layout

Count Layout
1 image Full-width
2 images Two equal columns
3 images Left spans full height; right has two stacked
4 images 2×2 grid
5+ images 2×2 grid with +N overlay

Files render as stacked cards below the image grid. Caption renders last.

---

13. Rooms

13.1 Room Creation

1. Add members (username lookup or invite link).
2. For groups (3+ members), a collapsed "Customize room" disclosure exposes name and avatar.
3. Tap Create room.

For 1:1 (exactly 2 members), the disclosure is not shown.

Derived name (when not customized): first two display names, then + N. Cap: 2 names.

Auto-nudge: At 5+ members, the disclosure auto-expands with the name field focused.

Default avatar: ui-profile-group composite.

13.2 Member Management

· Add member via username lookup or invite link.
· Remove member: owner (or moderator in Discord mode).
· Promote/demote: owner (Discord mode only).

Member list display: Member rows show profile image and display name (with nickname override). Display name resolution chain:

1. Cached display name (from previous decryption or message payload)
2. Message payload (sender's display name is inside MLS-encrypted messages)
3. encrypted_display from member list response (only if token is cached for that user)
4. Fallback: user_id or "Member" placeholder

Practical consequence: Member lists show placeholders until a message is received from that member. Once the first message arrives, the display name resolves.

13.3 Invite Links and QR

· Room-specific invite tokens.
· Default expiration: 7 days.
· Display: QR (via qr-code-styling), copy link, OS share sheet.
· Regenerate invalidates the current link and issues a new one.
· Scanning: camera (Capacitor) or paste (desktop).

13.4 Username Lookup

Full cycle:

Phase 1 — User taps "Add member." Sheet opens with a single Username field.

Phase 2 — Lookup (invisible):

Step Where What happens
1 Client Blinds the username with a random factor
2 Client → Server POST /oprf/blind
3 Server OPRF evaluation with k
4 Client Unblinds → token
5 Client → Server POST /users/lookup { username_token }
6 Server Matches token, returns { user_id, encrypted_display }
7 Client Derives display_name_key from token
8 Client Decrypts display name

Server saw two opaque values. Never saw the plaintext username or display name.

Rate limiting: OPRF endpoint rate-limited. Client surfaces: "Too many attempts. Please wait a moment and try again."

Phase 3 — Confirmation card. On success: profile image + display name + username + "Add to room" button. On failure: hint card with case-sensitivity, no-spaces, full-username reminders. Fallback: "Invite via link instead."

Phase 4 — Adding to room:

Step What happens
1 POST /rooms/:id/members { user_id }
2 Server adds to room_members, inserts pending_mls_adds for each of B's devices
3 Server publishes mls.add_pending on room channel
4 Any online member generates Add commit + Welcome
5 Server publishes mls.welcome_ready to B
6 Server publishes room.member_added on room channel

System message: "You added Alex."

Phase 5 — Recipient side. Online: receives mls.welcome_ready, fetches welcome, processes, fetches messages from welcome epoch, room appears. Offline: push notification. On open, processes pending welcome.

Phase 6 — Post-add. Display name cached locally. Future messages carry sender's display name inside encrypted payload.

Failure modes:

Failure Recovery
Username doesn't exist Retry or invite link
Rate-limited Wait and retry
Already a member Toast
Room full Toast
MLS add fails Retry; client-mls-request after 5min

13.5 Room Settings

Display pattern:

· Desktop (≥1024px): Off-canvas right drawer within Panel 3. Width min(480px, 40%). No backdrop scrim. Dismiss: X, Escape, click outside.
· Mobile: Full-screen modal with X.

Header: Room Settings title with X on the right. No gear icon.

Profile block: Room avatar, name, E2EE badge directly below.

Sections (disclosure pattern, collapse state persisted in room_preferences):

Section Rows Permission
Customise chat Change theme, Edit nicknames Per-user
Members View members, Add member, Invite via link All
Notifications Mute, Sound, Vibration Per-user
Privacy Read receipts, Typing indicators Per-user
Media & Storage Auto-download, Storage used, Clear cache Per-user
Retention & Limits Message retention, Max file size Owner
Moderation Moderation mode Owner (Discord mode)
Danger Zone Transfer ownership, Delete room Owner
Leave room — All

1:1 rooms replace Members/Invites with a "This conversation" section (Block, Delete chat).

Icon circles on each row (consistent color scheme: accent for positive actions, neutral grey for informational, danger red for destructive).

Destructive actions: Delete room and transfer ownership require type-to-confirm. Kick and leave use single-confirmation dialogs.

13.6 Room Metadata JSON

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

Encrypted with room_metadata_key.

13.7 Room Avatar Upload

1. Tap avatar in Room Settings → Profile (owner only).
2. Source sheet: Camera / Photo library / Remove photo.
3. Circular crop (1:1 aspect ratio, drag, pinch/scroll to zoom).
4. Multi-size preview.
5. Upload as encrypted attachment; update metadata JSON; broadcast room.updated.

Constraints: 20 MB max source, 128×128 min, JPEG/PNG/WebP/AVIF/HEIC. Output 512×512 AVIF q80.

13.8 Room List Ordering

Default: Fixed by join date. New rooms append to the bottom. Activity does NOT reorder the list.

Manual reordering: Activated from the room list header menu. Drag handle on each row. Keyboard alternative: Alt+↑ / Alt+↓. "Reset to default" reverts to join date.

Order persists locally and syncs across devices via PATCH /users/me/room-order.

Filters:

Filter Behavior
Unread only Show only rooms with unread messages
Mentions only Show only rooms with unread mentions
Muted rooms Show only muted rooms
Active hangouts Show only rooms with active hangouts

Reorder mode is disabled while filters are active.

13.9 Conversation Row Content

Position Content
Left ui-profile (composite for groups)
Top line Room name (bold if unread) + optional hangout indicator
Bottom line Last message preview, or typing indicator, or draft
Top right Timestamp
Bottom right Unread badge or mute icon

Hangout indicator: small headphones icon + count inline next to the room name when the room has an active hangout. Uses the hangout's own icon if there's exactly one active hangout; a generic icon + count if multiple.

Indicators (prioritized):

Indicator Meaning
Red @ pill Unread mentions in a muted room
Green pill with count Unread messages
Green dot Manually marked unread
Grey pill with count Unread in a muted room
Bell-slash Room is muted
Pencil Draft exists

Message preview rules:

Last message Preview
Text Sender: message text (or You: ...)
Image Sender: 📷 Photo
Video Sender: 📹 Video
Voice Sender: 🎤 0:15
Document Sender: 📎 filename.pdf
Sticker Sender: [Sticker]
Tombstone Falls back to previous non-tombstoned message
System message Falls back to previous message

Typing indicator replaces the preview: Alex is typing....

13.10 Leaving and Deletion

· Leave room (all members): Confirmation; warns if last member.
· Delete room (owner): Type-to-confirm. Other members receive room.deleted, room removed locally, system message: "This room was deleted by [owner]."
· Rejoining after leaving: Fresh view from rejoin forward. Historical messages from before the leave are not re-delivered (MLS forward secrecy).

13.11 Member Roles

Action Owner Moderator (Discord) Member
Send messages ✅ ✅ ✅
Upload attachments ✅ ✅ ✅
Add members ✅ ✅ ✅
Create invites ✅ ✅ ✅
Edit room metadata ✅ ❌ ❌
Change retention ✅ ❌ ❌
Kick members ✅ ✅ ❌
Delete any message ✅ ✅ ❌
Delete own message ✅ ✅ ✅
Transfer ownership ✅ ❌ ❌
Delete room ✅ ❌ ❌
Change disappearing timer ✅ ❌ ❌

---

14. Curation Views

14.1 Media Grid

Combined photos and videos in a single grid. Grouped by date, newest first.

```
Today
▢ ▶ ▢
▢ ▢ ▶
Yesterday
▢ ▢ ▢
Sep 23
▢ ▢ ▢
```

Layout: 3-column flush grid. Square thumbnails with object-fit: cover.

Video thumbnails:

· Centered play icon overlay (semi-transparent black circle, white triangle)
· Duration badge in bottom-right corner (0:15, 1:23:45)
· Poster frame from the media pipeline

Section headers: Small, muted, left-aligned. Not sticky.

No per-tile date overlay. Date is in the section header.

Inline video preview:

· Desktop: hover with 250ms intent delay. Muted, looping, streaming.
· Mobile: long-press with 250ms hold. Muted, looping, streaming.
· Bound: the gesture duration. No byte or time cap.
· Only gate: prefers-reduced-motion: reduce disables preview entirely. Static poster with play icon.
· Prefetch: first 2–3 chunks of visible videos during idle.
· State: single global preview; clean teardown on switch. Decrypted chunks persist in the range cache.

Empty state: "No media yet. Photos and videos shared in your chats will appear here."

14.2 Documents and Links

· List groups using list-row. Grouped by date (newest first).
· Documents show file-type icon (including audio category), filename, size, sender, date.
· Links show favicon, title, domain, sender, date.

14.3 Filter and Search

Filter chips hidden by default behind a filter icon in the header.

Filter sheet:

Filter Options
Chat All chats / specific
Date Today / Last 7 days / Last 30 days / Custom
Media type All / Photos / Videos
Sender Any / specific
Starred All / Starred only

Active filters appear as dismissible pills in the header.

Search icon toggles search visibility (not always shown).

Search fields: filename, sender, chat (media); title, domain, sender, chat (links).

Threshold: 0.35. includeMatches: true for highlighting.

14.4 "View in Chat" Action

Opens /app.html?view=chat&id=:roomId&messageId=:msgId. Scrolls to the message, highlights for 2s. If the message is tombstoned, a system notice appears.

Label: "View in chat" in overflow menus. "View in chat — See the message and its conversation" with sub-line in info sections.

Icon: chat bubble glyph.

14.5 Media Viewer

Scoping:

Opened from Source list
Chat thread All media in the current room, chronological
Curation view The currently filtered list

List-detail navigation (see §6.6): horizontal swipe on mobile, hover arrows + keyboard on desktop. Position indicator in header (Media · 3 of 47).

Scrolling: CSS Scroll Snap (scroll-snap-type: x mandatory).

Morph: View Transitions API.

Navigation: Arrows (desktop), swipe (mobile), keyboard (←/→).

Zoom and pan (images):

· Pinch, double-tap, scroll wheel.
· Min 1×, max 5×.
· Drag to pan when zoomed; drag at 1× navigates.
· Zoom resets per slide.

Swipe-to-dismiss:

· Swipe down dismisses at 1×; when zoomed, swipe down pans.
· Escape (desktop) or click outside dismisses.
· Back button (mobile) dismisses.

Action bar (top, glass chrome):

Action Availability
Save Always
Share Always
Copy Always
View in chat Always
Delete Only if the current user uploaded the attachment
Info Always
Star Always

Thumbnail strip: Bottom of the viewer. Virtualized (±20 items). Two-way sync with the main slideshow. Thumbnails decrypted and cached locally (128×128 WebP). Generation is lazy and runs in a Web Worker.

Video:

· <video> element in a slide.
· IntersectionObserver pauses playback when the slide scrolls out of view.
· Poster frame shows while loading.
· Auto-play respects the Auto-download media setting.

Prefetching: Active slide ±2 for decryption. Thumbnails ±20.

Closing behavior:

Trigger Effect
Swipe down Dismiss
Tap outside Dismiss
Escape (desktop) Dismiss
Share Opens OS share sheet; viewer stays open
Save Saves; toast confirms; viewer stays open
View in chat Closes viewer; navigates
Delete Confirmation; viewer moves to next item or closes

14.6 Document Detail

New view. Shares layout with link detail.

```
┌─────────────────────────────────────────┐
│  ←  Document · 5 of 23              [⋯] │
│                                         │
│  ┌───────────────────────────────────┐  │
│  │       [PDF preview or              │  │
│  │        file icon]                  │  │
│  └───────────────────────────────────┘  │
│                                         │
│  Q3-report.pdf                          │
│  2.4 MB · PDF · 12 pages                │
│                                         │
│  [ Open in app ]                        │
│                                         │
│  [ Copy ]  [ Share ]  [ Star ]          │
│                                         │
├─────────────────────────────────────────┤
│  Shared by thomas                       │
│  In Design Team                →        │
│  3 weeks ago                            │
└─────────────────────────────────────────┘
```

Preview behavior:

Category Preview
PDF Inline viewer via <embed> or <iframe> on a blob URL
Image-as-doc Rendered as an image
Video-as-doc Rendered as a video
Audio-as-doc Rendered as an audio player (message-audio-player)
Text Plain text render, monospace
Code Syntax-highlighted via highlight.js
Office Not previewed; file card with "Open in app"
Archives Not previewed; file card with "Open in app"
Unknown Not previewed; file card with "Open in app"

"Open in app": On mobile, uses @capacitor/filesystem and @capacitor/share. On desktop, openPath() from tauri-plugin-shell. On web, a download link.

14.7 Link Detail

New view.

```
┌─────────────────────────────────────────┐
│  ←  Link · 12 of 47                 [⋯] │
│                                         │
│  ┌───────────────────────────────────┐  │
│  │      [OG preview image]           │  │
│  └───────────────────────────────────┘  │
│                                         │
│  🌐 davidproduction.jp                  │
│  株式会社デイヴィッドプロダクション      │
│  アニメーションスタジオ...              │
│                                         │
│  [ Open in browser ]                    │
│                                         │
│  [ Copy ]  [ Share ]  [ Star ]          │
│                                         │
├─────────────────────────────────────────┤
│  Shared by thomas                       │
│  In Design Team                →        │
│  3 weeks ago                            │
│                                         │
│  ┌───────────────────────────────────┐  │
│  │  In the message                   │  │
│  │  thomas: "Check out this studio,  │  │
│  │  they made the animation for..."  │  │
│  │  [ View in chat ]                 │  │
│  └───────────────────────────────────┘  │
└─────────────────────────────────────────┘
```

Elements:

Element Behavior
OG preview image Full-width, max-height ~50vh
Domain with favicon Small favicon + domain, muted
Title Large, wraps to multiple lines
Description Muted, max 4 lines with ellipsis
Open in browser Primary action, accent-colored, full-width
Copy / Share / Star Row of chip-style buttons
Info section Shared by, In [room], date (relative, locale-aware)
Message context card Compact preview of the original message with "View in chat"
Overflow menu Copy link, Share, Star/Unstar, Delete (if owner)

Empty/error states:

State Display
Preview image failed to load Solid color placeholder with domain initial
OG fetch failed at share time Domain-only card
Link no longer reachable No special treatment; browser handles
Attachment (OG image) missing Placeholder; no error

14.8 Curation Empty States

Surface Copy
Media "No media yet. Photos and videos shared in your chats will appear here."
Documents "No documents yet."
Links "No links yet."
Filtered "No [media/docs/links] match your filters."
Starred filter "No starred items yet. Star anything to find it here."

---

15. Calls and Hangouts

15.1 Call Model

Room-scoped. A 1:1 call is a 2-member room. A group call is a room with 3+ members.

15.2 Pre-Call Preview

```
┌─────────────────────────────────────────┐
│  ←   Call                          [✕]  │
│                                         │
│         [Camera preview or profile]     │
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

15.3 Incoming Call

· Full-screen: Foreground.
· Banner: Another app (iOS via CallKit, Android via full-screen intent).
· Actions: Decline, Accept, Message, More.

15.4 In-Call Screen

Voice 1:1: Profile-centric. Controls: speaker, mute, switch to video, minimize, end.

Voice group: Grid (up to 9 profiles) or list view with mic/video status. Active speaker highlighted with a green ring.

Video 1:1: Remote video full-screen; self-view as draggable PiP. Controls: speaker, mute, flip camera, minimize, end.

Video group: Grid (up to 6) or focus view with filmstrip.

15.5 Minimized Call

Picture-in-picture floating window, draggable. On desktop, the call can dock into a resizable pane within Panel 3.

15.6 Call History

Calls tab in the rail (and bottom nav on mobile). Rows show profile, room name, call type, direction (incoming / outgoing / missed), duration, timestamp.

Group calls over the participant cap: The call starts with the first N who accept. Additional members see: "This call is full."

Call history and disappearing messages: Call history is not affected by the room's disappearing timer.

Room deletion during a call: The call ends with reason: "room_deleted".

15.7 Hangouts

Persistent, named, room-scoped voice spaces.

Properties:

· Persistent — exists until deleted
· Named — encrypted metadata holds name, icon, description
· Multiple per room (default limit 10)
· Occupancy is ephemeral — server knows in memory only
· Room is the permission boundary

Entry point: Headphones icon in the room header (see §6.7).

Header icon states:

State Treatment
No hangouts Icon present, no badge
Hangouts exist, none active Subtle dot
Active hangouts Count badge (total participants)

Hangout list sheet (opened from header icon):

· All hangouts in the room
· Each with name, icon, participant count, join action
· "Create hangout" action at the top

Active strip: Appears above the message thread when at least one hangout has participants.

```
┌───────────────────────────────────────────┐
│ 🌴 The Lounge · 3 people         [ Join ]│
└───────────────────────────────────────────┘
```

· Single line, ~48px height
· Only present when participants > 0
· If multiple hangouts are active, stack or scroll horizontally

In-hangout bar: Replaces the active strip when the user has joined.

```
┌───────────────────────────────────────────┐
│ 🌴 The Lounge  🎤 On  🔊 On  [Expand] [✕]│
└───────────────────────────────────────────┘
```

Full hangout view:

· Room name at the top
· Hangout name as the title
· Participant grid (roster)
· Controls: mic, speaker, camera, screen share, leave
· On desktop: opens in Panel 3
· On mobile: full-screen sheet

List-detail navigation: The hangout list supports horizontal navigation through hangouts in the room. Position indicator in header.

Create/Edit sheet:

```
┌─────────────────────────────────────────┐
│  New hangout                       [✕]  │
│                                         │
│  Name                                   │
│  [ The Lounge                 ]         │
│                                         │
│  Icon                                   │
│  [ 😊 emoji ]  [ 📎 upload ]            │
│                                         │
│  Description (optional)                 │
│  [                            ]         │
│                                         │
│  [ Create ]                             │
└─────────────────────────────────────────┘
```

Metadata encryption: hangout_metadata_key = HKDF(RMK, "hangout-metadata-v1", 32).

Icon attachment: Encrypted with C2SP context "attachment" || 0x00 || "hangout-icon" || 0x00 || room_id.

Icon fallback: If the attachment is missing or unfetchable, fall back to the emoji variant or a default icon. Do not surface an error.

15.8 Hangout Client Behavior

Joining:

· Send POST .../join { client_id }. Server returns roster + ICE config.
· Roster has one entry per user, with a client_ids array.
· Joining participant initiates offers to every existing participant.
· Existing participants answer.
· Client must NOT call GET .../roster before joining (403 not_a_participant).

Heartbeat:

· POST .../heartbeat every 15 seconds.
· Server marks stale after 45 seconds.
· Three consecutive failures → treat as disconnected. Show reconnect action. Do not auto-rejoin.

Leaving:

· POST .../leave { client_id }. Removes only that client.
· If occupancy becomes empty, server tears it down.

Room deletion:

· Server publishes hangout.deleted for every hangout in the room.
· Client MUST tear down media on receiving hangout.deleted.

Reconnect:

· On WebSocket disconnect and reconnect, client does NOT auto-rejoin.
· Shows hangout as "disconnected" with a rejoin action.
· Fetches GET /rooms/:id/hangouts on reconnect to refresh counts.

Multi-device:

· A user may have multiple client_ids in a hangout.
· Client uses the same stable client_id as for MLS.
· Leaving from one device does not remove the user if another device is still present.
· If the user is in a hangout on device A and opens the room on device B, device B shows "You're in this on another device." Device B can call POST .../leave { client_id: <device_A_id> } to leave on behalf of device A.

Signal payload: hangout.signal includes target_client_id so recipients filter to the intended device.

15.9 Hangouts — Prohibitions

These are normative and protect the design:

1. Do not cache the roster beyond the session. Discard on leave. Do not persist to SQLite.
2. Do not display identities to non-participants. Counts only.
3. Do not log occupancy. No console logs, no analytics, no crash reports containing participant lists.
4. Do not auto-rejoin. Rejoin is always a user action.
5. Do not send notifications on hangout join.
6. Do not put hangouts in call history.
7. Do not synthesize presence. No "last seen in lounge," no "recently active in hangout."

15.10 Hangout Rate Limits

Client must respect:

· Create: 20/hour, 100/day
· Join: 30/minute
· Heartbeat: 10/minute per hangout (normal usage 4/minute)

If heartbeat limit is hit, log locally and do not retry aggressively.

15.11 Hangout Feature Disabled

If capabilities.hangouts_enabled=false:

· Hangout metadata remains visible (read/patch/delete work)
· Join/signal/heartbeat return 501
· Client shows: "Hangouts are disabled on this server." when join is attempted
· Existing hangouts can still be deleted by their creator/owner

---

16. Auth

16.1 Auth Gate

Single page (index.html). Three views: Login, Register, Recovery.

16.2 Login View

· Username field
· Password field
· "Log in with Face ID" (if enrolled)
· "Register with invite code →"
· "Lost access? Use recovery →"

Login flow (client):

1. Blind the typed username → POST /oprf/blind → unblind → token.
2. POST /auth/login/start { username_token, opaque_client_auth_state }.
3. OPAQUE AKE locally.
4. POST /auth/login/finish { username_token, ke3 }.
5. Store session token.
6. Cache OPRF token in memory.
7. window.location.href = '/app.html'.

16.3 Register View

1. Invite code (8-char Crockford Base32).
2. ALTCHA widget.
3. Username.
4. Display name.
5. Password.
6. "Create account".

Registration flow (client):

1. Blind the username → POST /oprf/blind → unblind → token.
2. Derive display_name_key = HKDF(token, "display-name-encryption-v1", 32).
3. Encrypt display name with display_name_key.
4. OPAQUE registration locally.
5. POST /auth/register/finish { username_token, encrypted_display, opaque_record, identity_pubkey, altcha }.
6. Server returns:
   · session_token
   · user_id
   · Recovery codes (plaintext, shown once)
7. Display recovery codes. Download is mandatory — Next disabled until downloaded.
8. Confirmation: type the 3rd code back.
9. Client caches the OPRF token.
10. window.location.href = '/app.html'.

Server-generated recovery codes. Hashed with Argon2id + per-code salt. Client never generates codes.

16.4 Recovery View

Separate flow, not a login-mode variant.

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

Recovery flow (client):

1. Blind username → OPRF → token.
2. POST /auth/recover/start { recovery_code, username_token }.
3. Server verifies code (Argon2id, constant-time).
4. Server returns OPAQUE registration challenge.
5. Client runs OPAQUE registration with new password.
6. POST /auth/recover/finish { username_token, recovery_session, opaque_record, encrypted_display, identity_pubkey }.
7. Server replaces OPAQUE record, consumes code, revokes all existing sessions, issues new session.
8. Client stores new session, redirects to /app.html.

Server behavior:

· Recovery codes hashed with Argon2id + per-code salt.
· Constant-time comparison.
· Code consumed (single-use or decrement) on success.
· Session revocation is mandatory.

Other devices: receive session.revoked → clear session → redirect to index.html.

Recovery code display screen (initial registration):

· Codes shown once, must be downloaded.
· Confirmation step requires typing one back.
· Explicit warning: "These codes cannot be recovered if lost."
· Regeneration only available from an authenticated session.

16.5 Biometric Unlock

· Root of trust remains OPAQUE + password.
· Biometric gates access to the locally stored OPAQUE RegistrationRecord.
· Enrollment offered once after a successful password login.
· Unenrollment in Settings → Account → Security.
· Auto-invalidated on password change.

Screen lock timeout: Settings → Privacy → Screen lock → Immediately / 1 min / 5 min / 30 min. Default: 1 minute.

Biometric login flow: Biometric unlocks the OPAQUE record → username retrieved from local storage → OPRF blinding → token → login proceeds. User types nothing.

16.6 Device Linking

Adding a new device when an existing session is active:

1. Existing device generates a device linking code (QR or short code).
2. New device enters or scans the code.
3. New device performs OPAQUE login with password.
4. New device uploads its MLS key package.
5. New device fetches user-scoped sync state: GET /users/me/sync?since_seq=0.
6. Device names and other user-scoped state populate.

Device names:

· Server stores encrypted_device_name as opaque ciphertext in user-scoped sync state.
· Client encrypts with device_name_key = HKDF(token, "device-name-encryption-v1", 32).
· Cross-device: device.name_updated event on private-user-{user_id}.

16.7 Passkeys (V2 Seam)

Not in V1. Data-driven action list in the login view.

16.8 Session Expiry

Auth token expired → redirect to index.html.
OPAQUE session expired → inline error.

16.9 Multi-Device Session Semantics

Event Effect
Log out on A Only A
Revoke B from A B via session.revoked
Delete account All devices
Change password All other devices
Complete recovery All other devices

16.10 Multi-Device Read State Sync

· Client publishes read state via POST /users/me/read-state { room_id, last_read_message_id }.
· Server assigns user_seq, publishes read.sync on private-user-{user_id}.
· Client stores last_user_seq in _meta.
· On boot, GET /users/me/sync?since_seq=<cursor> fetches missed state.
· Debounced: at most one POST per room per 30 seconds, plus immediate flush on room switch and app background.

---

17. Settings

Single view. Search bar at the top. Rendered in Panel 3 (desktop) or full-screen push (mobile).

17.1 Account

Row Control
Profile Navigate → display name (encrypted), profile image, username (read-only)
Password Navigate → change password
Recovery codes Navigate → status, regenerate
Devices Navigate → list, revoke
Biometric unlock Toggle
Passkeys Navigate → V2 placeholder
Starred items Navigate → list

Display name update: Re-encrypt with cached display_name_key and fresh nonce. PATCH /users/me { encrypted_display }.

Device names: Encrypted locally, synced via user-scoped state.

17.2 Privacy

Row Default
Read receipts On
Typing indicators On
Message preview in notifications On
Screen lock Off
Screen lock timeout 1 minute
Blocked users Navigate → list
Emergency wipe Configure trigger

Removed: "Last seen" row. Presence is out of scope.

17.3 Notifications

Push notifications, Sound, Vibration, Per-room overrides, Muted rooms.

17.4 Chats

Font size, Default chat theme, Per-chat themes, Enter to send, Voice message auto-play, Voice playback speed, Draft messages, Stickers help.

17.5 Media & Storage

Storage usage, Clear cached media, Auto-download media/docs, Transcode quality, Strip metadata, Max upload size.

17.6 Transcription (STT)

Row Default
Auto-transcribe voice messages Off
Show transcript below voice message On
Default mode Fast
Downloaded modes Navigate → list, delete
Transcription language Auto-detect

Labels: Fast / Accurate / Multilingual. Never "model."

17.7 Read Aloud (TTS)

Settings → Accessibility → Read aloud.

Row Default
Read aloud by default Off
Voice English (US) — Natural
Speed 1.0×
Auto-read incoming messages Off
Downloaded voices Navigate → list, remove
Download additional voices Action

Plain language throughout. No "model" terminology.

17.8 Calls

Default call mode, Call ringtone, Video quality, Show calls tab.

17.9 Appearance

Theme, Accent color, Wallpaper, Reduced motion, Sticker animations, Language.

17.10 About

Version, Terms, Privacy, Licenses, Log out, Log out and clear data, Delete account.

17.11 Starred Items

Settings → Account → Starred items.

List of all starred items (media, messages, links), filterable by type. Each row: thumbnail/icon, source (room name + sender), date starred. Tap to view.

Unstar: Swipe or long-press → "Unstar."

Empty state: "No starred items yet. Star anything to find it here."

17.12 Administration

Rendered only if capabilities.admin is present.

17.13 Sign-Out and Account Deletion

Log out: Session cleared, local data preserved (encrypted at rest).

Log out and clear data: Everything cleared.

Account deletion: Warning screen → type username to confirm → server anonymizes → local data wiped → Auth Gate.

Recovery triggers session revocation on all other devices. Affected devices handle it via session.revoked.

---

18. Notifications

18.1 In-App Banner

Top of detail panel (desktop) / top of screen below status bar (mobile). One at a time. Auto-dismiss after 5s. Mute exception for mentions. No inline reply.

Hangout events do NOT trigger notifications.

18.2 Push Notifications

Case matrix:

App state WebSocket Mechanism
Foreground, same room Connected No notification
Foreground, different room Connected In-app banner
Backgrounded (< 30s) Connected No push yet
Backgrounded (> 30s) Disconnected Push notification
Lock screen Disconnected Push notification
App killed Disconnected Push notification

Permission flow: Pre-prompt (once, after 24h + activity) → OS prompt. iOS provisional notifications adopted. No re-prompt after denial.

Push actions:

Action iOS Android Web
Reply ✅ inline ✅ inline ❌
Mark as read ✅ ✅ ✅
Tap ✅ deep link ✅ deep link ✅ deep link

Reply architecture: NSE decrypts, app process encrypts and sends.

Screen lock interaction: Reply prompts biometric if Screen lock is on.

Notification categories:

Category Actions
MESSAGE Reply, Mark as read
MESSAGE_MUTED Reply
MENTION Reply, Mark as read
CALL Accept, Decline
MEMBER_CHANGE (none)

No push on hangout join.

---

19. Cross-Platform

19.1 Capacitor

Push via @capacitor/push-notifications. Secure storage via keychain. Status bar via capacitor-edge-to-edge. Biometric via @capacitor/biometrics. QR via camera plugin.

19.2 Tauri

Push via tauri-plugin-notification. Secure storage via tauri-plugin-stronghold. Safe areas via tauri-plugin-safe-area-insets-css. Biometric via tauri-plugin-biometric. Deep link via tauri-plugin-deep-link.

19.3 Shared CSS

Same CSS variables across platforms. No platform branching.

19.4 Privacy Screen

App switcher snapshot blurred.

Platform Mechanism
iOS Blur overlay in applicationWillResignActive
Android 13+ setRecentsScreenshotEnabled(false)
Android 12 and below Not blocked

User screenshots remain allowed.

19.5 Deep Linking

URL format: https://app.example.com/app.html?view=chat&id=r_abc123&messageId=m_xyz789

Pending slot: In-memory only. Invite tokens use sessionStorage for auth-flow survival.

Processing order: Boot → auth → sync start → route → clear.

Platform config:

Platform Universal links Custom scheme
iOS apple-app-site-association, Associated Domains CFBundleURLTypes
Android assetlinks.json, intent-filter intent-filter
Tauri Info.plist (macOS), registry (Windows) tauri.conf.json schemes
Web Query params only —

---

20. Empty States

Three variants: empty, no results, error.

Surface Variant Copy
Room list (new) empty "No chats yet. Create a room or join with an invite link."
Room list (search) no-results "No results for 'query'"
Room list (error) error "Couldn't load your chats"
Chat thread (new) empty "This is the beginning of the room."
Chat thread (MLS pending) empty "Setting up encryption…"
Media empty "No media yet. Photos and videos shared in your chats will appear here."
Documents empty "No documents yet."
Links empty "No links yet."
Curation (filtered) no-results "No [media/docs/links] match your filters."
Hangout list empty "No hangouts yet. Create one to start a voice space."
Hangout list (filtered) no-results "No hangouts match your filters."
Starred items empty "No starred items yet. Star anything to find it here."
Read-aloud voices empty "No voices downloaded. Download one to start reading aloud."
Calls empty "No calls yet."
Blocked users empty "No blocked users"
Downloaded modes empty "No modes downloaded"
Custom stickers empty "No custom stickers yet"
In-thread search empty "Nothing to search yet"

Rules: Shared-rooms section hidden when empty. One primary action per state.

---

21. Error Handling

21.1 Network Layer

WebSocket connection state authoritative. No global banner. Inline message states, one-shot toasts.

Retry strategy:

Operation Strategy
WebSocket reconnect Exponential backoff: 1s, 2s, 4s, 8s, 16s, 30s cap
Failed message send 3 attempts: 2s, 5s, 15s; then failed
Attachment upload 2 attempts: 5s; then manual retry
API GET 1 retry; then error state
API POST/PUT/DELETE No auto-retry

21.2 Auth Layer

Error UI
401 Redirect to index.html; preserve route
session.revoked Clear session and local data; redirect
account.disabled Clear session; navigate to Auth Gate
account.deleted Same with different copy
Recovery-triggered session revocation Same as session.revoked

21.3 Server Errors

Code UI
400 "Something went wrong."
403 "You don't have permission."
404 Context-specific
409 Context-specific
413 "This file is too large."
415 "This file type isn't supported."
429 Toast with countdown
5xx "The server had a problem."

21.4 Crypto Errors

Error Behavior
MLS decrypt failure Mark "cannot decrypt"
Key commitment mismatch Reject; warning
Safety number changed Warning banner
Key transparency failure Unverified
OPRF failure Retry; app works without display-name decryption until it succeeds

21.5 Media Errors

Transcode failure → passthrough. Upload failure → retry. Download failure (corrupt) → error. Presigned URL expired → fetch new URL.

21.6 Storage Errors

Quota warnings at 80%, blocks at 95%. Write failure → in-memory mode with banner. OPFS unavailable → fallback.

21.7 MLS Errors

Out-of-sync epoch → "Syncing encryption state…". Missing welcome → poll every 5s; after 5min, "Waiting for a room member to add you". Pending MLS remove → room marked "left". Keystore corruption → wipe, mark rooms error, offer rejoin.

21.8 Hangout Errors

Error Behavior
403 not_a_participant (roster fetch) Do not fetch roster before joining. UI bug if it occurs.
Join failure (501 feature disabled) "Hangouts are disabled on this server."
Join failure (503 at capacity) "This hangout is full."
Heartbeat timeout (3 consecutive) Show "Disconnected" state. Offer rejoin. Do not auto-rejoin.
hangout.deleted received Tear down media. Toast: "This hangout was deleted."
Signal delivery to wrong client Filter locally via target_client_id.
Signaling failure (WebRTC) "Couldn't establish a connection. Try again."
Rate limited on heartbeat Log locally. Do not retry aggressively.

21.9 Read-Aloud Errors

Error Behavior
TTS model download failure Toast: "Couldn't download voice. Tap to retry."
Synthesis failure Toast: "Couldn't read this message."
Bad transcript detected Suggestion UI (see §11.9)

21.10 Interaction Edge Cases

Scenario Behavior
Edit, then unsend Tombstone replaces the latest version
Unsend, then attempt to edit Edit option absent
Disappearing message edited Edit inherits expires_at
Reply to expiring message [message expired] after expiration
Message during MLS transition Composer "Securing…"; send queued
Same file in two rooms Stored separately (encryption binds to room_id)
Member list without cached display name user_id placeholder until first message
Star a message, then it's deleted Star persists locally as a standalone copy. Not shown in thread. Shown in Starred list.
Nickname set, then member leaves Nickname retained in local store
Read-aloud interrupted mid-message Position saved; resumes on next tap
Two hangouts active in the same room Both show in strip; both in list; both counted in header badge

---

22. Plugin Inventory

Plugin Options Purpose
state-plugin initialState Global keyed pub/sub
router-plugin — History API + query-param routing
socket-plugin socketUrl, apiBase, appKey WebSocket connection
crypto-plugin wasmPath, thresholds MLS wrapper
oprf-plugin apiBase OPRF blinding + token derivation
media-plugin presets, workerPoolSize Transcode, thumbnail, strip
transcription-plugin STT models config Local speech-to-text
speech-plugin TTS models config Local text-to-speech (Supertonic)
push-plugin vapidKey, apnsConfig, fcmConfig Push registration
storage-plugin dbName, blobRoot SQLite + blob filesystem
i18n-plugin locales, defaultLocale Translation
theme-plugin presets, accent Theme application
call-plugin iceServers, transport Call state machine
hangout-plugin iceServers, heartbeatInterval Hangout state + signaling
p2p-plugin iceServers, chunkSize WebRTC data channels
icon-plugin styleMap, registry Solar icon resolution
notification-plugin defaultSound In-app banners
search-plugin threshold, debounceMs Fuse.js lifecycle

No event bus. Cross-component communication uses emit and $state key subscriptions.

---

23. Appendices

23.1 SQLite Schema

Domain Tables
Meta _migrations, _meta (includes last_user_seq, oprf_token_cache_key)
Identity users (cached display names keyed by user_id)
Rooms rooms, room_members, room_order
Messages messages, message_versions, reactions
Attachments attachments
User state read_state, drafts, outbox, blocked_users, room_preferences, device_names, starred_items
Personalization wallpapers, sticker_packs, custom_stickers, blocked_sticker_packs, nicknames
Hangouts hangouts (metadata cache)
Speech speech_voices
Transcription transcription_modes
Calls calls, call_participants
Sync sync_state (per-room epoch/seq), processed_events
MLS mls_rooms
Settings settings

Key new tables:

```sql
CREATE TABLE starred_items (
    user_id       TEXT NOT NULL,
    item_id       TEXT NOT NULL,
    item_type     TEXT NOT NULL CHECK(item_type IN ('attachment', 'message', 'link')),
    room_id       TEXT,
    starred_at    INTEGER NOT NULL,
    deleted_at    INTEGER,
    PRIMARY KEY (user_id, item_id, item_type)
);

CREATE INDEX idx_starred_user ON starred_items(user_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_starred_room ON starred_items(room_id) WHERE room_id IS NOT NULL;

CREATE TABLE nicknames (
    room_id     TEXT NOT NULL,
    user_id     TEXT NOT NULL,
    nickname    TEXT NOT NULL,
    updated_at  INTEGER NOT NULL,
    PRIMARY KEY (room_id, user_id)
);

CREATE TABLE hangouts (
    hangout_id    TEXT PRIMARY KEY,
    room_id       TEXT NOT NULL,
    metadata_blob BLOB,               -- decrypted
    position      INTEGER NOT NULL,
    updated_at    INTEGER NOT NULL
);

CREATE INDEX idx_hangouts_room ON hangouts(room_id, position);

CREATE TABLE speech_voices (
    voice_id      TEXT PRIMARY KEY,
    language      TEXT NOT NULL,
    size_bytes    INTEGER NOT NULL,
    downloaded_at INTEGER
);

CREATE TABLE transcription_modes (
    mode_id       TEXT PRIMARY KEY,
    label         TEXT NOT NULL,
    model_id      TEXT NOT NULL,
    size_bytes    INTEGER NOT NULL,
    languages     TEXT NOT NULL,
    downloaded_at INTEGER
);
```

Meta keys:

· _meta.last_user_seq — user-scoped sync cursor
· _meta.read_aloud_settings — JSON { mode, voice, speed, auto_read }
· _meta.read_aloud_room_overrides — JSON { [roomId]: 'on' | 'off' }

Principles: Flat columns for queryable fields; JSON blobs for opaque metadata. IDs TEXT. Timestamps INTEGER ms. Encrypted payloads BLOB. WAL mode. Forward-only migrations.

23.2 Design Tokens

Full token list in src/styles/tokens.css. Structure:

· Primitives (neutral, accent, semantic)
· Spacing (4px base)
· Typography
· Radii, shadows, motion
· Z-index, layout metrics, safe areas

Dark mode swaps the semantic layer under [data-theme="dark"].

23.3 Event Catalog

Room channel (private-room-{room_id}):

Event Payload
message.new { id, room_id, sender_user_id, sender_client_id, epoch, seq, content_type, created_at }
message.deleted { id, room_id }
message.edited { id, room_id, reply_to, editor_user_id, edited_at }
room.updated { room_id, metadata?, retention_days?, max_file_size_bytes? }
room.member_added { room_id, user_id, role }
room.member_removed { room_id, user_id }
epoch.updated { room_id, epoch, sequence }
mls.add_pending { room_id, target_user_id, client_ids }
read.count { room_id, message_id, read_by_count }
reaction.added { room_id, message_id, user_id, emoji }
reaction.removed { room_id, message_id, user_id, emoji }
hangout.created { room_id, hangout_id, created_by, created_at }
hangout.updated { room_id, hangout_id, metadata?, position? }
hangout.deleted { room_id, hangout_id }
hangout.occupancy { room_id, hangout_id, participant_count } (count only)

User channel (private-user-{user_id}):

Event Payload Durable
read.sync { room_id, last_read_message_id, user_seq } Yes
room_order.sync { order: string[], user_seq } Yes
device.added { device_id, user_seq } Yes
device.revoked { device_id, reason } Yes
device.name_updated { device_id, encrypted_name, user_seq } Yes
session.revoked { session_id, reason } Yes
account.disabled { reason } Yes
account.deleted {} Yes
mls.welcome_ready { room_id, welcome_id } No
room.transfer_initiated { room_id, from_user_id, transfer_id } Yes
room.transfer_cancelled { room_id, transfer_id, reason } Yes
starred_item.added { item_id, item_type, room_id, user_seq } Yes
starred_item.removed { item_id, user_seq } Yes
preference.updated { key, user_seq } (value not in event) Yes
hangout.signal { room_id, hangout_id, sender_user_id, target_client_id?, signal_type, payload } No

Client events (Sockudo-published, ephemeral):

Event Payload Purpose
client-typing.start { user_id } Typing indicator
client-typing.stop { user_id } Typing indicator
client-mls-request { room_id } Prompt pending MLS add

Read state writes go through REST (POST /users/me/read-state), not client events.

23.4 Client Events

See table above. No event bus; in-app events use emit.

23.5 Error Copy Reference

Consolidated error strings, grouped by layer.

23.6 Server Assumptions

The client implements against Server Specification v2.0 and its amendments (25–29). Blocking items: OPRF, encrypted display names, user-scoped sync, pending_mls_adds, recovery flow, key transparency, call signaling, message editing/reactions/threading, hangouts, model hosting (STT+TTS), starred items, generic preferences.

Breaking-change coordination: OPRF + user-scoped sync + device model changes ship as one release. No partial rollout.

---

24. Design Decisions Log

This section records significant decisions made during the drafting of Client Specification v1.0. It is not a version history; no prior version exists.

# Date Decision
1 2026-09-29 OPRF-based username lookup. Encrypted display names.
2 2026-09-29 Server-generated recovery codes. Separate recovery flow with OPAQUE re-registration.
3 2026-09-29 Device names synced via user-scoped state, encrypted with OPRF-derived key.
4 2026-09-29 Multi-device read state via POST /users/me/read-state and user-scoped sync.
5 2026-09-30 Hangouts adopted (server Amendment 25). Room Metadata Key formalized.
6 2026-09-30 Media grid: combined photos/videos, date-grouped, inline video preview.
7 2026-09-30 No Music tab. Voice messages not curated. Audio files in Docs.
8 2026-09-30 Link detail view defined. "View in chat" replaces "Jump to chat." Document detail view defined.
9 2026-09-30 List-detail navigation (horizontal swipe on mobile, hover arrows on desktop).
10 2026-09-30 Room settings: disclosure sections, icon-circle rows, E2EE badge, X on mobile.
11 2026-09-30 Header consolidation: video icon removed, headphones icon added for hangouts.
12 2026-09-30 Read-aloud (TTS) with Supertonic. Composer toggle, sentence-level control, auto-resume.
13 2026-09-30 Inline audio player with transcribe button (two zones). User-facing mode labels.
14 2026-09-30 Bad transcript detection with suggestion UI.
15 2026-09-30 Native keyboard sticker limitation documented. Paste-as-sticker fallback.
16 2026-09-30 Starred items synced across devices (server Amendment 27).
17 2026-09-30 Nicknames synced via user preferences (server Amendment 28).
18 2026-09-30 Read-aloud settings synced via user preferences (server Amendment 28).
19 2026-09-30 TTS model hosting added to capabilities (server Amendment 26).

---

25. Document Status

This is the contract for the client side of the system. Every client implementation task references this document. If a task conflicts with this spec, the task is wrong and must be revised.

Design decisions are tracked in §24.

