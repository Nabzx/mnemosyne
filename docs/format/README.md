# On-disk format

The layout of a `.mnem/` store: the object database, the refs, `HEAD`, and
`config`. This is the specification. It is enough to write a reader in another
language, and it is checked against the implementation by the tests in
`crates/mnem-core` and the [golden vectors](golden-vectors.md).

## Status: final for Era 1

This describes **`format_version` 1**, and as of `v0.0.7` it is **final for the
Era 1 substrate**. Every `0.0.x` release reads and writes `format_version` 1, a
store written by one reads on every other, and the substrate (commit, branch,
merge, blame, bisect, time travel) needs no further field. The benchmark
(`docs/benchmark.md`) is the evidence that it does what it claims.

The next change is **`format_version` 2 in Era 2**, the collaboration layer: a
stored `Contradiction` object (a kept record of an incompatible-claims merge),
whatever the sync protocol adds to a `Commit` or the refs, and the prolly-tree
`State` form if the benchmark's storage numbers cross the trigger in ADR-0017.
Those are additive where possible (ADR-0002); where not, `mnem migrate` ships
with the release (ADR-0007). It is never a silent change: the golden-vector
test fails first.

The software version is `0.0.x` initial development, where the public API may
change on any release. The **`format_version` integer is the data contract**,
and it is the thing to check, not the software version. A release that meets a
`format_version` it does not support refuses the store with a message naming the
version it would need.

A later format change bumps `format_version` by one and, if it would make an
older reader misread an existing object, ships `mnem migrate` (ADR-0007). It is
never a silent change: the golden-vector test fails first.

The prolly-tree state form and the signature table are named below but are not
part of `format_version` 1.

## The store directory

```
.mnem/
  store.redb    the object, ref, staging and index tables (see below)
  HEAD          the current position, one line of text
  config        key = value lines
```

`.mnem/` sits at the store root. Nothing else in the directory is read.

### Discovery

`open` walks up from the given path to the nearest ancestor that contains a
`.mnem/` directory, and uses that. `init` refuses to create a store at a path
that already has one, or anywhere beneath an existing store: stores do not nest.

### Atomicity

`config` and `HEAD` are written by writing a sibling temporary file, syncing it,
and renaming it over the target, so a reader never sees a half-written file. A
`commit` is one `redb` write transaction (ADR-0008): it lands whole or not at
all.

## `config`

A UTF-8 text file of `key = value` lines. Whitespace around the key and value is
trimmed. A line whose first non-space character is `#` is a comment. A blank
line is ignored. An unknown key is ignored, so a later version can add keys
without breaking an older reader's parse.

| Key | Value | Meaning |
| --- | --- | --- |
| `format_version` | integer | the format this store is written in, `1` for this line. A reader that supports up to version *N* accepts a store with `format_version` &le; *N* and refuses anything higher. |
| `hash_algo` | string | the object hash. `blake3` is the only accepted value. |
| `default_branch` | string | the branch `HEAD` points at in a fresh store, `main` unless overridden at `init`. |

All three keys are required. A missing key, or a `hash_algo` other than
`blake3`, is a corrupt store.

A fresh store:

```
# Mnemosyne store config. See docs/format/.
format_version = 1
hash_algo = blake3
default_branch = main
```

## `HEAD`

One line of UTF-8 text, with an optional trailing newline. Either:

- `ref: <branch>` &mdash; attached to a branch. `<branch>` follows the ref name
  rules below. The next commit moves that branch.
- `<64 hex characters>` &mdash; detached, pointing straight at a commit id. A
  commit from a detached `HEAD` is refused; check out a branch first.

Anything else is a corrupt store. A fresh store's `HEAD` is `ref: main`.

## The database

One `redb` file, `.mnem/store.redb`. `redb` is a pure-Rust embedded key-value
store with a single-file, copy-on-write B-tree layout; its own file-format
version travels inside the file and is handled by the `redb` crate, pinned at
major version 2 (ADR-0008).

Five tables:

| Table | Key | Value | Holds |
| --- | --- | --- | --- |
| `objects` | 32 raw bytes, an `ObjectId` | the object's canonical CBOR | every memory node, state and commit |
| `refs` | branch name, UTF-8 | 32 raw bytes, a commit `ObjectId` | one row per branch |
| `staging` | node id, UTF-8 | 32 raw bytes, a node `ObjectId` | the nodes staged for the next commit |
| `staging_tombstones` | node id, UTF-8 | empty | the nodes staged for deletion (ADR-0012) |
| `commit_nodes` | 32 raw bytes, a commit `ObjectId` | CBOR `{ node id -> "added" \| "modified" \| "removed" }` | each commit's change set against its first parent (ADR-0015) |

`objects` is append-only in practice: an id is the hash of its bytes, so writing
the same object twice is a no-op and an entry is never rewritten. `staging` and
`staging_tombstones` are local working state and are not part of the portable
history; a fresh clone or a different machine does not carry them. `commit_nodes`
is a **derived index**: every entry is recomputable from the `objects` table
(`Store::rebuild_index`), it is not part of `format_version`, and a reader that
finds it missing or stale falls back to recomputing from the two states.

