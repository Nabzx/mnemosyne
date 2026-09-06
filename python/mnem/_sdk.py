"""The agent-facing Mnemosyne API, ergonomics over the ``_mnem`` binding.

ADR-0004: this layer is pure ergonomics. It adds Pythonic names, dataclasses,
JSON handling and sensible defaults; it holds no operation semantics, which
live in the Rust core.
"""

from __future__ import annotations

import json
import os
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from . import _mnem

__all__ = ["Commit", "MemoryNode", "Provenance", "Store", "init", "open"]


@dataclass(frozen=True, slots=True)
class Provenance:
    """How a memory node came to exist (ADR-0003). Every field is optional."""

    agent_step: str | None = None
    observation: str | None = None
    tool_call: str | None = None
    source: str | None = None
    note: str | None = None


@dataclass(slots=True)
class MemoryNode:
    """A memory node to stage. ``content`` is any JSON-serialisable value."""

    id: str
    content: Any
    content_kind: str = "note"
    provenance: Provenance = field(default_factory=Provenance)
    event_time: int | None = None


@dataclass(frozen=True, slots=True)
class Commit:
    """A commit, as returned by :meth:`Store.log`."""

    id: str
    parents: tuple[str, ...]
    state: str
    message: str
    author: str
    time: int

    @classmethod
    def _from_row(cls, row: dict[str, Any]) -> Commit:
        return cls(
            id=row["id"],
            parents=tuple(row["parents"]),
            state=row["state"],
            message=row["message"],
            author=row["author"],
            time=row["time"],
        )


def _resolve_author(author: str | None) -> str:
    """Match the CLI: an explicit author, then ``$MNEM_AUTHOR``, then ``unknown``."""
    return author or os.environ.get("MNEM_AUTHOR") or "unknown"


def _now_ms() -> int:
    return int(time.time() * 1000)


class Store:
    """An open Mnemosyne store. Use :func:`open` or :func:`init` to get one."""

    __slots__ = ("_inner",)

    def __init__(self, inner: _mnem.Store) -> None:
        self._inner = inner

    @classmethod
    def init(cls, path: str | os.PathLike[str] = ".") -> Store:
        """Create a store at ``path/.mnem`` and open it."""
        return cls(_mnem.Store.init(os.fspath(path)))

    @classmethod
    def open(cls, path: str | os.PathLike[str] = ".") -> Store:
        """Open the nearest store at or above ``path``."""
        return cls(_mnem.Store.open(os.fspath(path)))

    @property
    def root(self) -> Path:
        """The directory that holds ``.mnem``."""
        return Path(self._inner.root())

    @property
    def format_version(self) -> int:
        """The store's on-disk format version."""
        return self._inner.format_version()

    def head(self) -> str:
        """The raw contents of ``HEAD``, for example ``"ref: main"``."""
        return self._inner.head()

    def add(
        self,
        id: str,
        content: Any,
        *,
        content_kind: str = "note",
        provenance: Provenance | None = None,
        event_time: int | None = None,
    ) -> str:
        """Stage a memory node for the next commit.

        ``content`` is any JSON-serialisable value, stored verbatim (ADR-0003).
        Returns the node object's id as hex.
        """
        p = provenance or Provenance()
        return self._inner.add(
            id,
            json.dumps(content),
            content_kind=content_kind,
            agent_step=p.agent_step,
            observation=p.observation,
            tool_call=p.tool_call,
            source=p.source,
            note=p.note,
            event_time=event_time,
        )

    def add_node(self, node: MemoryNode) -> str:
        """Stage a :class:`MemoryNode`."""
        return self.add(
            node.id,
            node.content,
            content_kind=node.content_kind,
            provenance=node.provenance,
            event_time=node.event_time,
        )

    def staged(self) -> list[tuple[str, str]]:
        """The staged nodes as ``(node_id, object_id)`` pairs, sorted by node id."""
        return [(node_id, object_id) for node_id, object_id in self._inner.staged()]

    def unstage(self, node_id: str) -> bool:
        """Drop a node from staging. Returns whether it was staged."""
        return self._inner.unstage(node_id)

    def commit(
        self,
        message: str,
        *,
        author: str | None = None,
        time_ms: int | None = None,
    ) -> str:
        """Record everything staged as a commit on the current branch.

        ``author`` falls back to ``$MNEM_AUTHOR`` then ``"unknown"``, matching
        the CLI. ``time_ms`` defaults to now. Returns the commit id as hex.
        """
        return self._inner.commit(
            message,
            _resolve_author(author),
            _now_ms() if time_ms is None else time_ms,
        )

    def log(
        self,
        *,
        first_parent: bool = False,
        limit: int | None = None,
    ) -> list[Commit]:
        """The commit history from ``HEAD``, newest first."""
        rows = self._inner.log(first_parent=first_parent, limit=limit)
        return [Commit._from_row(row) for row in rows]

    def __repr__(self) -> str:
        return f"<mnem.Store root={str(self.root)!r}>"


def init(path: str | os.PathLike[str] = ".") -> Store:
    """Create a store at ``path/.mnem`` and open it."""
    return Store.init(path)


def open(path: str | os.PathLike[str] = ".") -> Store:
    """Open the nearest store at or above ``path``."""
    return Store.open(path)
