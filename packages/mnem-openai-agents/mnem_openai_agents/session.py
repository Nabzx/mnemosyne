"""``MnemosyneSession``: an OpenAI Agents SDK ``Session`` over a Mnemosyne
store (#249, ADR-0020).

Extra methods outside the ``Session`` protocol still land one ticket at a
time:

- ``branch`` / ``switch`` / ``merge`` / ``why`` / ``bisect`` / ``history``,
  outside the ``Session`` protocol entirely: #308.

Once complete, this class implements the SDK's ``Session`` protocol
structurally (it is a ``Protocol``, not an ABC - no explicit subclassing
needed), so ``Runner.run(agent, prompt, session=MnemosyneSession(store,
session_id))`` works as a drop-in replacement for ``SQLiteSession``.

Passing ``hooks=session.hooks`` to that same ``Runner.run`` call turns on
provenance auto-capture (ADR-0020, #307): every node written as a result of
a tool call gets a real ``Provenance.source``/``observation`` for free, no
per-call payload to shape by hand the way LangGraph's ``_meta`` needs. This
is the one piece of caller wiring the mechanism needs - the SDK has no way
for a ``Session`` to register a hook on a run just by being passed as
``session=``; ``RunHooksBase`` and ``Session`` are independent parameters
on ``Runner.run``.
"""

from __future__ import annotations

import time
from typing import TYPE_CHECKING

import mnem
from agents import RunHooks

if TYPE_CHECKING:
    from agents.items import TResponseInputItem
    from agents.memory import SessionSettings
    from agents.tool import Tool

_SEQ_SUFFIX = ":_seq"
_RETRIES = 4
_BACKOFF_MS = 25


def _item_id(session_id: str, seq: int) -> str:
    """The sortable id for the ``seq``-th item written to ``session_id``.

    Zero-padded to 10 digits so ``working_memory()``'s ``BTreeMap``
    lexicographic order equals insertion order within a session, past 9
    items (ADR-0020).
    """
    return f"{session_id}:{seq:010d}"


def _seq_node_id(session_id: str) -> str:
    """The reserved counter node's id for ``session_id``."""
    return f"{session_id}{_SEQ_SUFFIX}"


def _read_seq(store: mnem.Store, session_id: str) -> int:
    node = store.working_node(_seq_node_id(session_id))
    return 0 if node is None else node.content


def _write_items(
    store: mnem.Store,
    session_id: str,
    contents: list,
    *,
    provenances: list[mnem.Provenance | None] | None = None,
    author: str = "agent",
) -> list[str]:
    """Stage ``contents`` as new items under ``session_id``, bump the
    counter, and commit once - one write, however many nodes it touches
    (ADR-0020, mirroring ``mnem.agents.remember_many``'s batch form).
    ``provenances`` (default: none) pairs one-to-one with ``contents``, since
    a single batched ``add_items`` call can carry several tool calls' worth
    of items, each wanting its own provenance, not one shared value (#307).
    Returns the new items' ids, in write order.

    Retries on a lost write race with the same bounded backoff
    ``mnem.agents`` uses. Not built on ``remember_many`` directly: a retry
    must re-read the counter fresh, since a concurrent writer may have
    moved it - a precomputed items list would restage with a stale ``seq``
    on retry.
    """
    if not contents:
        return []
    if provenances is None:
        provenances = [None] * len(contents)

    last: mnem.ConflictError | None = None
    for attempt in range(_RETRIES):
        try:
            start = _read_seq(store, session_id)
            ids = [_item_id(session_id, start + i) for i in range(len(contents))]
            for node_id, content, provenance in zip(ids, contents, provenances, strict=True):
                store.add(node_id, content, provenance=provenance)
            store.add(_seq_node_id(session_id), start + len(contents))
            n = len(contents)
            store.commit(f"add {n} item{'s' if n != 1 else ''} to {session_id}", author=author)
            return ids
        except mnem.ConflictError as exc:
            last = exc
            time.sleep(_BACKOFF_MS * (attempt + 1) / 1000)
    assert last is not None
    raise last


def _read_items(store: mnem.Store, session_id: str) -> list[mnem.MemoryNode]:
    """Every real transcript item under ``session_id``, in id order (write
    order, ADR-0020) - the reserved counter node is never included."""
    prefix = f"{session_id}:"
    seq_id = _seq_node_id(session_id)
    return [
        node
        for node_id, node in store.working_memory().items()
        if node_id.startswith(prefix) and node_id != seq_id
    ]


