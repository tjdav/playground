
@atoll/bot Package Specification v1.0.1

Status: Final - source of truth for @atoll/bot implementation tasks.
Supersedes: @atoll/bot Package Specification v1.0, @atoll/bot Draft v0.2,
all interim drafting messages.
Basis: Server Specification v3.0.2 · Client Specification v2.0 (Final) ·
Reconciliation (2026-10-04).
Stack: Node.js ≥ 22.22.2 (ESM) · JavaScript + JSDoc · node:test · pnpm
workspace.
Location: packages/bot/ in the monorepo.

Amendments in v1.0.1:
- §2.1: module mode changed from nodenext to es2022, moduleResolution from
  nodenext to bundler. Restores global visibility of JSDoc typedefs.
- §2.3: JSDoc description standard added. Hyphen rule enabled. definedTypes
  list documented.
- §3: every @property and @template carries a description.
- §4: every @param, @returns, and @template carries a description.
- §4.3: prose hyphen separator replaces em-dash in the spec's own text.

---

1. Overview

@atoll/bot is the SDK bot authors import. It parallels @atoll/extend. The
author writes handlers and declarations; the runtime handles crypto,
transport, reconnection, rate limiting, idempotency, and key management.

A bot is a headless long-lived process. It runs on the operator's machine.
It connects to the server as its own identity. It does not hold room
encryption material unless it has been granted read_content in a room.

The author never writes:

· MLS code
· Publisher key derivation
· Command decryption
· Signature verification
· Key transparency verification
· Sockudo connection management
· Rate limit backoff
· Avatar uploads

The author writes:

· Handlers for triggers, events, and commands
· Settings declarations
· Command declarations
· Capability declarations
· Response logic

---

2. Package structure

```
<repo>/
├── pnpm-workspace.yaml
├── tsconfig.base.json
├── eslint.config.js
├── package.json
├── packages/
│   ├── extend/                    @atoll/extend
│   └── bot/                       @atoll/bot
│       ├── package.json
│       ├── tsconfig.json          extends ../../tsconfig.base.json
│       ├── tsconfig.build.json
│       ├── eslint.config.js       extends ../../eslint.config.js
│       ├── src/
│       │   ├── index.js           public entry
│       │   ├── cli.js             binary entry
│       │   ├── types.js           JSDoc-only typedefs, no runtime
│       │   ├── errors.js
│       │   ├── define-bot.js
│       │   ├── define-settings.js
│       │   ├── define-command.js
│       │   ├── define-commands.js
│       │   ├── define-trigger.js
│       │   ├── runtime/
│       │   │   ├── index.js
│       │   │   ├── transport/     WebSocket, SSE, HTTP
│       │   │   ├── crypto/        publisher, command-result, settings, signing
│       │   │   ├── idempotency/
│       │   │   ├── triggers/      webhook server, cron scheduler
│       │   │   ├── keystore/
│       │   │   ├── storage/
│       │   │   └── diagnostics/
│       │   └── testing/
│       │       ├── index.js
│       │       ├── create-test-ctx.js
│       │       ├── create-test-runtime.js
│       │       └── mock-server.js
│       ├── tests/
│       │   ├── canary/
│       │   │   ├── settings.js
│       │   │   └── args.js
│       │   ├── unit/
│       │   ├── integration/
│       │   ├── batch-manifest.toml
│       │   └── manifest-check.js
│       └── dist/                  .d.ts output, gitignored
```

package.json:

```json
{
  "name": "@atoll/bot",
  "type": "module",
  "engines": { "node": ">=22.22.2" },
  "main": "./src/index.js",
  "types": "./dist/index.d.ts",
  "bin": { "atoll-bot": "./src/cli.js" },
  "exports": {
    ".": {
      "types": "./dist/index.d.ts",
      "import": "./src/index.js"
    },
    "./testing": {
      "types": "./dist/testing.d.ts",
      "import": "./src/testing/index.js"
    }
  },
  "files": ["src", "dist", "README.md", "LICENSE"],
  "scripts": {
    "typecheck": "tsc --noEmit",
    "build": "tsc -p tsconfig.build.json",
    "test:canary": "tsc --noEmit",
    "test:unit": "node --test tests/unit/",
    "test:integration": "node --test tests/integration/",
    "test": "pnpm test:canary && pnpm test:unit && pnpm test:integration",
    "lint": "eslint src tests",
    "validate": "pnpm lint && pnpm test && pnpm build"
  }
}
```

Zero runtime dependencies.

2.1 tsconfig.base.json

```json
{
  "compilerOptions": {
    "checkJs": true,
    "allowJs": true,
    "module": "es2022",
    "moduleResolution": "bundler",
    "target": "es2024",
    "lib": ["ES2024"],
    "noEmit": true,
    "resolveJsonModule": true,
    "esModuleInterop": true,
    "allowSyntheticDefaultImports": true,
    "skipLibCheck": true,

    "strict": true,
    "exactOptionalPropertyTypes": true,
    "noUncheckedIndexedAccess": true,
    "noImplicitOverride": true,
    "noFallthroughCasesInSwitch": true,
    "verbatimModuleSyntax": true,

    "types": ["node"]
  }
}
```

strict: true is a hard requirement. Every guarantee in this specification
- including @ts-expect-error canaries, duplicate-property detection
(TS1117), and per-key inference - is checker-enforced.

Module mode: es2022 with bundler resolution. Under this mode, a .js file
containing no top-level import or export is classified as a script. The
typedefs declared in src/types.js are therefore globally visible, and
downstream JSDoc can reference them by bare name without an import path.
This is required by the style used in §3 and §4.

Relative imports use explicit .js extensions by convention, for
compatibility with Node ESM consumers of the emitted declarations. The
amended module mode does not require them; the convention is maintained
anyway.

2.2 tsconfig.build.json

```json
{
  "extends": "./tsconfig.json",
  "compilerOptions": {
    "noEmit": false,
    "declaration": true,
    "emitDeclarationOnly": true,
    "outDir": "./dist"
  },
  "include": ["src/**/*.js"]
}
```

.d.ts is generated from JSDoc. Single source of truth. No hand-authored
.d.ts.

2.3 ESLint

Root config supplies the base. Package config extends it and adds:

· import/no-restricted-paths: src/runtime/ cannot import from
  src/testing/; src/testing/ cannot import from src/runtime/internal/.
· import/enforce-node-protocol-usage: forces node:fs over fs.

The root config's jsdoc/no-undefined-types list contains the names of the
typedefs declared in src/types.js. Types are added to the list when they
first trigger an error. The full initial list:

