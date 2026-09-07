"""The agent-facing Mnemosyne API, ergonomics over the ``_mnem`` binding.

ADR-0004: this layer is pure ergonomics. It adds Pythonic names, dataclasses,
JSON handling and sensible defaults; it holds no operation semantics, which
live in the Rust core.
"""

from __future__ import annotations

import json
import os
import time
from collections.abc import Callable
from contextlib import AbstractContextManager
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from . import _mnem

__all__ = [
    "Blame",
    "Commit",
    "Conflict",
    "MemoryNode",
    "MergeResult",
    "NodeChange",
    "Provenance",
    "Store",
    "init",
    "open",
]


@dataclass(frozen=True, slots=True)
class Provenance:
    """How a memory node came to exist (ADR-0003). Every field is optional."""

    agent_step: str | None = None
    observation: str | None = None
    tool_call: str | None = None
    source: str | None = None
    note: str | None = None

    @classmethod
    def _from_dict(cls, d: dict[str, Any]) -> Provenance:
        return cls(
            agent_step=d.get("agent_step"),
            observation=d.get("observation"),
            tool_call=d.get("tool_call"),
            source=d.get("source"),
            note=d.get("note"),
        )


@dataclass(slots=True)
class MemoryNode:
    """A memory node. ``content`` is any JSON-serialisable value."""

    id: str
    content: Any
    content_kind: str = "note"
    provenance: Provenance = field(default_factory=Provenance)
    event_time: int | None = None

    @classmethod
    def _from_dict(cls, d: dict[str, Any]) -> MemoryNode:
        return cls(
            id=d["id"],
            content=json.loads(d["content"]),
            content_kind=d["content_kind"],
            provenance=Provenance._from_dict(d["provenance"]),
            event_time=d.get("event_time"),
        )


@dataclass(frozen=True, slots=True)
class NodeChange:
    """One node's change between two states, as returned by :meth:`Store.diff`."""

    id: str
    kind: str  # "added" | "removed" | "modified"
    old: MemoryNode | None
    new: MemoryNode | None

    @classmethod
    def _from_dict(cls, d: dict[str, Any]) -> NodeChange:
        old = d["old"]
        new = d["new"]
        return cls(
            id=d["id"],
            kind=d["kind"],
            old=MemoryNode._from_dict(old) if old is not None else None,
            new=MemoryNode._from_dict(new) if new is not None else None,
        )


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


@dataclass(frozen=True, slots=True)
class Conflict:
    """One unresolved node in a merge (ADR-0014). Transient: not stored."""

    id: str
    kind: str  # "edit/edit" | "delete/edit" | "edit/delete" | "add/add"
    base: MemoryNode | None
    ours: MemoryNode | None
    theirs: MemoryNode | None

    @classmethod
    def _from_dict(cls, d: dict[str, Any]) -> Conflict:
        def side(v: dict[str, Any] | None) -> MemoryNode | None:
            return MemoryNode._from_dict(v) if v is not None else None

        return cls(
            id=d["id"],
            kind=d["kind"],
            base=side(d["base"]),
            ours=side(d["ours"]),
            theirs=side(d["theirs"]),
        )


@dataclass(frozen=True, slots=True)
class MergeResult:
    """The outcome of :meth:`Store.merge`."""

    status: str  # "up-to-date" | "fast-forwarded" | "merged" | "conflicts"
    commit: str | None
    conflicts: tuple[Conflict, ...]

    @property
    def ok(self) -> bool:
        """Whether the merge finished. ``False`` means unresolved conflicts."""
        return self.status != "conflicts"

    @classmethod
    def _from_dict(cls, d: dict[str, Any]) -> MergeResult:
        return cls(
            status=d["status"],
            commit=d["commit"],
            conflicts=tuple(Conflict._from_dict(row) for row in d["conflicts"]),
        )


