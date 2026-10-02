"""``MnemosyneMemory``: an AutoGen ``Memory`` over a Mnemosyne store (#251,
ADR-0023).

``add``/``query``/``update_context`` are real (#319) - ``query`` ignores its
own argument and returns everything chronologically, and ``update_context``
formats byte-identically to ``ListMemory``'s own output, matching the SDK's
real reference behaviour rather than a degraded one. ``clear``/``close``
still raise ``NotImplementedError`` - lands in #320. A worked example is
#321.

``MnemosyneMemory`` subclasses ``autogen_core.memory.Memory`` directly (an
ABC, unlike CrewAI's structural ``StorageBackend`` Protocol) - one store per
instance, matching how every real AutoGen example constructs a ``Memory``
and hands it to one agent (ADR-0023), not a shared store with many
namespaces the way LangGraph's ``BaseStore`` or CrewAI's globally-registered
``StorageBackend`` are.
"""

from __future__ import annotations

import base64
import time
from typing import TYPE_CHECKING, Any

import mnem
from autogen_core import Image
from autogen_core.memory import Memory, MemoryContent, MemoryQueryResult, UpdateContextResult
from autogen_core.models import SystemMessage

if TYPE_CHECKING:
    from autogen_core import CancellationToken
    from autogen_core.model_context import ChatCompletionContext

_SEQ_SUFFIX = ":_seq"
_RETRIES = 4
_BACKOFF_MS = 25


def _item_id(name: str, seq: int) -> str:
    """The sortable id for the ``seq``-th item written under ``name``.

    Zero-padded to 10 digits so ``working_memory()``'s ``BTreeMap``
    lexicographic order equals insertion order past 9 items - reuses
    ADR-0020's scheme directly (ADR-0023); nothing about ``Memory``'s own
    model argues for a different shape the way CrewAI's hierarchical scope
    paths did (ADR-0021).
    """
    return f"{name}:{seq:010d}"


def _seq_node_id(name: str) -> str:
    """The reserved counter node's id for ``name``."""
    return f"{name}{_SEQ_SUFFIX}"


def _read_seq(store: mnem.Store, name: str) -> int:
    node = store.working_node(_seq_node_id(name))
    return 0 if node is None else node.content


def _write_item(store: mnem.Store, name: str, content: Any, *, author: str = "agent") -> str:
    """Stage one new item under ``name``, bump the counter, commit once.

    AutoGen's own ``add()`` takes exactly one ``MemoryContent`` per call -
    no batch form, unlike the OpenAI Agents SDK's ``add_items`` (ADR-0020) -
    so this writes one item, not a plural ``_write_items`` that would only
    ever be called with a single-element list. Retries on a lost write
    race with the same bounded backoff ``mnem.agents`` uses; a retry
    re-reads the counter fresh, since a concurrent writer may have moved it.
    """
    last: mnem.ConflictError | None = None
    for attempt in range(_RETRIES):
        try:
            seq = _read_seq(store, name)
            node_id = _item_id(name, seq)
            store.add(node_id, content)
            store.add(_seq_node_id(name), seq + 1)
            store.commit(f"add item to {name}", author=author)
            return node_id
        except mnem.ConflictError as exc:
            last = exc
            time.sleep(_BACKOFF_MS * (attempt + 1) / 1000)
    assert last is not None
    raise last


def _read_items(store: mnem.Store, name: str) -> list[mnem.MemoryNode]:
    """Every real item under ``name``, in id order (write order) - the
    reserved counter node is never included."""
    prefix = f"{name}:"
    seq_id = _seq_node_id(name)
    return [
        node
        for node_id, node in store.working_memory().items()
        if node_id.startswith(prefix) and node_id != seq_id
    ]


def _content_to_node_content(content: MemoryContent) -> dict[str, Any]:
    """``MemoryContent`` as JSON-safe node content (ADR-0003: "a string or
    any JSON value"). ``content.content``'s own union type - ``str``,
    ``bytes``, a dict, or an ``Image`` - needs explicit handling:
    ``model_dump(mode="json")`` silently mis-serializes ``bytes`` (decodes
    valid UTF-8 as a plain ``str``, losing the bytes/str distinction
    entirely, and raises outright on invalid UTF-8 - verified directly, not
    assumed) and has no real handling for ``Image`` at all. Reserved marker
    keys (``__bytes_b64__``/``__image_b64__``) are an accepted, documented
    risk against a real ``dict`` payload that happens to use the same key.
    """
    data = content.model_dump(mode="json", exclude={"content"})
    raw = content.content
    if isinstance(raw, bytes):
        data["content"] = {"__bytes_b64__": base64.b64encode(raw).decode("ascii")}
    elif isinstance(raw, Image):
        data["content"] = {"__image_b64__": raw.to_base64()}
    else:
        data["content"] = raw  # str, or an already JSON-compatible dict
    return data


def _node_content_to_content(data: dict[str, Any]) -> MemoryContent:
    raw = data["content"]
    if isinstance(raw, dict) and "__bytes_b64__" in raw:
        restored: Any = base64.b64decode(raw["__bytes_b64__"])
    elif isinstance(raw, dict) and "__image_b64__" in raw:
        restored = Image.from_base64(raw["__image_b64__"])
    else:
        restored = raw
    return MemoryContent.model_validate({**data, "content": restored})


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
        """Formats byte-identically to ``ListMemory``'s own output - a
        numbered ``SystemMessage`` listing every memory in chronological
        order, appended via ``model_context.add_message`` - matching the
        SDK's real reference behaviour, not an adapter-specific format."""
        contents = [_node_content_to_content(node.content) for node in _read_items(self._store, self._name)]
        if not contents:
            return UpdateContextResult(memories=MemoryQueryResult(results=[]))

        memory_strings = [f"{i}. {c.content!s}" for i, c in enumerate(contents, 1)]
        memory_context = (
            "\nRelevant memory content (in chronological order):\n"
            + "\n".join(memory_strings)
            + "\n"
        )
        await model_context.add_message(SystemMessage(content=memory_context))
        return UpdateContextResult(memories=MemoryQueryResult(results=contents))

    async def query(
        self,
        query: str | MemoryContent = "",
        cancellation_token: CancellationToken | None = None,
        **kwargs: Any,
    ) -> MemoryQueryResult:
        """Ignores ``query`` entirely and returns every item chronologically
        - matches ``ListMemory``'s own real behaviour, not a degraded
        implementation (ADR-0023)."""
        _ = query, cancellation_token, kwargs
        contents = [_node_content_to_content(node.content) for node in _read_items(self._store, self._name)]
        return MemoryQueryResult(results=contents)

    async def add(
        self,
        content: MemoryContent,
        cancellation_token: CancellationToken | None = None,
    ) -> None:
        _ = cancellation_token
        _write_item(self._store, self._name, _content_to_node_content(content))

    async def clear(self) -> None:
        raise NotImplementedError("lands in #320")

    async def close(self) -> None:
        raise NotImplementedError("lands in #320")
