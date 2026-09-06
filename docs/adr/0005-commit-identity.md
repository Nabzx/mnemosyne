# ADR-0005: Commit identity, hashing and signing

- Status: Accepted
- Date: 2026-09-06
- Issue: #16 (grilling), #15 (research)

## Context

ADR-0002 fixed that objects are content-addressed and left "assume a 32-byte
content hash" for this ticket. ADR-0003 fixed the memory node model and the
`event_time` field. This ADR decides the hash function, the signing scheme, and
the fields of a commit.

The #15 survey found: BLAKE3 is 4 to 10 times faster than SHA-256 and
parallelises, with the same security profile; git chose SHA-256 for ecosystem
interop, which a local-first tool with its own format does not need; SSH-style
ed25519 signing has displaced GPG; and a content-addressed store is better served
by keeping signatures out of the hashed object.

## Decision

### Hash function

The content hash is **BLAKE3**, 32 bytes, displayed as 64 hex characters. `mnem`
accepts an unambiguous hex prefix, in the spirit of git's short hashes.

`config` records `hash_algo: "blake3"`. There is no runtime switch. A store is
single-algorithm for its life; moving to another hash is a `mnem migrate` under
ADR-0002's migration story, not a toggle.

### Object identity

An object's id is the BLAKE3 hash of its **deterministic canonical
serialisation**. The requirement is deterministic and canonical: the same
logical content produces the same bytes and therefore the same id. The encoding
that achieves this (canonical CBOR, or a hand-rolled canonical form) is
ADR-0008's decision.

For a commit, the hashed content is exactly:

```
{ kind, parents, state, message, author, time }
```

and nothing else. The signature is not in the hashed content.

### The commit object

| Field | Type | Notes |
| --- | --- | --- |
| `kind` | string, always `"commit"` | ADR-0002's self-describing objects |
| `parents` | list of commit ids | `[]` for the first commit, one normally, two or more for a merge. No sentinel. |
| `state` | state id | the state this commit points at |
| `message` | string | freeform |
| `author` | string | opaque to the core. `mnem` sets it from config or the caller passes it. Structured agent identity is a v2 and v3 concern and can be added as a new field additively. |
| `time` | integer | Unix milliseconds. The record time, always set by `mnem` at commit. Distinct from a memory node's `event_time`, which uses the same representation. |

Not present:

- **`committer`**: no rebase or patch flow in v1, so one `author` is enough.
- **a run reference**: grouping commits by agent run is deferred; not in v0.1.
- **`signature`**: held separately, see below.
- **`format_version`**: lives in `config`, not per commit (ADR-0002).

### Signing

Signing uses **ed25519**. A signing key has the same shape as an SSH
`id_ed25519`, so a user can reuse an existing one.

Signatures live in a **side table**, not in the commit object:

```
signatures : commit id -> { public_key, signature, signed_at }
```

The signature covers a **signable payload**, `{ commit_id, signer_key_id,
signed_at }`, signed as a whole. This binds "this key attested this commit at
this time". Signing the bare commit id would not bind the signer or the time;
signing the full canonical bytes would be redundant, since the id already
commits to them.

Because signatures are a side table:

- A commit with no signature is valid. An unsigned store is normal.
- A commit can carry several signatures.
- A signature can be added after the fact, or by someone other than the author.
- Signing never changes a commit's id.

The table value can later hold a Sigstore-style certificate bundle in place of a
bare signature. Sigstore itself is not adopted in v1, because it needs Fulcio
and Rekor, which is heavy infrastructure for a local-first tool.

### What ships in v1

- `mnem verify`: check every object's id against its bytes, and check any
  signatures in the side table. (ADR-0002 already committed this.)
- `mnem sign`: attach an ed25519 signature to a commit. Thin: a vetted crate, the
  side table, and the signable payload.

Signing is never required by any operation.

## Consequences

- Hashing is fast and parallel, which matters when a commit hashes a large state.
- Commit identity is purely a function of content. The graph is stable under
  signing, re-signing, and multi-party signing.
- BLAKE3 is not a NIST standard, so a FIPS-bound user cannot use Mnemosyne until
  a SHA-256 mode exists. The `hash_algo` field reserves that path; it is not
  built.
- A bare-string `author` means the core offers no identity guarantees in v1. The
  v2 agent-identity work fills this in.
- `time` as Unix milliseconds is unambiguous and trivial to serialise, at the
  cost of not carrying a timezone. `mnem` renders local time on display.

## Alternatives considered

**SHA-256, to match git and Git 3.0.** Rejected. Git's reason is interop with a
huge installed base and forge support; Mnemosyne interoperates with nothing
outside a `.mnem/` store. BLAKE3 is faster with no security cost.

**A configurable hash from day one.** Rejected. A store never mixes hashes, so
the abstraction would exist only for a migration that a `mnem migrate` handles
better.

**A signature field in the commit object, like git.** Rejected. It ties commit
identity to the signature, so re-signing or adding a second signature rewrites
the commit and orphans its children. Git accepts this partly for transport
reasons we do not have.

**Signing the bare commit id.** Rejected. It does not bind who signed or when.

**Structured `author` now.** Rejected. Agent identity is unsettled and belongs
with the v2 work; a string does not block that and stays forward-compatible.

**`committer` alongside `author`.** Rejected. The split serves git's rebase and
patch flows, which v1 does not have.
