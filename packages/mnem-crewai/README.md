# mnem-crewai

A [CrewAI](https://github.com/crewAIInc/crewAI) `StorageBackend` backed by
[Mnemosyne](https://github.com/Nabzx/mnemosyne). Every save/update/delete is a
commit, and `mnem`'s own `blame`/`bisect`/`log` work per memory record. See
[ADR-0021](https://github.com/Nabzx/mnemosyne/blob/main/docs/adr/0021-the-crewai-adapter.md)
and epic [#232](https://github.com/Nabzx/mnemosyne/issues/232).

```python
from crewai.memory.storage.factory import set_memory_storage_factory
from mnem_crewai import MnemosyneStorageBackend

set_memory_storage_factory(lambda spec: MnemosyneStorageBackend("./agent-memory"))
```

`set_memory_storage_factory` is a one-time, process-wide setter (CrewAI's own
factory contract), not a per-`Memory`-instance argument.

## Search: real cosine similarity, with a stated ceiling

`search(query_embedding, ...)` is exact, unindexed cosine similarity over
every embedding in the scanned scope - the same math a vector store like
LanceDB or Qdrant would run, just without a sublinear index. Correct, not
approximated, at a stated cost: `O(records in scope)` per query. Fine up to
roughly a few thousand records per scope; beyond that, use one of CrewAI's
own LanceDB/Qdrant backends instead, or keep Mnemosyne alongside one of them
for its history/blame value rather than through this adapter's `search`.

`search`/`list_records` never filter on `MemoryRecord.private` - CrewAI's own
`RecallFlow` is the real enforcement point (`include_private`/`source`
matching happens there, after calling `storage.search()`), and the
`StorageBackend` protocol never passes a requester identity to the backend
itself to filter by. See
[ADR-0025](https://github.com/Nabzx/mnemosyne/blob/main/docs/adr/0025-crewai-private-filtering-is-not-the-adapters-job.md).

## Async

`asave`/`asearch`/`adelete` wrap their sync counterpart in
`asyncio.to_thread` (matching the LangGraph adapter's existing precedent)
rather than a native async re-implementation - every `StorageBackend`
protocol method is real. Only a worked example (#316) is left.
