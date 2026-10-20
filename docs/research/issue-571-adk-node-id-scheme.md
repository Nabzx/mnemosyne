# Research: node id scheme and scope mapping (issue #571)

Feeds ADR-0026. Scopes a node id convention for ADK's own real
`app_name`/`user_id`/`session_id`/`event_id` hierarchy, read against
all four existing adapters' own real precedent (ADR-0016, ADR-0020,
ADR-0021, ADR-0023) rather than inventing a fifth scheme from a blank
page.

## The real scoping key is flat, not a tree: `(app_name, user_id)`

Checked directly against `search_memory`'s own real signature (#569):
`search_memory(*, app_name, user_id, query)` - no `session_id`
parameter at all. `add_events_to_memory`'s own docstring confirms
`session_id` is optional and implementation-defined ("Implementations
may ignore `session_id` if it is not applicable"). ADK's own guide
states the scoping rule in plain language (#570): "Memory is scoped
by the `(app_name, user_id)` pair, so one user never sees another's
memories." There is no real tree-walk operation anywhere in
`BaseMemoryService` the way CrewAI's `list_scopes`/`get_scope_info`
are real one-level walks (ADR-0021) - `session_id` is a property of
where a memory *came from*, not a retrieval key ADK itself ever
searches by.

**This means**: the scope prefix should be flat, `:`-joined
(`{app_name}:{user_id}:`), matching LangGraph's, the OpenAI Agents
SDK's, and AutoGen's own precedent (ADR-0016/0020/0023) - not
CrewAI's `/`-delimited real hierarchy (ADR-0021), which earns its own
path syntax specifically because CrewAI has real nested-scope
operations to support. ADK has none, so fighting for path semantics
here would be convention for its own sake, the exact trap ADR-0021's
own "alternatives considered" section already named and rejected for
the opposite reason.

## A genuine hybrid: real ids exist for events, optional for direct writes

Two different situations, not one:

- **Events, via `add_session_to_memory`/`add_events_to_memory`**:
  `Event.id` is a real, always-present field (#569) - the same
  situation CrewAI's `MemoryRecord.id` put ADR-0021 in: a real id
  already exists, so honor it literally rather than inventing a
  parallel scheme. Node id: `{app_name}:{user_id}:{session_id}:
  {event.id}`.
- **Direct writes, via `add_memory`**: `MemoryEntry.id` is
  `Optional[str] = None` (#569), and #570's own research found no
  real guide or sample that ever populates it - the same situation
  the OpenAI Agents SDK's transcript items and AutoGen's
  `MemoryContent` put ADR-0020/ADR-0023 in: no real id to honor, so
  mnem-adk needs its own.

## Recommendation for ADR-0026

Reuse ADR-0020/ADR-0023's own reserved-sequence-counter idiom for the
one real gap (`add_memory` entries with no id), rather than inventing
a sixth idiom for a problem two adapters already solved:

- Scope prefix: `{app_name}:{user_id}:`, flat and `:`-joined.
- An ingested event's node id: `{app_name}:{user_id}:{session_id}:
  {event.id}` - the real id, honored literally.
- An `add_memory` entry that already carries `MemoryEntry.id`: `
  {app_name}:{user_id}:_memory:{memory_entry.id}` - still honored
  literally, placed under a reserved `_memory:` sub-prefix so a
  caller's own `session_id` can never collide with it.
- An `add_memory` entry with no id: `{app_name}:{user_id}:
  _memory:{seq:010d}`, backed by a reserved `{app_name}:{user_id}:
  _memory:_seq` counter node, zero-padded for the same reason
  ADR-0020 zero-pads - `BTreeMap` lexicographic order then matches
  insertion order with no scan needed.
- `_memory:_seq` is excluded from `search_memory` and any content
  iteration, the same treatment ADR-0020's `_seq` and ADR-0016's
  `_meta` already get as adapter-internal bookkeeping, not a
  real memory.

Related: #568 (map), #569 (the real field types this scheme is built
on), #570 (the scoping-by-pair finding this directly follows from),
#574 (the delete-capability decision, which will need this same
scope prefix to address a node by).
