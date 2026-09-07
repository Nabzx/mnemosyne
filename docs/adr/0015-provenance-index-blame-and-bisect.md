# ADR-0015: The provenance index, blame and bisect

- Status: Accepted
- Date: 2026-09-07
- Issue: #50 (grilling), research #135

## Context

Phase 4 (`v0.0.5`) is "it explains". Two operations:

- **`blame <node-id>`** — resolve a memory node to the commit, and the
  provenance, that introduced its current value.
- **`bisect`** — binary-search a commit range for the first commit where a
  supplied predicate holds ("this wrong belief is present").

Both are read-only walks over objects that already exist. **Nothing here changes
`format_version`.** Provenance lives on the `MemoryNode` (ADR-0003), not the
`Commit`, and may be entirely empty; `blame` therefore always resolves to a
*commit* and additionally surfaces whatever provenance the node carries.

The research (`docs/research/issue-135-provenance-blame-bisect.md`) found that at
this scale — hundreds of node ids, short histories — `blame` is a fast
first-parent walk and `bisect` is O(log N) state reads, so neither *needs* an
index. This ADR fixes the two operations and the one small index that is worth
building anyway.

## Decision

### The commit-nodes index (the reverse map)

A new `redb` table, `commit_nodes`, maps a commit id to the node ids it changed
relative to its first parent, each tagged with how it changed:

```
commit_nodes:  commit_id (32 bytes)  ->  CBOR(BTreeMap<String, ChangeKind>)

enum ChangeKind { Added, Modified, Removed }
```

- **Source.** `diff(parents[0], commit)` — the same `Added` / `Removed` /
  `Modified` classification `Store::diff` already produces (ADR-0012). The first
  commit's entry is every node id in its state, all `Added`.
- **Merge commits.** Diffed against `parents[0]` only, consistent with the
  first-parent model `log` and `blame` use. A merge commit legitimately "brought
  in" whatever the merged-in side added relative to ours.
- **Write path.** Written inside the **existing commit write-transaction**
  (`Store::merge` and `Store::commit` each already open one), so the index entry
  and the commit are always consistent. A merge commit's entry is written in the
  same txn that writes the merge commit.
- **Rebuild.** `Store::rebuild_index()` does a one-pass walk of every commit and
  rewrites the table. Used on a format upgrade, if the table is missing, or in
  tests. There is no automatic rebuild on `open`: a fresh `0.0.5` store builds
  the index forward from its first commit, and there are no older stores in the
  wild. A missing entry is never *wrong* — every reader falls back to a direct
  walk — so a partially built index is safe.
- **Exposure.** `Store::changed_by(commit) -> BTreeMap<String, ChangeKind>` in
  the core and SDK; `mnem show <commit> --stat` prints one classified line per
  changed node (`+ plan`, `~ owner`, `- status`).

The **forward map** (`node_id -> introducing commit` for the current tip) is
**deferred**. It would give `blame` an O(1) answer it does not need yet, and it
is the awkward one to maintain: "current" is per-branch, so it must be patched
on every commit *and* every checkout. It lands when a real store makes the
`blame` walk too slow, or when the Era 2 review UI needs the reverse lookup at
interactive latency — the same "reassess on a real ceiling" trigger ADR-0012 and
ADR-0013 use for the prolly tree.

### `blame`

```rust
pub struct Blame {
    pub commit: ObjectId,        // the commit that introduced the current value
    pub node: MemoryNode,        // the node as written at that commit
    pub time: i64,               // node.event_time, or the commit's record time
}

impl Store {
    pub fn blame(&self, node_id: &str, at: &str) -> Result<Blame>;
}
```

- `at` is a commit-ish (branch name or hex prefix, `resolve_commitish`),
  defaulting to `HEAD`. A detached `HEAD` or an unborn branch is a clean error.
- Let `target = state_map_at(at)[node_id]`; if the id is absent, error:
  `blame: "<node_id>" is not in memory at <at>`. "When was it deleted" is a
  `bisect` predicate or a future `mnem log --follow`, not `blame`.
- Walk the first-parent chain from `at`. At each commit `Ci` with first parent
  `Cp`: if `state_map_at(Ci)[node_id] != state_map_at(Cp)[node_id]`, then `Ci`
  changed the value. The root commit (no parent) always counts.
- **Through a merge.** If `Ci` is a merge and its value for `node_id` differs
  from `parents[0]` but equals some `parents[k]`, continue the walk from
  `parents[k]` instead of stopping. This follows the value to the commit that
  actually wrote it, rather than reporting "a merge happened".
- **Index use.** `blame` may consult `commit_nodes` as a skip filter — if a
  commit's change set does not contain `node_id`, its state need not be loaded.
  The index is an accelerator, never the source of truth.
- `provenance` is read off `Blame.node.provenance`; an empty provenance is
  legal and means `blame` resolved only to the commit.

### `bisect`

