# ADR-0020: The OpenAI Agents SDK adapter

- Status: Accepted
- Date: 2026-09-23
- Issue: #249 (research + grilling, one combined ticket)

## Context

Research (`docs/research/issue-249-openai-agents-survey.md`, read against the
real `openai/openai-agents-python` source) found the SDK's one pluggable
memory seam: a `Session` protocol -
`get_items(limit) / add_items(items) / pop_item() / clear_session()` - over a
**flat, ordered, position-addressed transcript** of raw API items (a message,
a tool call, a tool-call output, ...), passed straight into `Runner.run(...,
session=...)`. This is a real shape mismatch against Mnemosyne's keyed
`BTreeMap<String, MemoryNode>` node model: a transcript item has no natural
node id the way a LangGraph `(namespace, key)` pair or an MCP `remember`
call's explicit `id` does.

ADR-0016 already solved a structurally similar problem for LangGraph by not
forcing every foreign concept through the adopted interface: implement
`BaseStore` narrowly for drop-in compatibility, and put `branch` / `switch` /
`why` / `history` on the adapter class itself, outside the protocol. This ADR
follows that precedent for `Session`.

## Decision

### Packaging

`packages/mnem-openai-agents/`, PyPI `mnem-openai-agents`, import
`mnem_openai_agents`, following the packages actually shipped (not
ADR-0016's now-superseded `mnemosyne-*` naming - see `docs/adr/README.md`'s
naming note). Pins `mnem-agents ~= 0.0`, versions independently, tested
against the built wheel, same as `mnem-mcp` and `mnem-langgraph`.

### Namespacing: a shared store, sessions as an id prefix

One `Store` serves many sessions. A session's nodes live under the prefix
`{session_id}:`, mirroring the LangGraph adapter's
`":".join(namespace) + ":" + key` scheme exactly, rather than inventing a
second convention. `MnemosyneSession(store, session_id)` takes an already-open
`Store` (the caller owns the lifecycle, same as the LangGraph adapter's
`MnemosyneStore`), not a path - unlike `mnem-mcp`'s one-store-per-process
rule, which fits a stateless server, not an in-process SDK object serving
many concurrent conversations.

### Storage granularity: one node per transcript item

Every message, tool call, and tool-call output becomes its own node, not one
node per whole transcript. This is the one choice that makes the adapter
worth adopting over the SDK's own `SQLiteSession`: `blame` and `bisect`
resolve to *the message that introduced a belief*, not just "the commit that
touched the transcript." An `add_items([item1, item2, item3])` call (the SDK
batches a turn's message plus its tool call plus the tool's output in one
call) becomes **one commit adding three nodes**, mirroring
`mnem.agents.remember_many`'s existing batch form (ADR-0016) rather than
three separate commits - one write, however many nodes it touches.

Item content is the raw `TResponseInputItem` dict, stored as-is (Mnemosyne's
`MemoryNode` content is already opaque JSON, ADR-0003); nothing is
reinterpreted or restructured.

### Node id scheme: a reserved per-session sequence counter

