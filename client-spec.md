Client Specification v1.0

Status: Final — source of truth for client implementation tasks.
Server basis: Server Specification v2.0 including Amendments 25–38.
Stack: Coralite 1.0.0-rc.5 · Wire CoreCrypto 10.5.2 · Mediabunny · Supertonic · Moonshine / Whisper · Capacitor · Tauri
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
· Extension system — rail/list/detail surfaces, panel and overlay presentation, slots, events, sessions, preferences, storage, fetch (see §26)
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
· Per-user nicknames
· Starred items — synced across devices
· Biometric unlock (Face ID, Touch ID, Windows Hello)
· Recovery code flow (server-generated, server-hashed)
· Device linking for new devices
· Multi-device read state sync
· Key transparency automatic verification
· Sessions — persistent, room-scoped, extension-owned real-time spaces (voice, watch-together, games, and future types)
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
· Extension asset pipeline — locales, static assets, WASM modules

2.2 Server Requirements (Client V1 Dependencies)

Provided by Server Specification v2.0 and amendments:

· OPRF-based username lookup (Amendments 13, 30, 31, 32, 33)
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
· Recovery flow (POST /auth/recover/start + /finish)
· Schema naming cleanup
· Sessions — generalized from hangouts (Amendment 35)
· Model hosting for STT and TTS (Amendment 26)
· starred_items table + sync (Amendment 27)
· Generic user preferences endpoint (Amendment 28)
· Extension proxy endpoint — POST /extensions/proxy (Amendment 38)

2.3 Out of Scope

· SPA routing between pages (each page is a static HTML document)
· Contact/friend list (rooms only)
· Server-side key escrow
· Plaintext export of message contents
· Username changes
· Federation between servers (ActivityPub is not used)
· Multi-tenancy
· Multiple accounts on one device
· Analytics or crash reporting
· Message forwarding
· Message pinning
· Chat screenshot export
· GIF search (external API)
· User status / away message
· Ambient presence — sessions are the only co-presence feature
· Office document inline rendering
· Native keyboard sticker integration (platform limitation; watchlisted)
· Server-side extension behavior (extensions are client-only)
· Runtime extension installation (build-time only in v1)
· Extension streaming through the proxy (use direct fetch)

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
9. Chrome is fixed, content is free — the shell owns headers, actions, overlay frames, and settings chrome. Extensions own only what is inside the content area (§26).

---

4. Architecture

4.1 Pages

Two static pages built by Coralite:

Page File Purpose
Auth Gate src/pages/index.html Login, registration, recovery.
Messenger src/pages/app.html Messenger shell. In-page navigation only.

Navigation from the auth gate to the messenger is a full page load (window.location.href = '/app.html'). No client-side routing between pages.

4.2 In-Page Routing

Within app.html, a router plugin manages view state via query parameters.

URL format:

```
/app.html?rail=<extensionId>&detail=<route>&id=<selectionId>
```

Param Optional Meaning
rail yes Active rail extension. Derived from detail when absent.
detail yes Active detail route. Absent for list-only views.
id yes Selection within the detail (room, message, media item).

Secondary params (e.g., messageId, sentenceIndex) are documented per extension.

Examples:

```
?rail=media                              media list, no detail
?detail=media-viewer&id=a3f9             media list + viewer (rail derived)
?detail=chat&id=r_abc123&messageId=m_xyz chat list + thread
?rail=chat&detail=hangout&id=h_abc       chat rail, session overlay
?rail=chat                               chat list only
```

Route ownership: Every route is owned by exactly one extension. extensions.ownerOfRoute(route) resolves it. Duplicate routes are a build error (§26.13).

Back stack: The browser history is the stack. Every navigation calls history.pushState. Every dismissal calls history.back(). popstate re-runs route resolution.

Deep links (from push notifications, invite links, OAuth callbacks):

```
https://app.example.com/app.html?rail=chat&detail=chat&id=r_abc123&messageId=m_xyz789
```

4.3 Directory Structure

```
atoll/                              monorepo root
├── packages/
│   ├── extend/                     @atoll/extend — extension SDK
│   │   ├── src/
│   │   │   ├── define-extension.js
│   │   │   ├── validate.js
│   │   │   ├── ctx.js
│   │   │   └── index.js
│   │   └── package.json
│   └── app/                        the SPA
│       ├── coralite.config.js
│       ├── capacitor.config.ts
│       ├── src-tauri/
│       ├── src/
│       │   ├── pages/
│       │   │   ├── index.html
│       │   │   └── app.html
│       │   ├── components/
│       │   │   ├── primitives/       # ui-*
│       │   │   ├── composed/         # profiles, bubbles, rows, cards
│       │   │   ├── containers/       # list/thread composers
│       │   │   └── shell/            # messenger-shell, hosts
│       │   ├── extensions/           # first-party extensions
│       │   │   ├── chat/
│       │   │   ├── media/
│       │   │   ├── documents/
│       │   │   ├── links/
│       │   │   ├── calls/
│       │   │   ├── settings/
│       │   │   ├── hangouts/         # voice session type
│       │   │   ├── profile/
│       │   │   ├── join/
│       │   │   └── admin/
│       │   ├── plugins/              # factory → definePlugin
│       │   ├── lib/
│       │   │   ├── mls/              # CoreCrypto wrapper
│       │   │   ├── crypto/           # C2SP, HKDF, Web Crypto, content-key
│       │   │   ├── oprf/             # OPRF blinding
│       │   │   ├── media/            # Mediabunny pipeline
│       │   │   ├── transcription/    # STT model loading + inference
│       │   │   ├── speech/           # TTS model loading + synthesis
│       │   │   ├── sessions/         # session lifecycle + signaling
│       │   │   ├── api/              # Fetch wrapper
│       │   │   ├── p2p/              # WebRTC data channels
│       │   │   ├── db/               # SQLite repositories
│       │   │   ├── blobs/            # Blob filesystem
│       │   │   ├── actions/          # Orchestration functions
│       │   │   ├── events/           # Socket event handlers
│       │   │   ├── validation/
│       │   │   └── utils/
│       │   ├── styles/
│       │   │   ├── tokens.css
│       │   │   └── utilities.css
│       │   └── assets/
│       ├── migrations/
│       ├── package.json
│       └── tailwind.config.js
├── pnpm-workspace.yaml
└── package.json
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
import extensionPlugin from '@atoll/extend/plugin'
import statePlugin from './src/plugins/state-plugin.js'
import routerPlugin from './src/plugins/router-plugin.js'
// ... other plugins

import chat from './src/extensions/chat/index.js'
import media from './src/extensions/media/index.js'
import documents from './src/extensions/documents/index.js'
import links from './src/extensions/links/index.js'
import calls from './src/extensions/calls/index.js'
import settings from './src/extensions/settings/index.js'
import hangouts from './src/extensions/hangouts/index.js'
import profile from './src/extensions/profile/index.js'
import join from './src/extensions/join/index.js'
import admin from './src/extensions/admin/index.js'

export default defineConfig({
  plugins: [
    // Extension system — first-party extensions
    extensionPlugin({ extensions: [
      chat, media, documents, links, calls, settings,
      hangouts, profile, join, admin
    ]}),

    // Shell and infrastructure
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

· Phase 1 — (pluginContext) => ... runs once per plugin per app load. Expensive setup. Can be async.
· Phase 2 — (instanceContext) => ({ ... }) runs per component instance. Cheap setup.

Plugin name → context key. A plugin named 'router' becomes client({ router }).

Cross-plugin access. Plugins access each other via instanceContext.<name>.

Global-singleton guard. Expensive one-time work uses pluginContext.__<name>_active__ for idempotency.

Asset pipeline. Assets are registered in the assets array with either { pkg, path, dest } or { src, dest } form. Coralite tracks changes and rebuilds only modified assets.

Extension system is not a Coralite concept. @atoll/extend is an Atoll package that produces Coralite plugins. Coralite does not depend on Atoll; Atoll depends on Coralite. See §26.1.

4.5 State Management

Global state is managed by the state plugin. Keyed pub/sub store.

```js
export default defineComponent({
  client({ globalStore }) {
    const { $state } = globalStore
    const rooms = $state.rooms
    $state.selectedRoomId = 'r_abc123'
    $state.subscribe('rooms', (rooms) => { /* re-render */ })
  }
})
```

Shell state keys:

Key Shape
currentUser User \| null
isAuthenticated boolean
oprfToken Uint8Array \| null (in memory only)
capabilities Capabilities
rooms Record<roomId, Room>
roomOrder string[]
selectedRoomId string \| null
unreadCounts Record<roomId, number>
activeCall Call \| null
callHistory Call[]
readAloudMode boolean
readAloudState { playing, messageId, sentenceIndex }
settings Record<string, unknown>
ui { activeRail, modal, toasts, offline }
pendingScrollToMessage string \| null

Extension state is separate. Each extension has its own writable state slice, provided as ctx.state. Extensions do not write to $state. They read $state (read-only) and write only their own slice. See §26.10.

Source of truth: SQLite. The store is an in-memory view.

4.6 Cross-Component Communication

Relationship Mechanism
Child → parent emit(name, detail) — DOM-scoped CustomEvent, bubbles to ancestor
Sibling / cross-tree $state key change
Global signal $state key observed by target
Extension → extension publicEvents with schemas (§26.7)
Extension → shell ctx methods (navigate, present, dismiss, toast, notify)
Shell → extension Slots mount the extension's component; public events invoke the extension's listeners

No global event bus. Cross-extension communication is declarative and validated.

4.7 Boot Sequence

```
1. index.html renders (auth gate)
2. app.html loads
3. Coralite shell renders
4. state-plugin initializes
5. router-plugin initializes (parses query params)
6. extension-plugin registers all extensions
   └─ Build-time validation already passed; registry is populated
7. Read session token from secure storage
   └─ Invalid → redirect to index.html
8. Open SQLite, run migrations
9. Hydrate from SQLite:
   ├─ Rooms, members, room order
   ├─ Unread counts, drafts, preferences
   ├─ Cached display names, device names, nicknames
   ├─ Session metadata per room
   ├─ Starred items
   ├─ Cached read-aloud settings
   ├─ Extension storage (namespaced)
   ├─ Extension preferences (mirrored locally)
   └─ Sync cursors (per-room + user-scoped)
10. User-scoped state sync:
    GET /users/me/sync?since_seq=<last_user_seq>
    Apply read_state, user_preferences, device_state, starred_items
    Store new max_seq
11. CoreCrypto initialization
12. OPRF token derivation
13. Render UI with hydrated data
14. Fetch capabilities (GET /api/v1/capabilities)
    └─ Read extension_proxy_enabled, sessions_enabled, session_types[]