@dataclass(frozen=True, slots=True)
class Blame:
    """The result of :meth:`Store.blame`."""

    commit: str
    node: MemoryNode
    time: int  # the node's event_time, or the commit's record time

    @property
    def provenance(self) -> Provenance:
        """Shorthand for ``self.node.provenance``."""
        return self.node.provenance

    @classmethod
    def _from_dict(cls, d: dict[str, Any]) -> Blame:
        return cls(
            commit=d["commit"],
            node=MemoryNode._from_dict(d["node"]),
            time=d["time"],
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

    def rm(self, node_id: str) -> None:
        """Stage the deletion of a node from the next commit."""
        self._inner.rm(node_id)

    def staged_deletions(self) -> list[str]:
        """The node ids staged for deletion, sorted."""
        return list(self._inner.staged_deletions())

    def log(
        self,
        *,
        first_parent: bool = False,
        limit: int | None = None,
    ) -> list[Commit]:
        """The commit history from ``HEAD``, newest first."""
        rows = self._inner.log(first_parent=first_parent, limit=limit)
        return [Commit._from_row(row) for row in rows]

    # --- Phase 2 (ADR-0012) ---

    def head_commit(self) -> str | None:
        """The commit ``HEAD`` resolves to, or ``None`` on an unborn branch."""
        return self._inner.head_commit()

    def resolve(self, spec: str) -> str:
        """Resolve a branch name or commit id prefix to a full commit id."""
        return self._inner.resolve_commitish(spec)

    def new_branch(self, name: str, *, start: str | None = None) -> None:
        """Create a branch at ``start`` (a commit-ish) or at ``HEAD``."""
        self._inner.branch(name, start)

    def branches(self) -> list[tuple[str, str]]:
        """Every branch as ``(name, tip_commit_id)``, sorted by name."""
        return [(name, tip) for name, tip in self._inner.branches()]

    def delete_branch(self, name: str) -> bool:
        """Delete a branch. Returns whether it existed. Refuses the current one."""
        return self._inner.delete_branch(name)

    def checkout(
        self,
        target: str | None = None,
        *,
        create: str | None = None,
        discard: bool = False,
    ) -> None:
        """Move ``HEAD`` to ``target``. With ``create``, make that branch first
        (``target`` is then its start point, default ``HEAD``)."""
        if create is not None:
            self._inner.branch(create, target)
            target = create
        elif target is None:
            msg = "checkout needs a target, or create= to make a branch"
            raise ValueError(msg)
        self._inner.checkout(target, discard=discard)

    def working_memory(self) -> dict[str, MemoryNode]:
        """The current working memory, keyed by node id."""
        return {
            node_id: MemoryNode._from_dict(node)
            for node_id, node in self._inner.working_memory().items()
        }

    def working_node(self, node_id: str) -> MemoryNode | None:
        """One node from working memory, or ``None`` if absent or deleted."""
        node = self._inner.working_node(node_id)
        return MemoryNode._from_dict(node) if node is not None else None

    def state_at(self, commit: str) -> dict[str, MemoryNode]:
        """The full memory at a commit (a commit-ish), keyed by node id."""
        return {
            node_id: MemoryNode._from_dict(node)
            for node_id, node in self._inner.state_at(commit).items()
        }

    def diff(self, frm: str | None = None, to: str | None = None) -> list[NodeChange]:
        """Changes between two states. ``frm`` defaults to the ``HEAD`` commit,
        ``to`` to working memory."""
        return [NodeChange._from_dict(row) for row in self._inner.diff(frm, to)]

    # --- Phase 3 (ADR-0013, ADR-0014) ---

    def merge(
        self,
        theirs: str,
        *,
        resolutions: dict[str, str | MemoryNode] | None = None,
        strategy: str | None = None,
        message: str | None = None,
        author: str | None = None,
        time_ms: int | None = None,
    ) -> MergeResult:
        """Merge ``theirs`` (a commit-ish) into the current branch.

        ``resolutions`` maps a conflicting node id to ``"ours"``, ``"theirs"``,
        ``"base"``, ``"delete"``, or a :class:`MemoryNode` to set directly.
        ``strategy`` (``"ours"`` or ``"theirs"``) resolves any conflict not in
        the map. A merge with conflicts left unresolved writes nothing and
        returns a result whose ``status`` is ``"conflicts"``; call again with a
        fuller ``resolutions`` map.
        """
        wire: dict[str, Any] | None = None
        if resolutions is not None:
            wire = {}
            for node_id, pick in resolutions.items():
                if isinstance(pick, MemoryNode):
                    wire[node_id] = {
                        "content": json.dumps(pick.content),
                        "content_kind": pick.content_kind,
                    }
                else:
                    wire[node_id] = pick
        row = self._inner.merge(
            theirs,
            resolutions=wire,
            strategy=strategy,
            message=message,
            author=_resolve_author(author),
            time_ms=_now_ms() if time_ms is None else time_ms,
        )
        return MergeResult._from_dict(row)

    # --- Phase 4 (ADR-0015) ---

    def blame(self, node_id: str, at: str | None = None) -> Blame:
        """Resolve ``node_id`` to the commit, and the node, that gave it its
        current value as of ``at`` (a commit-ish; ``None`` is ``HEAD``).

        Follows the contributing parent through merges. Raises
        :class:`InvalidRefError` if the node is not in memory at ``at``.
        """
        return Blame._from_dict(self._inner.blame(node_id, at))

    def bisect(
        self,
        predicate: Callable[[dict[str, MemoryNode]], bool],
        *,
        bad: str | None = None,
        good: str | None = None,
    ) -> str:
        """The first commit where ``predicate`` holds.

        ``predicate`` is called with the memory at a commit (``{node_id:
        MemoryNode}``). ``bad`` defaults to ``HEAD``, ``good`` to the root of
        ``bad``'s first-parent chain; ``predicate`` must be false at ``good``
        and true at ``bad``, and is assumed monotonic between them. Returns the
        boundary commit id.
        """

        def wrapped(raw: dict[str, Any]) -> bool:
            state = {k: MemoryNode._from_dict(v) for k, v in raw.items()}
            return bool(predicate(state))

        return self._inner.bisect(wrapped, bad=bad, good=good)

    def changed_by(self, commit: str) -> dict[str, str]:
        """The node ids a commit changed against its first parent, each mapped
        to ``"added"``, ``"modified"`` or ``"removed"``."""
        return dict(self._inner.changed_by(commit))

    def branch(self, name: str, *, start: str | None = None) -> _BranchScope:
        """Create ``name`` (at ``start`` or ``HEAD``) and return a context
        manager. Entering checks it out; leaving checks the previous branch back
        and leaves ``name`` in place (ADR-0004)::

            with store.branch("assume-downgrade"):
                store.add("customer", "downgraded")
                store.commit("try it")
            # back on the original branch; "assume-downgrade" still there

        Commit inside the block to keep work; uncommitted staged changes at the
        block's exit are discarded. On an exception the previous branch is still
        checked back and the exception re-raised.
        """
        self._inner.branch(name, start)
        return _BranchScope(self, name)

    def __repr__(self) -> str:
        return f"<mnem.Store root={str(self.root)!r}>"


class _BranchScope(AbstractContextManager["_BranchScope"]):
    """The context manager returned by :meth:`Store.branch`."""

    __slots__ = ("_name", "_return_to", "_store")

    def __init__(self, store: Store, name: str) -> None:
        self._store = store
        self._name = name
        self._return_to: str | None = None

    def __enter__(self) -> _BranchScope:  # noqa: PYI034 - Self needs 3.11
        head = self._store.head()
        self._return_to = head[len("ref: ") :] if head.startswith("ref: ") else None
        self._store.checkout(self._name)
        return self

    def __exit__(self, *exc: object) -> None:
        if self._return_to is not None:
            self._store.checkout(self._return_to, discard=True)


def init(path: str | os.PathLike[str] = ".") -> Store:
    """Create a store at ``path/.mnem`` and open it."""
    return Store.init(path)


def open(path: str | os.PathLike[str] = ".") -> Store:
    """Open the nearest store at or above ``path``."""
    return Store.open(path)