Capability, Mode, SettingKind, ArgKind, LogLevel, Avatar, RoomRef,
MessageRef, AttachmentRef, SettingDecl, SettingValue, SettingsDecl,
ArgDecl, ArgValue, ArgOf, ArgsReader, CommandResult, WebhookTrigger,
ScheduleTrigger, TriggerDecl, SettingsStore, StorageStore, RoomsStore,
PostOptions, ReplyOptions, LocalOptions, BotCtx, MessageDataMember,
MessageDataObserver, MessageDataDeleted, MessageEvent, HandlerMap,
CommandDecl, BotConfig, Bot.

All public JSDoc carries descriptions. Every @param, @returns, and
@property tag has a description after its type, separated by ` - `
(space, hyphen, space). The description rules
(jsdoc/require-param-description, jsdoc/require-returns-description,
jsdoc/require-property-description) are enabled at workspace scope and
are not disabled at package scope.
jsdoc/require-hyphen-before-param-description is set to 'always'.

Descriptions are short and concrete. If a tag's meaning is obvious from
its name and type, a one-clause sentence still applies: say what it
means.

---

3. Type surface

All public typedefs live in src/types.js. No runtime. The file exists
solely to emit into dist/types.d.ts. The file is a script: no imports, no
exports, no runtime code.

3.1 Vocabulary types

```js
/**
 * The eight capability scopes a bot may declare.
 * @typedef {'post_message' | 'post_attachment' | 'post_reaction'
 *   | 'read_commands' | 'read_metadata' | 'read_content'
 *   | 'edit_message' | 'delete_message'} Capability
 */

/**
 * Derived per-room mode. Never authored, never displayed.
 * @typedef {'write_only' | 'observer' | 'member'} Mode
 */

/**
 * Setting field types.
 * @typedef {'text' | 'secret' | 'number' | 'boolean' | 'select'
 *   | 'multiselect' | 'room-select' | 'user-select'
 *   | 'time-range' | 'duration' | 'color' | 'file'} SettingKind
 */

/**
 * Command argument types.
 * @typedef {'string' | 'number' | 'boolean' | 'select'} ArgKind
 */

/** @typedef {'debug' | 'info' | 'warn' | 'error'} LogLevel */

/**
 * A bot's avatar. Either a file reference or an emoji glyph.
 * @typedef {object} Avatar
 * @property {string | null} fileId - File reference for an uploaded avatar
 *   image. Null when no file is set.
 * @property {string | null} emoji - Emoji glyph used as a fallback avatar.
 *   Null when no emoji is set.
 */

/**
 * A room reference.
 * @typedef {object} RoomRef
 * @property {string} id - The room's server-assigned identifier.
 * @property {string | null} displayName - The room's display name, or null
 *   when none is set.
 * @property {number} memberCount - The number of members currently in the
 *   room.
 */

/**
 * A message reference.
 * @typedef {object} MessageRef
 * @property {string} id - The message's server-assigned identifier.
 * @property {string} roomId - The room the message was posted to.
 * @property {string} createdAt - ISO 8601 timestamp of when the message
 *   was created.
 */

/**
 * An attachment reference.
 * @typedef {object} AttachmentRef
 * @property {string} fileId - The attachment's content-addressed file
 *   identifier.
 * @property {number} size - The attachment's size in bytes.
 * @property {string} contentType - The attachment's MIME type.
 * @property {string} [thumbnailFileId] - Optional file identifier for a
 *   thumbnail, when one exists.
 */
```

3.2 Settings

```js
/**
 * A setting declaration.
 * @typedef {object} SettingDecl
 * @property {string} label - Human-readable label shown in the client's
 *   settings UI.
 * @property {SettingKind} type - The setting's field type. Determines the
 *   value shape and the client's input control.
 * @property {'user' | 'room'} [scope] - Where the setting is stored.
 *   Omit for user scope. Room scope makes the setting per-room.
 * @property {true} [required] - When true, the setting must have a value
 *   before the bot can run. Omit to make the setting optional.
 * @property {unknown} [default] - Default value used when the setting
 *   has not been set.
 * @property {string[]} [options] - Allowed values when type is 'select'
 *   or 'multiselect'.
 * @property {string} [placeholder] - Placeholder text shown in the
 *   input.
 * @property {string} [hint] - Short hint shown below the input.
 */

/**
 * Maps a setting declaration's `type` literal to its value type.
 * @template D - The setting declaration.
 * @typedef {D extends { type: 'secret' } ? string
 *   : D extends { type: 'text' } ? string
 *   : D extends { type: 'number' } ? number
 *   : D extends { type: 'boolean' } ? boolean
 *   : D extends { type: 'select' } ? string
 *   : D extends { type: 'multiselect' } ? string[]
 *   : D extends { type: 'room-select' } ? string
 *   : D extends { type: 'user-select' } ? string
 *   : D extends { type: 'time-range' } ? { start: string, end: string }
 *   : D extends { type: 'duration' } ? number
 *   : D extends { type: 'color' } ? string
 *   : D extends { type: 'file' } ? string
 *   : never} SettingValue
 */

/**
 * A map of setting keys to their declarations.
 * @typedef {Record<string, SettingDecl>} SettingsDecl
 */
```

3.3 Commands

```js
/**
 * A command argument declaration.
 * @typedef {object} ArgDecl
 * @property {ArgKind} type - The argument's value type.
 * @property {true} [required] - When true, the argument must be supplied.
 *   Omit the field to make the argument optional.
 * @property {string[]} [options] - Allowed values when type is 'select'.
 * @property {string} [description] - Human-readable hint shown in the
 *   slash palette.
 */

/**
 * Maps an argument declaration's `type` literal to its value type.
 * @template D - The argument declaration.
 * @typedef {D extends { type: 'string' } ? string
 *   : D extends { type: 'number' } ? number
 *   : D extends { type: 'boolean' } ? boolean
 *   : D extends { type: 'select' } ? string
 *   : never} ArgValue
 */

/**
 * Resolves the value type a handler receives for an argument, adding
 * `undefined` when the argument is optional.
 * @template D - The argument declaration.
 * @typedef {D extends { required: true } ? ArgValue<D> : ArgValue<D> | undefined} ArgOf
 */

/**
 * Reader passed to command handlers. Provides typed access to the
 * command's arguments by name.
 * @template {Record<string, ArgDecl>} D - The argument declaration map.
 * @typedef {object} ArgsReader
 * @property {<K extends keyof D & string>(key: K) => ArgOf<D[K]>} get -
 *   Returns the value of the named argument, or `undefined` when the
 *   argument is optional and was not supplied.
 */

/**
 * The value a command handler returns to the runtime. The runtime
 * routes each variant to the appropriate channel.
 * @typedef {
 *   { type: 'local_message', content: string }
 *   | { type: 'room_message', content: string,
 *       attachments?: AttachmentRef[], replyTo?: string }
 *   | { type: 'toast', content: string }
 *   | { type: 'panel', content: string }
 *   | { type: 'none' }
 * } CommandResult
 */
```

