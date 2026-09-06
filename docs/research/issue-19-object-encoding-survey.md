# Research: object encoding and the store engine (issue #19)

Feeds ADR-0008. Two questions: which storage engine, and which byte encoding for
an object.

ADR-0002 already settled most of the engine question: one `redb` file, objects
and refs as tables, self-describing objects with a `kind` tag, state `flat` in
v0.1. ADR-0005 requires a **deterministic canonical serialisation**, and left the
encoding to this ticket. So the engine part here is a confirmation and a layer
sketch; the encoding part is the real work.

## Storage engine

### redb, confirmed

`redb` is a pure-Rust ACID key-value store: copy-on-write B+trees, a stable and
documented file format, and per-transaction durability. Nothing found in a 2026
scan changes the ADR-0002 conclusion. `fjall` (LSM) and `sled` (perpetually
beta) remain the also-rans; `rocksdb` is a C++ dependency.

One property worth stating that ADR-0002 did not: **a `mnem commit` is a single
`redb` write transaction.** Writing the memory-node objects, the state object,
the commit object, and the ref update all happen in one atomic transaction. A
store is therefore never half-committed, and a crash mid-commit rolls back to the
previous commit. Crash-atomicity is free.

### The object-database layer

Two tables in the one `redb` file:

```
objects : [u8; 32]  ->  Vec<u8>     key is the BLAKE3 hash, value is the encoded object
refs    : &str       ->  [u8; 32]   branch name to commit hash; HEAD lives in the text file
```

`redb` supports `&[u8]` and `&str` keys and values with zero-copy reads, so no
richer typing is needed. Object lookup is a point query on `objects`. Writing an
object is idempotent: compute the hash, insert if absent.

## Object encoding

### The requirements, from earlier ADRs

1. **Deterministic and canonical** (ADR-0005): the same logical object must
   produce the same bytes, so the same hash.
2. **Self-describing and evolvable** (ADR-0002, ADR-0007): objects carry a `kind`
   tag, and additive changes (a new field, a new kind) must not break an older
   reader of the objects it does understand.
3. **Legible** (ADR-0002): `mnem cat-object` must produce something a person can
   read.
4. **Compact and quick** (ADR-0002): many small objects per commit.

### The candidates

| Format | Deterministic mode | Self-describing | Schema evolution | Legibility | Size and speed |
| --- | --- | --- | --- | --- | --- |
| **bincode** | none | no | no (field order is declaration order) | none | fastest, small |
| **postcard** | none | no | no | none | ~1.5x bincode, ~70% its size |
| **MessagePack** (`rmp-serde`) | weak, no standard | partly (type tags) | tolerable | poor | smallest on the wire, slower to decode |
| **CBOR** (`ciborium`, `cbor2`) | **RFC 8949 §4.2, plus the CDE and dCBOR profiles** | yes | yes (unknown fields skippable or capturable) | good (CBOR diagnostic notation) | mid size, fast encode, slower decode in `ciborium` |
| **JSON / JCS** (RFC 8785) | JCS is a full standard | yes | yes | best | largest, slowest, no native bytes |

### Why the binary schema-coupled formats are out

bincode and postcard are the fastest and smallest, but they fail requirements 1,
2 and 3 outright. They have no canonical mode, they are not self-describing, and
adding a field is a silent break unless every object is manually versioned. For a
project whose whole thesis is a stable, inspectable, frozen format, they are the
wrong tool.

### Why CBOR wins

CBOR is the only candidate that meets all four requirements:

- **Determinism is standardised.** RFC 8949 §4.2 defines deterministic encoding:
  preferred (shortest-form) integers and floats, no indefinite-length items,
  and map keys in bytewise sorted order. The stricter CDE and dCBOR profiles go
  further. This is exactly ADR-0005's requirement, already specified and tested
  by others, rather than rules we invent.
- **It is self-describing.** The `kind` tag is just the first entry of the CBOR
  map. An older reader meeting an unknown field can skip it; a newer object kind
  is refused cleanly by its `kind`. This matches ADR-0002 and ADR-0007 with no
  extra machinery.
- **It is legible.** CBOR diagnostic notation renders an object as readable text,
  and `mnem cat-object` can emit that or JSON.
- **It carries bytes natively**, which JSON cannot without base64, and it is far
  more compact.

The cost is decode speed: `ciborium` decodes CBOR more slowly than it encodes,
and slower than bincode. For a store of small objects read a few at a time this
is not a real constraint, and it is the correct trade for the three requirements
bincode fails.

### The memory-node `content` field

ADR-0003 says `content` is a string or any JSON value. CBOR's data model is a
superset of JSON's, so `content` maps directly: a CBOR string, or a CBOR
map/array/number/bool/null. The one care point is floats: deterministic CBOR has
specific float rules (shortest form that round-trips), and an agent's JSON
content can contain floats. The encoder must apply those rules to `content` too,
not just to our own fields.

## Recommendation for ADR-0008

- **Engine:** `redb`, one file, tables `objects` and `refs` as above. A commit is
  one write transaction.
- **Encoding:** **CBOR, restricted to a deterministic profile.** Start from RFC
  8949 §4.2; the grilling decides whether to adopt the stricter CDE or dCBOR
  profile.
- **Crate:** `ciborium` for the CBOR data model (mature, ubiquitous), with a thin
  deterministic-ordering and preferred-form pass on top, since the §4.2 rule set
  is small and well specified. `cbor2` is the alternative, with §4.2 canonical
  encoding built in but a shorter track record. The grilling picks.
- **Framing:** none beyond CBOR itself. `kind` is the first map entry.
  `format_version` stays in `config` (ADR-0007), not per object. `redb` frames
  the key and value.
- **`mnem cat-object`:** renders CBOR diagnostic notation by default, JSON with
  a flag.
- The exact profile, the struct-to-CBOR mapping, and the float rules go in
  `docs/format/`.

## Open questions for the grilling (#20)

- `ciborium` plus a hand-rolled §4.2 pass, versus `cbor2`'s built-in canonical
  encoding, versus a fully hand-rolled canonical binary form. This survey leans
  `ciborium` plus a thin pass.
- Which determinism profile exactly: RFC 8949 §4.2 core, or CDE, or dCBOR.
- Confirm `content` maps straight to the CBOR data model, and how floats inside
  `content` are canonicalised.
- Confirm no per-object framing header: `kind` as the first CBOR map entry is
  enough.

## Sources

- [redb design doc](https://github.com/cberner/redb/blob/master/docs/design.md) and [redb 1.0 release](https://www.redb.org/post/2023/06/16/1-0-stable-release/)
- [RFC 8949 (CBOR), Section 4.2 deterministic encoding](https://www.rfc-editor.org/rfc/rfc8949.html) and [CBOR: On Deterministic Encoding (draft-bormann-cbor-det)](https://www.ietf.org/archive/id/draft-bormann-cbor-det-04.txt)
- [dCBOR deterministic profile](https://blockchaincommons.github.io/WIPs-IETF-draft-deterministic-cbor/draft-mcnally-deterministic-cbor.html) and [the CBOR determinism chapter](https://cborbook.com/part_2/determinism.html)
- [`cbor2` crate (RFC 8949, canonical encoding)](https://github.com/ldclabs/cbor2)
- [Rust serialization benchmark](https://github.com/djkoloski/rust_serialization_benchmark) and [json vs bin sizes](https://github.com/zeenix/json-vs-bin)
- [RFC 8785 JSON Canonicalization Scheme](https://datatracker.ietf.org/doc/rfc8785/)
