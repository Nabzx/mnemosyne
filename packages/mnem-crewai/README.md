# mnem-crewai

A [CrewAI](https://github.com/crewAIInc/crewAI) `StorageBackend` backed by
[Mnemosyne](https://github.com/Nabzx/mnemosyne).

**Scaffolding only.** `MnemosyneStorageBackend` exists and imports cleanly,
but every method still raises `NotImplementedError`. Real behaviour lands
one ticket at a time - see
[ADR-0021](https://github.com/Nabzx/mnemosyne/blob/main/docs/adr/0021-the-crewai-adapter.md)
and epic [#232](https://github.com/Nabzx/mnemosyne/issues/232).

```python
from mnem_crewai import MnemosyneStorageBackend

backend = MnemosyneStorageBackend("./agent-memory")
```
