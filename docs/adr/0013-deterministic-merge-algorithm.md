# ADR-0013: The deterministic merge algorithm

- Status: Accepted
- Date: 2026-09-07
- Issue: #43 (grilling), research #42

## Context

Phase 3 (`v0.0.4`) adds `merge`: combining two lines of memory. It is the last
purely mechanical piece of the substrate. The merge is **structural and
deterministic**: it works on the `State` map (node id to object id), one entry
at a time, and never looks inside a node's content. Semantic merge over claim
contents is Era 2 and lives behind a trait in a separate crate.

The research (`docs/research/issue-42-merge-survey.md`) surveyed the per-id
three-way merge table, the merge base, and the invariants. ADR-0012 named this
phase as a reassessment point for the prolly-tree `State` form. Issue #45 built
`Store::merge_base` / `merge_bases` / `ancestors` / `is_ancestor`.

This ADR fixes the algorithm. ADR-0014 fixes the conflict object and the
resolution API. Neither changes `format_version`: a merge commit is an ordinary
two-parent `Commit`, and the merged `State` is an ordinary flat `State`.

## Decision

### The merge base

`merge` requires **exactly one** merge base (`Store::merge_base`, #45). If two
commits have several lowest common ancestors (a criss-cross history) or none
(disjoint histories), `merge` refuses with a message naming them. Recursive
merge over a virtual base (Git's `ort` strategy) is deferred; the manual path is
to merge one side first. A single-agent substrate with a human in the loop
rarely produces a criss-cross.

### Fast-forward

If `theirs` is a descendant of the current branch tip, there is nothing to
merge. `merge` **fast-forwards**: it moves the branch ref to `theirs`, writes no
merge commit, and reports `FastForwarded(theirs)`. The CLI prints
`Fast-forwarded '<branch>' to <short id>`. There is no `--no-ff`; agents do not
want merge commits for trivial merges. If `theirs` is an ancestor of the tip (or
equal), `merge` reports "already up to date" and does nothing.

### The per-id three-way merge

Let `B`, `O`, `T` be the `State` maps of the base, ours (the current branch
tip), and theirs. For each node id present in any of them, with `b`, `o`, `t`
its object id in each (or absent):

| `b` | `o` | `t` | result |
| --- | --- | --- | --- |
| x | x | x | keep x |
| x | y | x | keep y (ours changed) |
| x | x | y | keep y (theirs changed) |
| x | y | y | keep y (convergent edit) |
| x | y | z | **conflict** `EditEdit` |
| x | absent | x | absent (ours deleted) |
| x | x | absent | absent (theirs deleted) |
| x | absent | absent | absent (convergent delete) |
| x | absent | z | **conflict** `DeleteEdit` |
| x | y | absent | **conflict** `EditDelete` |
| absent | y | absent | keep y (ours added) |
| absent | absent | z | keep z (theirs added) |
| absent | y | y | keep y (convergent add) |
| absent | y | z | **conflict** `AddAdd` |

The table is total: every combination is an automatic result or a named
conflict. `delete/edit` and `add/add` (with differing objects) are conflicts,
not silent resolutions, because auto-resolving loses a side's intent.

The merged map is built by walking the **union of ids in sorted order**, so a
clean merge produces a bit-identical `State` object regardless of which side is
"ours", and the conflict list is in node-id order.

### The merge call (Model A: stateless)

```
Store::merge(
    theirs: &str,                          // a commit-ish
    resolutions: &BTreeMap<String, Resolution>,
    strategy: Option<MergeStrategy>,        // Ours | Theirs
    message: Option<&str>,
    author: &str,
    time_ms: i64,
) -> Result<MergeOutcome>

enum MergeOutcome {
    AlreadyUpToDate,
    FastForwarded(ObjectId),
    Merged(ObjectId),                       // the new two-parent commit
    Conflicts(Vec<Conflict>),               // nothing was written
}
```

There is **no pending-merge state**, no `MERGE_HEAD`, no local table, no
`--continue`. `merge` is a function: run it, and if it returns `Conflicts`,
inspect them, build a resolution map, and call `merge` again with it.

- A `resolutions` entry for an id that is not a conflict is an error, so a stale
  map is caught rather than silently ignored.
- `strategy: Some(Ours)` resolves every conflict to `o`; `Some(Theirs)` to `t`.
  An explicit `resolutions` entry overrides the strategy for that id.
- If, after applying `resolutions` and any `strategy`, some conflicts remain
  unresolved, `merge` returns `Conflicts` with just those.

### The merge commit

On a clean merge or a fully resolved one, `merge` writes the merged `State`,
then a `Commit` with:

- `parents = [ours_tip, theirs_tip]` (ours first, so `log --first-parent`
  follows the branch we were on)
- `message` = the caller's, or `merge <theirs> into <branch>`
- `author`, `time` from the caller, as with `commit`
- a compare-and-swap of the branch ref against `ours_tip` (ADR-0009), so a
  concurrent commit makes the merge fail rather than clobber

### Preconditions

`merge` is refused, with a clear message, when:

- `HEAD` is detached (merge only from a branch, like `commit`)
- `staging` or the tombstone set is non-empty (a dirty index; commit or unstage
  first, no `--discard`)

### `State` stays flat; prolly reassessed

The merge is O(distinct node ids across `B`, `O`, `T`); the base walk is O(history
size). Both are trivial at an agent's scale (hundreds of ids, short histories).
The prolly-tree `State` form's payoff (diff and merge proportional to the change,
not the size) matters at thousands to millions of ids. **Flat `State` is kept for
Phase 3.** The trigger for prolly is unchanged: a real store hitting the storage
or latency ceiling, which then opens its own research ticket.

## Consequences

- `merge` is a pure-ish function with a small surface and no extra states to
  reason about. The SDK reads naturally; the CLI resolves conflicts with flags.
- No `format_version` change: a merge commit and a merged `State` are ordinary
  objects.
- A criss-cross history is a hard error, not a silent guess. Accepted: it is
  rare here, and the workaround (merge one side first) is clear.
- A merge that hits conflicts writes nothing, so there is never a
  half-merged store on disk. The cost is that the caller re-runs `merge` with
  the full resolution map rather than resolving incrementally.
- Persistent, reviewable conflicts (for the Era 2 review model) will need the
  pending-merge state this ADR omits. That is Era 2's to add.

## Alternatives considered

**A pending-merge state with `merge --continue` / `--abort`** (Git's model).
Rejected for `v0.0.4`: it needs a local table, a `MERGE_HEAD` equivalent, and
several mid-merge states, for a workflow (interactive incremental conflict
resolution) that a single agent does not do. It returns in Era 2 with the review
model, which genuinely needs conflicts to persist and be reviewed.

**Auto-resolving `delete/edit` and `add/add`** (the edit wins, the newer add
wins). Rejected: it silently discards one side's intent. A conflict makes the
caller choose.

**Always writing a merge commit, even for a fast-forward** (`--no-ff` by
default). Rejected: it clutters the history with trivial merges. `--no-ff` can be
added later if wanted.

**Recursive merge over a virtual base for criss-cross histories.** Deferred, not
rejected. It is correct and Git does it, but it is a body of work for a case
that a human-gated single agent rarely produces.
