"""``MnemosyneStorageBackend``: a CrewAI ``StorageBackend`` over a Mnemosyne
store (#250, ADR-0021).

``save`` / ``search`` / ``delete`` / ``update`` / ``get_record`` /
``list_records`` / ``count`` / ``list_categories`` / ``reset`` and the
async wrappers still raise ``NotImplementedError`` - real behaviour lands
one ticket at a time:

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

from typing import TYPE_CHECKING, Any

import mnem
from crewai.memory.types import MemoryRecord, ScopeInfo

if TYPE_CHECKING:
    from datetime import datetime


def _node_id(scope: str, record_id: str) -> str:
    """CrewAI's own ``/``-delimited scope path, honored literally
    (ADR-0021): ``{scope}/{record_id}``. ``scope.rstrip("/")`` handles the
    root scope (``"/"``) and every other scope with one rule - root
    strips to ``""``, so the result is ``/{record_id}``; any other scope
    has no trailing slash to strip, so the result is ``{scope}/{record_id}``.
    """
    return f"{scope.rstrip('/')}/{record_id}"


def _scope_of(node_id: str) -> str:
    """The scope a node id was written under - everything but the final
    ``/record_id`` segment."""
    scope, _, _record_id = node_id.rpartition("/")
    return scope or "/"


def _ancestors(scope: str) -> list[str]:
    """``scope`` itself, plus every ancestor up to and including root
    (``"/"``) - "/a/b/c" implies "/a/b" and "/a" are real scopes too, even
    with nothing stored directly at them."""
    result = []
    current = scope
    while True:
        result.append(current)
        if current == "/":
            break
        parent, _, _ = current.rpartition("/")
        current = parent or "/"
    return result


def _all_scopes(store: mnem.Store) -> set[str]:
    """Every scope with a real record, plus every implied ancestor scope."""
    scopes: set[str] = {"/"}
    for node_id in store.working_memory():
        scopes.update(_ancestors(_scope_of(node_id)))
    return scopes


def _is_in_scope_subtree(node_id: str, scope: str) -> bool:
    """Whether ``node_id`` was written at ``scope`` or anywhere under it."""
    node_scope = _scope_of(node_id)
    if node_scope == scope:
        return True
    prefix = scope if scope != "/" else ""
    return node_scope.startswith(prefix + "/")


def _record_to_content(record: MemoryRecord) -> dict[str, Any]:
    """The full record as opaque JSON content (ADR-0021) - embedding
    included, overriding ``MemoryRecord.embedding``'s own ``exclude=True``
    (which exists to keep it out of CrewAI's own LLM-facing dumps, not
    storage)."""
    data = record.model_dump(mode="json")
    data["embedding"] = record.embedding
    return data


def _content_to_record(content: dict[str, Any]) -> MemoryRecord:
    return MemoryRecord.model_validate(content)


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
        normalized = scope.rstrip("/") or "/"
        records = [
            _content_to_record(node.content)
            for node_id, node in self._store.working_memory().items()
            if _is_in_scope_subtree(node_id, normalized)
        ]
        categories = sorted({category for r in records for category in r.categories})
        timestamps = [r.created_at for r in records]
        return ScopeInfo(
            path=normalized,
            record_count=len(records),
            categories=categories,
            oldest_record=min(timestamps) if timestamps else None,
            newest_record=max(timestamps) if timestamps else None,
            child_scopes=self.list_scopes(normalized),
        )

    def list_scopes(self, parent: str = "/") -> list[str]:
        normalized = parent.rstrip("/") or "/"
        prefix = "" if normalized == "/" else normalized
        children = set()
        for candidate in _all_scopes(self._store):
            if candidate == normalized or not candidate.startswith(prefix + "/"):
                continue
            remainder = candidate[len(prefix) + 1 :]
            if "/" not in remainder:
                children.add(candidate)
        return sorted(children)

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
