# ADR-0012: The branch and checkout model

- Status: Accepted
- Date: 2026-09-06
- Issue: #35 (grilling), research #34

## Context

Phase 2 (`v0.0.3`) adds branching, checkout, reading working memory at any past
commit, a structural diff, and node deletion. It is still Era 1: single-agent,
local, deterministic (ADR-0010).

The research (`docs/research/issue-34-branching-survey.md`) found that the hard
part is already done. ADR-0009 built the `refs` table (`name -> [u8; 32]`) with
compare-and-swap moves and name validation, and `Store::commit` already creates
and advances a branch. A branch is a 32-byte value in one row, so branch
creation is O(1) and copies nothing. What is missing is the surface and one
decision: what "working memory" is, given that there is no working-tree concept
today, only `staging` (the index equivalent).

This ADR fixes that surface. It changes no on-disk format: `format_version`
stays 1.

## Decision

### Branches are pointer-only

No copy-on-write of state at branch time, because nothing is copied.

- `Store::branch(name, start: Option<CommitIsh>)` inserts a `refs` row at the
  given start point, or where `HEAD` resolves. Fails if the name already exists
  (no silent move) or is invalid (ADR-0009 rules).
- `Store::branches()` is `refs::list`.
- `Store::delete_branch(name)` refuses the branch `HEAD` is on and otherwise
  drops the row. There is no force variant: Phase 2 has no garbage collection,
  so a deleted branch's commits stay reachable by id.

CLI: `mnem branch` lists (current marked `*`, with the tip's 12-hex id and
message); `mnem branch <name> [<start-point>]` creates; `mnem branch -d <name>`
deletes.

### `checkout` is one atomic `HEAD` write

`Store::checkout(target: CommitIsh, discard: bool)`:

- Resolves `target`: an exact branch name first, then an unambiguous commit-id
  hex prefix (minimum four characters). A branch target gives an attached
  `HEAD`; a commit target gives a detached `HEAD`.
- Refused when `staging` or `staging_tombstones` is non-empty, with a message
  naming the pending ids, unless `discard` is set (then both are cleared).
- Writes `HEAD` atomically (temp file plus rename). Nothing else is touched. No
  lock is taken.
- Checkout of the current branch, or of the commit `HEAD` already resolves to,
  is a no-op that succeeds.

CLI: `mnem checkout <target>`, `mnem checkout -b <name> [<start-point>]`
(create-and-switch; the name must not exist). On success it prints a short line:
`Switched to branch 'h1'`, `Created branch 'h1' at 45ee8a6b`, or `HEAD is now at
45ee8a6b (detached)`.

### Working memory is a view, never stored

Working memory is defined as:

> the `HEAD` commit's `State`, minus the ids in `staging_tombstones`, with
> `staging` overlaid.

It is computed on every read. There is no `working` table.

- `Store::working_memory() -> BTreeMap<String, MemoryNode>` (decoded).
- The SDK returns it as `dict[str, MemoryNode]`, keyed by node id.
- `mnem show` with no argument prints working memory.

### Time travel reads any commit

- `Store::state_at(commit) -> BTreeMap<String, MemoryNode>` loads a commit's
  `State` and its nodes, decoded. `Store::state_map_at(commit) -> BTreeMap<String,
  ObjectId>` is the raw form for callers that do not need content (`diff` uses
  it internally).
- Any commit that exists in `objects` and is a `Commit`. No branch, no ancestry
  requirement. Read-only, so no lock.
- `mnem show <commit>` prints the full memory snapshot at that commit.

### Node deletion

- `Store::rm(id)` stages a tombstone in a new `staging_tombstones` table (a set
  of node ids). This table is **local working state**, like `staging`: it is not
  part of the portable history and does not travel with a store.
- `rm` of an id that is neither in the `HEAD` state nor staged is an error. `rm`
  of an id that is only a staged add (not in `HEAD`) unstages it instead of
  tombstoning.
- `commit` builds the new `State` from the parent's `State` plus `staging`, then
  removes every tombstoned key. A commit with only tombstones staged is valid.
- `unstage(id)` clears the id from both `staging` and `staging_tombstones`.

CLI: `mnem rm <id>`.