Each item's id is `{session_id}:{seq:010d}`, `seq` a monotonically
increasing integer. Zero-padding keeps `BTreeMap` lexicographic order equal
to insertion order within a session, so `get_items` and `pop_item` are plain
keyed range reads, no full-namespace scan. The counter itself lives in a
reserved node, `{session_id}:_seq`, updated in the same commit as every
write - reusing this codebase's own precedent for an out-of-band reserved
key (LangGraph's `_meta`, ADR-0016) rather than a new idiom. `_seq` is
excluded from `get_items`, `pop_item`, and any content iteration; it is
adapter-internal state, not a transcript item.

### `get_items`, `pop_item`, `clear_session`

- `get_items(limit)`: read the keyed range under `{session_id}:` (excluding
  `_seq`), in id order; `limit` (or `session_settings.limit` if unset) takes
  the tail. `SessionSettings.limit` passes straight through - no Mnemosyne-
  side equivalent is needed, it is already a bounded keyed read.
- `pop_item()`: `mnem.agents.forget(store, highest_seq_id)` - a tombstone
  commit, not a hard delete. History survives; the item is gone from
  `working_memory`.
- `clear_session()`: one batched tombstone commit forgetting every node
  under `{session_id}:` (including resetting `_seq`), mirroring
  `remember_many`'s batch shape. Not a hard delete for the same reason:
  `SQLiteSession` callers call this as routine cleanup, not as a rare
  "actually erase this" operation, and a real delete would silently defeat
  Mnemosyne's whole point the first time an ordinary caller cleared a
  session.

### Provenance: `on_tool_end` auto-capture, on by default

`RunHooksBase.on_tool_end` receives the tool's name and output directly - a
carrier MCP (`remember`'s `source` argument) and LangGraph (`_meta`) each had
to invent by hand. `MnemosyneSession` installs an internal hook by default
that populates `Provenance.source` (the tool name) and `observation` (the
tool output) for any node written as a result of that call, no caller
action required. This is a deliberate departure from LangGraph's opt-in
`_meta` precedent: unlike `_meta`, which asks the caller to shape their own
data, this is purely additive metadata the adapter is already positioned to
fill for free, and defaulting it off would mean most callers never see it.
Documented plainly in the package README, not silent.

### Extra methods, outside `Session`

`branch(name)`, `switch(name)`, `merge(...)`, `why(item_id)`, `bisect(...)`,
`history(limit)` on the `MnemosyneSession` class directly, for code that
wants Mnemosyne's actual differentiators rather than just drop-in
compatibility - exactly ADR-0016's LangGraph pattern, not exposed through
`Session`'s four methods (which have no way to express a branch or a
conflict at all).

### Errors

The same taxonomy as every other adapter (ADR-0016): `not_found`,
`invalid_ref`, `conflict`, `corrupt_store`, `no_store`, `store_io`, `busy`
for write-lock contention, with the bounded retry-on-`ConflictError` wrapper
the SDK hardening work already added.

### The worked example

One script in `examples/`, alongside the SDK, LangGraph, and MCP examples
(ADR-0016): an agent using `Runner.run(agent, prompt, session=session)`
across a couple of turns including a tool call, then using `blame`/`bisect`
on the resulting history - demonstrating that the adapter is a drop-in
`Session` first, with Mnemosyne's differentiators one call away.

## Consequences

- A fourth adapter package, versioned and released independently, same
  operational shape as `mnem-mcp` and `mnem-langgraph`.
- `blame` and `bisect` stay meaningful per message, not just per API call -
  the reason to choose this adapter over `SQLiteSession` at all.
- A session's history carries a real per-item audit trail automatically via
  `on_tool_end`, with no caller code required, at the cost of the adapter
  making one behavioural choice (installing a hook) the caller did not
  explicitly request - documented, not hidden.
- `clear_session` is cheap only in the sense that it is one commit; the
  tombstoned nodes and the session's full prior history remain in the
  store, by design, and a caller who genuinely wants storage reclaimed has
  no path to that yet (same open question every other adapter already
  carries, not new here).
- The `_seq` reserved-key idiom is now used twice (LangGraph's `_meta`
  shares the spirit, not the exact mechanism) - worth a shared helper if a
  third adapter needs an analogous piece of internal bookkeeping.

## Alternatives considered

**One node per whole transcript, overwritten on every `add_items` call.**
Rejected: cheapest to build, but `blame`/`bisect` degrade to "this commit
changed the transcript," which quietly defeats the adapter's entire reason
to exist for the one framework where per-message provenance would be most
visible.

**Ordering via `Store::log` instead of a sortable node id.** Rejected: a
batched `add_items` call becomes one commit touching several nodes, and
`Store::log` gives commit order, not the relative order of nodes touched
within one commit - not a contract any current ADR guarantees. A sortable id
under a reserved counter is a few more lines and sidesteps the question
entirely.

**One Mnemosyne store per session (mirroring `mnem-mcp`'s one-store-per-
process rule).** Rejected: fits a stateless server spawned once per agent,
not an in-process SDK object a host application constructs many of, one per
conversation, often concurrently. The LangGraph adapter's namespace-as-prefix
precedent fits an in-process object far better.

**`on_tool_end` provenance capture opt-in, matching LangGraph's `_meta`.**
Rejected: `_meta` is opt-in because it asks the caller to shape their own
payload; here the adapter can populate real provenance with zero caller
effort, and opt-in would mean most integrations never turn it on and lose
the value for no real cost saved.

**A hard delete for `pop_item`/`clear_session`.** Rejected outright, not
seriously considered: it defeats Mnemosyne's core premise the moment an
ordinary `SQLiteSession`-shaped caller calls `clear_session()` as routine
cleanup, which is exactly when the audit trail matters most.
