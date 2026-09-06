# On-disk format

The layout of a `.mnem/` store: the object database, the refs, and the
provenance index.

This grows alongside the Phase 1 build and is marked frozen for the v0.x line at
the end of Phase 1 (issue #32). It is finalised for v1 in Phase 6.

## Settled so far

- **Container**: one `redb` file, `.mnem/store.redb`, plus plain-text `HEAD` and
  `config`. ADR-0002.
- **Object identity**: BLAKE3 of the object's canonical CBOR bytes. ADR-0005.
- **Encoding**: CBOR restricted to a deterministic profile (ADR-0008),
  implemented in `mnem-core`'s `codec` module:
  - RFC 8949 §4.2: map keys sorted bytewise by their encoded form; shortest-form
    integers; definite-length items only.
  - Mnemosyne's own rules: every float is encoded as a 64-bit CBOR float, never
    16 or 32 bit (simpler and unambiguous, a legal narrowing of §4.2). No CBOR
    tags. `content` is the JSON data model with finite numbers only; a
    non-finite float is rejected at the SDK boundary.
  - String-keyed maps, no positional arrays. No per-object framing: an object's
    bytes are exactly its canonical CBOR, and `kind` is a map entry.
  - Verified by the `codec` tests: idempotence, and key-order independence
    inside `content`.
- **Tables**: `objects: [u8; 32] -> Vec<u8>`, `refs: &str -> [u8; 32]`. ADR-0008.
- **`config`**: holds `format_version`, `hash_algo`, and `default_branch`.
  ADR-0007, ADR-0005, ADR-0009.
- **Refs**: `refs` table, flat `name -> commit id`. `HEAD` is a one-line text
  file: `ref: <branch>` or a bare commit id. Ref moves are compare-and-swap.
  ADR-0009.

## To be written here

- The exact CBOR shape of each object kind (memory node, state, commit).
- The `docs/format/golden-vectors.md` file: objects and their exact hex
  encodings, pinned by the encoder's CI test.
- The `redb` file-format version a store is written with.

Once frozen for a version line, a store written by one release in that line is
readable by every other release in it. A breaking change needs a major version
bump and `mnem migrate` (`AGENTS.md`, ADR-0007).
