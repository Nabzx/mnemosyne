# ADR-0008: Object encoding and the store engine

- Status: Accepted
- Date: 2026-09-06
- Issue: #20 (grilling), #19 (research)

## Context

ADR-0002 fixed `redb` as the store engine and left the byte encoding of an
object to this ticket. ADR-0005 requires a deterministic canonical serialisation:
the same logical object must produce the same bytes, and therefore the same
BLAKE3 hash. ADR-0002 and ADR-0007 require that objects are self-describing and
that additive changes do not break an older reader.

The #19 survey compared bincode, postcard, MessagePack, CBOR and JSON. bincode
and postcard are the fastest and smallest, but have no canonical mode and no
schema evolution. JSON is legible but large, slow, and cannot hold bytes
natively. CBOR is the only candidate that is self-describing, has a standardised
deterministic encoding (RFC 8949 §4.2), and stays legible through diagnostic
notation.

## Decision

### Format

Objects are encoded as **CBOR** (RFC 8949), restricted to a deterministic
profile.

### The deterministic profile

The base is **RFC 8949 §4.2**:

- integers and floats in their shortest form that round-trips (preferred
  serialisation)
- no indefinite-length items
- map keys sorted in bytewise lexicographic order of their encoded form

Two additions of our own, recorded here and in `docs/format/`:

- `content` (ADR-0003) is the JSON data model exactly. Finite numbers only. A
  non-finite float (`NaN`, `±Infinity`) is rejected at the SDK boundary and
  never reaches the core. Finite floats keep their value; they are not reduced
  to integers (this is not dCBOR).
- No CBOR tags are used. Every value is a plain CBOR string, byte string,
  integer, float, boolean, null, array or map.

CDE and dCBOR were considered and not adopted: both are still drafts, and
dCBOR's float-to-integer reduction would silently rewrite an agent's `content`.

### Struct-to-CBOR mapping

Every object is a **string-keyed CBOR map**. The `kind` key is present in every
object and is conventionally written first, though the deterministic profile
sorts keys regardless.

Positional arrays were rejected: legibility and additive evolution are explicit
requirements, and an array makes adding a field fragile and the object
unreadable. The cost of repeating short field names per object is accepted.

### Implementation

`ciborium` provides the CBOR data model. A thin pass of ours enforces the
deterministic profile on encode: sort map keys, force shortest-form numbers,
forbid indefinite-length. The rule set is small and is tested against the RFC's
own examples.

`cbor2` (canonical encoding built in) and a fully hand-rolled serialiser were
considered. `cbor2` has a short track record, which is a real risk for a
dependency the frozen format rests on. A hand-rolled serialiser is more than is
needed once `ciborium` covers the data model.

### No per-object framing

An object's on-disk bytes are exactly its canonical CBOR. There is no magic
number, no length prefix, no version byte on the object. `kind` identifies the
object type; `format_version` lives in `config` (ADR-0007); `redb` frames the
key and value.

### The store engine layer

Two tables in the one `redb` file:

```
objects : [u8; 32]  ->  Vec<u8>     BLAKE3 hash to canonical CBOR bytes
refs    : &str       ->  [u8; 32]   branch name to commit hash
```

`redb = "2"` is pinned in `Cargo.toml`. `docs/format/` records the `redb`
file-format version a store was written with. A `mnem commit` is a single
**durable** `redb` write transaction: fsync on commit, correctness over speed. A
`--no-fsync` escape hatch for bulk import may be added later.

### Verifying the canonical encoding

Two CI checks, from Phase 1:

- **A property test**: `encode(decode(encode(x))) == encode(x)` for generated
  objects (idempotence), and two semantically equal objects (for instance a map
  built with keys inserted in different orders) encode to identical bytes.
- **Golden vectors** in `docs/format/`: a fixed set of objects and their exact
  hex encodings, pinned by the test, so an accidental change to the encoder is
  caught immediately.

## Consequences

- The encoder is a small amount of our own code on top of `ciborium`, fully
  testable against a public spec.
- Objects are inspectable: `mnem cat-object` decodes the CBOR and prints
  diagnostic notation, or JSON with a flag.
- Decode is slower than bincode. For a store read a few objects at a time this
  does not matter, and it buys the three properties bincode lacks.
- `redb` and `ciborium` are the two dependencies the frozen format rests on.
  Both are widely used with stable formats. A break in either is absorbed by a
  `mnem migrate` under ADR-0007.
- `mnem commit` fsyncs, so a commit-heavy workload is bounded by disk sync
  latency until the `--no-fsync` flag exists.

## Alternatives considered

**bincode or postcard.** Rejected. No canonical mode, not self-describing, no
schema evolution. Wrong tool for a frozen inspectable format.

**MessagePack.** Rejected. Its deterministic story is weaker than CBOR's §4.2,
and its diagnostic tooling is poorer.

**JSON with JCS (RFC 8785).** Rejected. Largest and slowest, and no native byte
strings, so a BLAKE3 hash inside an object would need base64.

**CDE or dCBOR.** Rejected for now. Both are drafts; dCBOR alters data.

**`cbor2` for its built-in canonical encoding.** Rejected. Too new to be a
load-bearing frozen dependency; `ciborium` plus a small pass is safer.

**Positional CBOR arrays instead of maps.** Rejected. Fights legibility and
additive evolution.
