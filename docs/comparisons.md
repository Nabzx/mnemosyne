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

`docs/benchmark.md`'s audit-query table stays scoped to `dict`/JSONL/`mnem`
(ADR-0017) - that page is CI-gated, and Mem0's and Zep's behaviour isn't
`mnem`'s own build to verify. The same seven questions, answered here
against each tool's real source instead
([ADR-0022](adr/0022-mem0-and-zep-on-the-audit-query-table.md)):

| Question | Mem0 | Zep (Graphiti) |
| --- | --- | --- |
| what does it believe now | yes | yes |
| what did it believe at step t | no built-in call (`history` is scoped per memory, no global reconstruction) | yes, via `SearchFilters`' `valid_at`/`invalid_at` date filters - but `search()`'s own convenience wrapper always uses "now" |
| when did belief X first go wrong (bisect) | no | no |
| which observation set X | shallow: `actor_id`/`role` per history event | real: `EntityEdge.episodes` references the originating observation(s) |
| what did step t change | qualified: a real diff per memory, no cross-memory "commit" | no explicit diff API; derivable from two point-in-time queries |
| merge two agents' memories, surfacing conflicts | no | no (a later fact invalidates an earlier one, not a flagged conflict) |
| storage after L steps | `O(keys)` live state + `O(events)` append-only history | `O(all facts ever asserted)`, monotonically growing |

Bisect is the one row where `mnem`, Mem0, and Zep are all behind a plain
JSONL log's linear scan - worth stating plainly rather than only ever
naming where `mnem` wins.

## Letta - the closest of the memory-layer tools, and a real git repo

[Letta](https://github.com/letta-ai/letta-code) (formerly MemGPT) has
pivoted since this comparison first looked at it: the block-history/
rollback API it used to ship is retired, sitting only on an `archive`
branch. The active product, `letta-code`, is a CLI tool whose agent memory
is stored as a literal git repository - cloned, pulled, committed, and
pushed like any other, confirmed directly in its own source.

That makes it the closest thing to `mnem`'s own idea of any tool compared
here - and also the sharpest contrast. Letta's git usage is sync plumbing,
not a versioning feature: no `log`, no checkout-at-revision, no `branch`,
no `merge` - a push conflict just surfaces as a raw error. Its own
first-party docs recommend driving raw git by hand (`git add`, `git
commit`, `git push`) for basic operations like moving memory between
agents. Letta hands you a real git repo and tells you to use real git
yourself. `mnem` gives you `blame`, `bisect`, and a typed merge-conflict
object on the same underlying idea.

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
