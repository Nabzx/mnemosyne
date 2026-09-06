# ADR-0002: On-disk object format

- Status: Accepted
- Date: 2026-09-06
- Issue: #4 (grilling), #3 (research)

## Context

A `.mnem/` store has to hold four kinds of thing: many small **memory nodes**
(thousands per run), a **state** per commit (the set of nodes visible then), the
**commits** that form the graph, and a handful of **refs**. The store must
support cheap branching, a cheap structural diff between two states (Phase 2), a
three-way merge (Phase 3), and a format that a store written by one v1.x release
can be read by every other v1.x release.

The #3 survey compared Git's loose-object and packfile model, an embedded
key-value object store, a prolly tree for state, the `prollytree` crate used
wholesale, Git4Data, and DVC or `oras`. Its recommendation, pressure-tested in
#4:

- Git loose objects hit the many-small-objects failure mode, and building
  packing and garbage collection to escape it is a storage engine we should not
  write.
- `prollytree` as the core data structure would make our frozen format someone
  else's, resting on their release cadence.
- Git4Data needs a database server; DVC and `oras` are built for a few large
  artefacts, not a fine-grained local graph.
- An embedded key-value store removes the small-object problem for free, and a
  prolly tree for state gives cheap diff and node-level merge when those phases
  need them.

This ADR fixes the store architecture. It does not fix anything byte-level.

## Decision

### Engine

The store is a single `redb` file, `.mnem/store.redb`. `redb` is pure Rust, has
no C dependency, is crash-safe, uses copy-on-write B+trees, and has an
explicitly stable, documented on-disk format with an upgrade path. It is named
here as the one load-bearing storage dependency: the freeze-for-a-major-version
commitment rests on `redb`'s format stability as well as our own.

### Object model

Every object (memory node, state, commit) is self-describing and carries a
`kind` tag. A reader that meets an unknown `kind` rejects the store rather than
guessing. Adding a new object kind is an additive, minor change; it does not
break older readers of the objects they do understand.

### Store layout

```
.mnem/
  store.redb     redb database:
                   table  objects : content hash -> object bytes
                   table  refs    : ref name     -> commit hash
  HEAD           plain text, the current pointer, the source of truth
  config         plain text, holds format_version and later the prolly parameters
```

`HEAD` is a plain-text file in the spirit of Git's `HEAD`: it is the one pointer
a person most often reads or sets by hand, so it stays out of the binary store.
Named refs live in the `refs` table.

### State representation

The state is a **plain sorted content-addressed map** (`kind = flat`) in v0.1: an
ordered list of `(node key, node hash)` pairs, itself content-addressed.

A **self-implemented prolly tree** (`kind = prolly`) is added in Phase 2, when
`diff` first needs it, and pays off again for merge in Phase 3. Because objects
are self-describing this is an additive change, not a break. We implement the
prolly tree ourselves so the format stays ours and frozen; the `prollytree`
crate is a reference and a spike target, not a dependency.

### No packfiles, no bespoke garbage collection

Not in the v0.x line. `redb` handles its own compaction. A `mnem compact`
command may be added later if a real store grows unreasonably, but it is not a
launch requirement.

### Legibility

The store is not readable with Git tools, which is expected: our objects were
never Git objects. In their place:

- `mnem cat-object <hash>`: print one object.
- `mnem verify`: check every object's hash, and later its signature.
- `mnem fsck`: check graph consistency (no dangling parents, every ref resolves).
- `docs/format/`: a written spec complete enough for a third party to build a
  reader.

`redb`'s own documented format keeps the outer container inspectable.

### Migration

`config` carries a `format_version` integer. A major version bump ships a
one-shot `mnem migrate` that reads the old store and writes a new one; we commit
to providing that path for any v1 to v2 change. Within a major version only
additive changes are allowed: new object kinds, new optional fields. This
matches the invariant in `AGENTS.md`.

### Scope

This ADR is architectural. It does not decide:

- the serialisation encoding of an object's bytes (CBOR, bincode, other): #19
  and ADR-0008.
- the prolly-tree chunking and encoding parameters: ADR-0008.
- the content hash function and the commit header layout: #15 and ADR-0005.

Those can move without reopening this ADR.

## Consequences

- Phase 1 is smaller: a `redb` table and a flat state, no tree, no packing.
- The many-small-objects problem is gone from day one.
- Phase 2 and Phase 3 get the prolly tree's cheap diff and node-level merge
  without a format break, because the object model was built to absorb it.
- We take one deep dependency, `redb`, and its format stability is now part of
  our promise. If `redb` ever breaks its format without an upgrade path, that is
  our problem to absorb in a `mnem migrate`.
- Anyone wanting to read a store needs our spec, not just `git cat-file`. The
  inspection commands and `docs/format/` are load-bearing, not optional.
- A store cannot be inspected or repaired with generic tools while `mnem` is
  unavailable. `redb`'s documented format is the fallback.

## Alternatives considered

**Git's loose objects and packfiles, via `gitoxide`.** Rejected. Thousands of
tiny loose files per run is the exact case packfiles exist to fix, and building
packing, an index, `repack` and garbage collection ourselves is a large
distraction from the product. `gitoxide` would also pull Git's tree and blob
model into ours.

**A prolly tree from the start, in Phase 1.** Rejected for v0.1. It is real work
with a chunking-boundary subtlety, and Phase 1 does not yet need diff or merge.
The self-describing object model lets us add it in Phase 2 without a break, so
there is no format cost to waiting.

**The `prollytree` crate as the core data structure.** Rejected. It is young and
single-maintainer. Depending on it for the core means our frozen format is
really its format, and the freeze-for-years commitment then rests on its release
cadence. It may also not model our commit header, provenance or signing without
a fork. Kept as a reference and a spike target.

**A hand-rolled append-only log plus index.** Rejected. That is building a
storage engine. `redb` already solves crash safety, compaction and small-object
locality, with a stable format.

**Git4Data (arXiv 2609.02106).** Rejected. It needs a database server. Mnemosyne
v1 is a local, embeddable, offline library. The "cost proportional to the
change" principle is one we keep, through structural sharing in the prolly tree.

**DVC or `oras` style artefact stores.** Rejected. Both are built for a small
number of large artefacts with an external remote, not a fine-grained local
graph of thousands of tiny objects.
