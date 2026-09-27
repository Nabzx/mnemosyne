"""``MnemosyneSession``: an OpenAI Agents SDK ``Session`` over a Mnemosyne
store (#249, ADR-0020).

The ``Session`` methods still raise ``NotImplementedError`` - the real
behaviour lands one ticket at a time:

- ``get_items`` / ``add_items``: #305 (the id scheme below, #304, first).
- ``pop_item`` / ``clear_session``: #306.
- Provenance auto-capture via ``on_tool_end``: #307.
- ``branch`` / ``switch`` / ``merge`` / ``why`` / ``bisect`` / ``history``,
  outside the ``Session`` protocol entirely: #308.

Once complete, this class implements the SDK's ``Session`` protocol
structurally (it is a ``Protocol``, not an ABC - no explicit subclassing
needed), so ``Runner.run(agent, prompt, session=MnemosyneSession(store,
session_id))`` works as a drop-in replacement for ``SQLiteSession``.
"""

from __future__ import annotations

import time
from typing import TYPE_CHECKING

import mnem

if TYPE_CHECKING:
    from agents.items import TResponseInputItem
    from agents.memory import SessionSettings

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
    provenance: mnem.Provenance | None = None,
    author: str = "agent",
) -> list[str]:
    """Stage ``contents`` as new items under ``session_id``, bump the
    counter, and commit once - one write, however many nodes it touches
    (ADR-0020, mirroring ``mnem.agents.remember_many``'s batch form).
    Returns the new items' ids, in write order.

    Retries on a lost write race with the same bounded backoff
    ``mnem.agents`` uses. Not built on ``remember_many`` directly: a retry
    must re-read the counter fresh, since a concurrent writer may have
    moved it - a precomputed items list would restage with a stale ``seq``
    on retry.
    """
    if not contents:
        return []

    last: mnem.ConflictError | None = None
    for attempt in range(_RETRIES):
        try:
            start = _read_seq(store, session_id)
            ids = [_item_id(session_id, start + i) for i in range(len(contents))]
            for node_id, content in zip(ids, contents, strict=True):
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

    async def get_items(self, limit: int | None = None) -> list[TResponseInputItem]:
        raise NotImplementedError("lands in #305")

    async def add_items(self, items: list[TResponseInputItem]) -> None:
        raise NotImplementedError("lands in #305")

    async def pop_item(self) -> TResponseInputItem | None:
        raise NotImplementedError("lands in #306")

    async def clear_session(self) -> None:
        raise NotImplementedError("lands in #306")
