"""Tests for why/bisect (#321, ADR-0023) - extra methods outside the
Memory protocol, same shape as the LangGraph, OpenAI Agents SDK, and
CrewAI adapters' own (ADR-0016/0020/0021)."""

import asyncio

import mnem
import pytest
from autogen_core.memory import MemoryContent, MemoryMimeType

from mnem_autogen import MnemosyneMemory
from mnem_autogen.memory import _item_id


def _run(coro):
    return asyncio.run(coro)


@pytest.fixture
def memory(tmp_path: object) -> MnemosyneMemory:
    return MnemosyneMemory("agent-a", str(mnem.init(tmp_path).root))


def test_why_returns_the_commit_that_wrote_the_item(memory: MnemosyneMemory) -> None:
    _run(memory.add(MemoryContent(content="first", mime_type=MemoryMimeType.TEXT)))

    item_id = _item_id("agent-a", 0)
    blame = memory.why(item_id)

    assert blame.commit == memory._store.head_commit()
    assert blame.node.content["content"] == "first"


def test_why_raises_for_an_unknown_item_id(memory: MnemosyneMemory) -> None:
    with pytest.raises(mnem.InvalidRefError):
        memory.why(_item_id("agent-a", 0))


def test_bisect_finds_the_commit_where_an_item_first_appears(memory: MnemosyneMemory) -> None:
    _run(memory.add(MemoryContent(content="first", mime_type=MemoryMimeType.TEXT)))
    first_commit = memory._store.head_commit()

    _run(memory.add(MemoryContent(content="second", mime_type=MemoryMimeType.TEXT)))
    second_id = _item_id("agent-a", 1)

    boundary = memory.bisect(lambda working_memory: second_id in working_memory)

    assert boundary == memory._store.head_commit()
    assert boundary != first_commit
