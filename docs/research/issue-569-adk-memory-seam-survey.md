# Research: ADK's memory seam against the real source (issue #569)

Feeds ADR-0026. Read directly against `google/adk-python`'s real
source, pinned to the `v2.12.0` tag (the version actually published to
PyPI on 2026-10-13 and the version in the project's own `.venv`), not
from documentation or from memory. Three real corrections to the map's
own "Decisions so far" follow; everything else checked out exactly as
stated.

## What the map got right

- `MemoryEntry` (`src/google/adk/memory/memory_entry.py`): `content`
  (`google.genai.types.Content`), `custom_metadata`
  (`dict[str, Any]`), `id` (`Optional[str]`), `author`
  (`Optional[str]`), `timestamp` (`Optional[str]`, ISO 8601
  preferred). Exact match.
- `Session` (`src/google/adk/sessions/session.py`): `id`, `app_name`,
  `user_id`, `state`, `events`, `last_update_time`. Exact match (plus
  a private `_storage_update_marker`, internal, irrelevant here).
- `Event` fields `id`, `author`, `invocation_id`, `timestamp` (a
  `float`, defaulted via `platform_time.get_time()`), `content`,
  `branch`, `partial` all real, confirmed in
  `src/google/adk/events/event.py`.
- `add_session_to_memory(session)` is abstract, and its docstring says
  "A session may be added multiple times during its lifetime",
  verbatim.
- `add_events_to_memory(*, app_name, user_id, events, session_id=None,
  custom_metadata=None)` is real, not abstract: it raises
  `NotImplementedError` by default, with a message pointing the
  caller back to `add_session_to_memory`. Its own docstring states
  plainly that `events` is a delta ("intended for cases where callers
  want to persist only a subset of events... Implementations should
  treat `events` as an incremental update (delta) and must not assume
  it represents the full session").
- `add_memory(*, app_name, user_id, memories, custom_metadata=None)`
  is real, not abstract, same `NotImplementedError`-by-default shape.
- `search_memory(*, app_name, user_id, query)` is abstract, returns
  `SearchMemoryResponse(memories: list[MemoryEntry])`. Exact match.
- `InMemoryMemoryService`'s real ranking
  (`src/google/adk/memory/in_memory_memory_service.py`): top 10
  (`_MAX_SEARCH_RESULTS = 10`) by word-overlap count, a stable sort
  (the comparator reads only the count, so ties keep insertion order),
  and a real Latin/non-Latin token split (`_extract_searchable_words`
  additionally splits a mixed-script token into single-script runs
  via `itertools.groupby(word, _is_latin)`, specifically so an
  embedded Latin word inside unspaced CJK text, the docstring's own
  example, matches). It skips events with no content parts, checked
  twice (on add and on search).
- `load_memory` (`src/google/adk/tools/load_memory_tool.py`) is a
  real `FunctionTool` wrapping `tool_context.search_memory(query)`.
  `preload_memory` (`preload_memory_tool.py`) is a real `BaseTool`
  that overrides `process_llm_request` and runs automatically before
  every LLM call, never exposed to the model as a callable tool -
  both exactly as the map described.

## Three real corrections

### 1. `delete_memory` does not exist in the released 2.12.0 package

The map lists `delete_memory(*, app_name, user_id, memory_id=None)`
as a real `BaseMemoryService` method. It is real code, but not in
2.12.0: a GitHub code search for `delete_memory` in
`google/adk-python` hits a commit (`557743c`) on the default branch
with the exact signature and docstring the map describes, but
`git compare v2.12.0...557743c` reports `"status": "diverged"`,
`ahead_by: 132`. Fetching `base_memory_service.py` at the literal
`v2.12.0` tag (not the default branch) confirms it directly: no
`delete_memory` anywhere in the file. PyPI's own `google-adk` JSON API
confirms `2.12.0`, uploaded 2026-10-13T16:42:55, is still the latest
release - no `2.12.1` or `2.13.0` exists yet to have shipped it either.

**This means**: `delete_memory` is a real, upcoming method, not yet
in anything installable today. Building against it now is building
against unreleased, unversioned API surface that could still change
shape before it ships for real.

### 2. The "keeps deleted ids" claim is not true of 2.12.0, because there is no delete to defend against

Follows directly from #1. With no `delete_memory` anywhere in 2.12.0,
`InMemoryMemoryService` has no delete operation at all - nothing is
ever removed from memory once added, so there is nothing for a
tombstone or an id-exclusion rule to protect. The id-tracking the map
pointed to (`existing_ids = {event.id for event in existing_events}`
inside `add_events_to_memory`) is real code, but it does a different
job: it stops the *same* event id from being appended twice to the
same session's delta list on a repeated call - plain deduplication of
new writes, not protection against a deleted id resurrecting. There is
no code path in 2.12.0 where a previously-removed id is re-excluded,
because there is no removal.

**This means**: ticket #6's own premise, "confirm how
`InMemoryMemoryService` keeps a tombstoned id from resurrecting", has
no real behaviour to confirm in 2.12.0 - the reference implementation
doesn't do this because it has nothing that deletes. `mnem-adk`'s own
tombstone-not-delete convention (matching ADR-0020/ADR-0023's own
precedent) can still be built as an addition beyond what `2.12.0`
itself defines, the same way the other adapters' "extra methods" sit
outside the adopted interface - just not as matching an existing
reference behaviour, because none exists yet.

### 3. `Runner(...)` is keyword-only, not positional

The map's shorthand, `Runner(app_name, agent, session_service,
memory_service)`, reads as positional construction. The real
constructor (`src/google/adk/runners.py`) is `def __init__(self, *,
app=None, app_name=None, agent=None, node=None, plugins=None,
artifact_service=None, session_service, memory_service=None,
credential_service=None, plugin_close_timeout=5.0,
auto_create_session=False)` - the bare `*` right after `self` makes
every one of those keyword-only. `Runner(app_name, agent,
session_service, memory_service)` would raise a `TypeError` outright;
it has to be `Runner(app_name=..., agent=..., session_service=...,
memory_service=...)`. Also worth carrying forward: the constructor's
own docstring recommends passing a real `App` instance over a bare
`agent`, calling that "the recommended way to create a runner" -
`agent`/`node` get wrapped into an `App` internally either way.

## Recommendation for ADR-0026

1. Do not implement `delete_memory` against `BaseMemoryService` as an
   override in this round - it isn't a real override of anything
   2.12.0 defines. If `mnem-adk` wants a delete capability now, build
   it as an extra method on the adapter class, outside
   `BaseMemoryService`, exactly the shape ADR-0016/ADR-0020/ADR-0023
   already use for `branch`/`switch`/`why`/`bisect`. Revisit a real
   override once a `google-adk` release actually ships the method.
2. Ticket #6's own job needs rescoping: there is no real
   `InMemoryMemoryService` behaviour to match for "keeping a deleted
   memory deleted", because 2.12.0 has no delete path at all. The
   grilling question becomes "does `mnem-adk` want a delete capability
   ahead of ADK itself shipping one", not "how does the reference
   implementation keep a deletion from resurrecting."
3. Every other construction detail, Runner included, needs the
   keyword-only form in every example and spike from here on.

Related: #568 (map), #571-574 (downstream research, unaffected by
this), #578 (granularity, unaffected), #584 (the offline spike, must
use `Runner(...)` with keywords).
