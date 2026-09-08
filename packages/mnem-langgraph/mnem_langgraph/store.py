"""``MnemosyneStore``: a LangGraph ``BaseStore`` over a Mnemosyne store (#58,
ADR-0016).

Mapping:

- a namespace tuple plus key becomes one node id, ``":".join([*namespace, key])``
  (so a ``":"`` in a segment is not supported);
- the ``value`` dict is the node content, with a reserved ``_meta`` key lifted
  into provenance so ``blame`` works and dropped from what is stored;
- ``search`` is a plain substring / equality filter, not semantic;
- every ``put`` / ``delete`` is one commit.

Only ``batch`` / ``abatch`` are implemented; ``BaseStore`` builds ``get`` /
``put`` / ``search`` / ``delete`` / ``list_namespaces`` (and the async forms) on
top. ``abatch`` runs ``batch`` in a worker thread so it does not block the loop.
One :class:`mnem.Store` handle is held for the object's life: redb allows a
single open handle per file per process.
"""

from __future__ import annotations

import asyncio
import json
from datetime import datetime, timezone
from typing import Any

import mnem
from langgraph.store.base import (
    BaseStore,
    GetOp,
    Item,
    ListNamespacesOp,
    PutOp,
    SearchItem,
    SearchOp,
)
from mnem import agents

_META = "_meta"


def _node_id(namespace: tuple[str, ...], key: str) -> str:
    return ":".join([*namespace, key])


def _split(node_id: str) -> tuple[tuple[str, ...], str]:
    parts = node_id.split(":")
    return tuple(parts[:-1]), parts[-1]


def _provenance_kwargs(value: dict[str, Any]) -> dict[str, Any]:
    meta = value.get(_META, {})
    if not isinstance(meta, dict):
        return {}
    return {
        "source": meta.get("source"),
        "step": meta.get("step"),
        "observation": meta.get("observation"),
        "tool_call": meta.get("tool_call"),
        "note": meta.get("note"),
    }


def _timestamp(node: mnem.MemoryNode) -> datetime:
    if node.event_time is not None:
        return datetime.fromtimestamp(node.event_time / 1000, tz=timezone.utc)
    return datetime.now(tz=timezone.utc)


class MnemosyneStore(BaseStore):
    """A ``BaseStore`` whose every write is a Mnemosyne commit."""

    supports_ttl = False

    def __init__(self, path: str = ".", *, author: str = "agent") -> None:
        self._store = mnem.open(path)
        self._author = author

    # --- the interface ---------------------------------------------------

    def batch(self, ops: Any) -> list[Any]:
        return [self._run(op) for op in ops]

    async def abatch(self, ops: Any) -> list[Any]:
        return await asyncio.to_thread(self.batch, list(ops))

    def _run(self, op: Any) -> Any:
        if isinstance(op, GetOp):
            return self._get(op.namespace, op.key)
        if isinstance(op, PutOp):
            return self._put(op.namespace, op.key, op.value)
        if isinstance(op, SearchOp):
            return self._search(op)
        if isinstance(op, ListNamespacesOp):
            return self._list_namespaces(op)
        msg = f"unsupported store op: {type(op).__name__}"
        raise NotImplementedError(msg)

    def _get(self, namespace: tuple[str, ...], key: str) -> Item | None:
        node = self._store.working_node(_node_id(namespace, key))
        if node is None:
            return None
        ts = _timestamp(node)
        return Item(
            value=node.content,
            key=key,
            namespace=namespace,
            created_at=ts,
            updated_at=ts,
        )

    def _put(
        self, namespace: tuple[str, ...], key: str, value: dict[str, Any] | None
    ) -> None:
        node_id = _node_id(namespace, key)
        if value is None:
            agents.forget(self._store, node_id, author=self._author)
            return
        agents.remember(
            self._store, node_id, value, author=self._author,
            summary=f"put {node_id}", **_provenance_kwargs(value),
        )

    def _search(self, op: SearchOp) -> list[SearchItem]:
        prefix = tuple(op.namespace_prefix)
        query = (op.query or "").lower()
        out: list[SearchItem] = []
        for node_id, node in self._store.working_memory().items():
            namespace, key = _split(node_id)
            if namespace[: len(prefix)] != prefix:
                continue
            value = node.content if isinstance(node.content, dict) else {"value": node.content}
            if op.filter and any(value.get(k) != v for k, v in op.filter.items()):
                continue
            if query and query not in json.dumps(value, default=str).lower():
                continue
            ts = _timestamp(node)
            out.append(SearchItem(namespace, key, value, ts, ts))
        return out[op.offset : op.offset + op.limit]

    def _list_namespaces(self, op: ListNamespacesOp) -> list[tuple[str, ...]]:
        seen: set[tuple[str, ...]] = set()
        for node_id in self._store.working_memory():
            namespace, _ = _split(node_id)
            if op.max_depth is not None:
                namespace = namespace[: op.max_depth]
            if namespace and _matches(namespace, op.match_conditions):
                seen.add(namespace)
        ordered = sorted(seen)
        return ordered[op.offset : op.offset + op.limit]

    # --- extra methods for a graph node --------------------------------

    def branch(self, name: str, *, start: str | None = None) -> str:
        """Fork memory onto a new branch (not part of ``BaseStore``)."""
        self._store.new_branch(name, start=start)
        return name

    def switch(self, target: str) -> None:
        """Move to a branch or commit."""
        self._store.checkout(target)

    def why(self, namespace: tuple[str, ...], key: str) -> mnem.Blame:
        """The commit and provenance that set this memory's current value."""
        return self._store.blame(_node_id(namespace, key))

    def history(self, *, limit: int = 20) -> list[mnem.Commit]:
        """The recent commits, newest first."""
        return self._store.log(limit=limit)


def _matches(namespace: tuple[str, ...], conditions: Any) -> bool:
    for condition in conditions or ():
        path = tuple(p for p in condition.path if p != "*")
        if condition.match_type == "prefix" and namespace[: len(path)] != path:
            return False
        if condition.match_type == "suffix" and namespace[-len(path):] != path:
            return False
    return True
