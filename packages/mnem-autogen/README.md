# mnem-autogen

An [AutoGen](https://github.com/microsoft/autogen) `Memory` backed by
[Mnemosyne](https://github.com/Nabzx/mnemosyne).

**Scaffolding only.** `MnemosyneMemory` exists and imports cleanly, is a
real `Memory` subclass, but every method still raises `NotImplementedError`.
Real behaviour lands one ticket at a time - see
[ADR-0023](https://github.com/Nabzx/mnemosyne/blob/main/docs/adr/0023-the-autogen-adapter.md)
and epic [#232](https://github.com/Nabzx/mnemosyne/issues/232).

```python
from mnem_autogen import MnemosyneMemory

memory = MnemosyneMemory("my-agent", "./agent-memory")
```
