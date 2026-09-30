# ADR-0025: CrewAI private-filtering is not the adapter's job

- Status: Accepted
- Date: 2026-09-30
- Issue: #314

## Context

ADR-0021's "Provenance and privacy" section states that enforcing
`MemoryRecord.private` ("filtering by `source` unless `include_private=True`")
"happens in the adapter's `search`/`list_records` implementations."
Building #314's `search` against the real, installed `crewai` package found
this isn't buildable as written: `StorageBackend.search`'s actual signature
(`crewai.memory.storage.backend`, a `Protocol`) is `(query_embedding,
scope_prefix=None, categories=None, metadata_filter=None, limit=10,
min_score=0.0)` - no `source`, `include_private`, or any other requester
identity is ever passed in. An adapter cannot filter by "who's asking" when
it is never told who's asking.

Reading CrewAI's own reference backends confirms this isn't an oversight in
the protocol: neither `LanceDBStorage.search` nor `QdrantEdgeStorage.search`
filters on `private` at all. The real filtering happens one layer up, in
`crewai.memory.recall_flow.RecallFlow._do_search`, strictly *after* calling
`storage.search(...)`:

```python
if not self.state.include_private and raw:
    raw = [
        (r, s) for r, s in raw
        if not r.private or r.source == self.state.source
    ]
```

`RecallFlow` holds `include_private` and `source` on its own `state`, which
`storage.search()` never sees. `private` is a signal the storage layer
carries and the *caller* enforces - the same shape as `importance` and
`last_accessed`, which ADR-0021 already correctly treats as round-trip-only.

## Decision

### This narrows ADR-0021, it does not reverse it

Every other decision in ADR-0021 - packaging, node id scheme, storage
granularity, search's own cosine-similarity semantics, `importance`/
`last_accessed` round-tripping - stands unchanged. This corrects one
specific claim: private-enforcement is not, and cannot be, the adapter's
job, because the protocol never gives it the information to do that job.

### `private` is round-tripped only, exactly like `importance` and `last_accessed`

`MnemosyneStorageBackend.search` and `list_records` never filter on
`private`. A private record scores and returns like any other; the field
is stored and surfaced exactly as CrewAI handed it over. This matches both
reference backends' real behaviour, not a degraded version of it.

## Consequences

- `search`/`list_records` are simpler than ADR-0021 described - one fewer
  piece of state to track correctly, since there is nothing to enforce.
- An integrator calling `MnemosyneStorageBackend.search`/`list_records`
  directly (outside CrewAI's own `RecallFlow`) gets every record back,
  private ones included - matching what LanceDB/Qdrant's own backends would
  do if called the same way. `RecallFlow` remains the only real enforcement
  point CrewAI itself ships, and this adapter does not invent a second one.
- `#312`'s already-shipped `list_records` never filtered on `private`
  either; this ADR confirms that was the correct behaviour, not an
  oversight to fix.

## Alternatives considered

**Add an adapter-only `include_private`/`source` keyword argument beyond the
protocol's own signature.** Rejected: `RecallFlow`, the only real caller
CrewAI ships, always calls the plain protocol signature - it would never
pass the extra argument, so this would only help a caller using the adapter
directly outside CrewAI's own recall path, at the cost of a search/
list_records signature that diverges from every other real `StorageBackend`
implementation's actual shape.

**Leave ADR-0021's claim as written and skip filtering silently, undocumented.**
Rejected: ADR-0021 is Accepted; a later correction to a specific decision
needs its own record with the real reasoning, the same precedent ADR-0022
already set narrowing ADR-0017 - not a silent divergence between what the
ADR says and what the code does.
