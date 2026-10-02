"""Tests for add/query/update_context (#319, ADR-0023)."""

import asyncio

import mnem
import pytest
from autogen_core import Image
from autogen_core.memory import ListMemory, MemoryContent, MemoryMimeType
from autogen_core.model_context import UnboundedChatCompletionContext

from mnem_autogen import MnemosyneMemory


def _run(coro):
    return asyncio.run(coro)


@pytest.fixture
def memory(tmp_path: object) -> MnemosyneMemory:
    return MnemosyneMemory("agent-a", str(mnem.init(tmp_path).root))


def test_add_then_query_round_trips_in_order(memory: MnemosyneMemory) -> None:
    _run(memory.add(MemoryContent(content="first", mime_type=MemoryMimeType.TEXT)))
    _run(memory.add(MemoryContent(content="second", mime_type=MemoryMimeType.TEXT)))

    result = _run(memory.query(""))

    assert [c.content for c in result.results] == ["first", "second"]


def test_query_ignores_its_own_argument(memory: MnemosyneMemory) -> None:
    _run(memory.add(MemoryContent(content="only item", mime_type=MemoryMimeType.TEXT)))

    result = _run(memory.query("this text is ignored"))

    assert [c.content for c in result.results] == ["only item"]


def test_metadata_round_trips_fully_opaque(memory: MnemosyneMemory) -> None:
    _run(
        memory.add(
            MemoryContent(
                content="note",
                mime_type=MemoryMimeType.TEXT,
                metadata={"anything": "goes", "nested": {"a": 1}},
            )
        )
    )

    result = _run(memory.query(""))

    assert result.results[0].metadata == {"anything": "goes", "nested": {"a": 1}}


def test_json_content_round_trips(memory: MnemosyneMemory) -> None:
    _run(memory.add(MemoryContent(content={"tier": "enterprise"}, mime_type=MemoryMimeType.JSON)))

    result = _run(memory.query(""))

    assert result.results[0].content == {"tier": "enterprise"}


def test_bytes_content_round_trips(memory: MnemosyneMemory) -> None:
    payload = b"\x00\x01\xff not valid utf-8 \xfe"
    _run(memory.add(MemoryContent(content=payload, mime_type=MemoryMimeType.BINARY)))

    result = _run(memory.query(""))

    assert result.results[0].content == payload
    assert isinstance(result.results[0].content, bytes)


def test_image_content_round_trips(memory: MnemosyneMemory) -> None:
    from PIL import Image as PILImage

    pil_image = PILImage.new("RGB", (2, 2), color=(255, 0, 0))
    _run(memory.add(MemoryContent(content=Image.from_pil(pil_image), mime_type=MemoryMimeType.IMAGE)))

    result = _run(memory.query(""))

    restored = result.results[0].content
    assert isinstance(restored, Image)
    assert restored.image.size == (2, 2)


def test_update_context_is_byte_identical_to_list_memory() -> None:
    """The real DoD: the same content through MnemosyneMemory.update_context
    and ListMemory.update_context must produce the same SystemMessage text.
    """
    contents = [
        MemoryContent(content="the acme account is on the enterprise plan", mime_type=MemoryMimeType.TEXT),
        MemoryContent(content="support ticket 4821 was resolved", mime_type=MemoryMimeType.TEXT),
    ]

    async def go(path: str) -> tuple[str, str]:
        mnem.init(path)
        mnemosyne_memory = MnemosyneMemory("agent-a", path)
        for content in contents:
            await mnemosyne_memory.add(content)
        mnemosyne_ctx = UnboundedChatCompletionContext()
        await mnemosyne_memory.update_context(mnemosyne_ctx)

        list_memory = ListMemory(memory_contents=list(contents))
        list_ctx = UnboundedChatCompletionContext()
        await list_memory.update_context(list_ctx)

        mnemosyne_messages = await mnemosyne_ctx.get_messages()
        list_messages = await list_ctx.get_messages()
        return mnemosyne_messages[0].content, list_messages[0].content

    import tempfile

    with tempfile.TemporaryDirectory() as d:
        mnemosyne_text, list_memory_text = asyncio.run(go(d))

    assert mnemosyne_text == list_memory_text


def test_update_context_with_no_items_adds_no_message(memory: MnemosyneMemory) -> None:
    ctx = UnboundedChatCompletionContext()
    result = _run(memory.update_context(ctx))

    assert result.memories.results == []
    assert _run(ctx.get_messages()) == []