## Object identity

An object's id is the BLAKE3 hash of its canonical CBOR bytes (ADR-0005). It is
32 bytes. In text it is 64 lowercase hex characters. In CBOR, anywhere an id
appears as a value, it is a **byte string of length 32** (`0x58 0x20` then the
bytes), never an array of integers.

There is no framing around an object: the bytes in the `objects` table are
exactly the canonical CBOR of the object, and the `kind` field inside that map
is what tells a reader what it is holding.

## Canonical encoding

Objects are encoded as CBOR (RFC 8949) restricted to a deterministic profile, so
that the same logical object always produces the same bytes and therefore the
same id. The profile is RFC 8949 section 4.2 plus three Mnemosyne rules.

From RFC 8949 section 4.2:

- **Map keys are sorted** by the bytewise comparison of their encoded form.
  Because CBOR keys are length-prefixed, this is length-first: a shorter key
  sorts before a longer one, and keys of equal length sort bytewise. Every key
  in this format is a text string.
- **Integers are in shortest form**: the smallest of the 0, 1, 2, 4 or 8 byte
  encodings that holds the value. CBOR has no 3, 5, 6 or 7 byte integer, so a
  value needing five bytes is written in eight.
- **Definite-length only**: no indefinite-length strings, arrays or maps.

Mnemosyne's rules:

- **Every float is a 64-bit CBOR float** (`0xfb`), never 16 or 32 bit. This is a
  legal narrowing of the profile and keeps float encoding unambiguous. `content`
  numbers follow the JSON data model and must be finite; a non-finite number is
  rejected before it reaches the store.
- **No CBOR tags.** No value carries a tag.
- **String-keyed maps only.** Every map, at every level including inside
  `content`, has text-string keys. No integer keys, no positional array standing
  in for a struct.

The encoder builds a CBOR value, sorts every map by encoded key, then writes it.
The `codec` tests check idempotence and that the key order inside `content` does
not affect the bytes.

## Object kinds

Every object is a CBOR map with a `kind` entry. Three kinds. An unknown kind is a
clean decode error, not a silent skip.

### `memory_node`

The versioned unit (ADR-0003). Successive writes with the same `id` are updates
to one logical node; each write is its own immutable object.

| Field | CBOR type | Required | Notes |
| --- | --- | --- | --- |
| `kind` | text string | yes | `"memory_node"` |
| `id` | text string | yes | the stable logical key |
| `content` | any JSON value | yes | stored verbatim; string-keyed maps, finite numbers |
| `content_kind` | text string | yes | `"note"` or `"claim"`. `0.0.x` only writes and reads `"note"`; `"claim"` is defined for a later era |
| `provenance` | map | omitted when empty | see below; an entirely empty provenance is not encoded |
| `event_time` | integer | omitted when absent | when the agent formed the node, Unix milliseconds, may be negative |

`provenance` (ADR-0003), all fields optional text strings, each omitted when
unset: `agent_step`, `observation`, `tool_call`, `source`, `note`.

### `state`

The set of memory nodes visible at a commit (ADR-0003).

| Field | CBOR type | Required | Notes |
| --- | --- | --- | --- |
| `kind` | text string | yes | `"state"` |
| `nodes` | map | yes | node id (text) to that node's `ObjectId` (32-byte string), one entry per logical node, in canonical key order |

`format_version` 1 has only this flat form. A prolly-tree form arrives as a
second shape in a later `format_version`.

### `commit`

A commit (ADR-0005). Its hashed bytes are exactly this object. A signature, if
one exists, lives in a separate table, is not part of the commit and never
changes its id; that table is not in `format_version` 1.

| Field | CBOR type | Required | Notes |
| --- | --- | --- | --- |
| `kind` | text string | yes | `"commit"` |
| `parents` | array of 32-byte strings | yes | empty for the first commit, one normally, two or more for a merge. Order is significant; `parents[0]` is the first parent |
| `state` | 32-byte string | yes | the `ObjectId` of this commit's `state` |
| `message` | text string | yes | freeform |
| `author` | text string | yes | who or what made the commit, opaque to the core |
| `time` | integer | yes | the record time, Unix milliseconds. Distinct from a node's `event_time` |

## Refs and branches

`format_version` 1 has branches only, in a flat namespace (ADR-0009). A branch
is a row in the `refs` table: a name mapping to a commit id. The default is
`main`. There is no `refs/heads/` hierarchy and no tags.

A ref name must be non-empty, must not be `HEAD`, must not begin or end with `/`
or `.`, must not contain `..`, whitespace or control characters, and may
otherwise contain letters, digits, `-`, `_`, `.` and `/`.

Ref moves are compare-and-swap: an update names the commit it expects the branch
to be at, and fails if the branch has moved. The first commit on a branch
creates its row. A reflog is reserved but not written in `format_version` 1.

## Golden vectors

[`golden-vectors.md`](golden-vectors.md) lists concrete objects with their exact
CBOR bytes and ids. `crates/mnem-core/tests/golden_vectors.rs` pins them and
checks the doc against the code, so neither can drift without a test failing.
That failure is the signal that a `format_version` bump is due.