15. Connect WebSocket (pusher-js)
16. Subscribe to private-user-{user_id} and all private-room-{room_id}
17. Delta sync per room (since cursor)
18. Background: key package replenishment, welcome polling,
    pending removes, thumbnail generation, TTS/STT model idle prefetch
```

Failure modes:

Step Failure Behavior
6 Extension registration fails Build would have caught it; if runtime, render error, other extensions unaffected
7 Corrupt token Clear storage, redirect
8–9 SQLite unavailable Degradation notice; memory-only mode
10 Sync fetch fails Retry; app works from local state
11 Keystore corrupted Wipe, re-init; rooms marked mls_status='error'
12 OPRF fails Retry; app works without display-name decryption
15 Socket fails Retry with backoff; app remains usable from cache

4.8 Sync & Offline

Two cursors:

· Per-room (epoch, seq) for message sync. Room events are best-effort WebSocket; REST delta sync is authoritative.
· Per-user last_user_seq for user-scoped state. User events are durable; catch-up via GET /users/me/sync.

Readable offline: All cached messages, media, metadata, nicknames, starred items, extension storage.

Outbox queue: Messages composed while offline persist in SQLite. On reconnect, processed FIFO. On failure after retries, marked failed with Retry/Delete actions.

Read state while offline: Advances locally; debounced flush to POST /users/me/read-state (at most one POST per room per 30 seconds, plus immediate flush on room switch and app background).

No global offline banner. Feedback is inline on messages and via one-shot toasts.

4.9 Component Authoring Rules

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
Session session-*
Extension (third-party) x-<slug>-<component>

5.2 CSS Strategy

Layer Tool Scope
Design tokens CSS custom properties Global
Global utilities Tailwind Entire app
Component styles Coralite scoped <style> Per-component

5.3 Theming

Global theme: light / dark / auto. Token swap under [data-theme="dark"].

Accent palette — locked (2026-10-02):

Token Light mode Dark mode
--accent-500 #2FB6AA Lagoon Blue #4DD0C4 (brightened)
--accent-700 #297370 Dark Aquamarine #297370 (surface fill)
--accent-warm-500 #EC7562 Coral Reef #EC7562 (holds)
--neutral-warm-50 #F9F1E4 Papaya Whip —
--neutral-warm-200 #E0C0A9 Beige Skin —

Elevation tokens (both modes):

Token Role
--surface-0 App background, thread background
--surface-1 Incoming bubble, rail, list rows
--surface-2 Composer, overlay frame, cards
--surface-3 Popovers, tooltips, hover states

Semantic mapping: Components reference semantic tokens (--bubble-incoming, --text-primary), never ramp tokens directly. Semantic tokens remap under [data-theme="dark"].

Contrast rules:

· Teal fill on light backgrounds uses dark text (--neutral-900), not white.
· Accent-color text on light backgrounds uses --accent-700 or darker.
· Destructive actions use a distinct red family, not coral.
· Coral is reserved for reactions, mentions, and warm emphasis.

Per-room, per-user theming: Room container overrides a subset of tokens. Stored in room_preferences (local), synced via user preferences.

5.4 Icons

Solar icon set, tree-shaken via @solar-icons/static. No sprite. The icon-plugin resolves a canonical name and state to a concrete icon.

Extensions may only use canonical names from the Solar set. Extension authors cannot ship their own SVGs. The vocabulary command lists every available icon name.

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

The rail is registry-driven. It renders every extension with a rail declaration, sorted by rail.order. There is no hard-coded rail list. See §26.5.

Explorer pattern. Selecting an item in Panel 2 highlights it and renders detail in Panel 3. Overlays cover both panels without changing the rail.

6.2 Tablet (768–1023px)

Rail collapses to bottom nav. Panels 2 and 3 remain side by side.

6.3 Mobile (< 768px)

Single column. Bottom nav capped at 5 slots. Core extensions claim the fixed slots by default. Third-party extensions default to mobile.panel1.placement: 'more' and open in a More sheet.

Media tab hosts a segmented control: Media / Docs / Links.

The Music tab is not present. Voice messages are not curated. Shared audio files appear in Docs.

6.4 Floating Glass Chrome

Chat header and composer are absolute overlays with backdrop-filter: blur(20px) saturate(180%). The thread scrolls underneath.

Dynamic offsets. Composer height tracked via ResizeObserver. Header includes env(safe-area-inset-top); composer includes env(safe-area-inset-bottom).

Scroll edge cue. A data-scrolled attribute adds a border when content is behind the glass.

Fallback. @supports not (backdrop-filter: blur(20px)) and a low-end device check disable the glass effect.

Dark mode caveat: Saturation boost may amplify teal in content behind glass. Saturation values may differ per theme.

6.5 Safe Areas

Edge-to-edge rendering. Safe area insets provided by:

· Capacitor: capacitor-edge-to-edge or @aashu-dubey/capacitor-statusbar-safe-area
· Tauri: tauri-plugin-safe-area-insets-css
· Web: env(safe-area-inset-*)

Root viewport height uses 100dvh.

6.6 Navigation Patterns

Affordance Purpose
Back arrow (←) Nested navigation (mobile). Pops the history stack.
Close (✕) Modal dismissal. Pops the history stack.
OK / Cancel Forced decisions

Platform split:

· Desktop: Explorer pattern. Detail renders in Panel 3. Overlays appear as centered sheets (settings) or off-canvas right drawers (room settings) with no scrim.
· Mobile: Full-screen stack. Overlays are full-screen sheets or full-screen modals. Dismissal routes through history.back().

Action placement:

Position Reserved for
Top-left Navigation (back arrow or X)
Top-center Title
Top-right Contextual actions (declared in the surface's actions array)

List-detail navigation. Detail views opened from a list support horizontal navigation through the list:

· Mobile: horizontal swipe with scroll-snap.
· Desktop: hover arrows, keyboard ←/→.
· Gesture direction locking (8–10px threshold).

6.7 Header Consolidation

The video icon is not present in the room header. Video is selected from the pre-call preview screen.

Platform header layout:

Platform Layout
Mobile [←] [name] [phone] [headphones] [gear]
Desktop [name] [headphones] [search] [phone] [⋯]

Session icon: headphones glyph (Solar headphones). Not a megaphone.

State Treatment
No sessions No badge
Sessions exist, none active Subtle dot
Sessions active Count badge (total participants)

Room settings on desktop: moves into [⋯] overflow. On mobile: stays as gear icon.

Headers are host-drawn. Extensions declare the title and actions; the shell renders the header. See §26.5.

---

7. Component Inventory

Component tags correspond to <template id="..."> values inside .html files. Components are ambient — registered globally, referenced by tag name, no imports required.

7.1 Primitives

Component Responsibility Key attributes
ui-profile User/group profile size, shape, src, state, fallback
ui-profile-group Composite of 2–4 profiles max size
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
message-audio-player Audio player with transcribe + read-aloud
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
session-card Session entry in a list
session-strip Active session bar above thread
session-bar In-session control bar
session-participant Participant in roster

7.3 Containers

Component Responsibility
conversation-list Virtualized list (reads $state.rooms)
message-thread Virtualized message list
message-composer Composer with read-aloud toggle
media-grid 3-column grid, photos + videos, date-grouped
document-list Virtualized list group
link-list Virtualized list group
settings-list Settings rows
auth-form Auth gate form
media-viewer Full-screen slideshow with list-detail navigation
link-detail Link detail view
document-detail Document detail view
session-list Sessions in a room
session-roster Participants in a session
session-create Create/edit session sheet
session-signal-handler WebRTC signaling client
call-participant-grid Call layout
message-reply-preview Reply bar
sticker-picker Sticker sheet
read-aloud-controls Playback controls for TTS

7.4 Shell

Component Responsibility
messenger-shell Root layout
rail-host Registry-driven rail
surface-host Mounts list and detail surfaces per extension
overlay-host Overlay stack renderer
modal-host Modal renderer
toast-host Toast queue
notification-banner-host In-app banners
call-overlay-host Call overlays
error-boundary Per-surface error fallback

7.5 Views

Views are extension-owned. Each view's routes, titles, and actions are declared in its extension object (§26.3). The mapping:

Extension Routes
core.chat chats (list), chat (detail), room-settings (overlay)
core.media media (list), media-viewer (detail)
core.documents documents (list), document (detail)
core.links links (list), link (detail)
core.calls calls (list), call (detail)
core.settings settings (list), settings-section (detail)
core.sessions sessions (list), session (detail)
core.profile profile (detail, no rail)
core.join join (detail, no rail)
core.admin admin (list, no rail, gated on capabilities.admin)

---

8. Crypto & E2EE

8.1 Three Encryption Layers

Layer Purpose Implementation
MLS Message encryption, group state, key packages, welcomes, commits Wire CoreCrypto 10.5.2 (WASM, bundled)
C2SP chunked AES-256-GCM Attachment blob encryption, range-based decryption Web Crypto API
OPRF token Username lookup, display-name encryption, device-name encryption, RMK delivery Client-side OPRF blinding

8.2 Room Metadata Key (RMK)

Property Value
Length 32 bytes (AES-256)
Generation Random, at room creation, by the creator's client
Distribution Delivered via the MLS Welcome GroupInfo extension
Rotation None — static for the room's lifetime
Domain separation HKDF sub-keys per purpose

```
room_metadata_key    = HKDF(RMK, "room-metadata-v1",    32)
session_metadata_key = HKDF(RMK, "session-metadata-v1", 32)
```

8.3 OPRF-Based Lookup

Purpose: The server must not hold plaintext usernames or display names.

```
Given: username, server key k, hash-to-curve H

Client:
  1. h         = H(username)
  2. r         = random blinding factor
  3. blinded   = h * r
  4. sends { blinded } to server

Server:
  5. blinded_oprf = blinded ^ k
  6. returns { blinded_oprf }

Client:
  7. token = OprfClient::finalize(username, blinded_oprf)
