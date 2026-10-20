# Research: provenance mapping from ADK events (issue #572)

Feeds ADR-0026. Maps ADK's real `Event` fields onto mnem's real
`Provenance` struct (`agent_step`, `observation`, `tool_call`,
`source`, `note` - every field `Option<String>`, confirmed directly
in `crates/mnem-store/src/object.rs`), against the real
`google.genai.types.FunctionCall`/`FunctionResponse` shapes, pinned
to `google-genai` 2.31.0.

## No hook needed: ADK's own `Event` already carries what ADR-0020 needed a callback for

ADR-0020 had to install an internal `on_tool_end` hook for the OpenAI
Agents SDK, because `Session.add_items()` receives raw transcript
items with no structured way to tell which tool produced which item.
ADK's own `Event` does not have that gap: `author`, `invocation_id`,
and the structured `function_call`/`function_response` parts inside
`event.content.parts` are all already present on the object handed
to `add_events_to_memory`/`add_session_to_memory` directly - confirmed
in the real sample fetched for #570, which reads
`event.content.parts[0].function_call.name` and
`.function_response.response` straight off a plain event, no
callback involved. This is a simpler problem than ADR-0020's, not a
harder one: pure field mapping, no hook to install.

## The real field shapes

`google.genai.types.FunctionCall`: `id`, `name`, `args` (a
`dict[str, Any]`). `FunctionResponse`: `id`, `name`, `response` (a
`dict[str, Any]`, with an "output"/"error" key convention the
caller's own tool decides).

## The mapping

| mnem `Provenance` field | ADK source |
| --- | --- |
| `source` | `Event.author` - the agent, `"user"`, or (on a function response event) the tool's own name |
| `tool_call` | On a `function_call` part: `FunctionCall.name`, with `args` if it fits as a string - "what was called, and with what" |
| `observation` | On a `function_response` part: `FunctionResponse.response` - "what came back" |
| `agent_step` | `Event.invocation_id` - the closest real ADK concept to "which step of the run produced this"; every event sharing one `invocation_id` came from one agent turn |
| `note` | Left empty by default; a real escape hatch for anything the four fields above do not capture |

## A real naming collision worth flagging plainly, not just mapping around

`Event.branch` is real (confirmed in #569's own field list) and its
real docstring says something structurally unrelated to mnem's own
`Branch`: "The format is like `agent_1.agent_2.agent_3`... used when
multiple sub-agent shouldn't see their peer agents' conversation
history." ADK's `branch` is a dotted sub-agent visibility path;
mnem's `Branch` is a version-control ref. Same word, two unrelated
concepts. Per `docs/agents/domain.md`'s own instruction to flag a
term collision openly rather than let it drift: `mnem-adk`'s own
code and docs should never write "branch" unqualified when
`event.branch` is meant - always `event.branch` or "ADK's sub-agent
branch", never bare "branch" in a context that could read as mnem's.

## Recommendation for ADR-0026

1. Lock the five-row mapping table above.
2. State the naming-collision rule explicitly in the ADR and in
   `mnem-adk`'s own README - the same kind of plain disclosure
   ADR-0016 already gave `_meta` and ADR-0020 gave `_seq`.
3. No hook, no callback, no new mnem-adk-internal state needed for
   provenance capture - it is a pure, synchronous field read on the
   `Event` object already in hand at write time.

Related: #568 (map), #569 (the real `Event`/`Provenance` field
types this mapping is built from), #571 (the node id scheme this
mapping's node writes will be stored under).
