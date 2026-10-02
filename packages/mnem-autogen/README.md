# mnem-autogen

An [AutoGen](https://github.com/microsoft/autogen) `Memory` backed by
[Mnemosyne](https://github.com/Nabzx/mnemosyne).

`add`/`query`/`update_context` are real. `clear`/`close` still raise
`NotImplementedError` - real behaviour lands one ticket at a time, see
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
