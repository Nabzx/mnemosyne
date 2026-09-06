# On-disk format

The layout of a `.mnem/` store: the object database, the refs, and the
provenance index.

This grows alongside the Phase 1 build and is marked frozen for the v0.x line at
the end of Phase 1 (issue #32). It is finalised for v1 in Phase 6.

## Settled so far

- **Container**: one `redb` file, `.mnem/store.redb`, plus plain-text `HEAD` and
  `config`. ADR-0002.
- **Object identity**: BLAKE3 of the object's canonical CBOR bytes. ADR-0005.
- **Encoding**: CBOR restricted to RFC 8949 §4.2 deterministic encoding, plus
  our rules (JSON data model for `content`, finite numbers only, no CBOR tags).
  String-keyed maps. No per-object framing. ADR-0008.
- **Tables**: `objects: [u8; 32] -> Vec<u8>`, `refs: &str -> [u8; 32]`. ADR-0008.
- **`config`**: holds `format_version` and `hash_algo`. ADR-0007, ADR-0005.

## To be written here

- The exact CBOR shape of each object kind (memory node, state, commit).
- The `docs/format/golden-vectors.md` file: objects and their exact hex
  encodings, pinned by the encoder's CI test.
- The `redb` file-format version a store is written with.
- The ref model, once ADR-0009 lands.

Once frozen for a version line, a store written by one release in that line is
readable by every other release in it. A breaking change needs a major version
bump and `mnem migrate` (`AGENTS.md`, ADR-0007).
