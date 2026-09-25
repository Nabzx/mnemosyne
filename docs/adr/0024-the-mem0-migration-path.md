# ADR-0024: The Mem0 migration path

- Status: Accepted
- Date: 2026-09-25
- Issue: #253 (research + grilling, one combined ticket)

## Context

Mem0 is a memory store, not an agent framework - "an adapter for Mem0"
doesn't mean the same thing as an adapter for CrewAI or AutoGen. There is no
`Session`/`Memory`/`StorageBackend`-shaped object to implement the way
ADR-0020/0021/0023 had. What makes sense instead is a migration path for
someone moving off Mem0, or running both side by side.

Research (`docs/research/issue-253-mem0-migration-survey.md`, extending
#282's already-verified `mem0ai/mem0` source reading) found two real
constraints Mem0's own `get_all` API imposes: it requires an explicit scope
(`user_id`/`agent_id`/`run_id`) up front, with no list-everything call, and
`top_k` is a cap with no cursor. It also produced a concrete, sourced answer
to a real external contributor's question on this same issue
(`samvallad33`): `mem.payload.get('data', '')` confirms Mem0's own code
silently defaults to an empty string when a record's content is missing,
not an error - the concern was real.

## Decision

### Both a tool and a guide

A `mnem.agents.import_mem0(...)` SDK function, plus a short written
migration guide. They answer different questions: the function does the
work, the guide states the one limitation that matters honestly up front -
this imports Mem0's **current state**, not a ported-over timeline, because
Mem0's own `history` table (verified in #282) is scoped per `memory_id`,
with no project-wide event log to port in the first place.

### Location: the SDK, not a script or a new package

`mnem.agents.import_mem0(store, mem0_memory, *, filters, ...)` lives
alongside the module's existing `remember` / `remember_many` / `forget`
(ADR-0016) - the same "the SDK holds the ergonomics" precedent, not
`scripts/` (repo tooling, not user-facing functionality) and not a new
`packages/` adapter (this isn't an ongoing integration surface anyone
depends on continuously; it's a function called once).

### Empty content: skip and log, never import-as-empty or fail the batch

A migration tool handling someone else's real data is conservative about
what it writes. A record whose `memory` field is empty is skipped, logged
by its Mem0 `id` so the caller can inspect the source data themselves, and
the import continues - not silently written as an empty `mnem` node, and
not a reason to abort an otherwise-good migration.

### Node id scheme: `mem0:{memory_id}`, namespaced

Mem0's own `memory_id` (a UUID) becomes `mem0:{memory_id}`, not a bare id.
A bare id risks colliding with existing content in a target store that
already holds other memories (this is explicitly a "moving into an existing
store, possibly alongside other sources" case, unlike a fresh adapter's own
namespace); the prefix costs nothing and keeps the import's origin visible
in the id itself, permanently.

### Pagination: a real loop, not a single large default

`import_mem0` calls `get_all` with an initial `top_k`, and keeps raising it
(or otherwise re-querying) until a call returns fewer results than
requested, rather than trusting one fixed cap. A migration tool that
silently drops memories past an arbitrary count is a worse failure mode
than a slower import.

### Provenance mapping

Mem0's promoted payload fields - `user_id`, `agent_id`, `run_id`,
`actor_id`, `role` - map into `Provenance` the same way every adapter's
mapping has worked: `actor_id` (or `role` where `actor_id` is absent) as
`source`, `created_at` preserved, everything else Mem0 attaches folded into
`metadata` rather than dropped.

### Errors

The same taxonomy as every adapter (ADR-0016), plus one migration-specific
outcome: a per-record skip (empty content) is reported in the function's
return value (a count and a list of skipped ids), not raised as an error -
it's an expected, handled case, not a failure.

## Consequences

- A migration function, not a package - no new PyPI release train, ships
  in the next `mnem-agents` release.
- Provenance survives the move (Mem0's `actor_id`/`role`/`created_at` are
  real fields, not invented), but Mem0's own per-memory edit history does
  not - stated honestly in the guide, not silently dropped without
  mention.
- The `mem0:` id prefix means a store that has both native `mnem` content
  and imported Mem0 content keeps them visibly distinct - a deliberate,
  permanent marker, not a one-time migration artifact that disappears.
- Answers a real, publicly-asked question (the external comment on #253)
  with a concrete decision, not just a private research finding.

## Alternatives considered

**A standalone `scripts/import_mem0.py`.** Rejected: this project's
`scripts/` directory is repo tooling (changelog/ADR-index checks), not
user-facing functionality - a real migration tool belongs where the SDK's
other ergonomic helpers already live.

**A dedicated `packages/mnem-mem0-import` package.** Rejected: this isn't
an ongoing integration surface anyone depends on continuously the way the
adapters are; a new independently-versioned package is disproportionate to
"a function someone calls once."

**Bare `memory_id` as the node id.** Rejected: real collision risk in a
store that already holds other content, for no benefit over a namespaced
id - the prefix is free and permanently useful context.

**Import-as-empty or fail-the-batch for missing content.** Rejected:
writing an empty node silently is worse than not writing anything, and
failing an entire migration over one bad record punishes the rest of a
otherwise-good import.

**A single large default `top_k`.** Rejected: silently caps how much gets
migrated with no signal to the caller that anything was left behind - the
paging loop costs a few extra API calls, not correctness.
