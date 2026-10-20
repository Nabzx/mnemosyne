# ADR-0026: The Google ADK adapter

- Status: Accepted
- Date: 2026-10-20
- Issue: #568 (map); #571, #572, #573, #574, #576, #578 (research and decisions)

## Context

Research against ADK's own real `v2.12.0` source
(`docs/research/issue-569-adk-memory-seam-survey.md`,
`issue-570-adk-memory-call-patterns.md`) found Google's Agent
Development Kit's one pluggable memory seam,
`google.adk.memory.BaseMemoryService`, is wider than any adapter
built so far: an abstract `add_session_to_memory(session)`, optional
`add_events_to_memory(*, app_name, user_id, events, session_id=None,
custom_metadata=None)` and `add_memory(*, app_name, user_id, memories,
custom_metadata=None)`, and an abstract `search_memory(*, app_name,
user_id, query) -> SearchMemoryResponse`. `delete_memory` does not
exist in the released `2.12.0` package - confirmed real only on an
unreleased commit well ahead of the `v2.12.0` tag.

Two real call patterns exist in practice (#570): a manual batch call
after several turns, and a callback-driven per-turn auto-save. Both
need the caller to re-fetch the session first, since an in-memory
`Session` object can go stale. Memory itself is a corpus, not a
dictionary: `MemoryEntry.id` is optional and never caller-assigned in
any real sample.

`search_memory` is scoped by `(app_name, user_id)` only - it takes no
`session_id` (#571) - and the reference `InMemoryMemoryService` ranks
by real keyword overlap, not an unranked default (#573). `Event`
objects already carry everything needed for provenance directly on
the object (`author`, `invocation_id`, structured `function_call`/
`function_response` parts), unlike the OpenAI Agents SDK's need for an
`on_tool_end` capture hook (#572, ADR-0020). `google-adk` requires
Python `>=3.11`, one version above every other adapter package's own
`>=3.10` floor, confirmed by a real failed install on a `3.10` venv
(#576); its `mcp` extra resolves cleanly against `mnem-mcp`'s own pin.

## Decision

### Packaging

`packages/mnem-adk/`, PyPI `mnem-adk`, import `mnem_adk`, following
the same convention as the other three adapters. Pins `mnem-agents ~=
0.0`, versions independently. `requires-python = ">=3.11"` - a real,
stated exception to this project's otherwise-uniform `>=3.10` floor,
forced by `google-adk` itself (#576).

### Scope and node id scheme: a flat prefix, `Event.id` honoured literally (#571)

Scoped by `{app_name}:{user_id}:`, not a `/`-path: `search_memory`'s
own real signature takes no `session_id`, and ADK's own guide states
the scoping rule directly. CrewAI's own `/`-path scheme (ADR-0021)
does not fit here - it earns its syntax from tree-walk operations ADK
has none of.

- An ingested event: `{app_name}:{user_id}:{session_id}:{event.id}`.
- A direct `add_memory` write with its own id:
  `{app_name}:{user_id}:_memory:{memory_entry.id}`.
- A direct `add_memory` write with none:
  `{app_name}:{user_id}:_memory:{seq:010d}`, backed by a reserved
  `{app_name}:{user_id}:_memory:_seq` counter node, excluded from
  search and content iteration - the same idiom ADR-0020 and ADR-0023
  already use for items with no natural id.

### Storage granularity: one node per event (#578)

Matches the id scheme above directly - a per-session or
per-extracted-fact granularity would have needed a different key
shape from the start. Matches every other adapter's own discipline
(ADR-0020, ADR-0021, ADR-0023): pass storage items through literally,
one node per item, no semantic extraction step anywhere in this
ecosystem to point to.

### Provenance mapping: a pure field read, no capture hook (#572)

Unlike the OpenAI Agents SDK, which needed an `on_tool_end` hook
installed to capture tool provenance (ADR-0020), ADK's own `Event`
already carries everything needed directly on the object handed to
`add_events_to_memory`/`add_session_to_memory`:

| `Provenance` field | `Event` source |
| --- | --- |
| `source` | `Event.author` |
| `tool_call` | a `function_call` part's `name`/`args` |
| `observation` | a `function_response` part's `response` |
| `agent_step` | `Event.invocation_id` |
| `note` | left empty, an escape hatch |

A real naming collision to flag plainly, not map around quietly:
ADK's own `Event.branch` is a dotted sub-agent visibility path
("`agent_1.agent_2.agent_3`"), unrelated to mnem's own `Branch`. Both
this adapter's code and its README always write `event.branch`
explicitly, never bare "branch" in a context that could read as
mnem's.

### Search ranking: match `InMemoryMemoryService` exactly (#573)

Confirmed directly against the `v2.12.0` source, not assumed:
NFC-normalised `\w+` tokenising, Latin/non-Latin script-run widening,
a non-ASCII query token also matching by substring over the lowered
event text (CJK carries no word boundaries for token-set membership
alone to catch), zero-score events dropped, stable descending sort by
score, top 10 (`_MAX_SEARCH_RESULTS`). Neither prior precedent
transplants whole: AutoGen's `ListMemory` ignores its query argument
because it has no real ranking to match (ADR-0023); CrewAI's
`search()` takes a vector with no keyword to match against at all
(ADR-0021). ADK's own reference service has a real, specific ranking
algorithm, so matching it is the right call here, not degrading it.

### `delete_memory`: `mnem-adk`'s own extra method, built now (#574)

`google-adk` `2.12.0` has no `delete_memory` to match or defer to -
confirmed real only on an unreleased commit (#569). Rather than wait
on an upstream shape that may still change, `mnem-adk` exposes its own
`delete_memory`-shaped extra method now, the same way every other
adapter already exposes capability beyond its base contract (`why`,
`history`, `branch`, `switch`, `merge`, `bisect`). It is a tombstone
commit over the node's own id, never a hard delete - mnem's
cross-adapter principle regardless of what ADK eventually calls it.
If ADK later ships its own `delete_memory` with a different real
signature, this method can satisfy that interface too without
rework.

### Extra methods, outside `BaseMemoryService`

`why`, `history`, `branch`, `switch`, `merge`, `bisect` and
`delete_memory` (above) all live on the adapter class itself, outside
the ADK protocol - the same precedent ADR-0016 set for LangGraph and
every adapter since has followed.

### Errors

The same taxonomy as every other adapter (ADR-0016): `not_found`,
`invalid_ref`, `conflict`, `corrupt_store`, `no_store`, `store_io`,
`busy`.

### `add_session_to_memory`: idempotent, one commit per call

Re-ingesting the same `Session` must not duplicate nodes: `Event.id`
is the node id, so a repeat call lands on the same keys and a second
`add_events_to_memory` dedupes against existing ids the same way
`InMemoryMemoryService` already does. Retries on `ConflictError`, one
commit per call.

### The worked example

One script in `examples/`, alongside the other three: an ADK `Runner`
wired to a `MnemosyneMemoryService`, a scripted model in replay mode
(no key needed, offline in CI, the same discipline
`examples/claude_agent.py` already uses), demonstrating
`load_memory`/`preload_memory` composing on one agent, then
`blame`/`bisect` on the resulting history. A `--live` mode runs the
same script against the real demo model.

## Consequences

- A fourth-generation adapter package, independently versioned,
  alongside `mnem-mcp`, `mnem-langgraph`, `mnem-openai-agents`,
  `mnem-crewai` and `mnem-autogen`.
- The widest `BaseMemoryService` implementation that exists today:
  `add_session_to_memory`, `add_events_to_memory`, `add_memory` and
  `search_memory` all real, where only the managed
  `VertexAiMemoryBankService` currently implements all four.
- `packages/mnem-adk/pyproject.toml` carries a real, stated
  `requires-python = ">=3.11"` exception to this project's otherwise
  uniform `>=3.10` floor, and the adapters CI job installs it in the
  same sequence as the other adapter packages.
- `blame`/`bisect` resolve per event, same reasoning as every prior
  adapter.
- Callers get a real `delete_memory` ahead of ADK's own upstream
  method; if that upstream method's eventual real signature differs,
  this adapter's method may need a second, compatible overload later.

## Alternatives considered

**A `/`-path node id scheme, mirroring CrewAI (ADR-0021).** Rejected:
CrewAI's own scheme earns its syntax from real `list_scopes`/
`get_scope_info` tree-walk operations; ADK's `search_memory` has
nothing equivalent, so a flat `:`-joined prefix matching three of the
four existing adapters is the right default here, not CrewAI's
exception.

**Defer `delete_memory` until ADK ships its own.** Considered
seriously, rejected: `mnem`'s own store already has the tombstone
machinery to support this now at little cost, and waiting ships the
adapter with a real, visible capability gap other adapters do not
have, for an upstream timeline outside this project's control.

**A degraded or invented `search_memory` ranking.** Rejected: unlike
AutoGen's `ListMemory`, which has no real ranking to match, ADK's
reference service has a specific, real one, so degrading it would
answer a narrower question than the one the ecosystem's own default
already asks.

**One node per session, or per extracted fact.** Rejected: per-session
loses the per-turn granularity blame/bisect exist to provide;
per-extracted-fact needs an LLM extraction step this adapter would
have to own itself, with no existing adapter or research finding to
ground it.
