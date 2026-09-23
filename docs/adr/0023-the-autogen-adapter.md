# ADR-0023: The AutoGen adapter

- Status: Accepted
- Date: 2026-09-23
- Issue: #251 (research + grilling, one combined ticket)

## Context

Research (`docs/research/issue-251-autogen-survey.md`, read against the real
`microsoft/autogen` source) found AutoGen's pluggable memory primitive,
`Memory`, is the narrowest contract of the three adapters designed so far:
`MemoryContent` has no `id` field at all, and the bundled reference
implementation, `ListMemory`, **ignores its own `query()` argument entirely**
and returns every stored item chronologically - its own docstring says so
plainly. That is not a limitation to work around; it means a Mnemosyne-backed
`Memory` doing the same thing is the SDK's own real default behaviour, not a
degraded implementation, sidestepping the embedder question ADR-0021 had to
resolve carefully for CrewAI.

`update_context(model_context)` is a genuinely new integration shape: called
automatically before every model inference, it actively injects a message
into the conversation - push, not pull, unlike every adapter designed before
it. `add()` has no automatic call site anywhere in the codebase; every
example calls it directly from application code.

## Decision

### Packaging

`packages/mnem-autogen/`, PyPI `mnem-autogen`, import `mnem_autogen`,
following the same convention as the other three adapters. Pins
`mnem-agents ~= 0.0`, versions independently.

### Node id scheme: reuse ADR-0020's sequence-counter scheme directly

`{name:_seq}`-backed, zero-padded ids (`{name}:{seq:010d}`), `name` being the
`Memory` instance's own identifier (`ListMemory`'s existing `name` field).
No new id convention: nothing about `Memory`'s own model argues for a
different shape the way CrewAI's hierarchical scope paths did (ADR-0021).
Reusing an established pattern here is consistency with a real reason behind
it, not just tidiness for its own sake.

### Namespacing: one Mnemosyne store per `Memory` instance

Unlike LangGraph's `BaseStore` (one store, many namespaces, ADR-0016) or
CrewAI's globally-registered `StorageBackend` (ADR-0021), a `Memory`
instance in every AutoGen example is constructed directly and handed to one
agent - there is no established pattern of one `Memory` serving multiple
logical scopes. Closer in spirit to the OpenAI Agents SDK's `Session`
(ADR-0020) than to either of the shared-store adapters.

### `query()`: ignore the argument, return everything chronologically

Matches `ListMemory`'s own real behaviour exactly - not a degraded
implementation, the SDK's own actual default. No ranking, no filtering, no
embedder dependency anywhere in this adapter.

### `update_context()`: match `ListMemory`'s exact output shape

One `SystemMessage`, a numbered list of memory contents in chronological
order - byte-for-byte the same format `ListMemory` produces, so a caller
already familiar with the reference implementation's output sees no
surprise switching backends.

### `metadata`: fully opaque, no reserved-key convention

Stored and returned exactly as given, no interpretation, no `_meta`-style
carrier the adapter tries to lift into `Provenance`. Unlike the OpenAI
Agents SDK (`on_tool_end`, ADR-0020) or LangGraph (`_meta`, ADR-0016), there
is no automatic call site here that would make a reserved-key convention pay
off - `add()` is always explicit application code, so inventing a rule
nobody is forced to use just adds surface without a mechanism behind it.

### `clear()`: a tombstone commit, never a hard delete

`mnem.agents.forget` on every node under the instance's namespace, one
batched commit - the same reasoning ADR-0020 already settled for
`clear_session`: a routine `clear()` call must not silently defeat the whole
point of using Mnemosyne.

### Errors

The same taxonomy as every other adapter (ADR-0016): `not_found`,
`invalid_ref`, `conflict`, `corrupt_store`, `no_store`, `store_io`, `busy`.

### The worked example

One script in `examples/`, alongside the other three: an `AssistantAgent`
constructed with a `MnemosyneMemory` instance, a couple of turns adding
content via `add()`, `update_context` demonstrably injecting it before
inference, then `blame`/`bisect` on the resulting history.

## Consequences

- A fourth adapter package (alongside `mnem-mcp`, `mnem-langgraph`, and the
  designed-but-unbuilt `mnem-openai-agents`/`mnem-crewai`), independently
  versioned.
- The simplest adapter built so far in terms of what it has to reconcile -
  no embedder question, no tree-shaped scoping, no invented provenance
  carrier - because `Memory`'s own contract is the narrowest one studied.
- `blame`/`bisect` resolve per memory item, same reasoning as every prior
  adapter.
- `metadata`'s opaqueness means provenance is entirely the calling
  application's responsibility if it wants any - a real, stated limitation,
  not a silent gap.

## Alternatives considered

**Implement real ranking in `query()` (an adapter-side embedder, or a
keyword match over `content`).** Rejected: the reference implementation
itself does neither, so there is no real bar to clear here the way CrewAI's
`search()` set one. Inventing ranking capability nothing in the ecosystem
expects is scope this adapter doesn't need.

**A reserved-key convention for `metadata`, mirroring LangGraph's `_meta`.**
Rejected: `_meta` pays off because LangGraph callers are already shaping a
`value` dict for `put()`; here `add()` has no equivalent structure and no
automatic call site to make a convention discoverable or worth adopting.

**A shared store with `name` as a prefix, mirroring LangGraph/OpenAI Agents
SDK.** Rejected: no example or established pattern in AutoGen's own codebase
constructs one `Memory` to serve multiple logical scopes - one store per
instance matches how the SDK is actually used.

**A hard delete for `clear()`.** Rejected outright, not seriously
considered - same reasoning as every prior adapter's tombstone decision
(ADR-0020, ADR-0021): a routine SDK call must not be able to silently erase
history.
