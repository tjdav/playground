
# V2 Task Ledger

The status of every V2 server-side task. Paste this file into a new
conversation to reacquire project context.

The client side of V2 is not tracked here. It depends on a client
specification that has not yet been written.

---

## How to Read This File

- **Status values:** `done`, `in progress`, `pending`, `blocked`, `dropped`.
- **Dependencies** are task IDs that must be `done` before this task starts.
- **Deviations** are facts about the implementation that differ from the
  original task prompt. They are recorded because future tasks may depend
  on the actual behavior.
- **Verified facts** are items that have been empirically confirmed. They
  are recorded in `verifications.md`.

---

## Legend

- `V-*` — verification task (produces a report, does not change code)
- `T-*` — infrastructure task
- `21a`, `21b`, ... — implementation tasks, numbered by phase

---

## Completed Tasks

### T-TEST — Batched Test Execution Infrastructure

**Status:** done
**Deliverable:** `batch-manifest.toml`, `Makefile`, `check-test-batches.sh`,
`test_batch_manifest.rs`, `TESTING.md`
**Verified fact:** See `verifications.md` → "Batched Test Execution Model"
**Deviation:** None.

---

### V-A — Username Token Opacity Analysis

**Status:** done
**Deliverable:** `verification/oprf-token-opacity/report.md`
**Note:** Superseded by V-A2 on the token format question. Questions 2, 3,
and 4 stand.

---

### V-A2 — Empirical Username Token Value

**Status:** done
**Deliverable:** `verification/oprf-token-value/` and
`verification/oprf-token-opacity/report-addendum.md`
**Verified fact:** Token is 64 bytes, 86-character base64url.
**Deviation:** The original V-A report claimed 32 bytes. Corrected.

---

### V-C — OPRF Key Storage Format

**Status:** done
**Deliverable:** `verification/oprf-key-storage/`
**Verified fact:** Outcome 2. File stores serialized `ServerSetup` (128 bytes).
`root_secret = SHA-256(oprf_key_bytes)`.
**Deviation:** The original spec §5.8 assumed a raw seed. Corrected via
Amendment 30.

---

### 21a — OPRF Blind Endpoint and Key Derivation

**Status:** done
**Deliverable:** `POST /api/v1/oprf/blind`, `server/src/oprf/`, key
derivation from `ServerSetup`
**Dependencies:** V-C
**Deviation:** None. Implemented with the corrected derivation from V-C.

---

### 21b — Identity Schema Rewrite

**Status:** done
**Deliverable:** Migration `0020_users_oprf_identity.sql`,
`server/src/identity/`, rewritten registration, login, lookup, GDPR
**Dependencies:** 21a
**Verified facts:**
- Token is 64 bytes, 86-char base64url (from V-A2).
- `encrypted_display` decrypts to 28–284 bytes.
- Lookup returns 200 vs 404 (from task 21b; see verifications §16.20).
**Deviations:**
- `requires_reregistration` column was NOT added by this task. Task 21c
  adds it. Login query currently does not check it.
- Lookup endpoint returns 200/404 (not indistinguishable shape/timing).
  This corrects the spec §8.2.3, see Amendment 32.
- On account deletion, `username_token` is replaced with a random 86-char
  string, not set to NULL.
**Notes:**
- The test suite grew to ~55 test binaries during this task. The batched
  test infrastructure (T-TEST) exists because of this.

---

### 23 — User-Scoped Sync Foundation

**Status:** done
**Deliverable:** Migration `0021_user_seq.sql`, `server/src/sync/`,
`GET /users/me/sync`, `private-user-{user_id}` channel auth
**Dependencies:** 21b
**Deviations:** None.
**Notes:**
- The sync endpoint returns empty arrays for `read_state`,
  `user_preferences`, `device_state`, and `starred_items`. Feature tasks
  fill these in.
- `full_resync_required` is always `false` until retention pruning lands.

---

### 24a — Read State Write and Sync