```rust
impl Store {
    pub fn bisect(
        &self,
        bad: &str,                                             // predicate true; default HEAD
        good: Option<&str>,                                    // predicate false; default the root
        predicate: impl Fn(&BTreeMap<String, MemoryNode>) -> bool,
    ) -> Result<ObjectId>;                                     // the first `bad` commit
}
```

- `bad` and `good` are commit-ishes. `good` defaults to the root commit reached
  by the first-parent walk from `bad`.
- **Up-front checks:** `good` is an ancestor of `bad` (`is_ancestor`); the
  predicate is `false` at `good` and `true` at `bad`. Otherwise a clear error
  (`the predicate is already true at <good>`, `<good> is not an ancestor of
  <bad>`).
- **Monotonicity is assumed**, as in `git bisect`: `false` up to a boundary,
  `true` from there on. A non-monotonic predicate gives a boundary that is one
  of possibly several transitions; this is documented, not detected.
- **Linear history only.** Walk the first-parent chain from `bad` back to
  `good`, giving an ordered list, and binary-search it: O(log N) `state_at`
  evaluations. Merge commits on the chain are evaluated normally (their state is
  fully materialised); second parents are not descended into. `git bisect`'s
  skip / multiple-bad-region handling is not built.
- Returns the first commit on the chain (oldest-to-newest) for which the
  predicate holds — the boundary.

### The CLI and the SDK

```
mnem blame <node-id> [<commit>]
mnem bisect --node <id> --equals <json-value>   [--good <commit>] [<bad-commit>]
mnem bisect --node <id> --absent                [--good <commit>] [<bad-commit>]
mnem bisect --node <id> --present               [--good <commit>] [<bad-commit>]
```

- `mnem blame plan` prints the short commit id, the effective time, the author,
  and the provenance fields that are set, then the node content.
- `mnem bisect` builds the predicate from `--node` plus one of `--equals`
  (content equals this JSON value), `--absent`, `--present`. On success it
  prints the boundary commit **and its `blame` for `--node`** — "the belief
  entered at `<short>`, written at `<step>` from `<observation>`" — so finding
  and explaining a bad belief is one command. A general predicate command
  (`git bisect run` style) is a later ticket.
- SDK: `store.blame(node_id, at="HEAD") -> Blame`;
  `store.bisect(bad="HEAD", good=None, predicate=...) -> str` taking a Python
  callable over a `{id: MemoryNode}` dict; `store.changed_by(commit) -> dict`.
  `Blame` is a frozen dataclass (`commit`, `node`, `time`).

### The reflog

ADR-0009 parked the reflog — a small append-only `(ref, old, new, time, op)`
table — "for Phase 2, or alongside `blame` and `bisect` in Phase 4". It is
**deferred again**: `blame` and `bisect` walk the commit graph, not ref history,
so neither needs it, and folding it in widens the phase for no gain to the "it
explains" story. It moves to its own ticket, to be picked up in Phase 5 or 6 or
when GC design starts (the two interact).

## Consequences

- `commit_nodes` is cheap and monotonic: one derived write per commit, in a txn
  that is already open, never invalidated because commits are immutable. It is a
  real piece of substrate — the basis for `--stat` output now and the Era 2
  audit view later — not speculative infra.
- `blame` and `bisect` are correct with an empty or absent index; the index only
  makes `blame` skip work. A store can always `rebuild_index()`.
- `blame` follows values through merges, so its answer is "who wrote this",
  which is what a user debugging a belief wants.
- `bisect` inherits `git bisect`'s monotonicity assumption and its linear-history
  simplification. Documented; revisited only if a real multi-agent history needs
  more.
- `blame` quality still depends on the agent populating provenance (ADR-0003).
  An agent that passes nothing gets `blame` that resolves to the commit and its
  time, and no more.

## Alternatives considered

**Build the forward map (`node -> introducing commit`) now.** Rejected: it is
speculative at this scale, and it is the map that needs per-checkout
maintenance. The reverse map gives most of the value (reverse lookups, `--stat`)
with none of the invalidation risk.

**No index at all this phase; #51 becomes test expansion.** Rejected: the
reverse map is a dozen lines in a path that is already open, and it is genuinely
useful now and structurally needed for Era 2. Deferring it would be deferring
work that has no reason to wait.

**A predicate DSL or `mnem bisect run <command>` now.** Deferred: the
`--node/--equals/--absent` helper covers the buggy-belief case, and a general
predicate language (jq paths, external commands) is its own design. The core
closure is fully general for the SDK and tests.

**`blame` reports the merge commit without recursing.** Rejected: "a merge
happened here" is not an explanation. Following the contributing parent is a few
lines and gives the origin.

**Fold the reflog in while ref-move code is being touched.** Rejected: it is not
needed by anything in Phase 4, and it interacts with the unstarted GC design.
Recorded as deferred so it is not lost.