required?: true is not a mistake. required?: boolean would widen true to
boolean, breaking D[K] extends { required: true }. Explicit required:
false is a type error by design. Omit the field to make an argument
optional.

3.4 Triggers

```js
/**
 * A webhook trigger. The runtime binds an HTTP server and dispatches
 * matching requests to the bot's `webhook` handler.
 * @typedef {object} WebhookTrigger
 * @property {'webhook'} type - Discriminator for the webhook variant.
 * @property {string} path - URL path the webhook listens on, relative to
 *   the configured base path.
 * @property {'POST' | 'PUT' | 'PATCH'} [method] - HTTP method accepted.
 *   Defaults to POST.
 * @property {string} [secret] - Name of an environment variable holding
 *   the shared secret used to verify the request signature. The variable
 *   name ending determines the verification scheme: `_SECRET` for HMAC,
 *   `_TOKEN` for bearer.
 * @property {string} [idempotency] - Where to read the idempotency key
 *   from. `header:<name>` reads the named request header. Omit to
 *   generate a per-request UUID.
 * @property {number} [retries] - Number of retry attempts on handler
 *   failure. Defaults to zero.
 * @property {number} [retryDelayMs] - Delay in milliseconds between
 *   retries.
 */

/**
 * A schedule trigger. The runtime's cron engine dispatches matching
 * ticks to the bot's `schedule` handler.
 * @typedef {object} ScheduleTrigger
 * @property {'schedule'} type - Discriminator for the schedule variant.
 * @property {string} name - Stable name for the trigger. Used in
 *   idempotency keys and runtime state.
 * @property {string} cron - Five-field cron expression. No seconds. No
 *   macros.
 * @property {string} [timezone] - IANA timezone name. Defaults to UTC.
 */

/**
 * A trigger declaration. Either a webhook or a schedule.
 * @typedef {WebhookTrigger | ScheduleTrigger} TriggerDecl
 */
```

3.5 Context and bot

```js
/**
 * Read and write access to a bot's settings, scoped per user or per
 * room. Keys are the author's setting names. Values are typed by the
 * declaration.
 * @template {Record<string, SettingDecl>} S - The bot's settings
 *   declaration map.
 * @typedef {object} SettingsStore
 * @property {<K extends keyof S & string>(key: K, opts?: { room?: string }) =>
 *   Promise<SettingValue<S[K]> | undefined>} get - Reads a setting. Pass
 *   `{ room }` for a room-scoped setting.
 * @property {<K extends keyof S & string>(key: K, value: SettingValue<S[K]>,
 *   opts?: { room?: string }) => Promise<void>} set - Writes a setting.
 *   Pass `{ room }` for a room-scoped setting.
 * @property {(key: string, opts?: { room?: string }) => Promise<void>} delete -
 *   Deletes a setting. Pass `{ room }` for a room-scoped setting.
 * @property {<K extends keyof S & string>(key: K,
 *   cb: (value: SettingValue<S[K]> | undefined) => void,
 *   opts?: { room?: string }) => () => void} subscribe - Subscribes to
 *   changes for a setting. Returns an unsubscribe function.
 */

/**
 * Bot-scoped key/value storage. Persistent, encrypted at rest, local to
 * the runtime machine. Not synced.
 * @typedef {object} StorageStore
 * @property {(key: string) => Promise<unknown>} get - Reads a value by
 *   key. Returns undefined when the key is not present.
 * @property {(key: string, value: unknown) => Promise<void>} set -
 *   Writes a value by key.
 * @property {(key: string) => Promise<void>} delete - Deletes a value
 *   by key.
 * @property {() => Promise<void>} clear - Removes all author storage.
 *   Runtime state under the reserved prefix is not affected.
 */

/**
 * Read access to the rooms the bot has been granted into.
 * @typedef {object} RoomsStore
 * @property {() => Promise<RoomRef[]>} list - Returns every room the
 *   bot is currently granted into.
 * @property {(roomId: string) => Promise<RoomRef | null>} get - Returns
 *   a single room by identifier, or null when the bot is not granted
 *   into it.
 */

/**
 * Options for posting a message into a room.
 * @typedef {object} PostOptions
 * @property {string} [roomId] - Target room. Defaults to the invoking
 *   room when the bot was invoked from a room-scoped context.
 * @property {string} [text] - Message text.
 * @property {AttachmentRef[]} [attachments] - Attachments to include.
 * @property {string} [replyTo] - Message id to reply to.
 */

/**
 * Options for replying to a message. Same as PostOptions, with the
 * `replyTo` field required.
 * @typedef {PostOptions & { replyTo: string }} ReplyOptions
 */

/**
 * Options for a local message. Local messages are visible only to the
 * bot's owner.
 * @typedef {object} LocalOptions
 * @property {string} text - Message text.
 * @property {'text' | 'markdown'} [format] - Rendering format. Defaults
 *   to text.
 */

/**
 * Invocation context. Reconstructed per handler call. Not a stable
 * identity. Fields are populated per invocation kind and derived mode.
 *
 * @template {Record<string, SettingDecl>} [S=Record<string, SettingDecl>] -
 *   The bot's settings declaration map.
 * @typedef {object} BotCtx
 * @property {{ id: string, label: string, ownerUserId: string,
 *   avatar: Avatar }} bot - Metadata about the running bot.
 * @property {{ roomId: string, mode: Mode,
 *   scopes: Capability[] } | null} grant - The grant context for the
 *   current room, or null when the invocation has no room scope.
 * @property {RoomRef | null} room - The current room, or null when the
 *   invocation has no room scope.
 * @property {{ id: string, type: string, roomId: string | null,
 *   timestamp: string } | null} event - Metadata about the triggering
 *   event, or null when the invocation is not event-driven.
 * @property {SettingsStore<S>} settings - Read and write access to the
 *   bot's settings.
 * @property {StorageStore} storage - Bot-scoped persistent storage.
 * @property {RoomsStore} rooms - Read access to granted rooms.
 * @property {(opts: PostOptions) => Promise<MessageRef>} post - Posts a
 *   message into a room.
 * @property {(opts: ReplyOptions) => Promise<MessageRef>} reply - Posts
 *   a reply to a specific message.
 * @property {(opts: LocalOptions) => Promise<void>} sendLocal - Sends a
 *   local message to the bot's owner.
 * @property {(level: LogLevel, message: string, meta?: object) => void} log -
 *   Writes a structured log line. Redaction is enforced by the runtime.
 * @property {(url: string, opts?: RequestInit) => Promise<Response>} fetch -
 *   Performs an HTTP request. Query strings are stripped from logs.
 * @property {(url: string, opts?: RequestInit) => Promise<Response>} fetchUserUrl -
 *   Performs an HTTP request on behalf of a user. Subject to the same
 *   logging redaction as `fetch`.
 * @property {AbortSignal} signal - Aborted when the runtime shuts down
 *   or the handler times out.
 * @property {(path: string) => Promise<string>} uploadAvatar - Uploads
 *   an avatar file and returns its file identifier.
 */
```