**Status:** done
**Deliverable:** Migration `0022_read_state.sql`,
`server/src/sync/read_state.rs`, `POST /users/me/read-state`,
`read.sync` durable event
**Dependencies:** 23
**Deviations:** None.
**Verified behavior:**
- Full sync (`since_seq=0`) excludes tombstones.
- Delta sync (`since_seq>0`) includes tombstones.
- `max_seq` never regresses below `since_seq`.
- Write publishes `read.sync` after commit; publish failures are logged
  and do not affect the response.
- Write rate limit is 120/min per user (added as `RATE_READ_STATE_PER_MIN`).

---

## Pending Tasks

### V-B — Key Transparency Scale Verification

**Status:** pending
**Dependencies:** None
**Blocks:** 34a
**Notes:** See `verifications.md` → "V-B".

---

### 21c — OPRF Admin Rotation

**Status:** pending
**Dependencies:** 21b
**Deliverable:** `requires_reregistration` column, `POST /admin/oprf/rotate`,
audit entries
**Notes:**
- Adds the column that 21b did not add.
- Modifies the login query to check `requires_reregistration`.
- This is the last task in the identity segment.

---

### 24b — Room Order Preference

**Status:** pending
**Dependencies:** 23, 24a
**Deliverable:** `PATCH /users/me/room-order`, `room_order.sync` durable event
**Notes:** Reuses the pattern from 24a.

---

### 24c — Generic Preferences Endpoint

**Status:** pending
**Dependencies:** 23, 24a
**Deliverable:** `GET/PATCH/DELETE /users/me/preferences/:key`,
`preference.updated` durable event
**Notes:** Reserved key handling for `room_order` folds into this task.

---

### 25 — Device Model

**Status:** pending
**Dependencies:** 23, 24a
**Deliverable:** `device_names` table, encrypted device name handling,
sync integration, drop `devices.name`

---

### 26 — Room Metadata

**Status:** pending
**Dependencies:** 21b
**Deliverable:** Drop `rooms.name_encrypted`, add `rooms.metadata` +
`metadata_version`, `PATCH /rooms/:id`
**Notes:** Can run in parallel with 24b and 24c.

---

### 27 — Message Editing

**Status:** pending
**Dependencies:** 11 (V1)
**Deliverable:** `edit_of`, `edit_sequence`, `edited_at` columns,
`PATCH /rooms/:id/messages/:msg_id`, `edit_window_seconds`

---

### 28 — Reactions

**Status:** pending
**Dependencies:** 11 (V1)
**Deliverable:** `reactions` table, add/remove/list endpoints,
`reaction.added`/`reaction.removed` events

---

### 29 — Threading

**Status:** pending
**Dependencies:** 11 (V1)
**Deliverable:** `reply_to` column, updated message submission

---

### 30 — Member Pagination

**Status:** pending
**Dependencies:** 7a (V1)
**Deliverable:** Cursor pagination on `GET /rooms/:id/members`

---

### 31 — Retention Preview

**Status:** pending
**Dependencies:** 26
**Deliverable:** `POST /rooms/:id/retention/preview`

---

### 32 — Avatar Upload

**Status:** pending
**Dependencies:** 12a (V1)
**Deliverable:** `POST /users/me/avatar`

---

### 33 — Pending MLS Adds

**Status:** pending
**Dependencies:** 10 (V1)
**Deliverable:** `pending_mls_adds` table, coordination endpoints

---

### 34a — Key Transparency Log

**Status:** pending
**Dependencies:** 21b, V-B
**Deliverable:** `key_transparency_log`, `GET /admin/key-transparency`

---

### 34b — Key Transparency Proofs

**Status:** pending
**Dependencies:** 34a
**Deliverable:** Merkle tree, snapshots, `POST /admin/key-transparency/snapshot`

---

### 35 — Link Preview Proxy

**Status:** pending
**Dependencies:** 21b
**Deliverable:** `POST /link-preview/proxy`, SSRF guard, capability advertisement

---

### 36a — Call Signaling