```

Token format (Amendment 31): 64-byte SHA-512 output of voprf::OprfClient::finalize(), encoded as an 86-character unpadded base64url string. Not the 32-byte group element h^k.

Key derivation (Amendment 33):

```
display_name_key = HKDF(token, "display-name-encryption-v1", 32)
device_name_key  = HKDF(token, "device-name-encryption-v1",  32)
```

Bounds (Amendment 33):

· username_token: exactly 86 chars, base64url, decodes to 64 bytes.
· encrypted_display: decodes to 28–284 bytes.
· display_name_key ciphertext = 12-byte nonce || AES-256-GCM ciphertext.

Client caches the token in memory for the session. Never stored at rest. Re-derived on login.

Token stability: k is immutable for the account lifetime. The token is stable across devices and sessions.

8.4 C2SP Attachment Encryption

Summary:

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

Purpose values:

Purpose Use
"message" Regular message attachments
"room-avatar" Room avatars
"user-avatar" User profile images
"session-icon" Session icons
"sticker" Custom stickers

Padding: Plaintext padded to the nearest ATTACHMENT_BUCKET_SIZES bucket before encryption.

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
2. entropy_seed = HKDF-Expand(device_secret, info="mls-entropy-v1", length=32)
3. Open the encrypted keystore
4. CoreCrypto.init(client_id, keystore, entropy_seed)
5. CoreCrypto.mlsInit()
```

Key package lifecycle: Target 20 unconsumed per device. Low watermark 5. Replenish on login and on app launch. Rotate every 30 days.

Group lifecycle:

· Create: mlsCreateConversation(room_id), batch-add members in a single commit.
· Join: Poll for welcome (GET /welcomes), mlsProcessWelcomeMessage, consume, fetch messages from welcome epoch.
· Leave: Server queues pending_mls_remove; another online member generates the Remove commit; leaving client deletes local state.

Epoch transitions: Composer enters "Securing…" state. Outgoing messages queued and flushed once the new epoch settles. Concurrent commits handled by CoreCrypto's deterministic tie-break.

8.6 Key Transparency

1. Fetch identity keys with tree_head, auditor_signatures, inclusion_proof.
2. Verify the server signature, each auditor signature, and the inclusion proof locally.
3. Mark the contact as transparency-verified on success.
4. Display a green checkmark.

Failure cases:

Case Display
Inclusion proof fails Warning; not verified
Auditor signatures missing "Verified by server only"
Auditor signatures invalid Warning; unverified
Network unavailable "Verification pending"

8.7 Safety Numbers

Safety numbers computed locally from MLS identity keys. Displayed in the profile view and room profile. Verification is soft; SAFETY_NUMBER_MODE is surfaced.

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

Mediabunny handles demux, decode, encode, resize, rotate, crop, thumbnail extraction. GPU path uses WebCodecs. CPU fallback is ffmpeg.wasm, loaded lazily.

Worker pool: Concurrency cap of 2 (1 on low-end devices). Jobs above 500 MB processed alone. Each job exposes an AbortController.

Failed transcode: Fall back to passthrough — file uploaded unprocessed as a generic attachment.

9.3 Thumbnail and Poster Generation

128×128 WebP thumbnail generated alongside the full attachment. Encrypted separately (same key/salt/context), uploaded as companion blob. Manifest carries thumbnail_file_id.

For video, poster frame extracted for thumbnail strip and preview.

9.4 Padding

1. Select smallest bucket T containing plaintext.
2. Compute L_padded from T.
3. Append zero bytes to reach L_padded.
4. Chunk padded plaintext.
5. Encrypt each chunk.

9.5 Metadata Stripping

All processed media stripped of EXIF, GPS, device info, XMP, IPTC, ICC, ID3. Documents are not modified.

9.6 Fast-Start MP4

For MP4 files, validate moov atom is at start. Files where moov follows mdat are re-muxed with mp4box.js. If re-muxing fails, transcode to WebM/VP9.

9.7 Progress and Cancel

Per-file progress reported inline in the message thread:

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

Presigned URLs used when capabilities.storage_presign_supported is true. URLs not cached; on expiry, fresh URL requested and download resumes.

10.3 P2P Fallback

When a file exceeds effective_max_file_size_bytes for the room:

1. Member picker modal appears.
2. WebRTC data channels established (one per recipient).
3. File transfers directly, encrypted with an ephemeral key per recipient.
4. Chunk-level acknowledgment enables resume.
5. Cancel discards partial data on both sides.

Online-only.

10.4 Deletion

Operation Scope Permission
Delete for me Local only Anyone
Unsend All members Sender, within 24h
Remove from room All members Owner (or moderator in Discord mode)

Blob deletion triggered by unsend/remove. Server prunes when no references remain.

10.5 Attachment Icon Mapping

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
Sender name N/A Above the bubble on first message of a group
Bubble color var(--bubble-outgoing) var(--bubble-incoming)
Timestamp Inside bubble, bottom-right Inside bubble, bottom-left
Read receipt "Read" / "Read N" next to timestamp —

Bubble max width: min(75%, 600px) on desktop. Corner radius: 18px default; 4px on tail corner and inner group corners.

11.2 Sending States

State Icon
Queued Clock
Sending Diagonal upward-left arrow
Sent No icon (timestamp only)
Read "Read" / "Read N"
Failed Circular arrow; tap to retry

11.3 Message Grouping

Consecutive messages from the same sender within 5 minutes grouped: avatar and sender name on first message; timestamp on last; inner corners tighten.

11.4 Date Separators

Centered pill-style separators on calendar day change: "Today", "Yesterday", "March 5, 2026". Sticky date header floats at the top of the viewport.

11.5 New Messages Divider

Horizontal line at the last-read position. Shown when opening a room with unread messages; hidden once scrolled past.

11.6 Scroll Behavior

· On open: Scroll to bottom, instant.
· New incoming while pinned: Auto-follow if within 80px of the bottom.
· New incoming while scrolled up: Do not follow; increment unseen count.
· Own send: Always scroll to bottom.
· Scroll-to-bottom button: Floating, appears when scrolled up; shows unseen count.
· Infinite scroll: Trigger at ~200px from top.
· Prepend anchoring: anchorTo: 'end'.
· Stable keys: Message IDs.

11.7 Message Context Menu

Long-press (mobile) or right-click / hover bar (desktop) opens the message context menu.

Universal actions: Copy, Reply, React, Select, Delete for me, Read aloud.

Sender-only: Unsend (within 24h).

Conditional: Edit (own messages, within 15-minute window), Show original (if edited), Delete (Discord mode).

Hover bar (desktop only): compact strip with react, reply, edit, overflow.

Browser context menu suppression scoped to .chat-bubble elements only.

11.8 Message Editing

Affordance: Long-press → "Edit" (mobile); hover bar → edit icon (desktop).

Edit mode: Composer morphs — banner shows "Editing," input pre-filled, send button becomes checkmark.

Window: 15 minutes from original send time.

Edits: Unlimited within the window. Each edit is a new signed message with reply_to set and sequence incrementing.

Indicator: "Edited" label next to timestamp inside the bubble.

History: "Show original" opens a sheet showing the full edit chain.

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

In-thread display: message-audio-player with wavesurfer.js.

Transcribed state adds the transcript below the waveform, with a mode label ("Fast · Transcribed").

Transcription mode picker:

Label Model Description
Fast Moonshine Tiny Best for English. Quickest.
Accurate Moonshine Small Best for English. Slower, more precise.
Multilingual Whisper Tiny Supports 99 languages.

Bad transcript detection: Empty, known silence markers, under 3 words for audio > 10s, under 5% of estimated word count. Triggers suggestion UI.

Peaks: Compact float array (~100–200 values) included in payload.

Max duration: 30 minutes.

11.10 Read-Aloud (TTS)

Mode toggle in composer (grey off, accent-filled on). Long-press opens options sheet (speed, voice, "Read from here"). Persists across sessions.

Applies to: Text messages, voice transcripts, captions, messages with attachments and no caption (reads filename), link previews (title + description), system messages, tombstones. Not stickers or images without captions.

Sentence-level control:

· Messages pre-segmented into sentences client-side.
· Each sentence is a tap target.
· Tapping a sentence starts/jumps to it.
· Currently-speaking sentence has subtle highlight.

Auto-resume: Position stored per-message. Cleared on message completion, "Restart," message edit, 24-hour expiry.

TTS model: Supertonic 3 (~400 MB ONNX). 31 languages. WebGPU + WASM. 100% local. Downloaded on first use. Cached via Cache API. Never calls an external CDN.

Screen reader coexistence: Detect OS screen reader; offer to disable our read-aloud.

Keyboard shortcut: Ctrl/Cmd + Shift + R toggles mode.

11.11 Stickers and Emoji

Emoji picker: emoji-picker-element.

Stickers:

Tier Source Delivery
Bundled Ships with the app Static assets
Shared pack Encrypted attachment Cached locally
Custom User-created Processed as media

Pack manifest: JSON with id, name, publisher, version, tray_image, stickers[].

Animated stickers: Lottie JSON via dotlottie-web with expressions disabled.

Entrance transitions: none, pop, drop, slide, crash, fade. Respects prefers-reduced-motion.

Sticker combination: Long-press a sticker → "Combine" → multi-select up to 6 → drag, resize, rotate → send. Rendered to a canvas and sent as an image attachment.

Native keyboard limitation: Documented. Paste-as-sticker fallback for small square images.

11.12 Link Previews

Generated client-side by the sender. Debounced (500ms). 3-second timeout. Fallback to minimal card with domain and URL on failure.

Multiple links: Only the first URL gets a preview.

Opt-in server proxy: For CORS-blocked URLs, an optional server proxy can fetch metadata. Uses agent-fetch for SSRF protection. No logging, encrypted URL relay.

11.13 Reply / Quote

· Trigger: Swipe right on mobile; hover bar or right-click → Reply on desktop.
· Preview bar above composer shows quoted sender and snippet.
· Quoted snippet tappable to jump to original.
· Payload: reply_to: "<message_id>".

11.14 @Mentions

Composer autocomplete: typing @ opens a popover listing room members. Prefix-based filtering. Selection required.

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

Rendering: Accent-color pill, tappable to profile. Mentions of current user get additional emphasis.

Notification override: Mentions override mute but not "Nothing."

@everyone / @here: Not in V1.

11.15 Markdown Formatting

Supported: bold, italic, strikethrough, inline code, links, bullet lists, numbered lists, block quotes.

11.16 Message Tombstone

Deleted message bubble replaced in place with muted pill: "This message was deleted."

11.17 In-Thread Search

· Entry point: Magnifying glass in the chat header.
· Scope: Current room only.
· Library: Fuse.js, lazy-loaded.
· Highlight color: #FFE066.
· Jump to message: Scroll + 2s flash + "Back to search".

11.18 Global Search

Search field in the chat list header. Searches room names and message content. Results group by room.

11.19 Disappearing Messages

Room-level timer stored in encrypted metadata JSON.

Options: Off, 24h, 7d, 90d. Who sets: Room owner only. Applies to: New messages only.

Timer start: At send time. Sender computes expires_at and includes it in the payload.

Cleanup: Local-only periodic task queries SELECT id FROM messages WHERE expires_at <= now.

