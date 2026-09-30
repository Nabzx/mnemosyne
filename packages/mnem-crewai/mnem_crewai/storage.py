"""``MnemosyneStorageBackend``: a CrewAI ``StorageBackend`` over a Mnemosyne
store (#250, ADR-0021).

Every ``StorageBackend`` protocol method is real. ``asave``/``asearch``/
``adelete`` wrap their sync counterpart in ``asyncio.to_thread`` (matching
the LangGraph adapter's existing precedent, ADR-0021) rather than a native
async re-implementation - a worked example is the only thing left (#316).

``search``/``list_records`` never filter on ``MemoryRecord.private`` - that
enforcement is CrewAI's own ``RecallFlow``'s job, not the storage layer's;
the real ``StorageBackend.search`` protocol never passes a requester
identity to filter by (ADR-0025, correcting ADR-0021's original claim).

This class implements CrewAI's ``StorageBackend`` protocol structurally (it
is a ``Protocol``, not an ABC - no explicit subclassing needed), so
``set_memory_storage_factory(lambda spec: MnemosyneStorageBackend(
"./agent-memory"))`` registers it as the process-wide default (a one-time,
process-wide setter, not a per-``Memory``-instance argument - CrewAI's own
factory contract, not something this adapter can make more granular).
"""

from __future__ import annotations

import asyncio
import math
import time
from typing import TYPE_CHECKING, Any

import mnem
from crewai.memory.types import MemoryRecord, ScopeInfo

if TYPE_CHECKING:
    from datetime import datetime

_RETRIES = 4
_BACKOFF_MS = 25


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


def _cosine_similarity(a: list[float], b: list[float]) -> float:
    """Exact cosine similarity - the same math LanceDB/Qdrant would run
    against the same vectors (ADR-0021), just unindexed. Assumes equal-length
    vectors, which CrewAI's own single configured embedder guarantees for
    every record and query within one crew's data; a length mismatch raises
    (via ``zip(strict=True)``) rather than silently truncating to a
    technically-computable but wrong score.
    """
    dot = sum(x * y for x, y in zip(a, b, strict=True))
    norm_a = math.sqrt(sum(x * x for x in a))
    norm_b = math.sqrt(sum(y * y for y in b))
    if norm_a == 0.0 or norm_b == 0.0:
        return 0.0
    return dot / (norm_a * norm_b)


