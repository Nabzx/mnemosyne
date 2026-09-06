# Research: on-disk object format survey (issue #3)

Feeds ADR-0002. Question: what on-disk shape should a `.mnem/` store take?

## What the store has to hold

- **Memory nodes.** Many small objects. A run can produce thousands, each a
  short piece of text or JSON plus provenance. This is the dominant object by
  count.
- **State.** The set of memory nodes visible at a commit. Needs to be
  content-addressed so an unchanged state is free to reference, and needs a
  cheap structural diff between two states (Phase 2) and a three-way merge
  (Phase 3).
- **Commits.** A small object: parent hashes, a state hash, author, timestamp,
  message, signature. Forms the graph.
- **Refs.** Branch names and HEAD. Few, small, frequently updated.

Constraints from `AGENTS.md`: the format is frozen within a major version, the
core is pure and offline, and a store written by one v1.x release is readable by
every other v1.x release. From the plan: the moat is our format, so the core
data structures should be ours, not borrowed wholesale.

## Options

### 1. Git's model: loose objects, packfiles, refs as files

Content-addressed objects written as individual zlib-compressed files under
`objects/ab/cdef...`, consolidated into packfiles over time, refs as plain files
under `refs/`.

- **Many small objects.** Poor. This is the exact failure mode Git added
  packfiles to escape: directory traversal, file-open overhead, metadata churn,
  weak locality. Thousands of loose files per run is the bad case.
- **Branch cost.** O(1). A branch is one small ref file.
- **Merge.** Nothing built in for our state shape. Git's merge is line and tree
  oriented; our state is a set of nodes.
- **Format stability.** Excellent and battle-tested. `gitoxide` gives a mature
  Rust reader and writer.
- **Build effort.** High. To avoid the loose-object problem we would have to
  build packing, an index, `repack`, and garbage collection. That is a storage
  engine, and a large distraction from the product.

### 2. Content-addressed objects in an embedded key-value store

Objects serialised and stored by hash as keys in an embedded store such as
`redb`. Refs in a small table.

- **Many small objects.** Good. The store is one file; there is no per-object
  filesystem cost. `redb` uses copy-on-write B+trees and has lmdb-class
  performance.
- **Branch cost.** O(1). A branch is one row in a refs table.
- **Merge.** Not provided, but the state can be given a structure that supports
  it (see option 3).
- **Format stability.** `redb`'s on-disk format is explicitly stable with a
  documented upgrade path. It becomes our one load-bearing storage dependency.
- **Build effort.** Low to moderate. We build the object model, the graph walk,
  and the operations. We do not build a storage engine.

Candidate engines: **`redb`** (pure Rust, single file, crash-safe, stable
format, B+tree, mature), `fjall` (LSM, write-optimised, younger), `sled`
(perpetually beta, effectively unmaintained), `rocksdb` (C++ dependency, heavy).
`redb` is the clear pick.

### 3. Prolly tree for state, key-value object store for the rest

The state at each commit is a **prolly tree**: a content-addressed,
history-independent probabilistic B-tree mapping a node key to a node hash.
Commits, memory nodes and refs live in a `redb` object store as in option 2.

This is DoltDB's storage model, and it exists as Rust crates (`prollytree`,
`prolly-map`).

- **Many small objects.** Good, same as option 2 for the blob store. The prolly
  tree adds internal nodes but chunking keeps the count reasonable.
- **Branch cost.** O(1), and structural sharing means a large state costs only
  its delta when it changes.
- **Diff.** Cheap and structural. An in-order walk of two content-addressed
  trees emits exactly the changed keys, skipping identical subtrees by hash.
- **Merge.** Three-way merge works at the tree-node level without enumerating
  every node. Non-overlapping key changes merge with no conflict.
- **Format stability.** Ours to fix, if we implement the tree rather than depend
  on a crate. Chunk parameters go in the store config.
- **Build effort.** Moderate to high. A minimal prolly tree is a few hundred
  lines, but it is real work and needs care around the chunking boundary.

### 4. Depend on the `prollytree` crate wholesale

`prollytree` already offers a content-addressed key-value store with branching,
three-way merge, cryptographic proofs, and Git-backed storage. It could cover
much of Phases 1 to 3.

- **Upside.** A large head start.
- **Downside.** It is young and single-maintainer. Making it the core data
  structure means our frozen format is really its format, and the
  freeze-for-years invariant then rests on someone else's release cadence. It
  also may not model our commit header, provenance, or signing without forking
  it.

Useful as a reference and for a spike, not as the core dependency.

### 5. Git4Data / a relational database with branching

Git4Data (arXiv 2609.02106, MatrixOrigin and Purdue) treats a table as a
versioned object and exposes snapshot, branch, diff and merge through SQL, with
cost proportional to the change size via immutable object storage and MVCC. It
is implemented in MatrixOne.

