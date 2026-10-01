"""Tests for the node id scheme and sequence counter (#318, ADR-0023)."""

import mnem
import pytest

from mnem_autogen.memory import (
    _item_id,
    _read_items,
    _read_seq,
    _seq_node_id,
    _write_item,
)


@pytest.fixture
def store(tmp_path: object) -> mnem.Store:
    return mnem.init(tmp_path)


def test_item_id_is_zero_padded() -> None:
    assert _item_id("agent-a", 0) == "agent-a:0000000000"
    assert _item_id("agent-a", 9) == "agent-a:0000000009"
    assert _item_id("agent-a", 10) == "agent-a:0000000010"


def test_seq_node_id() -> None:
    assert _seq_node_id("agent-a") == "agent-a:_seq"


def test_write_item_returns_its_id(store: mnem.Store) -> None:
    assert _write_item(store, "agent-a", "a") == "agent-a:0000000000"
    assert _write_item(store, "agent-a", "b") == "agent-a:0000000001"
    assert _write_item(store, "agent-a", "c") == "agent-a:0000000002"


def test_seq_increases_monotonically_across_writes(store: mnem.Store) -> None:
    assert _read_seq(store, "agent-a") == 0
    _write_item(store, "agent-a", "a")
    assert _read_seq(store, "agent-a") == 1
    _write_item(store, "agent-a", "b")
    assert _read_seq(store, "agent-a") == 2


def test_sorts_correctly_past_nine_items(store: mnem.Store) -> None:
    contents = [f"item-{i}" for i in range(12)]
    for content in contents:
        _write_item(store, "agent-a", content)
    assert [node.content for node in _read_items(store, "agent-a")] == contents


def test_seq_node_never_leaks_into_a_read(store: mnem.Store) -> None:
    _write_item(store, "agent-a", "a")
    _write_item(store, "agent-a", "b")
    items = _read_items(store, "agent-a")
    assert _seq_node_id("agent-a") not in [node.id for node in items]
    assert len(items) == 2
    # the counter node itself is real, just excluded from _read_items
    assert store.working_node(_seq_node_id("agent-a")) is not None


def test_read_items_scoped_to_one_memory_instance(store: mnem.Store) -> None:
    _write_item(store, "agent-a", "a")
    _write_item(store, "agent-b", "x")
    _write_item(store, "agent-b", "y")
    assert [n.content for n in _read_items(store, "agent-a")] == ["a"]
    assert [n.content for n in _read_items(store, "agent-b")] == ["x", "y"]


def test_read_seq_is_zero_for_an_unwritten_name(store: mnem.Store) -> None:
    assert _read_seq(store, "never-written") == 0
    assert _read_items(store, "never-written") == []
