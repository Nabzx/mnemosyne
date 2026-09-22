# How `mnem` compares

A written comparison, not a benchmark. [`docs/benchmark.md`](benchmark.md)
stays scoped to `mnem` against a plain `dict` and a JSONL log (ADR-0017),
because most of the tools below solve retrieval or context management, not
versioning - a shared numeric table would measure the wrong axis for at
least half of them. This is the axis that actually matters: does the tool
have a real `branch` and `merge`, and does a merge conflict block until
someone resolves it, or does it get decided for you.

## Mem0, Zep - memory stores, not version control

Both manage *what* an agent remembers: extraction, embeddings, retrieval
quality. Neither has anything resembling `branch` or `merge`.

- **[Mem0](https://github.com/mem0ai/mem0)**: the open-source package keeps
  only per-memory history (one row's own edit trail), not a project-wide
  event log - stated in Mem0's own docs, which describe a project-wide
  operations log as a paid-tier feature for dashboards and alerting, not a
  checkout-able history. No branch, no merge, no conflict object.
- **[Zep](https://github.com/getzep/zep)**: built on Graphiti, a temporal
  knowledge graph that closes a fact's validity window when a new one
  supersedes it, rather than deleting the old one - a real form of history,
  and the closest of the two to what `mnem` does. But it's linear: no named
  branches to diverge and later reconcile, no merge, no conflict to
  resolve. Its "audit logs" are API access logs, not a versioned memory
  state.

## Letta - the closest of the memory-layer tools

[Letta](https://github.com/letta-ai/letta) (formerly MemGPT) has real prior
art here: agent templates carry numbered versions with a documented
rollback endpoint, and a `block_history` mechanism chains a hash over each
change to a memory block - genuinely tamper-evident, not a marketing claim.

What it doesn't have: named branches you diverge and later merge, or a
conflict object. It's linear checkpoint-and-rollback, closer to `git reset`
than `git merge`.

## Memoria - real conflict detection, but merge doesn't block on it

[Memoria](https://github.com/matrixorigin/Memoria) (Apache-2.0, backed by
the database company MatrixOne) is the most direct positioning match: its
own tagline is "Snapshot · Branch · Merge · Rollback - for memory, not
code." The claims below are checked against the actual source
(`memoria/crates/memoria-git/src/service.rs` and the Python SDK's
`branches.py`, both in the repo linked above), not the README:

- Conflict *detection* is real. `classify_diff_rows` distinguishes ADDED /
  UPDATED / REMOVED / CONFLICTS, tracking both sides' content when a branch
  and main diverge on the same id.
- But the merge operation does not block on an unresolved conflict. The
  underlying DDL is literally `... when conflict skip`, and the Python
  SDK's `branches.merge(name, strategy="accept")` defaults to auto-accepting
  conflicts rather than refusing to complete. Resolving one conflict by
  hand needs a separate `diff_items()` call followed by
  `apply(accept_branch_conflicts=[...])`, or a `pick()` call with one
  blanket strategy for the whole operation - not a single cycle where a
  merge writes nothing until every conflict has an explicit answer, which
  can differ per item.
- The MCP tool surface an agent actually calls doesn't expose branch
  merging at all, in the version read for this comparison - it's reachable
  through the HTTP API and the Python SDK, an operator/developer surface,
  not something the agent invokes mid-run.

## ByteRover CLI - real git, wrapped around agent context

[ByteRover CLI](https://github.com/campfirein/byterover-cli) ships real
`branch` / `checkout` / `merge` / `commit` verbs, and they're not an
analogy: the CLI is backed by
[`isomorphic-git`](https://isomorphic-git.org/) under the hood, applied to
a serialised "context tree." A merge either succeeds or returns a
structured `conflicts` list by file path (`both_added` / `both_modified` /
`deleted_modified`) - closer in spirit to `mnem`'s block-and-report model
than Memoria's.

What's different: those conflicts are literal git text-merge conflicts on
serialised files, resolved with the usual `<<<<<<<` markers, not a typed
resolution over structured data (`ours` / `theirs` / `base` / a supplied
value). It's also scoped specifically to coding-agent context, not
general-purpose agent memory, and released under the Elastic License 2.0,
not a permissive one.

## What this comparison isn't

Not a claim of a clean field. Memoria and ByteRover are real, shipping,
non-trivial projects, and this isn't the first attempt at git-shaped ideas
for agent state. It's an honest account of where each one actually stands
today, checked against source rather than a marketing page - and where
`mnem`'s claim differs from all four: a structured, node-level, typed
three-way merge (`ours` / `theirs` / `base` / set a value) that writes
nothing until every conflict is explicitly resolved, not a database DDL
flag and not a text-merge marker.