- **Wrong deployment shape.** It needs a database server. Mnemosyne v1 is a
  local, embeddable, offline library. Rejected on that alone, though the
  "cost proportional to the change" principle is one we want.

### 6. DVC or `oras` style content-addressed artifact stores

DVC tracks large files with metadata in Git and data in a cache keyed by hash,
synced to an external remote. `oras` stores artefacts in an OCI registry.

- **Wrong granularity.** Both are built for a small number of large artefacts
  with a remote, not a fine-grained local graph of thousands of tiny objects.
  Rejected.

## Comparison

| | many small objects | branch cost | structural diff | merge | format stability | build effort |
| --- | --- | --- | --- | --- | --- | --- |
| 1 Git loose + pack | poor without packing | O(1) | no | no | excellent | high (build packing + GC) |
| 2 KV object store | good | O(1) | no | no | good (`redb` stable) | low to moderate |
| 3 prolly state + KV | good | O(1), delta-cost | yes, cheap | yes, node-level | ours to fix | moderate to high |
| 4 `prollytree` crate | good | O(1) | yes | yes | not ours | low, but risky |
| 5 Git4Data | n/a | n/a | yes | yes | n/a | n/a, wrong shape |
| 6 DVC / oras | n/a | n/a | no | no | n/a | n/a, wrong shape |

## Recommendation for ADR-0002

**Option 2 now, option 3 by Phase 3. Concretely:**

1. `.mnem/` is one `redb` file (`store.redb`), plus a plain-text `HEAD` and a
   plain-text `config` for legibility and easy inspection.
2. An **object database**: a `redb` table `objects: hash -> bytes` holding
   memory nodes, states and commits. The hash function and commit header are
   ADR-0005's call; assume a 32-byte content hash for now.
3. **Refs**: a `redb` table `refs: name -> hash`, with `HEAD` mirrored to the
   text file.
4. **State**: a plain sorted content-addressed map in v0.1 (option 2). The
   **prolly tree lands in Phase 2**, when `diff` first needs it, and pays off
   again for merge in Phase 3 (option 3). We implement a minimal prolly tree
   ourselves so the format stays ours and frozen; `prollytree` is a reference
   and a spike target, not a core dependency.
5. No packfiles, no bespoke garbage collection in v0.x. `redb` handles
   compaction. A `mnem compact` command can come later if a store grows
   unreasonably.
6. Legibility is recovered with `mnem cat-object`, `mnem verify` and `mnem fsck`
   style commands and a written format spec, so anyone can build a reader.

**Rationale.** The workload is thousands of tiny objects per run, which is
exactly where Git's loose objects fall over, and building packing and GC to fix
that is a storage engine we should not write. `redb` removes the small-object
problem for free, is pure Rust, and has a stable documented format. Deferring the
prolly tree to Phase 2 keeps Phase 1 small and de-risked while still reaching the
structure DoltDB has proven for this shape before diff and merge need it.

## Open questions for the grilling (#4, ADR-0002)

- State in v0.1: a plain sorted map, or the prolly tree from the start. This
  survey recommends the plain map and a Phase 2 upgrade; the grilling should
  confirm the format can absorb that change without a version bump, or decide to
  pay the prolly-tree cost up front.
- Is `redb` acceptable as the one storage dependency to freeze on for years. Its
  format is documented and stable, but name it explicitly as load-bearing.
- One `redb` file, or `redb` for refs and index with separate object files.
  This survey recommends one file.
- The object encoding (CBOR, bincode, other) and the exact prolly tree
  parameters are **#19 and ADR-0008's job**, not this ADR. Confirm that split.

## Sources

- [Git4Data (arXiv 2609.02106)](https://arxiv.org/abs/2609.02106)
- [Git's database internals: the packed object store (GitHub blog)](https://github.blog/open-source/git/gits-database-internals-i-packed-object-store/)
- [Jujutsu architecture](https://docs.jj-vcs.dev/latest/technical/architecture/) and [Jujutsu on LWN](https://lwn.net/Articles/958902/)
- [Efficient diff on prolly trees (DoltHub)](https://www.dolthub.com/blog/2020-06-16-efficient-diff-on-prolly-trees/) and [three-way merge in a SQL database (DoltHub)](https://www.dolthub.com/blog/2024-06-19-threeway-merge/) and [fast merge on prolly trees (DoltHub)](https://www.dolthub.com/blog/2025-07-16-announcing-fast-merge/)
- [Dolt storage engine docs](https://docs.dolthub.com/architecture/storage-engine/prolly-tree)
- [`prollytree` crate](https://crates.io/crates/prollytree) and [`prolly` (crabbuild)](https://github.com/crabbuild/prolly)
- [`redb`](https://github.com/cberner/redb)
- [`gitoxide`](https://github.com/GitoxideLabs/gitoxide)
