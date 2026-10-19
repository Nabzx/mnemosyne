# Research: how ADK apps call the memory service in practice (issue #570)

Feeds ADR-0026. Read against ADK's own real documentation
(`docs/guides/memory/memory_service/index.md`) and a real worked
sample (`contributing/samples/context_management/memory/`), both
pinned to the `v2.12.0` tag, not from a secondhand summary.

## Two real call patterns, not one

**Manual, explicit, after a batch of turns.** The real sample
(`main.py`) runs several turns on a session, then calls
`await runner.memory_service.add_session_to_memory(session_1)` once,
by hand, after the whole conversation is judged complete - not after
every turn. This is the pattern an application controls directly.

**Callback-driven auto-save, documented but not shown as a full
sample.** ADK's own guide recommends attaching a callback (an
`after_agent_callback`) that calls `callback_context.
add_session_to_memory()` (no arguments - `Context` already knows which
session it is scoped to), so each turn is ingested as it finishes
"rather than at some later point you have to remember to trigger."
Worth being precise about the evidence here: this is a real,
documented recommendation with a real code snippet in the guide
itself, not something found wired up end-to-end in a runnable sample
file the way the manual pattern is - the guide presents it as the
fix for "I forgot to call add_session_to_memory", not as a pattern
with its own worked example.

## A real, load-bearing detail: re-fetch before ingesting

Both the guide and the real sample agree on something easy to miss:
the session object a caller already holds may be stale by the time
ingestion happens. The guide states it directly ("Re-read it first
so the ingested copy has the final events"), and the sample's own
`run_prompt` helper returns `await runner.session_service.
get_session(...)` after every turn specifically so the caller always
holds the freshest copy before the eventual
`add_session_to_memory` call. **Any offline spike or test
(#584, #596) that constructs a `Session` once and reuses that same
in-memory object across multiple turns, then calls
`add_session_to_memory` on it, is testing a scenario the real
ecosystem does not actually rely on.** The test should re-fetch via
`session_service.get_session(...)` first, matching real usage.

## Ingestion is opt-in, confirmed directly

The guide states plainly: "Sessions do not reach memory on their
own. If no one calls `add_session_to_memory`, memory stays empty."
No autosave exists at the framework level; every real call pattern
above is something an application author wires up deliberately, by
hand or by callback. There's no third, implicit path.

## Memory is a corpus, not a dictionary: bears on node-id scope (#571)

The guide draws a real distinction worth carrying into the node-id
research: session *state* is a dictionary the caller addresses by a
chosen key, but memory is "a corpus, not a dictionary" - a caller
cannot read an entry back by name, only search it. `MemoryEntry.id`
exists but is optional and never caller-assigned anywhere in the
real guide or sample; nothing in ADK itself expects a stable,
caller-chosen memory id the way `mnem`'s own node id is. This is
good news for #571's own job: ADK imposes no competing id
convention mnem-adk's own scheme would have to fight.

## The two memory tools really do compose

The real sample's `agent.py` builds one agent with both
`load_memory_tool` and `preload_memory_tool` in its `tools` list at
once, confirming the guide's own claim that they compose rather than
being mutually exclusive choices: `preload_memory` covers the common
case automatically (injecting results into a
`<PAST_CONVERSATIONS>` block on every request, no tool-call round
trip, but every request pays for a search), and `load_memory` lets
the model dig further on demand, costing a real tool call only when
the model decides to use it.

## `VertexAiMemoryBankService` is the only built-in service implementing all three write methods

`InMemoryMemoryService` implements only `add_session_to_memory` and
`search_memory` - `add_events_to_memory` and `add_memory` both raise
`NotImplementedError` by default and `InMemoryMemoryService` never
overrides them. Of ADK's own three built-in services, only the
managed `VertexAiMemoryBankService` implements every one of
`add_session_to_memory`, `add_events_to_memory`, `add_memory`,
`search_memory`. If `mnem-adk` implements all three write paths plus
search (as scoped), it becomes the most complete *open-source*
`BaseMemoryService` implementation that exists today - a real,
checkable positioning claim for the README (#599) and any naming
decision (#580), not a marketing one.

## Recommendation for ADR-0026

1. Support both real call patterns - the manual batch call and the
   callback-driven per-turn call - since both are real, documented
   ADK usage, not just test against one.
2. Every offline spike, example, and integration test must re-fetch
   the session via `session_service.get_session(...)` before calling
   `add_session_to_memory`, matching real usage exactly, not a
   same-object-reused shortcut.
3. No id convention in ADK itself competes with mnem-adk's own node
   id scheme; #571 has a clear field to design in.
4. The "most complete open-source `BaseMemoryService`" framing is a
   real, verifiable claim worth using in the README and the launch
   posts, distinct from and stronger than naming Gemini or Argon.

Related: #568 (map), #569 (the base contract this call-pattern
research assumes), #571 (node id scheme, informed directly by the
memory-is-a-corpus finding above).
