# Research: a CrewAI adapter (issue #250)

Feeds the ADR that fixes a `mnem-crewai` adapter's contract, following the
same discipline as #249's OpenAI Agents SDK survey: read against the real
`crewAIInc/crewAI` source (MIT, `lib/crewai`), not its docs pages.

Two questions:

1. What is CrewAI's own pluggable memory primitive, and what shape does it
   require?
2. Is that shape compatible with Mnemosyne at all, given Mnemosyne is
   explicitly not a retrieval/semantic layer (`docs/comparisons.md`,
   ADR-0016) - and if so, on what terms?

## 1. The contract: `StorageBackend`, a vector-search-first protocol

`crewai.memory.storage.backend.StorageBackend` is the pluggable seam,
swapped in globally via `set_memory_storage_factory()` (a one-time,
process-wide setter, `factory.py`) rather than passed per-construction -
closer to a registration hook than an argument. It is far richer than the
OpenAI Agents SDK's four-method `Session`: `save` / `search` / `delete` /
`update` / `get_record` / `list_records` / `get_scope_info` / `list_scopes` /
`list_categories` / `count` / `reset`, each with a sync and an async
(`a`-prefixed) form.

**The record**, `MemoryRecord` (`types.py`): `id`, `content: str`,
`scope: str` (a hierarchical path, e.g. `/company/team/user`), `categories:
list[str]`, `metadata: dict`, `importance: float` (0-1, affects ranking),
`created_at` / `last_accessed`, `embedding: list[float] | None`, and -
notably - **`source: str | None` and `private: bool` are already first-class
fields**, described in the docstring as exactly what Mnemosyne calls
provenance and access scoping. No reserved `_meta`-key workaround is needed
here the way ADR-0016 needed one for LangGraph.

**Search is vector-first and mandatory.** `search(query_embedding:
list[float], scope_prefix=None, categories=None, metadata_filter=None,
limit=10, min_score=0.0)` takes only a pre-computed embedding vector, no
query string - unlike LangGraph's `search(ns, query?)`, there is no text to
fall back to filtering on. `factory.py`'s docstring confirms only two
built-in backends exist, LanceDB and Qdrant, both vector stores; no
non-vector backend ships in-tree as precedent.

## 2. The finding that changes the picture: the adapter never computes embeddings

