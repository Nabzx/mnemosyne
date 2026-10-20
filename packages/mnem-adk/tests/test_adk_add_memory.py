"""Tests for add_memory (#592, #571, ADR-0026) - against real
google.adk.memory.memory_entry.MemoryEntry objects, not mocks."""

import asyncio

import mnem
import pytest
from google.adk.memory.memory_entry import MemoryEntry
from google.genai import types

from mnem_adk import MnemosyneMemoryService
from mnem_adk._ids import memory_node_id, memory_seq_node_id, next_memory_node_id


def _run(coro):
    return asyncio.run(coro)


def _memory(text: str, *, memory_id: str | None = None, author: str | None = None) -> MemoryEntry:
    return MemoryEntry(
        content=types.Content(role="user", parts=[types.Part(text=text)]),
        id=memory_id,
        author=author,
    )


@pytest.fixture
def store(tmp_path: object) -> mnem.Store:
    return mnem.init(tmp_path)


@pytest.fixture
def service(store: mnem.Store) -> MnemosyneMemoryService:
    return MnemosyneMemoryService(store)


def test_an_entry_with_its_own_id_uses_it_literally(
    service: MnemosyneMemoryService, store: mnem.Store
) -> None:
    _run(
        service.add_memory(
            app_name="myapp", user_id="u1", memories=[_memory("hi", memory_id="mem-1")]
        )
    )
    assert store.working_node(memory_node_id("myapp", "u1", "mem-1")) is not None
    assert store.working_node(memory_seq_node_id("myapp", "u1")) is None


def test_an_entry_with_no_id_gets_the_next_sequence_number(
    service: MnemosyneMemoryService, store: mnem.Store
) -> None:
    _run(service.add_memory(app_name="myapp", user_id="u1", memories=[_memory("hi")]))
    expected = next_memory_node_id("myapp", "u1", 0)
    assert store.working_node(expected) is not None
    counter = store.working_node(memory_seq_node_id("myapp", "u1"))
    assert counter is not None
    assert counter.content == 1


def test_sequence_numbers_keep_advancing_across_calls(
    service: MnemosyneMemoryService, store: mnem.Store
) -> None:
    _run(service.add_memory(app_name="myapp", user_id="u1", memories=[_memory("a")]))
    _run(service.add_memory(app_name="myapp", user_id="u1", memories=[_memory("b")]))
    assert store.working_node(next_memory_node_id("myapp", "u1", 0)) is not None
    assert store.working_node(next_memory_node_id("myapp", "u1", 1)) is not None


def test_a_mixed_batch_assigns_sequence_numbers_only_to_entries_with_no_id(
    service: MnemosyneMemoryService, store: mnem.Store
) -> None:
    _run(
        service.add_memory(
            app_name="myapp",
            user_id="u1",
            memories=[_memory("a", memory_id="explicit"), _memory("b"), _memory("c")],
        )
    )
    assert store.working_node(memory_node_id("myapp", "u1", "explicit")) is not None
    assert store.working_node(next_memory_node_id("myapp", "u1", 0)) is not None
    assert store.working_node(next_memory_node_id("myapp", "u1", 1)) is not None
    counter = store.working_node(memory_seq_node_id("myapp", "u1"))
    assert counter.content == 2


def test_an_empty_batch_is_a_noop(service: MnemosyneMemoryService, store: mnem.Store) -> None:
    before = len(store.log())
    _run(service.add_memory(app_name="myapp", user_id="u1", memories=[]))
    assert len(store.log()) == before


def test_provenance_source_comes_from_author(
    service: MnemosyneMemoryService, store: mnem.Store
) -> None:
    _run(
        service.add_memory(
            app_name="myapp", user_id="u1", memories=[_memory("hi", memory_id="m1", author="support-bot")]
        )
    )
    node = store.working_node(memory_node_id("myapp", "u1", "m1"))
    assert node.provenance.source == "support-bot"


def test_different_users_do_not_share_a_sequence_counter(
    service: MnemosyneMemoryService, store: mnem.Store
) -> None:
    _run(service.add_memory(app_name="myapp", user_id="u1", memories=[_memory("a")]))
    _run(service.add_memory(app_name="myapp", user_id="u2", memories=[_memory("b")]))
    assert store.working_node(next_memory_node_id("myapp", "u1", 0)) is not None
    assert store.working_node(next_memory_node_id("myapp", "u2", 0)) is not None
