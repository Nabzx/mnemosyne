"""``MnemosyneSession``: an OpenAI Agents SDK ``Session`` over a Mnemosyne
store (#249, ADR-0020).

Scaffolding only (#303) - every method below raises ``NotImplementedError``.
The real behaviour lands one ticket at a time:

- ``get_items`` / ``add_items``: #305 (the node id scheme from #304 first).
- ``pop_item`` / ``clear_session``: #306.
- Provenance auto-capture via ``on_tool_end``: #307.
- ``branch`` / ``switch`` / ``merge`` / ``why`` / ``bisect`` / ``history``,
  outside the ``Session`` protocol entirely: #308.

Once complete, this class implements the SDK's ``Session`` protocol
structurally (it is a ``Protocol``, not an ABC - no explicit subclassing
needed), so ``Runner.run(agent, prompt, session=MnemosyneSession(session_id,
"./agent-memory"))`` works as a drop-in replacement for ``SQLiteSession``.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

import mnem

if TYPE_CHECKING:
    from agents.items import TResponseInputItem
    from agents.memory import SessionSettings


class MnemosyneSession:
    """A ``Session`` backed by a Mnemosyne store, scoped to one ``session_id``.

    Args:
        session_id: This session's own identifier - the id-prefix namespace
            every node this session writes lives under (ADR-0020).
        path: Where the store lives. Opened with :func:`mnem.open`, the same
            "nearest store at or above ``path``" lookup the CLI uses -
            mirroring the LangGraph adapter's ``MnemosyneStore``, which also
            takes a path and opens its own handle rather than accepting an
            already-open one.
        session_settings: Optional default settings (currently just
            ``limit``), matching the SDK's own ``Session`` protocol field.
    """

    def __init__(
        self,
        session_id: str,
        path: str = ".",
        *,
        session_settings: SessionSettings | None = None,
    ) -> None:
        self.session_id = session_id
        self.session_settings = session_settings
        self._store = mnem.open(path)

    async def get_items(self, limit: int | None = None) -> list[TResponseInputItem]:
        raise NotImplementedError("lands in #305")

    async def add_items(self, items: list[TResponseInputItem]) -> None:
        raise NotImplementedError("lands in #305")

    async def pop_item(self) -> TResponseInputItem | None:
        raise NotImplementedError("lands in #306")

    async def clear_session(self) -> None:
        raise NotImplementedError("lands in #306")
