"""Tests for clear/close (#320, ADR-0023)."""

import asyncio

import mnem
import pytest
from autogen_core.memory import MemoryContent, MemoryMimeType

from mnem_autogen import MnemosyneMemory
from mnem_autogen.memory import _seq_node_id


def _run(coro):
    return asyncio.run(coro)


@pytest.fixture
def memory(tmp_path: object) -> MnemosyneMemory:
    return MnemosyneMemory("agent-a", str(mnem.init(tmp_path).root))


def test_clear_empties_query_results(memory: MnemosyneMemory) -> None:
    _run(memory.add(MemoryContent(content="a", mime_type=MemoryMimeType.TEXT)))
    _run(memory.add(MemoryContent(content="b", mime_type=MemoryMimeType.TEXT)))

    _run(memory.clear())

    assert _run(memory.query("")).results == []


def test_clear_is_a_tombstone_not_a_hard_delete(memory: MnemosyneMemory) -> None:
    _run(memory.add(MemoryContent(content="a", mime_type=MemoryMimeType.TEXT)))
    before_commit = memory._store.head_commit()

    _run(memory.clear())

    old_state = memory._store.state_at(before_commit)
    assert old_state["agent-a:0000000000"].content["content"] == "a"


def test_clear_resets_the_seq_counter(memory: MnemosyneMemory) -> None:
    _run(memory.add(MemoryContent(content="a", mime_type=MemoryMimeType.TEXT)))
    _run(memory.add(MemoryContent(content="b", mime_type=MemoryMimeType.TEXT)))

    _run(memory.clear())
    _run(memory.add(MemoryContent(content="fresh start", mime_type=MemoryMimeType.TEXT)))

    assert memory._store.working_node(_seq_node_id("agent-a")).content == 1
    assert [c.content for c in _run(memory.query("")).results] == ["fresh start"]


def test_clear_with_nothing_to_forget_is_a_real_noop(memory: MnemosyneMemory) -> None:
    log_before = len(memory._store.log())

    _run(memory.clear())

    assert len(memory._store.log()) == log_before  # no new commit


def test_close_releases_the_store_handle_for_another_instance(tmp_path: object) -> None:
    path = str(mnem.init(tmp_path).root)
    memory = MnemosyneMemory("agent-a", path)
    _run(memory.add(MemoryContent(content="a", mime_type=MemoryMimeType.TEXT)))

    _run(memory.close())

    # redb allows exactly one open handle per store per process - this
    # would raise StoreIoError if close() hadn't actually released it.
    reopened = MnemosyneMemory("agent-a", path)
    assert [c.content for c in _run(reopened.query("")).results] == ["a"]