### Structural diff

- `Store::diff(from: DiffTarget, to: DiffTarget) -> Vec<NodeChange>` where
  `DiffTarget` is `Commit(ObjectId)` or `Working`.
- `NodeChange` is `Added { id, new }`, `Removed { id, old }`, or `Modified { id,
  old, new }`, carrying `ObjectId`s only. Computed as a merge-join over the two
  sorted `State` maps: O(nodes). Any two states; no ancestry required.
- The SDK returns `list[NodeChange]`, a frozen dataclass (`id`, `kind`, and
  `old` / `new` as `MemoryNode | None`).

CLI: `mnem diff` (`HEAD` vs working), `mnem diff <commit>` (vs working), `mnem
diff <a> <b>`. It shows the old and new content of each change by default;
`--stat` and `--name-only` give the terse id list.

### `status`

In scope for `v0.0.3`. `mnem status` prints the branch line (`On branch main`,
or `HEAD detached at 45ee8a6b`), the `HEAD` commit's short id and message, then
`diff(HEAD, Working)` rendered as `+` (new), `~` (update), `-` (removal) against
the node id. "Clean" when nothing is staged.

### The SDK branch context manager

The ADR-0004 carve-out. `with store.branch("h1"):`

- On enter: create `h1` at `HEAD`, checkout `h1`. Entering with dirty staging
  raises, per `checkout`'s rule.
- On exit, normal or exception: checkout back to the branch that was current on
  enter; leave `h1` in place for the caller to `merge` (Phase 3) or
  `delete_branch`; re-raise on exception.

No automatic merge. Merge is Phase 3.

### Deferred

- **The prolly-tree `State` form.** Phase 2 keeps the flat `State` (a full
  id-to-`ObjectId` map per commit). Storage grows as commits times nodes, which
  is fine at the scale an agent runs at. A prolly form (a second `State` kind, a
  `format_version` bump) is introduced when the first of these fires: Phase 3's
  three-way merge is judged to need it, or a real store hits the storage
  ceiling. Whichever comes first opens its own research ticket.
- **The reflog.** ADR-0009 reserved it; Phase 2 does not write it. Detaching
  prints the commit id so it is not lost.
- **Rev-navigation syntax** (`<commit>^`, `<commit>~2`). Explicit ids from
  `mnem log` for now.

## Consequences

- Branching and checkout cost one small write each. The design adds no on-disk
  format structure that a store reader sees; `format_version` stays 1.
- Working memory has no materialisation cost and no third place to keep in sync,
  at the price of an O(nodes) walk per full read. That walk is the true cost of
  "give me the whole memory" under any design.
- `staging_tombstones` is a second piece of local-only state. `commit`,
  `unstage` and `status` all have to account for it.
- An agent can now forget a stale belief (`rm`), fork memory to try an approach
  (`branch` plus the context manager), see what it changed (`diff`, `status`),
  and read what it knew at any past point (`show <commit>`).
- The flat-`State` ceiling is real and named. A long-lived agent will hit it
  before a short one; the trigger and the response are written down rather than
  discovered.

## Alternatives considered

**A materialised `working` table that `checkout` fills.** Closest to Git's
working tree. Rejected: an O(nodes) write on every checkout, and a third place
the node set lives and can drift. The view has none of that cost.

**Carry staged changes across a checkout when they do not collide** (Git's real
behaviour). Rejected: it doubles the checkout logic for a workflow that does not
need it. Refuse-by-default with `--discard` is one path.

**Defer node deletion to Phase 3.** Rejected: it would ship `v0.0.3` and
`v0.0.4` with no way for an agent to remove a fact, and Phase 3's merge would
have to invent the primitive anyway.

**Introduce the prolly tree now, in Phase 2** (ADR-0002 originally placed it
here). Rejected for this phase: none of Phase 2's deliverables need it to be
correct, and it is real work (a chunker, a node format, a rebalancing story, a
format change). Deferred with named triggers.

**Start writing a reflog in Phase 2**, since `checkout` is the first operation
that wants one. Rejected: the reflog is its own design (retention, `HEAD` versus
per-branch, a `mnem reflog` surface) and nothing in Phase 2's definition of done
needs it.
