# ADR-0021: The CrewAI adapter

- Status: Accepted
- Date: 2026-09-23
- Issue: #250 (research + grilling, one combined ticket)

## Context

Research (`docs/research/issue-250-crewai-survey.md`, read against the real
`crewAIInc/crewAI` source) found CrewAI's pluggable memory primitive,
`StorageBackend`, is far richer than the OpenAI Agents SDK's `Session`
(ADR-0020) and is vector-search-first: `search(query_embedding:
list[float], ...)` takes only a pre-computed embedding, no query string to
fall back on. The finding that makes this tractable: CrewAI's own `Memory`
class always computes embeddings itself (via its own configured embedder)
before handing a record to `storage.save()` or a query to
`storage.search()`. A backend never needs its own embedder dependency, only
to persist and compare vectors it is handed - which keeps this adapter
inside ADR-0004's boundary (adapters may take on network/runtime
dependencies MCP and LangGraph already do; an embedding-model dependency
would have been a materially different kind of commitment).

`MemoryRecord` already carries first-class `source` and `private` fields -
provenance and access scoping map straight through, unlike LangGraph's
invented `_meta` carrier (ADR-0016). Scope is a hierarchical `/`-delimited
path, not a flat namespace tuple, with real one-level child-walk operations
(`list_scopes`, `get_scope_info`) that no adapter built or designed so far
has needed.

## Decision

### Packaging

`packages/mnem-crewai/`, PyPI `mnem-crewai`, import `mnem_crewai`, following
the same shipped-naming convention as `mnem-mcp`, `mnem-langgraph`, and
`mnem-openai-agents` (ADR-0020). Pins `mnem-agents ~= 0.0`, versions
independently.

### Storage granularity: one node per memory record

Every `MemoryRecord` (content, scope, categories, metadata, importance,
source, private, and its embedding) becomes one Mnemosyne node, mirroring
ADR-0020's per-item granularity for the same reason: `blame` and `bisect`
should resolve to *which memory*, not just *which save call*. The full
record, embedding included, is the node's opaque JSON content (ADR-0003).

### Node id scheme: CrewAI's own `/`-delimited scope path, honored literally

A node id is `{scope}/{record_id}` (`record_id` CrewAI's own generated
`MemoryRecord.id`), rather than normalizing into the `:`-prefix convention
ADR-0016 and ADR-0020 established for LangGraph and the OpenAI Agents SDK.
This is a deliberate departure from adapter-to-adapter consistency: each
prior adapter already maps its own host framework's model onto Mnemosyne on
its own terms (ADR-0016 chose namespace-as-prefix *because* it matched
LangGraph specifically, not from a project-wide id-format rule), and
`list_scopes`/`get_scope_info` are genuine one-level tree-walk operations -
implementing them correctly needs the id to actually reflect `/`-nesting,
not just contain slashes incidentally.

### Search: real, unindexed, brute-force cosine similarity - with a stated ceiling

`search(query_embedding, scope_prefix, categories, metadata_filter, limit,
min_score)` is implemented as exact cosine similarity over every record's
stored embedding within the scanned scope - the same math LanceDB or Qdrant
would run against the same vectors, just without a sublinear index.
**Correct, not approximated or faked**, at a stated cost: O(records in
scope) per query. The package README documents a recommended ceiling
(on the order of a few thousand records per scope) and points to CrewAI's
own LanceDB/Qdrant backends beyond it - mirroring the LangGraph adapter's
already-accepted precedent of degrading `search` to something honestly
weaker than a purpose-built store, documented rather than silently
approximated (ADR-0016).

### Provenance and privacy: pass straight through

`MemoryRecord.source` maps directly to Mnemosyne's `Provenance.source` - no
invented carrier, unlike LangGraph's `_meta` key or the OpenAI Agents
adapter's `on_tool_end` hook (ADR-0020). `private` is stored and
re-surfaced as-is; enforcing it (filtering by `source` unless
`include_private=True`) happens in the adapter's `search`/`list_records`
implementations, exactly as CrewAI's own `Memory.recall()` already
documents the semantics.