3.6 Handlers

```js
/**
 * Message data seen by a member-mode bot. Content is decrypted.
 * @typedef {object} MessageDataMember
 * @property {string} id - The message's identifier.
 * @property {string} senderUserId - The sender's user identifier.
 * @property {string} senderClientId - The sending device's client
 *   identifier.
 * @property {string} createdAt - ISO 8601 timestamp of the message.
 * @property {string | null} plaintext - The decrypted message text, or
 *   null when the message is a commit or proposal.
 * @property {AttachmentRef[]} attachments - Attachments included with
 *   the message.
 * @property {string | null} replyTo - The id of the message this one
 *   replies to, or null.
 */

/**
 * Message data seen by an observer-mode bot. Content is not available.
 * @typedef {object} MessageDataObserver
 * @property {string} id - The message's identifier.
 * @property {string} senderUserId - The sender's user identifier.
 * @property {string} senderClientId - The sending device's client
 *   identifier.
 * @property {string} createdAt - ISO 8601 timestamp of the message.
 * @property {number} sizeBytes - The size of the message ciphertext in
 *   bytes.
 */

/**
 * Message data for a deletion event.
 * @typedef {object} MessageDataDeleted
 * @property {string} id - The identifier of the deleted message.
 */

/**
 * A message event delivered to the bot's `message` handler.
 * @typedef {object} MessageEvent
 * @property {string} id - The event's identifier.
 * @property {'message.new' | 'message.edited' | 'message.deleted'} type -
 *   The event type.
 * @property {string} roomId - The room the event occurred in.
 * @property {string} timestamp - ISO 8601 timestamp of the event.
 * @property {MessageDataMember | MessageDataObserver | MessageDataDeleted} data -
 *   The event payload. Shape depends on the bot's mode.
 */

/**
 * The set of handlers a bot may declare. Every handler is optional. At
 * least one must be present.
 * @template {Record<string, SettingDecl>} S - The bot's settings
 *   declaration map.
 * @typedef {object} HandlerMap
 * @property {(ctx: BotCtx<S>) => void | Promise<void>} [install] - Called
 *   once when the runtime starts.
 * @property {(ctx: BotCtx<S>) => void | Promise<void>} [uninstall] -
 *   Called once when the runtime stops.
 * @property {(ctx: BotCtx<S>,
 *   event: { botId: string, roomId: string, oldMode: Mode, newMode: Mode,
 *            scopes: Capability[], changed: string[] }) =>
 *   void | Promise<void>} [grantUpdated] - Called when the bot's grant in
 *   a room changes.
 * @property {(ctx: BotCtx<S>, event: MessageEvent) => void | Promise<void>} [message] -
 *   Called for message events in rooms the bot is granted into.
 * @property {(ctx: BotCtx<S>,
 *   event: { id: string, type: string, roomId: string, timestamp: string,
 *            data: object }) => void | Promise<void>} [room] - Called for
 *   non-message room events.
 * @property {(ctx: BotCtx<S>,
 *   payload: { path: string, body: string, headers: Record<string, string> }) =>
 *   void | Promise<void>} [webhook] - Called for webhook trigger hits.
 * @property {(ctx: BotCtx<S>, payload: { name: string }) =>
 *   void | Promise<void>} [schedule] - Called for schedule trigger hits.
 */
```

3.7 Bot config

```js
/**
 * A command declaration.
 * @template {Record<string, ArgDecl>} D - The argument declaration map.
 * @template {Record<string, SettingDecl>} S - The bot's settings
 *   declaration map.
 * @typedef {object} CommandDecl
 * @property {string} [description] - Human-readable description shown in
 *   the slash palette.
 * @property {D} args - The command's argument declarations.
 * @property {(ctx: BotCtx<S>, args: ArgsReader<D>) =>
 *   CommandResult | Promise<CommandResult>} handler - Called when the
 *   command is invoked.
 */

/**
 * The bot's configuration, as passed to `defineBot`.
 * @template {Record<string, SettingDecl>} S - The bot's settings
 *   declaration map.
 * @template {Record<string, CommandDecl<any, any>>} C - The bot's
 *   command collection.
 * @template {readonly TriggerDecl[]} T - The bot's trigger list.
 * @typedef {object} BotConfig
 * @property {string} id - The bot's identifier. Matches
 *   `^[a-z][a-z0-9.-]*$` and contains at least one dot.
 * @property {string} apiVersion - The SDK version the bot was authored
 *   against.
 * @property {string} hostApi - The runtime host API version the bot
 *   requires. Must match the runtime's version.
 * @property {string} label - Human-readable label shown in the client's
 *   bot list.
 * @property {Capability[]} capabilities - The bot's declared capability
 *   ceiling. Grants must be a subset.
 * @property {Avatar} [avatar] - The bot's avatar. File or emoji.
 * @property {S} [settings] - The bot's settings declarations.
 * @property {C} [commands] - The bot's commands.
 * @property {T} [triggers] - The bot's triggers.
 * @property {HandlerMap<S>} handlers - The bot's handlers. At least one
 *   must be present.
 */

/**
 * The value returned by `defineBot`.
 * @template {Record<string, SettingDecl>} S - The bot's settings
 *   declaration map.
 * @template {Record<string, CommandDecl<any, any>>} C - The bot's
 *   command collection.
 * @template {readonly TriggerDecl[]} T - The bot's trigger list.
 * @typedef {object} Bot
 * @property {BotConfig<S, C, T>} config - The validated configuration.
 */
```

---

4. Factories

4.1 defineSettings

```js
/**
 * Declares a bot's settings. Property names become setting keys.
 * The constraint on `Ds` preserves the `type` literals so that
 * `ctx.settings.get('apiToken')` resolves to `string`.
 *
 * @template {Record<string, SettingDecl>} Ds - The settings declaration.
 * @param {Ds} decls - The settings declaration. Keys are author-chosen
 *   setting names.
 * @returns {Ds} The same declaration, unchanged. The return exists for
 *   type inference.
 */
export function defineSettings (decls) {
  return decls
}
```

