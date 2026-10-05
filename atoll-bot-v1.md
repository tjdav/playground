@atoll/bot Package Specification v1.0

Status: Final — source of truth for @atoll/bot implementation tasks.
Supersedes: @atoll/bot Draft v0.2, all interim drafting messages.
Basis: Server Specification v3.0.2 · Client Specification v2.0 (Final) · Reconciliation (2026-10-04).
Stack: Node.js ≥ 22.22.2 (ESM) · JavaScript + JSDoc · node:test · pnpm workspace.
Location: packages/bot/ in the monorepo.

---

1. Overview

@atoll/bot is the SDK bot authors import. It parallels @atoll/extend. The author writes handlers and declarations; the runtime handles crypto, transport, reconnection, rate limiting, idempotency, and key management.

A bot is a headless long-lived process. It runs on the operator's machine. It connects to the server as its own identity. It does not hold room encryption material unless it has been granted read_content in a room.

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
    "module": "nodenext",
    "moduleResolution": "nodenext",
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

strict: true is a hard requirement. Every guarantee in this specification — including @ts-expect-error canaries, duplicate-property detection (TS1117), and per-key inference — is checker-enforced.

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

.d.ts is generated from JSDoc. Single source of truth. No hand-authored .d.ts.

2.3 ESLint

Root config supplies the base. Package config extends it and adds:

· import/no-restricted-paths: src/runtime/ cannot import from src/testing/; src/testing/ cannot import from src/runtime/internal/.
· import/enforce-node-protocol-usage: forces node:fs over fs.

The root config's jsdoc/no-undefined-types list is trimmed to a minimal baseline. Types are added when they first trigger an error, not preemptively.

---

3. Type surface

All public typedefs live in src/types.js. No runtime. The file exists solely to emit into dist/types.d.ts.

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

/** @typedef {{ fileId: string | null, emoji: string | null }} Avatar */

/** @typedef {{ id: string, displayName: string | null, memberCount: number }} RoomRef */

/** @typedef {{ id: string, roomId: string, createdAt: string }} MessageRef */

/** @typedef {{ fileId: string, size: number, contentType: string,
 *   thumbnailFileId?: string }} AttachmentRef */
```

3.2 Settings

```js
/** @typedef {object} SettingDecl
 * @property {string} label
 * @property {SettingKind} type
 * @property {'user' | 'room'} [scope]
 * @property {true} [required]
 * @property {unknown} [default]
 * @property {string[]} [options]
 * @property {string} [placeholder]
 * @property {string} [hint]
 */

/**
 * Maps a setting declaration's `type` literal to its value type.
 * @template D
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

/** @typedef {Record<string, SettingDecl>} SettingsDecl */
```

3.3 Commands

```js
/** @typedef {object} ArgDecl
 * @property {ArgKind} type
 * @property {true} [required]
 * @property {string[]} [options]
 * @property {string} [description]
 */

/** @template D
 * @typedef {D extends { type: 'string' } ? string
 *   : D extends { type: 'number' } ? number
 *   : D extends { type: 'boolean' } ? boolean
 *   : D extends { type: 'select' } ? string
 *   : never} ArgValue */

/** @template D
 * @typedef {D extends { required: true } ? ArgValue<D> : ArgValue<D> | undefined} ArgOf */

/** @template {Record<string, ArgDecl>} D
 * @typedef {object} ArgsReader
 * @property {<K extends keyof D & string>(key: K) => ArgOf<D[K]>} get */

/** @typedef {
 *   { type: 'local_message', content: string }
 *   | { type: 'room_message', content: string,
 *       attachments?: AttachmentRef[], replyTo?: string }
 *   | { type: 'toast', content: string }
 *   | { type: 'panel', content: string }
 *   | { type: 'none' }
 * } CommandResult */
```

required?: true is not a mistake. required?: boolean would widen true to boolean, breaking D[K] extends { required: true }. Explicit required: false is a type error by design. Omit the field to make an argument optional.

3.4 Triggers

```js
/** @typedef {object} WebhookTrigger
 * @property {'webhook'} type
 * @property {string} path
 * @property {'POST' | 'PUT' | 'PATCH'} [method]
 * @property {string} [secret]
 * @property {string} [idempotency]
 * @property {number} [retries]
 * @property {number} [retryDelayMs]
 */

