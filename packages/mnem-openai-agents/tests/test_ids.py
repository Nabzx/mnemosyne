"""Tests for the node id scheme and sequence counter (#304, ADR-0020)."""

import mnem
import pytest

from mnem_openai_agents.session import (
    _item_id,
    _read_items,
    _read_seq,
    _seq_node_id,
    _write_items,
)


@pytest.fixture
def store(tmp_path: object) -> mnem.Store:
    return mnem.init(tmp_path)


def test_item_id_is_zero_padded() -> None:
    assert _item_id("u1", 0) == "u1:0000000000"
    assert _item_id("u1", 9) == "u1:0000000009"
    assert _item_id("u1", 10) == "u1:0000000010"


def test_seq_node_id() -> None:
    assert _seq_node_id("u1") == "u1:_seq"


def test_write_items_returns_ids_in_order(store: mnem.Store) -> None:
    ids = _write_items(store, "u1", ["a", "b", "c"])
    assert ids == ["u1:0000000000", "u1:0000000001", "u1:0000000002"]


def test_write_items_with_empty_contents_is_a_noop(store: mnem.Store) -> None:
    assert _write_items(store, "u1", []) == []
    assert _read_seq(store, "u1") == 0


def test_seq_increases_monotonically_across_writes(store: mnem.Store) -> None:
    assert _read_seq(store, "u1") == 0
    _write_items(store, "u1", ["a"])
    assert _read_seq(store, "u1") == 1
    _write_items(store, "u1", ["b", "c"])
    assert _read_seq(store, "u1") == 3
    _write_items(store, "u1", ["d"])
    assert _read_seq(store, "u1") == 4


def test_sorts_correctly_past_nine_items(store: mnem.Store) -> None:
    contents = [f"item-{i}" for i in range(12)]
    _write_items(store, "u1", contents)
    assert [node.content for node in _read_items(store, "u1")] == contents


def test_seq_node_never_leaks_into_a_read(store: mnem.Store) -> None:
    _write_items(store, "u1", ["a", "b"])
    items = _read_items(store, "u1")
    assert _seq_node_id("u1") not in [node.id for node in items]
    assert len(items) == 2
    # the counter node itself is real, just excluded from read_items
    assert store.working_node(_seq_node_id("u1")) is not None


def test_read_items_scoped_to_one_session(store: mnem.Store) -> None:
    _write_items(store, "u1", ["a"])
    _write_items(store, "u2", ["x", "y"])
    assert [n.content for n in _read_items(store, "u1")] == ["a"]
    assert [n.content for n in _read_items(store, "u2")] == ["x", "y"]
