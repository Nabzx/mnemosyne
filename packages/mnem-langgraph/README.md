# mnem-langgraph

A [LangGraph](https://langchain-ai.github.io/langgraph/) `BaseStore` backed by
[Mnemosyne](https://github.com/Nabzx/mnemosyne). The agent's long-term memory
gets history: every `put` is a commit, and you can ask `why` a memory says what
it does, or `branch` to test a hunch.

```python
from mnem_langgraph import MnemosyneStore

store = MnemosyneStore("./agent-memory")          # one .mnem store
store.put(("memories", "u1"), "plan",
          {"tier": "enterprise", "_meta": {"source": "ticket-4821"}})

item  = store.get(("memories", "u1"), "plan")     # -> Item
hits  = store.search(("memories",), query="enterprise")
blame = store.why(("memories", "u1"), "plan")     # which commit, and from where
```

Pass it to a graph the usual way (`builder.compile(store=store)`), or use it
directly from a node.

## Mapping

- **namespace + key -> node id** `":".join([*namespace, key])`. A `":"` inside a
  segment is not supported.
- **value -> node content.** A reserved `_meta` key (`source` / `step` /
  `observation` / `tool_call` / `note`) is lifted into provenance so `blame`
  works; it is stored too, so `get` round-trips.
- **`search`** is a plain substring (`query`) and equality (`filter`) match, not
  semantic. Wrap a vector store if you need ranking.
- **Async** (`aget` / `aput` / `asearch`) runs the sync path in a worker thread.

## Extra methods

Not on the `BaseStore` interface, for a graph node to call: `branch(name)`,
`switch(target)`, `why(namespace, key)`, `history(limit=...)`.

See [ADR-0016](../../docs/adr/0016-mcp-tools-and-the-adapter-contract.md).