def _forget_many(store: mnem.Store, node_ids: list[str], *, author: str = "agent") -> None:
    """Tombstone every id in ``node_ids`` as one commit - ``remember_many``'s
    batch shape, for deletions (ADR-0020). Unlike ``_write_items``, a retry
    restages the same fixed id list rather than re-reading anything: the ids
    to forget don't depend on timing the way a fresh ``seq`` does, exactly
    matching ``remember_many``'s own simpler retry.
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


def _resolve_limit(limit: int | None, settings: SessionSettings | None) -> int | None:
    """``limit``, falling back to ``settings.limit`` when unset - the same
    precedence :class:`agents.memory.sqlite_session.SQLiteSession` uses, not
    imported from there since it is not part of that module's public
    surface."""
    if limit is not None:
        return limit
    return settings.limit if settings is not None else None


class _ToolEndProvenanceHooks(RunHooks):
    """Populates a session's pending per-tool-call provenance from
    ``RunHooksBase.on_tool_end`` (ADR-0020, #307) - the carrier LangGraph's
    ``_meta`` and MCP's ``remember`` argument each had to invent by hand.

    ``add_items`` consumes the pending entries by matching each item's own
    ``call_id`` when it writes, since a single batched call can hold several
    tool calls' worth of items.
    """

    def __init__(self, session: MnemosyneSession) -> None:
        self._session = session

    async def on_tool_end(self, context: object, agent: object, tool: Tool, result: object) -> None:
        call_id = getattr(context, "tool_call_id", None)
        if call_id is None:
            return
        self._session._pending_provenance[call_id] = mnem.Provenance(
            source=getattr(tool, "name", None),
            observation=str(result),
        )


class MnemosyneSession:
    """A ``Session`` backed by a Mnemosyne store, scoped to one ``session_id``.

    Args:
        store: An already-open :class:`mnem.Store`, shared across every
            session a host application constructs (ADR-0020) - a session's
            nodes live under the ``{session_id}:`` prefix, so many sessions
            can share one store the way the LangGraph adapter's namespace
            prefix does. The caller owns the store's lifecycle.
        session_id: This session's own identifier.
        session_settings: Optional default settings (currently just
            ``limit``), matching the SDK's own ``Session`` protocol field.

    Attributes:
        hooks: Pass as ``Runner.run(..., hooks=session.hooks)`` to turn on
            provenance auto-capture (#307) - every node ``add_items`` writes
            as a result of a tool call gets a real ``Provenance.source``/
            ``observation`` for free. Not required for the rest of the
            adapter to work; a run with no ``hooks=`` still reads and writes
            normally, just without that provenance.
    """

    def __init__(
        self,
        store: mnem.Store,
        session_id: str,
        *,
        session_settings: SessionSettings | None = None,
    ) -> None:
        self.session_id = session_id
        self.session_settings = session_settings
        self._store = store
        self._pending_provenance: dict[str, mnem.Provenance] = {}
        self.hooks = _ToolEndProvenanceHooks(self)

    async def get_items(self, limit: int | None = None) -> list[TResponseInputItem]:
        contents = [node.content for node in _read_items(self._store, self.session_id)]
        effective_limit = _resolve_limit(limit, self.session_settings)
        if effective_limit is None:
            return contents
        if effective_limit <= 0:
            return []
        return contents[-effective_limit:]

    async def add_items(self, items: list[TResponseInputItem]) -> None:
        items = list(items)
        call_ids = [item.get("call_id") for item in items]
        # get(), not pop(), per item: one tool call's function_call and
        # function_call_output items share a call_id, both in this batch.
        provenances = [self._pending_provenance.get(cid) for cid in call_ids]
        for cid in set(call_ids):
            self._pending_provenance.pop(cid, None)
        _write_items(self._store, self.session_id, items, provenances=provenances)

    async def pop_item(self) -> TResponseInputItem | None:
        items = _read_items(self._store, self.session_id)
        if not items:
            return None
        last = items[-1]
        mnem.agents.forget(self._store, last.id)
        return last.content

    async def clear_session(self) -> None:
        node_ids = [node.id for node in _read_items(self._store, self.session_id)]
        seq_id = _seq_node_id(self.session_id)
        if self._store.working_node(seq_id) is not None:
            node_ids.append(seq_id)
        _forget_many(self._store, node_ids)