Interactions: Edits inherit expires_at. Replies show [message expired] after expiration. Curation views reflect removal. Search indexes updated on delete.

11.20 Nicknames

Per-user aliases for room participants.

Where shown: Chat thread sender names, member list, reply preview, mention autocomplete, profile view.

Not shown: To other members (nicknames are user-scoped). On the user's own messages.

Edit UI: Room Settings → Customise chat → Edit nicknames.

Storage: Local in room_preferences; synced via user_preferences key nicknames:{room_id}.

Fallback: Member's display name.

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

Bar above the composer, part of the glass overlay. Composer height updates via ResizeObserver.

12.3 Attachment Preview

Between pick and send, a preview screen shows processed result with thumbnail, filename, size, quality override, caption input, remove, reorder.

12.4 Enter Key Behavior

Platform Enter Shift+Enter
Desktop Send Newline
Mobile Newline —
Tablet (physical keyboard) Send Newline

12.5 Focus Behavior

After sending, focus remains in the input. Virtual keyboard stays open on mobile.

12.6 Multi-Attachment Layout

Count Layout
1 image Full-width
2 images Two equal columns
3 images Left full-height; right has two stacked
4 images 2×2 grid
5+ images 2×2 grid with +N overlay

Files render as stacked cards below the image grid. Caption renders last.

12.7 Extension Slots on the Composer

core.chat declares these slots on its detail surface:

Slot Renders Multiple
chat.composer.above Strip between reply preview and composer Yes
chat.composer.attachMenu Items in the ＋ popover Yes
chat.composer.leading Element before the ＋ button No
chat.composer.trailing Element after the send/mic button No

chat.composer.attachMenu is the recommended integration point for extensions. Each filler renders a single row in the ＋ popover: icon, label, tap handler. The host provides row chrome; the extension supplies icon, label, and behavior.

Composer state is private. Extensions cannot read the draft, cursor position, or selection. Extensions ask the composer to do things via public events (§26.7):

· chat:insert-text — insert text at start / end / cursor
· chat:focus — focus the composer
· chat:set-draft — replace the draft

Extensions cannot send on the user's behalf.

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

Member list display: Member rows show profile image and display name (with nickname override).

Display name resolution chain:

