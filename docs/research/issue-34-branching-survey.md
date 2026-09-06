# Research: cheap branching and time travel (issue #34)

Feeds ADR-0012 (the branch and checkout model). Phase 2 delivers `branch`,
`checkout`, reading working memory at any past commit, and a structural diff,
all still Era 1: single-agent, local, deterministic.

Three questions:

1. How do Git and Jujutsu keep branch creation constant time, and does
   Mnemosyne already have that?
2. What does `checkout` have to touch, and what is "working memory" here?
3. Is the `flat` state form (ADR-0002) enough for time travel and diff, or is
   the prolly-tree form needed now?

## 1. Branching is already cheap

### How Git and Jujutsu do it

A **Git branch** is a 40-byte file under `.git/refs/heads/` (or one line in
`packed-refs`) holding a single commit id. Creating a branch writes those bytes
and touches nothing else, so it is O(1) regardless of repository or working-tree
size. `HEAD` names the current branch; a commit moves whatever branch `HEAD`
points at.

**Jujutsu** calls them bookmarks and they are also just named pointers to
revisions. Two differences from Git: there is no "current" bookmark, so a new
commit does not move any bookmark on its own; and a bookmark automatically
follows its target if that commit is later rewritten. Branch creation is the
same constant-time pointer write.

### Mnemosyne already has the pointer model

ADR-0009 built the `refs` table: `name -> [u8; 32]`, one row per branch, with
`list` / `get` / `set` / `compare_and_set` / `delete` and name validation.
`Store::commit` already creates the branch row on the first commit and advances
it with a compare-and-swap after. `HEAD` is the one-line text file, attached
(`ref: <branch>`) or detached (a bare commit id), and a commit from a detached
`HEAD` is already refused.

So "make branching cheap" is not open work. What is missing is only the
surface:

- `Store::branch(name)` — insert a `refs` row pointing at where `HEAD` resolves.
- `Store::branches()` — `refs::list`, already there.
- `Store::delete_branch(name)` — `refs::delete`, plus a guard so the branch
  `HEAD` is on cannot be deleted.
- `Store::checkout(target)` — see below.

Recommendation: branches stay pointer-only. No copy-on-write of state at branch
time, because nothing is copied: a branch is a 32-byte value in one row.

## 2. What `checkout` touches, and what "working memory" is

### The gap today

`CONTEXT.md` defines **working memory** as "the current, mutable memory state
the agent reads and writes, materialised from a commit ... the equivalent of a
working tree". Right now there is no such thing. There is `staging` (the index
equivalent): `mnem add` writes a node straight into the `staging` table, and
`commit` overlays `staging` on the parent commit's state. There is no step that
takes a commit and presents its full node set as something to read.

### Three ways to model working memory

**(a) A materialised `working` table.** `checkout` walks the target commit's
state and writes every `(node_id -> object_id)` into a `working` table. Reads
hit that table. This is closest to Git's working tree. Cost: O(nodes) writes on
every checkout, and a third place where the node set lives.

**(b) Working memory is a view, never stored.** "Working memory at `HEAD`" is
defined as *the `HEAD` commit's state, with `staging` overlaid*. `checkout` only
moves `HEAD`. Reading working memory resolves `HEAD`, loads its state, applies
`staging` on top. Cost: O(nodes) to read, which is inherent, and zero to check
out. No new table.

**(c) `staging` holds the full state after checkout.** `checkout` clears
`staging` and refills it with the target state, so `staging` stops being a diff
and becomes the whole working set. This collapses two concepts into one but
breaks the "staging is what changed" meaning that `commit` and a future `mnem
status` rely on.

### Recommendation: (b), a view

`checkout <branch-or-commit>` is one atomic `HEAD` write and nothing else. It is
refused if `staging` is non-empty, the same way Git refuses a checkout that
would discard uncommitted changes; a `--force` / `discard=True` escape hatch can
come later. Checking out a commit id rather than a branch gives a detached
`HEAD`, which already works.

**Reading working memory** is then a pure function: `resolve(HEAD).state`
overlaid with `staging::list`. Add `Store::working_memory() -> Vec<(String,
MemoryNode)>` and `Store::working_node(id)` for the single-node case.

**Time travel** is the same walk without touching `HEAD`: `Store::state_at(commit)
-> Vec<(String, MemoryNode)>` loads any commit's state and its nodes. This is
what issue #38 asks for, and it is read-only, so it needs no locking and no
branch. `mnem show <commit>` and the SDK expose it.

### What `checkout` must guard

- Empty `staging`, or refuse (with a clear message naming the staged ids).
- A branch name must exist in `refs`; a commit id must exist in `objects` and be
  a `Commit`.
- The `HEAD` write is atomic (temp file plus rename), already the case.
- A reflog entry would belong here. ADR-0009 reserved the reflog and does not
  write it; Phase 2 keeps it reserved.

## 3. Flat state is enough for Phase 2

### The flat form and its ceiling

`State` today is `{ nodes: BTreeMap<String, ObjectId> }` and every commit stores
a full `State` object listing a pointer for *every* visible node. A run that
makes 500 commits over 500 nodes stores 500 `State` objects of ~500 entries
each. At ~40 bytes per entry that is ~10 MB of state objects for a store whose
actual memory content might be a few hundred KB. Storage grows as commits ×
nodes, not as changes.

For Phase 2's deliverables this does not matter:

