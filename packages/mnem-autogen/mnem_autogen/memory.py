"""``MnemosyneMemory``: an AutoGen ``Memory`` over a Mnemosyne store (#251,
ADR-0023).

Scaffolding only (#317) - every method below raises ``NotImplementedError``.
Real behaviour lands one ticket at a time:

- the node id scheme and sequence counter: #318
- ``add`` / ``query`` / ``update_context``: #319
- ``clear`` / ``close``: #320
- a worked example: #321

``MnemosyneMemory`` subclasses ``autogen_core.memory.Memory`` directly (an
ABC, unlike CrewAI's structural ``StorageBackend`` Protocol) - one store per
instance, matching how every real AutoGen example constructs a ``Memory``
and hands it to one agent (ADR-0023), not a shared store with many
namespaces the way LangGraph's ``BaseStore`` or CrewAI's globally-registered
``StorageBackend`` are.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

import mnem
from autogen_core.memory import Memory, MemoryContent, MemoryQueryResult, UpdateContextResult

if TYPE_CHECKING:
    from autogen_core import CancellationToken
    from autogen_core.model_context import ChatCompletionContext


class MnemosyneMemory(Memory):
    """A ``Memory`` backed by a Mnemosyne store - one store per instance.

    Args:
        name: This memory instance's own identifier, matching
            ``ListMemory``'s existing field - the node id scheme (#318) is
            keyed on it.
        path: Where the store lives. Opened with :func:`mnem.open`.
    """

    def __init__(self, name: str = "default_list_memory", path: str = ".") -> None:
        self._name = name
        self._store = mnem.open(path)

    @property
    def name(self) -> str:
        return self._name

    async def update_context(
        self,
        model_context: ChatCompletionContext,
    ) -> UpdateContextResult:
        raise NotImplementedError("lands in #319")

    async def query(
        self,
        query: str | MemoryContent = "",
        cancellation_token: CancellationToken | None = None,
        **kwargs: Any,
    ) -> MemoryQueryResult:
        raise NotImplementedError("lands in #319")

    async def add(
        self,
        content: MemoryContent,
        cancellation_token: CancellationToken | None = None,
    ) -> None:
        raise NotImplementedError("lands in #319")

    async def clear(self) -> None:
        raise NotImplementedError("lands in #320")

    async def close(self) -> None:
        raise NotImplementedError("lands in #320")