/** @typedef {object} ScheduleTrigger
 * @property {'schedule'} type
 * @property {string} name
 * @property {string} cron
 * @property {string} [timezone]
 */

/** @typedef {WebhookTrigger | ScheduleTrigger} TriggerDecl */
```

3.5 Context and bot

```js
/** @template {Record<string, SettingDecl>} S
 * @typedef {object} SettingsStore
 * @property {<K extends keyof S & string>(key: K, opts?: { room?: string }) =>
 *   Promise<SettingValue<S[K]> | undefined>} get
 * @property {<K extends keyof S & string>(key: K, value: SettingValue<S[K]>,
 *   opts?: { room?: string }) => Promise<void>} set
 * @property {(key: string, opts?: { room?: string }) => Promise<void>} delete
 * @property {<K extends keyof S & string>(key: K,
 *   cb: (value: SettingValue<S[K]> | undefined) => void,
 *   opts?: { room?: string }) => () => void} subscribe */

/** @typedef {object} StorageStore
 * @property {(key: string) => Promise<unknown>} get
 * @property {(key: string, value: unknown) => Promise<void>} set
 * @property {(key: string) => Promise<void>} delete
 * @property {() => Promise<void>} clear */

/** @typedef {object} RoomsStore
 * @property {() => Promise<RoomRef[]>} list
 * @property {(roomId: string) => Promise<RoomRef | null>} get */

/** @typedef {object} PostOptions
 * @property {string} [roomId]
 * @property {string} [text]
 * @property {AttachmentRef[]} [attachments]
 * @property {string} [replyTo] */

/** @typedef {PostOptions & { replyTo: string }} ReplyOptions */

/** @typedef {object} LocalOptions
 * @property {string} text
 * @property {'text' | 'markdown'} [format] */

/**
 * Invocation context. Reconstructed per handler call. Not a stable
 * identity. Fields are populated per invocation kind and derived mode.
 *
 * @template {Record<string, SettingDecl>} [S=Record<string, SettingDecl>]
 * @typedef {object} BotCtx
 * @property {{ id: string, label: string, ownerUserId: string,
 *   avatar: Avatar }} bot
 * @property {{ roomId: string, mode: Mode,
 *   scopes: Capability[] } | null} grant
 * @property {RoomRef | null} room
 * @property {{ id: string, type: string, roomId: string | null,
 *   timestamp: string } | null} event
 * @property {SettingsStore<S>} settings
 * @property {StorageStore} storage
 * @property {RoomsStore} rooms
 * @property {(opts: PostOptions) => Promise<MessageRef>} post
 * @property {(opts: ReplyOptions) => Promise<MessageRef>} reply
 * @property {(opts: LocalOptions) => Promise<void>} sendLocal
 * @property {(level: LogLevel, message: string, meta?: object) => void} log
 * @property {(url: string, opts?: RequestInit) => Promise<Response>} fetch
 * @property {(url: string, opts?: RequestInit) => Promise<Response>} fetchUserUrl
 * @property {AbortSignal} signal
 * @property {(path: string) => Promise<string>} uploadAvatar */
```

3.6 Handlers

```js
/** @typedef {object} MessageDataMember
 * @property {string} id
 * @property {string} senderUserId
 * @property {string} senderClientId
 * @property {string} createdAt
 * @property {string | null} plaintext
 * @property {AttachmentRef[]} attachments
 * @property {string | null} replyTo */

/** @typedef {object} MessageDataObserver
 * @property {string} id
 * @property {string} senderUserId
 * @property {string} senderClientId
 * @property {string} createdAt
 * @property {number} sizeBytes */

/** @typedef {object} MessageDataDeleted
 * @property {string} id */

