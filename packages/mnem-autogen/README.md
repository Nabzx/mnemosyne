# mnem-autogen

An [AutoGen](https://github.com/microsoft/autogen) `Memory` backed by
[Mnemosyne](https://github.com/Nabzx/mnemosyne).

Every `Memory` method is real - see
[ADR-0023](https://github.com/Nabzx/mnemosyne/blob/main/docs/adr/0023-the-autogen-adapter.md)
and epic [#232](https://github.com/Nabzx/mnemosyne/issues/232).

```python
from autogen_core.memory import MemoryContent, MemoryMimeType
from mnem_autogen import MnemosyneMemory

memory = MnemosyneMemory("my-agent", "./agent-memory")
await memory.add(MemoryContent(content="enterprise plan", mime_type=MemoryMimeType.TEXT))
```

`query(query, ...)` ignores its own argument and returns every item
chronologically, and `update_context(model_context)` formats byte-
identically to `ListMemory`'s own output - both match the SDK's real
reference behaviour (`ListMemory` itself ignores `query()`'s argument too),
not a degraded implementation.

`clear()` is one batched tombstone commit, never a hard delete - history
from before the clear stays recoverable through `mnem log`/`blame`.
`close()` actually releases the store handle, unlike `ListMemory`'s own
true no-op: redb allows only one open handle per store per process, so
this is what lets another instance open the same path afterward.

## Extra methods

Not on the `Memory` interface, for an application to call directly on the
memory instance: `why(item_id)` (the commit and provenance that gave an
item its current value) and `bisect(predicate)` (the first commit where
`predicate` holds) - the same shape as the LangGraph, OpenAI Agents SDK,
and CrewAI adapters' own.

See [`examples/autogen_memory.py`](https://github.com/Nabzx/mnemosyne/blob/main/examples/autogen_memory.py)
for a worked example: a real `AssistantAgent`, `update_context`
demonstrably injecting stored memories before inference, then `why`/
`bisect` tracing a memory back to the commit that wrote it. Needs the
`examples` extra: `pip install "mnem-autogen[examples]"`.
