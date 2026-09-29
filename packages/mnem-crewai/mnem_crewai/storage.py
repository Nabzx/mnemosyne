"""``MnemosyneStorageBackend``: a CrewAI ``StorageBackend`` over a Mnemosyne
store (#250, ADR-0021).

Scaffolding only (#310) - every method below raises ``NotImplementedError``.
Real behaviour lands one ticket at a time:

- the node id scheme and scope tree-walk: #311
- ``save`` / ``get_record`` / ``list_records``: #312
- ``delete`` / ``update`` / ``count`` / ``list_categories`` / ``reset``: #313
- ``search`` via real cosine similarity: #314
- ``asave`` / ``asearch`` / ``adelete``: #315
- a worked example: #316

Once complete, this class implements CrewAI's ``StorageBackend`` protocol
structurally (it is a ``Protocol``, not an ABC - no explicit subclassing
needed), so ``set_memory_storage_factory(lambda spec: MnemosyneStorageBackend(
"./agent-memory"))`` registers it as the process-wide default (a one-time,
process-wide setter, not a per-``Memory``-instance argument - CrewAI's own
factory contract, not something this adapter can make more granular).
"""

from __future__ import annotations

from typing import TYPE_CHECKING

import mnem

if TYPE_CHECKING:
    from datetime import datetime
    from typing import Any

    from crewai.memory.types import MemoryRecord, ScopeInfo


class MnemosyneStorageBackend:
    """A ``StorageBackend`` backed by a Mnemosyne store.

    Args:
        path: Where the store lives. Opened with :func:`mnem.open`, matching
            the LangGraph adapter's ``MnemosyneStore`` convention (ADR-0016) -
            CrewAI registers one backend per process via
            ``set_memory_storage_factory``, not many concurrently constructed
            instances the way the OpenAI Agents SDK adapter's ``Session`` is
            (ADR-0020), so there is no reason to depart from that precedent
            here.
    """

    def __init__(self, path: str = ".") -> None:
        self._store = mnem.open(path)

    def save(self, records: list[MemoryRecord]) -> None:
        raise NotImplementedError("lands in #312")

    def search(
        self,
        query_embedding: list[float],
        scope_prefix: str | None = None,
        categories: list[str] | None = None,
        metadata_filter: dict[str, Any] | None = None,
        limit: int = 10,
        min_score: float = 0.0,
    ) -> list[tuple[MemoryRecord, float]]:
        raise NotImplementedError("lands in #314")

    def delete(
        self,
        scope_prefix: str | None = None,
        categories: list[str] | None = None,
        record_ids: list[str] | None = None,
        older_than: datetime | None = None,
        metadata_filter: dict[str, Any] | None = None,
    ) -> int:
        raise NotImplementedError("lands in #313")

    def update(self, record: MemoryRecord) -> None:
        raise NotImplementedError("lands in #313")

    def get_record(self, record_id: str) -> MemoryRecord | None:
        raise NotImplementedError("lands in #312")

    def list_records(
        self,
        scope_prefix: str | None = None,
        limit: int = 200,
        offset: int = 0,
    ) -> list[MemoryRecord]:
        raise NotImplementedError("lands in #312")

    def get_scope_info(self, scope: str) -> ScopeInfo:
        raise NotImplementedError("lands in #311")

    def list_scopes(self, parent: str = "/") -> list[str]:
        raise NotImplementedError("lands in #311")

    def list_categories(self, scope_prefix: str | None = None) -> dict[str, int]:
        raise NotImplementedError("lands in #313")

    def count(self, scope_prefix: str | None = None) -> int:
        raise NotImplementedError("lands in #313")

    def reset(self, scope_prefix: str | None = None) -> None:
        raise NotImplementedError("lands in #313")

    async def asave(self, records: list[MemoryRecord]) -> None:
        raise NotImplementedError("lands in #315")

    async def asearch(
        self,
        query_embedding: list[float],
        scope_prefix: str | None = None,
        categories: list[str] | None = None,
        metadata_filter: dict[str, Any] | None = None,
        limit: int = 10,
        min_score: float = 0.0,
    ) -> list[tuple[MemoryRecord, float]]:
        raise NotImplementedError("lands in #315")

    async def adelete(
        self,
        scope_prefix: str | None = None,
        categories: list[str] | None = None,
        record_ids: list[str] | None = None,
        older_than: datetime | None = None,
        metadata_filter: dict[str, Any] | None = None,
    ) -> int:
        raise NotImplementedError("lands in #315")