1. Cached display name
2. Message payload (sender's display name inside MLS-encrypted messages)
3. encrypted_display from member list response (only if token cached for that user)
4. Fallback: user_id or "Member" placeholder

Practical consequence: Member lists show placeholders until a message is received from that member.

13.3 Invite Links and QR

· Room-specific invite tokens.
· Default expiration: 7 days.
· Display: QR (via qr-code-styling), copy link, OS share sheet.
· Regenerate invalidates current link and issues a new one.
· Scanning: camera (Capacitor) or paste (desktop).

13.4 Username Lookup

Phase 1 — User taps "Add member." Sheet opens with a single Username field.

Phase 2 — Lookup (invisible):

Step Where What happens
1 Client Blinds username with a random factor
2 Client → Server POST /oprf/blind
3 Server OPRF evaluation with k
4 Client Unblinds → token
5 Client → Server POST /users/lookup { username_token }
6 Server Matches token, returns { user_id, encrypted_display } (200) or { error: "not_found" } (404)
7 Client Derives display_name_key from token
8 Client Decrypts display name

Response semantics (Amendment 32): Found returns 200 with { user_id, encrypted_display }. Not-found returns 404 with { error: "not_found" }. Rate limiting is the enumeration defense; timing padding is not applied.

Phase 3 — Confirmation card. On success: profile image + display name + username + "Add to room" button. On failure: hint card. Fallback: "Invite via link instead."

Phase 4 — Adding to room:

Step What happens
1 POST /rooms/:id/members { user_id }
2 Server adds to room_members, inserts pending_mls_adds for each of B's devices
3 Server publishes mls.add_pending on room channel
4 Any online member generates Add commit + Welcome
5 Server publishes mls.welcome_ready to B
6 Server publishes room.member_added on room channel

System message: "You added Alex."

Phase 5 — Recipient side. Online: receives mls.welcome_ready, fetches welcome, processes, fetches messages from welcome epoch. Offline: push notification.

Phase 6 — Post-add. Display name cached locally.

13.5 Room Settings

Display pattern:

· Desktop (≥1024px): Off-canvas right drawer within Panel 3. Width min(480px, 40%). No backdrop scrim. Dismiss: X, Escape, click outside. Routable as ?rail=chat&detail=room-settings.
· Mobile: Full-screen modal with X.

Profile block: Room avatar, name, E2EE badge below.

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

1:1 rooms replace Members/Invites with "This conversation" (Block, Delete chat).

Icon circles on each row. Destructive actions require type-to-confirm. Kick and leave use single-confirmation dialogs.

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
3. Circular crop (1:1 aspect ratio).
4. Multi-size preview.
5. Upload as encrypted attachment; update metadata JSON; broadcast room.updated.

Constraints: 20 MB max source, 128×128 min, JPEG/PNG/WebP/AVIF/HEIC. Output 512×512 AVIF q80.

13.8 Room List Ordering

Default: Fixed by join date. New rooms append to the bottom.

Manual reordering: Activated from the room list header menu. Drag handle on each row. Keyboard alternative: Alt+↑ / Alt+↓. "Reset to default" reverts to join date.

Order persists locally and syncs across devices via PATCH /users/me/room-order.

Filters: Unread only, Mentions only, Muted rooms, Active sessions. Reorder mode is disabled while filters are active.

13.9 Conversation Row Content

Position Content
Left ui-profile (composite for groups)
Top line Room name (bold if unread) + optional session indicator
Bottom line Last message preview, or typing indicator, or draft
Top right Timestamp
Bottom right Unread badge or mute icon

Session indicator: small headphones icon + count inline next to the room name when the room has an active session.

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

Typing indicator replaces preview: "Alex is typing…"

13.10 Leaving and Deletion

· Leave room (all members): Confirmation; warns if last member.
· Delete room (owner): Type-to-confirm. Other members receive room.deleted, room removed locally, system message: "This room was deleted by [owner]."
· Rejoining after leaving: Fresh view from rejoin forward.

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

Layout: 3-column flush grid. Square thumbnails with object-fit: cover.

Video thumbnails: Centered play icon, duration badge bottom-right, poster frame from media pipeline.

Inline video preview:

· Desktop: hover with 250ms intent delay. Muted, looping, streaming.
· Mobile: long-press with 250ms hold. Muted, looping, streaming.
· prefers-reduced-motion: reduce disables preview entirely.
· Prefetch: first 2–3 chunks of visible videos during idle.

14.2 Documents and Links

· List groups using list-row. Grouped by date (newest first).
· Documents show file-type icon, filename, size, sender, date.
· Links show favicon, title, domain, sender, date.

14.3 Filter and Search

Filter chips hidden by default behind a filter icon in the header.

Filter sheet options: Chat, Date, Media type, Sender, Starred.

Active filters appear as dismissible pills in the header.

Search fields: filename, sender, chat (media); title, domain, sender, chat (links).

Threshold: 0.35. includeMatches: true for highlighting.

14.4 "View in Chat" Action

Opens ?rail=chat&detail=chat&id=:roomId&messageId=:msgId. Scrolls to the message, highlights for 2s.

14.5 Media Viewer

Scoping:

Opened from List
Chat thread All media in the current room, chronological
Curation view The currently filtered list

List-detail navigation: horizontal swipe on mobile, hover arrows + keyboard on desktop.

Scrolling: CSS Scroll Snap (scroll-snap-type: x mandatory).

Morph: View Transitions API.

Zoom and pan: Pinch, double-tap, scroll wheel. Min 1×, max 5×. Drag to pan when zoomed.

Swipe-to-dismiss: Swipe down dismisses at 1×; when zoomed, swipe down pans.

Action bar (top, glass chrome): Save, Share, Copy, View in chat, Delete (if owner), Info, Star.

Thumbnail strip: Bottom of viewer. Virtualized (±20 items). Two-way sync with main slideshow.

Prefetching: Active slide ±2 for decryption. Thumbnails ±20.

14.6 Document Detail

Shares layout with link detail. Preview behavior:

Category Preview
PDF Inline viewer via <embed> or <iframe> on a blob URL
Image-as-doc Rendered as an image
Video-as-doc Rendered as a video
Audio-as-doc Rendered as an audio player
Text Plain text render, monospace
Code Syntax-highlighted via highlight.js
Office / Archives / Unknown File card with "Open in app"

"Open in app": Capacitor Filesystem + Share (mobile), openPath() from tauri-plugin-shell (desktop), download link (web).

14.7 Link Detail

Elements: OG preview image, favicon + domain, title, description, "Open in browser," Copy/Share/Star chips, info section (shared by, in room, date), message context card with "View in chat."

Empty/error states: Solid color placeholder with domain initial; domain-only card; browser handles unreachable links; placeholder for missing OG image.

14.8 Curation Empty States

Surface Copy
Media "No media yet. Photos and videos shared in your chats will appear here."
Documents "No documents yet."
Links "No links yet."
Filtered "No [media/docs/links] match your filters."
Starred filter "No starred items yet. Star anything to find it here."

---

15. Calls and Sessions

15.1 Call Model

Room-scoped. A 1:1 call is a 2-member room. A group call is a room with 3+ members.

15.2 Pre-Call Preview

Default: voice, camera off, mic on. Background blur optional.

15.3 Incoming Call

· Full-screen: Foreground.
· Banner: Another app (iOS via CallKit, Android via full-screen intent).
· Actions: Decline, Accept, Message, More.

15.4 In-Call Screen

Voice 1:1: Profile-centric. Controls: speaker, mute, switch to video, minimize, end.

Voice group: Grid (up to 9 profiles) or list view with mic/video status. Active speaker highlighted with a green ring.

Video 1:1: Remote video full-screen; self-view as draggable PiP.

Video group: Grid (up to 6) or focus view with filmstrip.

15.5 Minimized Call

PiP floating window, draggable. On desktop, the call can dock into a resizable pane within Panel 3.

15.6 Call History

Calls tab in the rail (and bottom nav on mobile). Rows show profile, room name, call type, direction, duration, timestamp.

15.7 Sessions

Sessions are persistent, named, extension-owned, room-scoped real-time spaces. The mechanism is defined in Server Spec Amendment 35. This section describes the client UX for the reference type — the voice session — and the general contract that all session types follow.

Properties:

· Persistent — exists until deleted
· Named — encrypted metadata holds name, icon, description
· Multiple per room (default limit 10)
· Occupancy is ephemeral — server knows in memory only
· Room is the permission boundary

Entry point: Headphones icon in the room header (see §6.7).

Header icon states:

State Treatment
No sessions Icon present, no badge
Sessions exist, none active Subtle dot
Active sessions Count badge (total participants)

Session list sheet (opened from header icon): all sessions in the room, each with name, icon, participant count, join action. "Create session" action at the top.

Active strip: Appears above the message thread when at least one session has participants.

```
┌───────────────────────────────────────────┐
│ 🌴 The Lounge · 3 people         [ Join ]│
└───────────────────────────────────────────┘
```

· Single line, ~48px height
· Only present when participants > 0
· If multiple sessions are active, stack or scroll horizontally

In-session bar: Replaces the active strip when the user has joined.

```
┌───────────────────────────────────────────┐
│ 🌴 The Lounge  🎤 On  🔊 On  [Expand] [✕]│
└───────────────────────────────────────────┘
```

Full session view:

· Room name at the top
· Session name as the title
· Participant grid (roster)
· Controls: mic, speaker, camera, screen share, leave
· On desktop: opens as a centered overlay over the content area (see §26.5)
· On mobile: full-screen sheet

Create/Edit sheet:

```
┌─────────────────────────────────────────┐
│  New session                       [✕]  │
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

Metadata encryption: session_metadata_key = HKDF(RMK, "session-metadata-v1", 32).

Icon attachment: Encrypted with C2SP context "attachment" || 0x00 || "session-icon" || 0x00 || room_id.

15.8 Session Client Behavior

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

· Server publishes session.deleted for every session in the room.
· Client MUST tear down media on receiving session.deleted.

Reconnect:

· On WebSocket disconnect and reconnect, client does NOT auto-rejoin.
· Shows session as "disconnected" with a rejoin action.
· Fetches GET /rooms/:id/sessions on reconnect to refresh counts.

Multi-device:

· A user may have multiple client_ids in a session.
· Client uses the same stable client_id as for MLS.
· Leaving from one device does not remove the user if another device is still present.
· If the user is in a session on device A and opens the room on device B, device B shows "You're in this on another device." Device B can call POST .../leave { client_id: <device_A_id> } to leave on behalf of device A.

Signal payload: session.signal includes target_client_id so recipients filter to the intended device.

15.9 Sessions — Prohibitions

These are normative and protect the design:

1. Do not cache the roster beyond the session. Discard on leave. Do not persist to SQLite.
2. Do not display identities to non-participants. Counts only.
3. Do not log occupancy. No console logs, no analytics, no crash reports containing participant lists.
4. Do not auto-rejoin. Rejoin is always a user action.
5. Do not send notifications on session join.
6. Do not put sessions in call history.
7. Do not synthesize presence. No "last seen in lounge," no "recently active in session."

15.10 Session Rate Limits

Client must respect the server's rate limits:

· Create: 20/hour, 100/day
· Join: 30/minute
· Heartbeat: 10/minute per session (normal usage 4/minute)

If heartbeat limit is hit, log locally and do not retry aggressively.

15.11 Sessions Feature Disabled

If capabilities.sessions_enabled = false:

· Session metadata remains visible (read/patch/delete work)
· Join/signal/heartbeat return 501
· Client shows: "Sessions are disabled on this server." when join is attempted
· Existing sessions can still be deleted by their creator/owner

15.12 Other Session Types

Session types are declared by their owning extension (see §26.8). The voice type is the only one shipped in v1. Watch-together, collaborative games, and other real-time features are future extensions that use the same mechanism. The client's session infrastructure — join/leave/heartbeat/signal, the strip/bar/roster components, the list-detail navigation — is generic and supports any session type.

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
6. Server returns: session_token, user_id, recovery codes (plaintext, shown once).
7. Display recovery codes. Download is mandatory — Next disabled until downloaded.
8. Confirmation: type the 3rd code back.
9. Client caches the OPRF token.
10. window.location.href = '/app.html'.

16.4 Recovery View

Separate flow, not a login-mode variant.

Recovery flow (client):

1. Blind username → OPRF → token.
2. POST /auth/recover/start { recovery_code, username_token }.
3. Server verifies code (Argon2id, constant-time).
4. Server returns OPAQUE registration challenge.
5. Client runs OPAQUE registration with new password.
6. POST /auth/recover/finish { username_token, recovery_session, opaque_record, encrypted_display, identity_pubkey }.
7. Server replaces OPAQUE record, consumes code, revokes all existing sessions, issues new session.
8. Client stores new session, redirects to /app.html.

Other devices: receive session.revoked → clear session → redirect to index.html.

Recovery code display screen (initial registration): Codes shown once, must be downloaded. Confirmation step requires typing one back. Explicit warning: "These codes cannot be recovered if lost." Regeneration only available from an authenticated session.

16.5 Biometric Unlock

· Root of trust remains OPAQUE + password.
· Biometric gates access to the locally stored OPAQUE RegistrationRecord.
· Enrollment offered once after a successful password login.
· Unenrollment in Settings → Account → Security.
· Auto-invalidated on password change.

Screen lock timeout: Settings → Privacy → Screen lock → Immediately / 1 min / 5 min / 30 min. Default: 1 minute.

Biometric login flow: Biometric unlocks the OPAQUE record → username retrieved from local storage → OPRF blinding → token → login proceeds. User types nothing.

16.6 Device Linking

1. Existing device generates a device linking code (QR or short code).
2. New device enters or scans the code.
3. New device performs OPAQUE login with password.
4. New device uploads its MLS key package.
5. New device fetches user-scoped sync state: GET /users/me/sync?since_seq=0.
6. Device names and other user-scoped state populate.

Device names: Server stores encrypted_device_name as opaque ciphertext in user-scoped sync state. Client encrypts with device_name_key = HKDF(token, "device-name-encryption-v1", 32). Cross-device: device.name_updated event on private-user-{user_id}.

16.7 Passkeys (V2 Seam)

Not in V1. Data-driven action list in the login view.

16.8 Session Expiry

Auth token expired → redirect to index.html. OPAQUE session expired → inline error.

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
Profile Navigate → display name, profile image, username (read-only)
Password Navigate → change password
Recovery codes Navigate → status, regenerate
Devices Navigate → list, revoke
Biometric unlock Toggle
Passkeys Navigate → V2 placeholder
Starred items Navigate → list

Display name update: Re-encrypt with cached display_name_key and fresh nonce. PATCH /users/me { encrypted_display }.

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

17.8 Calls

Default call mode, Call ringtone, Video quality, Show calls tab.

17.9 Appearance

Theme, Accent color, Wallpaper, Reduced motion, Sticker animations, Language.

17.10 Extensions

Row Purpose
Installed extensions List of all registered extensions with enable/disable toggle
Network permissions Per-extension list of declared origins and user-entered origins
Session types List of session types declared by installed extensions

Reserved preference keys (room_order, _system:*) are not editable here.

17.11 Starred Items

Settings → Account → Starred items.

List of all starred items (media, messages, links), filterable by type. Unstar via swipe or long-press. Empty state: "No starred items yet. Star anything to find it here."

17.12 Administration

Rendered only if capabilities.admin is present.

17.13 Sign-Out and Account Deletion

· Log out: Session cleared, local data preserved (encrypted at rest).
· Log out and clear data: Everything cleared. Extension storage and preferences wiped.
· Account deletion: Warning screen → type username to confirm → server anonymizes → local data wiped → Auth Gate.

Account deletion server behavior (Amendment 33): username_token replaced with a random 86-character base64url string. encrypted_display and profile set to NULL.

Recovery triggers session revocation on all other devices. Affected devices handle it via session.revoked.

---

18. Notifications

18.1 In-App Banner

Top of detail panel (desktop) / top of screen below status bar (mobile). One at a time. Auto-dismiss after 5s. Mute exception for mentions. No inline reply.

Session events do NOT trigger notifications.

18.2 Push Notifications

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

Notification categories: MESSAGE, MESSAGE_MUTED, MENTION, CALL, MEMBER_CHANGE.

No push on session join.

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

URL format: https://app.example.com/app.html?rail=<id>&detail=<route>&id=<sel>

Pending slot: In-memory only. Invite tokens use sessionStorage for auth-flow survival.

Processing order: Boot → auth → sync start → route → clear.

OAuth callback pattern: Extensions that integrate with OAuth services (Mastodon, Bluesky, etc.) use the deep-link mechanism to receive callbacks. The callback arrives as ?rail=<ext>&detail=oauth-callback&code=...&state=.... The extension's onActivate handler detects the route, exchanges the code, and calls ctx.navigate() to land on its list.

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
Session list empty "No sessions yet. Create one to start a real-time space."
Session list (filtered) no-results "No sessions match your filters."
Starred items empty "No starred items yet. Star anything to find it here."
Read-aloud voices empty "No voices downloaded. Download one to start reading aloud."
Calls empty "No calls yet."
Blocked users empty "No blocked users"
Downloaded modes empty "No modes downloaded"
Custom stickers empty "No custom stickers yet"
In-thread search empty "Nothing to search yet"

Rules: Shared-rooms section hidden when empty. One primary action per state.

Extensions may declare empty, loading, and error on their surfaces with either a declarative shape (icon, title, body, action) or a custom component. See §26.5.

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
OPRF failure Retry; app works without display-name decryption

21.5 Media Errors

Transcode failure → passthrough. Upload failure → retry. Download failure (corrupt) → error. Presigned URL expired → fetch new URL.

21.6 Storage Errors

Quota warnings at 80%, blocks at 95%. Write failure → in-memory mode with banner. OPFS unavailable → fallback.

21.7 MLS Errors

Out-of-sync epoch → "Syncing encryption state…". Missing welcome → poll every 5s; after 5min, "Waiting for a room member to add you". Pending MLS remove → room marked "left". Keystore corruption → wipe, mark rooms error, offer rejoin.

21.8 Session Errors

Error Behavior
403 not_a_participant (roster fetch) Do not fetch roster before joining
Join failure (501 feature disabled) "Sessions are disabled on this server."
Join failure (503 at capacity) "This session is full."
Heartbeat timeout (3 consecutive) Show "Disconnected" state. Offer rejoin. Do not auto-rejoin.
session.deleted received Tear down media. Toast: "This session was deleted."
Signal delivery to wrong client Filter locally via target_client_id
Signaling failure (WebRTC) "Couldn't establish a connection. Try again."
Rate limited on heartbeat Log locally. Do not retry aggressively.

21.9 Read-Aloud Errors

Error Behavior
TTS model download failure Toast: "Couldn't download voice. Tap to retry."
Synthesis failure Toast: "Couldn't read this message."
Bad transcript detected Suggestion UI (see §11.9)

21.10 Extension Errors

Error Behavior
Extension component render failure Error boundary renders host-owned fallback. Rail icon remains clickable.
Extension fetch (proxy) 502 upstream_response_too_large Extension retries with transport: 'direct'; one-time toast
Extension fetch (proxy) disabled transport: 'proxy' fails; transport: 'auto' falls back to direct
Extension network origin denied by user Extension handles error; no automatic retry

21.11 Interaction Edge Cases

Scenario Behavior
Edit, then unsend Tombstone replaces latest version
Unsend, then attempt to edit Edit option absent
Disappearing message edited Edit inherits expires_at
Reply to expiring message [message expired] after expiration
Message during MLS transition Composer "Securing…"; send queued
Same file in two rooms Stored separately (encryption binds to room_id)
Member list without cached display name user_id placeholder until first message
Star a message, then it's deleted Star persists locally as standalone copy
Nickname set, then member leaves Nickname retained in local store
Read-aloud interrupted mid-message Position saved; resumes on next tap
Two sessions active in the same room Both show in strip; both in list; both counted in header badge

---

22. Plugin Inventory

Plugin Options Purpose
extension-plugin extensions Atoll extension system. Registers, validates, mounts extensions. From @atoll/extend.
state-plugin initialState Global keyed pub/sub
router-plugin — History API + query-param routing
socket-plugin socketUrl, apiBase, appKey WebSocket connection
crypto-plugin wasmPath, thresholds MLS wrapper
oprf-plugin apiBase OPRF blinding + token derivation
media-plugin presets, workerPoolSize Transcode, thumbnail, strip
transcription-plugin STT models config Local speech-to-text
speech-plugin TTS models config Local text-to-speech
push-plugin vapidKey, apnsConfig, fcmConfig Push registration
storage-plugin dbName, blobRoot SQLite + blob filesystem
i18n-plugin locales, defaultLocale Translation
theme-plugin presets, accent Theme application
call-plugin iceServers, transport Call state machine
session-plugin iceServers, heartbeatInterval Session state + signaling
p2p-plugin iceServers, chunkSize WebRTC data channels
icon-plugin styleMap, registry Solar icon resolution
notification-plugin defaultSound In-app banners
search-plugin threshold, debounceMs Fuse.js lifecycle

No event bus. Cross-component communication uses emit and $state key subscriptions. Cross-extension communication uses publicEvents (§26.7).

---

23. Appendices

23.1 SQLite Schema

Domain tables:

Domain Tables
Meta _migrations, _meta (includes last_user_seq, oprf_token_cache_key)
Identity users (cached display names keyed by user_id)
Rooms rooms, room_members, room_order
Messages messages, message_versions, reactions
Attachments attachments
User state read_state, drafts, outbox, blocked_users, room_preferences, device_names, starred_items
Personalization wallpapers, sticker_packs, custom_stickers, blocked_sticker_packs, nicknames
Sessions sessions (metadata cache only; occupancy is not persisted)
Extension state extension_storage (namespaced by extension_id), extension_preferences (local mirror)
Speech speech_voices
Transcription transcription_modes
Calls calls, call_participants
Sync sync_state (per-room epoch/seq), processed_events
MLS mls_rooms
Settings settings

Key tables:

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

CREATE TABLE sessions (
    session_id    TEXT PRIMARY KEY,
    room_id       TEXT NOT NULL,
    extension_id  TEXT NOT NULL,
    session_type  TEXT NOT NULL,
    metadata_blob BLOB,
    position      INTEGER NOT NULL,
    updated_at    INTEGER NOT NULL
);

CREATE INDEX idx_sessions_room ON sessions(room_id, position);
CREATE INDEX idx_sessions_extension ON sessions(room_id, extension_id, session_type);

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

CREATE TABLE extension_storage (
    extension_id  TEXT NOT NULL,
    key           TEXT NOT NULL,
    value_blob    BLOB NOT NULL,
    updated_at    INTEGER NOT NULL,
    PRIMARY KEY (extension_id, key)
);

CREATE INDEX idx_extension_storage_ext ON extension_storage(extension_id);

CREATE TABLE extension_preferences (
    extension_id  TEXT NOT NULL,
    key           TEXT NOT NULL,
    value_json    TEXT NOT NULL,
    user_seq      INTEGER NOT NULL,
    updated_at    INTEGER NOT NULL,
    PRIMARY KEY (extension_id, key)
);
```

Meta keys:

· _meta.last_user_seq — user-scoped sync cursor
· _meta.read_aloud_settings — JSON { mode, voice, speed, auto_read }
· _meta.read_aloud_room_overrides — JSON { [roomId]: 'on' | 'off' }
· _meta.extension_origins — JSON { [extensionId]: { [origin]: 'allowed' | 'denied' } }

Principles: Flat columns for queryable fields; JSON blobs for opaque metadata. IDs TEXT. Timestamps INTEGER ms. Encrypted payloads BLOB. WAL mode. Forward-only migrations.

23.2 Design Tokens

Full token list in src/styles/tokens.css. Structure:

· Primitives (neutral, accent, accent-warm, semantic)
· Spacing (4px base)
· Typography
· Radii, shadows, motion
· Z-index, layout metrics, safe areas

Semantic tokens remap under [data-theme="dark"]:

· --surface-0 through --surface-3
· --text-primary, --text-muted
· --bubble-incoming, --bubble-outgoing
· --accent-* (brightened in dark mode)
· --border-*, --divider

Extensions reference only semantic tokens. Ramp tokens are internal to the token file.

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
session.created { room_id, session_id, extension_id, session_type, created_by, created_at }
session.updated { room_id, session_id, metadata?, position? }
session.deleted { room_id, session_id, extension_id }
session.occupancy { room_id, session_id, participant_count } (count only)

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
starred_item.removed { item_id, item_type, user_seq } Yes
preference.updated { key, user_seq } Yes
session.signal { room_id, session_id, sender_user_id, sender_client_id, target_client_id?, signal_type, payload } No

Client events (Sockudo-published, ephemeral):

Event Payload Purpose
client-typing.start { user_id } Typing indicator
client-typing.stop { user_id } Typing indicator
client-mls-request { room_id } Prompt pending MLS add

Read state writes go through REST (POST /users/me/read-state), not client events.

23.4 Server Assumptions

The client implements against Server Specification v2.0 and its amendments (25–38). Blocking items: OPRF, encrypted display names, user-scoped sync, pending_mls_adds, recovery flow, key transparency, call signaling, message editing/reactions/threading, sessions, model hosting (STT+TTS), starred items, generic preferences, extension proxy.

Breaking-change coordination: OPRF + user-scoped sync + device model changes ship as one release. No partial rollout.

---

24. Design Decisions Log

Significant decisions made during drafting.

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
11 2026-09-30 Header consolidation: video icon removed, headphones icon added for sessions.
12 2026-09-30 Read-aloud (TTS) with Supertonic. Composer toggle, sentence-level control, auto-resume.
13 2026-09-30 Inline audio player with transcribe button (two zones). User-facing mode labels.
14 2026-09-30 Bad transcript detection with suggestion UI.
15 2026-09-30 Native keyboard sticker limitation documented. Paste-as-sticker fallback.
16 2026-09-30 Starred items synced across devices (server Amendment 27).
17 2026-09-30 Nicknames synced via user preferences (server Amendment 28).
18 2026-09-30 Read-aloud settings synced via user preferences (server Amendment 28).
19 2026-09-30 TTS model hosting added to capabilities (server Amendment 26).
20 2026-10-02 Accent palette locked: Lagoon Blue #2FB6AA, Dark Aquamarine #297370, Coral Reef #EC7562, Papaya Whip #F9F1E4, Beige Skin #E0C0A9. OKLCH luminance inversion for dark mode. 4-step tonal elevation surfaces. Semantic tokens remap under [data-theme="dark"].
21 2026-10-02 Extension system adopted. Rail/list/detail surfaces. Panel and overlay presentation. Chrome is fixed, content is free. Core extensions use the same API as third-party. defineExtension in @atoll/extend, not Coralite. Monorepo. Components are ambient.
22 2026-10-02 URL scheme changed from ?view= to ?rail=&detail=&id=. Route ownership resolves through the extension registry. Browser history is the back stack.
23 2026-10-02 Sessions replace hangouts (server Amendment 35). Extension-owned session types. Server is signaling relay with in-memory occupancy.
24 2026-10-02 Extension proxy adopted (server Amendment 38). Single ctx.fetch with transport: 'proxy' \| 'direct' \| 'auto'. Default proxy. Auth headers force direct.
25 2026-10-02 Extension state namespacing. ctx.state writable and namespaced. $state read-only. Extension storage encrypted at rest with device-scoped key.
26 2026-10-02 Public events. Extensions declare publicEvents with schemas. Emits validated against declared schemas at build time.
27 2026-10-02 Slots system. Declared on host, mounted on filler. Surface-scoped names. Build-time validated.
28 2026-10-02 Localization via plugin context. t available in getters and client(), never in templates. Locale files declared in manifest, bridged into Coralite assets by extension-plugin.
29 2026-10-02 Extension assets declared in manifest, namespaced under assets/extensions/<id>/, resolved via ctx.asset().
30 2026-10-02 Build-time validation via plugin hooks. Not a parallel subsystem. Errors surface through Coralite's existing channel.

---

25. Document Status

This is the contract for the client side of the system. Every client implementation task references this document. If a task conflicts with this spec, the task is wrong and must be revised.

Design decisions are tracked in §24.

---

26. Extension System

26.1 Overview and Boundary

Extensions are an Atoll Chat concept, not a Coralite concept. defineExtension is exported from @atoll/extend and produces a Coralite plugin internally. Extension authors work entirely in Atoll's vocabulary — rail, list, detail, surfaces, slots, ctx — and never interact with Coralite's plugin API directly.

Coralite does not depend on Atoll. Atoll depends on Coralite. Every extension is a Coralite plugin from Coralite's point of view; it is an extension from Atoll's point of view.

Extensions are client-only. They extend the UI. Server capabilities are added to the server spec, and any client — including extensions — can use them. An extension cannot add server behavior. Extensions that need to fetch data do so through ctx.fetch or through server endpoints exposed to all clients.

Core extensions use the identical API. There is no privileged path. Core extensions are declared with the same defineExtension call, loaded through the same plugin array, validated by the same rules, and mounted by the same shell code. The distinction between core and third-party is a naming convention (core.* vs. com.example.*) and nothing more.

26.2 defineExtension

defineExtension(ext) validates the extension object, applies defaults, normalizes it into the internal shape, registers components, and returns a Coralite plugin.

```js
// packages/app/src/extensions/chat/index.js
import { defineExtension } from '@atoll/extend'

export default defineExtension({
  id: 'core.chat',
  // ...the declarative shape
})
```

The returned plugin is added to the app's plugin array alongside everything else. From Coralite's perspective, it is a normal plugin.

From the author's perspective, defineExtension is the only API surface they touch. definePlugin, defineComponent, and Coralite's lifecycle hooks are all implementation details of Atoll.

26.3 The Extension Object

Complete schema. Fields marked required must be present.

```js
{
  id: string,                    // required — 'core.chat', 'com.example.rss'
  apiVersion: string,            // required — e.g. '1.0.0'
  hostApi: string,               // required — semver range, e.g. '^1.0'
  label: string,                 // required for rail-bearing extensions
  permissions: Permission[],     // optional — declared capabilities

  rail: RailSlot | null,         // optional — panel 1 slot

  list: ListSurface | null,      // required for list-detail extensions
  detail: DetailSurface,         // required — every extension has one

  slots: SlotMount[],            // optional — slots this extension fills
  emits: EventDeclaration[],     // optional — events this extension emits
  publicEvents: EventDeclaration[], // optional — events this extension accepts
  listens: EventListener[],      // optional — declared listeners

  sessions: SessionTypeDecl[],   // optional — session types this extension owns
  preferences: PreferenceDecl[], // optional — cross-device preference keys

  locales: LocaleDecl | null,    // optional — translations
  assets: AssetDecl[],           // optional — static files

  onRegister: (ctx) => void,     // optional — one-time setup
  onActivate: (ctx) => void,     // optional — becomes active rail
  onDeactivate: (ctx) => void    // optional — no longer active rail
}
```

Normative statement: An extension declares only its public surface. Private state, internal components, and implementation details are not part of the extension object and are not visible to other extensions.

RailSlot:

```js
{
  icon: { name: string, state?: string },
  order: number,
  badge: { deps: string[], compute: (ctx) => number | 'dot' | null } | null,
  visible: (ctx) => boolean,
  mobile: { placement: 'bottom' | 'more' | 'hidden', order?: number }
}
```

ListSurface:

```js
{
  route: string,
  title: string | (ctx) => string,
  component: string,
  actions: ActionDescriptor[],
  slots: Record<string, SlotDeclaration>,
  empty: EmptyDecl | null,
  loading: LoadingDecl | null,
  error: ErrorDecl | null
}
```

DetailSurface:

```js
{
  route: string,
  title: string | (ctx) => string,
  component: string,
  actions: ActionDescriptor[],
  slots: Record<string, SlotDeclaration>,
  surfaces: ('panel' | 'overlay')[],
  defaultSurface: 'panel' | 'overlay',
  back: 'auto' | true | false,
  scope: Record<string, string>,
  settings: SettingsDecl | null,
  empty: EmptyDecl | null,
  loading: LoadingDecl | null,
  error: ErrorDecl | null
}
```

ActionDescriptor:

```js
{
  id: string,
  icon: string,                          // Solar icon name
  label: string,
  onSelect: (ctx) => void,
  visible: (ctx) => boolean,
  disabled: (ctx) => boolean,
  active: (ctx) => boolean,
  badge: (ctx) => number | 'dot' | null,
  variant: 'default' | 'danger',
  placement: { desktop?: 'inline' | 'overflow', mobile?: 'inline' | 'overflow' }
}
```

SlotDeclaration (on host):

```js
{ multiple: boolean, order: 'ascending' | 'descending' | 'fixed' }
```

SlotMount (on filler):

```js
{
  slot: string,                 // fully-qualified slot name
  component: string,
  order: number,
  visible: (ctx) => boolean
}
```

EventDeclaration (both emits and publicEvents):

```js
{
  event: string,                // e.g. 'media:open'
  schema: Record<string, string>  // flat key → type
}
```

Types: 'string', 'number', 'boolean', 'string?' (optional), 'a|b|c' (enum). No nested objects.

EventListener:

```js
{
  event: string,
  handler: (detail, ctx) => void
}
```

SessionTypeDecl:

```js
{
  type: string,
  maxParticipants: number,
  maxPerRoom: number,
  heartbeatInterval: number,
  metadata: Record<string, string>,     // schema, client-side only
  signaling: Record<string, Record<string, string>>  // schema, client-side only
}
```

PreferenceDecl:

```js
{ key: string, type: 'string' | 'number' | 'boolean' | 'object' | 'array', label: string }
```

LocaleDecl:

```js
{
  default: string,
  files: Record<string, string>  // locale → path
}
```

AssetDecl:

```js
{ src: string, dest: string }
```

EmptyDecl, LoadingDecl, ErrorDecl:

```js
// Empty: either declarative or custom component
{ icon?: string, title: string, body?: string, action?: ActionDescriptor }
// or
{ component: string }

// Loading: preset variant or custom component
{ variant: 'list' | 'grid' | 'thread' | 'detail' }
// or
{ component: string }

// Error: retry flag or custom component
{ retry: boolean }
// or
{ component: string }
```

SettingsDecl:

```js
{
  title: string,
  component: string,
  sections?: SettingsSectionDecl[]
}
```

26.4 Context (ctx)

ctx is the extension's view of the shell. It is passed to every callback the extension declares — title, onSelect, visible, compute, listens[].handler, and lifecycle hooks.

Shape:

```ts
interface ExtensionCtx {
  // identity
  id: string

  // global, read-only
  $state: Readonly<GlobalState>
  capabilities: Capabilities
  platform: 'mobile' | 'tablet' | 'desktop'

  // own state, namespaced, writable
  state: Record<string, unknown>
  storage: {
    get<T>(key: string): Promise<T | undefined>
    set<T>(key: string, value: T): Promise<void>
    delete(key: string): Promise<void>
    clear(): Promise<void>
  }
  preferences: {
    get<T>(key: string): Promise<T | undefined>
    set<T>(key: string, value: T): Promise<void>
    delete(key: string): Promise<void>
    subscribe<T>(key: string, cb: (value: T) => void): () => void
  }

  // surface context — null when the invocation isn't surface-bound
  surface: 'list' | 'detail' | null
  scope: Record<string, unknown> | null
  selection: { id: string; type: string } | null
  position: { index: number; total: number } | null

  // navigation — only this extension's routes and surfaces
  navigate(route: string, params?: object): void
  back(): void
  present(opts: PresentOptions): void
  dismiss(): void

  // shell utilities
  toast(message: string, opts?: ToastOptions): void
  notify(message: string, opts?: NotifyOptions): void
  openExternal(url: string): void
  asset(path: string): string
  hasPermission(permission: string): boolean

  // network
  fetch(url: string, opts?: FetchOptions): Promise<Response>
  fetchUserUrl(url: string, opts?: FetchOptions): Promise<Response>

  // localization
  t(key: string, vars?: Record<string, string | number>): string

  // lifecycle
  signal: AbortSignal
}
```

What's populated at each invocation point:

Invocation surface scope selection position
badge.compute null null null null
slots[].visible host's host's host's host's
title (list) 'list' extension's null null
title (detail) 'detail' extension's set set
actions[].onSelect surface extension's set set
empty.action.onSelect surface extension's null null
listens[].handler null null null null
onRegister null null null null
onActivate null activation scope null null
onDeactivate null activation scope null null

Normative statements:

· ctx.$state is read-only. ctx.state is the extension's writable slice. Extensions cannot write to another extension's state, and cannot write to the shell's state.
· ctx is reconstructed per invocation. It is not a stable identity and should not be stored.
· ctx.navigate, ctx.present, and ctx.dismiss are extension-scoped. ctx.navigate('chat') from core.media fails — chat isn't media's route. ctx.dismiss() only closes the extension's own overlay.

PresentOptions:

```ts
{
  scope?: Record<string, unknown>,
  selection?: string,
  surface?: 'overlay'
}
```

FetchOptions:

```ts
{
  method?: 'GET' | 'POST' | 'HEAD',
  headers?: Record<string, string>,
  body?: string | Uint8Array,
  transport?: 'proxy' | 'direct' | 'auto',
  signal?: AbortSignal
}
```

26.5 Surfaces and Routes

Rail, list, detail. Three surfaces, mapped to the three panels of the desktop layout.

· Rail — the icon strip in Panel 1. Optional. Every rail-bearing extension appears in the strip, sorted by rail.order.
· List — the list surface in Panel 2. Required for list-detail extensions.
· Detail — the detail surface in Panel 3. Required for every extension.

Presentation. A detail surface is presented in one of two ways:

· panel — inline in Panel 3. Changes the rail context when the detail belongs to a different extension than the current rail.
· overlay — modal above the content area. Does not change the rail. Rail highlight remains where the user was.

An extension declares which surfaces it admits in detail.surfaces. The router chooses defaultSurface when resolving from a URL or a rail click. A caller can request a different admitted surface via ctx.present({ surface }).

Route ownership. Every route is owned by exactly one extension. extensions.ownerOfRoute(route) resolves it. Duplicate route names are a build error.

Normative statements:

· Every surface has a required title. The shell draws the header, actions, and overlay chrome. Extensions supply content.
· An overlay does not change the rail. A panel does.
· Overlay stacking: one per extension, LIFO. Escape, X, click-outside, ctx.dismiss(), and the back button all route through history.back().
· Back button behavior: pops the history stack. If the stack is empty, back goes to the owning extension's list, or the rail default if the extension has no list.

Chrome vs. content. The following are chrome — drawn by the shell, identical everywhere, not touched by extensions:

· Header (back arrow or X, title, actions row)
· Overlay frame (drawer / sheet / full-screen modal)
· Settings frame
· Rail icons and badges
· Mobile bottom nav and overflow sheet

The following are content — drawn by extensions inside the content area:

· List body
· Detail body
· Search fields and filters
· Empty, loading, and error states (unless declared declaratively)

26.6 Slots

A slot is a named insertion point in a surface. The host extension declares its slots; other extensions mount components into them.

Declaration (host side):

```js
detail: {
  route: 'chat',
  title: (ctx) => ctx.room?.name,
  component: 'view-chat',
  slots: {
    'aboveThread':         { multiple: true, order: 'ascending' },
    'composer.above':      { multiple: true },
    'composer.attachMenu': { multiple: true },
    'composer.leading':    { multiple: false },
    'composer.trailing':   { multiple: false }
  }
}
```

Mount (filler side):

```js
slots: [
  {
    slot: 'chat.composer.attachMenu',
    component: 'x-bookmarks-attach-action',
    order: 50,
    visible: (ctx) => ctx.$state.selectedRoomId !== null
  }
]
```

Normative statements:

· Slot names are surface-scoped strings: <extensionShortId>.<slotName>. They are not globally registered.
· Slot names are string literals. Build-time validation confirms every mounted slot resolves to a declared slot on a registered extension.
· If a slot declares multiple: false and more than one extension mounts it, the build fails.
· A slot host's visible receives the host's surface context, not the filler's.

26.7 Events

Three event mechanisms:

1. emits — events this extension broadcasts. Declared with schemas.
2. publicEvents — events this extension accepts from other extensions. Declared with schemas.
3. listens — handlers for public events. Declared with handlers.

Declaration:

```js
// core.chat declares public events it accepts
publicEvents: [
  { event: 'chat:compose', schema: {
      roomId: 'string',
      text: 'string',
      previewTitle: 'string?',
      previewDescription: 'string?'
    }
  }
],

// RSS emits chat:compose
emits: [
  { event: 'chat:compose', schema: {
      roomId: 'string',
      text: 'string',
      previewTitle: 'string?',
      previewDescription: 'string?'
    }
  }
]
```

Normative statements:

· Event schemas are flat objects. Types are string, number, boolean, string? (optional), or a |-separated enum. No nested objects. If an event needs nested data, it should be two events.
· Every emit call is validated against the declared schema at build time. Every detail.xyz access in a listener is validated against the schema of the event being listened to.
· Emitter and receiver schemas must match. Two extensions cannot declare different schemas for the same event name.
· An event declared in emits must either be the extension's own event (broadcast to listeners) or match a publicEvents declaration on another extension. Build-time validation checks this.
· Event ordering: registration order. Both handlers fire. No stopPropagation. No priority.

Common public events:

core.chat:

Event Schema
chat:compose { roomId, text, previewTitle?, previewDescription? }
chat:insert-text { roomId, text, position: 'start'\|'end'\|'cursor' }
chat:focus { roomId }
chat:set-draft { roomId, text }
chat:message-sent (emitted) { roomId, messageId, text }

core.media:

Event Schema
media:open { roomId, attachmentId, surface: 'panel'\|'overlay' }

26.8 Sessions

An extension declares the session types it owns:

```js
sessions: [
  {
    type: 'voice',
    maxParticipants: 12,
    maxPerRoom: 3,
    heartbeatInterval: 15,
    metadata: { name: 'string', icon_file_id: 'string?', description: 'string?' },
    signaling: {
      offer: { sdp: 'string' },
      answer: { sdp: 'string' },
      ice: { candidate: 'string' }
    }
  }
]
```

Normative statements:

· Session types are declared with the extension. Duplicate (extension_id, type) pairs across extensions are a build error.
· The server validates session type against its own SESSION_TYPES.toml configuration. The client's declaration is used for vocabulary, build-time validation, and emitting a recommended configuration to operators via atoll-chat build --emit-session-types.
· Metadata and signaling schemas are client-side only. The server never validates them. They exist so the extension's own code and its peers agree on the shape.

26.9 Preferences

Cross-device key/value store. Wraps the generic preferences endpoint (Server Spec Amendment 28).

Declaration:

```js
preferences: [
  { key: 'feedList', type: 'object', label: 'Subscribed feeds' },
  { key: 'readIds',  type: 'array',  label: 'Read articles' },
  { key: 'lastRefresh', type: 'number' }
]
```

Normative statements:

· Every preference key must be declared. Keys not declared are rejected at build time.
· The host prefixes every key with ext:{extensionId}: before sending to the server.
· Values are capped at 64 KB. Exceeding the cap is a runtime error with a clear message.
· Reserved keys (room_order) and prefixes (_system:) cannot be claimed.
· Values are not end-to-end encrypted. Extensions that store sensitive data should encrypt before setting.
· Sync is automatic. preferences.subscribe(key, cb) routes preference.updated events.

26.10 Storage

Encrypted local store. Device-scoped.

Normative statements:

· ctx.storage is encrypted at rest with a device-scoped key. Same treatment as the app's own sensitive state.
· Namespaced by extension id. Two extensions cannot read each other's storage.
· Cleared on "Log out and clear data," not on regular logout.
· Per-extension quota: 5 MB default, operator-configurable.
· ctx.state is memory. ctx.storage is disk. Extensions choose.

State lifecycle:

· ctx.state is in-memory. It persists for the session but not across reloads.
· On deactivate, state is preserved. Returning to the extension restores its state.
· Extensions that need across-reload persistence use ctx.storage.

26.11 Fetch

Single method with transport option.

Normative statements:

· The default transport is 'proxy'. It hides the user's IP and works around CORS.
· Extensions that send Authorization headers must use 'direct' and accept the IP exposure. Passing 'proxy' with an auth header is a build error.
· 'auto' uses proxy unless the request contains Authorization, in which case it uses direct.
· If extension_proxy_enabled is false, 'proxy' fails and 'auto' falls back to direct with a user-visible notice.
· Every user-entered origin requires explicit user approval, stored in _system:extension_origins:{extensionId} (host-managed, not extension-writable).
· Origins declared in the manifest are advisory. They feed the consent screen and code review, not runtime enforcement.

Response handling:

· content-length may be absent or unreliable. Use the decrypted body length.
· 502 upstream_response_too_large — extension retries with transport: 'direct' and shows a one-time "bypassed proxy" toast.
· 429 rate_limited — surface Retry-After and bucket ('extension' or 'user_total') to the user.

Streaming: Not supported through the proxy. Extensions that need streaming (SSE, WebSocket) use transport: 'direct'. The manifest declares streaming: true on the permission, and the build-time validator rejects transport: 'proxy' for those origins.

26.12 Lifecycle

onRegister(ctx) — runs once when the extension is loaded. Setup work. Hydrate from storage. Fetch preferences. Not called for every activation.

onActivate(ctx) — runs when the extension becomes the active rail context. Rail click, deep link, or ctx.navigate() to this extension's route.

onDeactivate(ctx) — runs when the extension is no longer the active rail context. Rail switch, deep link to another extension, or navigation away.

Normative statements:

· onActivate and onDeactivate do not fire on internal navigation (list → detail within the same extension).
· Overlays from other extensions do not fire these hooks. The active rail context is unchanged.
· All subscriptions and timers created in onActivate should be cleaned up on onDeactivate or via ctx.signal.
· ctx.signal aborts on onDeactivate.

26.13 Build-Time Validation

Validation runs inside defineExtension's plugin hooks. It is not a separate subsystem. Errors surface through Coralite's existing build error channel.

Phase 1 — Registration. Runs as each extension is loaded.

Checks: required fields, id format and uniqueness, rail shape, list/detail routes, surfaces values, settings shape, actions[] shape, icon names against Solar registry, component tags against registered components, slots declaration shape, slots[].slot mount shape, emits/publicEvents/listens format, mobile.panel1.placement values, apiVersion and hostApi semver, permissions allowlist, state key collisions with shell-reserved keys, preferences key patterns.

Phase 2 — Link. Runs after all extensions registered.

Checks: route uniqueness, reserved routes, slot resolution (every mounted slot resolves to a declared slot), slot multiplicity, listener/emitter match (every listens names an event declared in some emits or publicEvents), event schema match between emitter and receiver, circular slot mounts, rail order collisions (warning), scope key consistency (warning), duplicate action icons in overflow (warning).

Error format: Each error includes location (file and line), object path, context (what's actually available), and a suggestion.

Normative statements:

· Validation is mandatory. There is no --skip-validation flag. An extension that fails validation does not build.
· Runtime guards exist as a backstop for dynamic cases (dynamic routes, dynamic origins). Build-time validation is the primary mechanism.
· Accessibility is not checked by the extension validator. It is checked by Coralite's component validator (CORALITE-E105, accessible-name rules) and by Playwright + @axe-core/playwright in each extension's test suite.

26.14 Naming Conventions

Extension IDs:

· core.<name> — first-party extensions. Reserved.
· com.example.<name> — third-party extensions. Reverse-DNS.
· IDs are unique. Duplicates are a build error.

Component tags:

· First-party extensions use existing conventions: ui-*, chat-*, message-*, view-*, session-*.
· Third-party extensions must prefix: x-<slug>-<component>. Slug is derived from the extension id.
· The validator rejects component tags that don't match the convention.

Routes:

· Reserved routes: index, 404. Extension routes must not collide.
· Extension routes are short, lowercase, hyphen-separated.

Slots:

· Surface-scoped: <hostExtensionShortId>.<slotName>. The short ID is the last segment of the extension id (core.chat → chat).
· Slot names are camelCase.

Events:

· Namespaced: <source>:<event> (e.g., media:open, chat:compose).
· The namespace is the source extension's short ID.

Preference keys:

· camelCase, lowercase first letter.
· Reserved prefixes: _system:. Reserved keys: room_order.

26.15 Vocabulary Command

```
atoll-chat extensions vocab [--json] [--section <name>]
```

Runs offline. Reads the extension registry from the app's build output. Emits the complete author-facing vocabulary: components, slots, public events, routes, session types, reserved keys, icons, platforms, surfaces.

--json emits machine-readable output. Without it, human-readable summary. --section filters to one section.

Sections: components, slots, events, routes, sessions, preferences, permissions, icons, platforms, surfaces, reserved.

Normative statements:

· The vocabulary command exposes only the author-facing surface. Private state, internal components, and implementation details are not included. Exposing private surface is a breaking change to the extension API.
· The vocabulary is the contract. If a name appears in the vocabulary, extensions can depend on it. If it doesn't, they cannot.
· The human-readable form is regenerated as an appendix to this specification after every non-patch release.

---

Appendix A — Registered Components

Generated from the registry. Regenerated via atoll-chat extensions vocab. Lists every component tag available to extension templates, with summary, attributes, and slots.

Appendix B — Reserved Keys, Routes, and Slots

Category Reserved
Preference keys room_order
Preference prefixes _system:
Routes index, 404
Slots (none reserved — all slot names are host-declared)

Appendix C — Core Extensions

Extension ID Rail Routes Session types Notes
core.chat ✅ chats, chat, room-settings — Owns the message thread, composer, and room settings overlay
core.media ✅ media, media-viewer — Owns the media grid and viewer
core.documents ✅ documents, document — Owns document list and detail
core.links ✅ links, link — Owns link list and detail
core.calls ✅ calls, call — Owns call history and in-call UI
core.settings ✅ settings, settings-section — Owns all settings sections
core.hangouts — sessions, session voice Owns voice sessions
core.profile — profile — User profile view
core.join — join — Invite confirmation
core.admin — admin — Gated on capabilities.admin

Core extensions use the same API as third-party extensions. The only differences are the ID prefix (core.*) and the fact that they ship with the app.

---

End of Client Specification v1.0.
