
# Verification Log

This file records the outcome of every verification task run against the
server specification. Each entry explains *why* a section of the spec reads
the way it does.

Read this before changing any spec section marked as verified. The evidence
may not be reproducible from the spec text alone.

---

## How to use this file

- **Before implementing** a task that touches a verified section, read the
  relevant entry.
- **After running** a new verification task, add an entry here.
- **When amending** the spec, cite the verification entry that justifies the
  change.
- **When disputing** a spec section, look up its verification entry first.
  If there is no entry, the section is either unchanged from V1 or unverified.

---

## V-A — Username Token Opacity Analysis

**Date:** 2026-09-30
**Status:** Superseded by V-A2 on the token format question. Questions 2, 3,
and 4 are unaffected.
**Spec sections affected:** §5.8, §6.19, §7.1, §8.1.1, §8.2.3, §14.6

**Question asked:** Is the value stored in `users.username_token` opaque to
the server? What is the threat model if the OPRF key leaks? Is V2 strictly
better than V1?

**Answer found:**

- The server never sees the plaintext username at any endpoint.
- The blinding factor `r` is fresh per request; the blinded element is
  indistinguishable from random.
- If the OPRF key leaks alongside the database, offline enumeration costs
  ~100–1000× more per candidate than V1's SHA-256.
- V2 is strictly better than V1 because offline enumeration requires the
  OPRF key in addition to the database dump.
- The single point of failure introduced by V2 is the OPRF key.

**Incorrect claim in V-A:** The report stated that `username_token` is a
32-byte compressed Ristretto255 group element. V-A2 empirically corrected
this to 64 bytes.

**Full report:** `verification/oprf-token-opacity/report.md`

---

## V-A2 — Empirical Username Token Value

**Date:** 2026-09-30
**Status:** Complete. Canonical.
**Spec sections affected:** §6.19, §7.1

**Question asked:** What is the exact byte value and length of the token
produced by `voprf::OprfClient::finalize()`?

**Answer found:**

- The token is **64 bytes** — the SHA-512 output of `finalize()`.
- Encoded as **86-character unpadded base64url**.
- `voprf` 0.5.0 fuses unblinding and hashing inside `finalize()`. The
  intermediate 32-byte group element `h^k` is not exposed by the public API.
- The construction matches RFC 9497 §2.2 literally with length-prefixed
  hash input.
- The token is deterministic for a fixed `(input, server key)`. The client's
  blinding factor does not affect it.

**Test vector (input `"alice"`, fixed server key):**

- Length: 64 bytes
- Base64url: `jz-IVkzPXbFXEa1c12eCGvoglYmgWOxcHHyIjoabffqAApeINZb7gWdXMvNs0uimbFwaS4lRK9YrKdX-7BRgSQ`

**Spec impact:**

- §6.19 corrects the informal `token = h^k` to `token = OprfClient::finalize(...)`.
- §7.1 documents the 86-character base64url format.
- Storage is `TEXT`, no fixed-length constraint in SQLite.

**Full report:** `verification/oprf-token-opacity/report-addendum.md`

---

## V-B — Key Transparency Scale

**Date:** Pending
**Status:** Not yet run. Blocks task 34a.
**Spec sections affected:** §7.10, §8.3, §8.8, §16.7

**Question to answer:** Does the spec intend a full RFC 6962-style Merkle
tree with consistency proofs and Signed Tree Heads, or a simplified
append-only log with periodic snapshots?

**Why it matters:** The difference in implementation cost is 5–10×.
The schema uses `root_hash` and `tree_size`, which are RFC 6962 terminology,
but no proof endpoints, STH signing keys, gossip protocol, or client-side
verification are specified.

**Default recommendation if undecided:** RFC 6962-lite. Full Merkle tree
with STH signing and inclusion proofs, but no consistency proofs and no
gossip. 80% of the security benefit for 40% of the cost.