/** @typedef {object} MessageEvent
 * @property {string} id
 * @property {'message.new' | 'message.edited' | 'message.deleted'} type
 * @property {string} roomId
 * @property {string} timestamp
 * @property {MessageDataMember | MessageDataObserver | MessageDataDeleted} data */

/** @template {Record<string, SettingDecl>} S
 * @typedef {object} HandlerMap
 * @property {(ctx: BotCtx<S>) => void | Promise<void>} [install]
 * @property {(ctx: BotCtx<S>) => void | Promise<void>} [uninstall]
 * @property {(ctx: BotCtx<S>,
 *   event: { botId: string, roomId: string, oldMode: Mode, newMode: Mode,
 *            scopes: Capability[], changed: string[] }) =>
 *   void | Promise<void>} [grantUpdated]
 * @property {(ctx: BotCtx<S>, event: MessageEvent) => void | Promise<void>} [message]
 * @property {(ctx: BotCtx<S>,
 *   event: { id: string, type: string, roomId: string, timestamp: string,
 *            data: object }) => void | Promise<void>} [room]
 * @property {(ctx: BotCtx<S>,
 *   payload: { path: string, body: string, headers: Record<string, string> }) =>
 *   void | Promise<void>} [webhook]
 * @property {(ctx: BotCtx<S>, payload: { name: string }) =>
 *   void | Promise<void>} [schedule] */
```

3.7 Bot config

```js
/** @template {Record<string, ArgDecl>} D
 * @template {Record<string, SettingDecl>} S
 * @typedef {object} CommandDecl
 * @property {string} [description]
 * @property {D} args
 * @property {(ctx: BotCtx<S>, args: ArgsReader<D>) =>
 *   CommandResult | Promise<CommandResult>} handler */

/** @template {Record<string, SettingDecl>} S
 * @template {Record<string, CommandDecl<any, any>>} C
 * @template {readonly TriggerDecl[]} T
 * @typedef {object} BotConfig
 * @property {string} id
 * @property {string} apiVersion
 * @property {string} hostApi
 * @property {string} label
 * @property {Capability[]} capabilities
 * @property {Avatar} [avatar]
 * @property {S} [settings]
 * @property {C} [commands]
 * @property {T} [triggers]
 * @property {HandlerMap<S>} handlers */

/** @template {Record<string, SettingDecl>} S
 * @template {Record<string, CommandDecl<any, any>>} C
 * @template {readonly TriggerDecl[]} T
 * @typedef {object} Bot
 * @property {BotConfig<S, C, T>} config */
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
 * @template {Record<string, SettingDecl>} Ds
 * @param {Ds} decls
 * @returns {Ds}
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
 * @template {Record<string, ArgDecl>} D
 * @param {CommandDecl<D, any>} decl
 * @returns {CommandDecl<D, any>}
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
 * infer per-command types here — see the canary at
 * tests/canary/args.js for why.
 *
 * @template {Record<string, CommandDecl<any, any>>} C
 * @param {C} commands
 * @returns {C}
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
 * @param {TriggerDecl} trigger
 * @returns {TriggerDecl}
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
 * @template {Record<string, SettingDecl>} S
 * @template {Record<string, CommandDecl<any, any>>} C
 * @template {readonly TriggerDecl[]} T
 * @param {BotConfig<S, C, T>} config
 * @returns {Bot<S, C, T>}
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

Validation collects all failures and reports them at once. Startup aborts.

---

5. BotCtx — populated fields per mode

Field write_only observer member
bot ✓ ✓ ✓
grant ✓ ✓ ✓
room room-scoped room-scoped room-scoped
event null for message/room metadata only plaintext
settings ✓ ✓ ✓
storage ✓ ✓ ✓
rooms ✓ ✓ ✓
post ✓ ✓ ✓
reply ✓ ✓ ✓
sendLocal ✓ ✓ ✓
log ✓ ✓ ✓
fetch ✓ ✓ ✓
fetchUserUrl ✓ ✓ ✓
signal ✓ ✓ ✓
uploadAvatar ✓ ✓ ✓

Message and room handlers are never invoked in write_only. The author narrows event.data:

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

