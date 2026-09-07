# ADR-0014: The conflict object and the resolution API

- Status: Accepted
- Date: 2026-09-07
- Issue: #44 (grilling), research #42

## Context

ADR-0013 fixed the merge algorithm: a structural three-way merge over the
`State` map that produces a merged map plus a set of conflicts. This ADR fixes
what a conflict *is*, how a caller resolves one, and how that surfaces in the
CLI and the SDK.

The merge is a stateless call (ADR-0013, Model A): `merge` returns
`Conflicts(Vec<Conflict>)` and writes nothing; the caller inspects, builds a
resolution map, and calls `merge` again.

## Decision

### The conflict, a transient value

A conflict is a plain value returned by `merge`, not a stored content-addressed
object:

```rust
pub struct Conflict {
    pub id: String,
    pub kind: ConflictKind,
    pub base: Option<ObjectId>,    // None for AddAdd
    pub ours: Option<ObjectId>,    // None for DeleteEdit
    pub theirs: Option<ObjectId>,  // None for EditDelete
}

pub enum ConflictKind {
    EditEdit,     // both changed the node to different objects
    DeleteEdit,   // ours deleted, theirs changed
    EditDelete,   // ours changed, theirs deleted
    AddAdd,       // both added the id with different objects
}
```

`base`, `ours` and `theirs` are the node object ids on each side, so the caller
can load and compare their content (`Store::node`, ADR-0012). The SDK exposes it
as a frozen `Conflict` dataclass whose `base` / `ours` / `theirs` are
`MemoryNode | None`.

A **stored** conflict object, content-addressed and referenced from a persisted
merge state, is what the Era 2 review model needs so a reviewer can approve or
reject a resolution before it lands in shared memory. That is deferred to Era 2.

### Resolutions

Per conflicting id, the caller supplies one:

```rust
pub enum Resolution {
    Ours,           // take `ours`
    Theirs,         // take `theirs`
    Base,           // take `base`, reverting both edits (invalid for AddAdd)
    Delete,         // the merged state omits the id
    Set(MemoryNode) // a fresh node as the resolution
}
```

`Set(node)` writes the node object to `objects` (the same path `stage` uses) and
uses its id. `Base` on an `AddAdd` conflict is an error, since there is no base
object.

A resolution map (`BTreeMap<String, Resolution>`) is passed to `merge`. An entry
for an id that is not a conflict is an error. Unresolved conflicts after the map
and any `strategy` are re-returned.

### The merge outcome across the boundary

The SDK returns a small result object from `store.merge(...)`:

```python
@dataclass(frozen=True)
class MergeResult:
    status: str                 # "up-to-date" | "fast-forwarded" | "merged" | "conflicts"
    commit: str | None          # the new commit id, for fast-forwarded / merged
    conflicts: list[Conflict]   # for "conflicts", else empty
```

Usage:

```python
result = store.merge("feature")
if result.status == "conflicts":
    resolutions = {c.id: pick(c) for c in result.conflicts}
    result = store.merge("feature", resolutions=resolutions)
```

`resolutions` values are the strings `"ours" | "theirs" | "base" | "delete"` or a
`MemoryNode` (for `set`). `strategy="ours" | "theirs"` is the whole-merge
shortcut.

### The CLI

```
mnem merge <branch-or-commit>
mnem merge <branch-or-commit> --resolve <id>=<ours|theirs|base|delete> ...
mnem merge <branch-or-commit> --strategy <ours|theirs>
```

- A clean merge or a fully resolved one prints `[<short id>] merge <theirs> into
  <branch>` and exits 0.
- A fast-forward prints `Fast-forwarded '<branch>' to <short id>`.
- Unresolved conflicts are printed, one block per id (the kind and the content
  of each side), and the command exits non-zero. The user re-runs with
  `--resolve` flags or `--strategy`.
- `--resolve` with a `set` (a supplied node) is not offered on the CLI in
  `v0.0.4`; use the SDK or resolve to `ours` / `theirs` / `base`.

`mnem status` is unchanged: with no pending-merge state, there is nothing
mid-merge to show.

## Consequences

- A conflict is cheap: no object written, no id to garbage-collect, no schema.
  The trade is that conflicts do not survive the process; a caller that wants to
  resolve later re-runs `merge`.
- The resolution set is small and closed. `Set(node)` covers "neither side is
  right"; the other four cover the common picks.
- The CLI conflict-resolution flow is flag-driven rather than an editor loop.
  Blunt, but agents drive this far more than humans, and `--strategy ours` is
  the usual agent choice.
- Era 2's review model will add a stored conflict object and a persisted merge
  state on top of this; the transient `Conflict` shape here is the starting
  point for that object's fields.

## Alternatives considered

**A stored, content-addressed conflict object now.** Rejected for `v0.0.4`: it
buys "walk away and resolve later" and an audit trail, neither of which a
stateless single-agent merge needs. It is exactly what Era 2 adds, and building
it now would be guessing at the review model's requirements.

**A richer resolution language** (take-ours-for-these-fields, three-way text
merge of content). Rejected: that is semantic merge, which is Era 2 behind a
trait. The substrate's resolutions are whole-node.

**An interactive `mnem mergetool`.** Rejected for now: flags cover the human
case, and the primary caller is an agent.
