"""Tests for get_items and add_items (#305, ADR-0020)."""

import asyncio

import mnem
import pytest
from agents.memory.session_settings import SessionSettings

from mnem_openai_agents import MnemosyneSession
from mnem_openai_agents.session import _seq_node_id


def _run(coro):
    return asyncio.run(coro)


@pytest.fixture
def store(tmp_path: object) -> mnem.Store:
    return mnem.init(tmp_path)


@pytest.fixture
def session(store: mnem.Store) -> MnemosyneSession:
    return MnemosyneSession(store, "u1")


def test_round_trip_preserves_order(session: MnemosyneSession) -> None:
    items = [{"role": "user", "content": "hi"}, {"role": "assistant", "content": "hello"}]
    _run(session.add_items(items))
    assert _run(session.get_items()) == items


def test_multiple_add_items_calls_accumulate_in_order(session: MnemosyneSession) -> None:
    first = [{"role": "user", "content": "one"}]
    second = [{"role": "assistant", "content": "two"}, {"role": "user", "content": "three"}]
    _run(session.add_items(first))
    _run(session.add_items(second))
    assert _run(session.get_items()) == first + second


def test_get_items_limit_takes_the_tail(session: MnemosyneSession) -> None:
    items = [{"role": "user", "content": str(i)} for i in range(5)]
    _run(session.add_items(items))
    assert _run(session.get_items(limit=2)) == items[-2:]


def test_get_items_limit_zero_is_empty(session: MnemosyneSession) -> None:
    _run(session.add_items([{"role": "user", "content": "hi"}]))
    assert _run(session.get_items(limit=0)) == []


def test_get_items_no_limit_returns_everything(session: MnemosyneSession) -> None:
    items = [{"role": "user", "content": str(i)} for i in range(12)]
    _run(session.add_items(items))
    assert _run(session.get_items()) == items


def test_session_settings_limit_used_when_call_limit_unset(store: mnem.Store) -> None:
    session = MnemosyneSession(store, "u1", session_settings=SessionSettings(limit=1))
    items = [{"role": "user", "content": "a"}, {"role": "user", "content": "b"}]
    _run(session.add_items(items))
    assert _run(session.get_items()) == items[-1:]


def test_call_limit_overrides_session_settings_limit(store: mnem.Store) -> None:
    session = MnemosyneSession(store, "u1", session_settings=SessionSettings(limit=1))
    items = [{"role": "user", "content": "a"}, {"role": "user", "content": "b"}]
    _run(session.add_items(items))
    assert _run(session.get_items(limit=2)) == items


def test_add_items_is_one_commit_and_one_node_per_item(session: MnemosyneSession) -> None:
    items = [{"role": "user", "content": str(i)} for i in range(3)]
    before = len(session._store.log())
    _run(session.add_items(items))
    assert len(session._store.log()) == before + 1  # one commit for the whole batch

    memory = session._store.working_memory()
    item_nodes = [
        node_id
        for node_id in memory
        if node_id.startswith("u1:") and node_id != _seq_node_id("u1")
    ]
    assert len(item_nodes) == 3  # one node per item, not one per batch


def test_add_items_empty_list_is_a_noop(session: MnemosyneSession) -> None:
    before = len(session._store.log())
    _run(session.add_items([]))
    assert len(session._store.log()) == before
    assert _run(session.get_items()) == []


def test_content_stored_as_is_no_reinterpretation(session: MnemosyneSession) -> None:
    item = {"role": "tool", "tool_call_id": "call_1", "content": [{"type": "text", "text": "x"}]}
    _run(session.add_items([item]))
    assert _run(session.get_items()) == [item]