4.2 defineCommand

```js
/**
 * Declares a command. `D` is inferred from the sibling `args`
 * property and flows into the handler's `args` parameter via
 * direct structured substitution.
 *
 * The handler's `ctx` parameter is not typed against the bot's
 * settings declaration. See §8.
 *
 * @template {Record<string, ArgDecl>} D - The argument declaration map.
 * @param {CommandDecl<D, any>} decl - The command declaration.
 * @returns {CommandDecl<D, any>} The same declaration, unchanged. The
 *   return exists for type inference.
 */
export function defineCommand (decl) {
  return decl
}
```

4.3 defineCommands

```js
/**
 * Shape check for the commands collection. Per-command inference
 * lives in `defineCommand`, one level down. Do not attempt to
 * infer per-command types here - see the canary at
 * tests/canary/args.js for why.
 *
 * @template {Record<string, CommandDecl<any, any>>} C - The commands
 *   collection.
 * @param {C} commands - The commands collection.
 * @returns {C} The same collection, unchanged. The return exists for
 *   type inference.
 */
export function defineCommands (commands) {
  return commands
}
```

4.4 defineTrigger

```js
/**
 * Identity function. Shape-checked by the argument.
 *
 * @param {TriggerDecl} trigger - The trigger declaration.
 * @returns {TriggerDecl} The same declaration, unchanged.
 */
export function defineTrigger (trigger) {
  return trigger
}
```

4.5 defineBot

```js
/**
 * Top-level factory. Threads the settings declaration (S) into every
 * top-level handler's `ctx`, and the commands (C) and triggers (T)
 * into the config.
 *
 * @template {Record<string, SettingDecl>} S - The bot's settings
 *   declaration map.
 * @template {Record<string, CommandDecl<any, any>>} C - The bot's
 *   command collection.
 * @template {readonly TriggerDecl[]} T - The bot's trigger list.
 * @param {BotConfig<S, C, T>} config - The bot's configuration.
 * @returns {Bot<S, C, T>} The bot handle.
 */
export function defineBot (config) {
  validateConfig(config)
  return /** @type {any} */ ({ config })
}
```

4.6 Validation at startup

validateConfig fails startup on:

· id does not match ^[a-z][a-z0-9.-]*$ or contains no dot.
· capabilities is empty or contains a scope outside the vocabulary.
· Any command name is not a valid identifier (^[a-z][a-z0-9-]*$).
· Any setting key is not a valid identifier or starts with _runtime:.
· Any webhook trigger's secret env var is not set.
· Any schedule trigger's cron or timezone is invalid.
· No handlers are declared.
· hostApi does not match the runtime's version.

Validation collects all failures and reports them at once. Startup
aborts.

---

5. BotCtx - populated fields per mode

Field          write_only   observer   member
bot            ✓            ✓          ✓
grant          ✓            ✓          ✓
room           room-scoped  room-scoped  room-scoped
event          null for message/room     metadata only   plaintext
settings       ✓            ✓          ✓
storage        ✓            ✓          ✓
rooms          ✓            ✓          ✓
post           ✓            ✓          ✓
reply          ✓            ✓          ✓
sendLocal      ✓            ✓          ✓
log            ✓            ✓          ✓
fetch          ✓            ✓          ✓
fetchUserUrl   ✓            ✓          ✓
signal         ✓            ✓          ✓
uploadAvatar   ✓            ✓          ✓

Message and room handlers are never invoked in write_only. The author
narrows event.data:

```js
message: async (ctx, event) => {
  if ('plaintext' in event.data) {
    // member mode; event.data.plaintext is string | null
  } else if ('sizeBytes' in event.data) {
    // observer mode; metadata only
  }
}
```

---

6. Response methods

6.1 ctx.post

Mode        Encryption                                         Signing
write_only  Publisher key ECDH (ephemeral X25519 per message)  Ed25519 over length-prefixed payload
observer    Same                                               Same
member      MLS application message                            MLS handles authentication

Write-only and observer path:

1. Fetch or use cached publisher key for the room's current epoch.
2. Verify the signer's identity_pubkey against the KT log.
3. shared = ECDH(ephemeral_priv, publisher_public)
4. key = HKDF-Expand(shared, "publisher-message-v1", 32)
5. Encrypt with AES-256-GCM. Wire: ephemeral_pub(32) || nonce(12) || ct || tag(16).
6. Sign length-prefixed payload with bot_identity_private_key (Server §8.10).
7. POST /rooms/:id/bot-messages with { epoch, ciphertext, content_type: "bot", signature, request_id }.

Retry: on 5xx, network failure, or 429, retry up to three times with
exponential backoff (2s, 5s, 15s). 429 respects Retry-After. Idempotent
via request_id. On final failure, rejects with PostFailedError.

6.2 ctx.reply

Sugar over ctx.post with replyTo required.

6.3 ctx.sendLocal

Wire: POST /bots/me/messages with target: "owner",
result_type: "local_message", ciphertext, bot_result_pubkey, request_id.

Encryption: X25519 ephemeral per message.
key = HKDF-Expand(shared, "bot-command-result-v1", 32). AES-256-GCM.
Wire: nonce(12) || ct || tag(16).

Non-durable. Best-effort. If the owner is offline, the message is lost.
For anything the owner must see, use ctx.reply() in a room.

Owner pubkey transport: interim uses a synthetic command (__owner_local__).
Pending server resolution of reconciliation §7.1. Marked for removal.

6.4 Command result

Result type    Wire path
local_message  POST /bots/me/messages target: "invoker" result_type: "local_message"
toast          Same, result_type: "toast"
panel          Same, result_type: "panel"
room_message   ctx.post({ text: content, attachments, replyTo })
none           No result; ack only

Fresh ephemeral X25519 keypair per result. ephemeral_result_pubkey
arrives on bot.command_invoked. Bot derives:

```
shared = ECDH(bot_result_priv, ephemeral_result_pub)
key    = HKDF-Expand(shared, "bot-command-result-v1", 32)
```

Encrypt with AES-256-GCM. Wire: nonce(12) || ct || tag(16).

Non-durable. If the invoking client is offline, the result is lost.
Authors needing guaranteed delivery use ctx.reply().

Associated data: none, per reconciliation §3.1. Interim. If the server
team adds AAD, one line changes.

---

7. Settings runtime

7.1 Scope

Scope  Storage key                 Visible to
user   key                         All devices of the operator
room   room:{room_id}:key          All devices of the operator, scoped to a room