- **Time travel** is a point read of one `State` plus its nodes: O(nodes),
  which is the true cost of "give me the whole memory at commit C" under any
  design.
- **Structural diff** between commit A and B walks both sorted `State` maps once
  in lockstep and classifies each id as added / removed / modified (different
  `ObjectId`) / unchanged: O(nodes). `BTreeMap` makes this a merge-join.
- Agent runs are hundreds, not millions, of nodes and commits. 10 MB is fine.

### Prolly trees, and when they earn their place

A **prolly tree** (probabilistic B-tree; the storage engine behind Noms and
Dolt) is a content-addressed ordered map: each node is referenced by the hash of
its contents, not a file offset, and node boundaries are set by content-defined
chunking so the same logical data always chunks the same way. Two consequences:

- **Structural sharing.** Any subtree whose hash is unchanged between two
  versions is stored once. State storage becomes O(total distinct subtrees), not
  O(commits × nodes).
- **Diff and merge proportional to the change.** Comparing two trees skips every
  subtree pair with equal hashes, so a diff costs O(differences), not O(size).
  This is how Dolt diffs a table in time proportional to the diff.

The cost is real: a chunker, a tree node format, a rebalancing story, and a
second `State` kind in the on-disk format (a `format_version` bump under
ADR-0007). None of it is needed for Phase 2 to be correct.

### Recommendation: keep flat, add prolly when merge or scale forces it

Phase 2 ships flat `State`. `docs/format/` already says a prolly form arrives as
a second `State` shape in a later `format_version`; this survey narrows *when*:

- **Phase 3 (merge)** needs an efficient three-way diff against a common base.
  Flat diff is O(nodes) per pair, which is acceptable for the merge algorithm
  itself, so this is a "reassess", not an automatic trigger.
- A **real store hitting the storage ceiling** (a long-lived agent, or many
  branches) is the hard trigger.

Whichever comes first opens the prolly research ticket. Until then, flat is the
honest choice for the scale the substrate runs at.

## Sketch of the Phase 2 core surface

```
impl Store {
    fn branch(&self, name: &str) -> Result<()>;              // ref at HEAD
    fn branches(&self) -> Result<Vec<(String, ObjectId)>>;
    fn delete_branch(&self, name: &str) -> Result<bool>;     // not the current one
    fn checkout(&self, target: &str) -> Result<()>;          // branch name or commit id; empty staging

    fn working_memory(&self) -> Result<Vec<(String, MemoryNode)>>;   // HEAD state + staging
    fn state_at(&self, commit: ObjectId) -> Result<Vec<(String, MemoryNode)>>;  // time travel

    fn diff(&self, from: ObjectId, to: ObjectId) -> Result<Vec<NodeChange>>;
}

enum NodeChange {
    Added   { id: String, new: ObjectId },
    Removed { id: String, old: ObjectId },
    Modified{ id: String, old: ObjectId, new: ObjectId },
}
```

CLI: `mnem branch [name] [-d name]`, `mnem checkout <target>`, `mnem show
[<commit>]`, `mnem diff <from> [<to>]`. SDK: `store.branch("h1")` plus the
`with store.branch("h1"):` context manager (create, checkout, on exit checkout
back; **no** automatic merge, that is Phase 3).

## Open questions for the grilling (#35)

- View versus materialised working memory: confirm (b), the view. Any real need
  for a `working` table now?
- `checkout` with a dirty `staging`: refuse always, or allow when the staged
  ids do not collide with the target state (Git's "carry your changes across")?
- Does the branch context manager delete its branch on exit, or leave it for the
  user to `merge` or drop later?
- `mnem show` with no argument: working memory, or the `HEAD` commit's state?
- Diff output shape: the `NodeChange` list above, or a richer object that also
  carries the decoded old and new content for rendering?
- Confirm flat `State` for Phase 2 and the prolly triggers above.
- `checkout` and the reflog: still reserved, or does Phase 2 start writing it?

## Sources

- [Git branches are refs (Pro Git, "Git Branches in a Nutshell")](https://git-scm.com/book/en/v2/Git-Branching-Branches-in-a-Nutshell) and [Git References (Pro Git)](https://git-scm.com/book/en/v2/Git-Internals-Git-References)
- [Jujutsu bookmarks](https://docs.jj-vcs.dev/latest/bookmarks/) and [jj/docs/bookmarks.md](https://github.com/jj-vcs/jj/blob/main/docs/bookmarks.md)
- [Prolly Trees: a content-addressed B-tree with structural sharing (Lobsters discussion)](https://lobste.rs/s/grg6xr/prolly_trees_content_addressed_b_tree)
- [A Study in Structural Sharing in a Dolt Prolly Tree (DoltHub)](https://www.dolthub.com/blog/2024-04-12-study-in-structural-sharing/) and [Prolly Tree (Dolt docs)](https://docs.dolthub.com/architecture/storage-engine/prolly-tree)
- [How Dolt Scales to Millions of Versions, Branches, and Rows (DoltHub)](https://www.dolthub.com/blog/2025-05-16-millions-of-versions/)
- [How to Chunk Your Database into a Merkle Tree (DoltHub)](https://www.dolthub.com/blog/2022-06-27-prolly-chunker/)
- [`crabbuild/prolly`: content-addressed ordered map on prolly trees](https://github.com/crabbuild/prolly)
