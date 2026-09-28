"""Tests for branch/switch/merge/why/bisect/history (#308, ADR-0020)."""

import asyncio

import mnem
import pytest

from mnem_openai_agents import MnemosyneSession


def _run(coro):
    return asyncio.run(coro)


@pytest.fixture
def store(tmp_path: object) -> mnem.Store:
    return mnem.init(tmp_path)


@pytest.fixture
def session(store: mnem.Store) -> MnemosyneSession:
    return MnemosyneSession(store, "u1")


def test_branch_creates_without_switching(session: MnemosyneSession, store: mnem.Store) -> None:
    _run(session.add_items([{"role": "user", "content": "a"}]))
    before = store.head()
    session.branch("hypothesis")
    assert store.head() == before  # still on the original branch
    assert any(name == "hypothesis" for name, _ in store.branches())


def test_switch_moves_head(session: MnemosyneSession, store: mnem.Store) -> None:
    _run(session.add_items([{"role": "user", "content": "a"}]))  # a branch needs a real commit
    session.branch("hypothesis")
    session.switch("hypothesis")
    assert store.head() == "ref: hypothesis"


def test_why_traces_an_item_to_its_commit_and_provenance(
    session: MnemosyneSession, store: mnem.Store
) -> None:
    provenance = mnem.Provenance(source="lookup_order")
    from mnem_openai_agents.session import _write_items

    ids = _write_items(store, "u1", [{"content": "shipped"}], provenances=[provenance])
    blame = session.why(ids[0])
    assert blame.node.content == {"content": "shipped"}
    assert blame.provenance.source == "lookup_order"


def test_history_matches_store_log(session: MnemosyneSession, store: mnem.Store) -> None:
    _run(session.add_items([{"role": "user", "content": "a"}]))
    _run(session.add_items([{"role": "user", "content": "b"}]))
    assert [c.id for c in session.history()] == [c.id for c in store.log()]
    assert [c.id for c in session.history(limit=1)] == [c.id for c in store.log(limit=1)]


def _fork_and_diverge(session: MnemosyneSession, store: mnem.Store) -> None:
    """The real scenario examples/claude_agent.py uses: two branches each
    add an item from the same shared _seq counter value, producing a
    genuine add/add conflict on the same item id when merged - all through
    the session's own ordinary API, no hand-crafted node ids."""
    _run(session.add_items([{"role": "user", "content": "shared base item"}]))
    fork_point = store.head()

    session.branch("hypothesis")
    session.switch("hypothesis")
    _run(session.add_items([{"role": "assistant", "content": "confirmed by phone"}]))

    session.switch(fork_point.removeprefix("ref: "))
    _run(session.add_items([{"role": "assistant", "content": "misread from billing note"}]))


def test_merge_surfaces_a_real_conflict_then_resolves_it(
    session: MnemosyneSession, store: mnem.Store
) -> None:
    _fork_and_diverge(session, store)

    conflicted = session.merge("hypothesis")
    assert conflicted.status == "conflicts"
    assert len(conflicted.conflicts) == 1
    conflict = conflicted.conflicts[0]
    assert conflict.kind == "add/add"
    assert conflict.ours.content == {"role": "assistant", "content": "misread from billing note"}
    assert conflict.theirs.content == {"role": "assistant", "content": "confirmed by phone"}

    merged = session.merge("hypothesis", resolutions={conflict.id: "theirs"})
    assert merged.status == "merged"
    assert store.working_memory()[conflict.id].content == {
        "role": "assistant",
        "content": "confirmed by phone",
    }


def test_bisect_finds_the_commit_that_added_an_item(
    session: MnemosyneSession, store: mnem.Store
) -> None:
    # Items are append-only (add_items never overwrites an existing id), so a
    # meaningful bisect here is presence-based, not content-equality: the
    # predicate is false at the store's root commit (the item doesn't exist
    # yet) and true from the commit that added it onward.
    _run(session.add_items([{"role": "user", "content": "v1"}]))
    _run(session.add_items([{"role": "user", "content": "v2"}]))
    second_commit = store.head_commit()
    target_id = "u1:0000000001"

    boundary = session.bisect(lambda memory: target_id in memory)
    assert boundary == second_commit
    assert store.state_at(boundary)[target_id].content == {"role": "user", "content": "v2"}