Read `crewai.memory.unified_memory.Memory` (CrewAI's own concrete class,
what a crew actually talks to) end to end for `remember()`. It routes
through `_encode_batch`, which builds each `MemoryRecord` using
`self._embedder` (CrewAI's own configured embedder) **before** calling
`storage.save(records)`. Records handed to a `StorageBackend.save()` call
already carry a real embedding. Symmetrically, `Memory.recall(query: str,
depth="shallow"|"deep")` computes `embedding = embed_text(self._embedder,
query)` itself before calling `storage.search(query_embedding=...)` (`depth
="deep"`, the default, additionally runs an LLM-driven `RecallFlow` that
decomposes the query and selects scopes - all inside `Memory`, invisible to
the backend).

**This means a `StorageBackend` implementation never needs its own embedder
or embedding-model dependency.** Its job is to persist vectors it is handed
and compare vectors it is handed - not generate them. That removes the
biggest apparent obstacle (Mnemosyne pulling in an LLM/embedding dependency,
which ADR-0004 would never allow into the offline core, and which even an
adapter package taking on would be an odd new commitment).

What is left, and is real: Mnemosyne has no vector index. A correct backend
can still do **brute-force cosine similarity** over every record's stored
embedding within the searched scope - honest, exactly as accurate as
LanceDB's own ranking (same vectors, same math, no approximation), just
O(records in scope) per query rather than sublinear. `factory.py`'s own
docstring names "an in-memory fake for tests" as a legitimate custom-backend
use case, which is a lower bar than what's proposed here, but confirms
CrewAI itself doesn't assume every backend is an indexed vector database.

## 3. Scope is a tree, not a flat namespace tuple

`scope: str` paths (`/company/team/user`) with `list_scopes(parent)`
walking immediate children make CrewAI's scoping model hierarchical, unlike
LangGraph's flat `namespace: tuple[str, ...]` (ADR-0016) or the `{session_
id}:` flat prefix ADR-0020 chose for the OpenAI Agents SDK adapter. Two
live questions this raises:

- **Node id scheme**: does a Mnemosyne node id literally embed the `/`-
  delimited scope path (`{scope}/{record_id}`, honoring CrewAI's own
  separator), or does it get normalized to Mnemosyne's existing `:`-prefix
  convention (ADR-0016, ADR-0020) for consistency across adapters, at the
  cost of translating between two path grammars on every call?
- **`get_scope_info` / `list_scopes` / `list_categories`**: these are
  genuine tree-walk operations (count records, list child scopes, aggregate
  categories, recursively under a path) with no equivalent in any adapter
  built or designed so far. `working_memory()`'s `BTreeMap` ordering makes a
  prefix scan cheap; a *child-scope* walk (one level of a tree, not a flat
  prefix match) needs the id scheme to actually reflect scope nesting, not
  just be a flat string that happens to contain slashes.

## 4. Provenance and privacy map directly, unusually cleanly

`MemoryRecord.source` and `.private` are already first-class - `source`
maps straight onto Mnemosyne's `Provenance.source` with no adapter-invented
carrier, and `private` is a real access-scoping bit CrewAI's own `recall()`
already enforces (`include_private`, matching by `source`). This is the one
part of this adapter that is *more* straightforward than any prior one.

## 5. A striking vocabulary match, likely coincidental

`Memory`'s own public API is `remember()` / `remember_many()` / `recall()` /
`forget()` - the same verbs `mnem.agents` already uses (`remember`,
`remember_many`, `forget`, ADR-0016), independently arrived at. Worth naming
in an adapter's own docs as a point of natural fit, not engineered.

## Recommendation

Implement `StorageBackend` for real, not degraded: store each `MemoryRecord`
(content, scope, categories, metadata, importance, source, private,
embedding included) as one Mnemosyne node's content, one node per record,
mirroring ADR-0020's per-item granularity for the same reason - `blame` and
`bisect` should resolve to *which memory*, not just *which save call*.
Implement `search` as real, un-approximated brute-force cosine similarity
over the embeddings already present in the scanned scope - accurate, just
not sublinear, and documented as such (a scale ceiling, not a silent
approximation). Map `source` and `private` straight through, no invented
carrier. Use a `/`-based node id honoring CrewAI's own scope grammar rather
than translating into the `:`-prefix convention the other two adapters use,
since `list_scopes`'s child-walk semantics depend on real hierarchy, not a
flat prefix.

This is a recommendation, not a decision: whether brute-force search's scale
ceiling is acceptable to ship, and the exact id-scheme choice, are real forks
for the grilling to settle.

## Open questions for the grilling

- **Is un-approximated, unindexed cosine-similarity search an acceptable
  `v1`?** It is provably correct at any scale, just O(records in scope) per
  query - fine for hundreds to low thousands of records per scope, a real
  ceiling beyond that. Does the adapter's own docs state a recommended
  ceiling and point to LanceDB/Qdrant beyond it, or is that premature before
  anyone hits it?
- **Node id / scope-path scheme**: honor `/`-delimited scope paths literally
  in the node id, or normalize to the `:`-prefix convention ADR-0016/
  ADR-0020 already established? The tree-walk operations
  (`list_scopes`/`get_scope_info`) are the forcing function either way.
- **`importance` and `last_accessed`**: CrewAI's own ranking uses
  `importance` (0-1) and presumably recency; does the adapter treat these as
  opaque fields it round-trips faithfully (simplest, most honest), or try to
  derive/update them itself (risks silently disagreeing with what CrewAI's
  own `Memory` class would have computed)?
- **Async**: mirror the LangGraph adapter's precedent (sync methods call the
  SDK directly, async methods wrap in `asyncio.to_thread`, ADR-0016) rather
  than a native async re-implementation?
- **Package name and scope**: `packages/mnem-crewai/`, PyPI `mnem-crewai`,
  following the shipped (not ADR-0016's original) naming convention.

## Sources

- `crewAIInc/crewAI` (cloned at research time, MIT): `lib/crewai/src/crewai/
  memory/storage/backend.py`, `factory.py`, `types.py`,
  `unified_memory.py` (`remember`, `_encode_batch`, `recall`).
- [ADR-0016](../adr/0016-mcp-tools-and-the-adapter-contract.md) (the
  LangGraph namespace-as-prefix and `_meta`-carrier precedents this
  research compares against).
- [ADR-0020](../adr/0020-the-openai-agents-sdk-adapter.md) (the per-item
  node-granularity precedent this research follows).