The runtime handles prefixing. The author writes
ctx.settings.get('webhookSecret', { room: 'r_abc123' }).

7.2 Encryption

Non-secret. Two ciphertexts:

· value_encrypted_client - operator's preferences_key, standard
  preferences pattern.
· value_encrypted_bot - X25519 ECDH between the client's ephemeral key
  and the bot's bot_command_pubkey.

Secret. One ciphertext: value_encrypted_bot only. No client-readable
copy. No local plaintext mirror.

Wire format for value_encrypted_bot:

```
base64url(ephemeral_pub(32) || nonce(12) || ct || tag(16))
```

key = HKDF-Expand(shared, "bot-settings-v1", 32). Bot decrypts with
bot_command_private_key.

7.3 Subscribe

ctx.settings.subscribe(key, cb) returns an unsubscribe function. Fires
when bot.settings_updated arrives on the bot channel. Runtime refetches
affected keys. Coalesced within 500ms.

7.4 Reserved keys

Author keys must not start with _runtime:. Rejected with
SettingsError('reserved_prefix').

---

8. Command handler ctx.settings limitation

Inside defineCommand, the handler's args parameter is typed precisely.
ctx.settings.get('apiToken') is not - it returns the union of every
possible setting value type.

Why. defineCommand is called before defineBot. At the time
defineCommand infers D, the settings type S is not yet known.
TypeScript does not retroactively re-type callbacks. Top-level handlers
inside the defineBot call do get precise S.

Canonical pattern - runtime type guard.

```js
handler: async (ctx, args) => {
  const raw = await ctx.settings.get('apiToken')
  if (typeof raw !== 'string') {
    return { type: 'local_message', content: 'Missing or invalid API token.' }
  }
  const token = raw // string
  // ...
}
```

This is the only option that does not lie to the type system. Verbose
but honest.

Alternative: explicit cast - unsafe, rejected. Hoisting to a top-level
handler - moves the problem to storage, rejected.

A builder pattern may eliminate this in v2. Parked.

---

9. Storage

ctx.storage is bot-scoped, persistent, encrypted at rest with a key
derived from the bot's MLS identity private key via HKDF
("bot-storage-v1", 32 bytes). AES-256-GCM, nonce prepended. Not synced.
Local to the runtime machine.

_runtime: prefix is reserved. ctx.storage.clear() removes all author
storage. Runtime state under _runtime:* is untouched.

---

10. Triggers

10.1 Webhook

```js
triggers: [
  defineTrigger({
    type: 'webhook',
    path: '/github',
    method: 'POST',
    secret: 'GITHUB_WEBHOOK_SECRET',
    idempotency: 'header:x-github-delivery',
    retries: 0,
    retryDelayMs: 1000
  })
]
```

Runtime binds an HTTP server. Secret verification:

· Env var name ends in _SECRET: HMAC-SHA256 of the raw body, compared
  against the vendor header (X-Hub-Signature-256 for GitHub,
  X-Signature-256 generic). Constant-time compare.
· Env var name ends in _TOKEN: Authorization: Bearer <value>.
  Constant-time compare.
· Env var missing: startup fails.

Body passed to the handler as a string. Runtime does not parse.

10.2 Schedule

```js
defineTrigger({
  type: 'schedule',
  name: 'daily-digest',
  cron: '0 9 * * *',
  timezone: 'UTC'
})
```

Internal cron engine. Five fields. No seconds. No macros. DST: skipped
hour - no fire; repeated hour - fires once. Missed fires: default
skipped; catch_up = true fires once per miss on boot with a flag.

10.3 Idempotency

Five-minute window.

Trigger     Key
Webhook     header:<name> value, or generated UUID
Schedule    <name>:<ISO timestamp>
Command     Server-assigned command_id
Message     event.id

Persisted to ctx.storage under _runtime:idempotency:*. Replayed on
restart.

---

11. Handlers

Top-level handlers declared in handlers. Every handler receives ctx
typed with the bot's S.

At least one handler must be declared. Exceptions are logged and do not
disable the bot. Three consecutive failures within one minute pause the
bot; the operator is notified via bot.paused on their user channel. The
runtime does not see the pause event and treats it as indistinguishable
from server unavailability.

---

12. Errors

Every runtime-raised error carries a code string.

Code                              Trigger                                              Recoverable
validation_error                  Structural failure in defineBot                      No
capability_unknown                Capability outside vocabulary                        No
scope_not_declared                Grant asks for undeclared scope                      No
scope_dependency_missing          Grant lacks prerequisite                             No
post_failed                       ctx.post retries exhausted                           Author may catch
reply_requires_reply_to           ctx.reply without replyTo                            No
settings_reserved_prefix          Setting key starts with _runtime:                    No
settings_decrypt_failed           Bot settings ciphertext malformed                    No
settings_write_failed             Settings PATCH non-2xx                               Retry
storage_reserved_prefix           Storage key starts with _runtime:                    No
storage_capacity_exceeded         Storage backend full                                 Operator action
publisher_key_unavailable         No key for room's current epoch                      Retry after epoch advance
publisher_key_verification_failed Signature invalid or signer unknown                  No
command_result_encrypt_failed     Malformed ephemeral_result_pubkey                    No
key_rotation_failed               PATCH /bots/:id/keys non-2xx                         Retry
keystore_locked                   Keystore secret missing or wrong                     Operator action
keystore_corrupt                  Keystore unreadable                                  Operator action
rate_limited                      Local throttle would exceed server limit             Retry after backoff
webhook_secret_missing            Configured secret env var not set                    Operator action
webhook_signature_invalid         HMAC verification failed                             No

BotError is the base class. Every code is a subclass. err.code is
stable. err.message is human-readable and may change.

---

13. Testing entry point - @atoll/bot/testing

13.1 createTestCtx

```js
/**
 * Creates a BotCtx populated for tests. Response methods capture their
 * inputs to `calls` instead of sending. No crypto is exercised.
 *
 * @template {Record<string, SettingDecl>} [S=Record<string, SettingDecl>] -
 *   The settings declaration map.
 * @param {object} [overrides] - Fields to override on the created
 *   context.
 * @param {S} [overrides.settingsDecl] - The settings declaration to
 *   populate `settings` from.
 * @param {Record<string, unknown>} [overrides.settings] - Initial
 *   setting values.
 * @param {{ roomId: string, mode: Mode, scopes: Capability[] }} [overrides.grant] -
 *   The grant to populate the context with.
 * @param {RoomRef} [overrides.room] - The room to populate.
 * @param {Record<string, unknown>} [overrides.storage] - Initial
 *   storage values.
 * @returns {BotCtx<S> & { calls: {
 *   post: PostOptions[],
 *   reply: ReplyOptions[],
 *   sendLocal: LocalOptions[]
 * } }} A context with captured-call arrays attached.
 */
export function createTestCtx (overrides) { /* ... */ }
```

