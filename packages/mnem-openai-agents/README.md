# mnem-openai-agents

An [OpenAI Agents SDK](https://openai.github.io/openai-agents-python/) `Session`
backed by [Mnemosyne](https://github.com/Nabzx/mnemosyne). A drop-in
replacement for `SQLiteSession`, with real history underneath: every write is
a commit, so a tool-calling run's transcript can be blamed, bisected, or
branched later - not just replayed.

```python
import mnem
from mnem_openai_agents import MnemosyneSession

store = mnem.open("./agent-memory")     # the caller owns this handle
session = MnemosyneSession(store, "u1")  # scoped by an id prefix, not a store per session

await Runner.run(agent, "what's the status of order 4821?", session=session)
items = await session.get_items()       # every message, tool call, and tool output - one node each
```

## Provenance auto-capture

Pass `hooks=session.hooks` to the same `Runner.run` call to turn on
provenance auto-capture: every node written as a result of a tool call gets
a real `Provenance.source` (the tool's name) and `observation` (its output)
for free, no per-call payload to shape by hand.

```python
await Runner.run(agent, prompt, session=session, hooks=session.hooks)
```

This is on by default in the sense that `session.hooks` is always ready to
use - the SDK itself has no way for a `Session` to register a hook on a run
just by being passed as `session=`, so `hooks=` and `session=` stay
independent parameters on `Runner.run` and this one line is the real
integration point. A run with no `hooks=` still reads and writes normally,
just without that provenance - there is nothing to opt out of beyond simply
not passing it.

## Mapping

- **One node per transcript item**, not one per whole transcript - the
  reason `blame`/`bisect` stay meaningful at the level of the message that
  introduced something, not just "the commit that touched the transcript"
  (ADR-0020).
- **Node id**: `{session_id}:{seq:010d}`, a zero-padded, monotonically
  increasing per-session sequence number, so id order equals write order. A
  reserved `{session_id}:_seq` node holds the counter; it never appears in
  `get_items` or any other read.
- **Content**: the raw `TResponseInputItem` dict, stored as-is - no
  reinterpretation.
- **`pop_item` / `clear_session`**: tombstone commits, never a hard delete -
  the item or session is gone from a live read but still recoverable through
  the store's ordinary history.

## Extra methods

Not on the `Session` protocol, for code that wants Mnemosyne's own
differentiators: `branch(name)`, `switch(target)`, `merge(theirs, ...)`,
`why(item_id)`, `bisect(predicate)`, `history(limit=...)`. All but `why`
are **store-wide**, not scoped to one session's own items - branching or
merging moves the whole store, since ADR-0020's shared-store design means
other sessions may share it.

```python
session.branch("hypothesis")
session.switch("hypothesis")
await session.add_items([...])          # a divergent line of items
session.switch("main")

result = session.merge("hypothesis")     # status "conflicts" if any collide
if not result.ok:
    session.merge("hypothesis", resolutions={c.id: "theirs" for c in result.conflicts})

session.why(item_id)                     # -> Blame: which commit, what provenance
session.bisect(lambda memory: item_id in memory)
```

See [ADR-0020](https://github.com/Nabzx/mnemosyne/blob/main/docs/adr/0020-the-openai-agents-sdk-adapter.md)
and epic [#232](https://github.com/Nabzx/mnemosyne/issues/232). See
[`examples/openai_agents_memory.py`](https://github.com/Nabzx/mnemosyne/blob/main/examples/openai_agents_memory.py)
for a worked example: a real `Runner.run` tool-calling turn, provenance
auto-capture, then `why`/`bisect` tracing the answer back to the exact
tool call.

## Sharing a store with other adapters

This adapter's own node ids are `{session_id}:{seq:010d}`, with a
reserved `{session_id}:_seq` counter node. **This scheme is
byte-for-byte identical to the AutoGen adapter's own** (ADR-0023 reused
ADR-0020's scheme directly) - sharing a store with `mnem-autogen` using
the same identifier string for a `session_id` and a `name` is not a
risk, it is a **guaranteed** collision on every write, confirmed
directly, not assumed
([#362](https://github.com/Nabzx/mnemosyne/issues/362)). Always give
every `session_id` a value distinct from any `name` in use by an
AutoGen `Memory` sharing the same store. See the
[concurrency model](https://github.com/Nabzx/mnemosyne/blob/main/docs/concurrency-model.md)
for when a shared store can safely be opened at all.
