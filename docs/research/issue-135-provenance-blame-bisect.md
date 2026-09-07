# Research: the provenance index, blame and bisect (issue #135)

Feeds **ADR-0015** (the provenance index) and the Phase 4 build tickets
#51–#54. Phase 4 (`v0.0.5`) is "it explains": given a memory node, say where it
came from (`blame`); given a wrong belief, find the commit it entered
(`bisect`). Still Era 1: deterministic, no network, no model calls. Nothing here
changes `format_version` — `blame` and `bisect` are read-only walks over
objects that already exist.

Four questions:

1. What does `blame` resolve, and how, given that provenance lives on the node?
2. Is a provenance **index** needed at Phase 4 scale, and if so what is in it,
   where does it live, and how is it kept current?
3. What does `bisect` search, and how is the range given?
4. What is the buggy-run fixture (#54), and does the reflog land here?

---

## 1. `blame`

### What it resolves

From `CONTEXT.md` and ADR-0003: `blame` resolves a memory node to **the commit
and the provenance that introduced its current value**. Provenance is on the
`MemoryNode` (`object.rs`), not the `Commit`, and may be entirely empty — so
`blame` always resolves to a *commit*, and additionally surfaces whatever
provenance the node carries (`agent_step` / `observation` / `tool_call` /
`source` / `note`) and its `event_time` (falling back to the commit's record
time when absent, per ADR-0003).

### The algorithm

`State` at a commit is `{ node_id -> ObjectId }` (`state_map_at`). A node id's
"value" at a commit is that `ObjectId` (or absent). The value *changed* at
commit `C` iff `state_map_at(C)[id] != state_map_at(parent)[id]` — this covers
add (absent → present), edit (id → different id) and, if we want it, delete
(present → absent).

**Blame walk.** Start at the target commit-ish `C` (default `HEAD`). Let
`target = state_map_at(C)[id]`; error if the id is absent at `C` ("nothing to
blame: <id> is not in memory at <C>"). Walk the first-parent chain from `C`
towards the root. For each commit `Ci` with first parent `Cp`:

- if `state_map_at(Ci)[id]` differs from `state_map_at(Cp)[id]`, then `Ci` is
  the introducing commit for the value as of `C`. Return it.
- the root commit (no parent) always counts as a change (absent → present).

Return `{ commit: Ci, node: <the MemoryNode at Ci>, provenance, event_time,
time: Ci.time }`.

Cost: O(history length) point reads, each cheap. For an agent's memory
(hundreds of ids, short histories) this is microseconds. No index required for
`blame` to be correct or fast — see §2.

### The merge subtlety

If the current value of `id` arrived on the **theirs** side of a merge, the
first-parent walk sees the *merge commit* as the point of change (the first
parent, "ours", did not have that value). Git's `blame` follows the parent that
actually contributed the line. Options for Era 1:

- **(a) Report the merge commit**, with a note that the value came in via a
  merge and pointing at the other parent. Simple, honest, one walk.
- **(b) Recurse**: at a merge commit where the value differs from `parents[0]`
  but equals `parents[k]`, continue the walk from `parents[k]`. This gives the
  true origin commit but is a multi-parent walk.

Recommendation: **(b)**, but only the minimal form — pick the first parent whose
`state_map_at`value equals the value at the merge, and continue from there. It
is a few extra lines and gives the answer a user actually wants ("who first
wrote this"), not "a merge happened". A single agent with a person in the loop
rarely has deep merge nesting, so the walk stays short.

### `blame` on a deleted node

If `id` is absent at `C`, `blame <id>` errors. A separate question is "when was
`id` deleted" — that is really a `bisect` predicate (`|s| !s.contains_key(id)`)
or a future `mnem log --follow <id>`. Keep `blame` to "explain a value that
exists". Do not overload it.

---

## 2. The provenance index

### Is one needed?

At Phase 4 scale, **no** — for the two operations this phase ships:

| Operation | Without an index | With an index |
| --- | --- | --- |
| `blame <id>` | O(history) first-parent walk, ~µs | O(1) lookup of the introducing commit |
| `bisect` | O(log n) `state_at` reads | unchanged — bisect is about content predicates, not provenance |

`bisect` gets nothing from a provenance index (ADR-0003 already says "`bisect`
operates on node content and presence, not provenance"). `blame` is already
fast. So an index bought now is **speculative infrastructure** — exactly the
kind of thing the project has twice deferred for the prolly tree (ADR-0012,
ADR-0013) with a named "reassess when a real store hits a ceiling" trigger.

### What an index *would* hold, when it is built

The useful shape, when a real store makes the walk too slow, or when the Era 2
review/audit UI needs reverse lookups:

- **`node_id -> introducing commit`** for the current tip (answers `blame`
  in O(1)). Must be recomputed or patched on every commit and on every
  branch/checkout, because "current" is per-branch.
- **`commit -> [node_ids it introduced or changed]`** (the reverse: "what did
  this commit teach the agent"). Append-only, never rewritten, since a commit is
  immutable. This one is cheap and monotonic.

The reverse map is the safer first index: it is append-only, derived purely from
`diff(parent, commit)` at commit time, and never invalidated. The forward map is
the one that needs care (per-branch, mutated on checkout).

### Where it would live

A new `redb` table in the same database (`objects` / `refs` / `staging` live
there already). Not a separate file — one database keeps the write-transaction
story simple (a commit already opens one write txn; the index write joins it).
Key/value: `commit_id -> CBOR([node_id])` for the reverse map.

### Incremental vs rebuild

- **Reverse map**: built incrementally at commit time inside the existing write
  transaction (`diff` the new commit against `parents[0]`, write the id list).
  A full rebuild is a one-pass walk of all commits, used once on upgrade or if
  the table is missing.
- **Forward map**: not proposed for Phase 4.

### Recommendation

**Defer the index.** Phase 4 ships `blame` and `bisect` as plain graph walks.
Add ADR-0015 as a *short* ADR that:

1. records that `blame`/`bisect` walk the graph directly and why that is fine at
   this scale (the numbers above);
2. specifies the reverse map (`commit -> introduced node ids`) as the index that
   *will* be built, its on-disk home, and its trigger — the same pattern as the
   prolly-tree deferral: a real store where `blame` latency or an Era 2 UI needs
   it;
3. keeps #51 in the phase, but reframed: **#51 builds the reverse map** (cheap,
   append-only, immediately useful for `mnem show --stat`-style output and a
   foundation for Era 2), and `blame` reads it opportunistically with a walk
   fallback. Or #51 becomes "defer the index, expand the blame/bisect tests".

The grilling (#50) picks between "#51 builds the reverse map now" and "#51 is a
deferral ADR + more tests". Both are defensible; the reverse map is the more
productive use of the ticket and carries no invalidation risk.

---

## 3. `bisect`

### What it searches

Binary search a commit range for the **first commit where a supplied predicate
holds** (`CONTEXT.md`). The predicate is a function of the reconstructed memory
at that commit:

```rust
fn bisect(
    &self,
    good: &str,          // a commit-ish where the predicate is false
    bad: &str,           // a commit-ish where the predicate is true (default HEAD)
    predicate: impl Fn(&BTreeMap<String, MemoryNode>) -> bool,
) -> Result<ObjectId>
```

The predicate takes the full `state_at` map so it can express "node `plan` says
`enterprise`" or "node `x` is absent" or "any node mentions 'refund'". The
common case — "this one node has this wrong value" — is a helper that builds the
closure.

### The range and the monotonicity assumption

Like `git bisect`, `bisect` assumes the predicate is **monotonic** over the
range: false up to some commit, true from there on. `good` must be an ancestor
of `bad`, and the predicate must be false at `good` and true at `bad` — all
three checked up front, with a clear error otherwise ("the predicate is already
true at <good>", "not an ancestor").

### Linear history only, for now

Walk the **first-parent chain** from `bad` back to `good`, giving an ordered
list of N commits, and binary-search that list: O(log N) `state_at`
evaluations. Merge commits on the chain are evaluated like any other (their
`state` is fully materialised). Second parents are not descended into — a
single-agent history is essentially linear, and `git bisect`'s merge handling
(skip, multiple bad regions) is complexity Era 1 does not need. Document the
limitation.

### Result

Return the first `bad` commit (the boundary), plus optionally its `blame` for
whichever node the predicate helper targeted, so `mnem bisect` can print "the
belief entered at <commit>, written by <provenance>" in one shot — this is the
phase's headline demo.

---

## 4. The buggy-run fixture (#54) and the reflog

### The fixture

A synthetic agent run, built in a test (seeded, deterministic), where:

- commits 1..k build up plausible memory (a support agent learning about an
  account: plan tier, contacts, open tickets);
- at a **known commit `k`**, a wrong belief enters — e.g. `plan` flips from
  `enterprise` to `pro` off a misread observation, with provenance pointing at
  that observation;
- commits k+1..n continue, some of them touching other nodes, one or two even
  reading the wrong `plan` (so the error "spreads");
- the test then: (a) `bisect` with the predicate `plan == "pro"` and asserts the
  boundary is exactly commit `k`; (b) `blame plan` at `HEAD` and asserts it
  resolves to commit `k` and the planted provenance.

This is the Phase 4 definition of done and the demo GIF material.

### The reflog

ADR-0009 explicitly parked the reflog: "It lands as its own ticket in Phase 2,
or alongside `blame` and `bisect` in Phase 4." It is a small append-only
`(ref, old, new, time, op)` table written on every ref move. It is *not* needed
for `blame` or `bisect` (those walk the commit graph, not ref history), and
Phase 4 has no child ticket for it.

Recommendation: **defer again, deliberately.** Note in ADR-0015 (or a one-line
ADR-0009 amendment) that the reflog moves to Phase 5/6 or its own ticket, so the
deferral is on the record rather than forgotten. Folding it in now widens Phase
4 for no gain to the "it explains" story.

---

## Recommendations, in one place

- **`blame`**: first-parent walk from a commit-ish (default `HEAD`) for the
  commit that changed the target node id's value; follow the contributing parent
  through merges; return commit + node + provenance + effective time. Errors if
  the id is absent at the target commit.
- **The index**: defer the forward (`node -> introducing commit`) map. ADR-0015
  is short: it records the direct-walk approach and its scale justification, and
  specifies the **reverse map** (`commit -> introduced node ids`, append-only,
  in a new `redb` table, built in the commit write txn) as either what #51
  builds now or what is deferred with a trigger.
- **`bisect`**: binary search the first-parent chain between `good` (predicate
  false, ancestor) and `bad` (predicate true, default `HEAD`); predicate is
  `Fn(&state_at map) -> bool`; assumes monotonic; linear history only;
  O(log N) `state_at` reads. Helper for the "one node, one value" case.
- **Fixture (#54)**: seeded synthetic run, wrong belief at a known commit,
  tests that `bisect` finds it and `blame` explains it.
- **Reflog**: deferred again, on the record.

## Open questions for the grilling (#50)

- **The index.** Build the reverse map in #51 now, or make ADR-0015 a deferral +
  test-expansion ADR? If built: reverse map only, or also the forward map?
- **`blame` through merges.** Report the merge commit (simple) or recurse to the
  true origin (a few more lines, better answer)?
- **`blame` output.** Just the introducing commit, or also the chain of every
  commit that touched the node (a `mnem log --follow <id>` in disguise)?
- **`bisect` predicate.** A Rust closure only (SDK/tests), or also a CLI form —
  and if CLI, what language (`node == value`, a jq-ish path, an external command
  like `git bisect run`)?
- **`bisect` range.** Require `good` explicitly, or default it to the root
  commit? Error messages when the predicate is non-monotonic (git just gives a
  possibly-wrong answer; we could detect some cases).
- **Deleted / re-added nodes.** Does `blame` care about history before the most
  recent add, or only since the current value's introduction?
- **Reflog.** Confirm deferral, or fold the small table in now while ref-move
  code is being touched anyway.

## Sources

- [git-blame(1)](https://git-scm.com/docs/git-blame) — per-line last-change, `-C`/`-M`, following through merges
- [git-bisect(1)](https://git-scm.com/docs/git-bisect) and [`git bisect run`](https://git-scm.com/docs/git-bisect#_bisect_run) — good/bad boundary, monotonicity, `run` with a predicate command
- [How `git bisect` works (Julia Evans)](https://jvns.ca/blog/2015/06/12/git-bisect/) — the binary-search framing
- ADR-0003 (the memory node model) — provenance fields, `event_time`, "`bisect` operates on content and presence"
- ADR-0009 (the ref model) — the reflog deferral
- DoltHub [`dolt blame`](https://docs.dolthub.com/sql-reference/version-control/dolt-system-tables#dolt_blame_usdtablename) — row-level blame over a table, the closest prior art to node-level blame
