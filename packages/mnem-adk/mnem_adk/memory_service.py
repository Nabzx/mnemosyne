"""``MnemosyneMemoryService``: a Google ADK ``BaseMemoryService`` over a
Mnemosyne store (#568, ADR-0026).

Scaffolding only (#586) - see the ADR for the locked node id scheme,
provenance mapping, search ranking and delete-capability decisions
each real method fills in as its own ticket lands.

An independent, third-party ADK memory service, not built or endorsed by
Google.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

import mnem
from google.adk.memory import BaseMemoryService
from google.adk.memory.base_memory_service import SearchMemoryResponse

if TYPE_CHECKING:
    from collections.abc import Mapping, Sequence

    from google.adk.events.event import Event
    from google.adk.memory.memory_entry import MemoryEntry
    from google.adk.sessions.session import Session


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
        raise NotImplementedError("filled in by #590")

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
