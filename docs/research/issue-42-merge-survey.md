# Research: merge algorithms, conflicts and invariants (issue #42)

Feeds ADR-0013 (the deterministic merge algorithm) and ADR-0014 (the conflict
object and resolution API). Phase 3 (`v0.0.4`) is the last purely mechanical
piece: a three-way merge over the flat `State` map, conflict objects, a
resolution API, and a property and chaos harness. Still Era 1: structural and
deterministic, no semantics. Semantic merge over claim contents is Era 2.

Four questions:

1. What does a three-way merge of two `State` maps do, per node id?
2. How is the merge base found, and what happens on a criss-cross history?
3. How are conflicts represented, and what does the resolution API look like?
4. What invariants must the chaos harness check, and is flat `State` still
   enough?

## 1. The per-id merge rules

`State` is `{ nodes: BTreeMap<String, ObjectId> }`. A merge takes a **base** `B`
(the merge base's state), **ours** `O`, and **theirs** `T`, and produces a
merged map plus a set of conflicts. This is a tree merge, one entry per node id,
not a line merge; there is no content-level merge of a node (that is Era 2).

For each node id present in any of `B`, `O`, `T`, let `b`, `o`, `t` be the
`ObjectId` in each (or absent):

| `b` | `o` | `t` | result |
| --- | --- | --- | --- |
| x | x | x | keep x (unchanged) |
| x | y | x | keep y (ours changed, theirs did not) |
| x | x | y | keep y (theirs changed, ours did not) |
| x | y | y | keep y (both changed the same way, convergent) |
| x | y | z | **conflict**: edit/edit |
| x | absent | x | absent (ours deleted, theirs unchanged) |
| x | x | absent | absent (theirs deleted, ours unchanged) |
| x | absent | absent | absent (both deleted, convergent) |
| x | absent | z | **conflict**: delete/edit |
| x | y | absent | **conflict**: edit/delete |
| absent | y | absent | keep y (ours added) |
| absent | absent | z | keep z (theirs added) |
| absent | y | y | keep y (both added the same, convergent) |
| absent | y | z | **conflict**: add/add |

This is the classic filesystem three-way merge (Git's per-path logic, minus
renames, which do not exist here: a node id is stable, ADR-0003). The table is
total: every combination is either an automatic result or a named conflict.

**Determinism.** The result map is built by iterating the union of keys in
sorted order (a `BTreeSet`), so a clean merge is bit-identical regardless of
argument order, and the conflict list is in node-id order.

## 2. The merge base

The merge base of commits `O` and `T` is a **lowest common ancestor** in the
commit DAG: an ancestor of both that has no descendant which is also an ancestor
of both. Issue #45 computes it.

- The common case has one LCA. Walk both ancestor sets (BFS over `parents`),
  intersect, then take the elements with no other intersection element among
  their descendants. `State` is small and histories are short, so an O(nodes in
  history) walk is fine; no generation numbers or bloom filters needed yet.
- A **criss-cross** history (two branches merged into each other in the past)
  can have more than one LCA and no unique base. Git's `ort` strategy merges the
  LCAs into a **virtual base** and three-way-merges against that. That is
  correct but is a recursive merge and its own body of work.

**Recommendation for ADR-0013.** Phase 3 requires a **single** merge base. When
there are multiple LCAs, `merge` refuses with a clear error naming them, rather
than silently picking one or doing a recursive merge. Recursive merge over a
virtual base is deferred; the trigger is a real criss-cross showing up, which a
single-agent substrate with a human in the loop rarely produces. If it does, the
manual path is to merge one side first.

- The first commit on each branch shares the root; two branches off the same
  commit have that commit as the base. A branch never merged has exactly one
  base with any other branch descended from a shared point.
- If `O` is an ancestor of `T` (or vice versa), the merge is a fast-forward: the
  result is `T`, no merge commit. `merge` should detect this.

## 3. Conflicts and resolution

### The conflict object

When a merge has conflicts, it does **not** write a merge commit. Instead it
returns the conflict set to the caller, and (option A) leaves the store
untouched, or (option B) records a pending merge state that `mnem status` shows
and `mnem merge --continue` finishes.

A **conflict** names one node id and the three sides:

```
Conflict {
    id: String,
    kind: EditEdit | DeleteEdit | EditDelete | AddAdd,
    base:   Option<ObjectId>,   // None for AddAdd
    ours:   Option<ObjectId>,   // None for DeleteEdit
    theirs: Option<ObjectId>,   // None for EditDelete
}
```

Whether the conflict is a **first-class stored object** (like a memory node,
content-addressed, referenced from a `MergeState`) or a **transient value**
returned by the merge call is the ADR-0014 question. A stored conflict object
supports "walk away and come back", a `mnem status` that survives a restart, and
an audit trail; a transient value is simpler and matches "merge is one call".
For a single agent, the transient value is likely enough for `v0.0.4`, with the
stored form arriving when the review model (Era 2) needs conflicts to persist
and be reviewed.

### The resolution API

Per conflicting id, the caller picks one of:

- **ours** — take `O`'s object id
- **theirs** — take `T`'s object id
- **base** — take `B`'s object id (revert both edits)
- **delete** — the merged state omits the id
- **set(node)** — stage a new node object as the resolution

Once every conflict has a resolution, `merge` (or `merge --continue`) writes the
merged `State` and a two-parent `Commit`. The resolution API is:

```
store.merge(theirs: CommitIsh) -> MergeOutcome            // clean, ff, or conflicts
store.merge_resolve(id, Resolution) -> ()                 // record one resolution
store.merge_continue(message, author, time) -> ObjectId   // finish
store.merge_abort() -> ()                                 // drop the pending merge
```

CLI: `mnem merge <branch>`, `mnem merge --continue`, `mnem merge --abort`, and
`mnem status` shows the unresolved ids. SDK: `store.merge("other")` returns a
result object; a context-manager form is possible but not required.

### Interaction with `staging`

`merge` requires a clean index (no staged adds or tombstones), the same rule as
`checkout` (ADR-0012). The pending-merge resolutions live in their own local
table, not `staging`.

## 4. Invariants and the chaos harness (#49)

The harness generates random histories (two branches off a base, each with a
random sequence of adds, updates and deletes) and checks:

- **Clean-merge symmetry.** When `merge(O, T)` is clean, `merge(T, O)` is clean
  and produces the same `State` object id. Conflict *sets* are equal; the merge
  *commit* differs only in parent order and message.
- **Idempotence.** `merge(O, O)` is a fast-forward to `O`; merging an ancestor
  is a no-op.
- **Base identity.** `merge(O, B)` where `B` is the base is a fast-forward or a
  no-op (`O` already contains the base).
- **Convergence.** After resolving conflicts, `merge(O, T)` then `merge(T, O')`
  (where `O'` is the first merge result) converges: no new conflicts, same
  final state.
- **The table is total.** A generated case never produces an unclassified
  outcome: every id is in the merged map or in the conflict set, never both,
  never neither.
- **No lost writes.** Every node id that both sides agree on (or only one side
  touched) appears in the result with the agreed object id.

This is the spirit of the `ephor` exactly-once tests: hammer the operation with
random interleavings and assert the algebra holds. Deterministic seeds, like
`tests/time_travel.rs` (#41).

### Is flat `State` still enough?

ADR-0012 named Phase 3 merge as a **reassess** point for the prolly tree, not an
automatic trigger. The merge algorithm above is O(total distinct node ids across
`B`, `O`, `T`), which for an agent's memory (hundreds of ids) is trivial. The
merge base walk is O(history size). Neither needs structural sharing to be
correct or fast at this scale.

**Recommendation.** Keep flat `State` for Phase 3. The prolly tree's payoff is
diff and merge *proportional to the change* rather than to the size, which
matters at thousands to millions of ids, not hundreds. The trigger stays: a real
store hitting the storage or latency ceiling. Record this reassessment in
ADR-0013.

## Recommendations, in one place

- **ADR-0013 (algorithm):** the per-id table above; union-of-keys in sorted
  order for determinism; a single required merge base, refuse on multiple LCAs;
  fast-forward detection; a clean index required; flat `State` kept, prolly
  deferred again.
- **ADR-0014 (conflict object + API):** `Conflict { id, kind, base, ours,
  theirs }`; transient (returned) rather than stored for `v0.0.4`, stored form
  deferred to the Era 2 review model; resolutions are `ours | theirs | base |
  delete | set(node)`; `merge` / `merge_resolve` / `merge_continue` /
  `merge_abort`; pending resolutions in a local table; `mnem merge` +
  `--continue` / `--abort`, `mnem status` shows unresolved ids.

## Open questions for the grillings (#43, #44)

- **#43:** single-base-or-refuse versus recursive virtual base. Fast-forward:
  silent, or announced? Does `merge` ever touch `HEAD` other than the new merge
  commit? Confirm the per-id table, especially delete/edit and add-add as
  conflicts rather than "take the non-deleting side".
- **#44:** transient versus stored conflict object. Where do pending resolutions
  live. Is there a `mnem merge --continue` flow, or must a merge be resolved in
  one process. What `mnem status` shows mid-merge. The `set(node)` resolution:
  does it go through `add` (staging) or a dedicated path.

## Sources

- [git-merge(1)](https://www.kernel.org/pub/software/scm/git/docs/git-merge.html) and [git merge-strategies](https://git-scm.com/docs/merge-strategies) (modify/delete, add/add, stages 1/2/3, criss-cross)
- [git-merge-tree](https://git-scm.com/docs/git-merge-tree) (a merge can conflict without any single entry conflicting)
- [Criss-Cross Merge (revctrl.org)](https://tonyg.github.io/revctrl.org/CrissCrossMerge.html) and the `ort` virtual-merge-base approach
- [Three-Way Merge in a SQL Database (DoltHub)](https://www.dolthub.com/blog/2024-06-19-threeway-merge/) (per-row three-way merge, prolly-tree diff proportional to the change)
- [Evaluation of Version Control Merge Tools (arXiv 2410.09934)](https://arxiv.org/pdf/2410.09934) and [Analyzing Git Diff and Merge (arXiv 2507.22071)](https://arxiv.org/pdf/2507.22071)