All fields populated. Response methods capture to calls instead of
sending. Settings and storage are in-memory. No crypto exercised.

13.2 createTestRuntime

```js
/**
 * Creates a test runtime for a bot. Does not connect to a real server.
 *
 * @param {Bot} bot - The bot to test.
 * @param {object} [options] - Runtime options.
 * @param {{ roomId: string, mode: Mode, scopes: Capability[] }[]} [options.grants] -
 *   Grants to populate.
 * @param {{ validateCrypto?: boolean }} [options.options] - Additional
 *   runtime options.
 * @returns {Promise<TestRuntime>} The test runtime.
 */
export function createTestRuntime (bot, options) { /* ... */ }
```

Methods:

· inject(event) - dispatch to handler.
· dispatchWebhook({ path, body, headers })
· fireSchedule(name)
· advanceTime(ms)
· restart() - preserve storage and idempotency.
· simulateResponse(path, { status, body })
· calls - captured outbound.
· getPendingOutbound()
· getGrants(), getPublisherKeys()

13.3 Mock server

@atoll/bot/testing/mock-server implements every bot-facing endpoint.
Default: accept any ciphertext, record calls. With
{ validateCrypto: true }: verify signatures, parse embedded structures,
validate wire format.

Does not exercise real MLS, real KT inclusion proofs, or real publisher
key derivation. Those require a sandbox server.

---

14. Runtime specification

14.1 CLI

All commands run as atoll-bot <command>.

Command              Purpose
register <path>      Register a new bot account
login                Cache an operator session
logout               Clear the cached operator session
validate <path>      Validate without connecting
run <path>           Start the runtime
rotate-keys          Generate new keypairs, PATCH, replace keystore entries
avatar <path>        Upload or replace the bot's avatar
export-keys [--out <path>]  Export the keystore
import-keys <path>   Import a keystore
dev <path>           Dev mode; hot reload, verbose logging, mock server
inspect [room-id]    Dump the runtime's state
tail                 Live-stream the structured log, redacted
sandbox connect <url> --token <token>  Register a sandbox bot
sandbox run <path>   Run against the sandbox server
sandbox reset        Delete the sandbox bot

Exit codes: 0 success, 1 user error, 2 internal error.

14.2 Environment variables

Variable                         Required     Default                  Purpose
ATOL_BOT_TOKEN                   Runtime      -                        Bot auth token
ATOL_SERVER_URL                  Runtime      -                        Base URL
ATOL_BOT_HOST                    No           0.0.0.0                  Webhook bind host
ATOL_BOT_PORT                    No           8787                     Webhook bind port
ATOL_BOT_CONFIG                  No           <cwd>/bot.toml           Override file
ATOL_BOT_KEYSTORE                No           <cwd>/bot.keystore       Keystore path
ATOL_BOT_KEYSTORE_SECRET         Yes          -                        Keystore encryption secret
ATOL_USER_TOKEN                  Register only -                       Operator session
ATOL_LOG_LEVEL                   No           info                     debug | info | warn | error
ATOL_LOG_FORMAT                  No           json                     json | pretty
ATOL_RECONNECT_MAX_BACKOFF_MS    No           30000                    Backoff cap
ATOL_RECONNECT_BASE_BACKOFF_MS   No           1000                     Initial backoff
ATOL_HANDLER_TIMEOUT_MS          No           60000                    Per-handler timeout
ATOL_SHUTDOWN_DRAIN_MS           No           30000                    Drain

Precedence: env var > bot.toml > default.

14.3 bot.toml

```toml
[runtime]
log_level = "info"
log_format = "json"

[webhook]
host = "0.0.0.0"
port = 8787
base_path = ""
max_body_bytes = 1048576
timeout_ms = 5000

[cron]
timezone = "UTC"
catch_up = false

[reconnect]
base_backoff_ms = 1000
max_backoff_ms = 30000
jitter = 0.2

[shutdown]
drain_ms = 30000

[diagnostics]
enabled = true
retention_days = 7
```

Parsed with a minimal TOML reader. Validation errors fail startup.

14.4 Webhook HTTP server

Bound at ATOL_BOT_HOST:ATOL_BOT_PORT. Only started if a webhook trigger
is declared.

Request flow:

1. Method must match. Else 405.
2. Path must equal base_path + trigger.path. Else 404.
3. Body read up to max_body_bytes. Exceeded - 413.
4. Secret verification (see §10.1).
5. Idempotency key computed.
6. Dispatch to handlers.webhook.
7. 200 on success. 500 on handler throw. 504 on timeout.

14.5 Cron engine

Internal, no dependency. Five fields. Intl.DateTimeFormat for timezones.
DST handled per §10.2. Missed fires: skipped unless catch_up = true.
State at _runtime:cron:<name>:last_fire.

14.6 Logging

Structured JSON to stdout. Never to file.

```json
{
  "ts": "2026-10-04T12:34:56.789Z",
  "level": "info",
  "msg": "handler completed",
  "bot_id": "b_abc",
  "event_id": "evt_xyz",
  "room_id": "r_123",
  "code": null,
  "duration_ms": 42,
  "meta": {}
}
```

Redaction enforced at the logger, not configurable:

· Setting values never logged (keys are).
· Command ciphertext and plaintext never logged.
· Message plaintext never logged.
· Webhook bodies and headers never logged.
· Authorization headers never logged.
· Private keys and keystore contents never logged.
· Full URLs from ctx.fetch/ctx.fetchUserUrl logged with query string
  stripped.
· Response bodies never logged.

Reserved prefix _secret:. Attempting to log a reserved key throws in dev
mode.

14.7 Reconnection

Exponential backoff with jitter. Attempt 1: 1s ± 20%. Doubles to 30s cap.
No max attempts.

On reconnect:

1. Resubscribe to private-bot-{bot_id}.
2. Refetch grants.
3. Refetch settings.
4. Refetch publisher keys.
5. Replay pending outbound with original request_id.
6. Resume dispatch.

Outbound queue persisted to _runtime:outbound:<request_id>. Bounded at
1000 entries; oldest dropped with warn.

14.8 Graceful shutdown

Triggered by SIGTERM or SIGINT.

1. Stop accepting new invocations.
2. Stop the webhook HTTP server.
3. Stop the cron scheduler.
4. Drain in-flight handlers, up to ATOL_SHUTDOWN_DRAIN_MS.
5. Flush idempotency store.
6. Flush outbound queue.
7. Close WebSocket.
8. Exit 0.

