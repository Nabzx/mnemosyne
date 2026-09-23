# Research: an OpenAI Agents SDK adapter (issue #249)

Feeds the ADR that fixes `mnem-openai-agents`'s contract, the same way
issue #55 fed ADR-0016 for `mnem-mcp` and `mnem-langgraph`. Read against the
real SDK source (`openai/openai-agents-python`, MIT, PyPI `openai-agents`,
v0.22.3 at research time), not its docs pages, matching the discipline
`docs/comparisons.md` (#237) used for Memoria and ByteRover.

Two questions:

1. What is the SDK's own pluggable memory primitive, and what shape does it
   require?
2. Does that shape match Mnemosyne's node model well enough to be a thin
   wrapper, the way `BaseStore` did for LangGraph - or does it need a
   different answer?

## 1. The SDK's memory primitive: `Session`

`src/agents/memory/session.py` defines a `Protocol` (and an `ABC` twin for
internal use) with exactly four methods:

```python
class Session(Protocol):
    session_id: str
    session_settings: SessionSettings | None

    async def get_items(self, limit: int | None = None) -> list[TResponseInputItem]: ...
    async def add_items(self, items: list[TResponseInputItem]) -> None: ...
    async def pop_item(self) -> TResponseInputItem | None: ...
    async def clear_session(self) -> None: ...
```

This is a first-class parameter on the main run loop, not a side extension:
`Runner.run(agent, prompt, session=session)` (`src/agents/run.py`, every
`run` / `run_sync` / `run_streamed` overload). Passing a `Session` is how an
agent gets automatic conversation-history management at all; the SDK ships
one reference implementation, `SQLiteSession` (`:memory:` by default, or a
file path), and a compaction-wrapper `Session` for long-running
conversations. This is the exact role `BaseStore` plays for LangGraph and the
tool surface plays for MCP - the one seam a third-party memory backend is
meant to fill.

**The item type.** `TResponseInputItem = ResponseInputItemParam`, a type
alias into OpenAI's Responses API - a tagged union covering a chat message,
a function (tool) call, a function-call output, a reasoning item, an MCP
list-tools/approval item, and a handful of others (`src/agents/items.py`).
It is a **raw, ordered transcript entry**, not a belief with a stable
identity. Nothing in the type carries an id of its own; identity is
positional.

**Session's shape is a flat, ordered, position-addressed list.**
`get_items(limit)` returns the last N items in chronological order.
`add_items` appends. `pop_item` removes and returns the single most recent
item. `clear_session` empties it. There is no `key`, no namespace, no
concept of "this entry versus that entry" beyond order - it is closer to a
JSONL append log (the exact thing `docs/benchmark.md`'s Baseline B
represents) than to a keyed store.

**`session_settings`.** One field today, `limit: int | None`, resolved
per-call or from the session's default. Room for more later
(`SessionSettings` is a `pydantic.dataclasses.dataclass`), but nothing else
is defined yet.

## 2. The shape mismatch, and how the LangGraph adapter's precedent answers it

Mnemosyne's node model is a **keyed** map: `id -> MemoryNode`, read as
`working_memory() -> BTreeMap<String, MemoryNode>` (`checkout.rs`), each
write a `commit`. `BTreeMap` ordering is lexicographic by id string, not
insertion order - `"item:10"` sorts before `"item:2"`. The one place
Mnemosyne stores things in a true chronological order is the commit graph
itself: `Store::log(start, first_parent, limit)` walks commits **newest
first**, deterministic by record time then id (`log.rs`).

So a `Session` item has no natural Mnemosyne node id (an OpenAI message or
tool call is not "a belief with a stable identity" the way a LangGraph
`(namespace, key)` pair is), and Session's list semantics (`pop_item`,
positional `get_items(limit)`) map onto Mnemosyne's *commit history*, not
onto its *keyed node space*, the reverse of how the LangGraph adapter works.

ADR-0016 already answered a structurally similar problem for LangGraph by
**not** forcing every LangGraph concept through `BaseStore` alone: `BaseStore`
is implemented for drop-in compatibility, and `branch` / `switch` / `why` /
`history` are extra methods on the same adapter class, outside the
interface, for the differentiator surface. That precedent - implement the
framework's protocol narrowly and put Mnemosyne's actual value (branch,
merge, blame, bisect) on the adapter object itself, not squeezed through the
foreign interface - is the strongest candidate answer here too, since
`Session`'s four methods have no way to express a branch, a conflict, or a
`blame` result at all.

**Two live design questions this doesn't settle**, both real forks with
different downstream code:

- **What is a Session item's Mnemosyne node id?** A synthetic, sortable key
  (e.g. a zero-padded or numeric-suffixed `id`, since `BTreeMap` ordering is
  lexicographic and `"item:10"` would otherwise sort before `"item:2"`) makes
  each transcript entry its own node - individually blameable, at the cost of
  one node per turn in a possibly long-running conversation. Storing the
  whole item list as **one** node's content (one commit per `add_items` call)
  is far cheaper and simpler, but then `blame`/`bisect` only resolve to "this
  commit changed the transcript," not to a specific message - a real loss of
  the thing that makes Mnemosyne worth adopting over `SQLiteSession` at all.
- **Does `get_items`/`pop_item` read from the keyed node space (via a
  synthetic id scheme) or from `Store::log` (walking commits, since that is
  Mnemosyne's one truly-ordered primitive)?** These are different
  implementations with different failure modes - `log` is already ordered
  and paginated (`limit`) for free, but conflates "one commit" with "one
  Session item" in a way that may not survive `remember_many`-style batching
  or a future merge into the session's history.

## 3. A second seam: `RunHooks` / `AgentHooks`, for provenance

`src/agents/lifecycle.py` defines `RunHooksBase` and `AgentHooksBase`, async
callback classes an agent or a `Runner.run` call can be given, covering
`on_llm_start/end`, `on_agent_start/end`, `on_handoff`, and - the relevant
pair - **`on_tool_start`** / **`on_tool_end`**. This is a second, independent
integration point, orthogonal to `Session`: MCP's precedent
(`remember`'s `source` argument) and LangGraph's precedent (provenance
lifted from a reserved `_meta` key, ADR-0016) both had to invent a carrier
for provenance because their host interfaces don't have one either. Here,
`on_tool_end` receives the tool's name and output directly, which is a
cleaner, unprompted way to populate `Provenance.source` / `observation`
automatically for any node written as a result of that tool call - worth
deciding whether the adapter wires this by default or leaves it to the
caller, the same kind of call ADR-0016 made explicit for `_meta`.

## Recommendation

Implement `Session` for drop-in compatibility (so `Runner.run(agent, prompt,
session=MnemosyneSession(store))` works with zero other changes to an
existing OpenAI Agents SDK program), backed by a synthetic per-item node id
scheme rather than one node per whole transcript, so `blame` and `bisect`
stay meaningful per message - matching `packages/mnem-mcp`'s existing
precedent of "one write, one commit" rather than batching for throughput.
Put `branch` / `switch` / `merge` / `why` / `bisect` on the adapter class
directly, outside the `Session` protocol, exactly as ADR-0016 did for
`BaseStore`. Wire `on_tool_end` as an optional `AgentHooks` subclass the
adapter ships, off by default, that auto-populates provenance - documented,
not silent, matching the `_meta` precedent.

This is a recommendation, not a decision: the id-scheme and ordering-source
questions in section 2, and the hooks default in section 3, are exactly the
kind of fork a grilling should pressure-test rather than settle unilaterally
in a research doc.

## Open questions for the grilling

- **Node id scheme for a `Session` item**: synthetic per-item id (one node
  per message/tool-call/tool-output) versus one node per `add_items` batch.
  If per-item, what does the id look like, and does it survive
  `pop_item` cleanly (does popping tombstone the node, or does it need a
  real delete)?
- **Ordering source**: reconstruct `get_items` from the keyed node space (via
  the id scheme) or from `Store::log`? Does `clear_session` mean "tombstone
  every node in this session's namespace" or "branch to a fresh empty
  history"?
- **Namespacing**: one Mnemosyne store per `session_id` (mirroring
  `mnem-mcp`'s one-store-per-process rule), or one store with an id-prefix
  namespace per session (mirroring the LangGraph adapter's
  namespace-as-prefix)?
- **Hooks default**: does the adapter's `AgentHooks` provenance auto-capture
  ship on by default, or opt-in? MCP made provenance a first-class tool
  argument; LangGraph made it an opt-in `_meta` key. Which precedent does
  this adapter follow?
- **Package name and scope**: `packages/mnem-openai-agents/`, PyPI name, and
  whether `SessionSettings.limit` needs any Mnemosyne-side equivalent (a
  bounded `get_items` is a paginated `log`/`working_memory` read, not a new
  primitive) or is just passed through.

## Sources

- `openai/openai-agents-python` (cloned at research time, MIT, PyPI
  `openai-agents` v0.22.3): `src/agents/memory/session.py`,
  `sqlite_session.py`, `session_settings.py`, `src/agents/items.py`,
  `src/agents/lifecycle.py`, `src/agents/run.py`, `examples/memory/*.py`.
- [ADR-0016](../adr/0016-mcp-tools-and-the-adapter-contract.md) (the MCP/
  LangGraph adapter contract precedent this research follows).
- `crates/mnem-store/src/checkout.rs` (`working_memory`'s `BTreeMap`
  ordering), `crates/mnem-store/src/log.rs` (`Store::log`'s chronological
  order).
