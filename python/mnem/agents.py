"""Atomic write helpers for agents and adapters (ADR-0016).

``remember`` / ``remember_many`` / ``forget`` each do a stage-then-commit in one
call and retry a few times if a concurrent writer holds the branch. This is the
contract the MCP server (``mnem-mcp``) and the LangGraph adapter
(``mnem-langgraph``) share, kept here so the two do not drift.

There is no staging concept for a caller of this module: one call is one commit.
"""

from __future__ import annotations

import time
from collections.abc import Callable, Iterable, Mapping
from typing import Any, TypeVar

from ._mnem import ConflictError
from ._sdk import Provenance, Store

__all__ = ["forget", "remember", "remember_many"]

_RETRIES = 4
_BACKOFF_MS = 25

T = TypeVar("T")


def _with_retry(action: Callable[[], T]) -> T:
    """Run ``action``, retrying on a lost write race (a bounded number of times,
    with a short linear backoff), then re-raise the last :class:`ConflictError`."""
    last: ConflictError | None = None
    for attempt in range(_RETRIES):
        try:
            return action()
        except ConflictError as exc:
            last = exc
            time.sleep(_BACKOFF_MS * (attempt + 1) / 1000)
    assert last is not None
    raise last


def _provenance(fields: Mapping[str, Any]) -> Provenance:
    return Provenance(
        agent_step=fields.get("step"),
        observation=fields.get("observation"),
        tool_call=fields.get("tool_call"),
        source=fields.get("source"),
        note=fields.get("note"),
    )


def remember(
    store: Store,
    id: str,
    content: Any,
    *,
    source: str | None = None,
    step: str | None = None,
    observation: str | None = None,
    tool_call: str | None = None,
    note: str | None = None,
    summary: str | None = None,
    author: str = "agent",
    time_ms: int | None = None,
) -> str:
    """Record one fact as a single commit. Returns the commit id.

    The provenance fields are the same five as :class:`~mnem.Provenance`
    (``step`` maps to ``agent_step``). ``summary`` is the commit message,
    defaulting to ``"remember <id>"``.
    """
    prov = _provenance(
        {
            "step": step,
            "observation": observation,
            "tool_call": tool_call,
            "source": source,
            "note": note,
        }
    )

    def go() -> str:
        store.add(id, content, provenance=prov)
        return store.commit(
            summary or f"remember {id}", author=author, time_ms=time_ms
        )

    return _with_retry(go)


def remember_many(
    store: Store,
    items: Iterable[Mapping[str, Any]],
    *,
    summary: str | None = None,
    author: str = "agent",
    time_ms: int | None = None,
) -> str:
    """Record several facts as one commit. Returns the commit id.

    Each item is a mapping with ``id`` and ``content``, plus any of the
    provenance keys (``source`` / ``step`` / ``observation`` / ``tool_call`` /
    ``note``).
    """
    staged = [dict(item) for item in items]

    def go() -> str:
        for item in staged:
            node_id = item["id"]
            content = item["content"]
            store.add(node_id, content, provenance=_provenance(item))
        n = len(staged)
        message = summary or f"remember {n} node{'s' if n != 1 else ''}"
        return store.commit(message, author=author, time_ms=time_ms)

    return _with_retry(go)


def forget(
    store: Store,
    id: str,
    *,
    summary: str | None = None,
    author: str = "agent",
    time_ms: int | None = None,
) -> str:
    """Tombstone one node as a single commit. Returns the commit id."""

    def go() -> str:
        store.rm(id)
        return store.commit(
            summary or f"forget {id}", author=author, time_ms=time_ms
        )

    return _with_retry(go)