### `importance` and `last_accessed`: round-trip only

The adapter never recomputes or independently updates these fields. It
stores exactly what CrewAI hands it in `save()`/`update()`, and returns
exactly that on read. CrewAI's own `Memory`/`RecallFlow` code is the only
thing that should decide what these values mean; an adapter maintaining its
own view risks silently disagreeing with what CrewAI's own ranking would
have produced - the same principle ADR-0020 applied to why `on_tool_end`
provenance capture is documented rather than silent, just in the opposite
direction here (do less, not more).

### Registration

`crewai.memory.storage.factory.set_memory_storage_factory()` is a one-time,
process-wide setter, not a per-`Memory`-instance argument. The adapter
documents this plainly (call it once at application startup) rather than
pretending it is scoped like `Session`'s per-`Runner.run` argument
(ADR-0020) or `BaseStore`'s per-graph argument (ADR-0016) - it is not, and
saying otherwise would mislead an integrator.

### Async

Sync methods call the SDK directly; async (`a`-prefixed) methods wrap the
sync path in `asyncio.to_thread`, mirroring the LangGraph adapter's existing
precedent (ADR-0016) rather than a native async re-implementation.

### Errors

The same taxonomy as every other adapter (ADR-0016): `not_found`,
`invalid_ref`, `conflict`, `corrupt_store`, `no_store`, `store_io`, `busy`.

## Consequences

- A fifth adapter package (after `mnem-mcp`, `mnem-langgraph`, and the
  designed-but-unbuilt `mnem-openai-agents`), same independent-versioning
  shape.
- `blame`/`bisect` work per memory record, and CrewAI's own semantic recall
  keeps working exactly as before - the adapter is a real `StorageBackend`,
  not a degraded one, up to its stated scale ceiling.
- The one real limitation this ADR accepts and documents: no sublinear
  search index. A crew with memory scopes well beyond a few thousand
  records per scope should use LanceDB/Qdrant instead, or wrap Mnemosyne's
  history/blame value alongside one of them rather than through this
  adapter's own `search`.
- A fourth id-delimiter convention now exists across adapters (`:` prefix
  for LangGraph and OpenAI Agents SDK, `/` path for CrewAI) - not
  unified, deliberately, since each honors its own host framework rather
  than a project-wide scheme. Worth revisiting only if a real reason to
  unify appears, not for its own sake.

## Alternatives considered

**A fake or degraded `search()` (e.g. keyword matching over `content`).**
Rejected: unlike LangGraph's `search(ns, query?)`, CrewAI's `search()` gets
no query string, only a vector - there is nothing to keyword-match against.
A degraded implementation here would not just be weaker, it would be
answering a different question than the one asked.

**The adapter computes its own embeddings.** Rejected: unnecessary given
CrewAI's own `Memory` class already computes and hands over every embedding
involved, and it would have pulled a real embedding-model dependency into
an adapter package for no benefit.

**Normalizing node ids to the `:`-prefix convention for adapter-to-adapter
consistency.** Rejected: `list_scopes`/`get_scope_info` are real one-level
tree-walks that need the id to reflect actual `/`-nesting; fighting
CrewAI's own hierarchy to match a convention invented for a flat namespace
would make the one adapter with real tree operations harder to get right,
for a consistency benefit no contract actually depends on.

**The adapter maintains its own `importance`/`last_accessed` values (e.g.
bump on every search hit).** Rejected: these exist for CrewAI's own ranking
to use as it sees fit; an adapter second-guessing them is scope creep that
risks disagreeing with what CrewAI's own code would have computed.

**Block on building an indexed search variant before shipping.** Rejected:
blocks the adapter on infrastructure this project has no reason to own, for
a scale problem documented and stated rather than hidden - the same call
already made for LangGraph's `search` (ADR-0016).
