# mnem-adk

A Google ADK `BaseMemoryService` backed by [Mnemosyne](https://github.com/Nabzx/mnemosyne).
Whatever `load_memory` or `preload_memory` puts in your agent's prompt, mnem
can tell you which session, which turn and which tool output put it there,
and let you undo it.

An independent, third-party ADK memory service, not built or endorsed by Google.

```python
import mnem
from mnem_adk import MnemosyneMemoryService

store = mnem.open("./agent-memory")      # the caller owns this handle
memory_service = MnemosyneMemoryService(store)

runner = Runner(
    app_name="support",
    agent=agent,
    session_service=session_service,
    memory_service=memory_service,
)
# after a run: memory_service.add_session_to_memory(session) - or let a
# load_memory / preload_memory tool call search_memory for you
```

## Mapping (ADR-0026)

- **Scope**: a flat `{app_name}:{user_id}:` prefix, not a `/`-path -
  `search_memory`'s own real signature takes no `session_id`. One store can
  serve every app and user your host application uses.
- **One node per `Event`**, not one per session - `blame`/`bisect` stay
  meaningful at the level of the event that introduced something.
- **Node id**: an ingested event honours `Event.id` literally
  (`{app_name}:{user_id}:{session_id}:{event.id}`); a direct `add_memory`
  entry with its own id uses it, one with none gets a zero-padded sequence
  number from a reserved counter node.
- **Content**: the event's full `Content` object, serialised - not reduced
  to its text, matching `InMemoryMemoryService`'s own real behaviour.
- **Provenance**: a pure field read, no capture hook needed -
  `source` from `Event.author`, `tool_call` from a `function_call` part,
  `observation` from a `function_response` part, `agent_step` from
  `Event.invocation_id`.
- **`search_memory`**: `InMemoryMemoryService`'s own real keyword-overlap
  ranking, ported directly from its source, not approximated or degraded.
- **`delete_memory`**: this adapter's own extra method, outside
  `BaseMemoryService` - google-adk has no real `delete_memory` in the
  released `2.12.0` to match. A tombstone commit; a later re-ingestion of
  the same event does not resurrect it.

## Extra methods

Not on `BaseMemoryService`, for code that wants Mnemosyne's own
differentiators: `branch(name)`, `switch(target)`, `merge(theirs, ...)`,
`why(node_id)`, `bisect(predicate)`, `history(limit=...)`, plus
`delete_memory(node_id)` above. All are **store-wide**, not scoped to one
`(app_name, user_id)` - other scopes may share this store, and a branch or
merge moves the whole thing.

```python
memory_service.branch("hypothesis")
memory_service.switch("hypothesis")
await memory_service.add_session_to_memory(session)  # a divergent line
memory_service.switch("main")

result = memory_service.merge("hypothesis")
if result.status == "conflicts":
    memory_service.merge("hypothesis", resolutions={c.id: "theirs" for c in result.conflicts})

memory_service.why(node_id)   # -> Blame: which commit, what provenance
memory_service.bisect(lambda memory: node_id in memory)
```

## A real positioning claim, not an invented one

Of ADK's own four built-in, concrete memory services
(`InMemoryMemoryService`, `SqliteMemoryService`, `VertexAiMemoryBankService`,
`VertexAiRagMemoryService` - checked directly against each class's own
method overrides), only the managed `VertexAiMemoryBankService` implements
all of `add_session_to_memory`, `add_events_to_memory`, `add_memory` and
`search_memory`. `mnem-adk` implements all four, making it the most
complete open-source `BaseMemoryService` implementation that exists today.

## Known limits

- **Per-turn overhead over `InMemoryMemoryService`, measured, not
  estimated** - run `pytest packages/mnem-adk/tests/
  test_adk_overhead_measurement.py -s` for the real numbers on your own
  machine (they vary with hardware; a local run put `add_session_to_memory`
  at roughly 9ms/turn over `InMemoryMemoryService`'s plain in-process
  append, and `search_memory` under 1ms/turn over it). The gap exists
  because every `add_session_to_memory` call is a real, durable commit to
  disk, not a dict mutation; `search_memory` scans every candidate in the
  scope linearly on both sides, so its own overhead is mostly reading
  nodes back out of the store rather than holding live Python objects.
  Neither path is tuned further than matching the reference algorithm; a
  host application ingesting at a high turn rate should expect the write
  side's real disk cost.
- **`add_memory` does not guard against a previously-deleted id.**
  `add_session_to_memory`/`add_events_to_memory` both respect a tombstone
  from `delete_memory` and will not resurrect a deleted event on
  re-ingestion; a direct `add_memory` call with an explicit id is treated
  as an intentional write and is never silently dropped.
- **No `custom_metadata` support.** Both `add_events_to_memory` and
  `add_memory` accept the parameter (matching the real `BaseMemoryService`
  signature) but ignore it, the same way `InMemoryMemoryService` does for
  `custom_metadata` on `add_events_to_memory`.
- **`requires-python = ">=3.11"`**, one version above every other adapter
  package here - `google-adk` itself forces this floor.

See [ADR-0026](https://github.com/Nabzx/mnemosyne/blob/main/docs/adr/0026-the-google-adk-adapter.md)
and the [ADK adapter map](https://github.com/Nabzx/mnemosyne/issues/568).