Mode Encryption Signing
write_only Publisher key ECDH (ephemeral X25519 per message) Ed25519 over length-prefixed payload
observer Same Same
member MLS application message MLS handles authentication

Write-only and observer path:

1. Fetch or use cached publisher key for the room's current epoch.
2. Verify the signer's identity_pubkey against the KT log.
3. shared = ECDH(ephemeral_priv, publisher_public)
4. key = HKDF-Expand(shared, "publisher-message-v1", 32)
5. Encrypt with AES-256-GCM. Wire: ephemeral_pub(32) || nonce(12) || ct || tag(16).
6. Sign length-prefixed payload with bot_identity_private_key (Server §8.10).
7. POST /rooms/:id/bot-messages with { epoch, ciphertext, content_type: "bot", signature, request_id }.

Retry: on 5xx, network failure, or 429, retry up to three times with exponential backoff (2s, 5s, 15s). 429 respects Retry-After. Idempotent via request_id. On final failure, rejects with PostFailedError.

6.2 ctx.reply

Sugar over ctx.post with replyTo required.

6.3 ctx.sendLocal

Wire: POST /bots/me/messages with target: "owner", result_type: "local_message", ciphertext, bot_result_pubkey, request_id.

Encryption: X25519 ephemeral per message. key = HKDF-Expand(shared, "bot-command-result-v1", 32). AES-256-GCM. Wire: nonce(12) || ct || tag(16).

Non-durable. Best-effort. If the owner is offline, the message is lost. For anything the owner must see, use ctx.reply() in a room.

Owner pubkey transport: interim uses a synthetic command (__owner_local__). Pending server resolution of reconciliation §7.1. Marked for removal.

6.4 Command result

Result type Wire path
local_message POST /bots/me/messages target: "invoker" result_type: "local_message"
toast Same, result_type: "toast"
panel Same, result_type: "panel"
room_message ctx.post({ text: content, attachments, replyTo })
none No result; ack only

Fresh ephemeral X25519 keypair per result. ephemeral_result_pubkey arrives on bot.command_invoked. Bot derives:

```
shared = ECDH(bot_result_priv, ephemeral_result_pub)
key    = HKDF-Expand(shared, "bot-command-result-v1", 32)
```

Encrypt with AES-256-GCM. Wire: nonce(12) || ct || tag(16).

Non-durable. If the invoking client is offline, the result is lost. Authors needing guaranteed delivery use ctx.reply().

Associated data: none, per reconciliation §3.1. Interim. If the server team adds AAD, one line changes.

---

7. Settings runtime

7.1 Scope

Scope Storage key Visible to
user key All devices of the operator
room room:{room_id}:key (prefix added by runtime) All devices of the operator, scoped to a room

The runtime handles prefixing. The author writes ctx.settings.get('webhookSecret', { room: 'r_abc123' }).

7.2 Encryption

Non-secret. Two ciphertexts:

· value_encrypted_client — operator's preferences_key, standard preferences pattern.
· value_encrypted_bot — X25519 ECDH between the client's ephemeral key and the bot's bot_command_pubkey.

Secret. One ciphertext: value_encrypted_bot only. No client-readable copy. No local plaintext mirror.

Wire format for value_encrypted_bot:

```
base64url(ephemeral_pub(32) || nonce(12) || ct || tag(16))
```

key = HKDF-Expand(shared, "bot-settings-v1", 32). Bot decrypts with bot_command_private_key.

7.3 Subscribe

ctx.settings.subscribe(key, cb) returns an unsubscribe function. Fires when bot.settings_updated arrives on the bot channel. Runtime refetches affected keys. Coalesced within 500ms.

7.4 Reserved keys

Author keys must not start with _runtime:. Rejected with SettingsError('reserved_prefix').

---

8. Command handler ctx.settings limitation

Inside defineCommand, the handler's args parameter is typed precisely. ctx.settings.get('apiToken') is not — it returns the union of every possible setting value type.

Why. defineCommand is called before defineBot. At the time defineCommand infers D, the settings type S is not yet known. TypeScript does not retroactively re-type callbacks. Top-level handlers inside the defineBot call do get precise S.