def _forget_many(store: mnem.Store, node_ids: list[str], *, author: str = "agent") -> None:
    """Tombstone every id in ``node_ids`` as one commit - ``remember_many``'s
    batch shape, for deletions (mirrors the OpenAI Agents adapter's own
    helper, ADR-0020). A retry restages the same fixed id list rather than
    re-reading anything: which ids to forget doesn't depend on timing the
    way a fresh sequence counter would.
    """
    if not node_ids:
        return
    last: mnem.ConflictError | None = None
    for attempt in range(_RETRIES):
        try:
            for node_id in node_ids:
                store.rm(node_id)
            n = len(node_ids)
            store.commit(f"forget {n} node{'s' if n != 1 else ''}", author=author)
            return
        except mnem.ConflictError as exc:
            last = exc
            time.sleep(_BACKOFF_MS * (attempt + 1) / 1000)
    assert last is not None
    raise last


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
        if not records:
            return
        last: mnem.ConflictError | None = None
        for attempt in range(_RETRIES):
            try:
                for record in records:
                    self._store.add(
                        _node_id(record.scope, record.id),
                        _record_to_content(record),
                        provenance=mnem.Provenance(source=record.source),
                    )
                n = len(records)
                self._store.commit(f"save {n} record{'s' if n != 1 else ''}")
                return
            except mnem.ConflictError as exc:
                last = exc
                time.sleep(_BACKOFF_MS * (attempt + 1) / 1000)
        assert last is not None
        raise last

    def search(
        self,
        query_embedding: list[float],
        scope_prefix: str | None = None,
        categories: list[str] | None = None,
        metadata_filter: dict[str, Any] | None = None,
        limit: int = 10,
        min_score: float = 0.0,
    ) -> list[tuple[MemoryRecord, float]]:
        normalized = None if scope_prefix is None else (scope_prefix.rstrip("/") or "/")
        scored: list[tuple[MemoryRecord, float]] = []
        for node_id, node in self._store.working_memory().items():
            if normalized is not None and not _is_in_scope_subtree(node_id, normalized):
                continue
            record = _content_to_record(node.content)
            if record.embedding is None:
                continue
            if categories is not None and not any(c in record.categories for c in categories):
                continue
            if metadata_filter is not None and not all(
                record.metadata.get(k) == v for k, v in metadata_filter.items()
            ):
                continue
            score = _cosine_similarity(query_embedding, record.embedding)
            if score >= min_score:
                scored.append((record, score))
        scored.sort(key=lambda pair: pair[1], reverse=True)
        return scored[:limit]

    def delete(
        self,
        scope_prefix: str | None = None,
        categories: list[str] | None = None,
        record_ids: list[str] | None = None,
        older_than: datetime | None = None,
        metadata_filter: dict[str, Any] | None = None,
    ) -> int:
        normalized = None if scope_prefix is None else (scope_prefix.rstrip("/") or "/")
        to_delete = []
        for node_id, node in self._store.working_memory().items():
            if normalized is not None and not _is_in_scope_subtree(node_id, normalized):
                continue
            record = _content_to_record(node.content)
            if record_ids is not None and record.id not in record_ids:
                continue
            if categories is not None and not any(c in record.categories for c in categories):
                continue
            if older_than is not None and record.created_at >= older_than:
                continue
            if metadata_filter is not None and not all(
                record.metadata.get(k) == v for k, v in metadata_filter.items()
            ):
                continue
            to_delete.append(node_id)
        _forget_many(self._store, to_delete)
        return len(to_delete)

    def update(self, record: MemoryRecord) -> None:
        new_node_id = _node_id(record.scope, record.id)
        suffix = f"/{record.id}"
        last: mnem.ConflictError | None = None
        for attempt in range(_RETRIES):
            try:
                old_node_id = next(
                    (
                        node_id
                        for node_id in self._store.working_memory()
                        if node_id.endswith(suffix)
                    ),
                    None,
                )
                # The record's own scope may have changed since it was
                # saved - if so this is really a move, not an in-place
                # replace, so the old node under the old scope is
                # tombstoned in the same commit as the new one is written.
                if old_node_id is not None and old_node_id != new_node_id:
                    self._store.rm(old_node_id)
                self._store.add(
                    new_node_id,
                    _record_to_content(record),
                    provenance=mnem.Provenance(source=record.source),
                )
                self._store.commit(f"update {record.id}")
                return
            except mnem.ConflictError as exc:
                last = exc
                time.sleep(_BACKOFF_MS * (attempt + 1) / 1000)
        assert last is not None
        raise last

    def get_record(self, record_id: str) -> MemoryRecord | None:
        # Node ids are {scope}/{record_id}, and the scope isn't known here -
        # a scan, not an indexed lookup, since record_id (a uuid4) carries
        # no scope information to key on directly.
        suffix = f"/{record_id}"
        for node_id, node in self._store.working_memory().items():
            if node_id.endswith(suffix):
                return _content_to_record(node.content)
        return None

    def list_records(
        self,
        scope_prefix: str | None = None,
        limit: int = 200,
        offset: int = 0,
    ) -> list[MemoryRecord]:
        normalized = None if scope_prefix is None else (scope_prefix.rstrip("/") or "/")
        records = [
            _content_to_record(node.content)
            for node_id, node in self._store.working_memory().items()
            if normalized is None or _is_in_scope_subtree(node_id, normalized)
        ]
        records.sort(key=lambda r: r.created_at, reverse=True)
        return records[offset : offset + limit]

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
        normalized = None if scope_prefix is None else (scope_prefix.rstrip("/") or "/")
        counts: dict[str, int] = {}
        for node_id, node in self._store.working_memory().items():
            if normalized is not None and not _is_in_scope_subtree(node_id, normalized):
                continue
            for category in _content_to_record(node.content).categories:
                counts[category] = counts.get(category, 0) + 1
        return counts

    def count(self, scope_prefix: str | None = None) -> int:
        normalized = None if scope_prefix is None else (scope_prefix.rstrip("/") or "/")
        return sum(
            1
            for node_id in self._store.working_memory()
            if normalized is None or _is_in_scope_subtree(node_id, normalized)
        )

    def reset(self, scope_prefix: str | None = None) -> None:
        normalized = None if scope_prefix is None else (scope_prefix.rstrip("/") or "/")
        node_ids = [
            node_id
            for node_id in self._store.working_memory()
            if normalized is None or _is_in_scope_subtree(node_id, normalized)
        ]
        _forget_many(self._store, node_ids)

    async def asave(self, records: list[MemoryRecord]) -> None:
        await asyncio.to_thread(self.save, records)

    async def asearch(
        self,
        query_embedding: list[float],
        scope_prefix: str | None = None,
        categories: list[str] | None = None,
        metadata_filter: dict[str, Any] | None = None,
        limit: int = 10,
        min_score: float = 0.0,
    ) -> list[tuple[MemoryRecord, float]]:
        return await asyncio.to_thread(
            self.search,
            query_embedding,
            scope_prefix=scope_prefix,
            categories=categories,
            metadata_filter=metadata_filter,
            limit=limit,
            min_score=min_score,
        )

    async def adelete(
        self,
        scope_prefix: str | None = None,
        categories: list[str] | None = None,
        record_ids: list[str] | None = None,
        older_than: datetime | None = None,
        metadata_filter: dict[str, Any] | None = None,
    ) -> int:
        return await asyncio.to_thread(
            self.delete,
            scope_prefix=scope_prefix,
            categories=categories,
            record_ids=record_ids,
            older_than=older_than,
            metadata_filter=metadata_filter,
        )