**Status:** pending
**Dependencies:** 11 (V1)
**Deliverable:** `call_sessions`, `call_participants`, signal endpoint

---

### 36b — Call Lifecycle

**Status:** pending
**Dependencies:** 36a
**Deliverable:** Call cleanup job, `CALLING_ENABLED`, capability advertisement

---

### 37 — TURN Credentials

**Status:** pending
**Dependencies:** 36a
**Deliverable:** `POST /calls/turn-credentials`

---

### 39a — Capabilities Update

**Status:** pending
**Dependencies:** All feature tasks
**Deliverable:** Extended `/capabilities` response
**Notes:** Must be last. Every feature task's capability flag must exist
before this task runs.

---

### 39b — CLI Model Subcommands

**Status:** pending
**Dependencies:** 41a
**Deliverable:** `server models verify`, `server models fetch`

---

### 40a — Hangouts CRUD

**Status:** pending
**Dependencies:** 36a
**Deliverable:** `hangouts` table, CRUD endpoints, position management

---

### 40b — Hangout Occupancy

**Status:** pending
**Dependencies:** 40a
**Deliverable:** In-memory occupancy, heartbeat, debounced events

---

### 40c — Hangout Signaling

**Status:** pending
**Dependencies:** 36b, 40b
**Deliverable:** Signal endpoint with `target_client_id`, roster endpoint

---

### 41a — Model Hosting (Local)

**Status:** pending
**Dependencies:** None
**Deliverable:** `MODEL_HOSTING_MODE=local`, static file endpoints, manifest

---

### 41b — Model Hosting (External and Proxy)

**Status:** pending
**Dependencies:** 41a
**Deliverable:** External URL advertisement, proxy cache, manifest TTL

---

### 42 — Starred Items

**Status:** pending
**Dependencies:** 23, 24a
**Deliverable:** `starred_items` table, endpoints, durable events

---

### 44 — GDPR V2 Field Coverage

**Status:** pending
**Dependencies:** 21b, 22a, 24a, 24c, 25, 42
**Deliverable:** Export and deletion handling for V2 fields
**Notes:** Must be last of the identity-adjacent tasks.

---

### 15b-R — Push Payload sender_ref Retrofit

**Status:** pending
**Dependencies:** 21b
**Deliverable:** Change payload composer to use `sender_ref` instead of
`sender_user_id`
**Notes:** Small. V1 push delivery works; only the field name changes.

---

### 22a — Recovery Code Generation

**Status:** pending
**Dependencies:** 21b
**Deliverable:** `recovery_codes` table, code generation, Argon2id hashing

---

### 22b — Recovery Endpoints

**Status:** pending
**Dependencies:** 22a
**Deliverable:** `POST /auth/recover/start`, `POST /auth/recover/finish`,
OPAQUE re-registration, session revocation cascade

---

## Dropped Tasks

### 43 — Preferences Cleanup

**Status:** dropped
**Reason:** Duplicate of validation in task 24c. No orphan state exists
because `user_preferences` cascades on user deletion.

---

## Summary

| Status | Count |
|---|---|
| Done | 8 |
| Pending | 29 |
| Blocked (on V-B) | 1 (34a) |
| Dropped | 1 |
| **Total** | **38 tasks, 31 remaining** |

---

## Critical Path

```
21a (done) → 21b (done) → 21c
23 (done) → 24a (done) → 24b, 24c, 25
```

The remaining critical path is short. Most pending tasks run in parallel.

---

## Notes for Future Conversations

- **The spec URL is authoritative.** It is updated with every amendment.
  See §16 of the spec for the amendment log.
- **Read `verifications.md` before touching §5.8, §6.19, §6.20, §6.21,
  §7.1, §8.2.3, or §5.30.** Those sections were changed by verification
  findings.
- **Do not re-litigate verified facts.** If a task prompt contradicts a
  verified fact, the task prompt is wrong. Update the task, not the spec.
- **Batch tests.** See §5.30 of the spec. Never run unfiltered `cargo test`.
- **`make check-batches` before committing.** It catches orphan test files.