Canonical pattern — runtime type guard.

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

This is the only option that does not lie to the type system. Verbose but honest.

Alternative: explicit cast — unsafe, rejected. Hoisting to a top-level handler — moves the problem to storage, rejected.

A builder pattern may eliminate this in v2. Parked.

---

9. Storage

ctx.storage is bot-scoped, persistent, encrypted at rest with a key derived from the bot's MLS identity private key via HKDF ("bot-storage-v1", 32 bytes). AES-256-GCM, nonce prepended. Not synced. Local to the runtime machine.

_runtime: prefix is reserved. ctx.storage.clear() removes all author storage. Runtime state under _runtime:* is untouched.

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

· Env var name ends in _SECRET: HMAC-SHA256 of the raw body, compared against the vendor header (X-Hub-Signature-256 for GitHub, X-Signature-256 generic). Constant-time compare.
· Env var name ends in _TOKEN: Authorization: Bearer <value>. Constant-time compare.
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

Internal cron engine. Five fields. No seconds. No macros. DST: skipped hour → no fire; repeated hour → fires once. Missed fires: default skipped; catch_up = true fires once per miss on boot with a flag.

10.3 Idempotency

Five-minute window.

Trigger Key
Webhook header:<name> value, or generated UUID
Schedule <name>:<ISO timestamp>
Command Server-assigned command_id
Message event.id

Persisted to ctx.storage under _runtime:idempotency:*. Replayed on restart.

---

11. Handlers

Top-level handlers declared in handlers. Every handler receives ctx typed with the bot's S.

At least one handler must be declared. Exceptions are logged and do not disable the bot. Three consecutive failures within one minute pause the bot; the operator is notified via bot.paused on their user channel. The runtime does not see the pause event and treats it as indistinguishable from server unavailability.

---

12. Errors

Every runtime-raised error carries a code string.

Code Trigger Recoverable
validation_error Structural failure in defineBot No
capability_unknown Capability outside vocabulary No
scope_not_declared Grant asks for undeclared scope No
scope_dependency_missing Grant lacks prerequisite No
post_failed ctx.post retries exhausted Author may catch
reply_requires_reply_to ctx.reply without replyTo No
settings_reserved_prefix Setting key starts with _runtime: No
settings_decrypt_failed Bot settings ciphertext malformed No
settings_write_failed Settings PATCH non-2xx Retry
storage_reserved_prefix Storage key starts with _runtime: No
storage_capacity_exceeded Storage backend full Operator action
publisher_key_unavailable No key for room's current epoch Retry after epoch advance
publisher_key_verification_failed Signature invalid or signer unknown No
command_result_encrypt_failed Malformed ephemeral_result_pubkey No
key_rotation_failed PATCH /bots/:id/keys non-2xx Retry
keystore_locked Keystore secret missing or wrong Operator action
keystore_corrupt Keystore unreadable Operator action
rate_limited Local throttle would exceed server limit Retry after backoff
webhook_secret_missing Configured secret env var not set Operator action
webhook_signature_invalid HMAC verification failed No

BotError is the base class. Every code is a subclass. err.code is stable. err.message is human-readable and may change.

---

13. Testing entry point — @atoll/bot/testing

13.1 createTestCtx

```js
/**
 * @template {Record<string, SettingDecl>} [S=Record<string, SettingDecl>]
 * @param {object} [overrides]
 * @param {S} [overrides.settingsDecl]
 * @param {Record<string, unknown>} [overrides.settings]
 * @param {{ roomId: string, mode: Mode, scopes: Capability[] }} [overrides.grant]
 * @param {RoomRef} [overrides.room]
 * @param {Record<string, unknown>} [overrides.storage]
 * @returns {BotCtx<S> & { calls: {
 *   post: PostOptions[],
 *   reply: ReplyOptions[],
 *   sendLocal: LocalOptions[]
 * } }}
 */
export function createTestCtx (overrides) { /* ... */ }
```

All fields populated. Response methods capture to calls instead of sending. Settings and storage are in-memory. No crypto exercised.

