# mnem-openai-agents

An [OpenAI Agents SDK](https://openai.github.io/openai-agents-python/) `Session`
backed by [Mnemosyne](https://github.com/Nabzx/mnemosyne).

**Scaffolding only.** `MnemosyneSession` exists and imports cleanly, but every
method still raises `NotImplementedError`. Real behaviour lands one ticket at
a time - see [ADR-0020](https://github.com/Nabzx/mnemosyne/blob/main/docs/adr/0020-the-openai-agents-sdk-adapter.md)
and epic [#232](https://github.com/Nabzx/mnemosyne/issues/232).

```python
from mnem_openai_agents import MnemosyneSession

session = MnemosyneSession("u1", "./agent-memory")
```
