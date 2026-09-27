"""Tests for pop_item and clear_session (#306, ADR-0020)."""

import asyncio

import mnem
import pytest

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


def test_pop_item_on_empty_session_returns_none(session: MnemosyneSession) -> None:
    assert _run(session.pop_item()) is None


def test_pop_item_removes_exactly_the_most_recent_item(session: MnemosyneSession) -> None:
    items = [{"role": "user", "content": "a"}, {"role": "user", "content": "b"}]
    _run(session.add_items(items))
    popped = _run(session.pop_item())
    assert popped == items[-1]
    assert _run(session.get_items()) == items[:-1]


def test_popped_item_is_a_tombstone_not_a_hard_delete(
    session: MnemosyneSession, store: mnem.Store
) -> None:
    _run(session.add_items([{"role": "user", "content": "a"}]))
    before_commit = store.head_commit()
    _run(session.pop_item())

    assert _run(session.get_items()) == []  # gone from the live read
    # but the store's full history still has it - state_at the prior commit
    assert before_commit is not None
    old_state = store.state_at(before_commit)
    item_nodes = [n for nid, n in old_state.items() if nid != _seq_node_id("u1")]
    assert [n.content for n in item_nodes] == [{"role": "user", "content": "a"}]


def test_pop_then_add_does_not_collide_ids(session: MnemosyneSession) -> None:
    _run(session.add_items([{"role": "user", "content": "a"}]))
    _run(session.pop_item())
    _run(session.add_items([{"role": "user", "content": "b"}]))
    assert _run(session.get_items()) == [{"role": "user", "content": "b"}]


def test_clear_session_empties_get_items(session: MnemosyneSession) -> None:
    items = [{"role": "user", "content": str(i)} for i in range(3)]
    _run(session.add_items(items))
    _run(session.clear_session())
    assert _run(session.get_items()) == []


def test_clear_session_preserves_commit_history(
    session: MnemosyneSession, store: mnem.Store
) -> None:
    items = [{"role": "user", "content": str(i)} for i in range(3)]
    _run(session.add_items(items))
    before_commit = store.head_commit()
    log_before = len(store.log())

    _run(session.clear_session())

    assert len(store.log()) == log_before + 1  # one new commit, none erased
    assert before_commit is not None
    old_state = store.state_at(before_commit)
    item_nodes = [n for nid, n in old_state.items() if nid != _seq_node_id("u1")]
    assert len(item_nodes) == 3  # still recoverable at the prior commit


def test_clear_session_resets_seq_for_the_next_write(
    session: MnemosyneSession, store: mnem.Store
) -> None:
    _run(session.add_items([{"role": "user", "content": "a"}]))
    _run(session.clear_session())
    assert store.working_node(_seq_node_id("u1")) is None
    _run(session.add_items([{"role": "user", "content": "b"}]))
    assert _run(session.get_items()) == [{"role": "user", "content": "b"}]


def test_clear_session_on_empty_session_is_a_noop(
    session: MnemosyneSession, store: mnem.Store
) -> None:
    log_before = len(store.log())
    _run(session.clear_session())
    assert len(store.log()) == log_before
    assert _run(session.get_items()) == []


def test_clear_session_is_one_batched_commit(
    session: MnemosyneSession, store: mnem.Store
) -> None:
    items = [{"role": "user", "content": str(i)} for i in range(5)]
    _run(session.add_items(items))
    log_before = len(store.log())
    _run(session.clear_session())
    assert len(store.log()) == log_before + 1  # one commit, however many nodes