13.2 createTestRuntime

```js
/**
 * @param {Bot} bot
 * @param {object} [options]
 * @param {{ roomId: string, mode: Mode, scopes: Capability[] }[]} [options.grants]
 * @param {{ validateCrypto?: boolean }} [options.options]
 * @returns {Promise<TestRuntime>}
 */
export function createTestRuntime (bot, options) { /* ... */ }
```

Methods:

· inject(event) — dispatch to handler.
· dispatchWebhook({ path, body, headers })
· fireSchedule(name)
· advanceTime(ms)
· restart() — preserve storage and idempotency.
· simulateResponse(path, { status, body })
· calls — captured outbound.
· getPendingOutbound()
· getGrants(), getPublisherKeys()

13.3 Mock server

@atoll/bot/testing/mock-server implements every bot-facing endpoint. Default: accept any ciphertext, record calls. With { validateCrypto: true }: verify signatures, parse embedded structures, validate wire format.

Does not exercise real MLS, real KT inclusion proofs, or real publisher key derivation. Those require a sandbox server.

---

14. Runtime specification

14.1 CLI

All commands run as atoll-bot <command>.

Command Purpose
register <path> Register a new bot account
login Cache an operator session
logout Clear the cached operator session
validate <path> Validate without connecting
run <path> Start the runtime
rotate-keys Generate new keypairs, PATCH, replace keystore entries
avatar <path> Upload or replace the bot's avatar
export-keys [--out <path>] Export the keystore
import-keys <path> Import a keystore
dev <path> Dev mode; hot reload, verbose logging, mock server
inspect [room-id] Dump the runtime's state
tail Live-stream the structured log, redacted
sandbox connect <url> --token <token> Register a sandbox bot
sandbox run <path> Run against the sandbox server
sandbox reset Delete the sandbox bot

Exit codes: 0 success, 1 user error, 2 internal error.

14.2 Environment variables

Variable Required Default Purpose
ATOL_BOT_TOKEN Runtime — Bot auth token
ATOL_SERVER_URL Runtime — Base URL
ATOL_BOT_HOST No 0.0.0.0 Webhook bind host
ATOL_BOT_PORT No 8787 Webhook bind port
ATOL_BOT_CONFIG No <cwd>/bot.toml Override file
ATOL_BOT_KEYSTORE No <cwd>/bot.keystore Keystore path
ATOL_BOT_KEYSTORE_SECRET Yes — Keystore encryption secret
ATOL_USER_TOKEN Register only — Operator session
ATOL_LOG_LEVEL No info debug \| info \| warn \| error
ATOL_LOG_FORMAT No json json \| pretty
ATOL_RECONNECT_MAX_BACKOFF_MS No 30000 Backoff cap
ATOL_RECONNECT_BASE_BACKOFF_MS No 1000 Initial backoff
ATOL_HANDLER_TIMEOUT_MS No 60000 Per-handler timeout
ATOL_SHUTDOWN_DRAIN_MS No 30000 Drain

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

Bound at ATOL_BOT_HOST:ATOL_BOT_PORT. Only started if a webhook trigger is declared.

Request flow:

1. Method must match. Else 405.
2. Path must equal base_path + trigger.path. Else 404.
3. Body read up to max_body_bytes. Exceeded → 413.
4. Secret verification (see §10.1).
5. Idempotency key computed.
6. Dispatch to handlers.webhook.
7. 200 on success. 500 on handler throw. 504 on timeout.

14.5 Cron engine

Internal, no dependency. Five fields. Intl.DateTimeFormat for timezones. DST handled per §10.2. Missed fires: skipped unless catch_up = true. State at _runtime:cron:<name>:last_fire.

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
· Full URLs from ctx.fetch/ctx.fetchUserUrl logged with query string stripped.
· Response bodies never logged.

Reserved prefix _secret:. Attempting to log a reserved key throws in dev mode.

14.7 Reconnection

Exponential backoff with jitter. Attempt 1: 1s ± 20%. Doubles to 30s cap. No max attempts.