Drain timeout: exit 1. Unflushed state is lost. Keystore and persistent
storage untouched.

14.9 Keystore

File: ATOL_BOT_KEYSTORE (default ./bot.keystore). JSON, encrypted.

```
salt     = random 16 bytes
key      = scrypt(secret, salt, N=2^17, r=8, p=1, 32)
nonce    = random 12 bytes
ct, tag  = AES-256-GCM(key, nonce, plaintext)
file     = { version: 1, salt, nonce, ct: ct || tag }
```

Plaintext contents:

```json
{
  "version": 1,
  "bot_id": "b_abc",
  "bot_token": "...",
  "bot_identity_private": "<base64url, Ed25519 seed>",
  "bot_command_private": "<base64url, X25519 scalar>",
  "identity_private": "<base64url, Ed25519 seed>",
  "storage_seed": "<base64url, 32 bytes>",
  "operator_session": "<opaque, optional>",
  "created_at": "...",
  "rotated_at": "..."
}
```

Secret resolution:

1. ATOL_BOT_KEYSTORE_SECRET env var.
2. OS keychain: macOS Keychain, Linux libsecret, Windows Credential
   Manager. Entry name atoll-bot:<bot_id>.
3. Interactive prompt at first command. Stored to OS keychain if
   available.

Keystore operations shell out to platform tools (security on macOS,
secret-tool on Linux, cmdkey on Windows) rather than depend on a native
module. Slower, dependency-free.

If no secret resolves: keystore_locked. Runtime does not start.

Rotation. Generate three new keypairs. PATCH /bots/:id/keys. On success,
replace private keys and update rotated_at. bot_token, storage_seed, and
operator_session preserved.

Export/import. Same encrypted format. import-keys validates schema.

Loss. Unrecoverable. Operator must delete the bot and re-register.

14.10 Publisher key cache

Per-room, per-epoch. In-memory.

```
Map<room_id, {
  current_epoch: number,
  keys: Map<epoch, {
    publisher_public_key: Uint8Array,
    signer_user_id: string,
    signature: Uint8Array,
    verified_at: number
  }>
}>
```

Eviction: current epoch + two most recent prior epochs. Maximum 100 rooms
cached, LRU.

Verification on first use per (room, epoch). Re-verify after
bot.keys_rotated or kt.snapshot.

Unavailability: ctx.post waits up to 30 seconds, retrying with backoff.
Then publisher_key_unavailable.

14.11 Epoch tracking

Sources, in order of authority:

1. room.publisher_key_updated on private-bot-{bot_id} - authoritative.
2. epoch.updated on private-room-{room_id} - member mode.
3. GET /rooms/:id/publisher-key without ?epoch.
4. ctx.event.epoch on message events - member mode.

On disagreement: refetch.

Outbound encryption uses the current epoch's publisher key. Fallback to
the most recent prior epoch with a warn during the window between MLS
epoch advance and human publisher key publication.

14.12 Diagnostics

Snapshots captured on:

· Handler failure (three consecutive, before pause).
· Keystore warning.
· Reconnect storm (10 consecutive).
· Operator-requested (inspect --snapshot).

Content: timestamp, bot identity, recent redacted log lines, failing
handler name, stack trace, grant states, publisher key cache state,
keystore status.

No setting values, no command plaintext, no message content, no keystore
paths, no tokens.

Storage: _runtime:diagnostics:<timestamp>. Retention: 7 days (default).

Never transmitted. Never logged above debug.

---

15. Canaries

Two TypeScript/JSDoc canaries gate the type surface. Both live under
tests/canary/ and run under checkJs + strict.

tests/canary/settings.js - verifies indexed-access extraction on an
object-shaped settings declaration. @ts-expect-error on an unknown key.

tests/canary/args.js - verifies sibling-contextual substitution on a
per-command args descriptor. @ts-expect-error on an unknown key and an
unknown kind.

Both must pass (tsc --noEmit, exit 0) before the package builds. Any
change to the type surface requires re-running both.

---

16. Deferred to v1.1 or later

Item                                                        Reason
Builder pattern for defineCommand                           May eliminate §8's ctx.settings limitation. Needs a canary.
Direct property access on args (args.number)                Mapped-type instantiation blocked. Builder pattern may unlock it.
Record and replay (atoll-bot record/replay)                 Layers 1-6 suffice for v1.0.
Public sandbox server                                       Self-hosted only in v1.0.
Nested subcommands                                          Flat namespace in v1.0.
Localization of bot labels and descriptions                 English-only in v1.0.

---

17. Open server dependencies

Three items remain pending server team decisions. Interim implementation
follows the stated defaults.

Item                                     Interim                     Reference
Command result AAD                       None                        Reconciliation §3.1
Owner-local message transport            __owner_local__ synthetic   Reconciliation §7.1
request_id scoping in bot_request_log    Endpoint + target suffix    Reconciliation §2.3

---

18. Document status

This is the contract for the @atoll/bot package. Every implementation
task references this document. If a task conflicts, the task is wrong
and must be revised.

Two canaries gate any change to the type surface. Any change to the
encryption schemes requires coordination with the Server and Client
specifications.

End of @atoll/bot Package Specification v1.0.1.
```

---

**Diff summary against v1.0:**

- **Status line**: bumped to v1.0.1 with an Amendments block.
- **§2.1**: `module: "es2022"`, `moduleResolution: "bundler"`, plus the module-mode rationale and `.js`-extension convention paragraph.
- **§2.3**: `definedTypes` list spelled out with all 35 names; JSDoc description standard paragraph; hyphen rule.
- **§3.1–§3.7**: every `@property` and `@template` carries a description separated by ` - `. Ten typedefs in the original had no descriptions on their properties; all now do. Example: `Avatar`, `RoomRef`, `MessageRef`, `AttachmentRef`, `SettingDecl`, `ArgDecl`, `WebhookTrigger`, `ScheduleTrigger`, `SettingsStore`, `StorageStore`, `RoomsStore`, `PostOptions`, `LocalOptions`, `BotCtx`, `MessageDataMember`, `MessageDataObserver`, `MessageDataDeleted`, `MessageEvent`, `HandlerMap`, `CommandDecl`, `BotConfig`, `Bot`.
- **§4.1–§4.5**: every `@param`, `@returns`, and `@template` carries a description.
- **§4.3**: the prose em-dash inside the JSDoc block replaced with a hyphen.
- **§13.1, §13.2**: the `createTestCtx` and `createTestRuntime` JSDoc gained descriptions (they were undocumented in v1.0).

Sections 5 through 18 had no changes affecting JSDoc or config, so their text is carried forward unchanged.
