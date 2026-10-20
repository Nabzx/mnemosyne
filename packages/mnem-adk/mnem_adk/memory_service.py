"""``MnemosyneMemoryService``: a Google ADK ``BaseMemoryService`` over a
Mnemosyne store (#568, ADR-0026).

Scaffolding only (#586) - see the ADR for the locked node id scheme,
provenance mapping, search ranking and delete-capability decisions
each real method fills in as its own ticket lands.

An independent, third-party ADK memory service, not built or endorsed by
Google.
"""

from __future__ import annotations

import time
from typing import TYPE_CHECKING, Any

import mnem
from google.adk.memory import BaseMemoryService
from google.adk.memory.base_memory_service import SearchMemoryResponse

from ._convert import event_to_node

if TYPE_CHECKING:
    from collections.abc import Mapping, Sequence

    from google.adk.events.event import Event
    from google.adk.memory.memory_entry import MemoryEntry
    from google.adk.sessions.session import Session

_RETRIES = 4
_BACKOFF_MS = 25
_AUTHOR = "mnem-adk"


def _write_nodes(
    store: mnem.Store,
    nodes: list[tuple[str, dict[str, Any], mnem.Provenance]],
    *,
    message: str,
) -> None:
    """Stage every ``(node_id, content, provenance)`` in ``nodes`` and
    commit once - one write, however many nodes it touches (ADR-0016's own
    retry-on-conflict idiom). Node ids here are always deterministic (an
    event's own real id, or #588's explicit/sequence scheme), so a retry
    simply restages the same ops - unlike a sequence counter, there is
    nothing to re-read fresh."""
    if not nodes:
        return

    last: mnem.ConflictError | None = None
    for attempt in range(_RETRIES):
        try:
            for node_id, content, provenance in nodes:
                store.add(node_id, content, provenance=provenance)
            store.commit(message, author=_AUTHOR)
            return
        except mnem.ConflictError as exc:
            last = exc
            time.sleep(_BACKOFF_MS * (attempt + 1) / 1000)
    assert last is not None
    raise last


class MnemosyneMemoryService(BaseMemoryService):
    """A ``BaseMemoryService`` backed by a Mnemosyne store, shared across
    every ``(app_name, user_id)`` scope the host application uses (#571,
    ADR-0026) - the same shared-store shape the LangGraph and OpenAI Agents
    SDK adapters already use (ADR-0016, ADR-0020).

    Args:
        store: An already-open :class:`mnem.Store`. The caller owns the
            store's lifecycle.
    """

    def __init__(self, store: mnem.Store) -> None:
        self._store = store

    async def add_session_to_memory(self, session: Session) -> None:
        nodes = []
        for event in session.events:
            result = event_to_node(session.app_name, session.user_id, session.id, event)
            if result is not None:
                nodes.append(result)
        _write_nodes(self._store, nodes, message=f"ingest session {session.id}")

    async def add_events_to_memory(
        self,
        *,
        app_name: str,
        user_id: str,
        events: Sequence[Event],
        session_id: str | None = None,
        custom_metadata: Mapping[str, object] | None = None,
    ) -> None:
        raise NotImplementedError("filled in by #591")

    async def add_memory(
        self,
        *,
        app_name: str,
        user_id: str,
        memories: Sequence[MemoryEntry],
        custom_metadata: Mapping[str, object] | None = None,
    ) -> None:
        raise NotImplementedError("filled in by #592")

    async def search_memory(
        self, *, app_name: str, user_id: str, query: str
    ) -> SearchMemoryResponse:
        raise NotImplementedError("filled in by #593")
