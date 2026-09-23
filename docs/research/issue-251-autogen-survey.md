# Research: an AutoGen adapter (issue #251)

Feeds the ADR that fixes a `mnem-autogen` adapter's contract, following the
same discipline as #249 and #250: read against the real
`microsoft/autogen` source (MIT, `python/packages/autogen-core`,
`autogen-core` v0.7.5 at research time), not its docs pages.

Two questions:

1. What is AutoGen's own pluggable memory primitive, and what shape does it
   require?
2. How is it actually wired into an agent's turn - who calls what, and when?

## 1. The contract: `Memory`, the narrowest of the three adapters so far

`autogen_core.memory.Memory` (`_base_memory.py`) is an ABC with five
methods: `update_context(model_context)`, `query(query: str |
MemoryContent, **kwargs)`, `add(content: MemoryContent)`, `clear()`,
`close()`. Narrower than CrewAI's eleven-method `StorageBackend` (#250) and
even narrower than the OpenAI Agents SDK's four-method `Session` (#249) in
one specific way: **`MemoryContent` has no `id` field at all** - just
`content` (`str | bytes | dict | Image`), `mime_type`, and an optional
`metadata` dict. There is no `get`, no `list`, no delete-by-id, no `pop` -
only wholesale `clear()`. Items are genuinely unkeyed.

**`query` takes a raw string, not a pre-computed embedding** - unlike
CrewAI's `search(query_embedding: list[float], ...)` (#250's central
finding). This matters: whatever computes a ranking here would need to
embed the string itself, since AutoGen's `Memory` doesn't hand over a
vector the way CrewAI's `Memory` class does.

## 2. The reference implementation is honestly trivial - and that's the answer

`ListMemory` (`_list_memory.py`), the bundled reference implementation, is
a plain Python list. Its `query()` **ignores its argument entirely** and
returns every stored item, docstring: "Return all memories without any
filtering... Args: query: Ignored in this implementation." Its
`update_context` dumps every memory as one numbered `SystemMessage`, in
chronological (insertion) order.

This directly answers the question CrewAI's research had to work hard for
(#250: "does a backend need its own embedder?"). Here, the SDK's own
bundled default doesn't attempt ranking at all - so a Mnemosyne-backed
`Memory` that also ignores `query` and returns everything chronologically
is not a degraded implementation, it is **the same real behaviour as the
reference implementation itself**, not a compromise invented to work around
a missing capability. `autogen_ext.memory` ships real ranked backends
(ChromaDB-, Redis-backed) for anyone who wants that; `Memory` as a protocol
does not require it.

## 3. `update_context` is a genuinely new integration shape

No adapter studied so far has this: `Memory.update_context(model_context)`
is called automatically by `AssistantAgent` before every model inference
(`_assistant_agent.py`, confirmed: `update_context_result = await
mem.update_context(model_context)` inside the agent's run loop) and
**actively mutates the passed context** by appending a message - the
memory implementation pushes itself into the conversation, rather than the
caller pulling data out and deciding what to do with it (MCP's tool call,
LangGraph's `get`, the OpenAI Agents SDK's `get_items`, CrewAI's `search`
are all pull; this is push). A Mnemosyne-backed implementation needs to
decide what it injects and in what format - `ListMemory`'s own choice
(a single numbered-list `SystemMessage`, chronological) is the natural
default to match, since it's what any caller already expecting a
`ListMemory`-shaped `Memory` would see.

## 4. `add()` is caller-driven only - no automatic hook exists

Unlike the OpenAI Agents SDK's `on_tool_end` (ADR-0020) or CrewAI's
`Memory` class auto-computing embeddings before `save()` (#250), nothing in
AutoGen calls `Memory.add()` automatically from a tool call or an agent
turn. Every example (including `AssistantAgent`'s own docstring) calls
`await memory.add(MemoryContent(...))` directly from application code. So
provenance has nowhere to travel except the optional `metadata` dict a
caller chooses to populate - closer to LangGraph's opt-in `_meta` precedent
(ADR-0016) than to either #249 or #250's adapters, since there is no hook
to make it automatic even if we wanted to.

## Recommendation

Implement `Memory` matching `ListMemory`'s own real semantics, not a lesser
version of something fancier: `query()` ignores its argument and returns
every item in the scoped memory, chronologically - honest, not degraded,
since it's what the SDK's own default does. `update_context` formats
exactly as `ListMemory` does (one numbered `SystemMessage`, chronological)
for drop-in familiarity. One Mnemosyne node per `MemoryContent` item (same
per-item-granularity reasoning as #249/#250 - `blame`/`bisect` should
resolve to *which memory item*), with a synthetic id scheme since
`MemoryContent` has none of its own. `clear()` as a tombstone commit via
the already-shipped `mnem.agents.forget`, never a hard delete - the fourth
adapter running into the exact same "routine `clear()` call must not
defeat the whole point" reasoning ADR-0020 already settled for
`clear_session`.

This is a recommendation, not a decision: the id scheme, namespacing, and
whether `metadata` provenance gets any adapter-side structure are real
forks for the grilling.

## Open questions for the grilling

- **Node id scheme**: `MemoryContent` has no id. A synthetic per-item
  sequence id (mirroring ADR-0020's `{name}:{seq:010d}` scheme, with
  `ListMemory`'s own `name` field as the natural namespace) is the obvious
  candidate - is there a reason to deviate from that established pattern
  here, given nothing about AutoGen's own model argues for a different
  shape the way CrewAI's `/`-scoped tree did?
- **Namespacing**: one Mnemosyne store per named `Memory` instance
  (mirroring `mnem-mcp`'s one-store-per-process rule), or a shared store
  with the `name` field as an id prefix (mirroring LangGraph/OpenAI Agents
  SDK)? A `Memory` instance is typically constructed once per agent, closer
  in spirit to `Session`'s per-`Runner.run` scoping than to a store an
  application shares across many conversations.
- **Does `metadata` get any adapter-side structure** (e.g. reserved keys
  the adapter lifts into `Provenance`, mirroring LangGraph's `_meta`), or
  is it stored and returned completely opaque, with no attempt to interpret
  it? There's no hook to populate it automatically either way, unlike
  #249's `on_tool_end` - this is purely about whether the adapter tries to
  be helpful with whatever a caller puts there.
- **`update_context`'s exact format**: match `ListMemory`'s plain numbered
  list precisely, or a documented Mnemosyne-flavoured variant (e.g.
  including provenance inline if it's present)? Byte-for-byte compatibility
  with `ListMemory`'s output isn't a real constraint (nothing parses it
  back), but staying close avoids surprising a caller who's used to it.
- **Package name and scope**: `packages/mnem-autogen/`, PyPI
  `mnem-autogen`, following the established naming convention.

## Sources

- `microsoft/autogen` (cloned at research time, MIT, `autogen-core` v0.7.5):
  `python/packages/autogen-core/src/autogen_core/memory/_base_memory.py`
  (the `Memory` ABC, `MemoryContent`), `_list_memory.py` (`ListMemory`'s
  real behaviour), `python/packages/autogen-agentchat/src/
  autogen_agentchat/agents/_assistant_agent.py` (`update_context` called
  automatically before inference; `memory.add()` only ever called from
  application code in every example).
- [ADR-0020](../adr/0020-the-openai-agents-sdk-adapter.md) (the per-item
  granularity and tombstone-not-delete precedents this research follows).
- [ADR-0021](../adr/0021-the-crewai-adapter.md) (the "does a backend need
  its own embedder" question this research resolves differently, and why).