On reconnect:

1. Resubscribe to private-bot-{bot_id}.
2. Refetch grants.
3. Refetch settings.
4. Refetch publisher keys.
5. Replay pending outbound with original request_id.
6. Resume dispatch.

Outbound queue persisted to _runtime:outbound:<request_id>. Bounded at 1000 entries; oldest dropped with warn.

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

Drain timeout: exit 1. Unflushed state is lost. Keystore and persistent storage untouched.

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
2. OS keychain: macOS Keychain, Linux libsecret, Windows Credential Manager. Entry name atoll-bot:<bot_id>.
3. Interactive prompt at first command. Stored to OS keychain if available.

Keystore operations shell out to platform tools (security on macOS, secret-tool on Linux, cmdkey on Windows) rather than depend on a native module. Slower, dependency-free.

If no secret resolves: keystore_locked. Runtime does not start.

Rotation. Generate three new keypairs. PATCH /bots/:id/keys. On success, replace private keys and update rotated_at. bot_token, storage_seed, and operator_session preserved.

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

Eviction: current epoch + two most recent prior epochs. Maximum 100 rooms cached, LRU.

Verification on first use per (room, epoch). Re-verify after bot.keys_rotated or kt.snapshot.

Unavailability: ctx.post waits up to 30 seconds, retrying with backoff. Then publisher_key_unavailable.

14.11 Epoch tracking

Sources, in order of authority:

1. room.publisher_key_updated on private-bot-{bot_id} — authoritative.
2. epoch.updated on private-room-{room_id} — member mode.
3. GET /rooms/:id/publisher-key without ?epoch.
4. ctx.event.epoch on message events — member mode.

On disagreement: refetch.

Outbound encryption uses the current epoch's publisher key. Fallback to the most recent prior epoch with a warn during the window between MLS epoch advance and human publisher key publication.

14.12 Diagnostics

Snapshots captured on:

· Handler failure (three consecutive, before pause).
· Keystore warning.
· Reconnect storm (10 consecutive).
· Operator-requested (inspect --snapshot).

Content: timestamp, bot identity, recent redacted log lines, failing handler name, stack trace, grant states, publisher key cache state, keystore status.

No setting values, no command plaintext, no message content, no keystore paths, no tokens.

Storage: _runtime:diagnostics:<timestamp>. Retention: 7 days (default).

Never transmitted. Never logged above debug.

---

15. Canaries

Two TypeScript/JSDoc canaries gate the type surface. Both live under tests/canary/ and run under checkJs + strict.

tests/canary/settings.js — verifies indexed-access extraction on an object-shaped settings declaration. @ts-expect-error on an unknown key.

tests/canary/args.js — verifies sibling-contextual substitution on a per-command args descriptor. @ts-expect-error on an unknown key and an unknown kind.

Both must pass (tsc --noEmit, exit 0) before the package builds. Any change to the type surface requires re-running both.

---

16. Deferred to v1.1 or later

Item Reason
Builder pattern for defineCommand May eliminate §8's ctx.settings limitation. Needs a canary.
Direct property access on args (args.number) Mapped-type instantiation blocked. Builder pattern may unlock it.
Record and replay (atoll-bot record/replay) Layers 1–6 suffice for v1.0.
Public sandbox server Self-hosted only in v1.0.
Nested subcommands Flat namespace in v1.0.
Localization of bot labels and descriptions English-only in v1.0.

---

17. Open server dependencies

Three items remain pending server team decisions. Interim implementation follows the stated defaults.

Item Interim Reference
Command result AAD None Reconciliation §3.1
Owner-local message transport __owner_local__ synthetic command Reconciliation §7.1
request_id scoping in bot_request_log Endpoint + target discriminator suffix Reconciliation §2.3

---

18. Document status

This is the contract for the @atoll/bot package. Every implementation task references this document. If a task conflicts, the task is wrong and must be revised.

Two canaries gate any change to the type surface. Any change to the encryption schemes requires coordination with the Server and Client specifications.

End of @atoll/bot Package Specification v1.0.
