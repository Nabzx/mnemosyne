"""Converting a real ADK ``Event`` into a mnem memory node's content and
``Provenance`` (#572, ADR-0026).

ADK's own ``Event`` already carries everything needed here directly on the
object - unlike the OpenAI Agents SDK, which needed an ``on_tool_end`` hook
installed to capture tool provenance (ADR-0020), this is a pure field read:

- ``Provenance.source`` <- ``Event.author``
- ``Provenance.tool_call`` <- a ``function_call`` part's ``name``/``args``
- ``Provenance.observation`` <- a ``function_response`` part's ``response``
- ``Provenance.agent_step`` <- ``Event.invocation_id``
- ``Provenance.note`` left empty, an escape hatch

The node's own ``content`` is the event's full ``Content`` object,
serialised - not reduced to its text, matching ``InMemoryMemoryService``'s
own real behaviour of returning ``MemoryEntry(content=event.content, ...)``
unchanged (checked directly against the ``v2.12.0`` source).

A real naming collision worth flagging plainly: ADK's own ``Event.branch``
is a dotted sub-agent visibility path ("agent_1.agent_2.agent_3"), unrelated
to mnem's own ``Branch``. Always write ``event.branch`` explicitly; never
bare "branch" in a context that could read as mnem's.
"""

from __future__ import annotations

import json
from typing import TYPE_CHECKING, Any

import mnem

from ._ids import event_node_id

if TYPE_CHECKING:
    from google.adk.events.event import Event
    from google.adk.memory.memory_entry import MemoryEntry


def _tool_call_and_observation(content: Any) -> tuple[str | None, str | None]:
    """The ``tool_call``/``observation`` provenance fields from a
    ``Content``'s own ``function_call``/``function_response`` parts, if
    any. Several parts of each kind join with "; "."""
    if content is None or not content.parts:
        return None, None

    calls = []
    observations = []
    for part in content.parts:
        if part.function_call is not None:
            calls.append(f"{part.function_call.name}({json.dumps(part.function_call.args)})")
        if part.function_response is not None:
            observations.append(json.dumps(part.function_response.response))

    tool_call = "; ".join(calls) if calls else None
    observation = "; ".join(observations) if observations else None
    return tool_call, observation


def event_to_node(
    app_name: str, user_id: str, session_id: str, event: Event
) -> tuple[str, dict[str, Any], mnem.Provenance] | None:
    """The ``(node_id, content, provenance)`` for ingesting ``event``, or
    ``None`` if it carries nothing to convert - matching
    ``InMemoryMemoryService``'s own real skip condition (no content, or no
    parts) exactly."""
    if event.content is None or not event.content.parts:
        return None

    tool_call, observation = _tool_call_and_observation(event.content)
    provenance = mnem.Provenance(
        source=event.author or None,
        tool_call=tool_call,
        observation=observation,
        agent_step=event.invocation_id or None,
    )
    node_id = event_node_id(app_name, user_id, session_id, event.id)
    content = event.content.model_dump(mode="json", exclude_none=True)
    return node_id, content, provenance


def memory_entry_to_content_and_provenance(
    memory: MemoryEntry,
) -> tuple[dict[str, Any], mnem.Provenance]:
    """The ``(content, provenance)`` for a direct ``add_memory`` entry -
    the same ``Content``-extraction core ``event_to_node`` uses, since a
    ``MemoryEntry``'s own ``content`` is the same real ``types.Content``
    shape. No ``agent_step``: a direct write has no ``invocation_id``
    concept the way an ingested ``Event`` does."""
    tool_call, observation = _tool_call_and_observation(memory.content)
    provenance = mnem.Provenance(
        source=memory.author or None,
        tool_call=tool_call,
        observation=observation,
    )
    content = memory.content.model_dump(mode="json", exclude_none=True)
    return content, provenance