**Run before:** Task 34a begins.

---

## V-C — OPRF Key Storage Format

**Date:** 2026-09-30
**Status:** Complete. Canonical.
**Spec sections affected:** §5.8

**Question asked:** Can V2's key derivation scheme be implemented as
written — store a raw 32-byte `root_secret` at `OPAQUE_OPRF_KEY_PATH` and
derive all subkeys from it?

**Answer found:** **No.** Outcome 2 from the task's options.

- `opaque-ke` 4.0.1 cannot construct `ServerSetup` deterministically from a
  raw 32-byte seed. `ServerSetup::new_with_key_pair_and_seed` requires an
  RNG parameter and produces non-identical output across RNG instances.
- The file at `OPAQUE_OPRF_KEY_PATH` must continue to store the serialized
  `ServerSetup` struct (128 bytes), as in V1.
- `voprf::OprfServer::new_from_seed(seed, info)` accepts a 32-byte seed
  directly, so the username key can be derived from a root secret.

**Corrected derivation:**

```
oprf_key_bytes  = contents of OPAQUE_OPRF_KEY_PATH (serialized ServerSetup, 128 bytes)
root_secret     = SHA-256(oprf_key_bytes)
username_key    = HKDF-Expand(root_secret, info="username-oprf-v1", length=32)
backup_key      = HKDF-Expand(root_secret, info="backup-encryption-v1", length=32)
```

**Spec impact:** §5.8 was rewritten. The prior text assumed a raw seed;
the corrected text describes the serialized `ServerSetup` format and the
SHA-256 intermediate step.

**Compatibility:** The V1 file format is preserved. Existing V1 key files
work without modification. This is the only V1 storage format V2 does not
change.

**Full report:** `verification/oprf-key-storage/report.md`

---

## Batched Test Execution Model

**Date:** 2026-10-01
**Status:** Complete. Canonical.
**Spec sections affected:** §5.30 (new)

**Question asked:** How should tests be run in an environment with a
540-second tool execution timeout?

**Answer found:** The full integration suite (55+ test binaries) exceeds
the timeout when run unfiltered. Cargo runs test binaries sequentially, and
each binary performs non-trivial setup (SQLite init, migrations, crypto key
generation).

**Solution adopted:**

- Domain-based batches, each running under 60 seconds.
- `server/tests/batch-manifest.toml` declares batch membership.
- `server/Makefile` provides `test-<batch>` targets.
- `server/scripts/check-test-batches.sh` detects orphans.
- `server/tests/test_batch_manifest.rs` enforces the invariant at test time.
- `server/TESTING.md` documents the workflow.

**Enforcement:** Adding a test file without registering it in the manifest
fails both `make check-batches` and `cargo test --test test_batch_manifest`.

**Spec impact:** §5.30 added. All future tasks that add test files must
update the manifest.

---

## Pending Verifications

These are known ambiguities that have not yet been resolved. They do not
block Wave 1 or Wave 2, but must be resolved before their dependent tasks
begin.

| ID | Blocks | Question |
|---|---|---|
| V-B | 34a | Key transparency: RFC 6962 or simplified log? |

---

## How to Add a New Verification

When a verification task produces a report:

1. Add a new section above "Pending Verifications".
2. Include: ID, date, status, spec sections affected, question asked, answer
   found, and a link to the full report.
3. If the answer contradicts the current spec, propose the specific
   amendment and note it here.
4. Update §16 of the spec with an amendment entry.
5. Remove the corresponding row from "Pending Verifications".

---

## Notes

The verification tasks were introduced because two early tasks (the C2SP
chunk size debate and the OPRF key storage format) were being decided by
argument rather than by citation. The rule that emerged:

> Factual disputes about external specifications must be resolved by
> citation, not negotiation.

A verification task is the mechanism for producing that citation. It runs
code, reads documentation, and produces a report. The report is the
evidence. The spec is amended to reflect the evidence.
