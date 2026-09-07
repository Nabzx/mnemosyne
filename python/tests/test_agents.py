"""Tests for ``mnem.agents`` and the ``to_dict`` serialisers (#59)."""

import json

import pytest

import mnem
from mnem import agents
from mnem._mnem import ConflictError


def test_remember_is_one_commit_with_provenance(tmp_path: object) -> None:
    store = mnem.init(tmp_path)
    commit = agents.remember(
        store,
        "customer-4821",
        "on the Enterprise plan",
        source="ticket-4821",
        step="step-3",
        author="support-agent",
        time_ms=1000,
    )

    assert store.head_commit() == commit
    assert store.staged() == []  # nothing left half-staged

    node = store.working_node("customer-4821")
    assert node.content == "on the Enterprise plan"
    assert node.provenance.source == "ticket-4821"
    assert node.provenance.agent_step == "step-3"

    [c] = store.log()
    assert c.message == "remember customer-4821"
    assert c.author == "support-agent"


def test_remember_many_is_a_single_commit(tmp_path: object) -> None:
    store = mnem.init(tmp_path)
    commit = agents.remember_many(
        store,
        [
            {"id": "plan", "content": "enterprise", "source": "ticket-1"},
            {"id": "seats", "content": 240},
            {"id": "renewal", "content": "2027-01"},
        ],
        summary="learn the account",
        time_ms=1000,
    )

    assert store.head_commit() == commit
    assert len(store.log()) == 1
    mem = store.working_memory()
    assert set(mem) == {"plan", "seats", "renewal"}
    assert mem["seats"].content == 240
    assert mem["plan"].provenance.source == "ticket-1"


def test_forget_tombstones_in_one_commit(tmp_path: object) -> None:
    store = mnem.init(tmp_path)
    agents.remember(store, "temp", "scratch note", time_ms=1000)
    agents.remember(store, "keep", "real belief", time_ms=2000)

    commit = agents.forget(store, "temp", time_ms=3000)
    assert store.head_commit() == commit
    assert store.working_node("temp") is None
    assert store.working_node("keep").content == "real belief"
    assert store.log()[0].message == "forget temp"


def test_default_commit_messages(tmp_path: object) -> None:
    store = mnem.init(tmp_path)
    agents.remember(store, "a", 1, time_ms=1000)
    agents.remember_many(store, [{"id": "b", "content": 2}], time_ms=2000)
    agents.remember_many(
        store, [{"id": "c", "content": 3}, {"id": "d", "content": 4}], time_ms=3000
    )
    messages = [c.message for c in store.log()]
    assert messages == ["remember 2 nodes", "remember 1 node", "remember a"]


def test_to_dict_is_json_serialisable(tmp_path: object) -> None:
    store = mnem.init(tmp_path)
    agents.remember(
        store, "plan", "enterprise", source="ticket-4821", step="s1", time_ms=1000
    )
    first = store.head_commit()
    agents.remember(store, "plan", "pro", observation="obs-7", time_ms=2000)

    blame = store.blame("plan")
    bd = blame.to_dict()
    assert json.loads(json.dumps(bd)) == bd
    assert bd["commit"] == blame.commit
    assert bd["node"]["content"] == "pro"
    assert bd["node"]["provenance"]["observation"] == "obs-7"

    [change] = store.diff(first, None)
    cd = change.to_dict()
    assert cd["kind"] == "modified"
    assert cd["old"]["content"] == "enterprise"
    assert cd["new"]["content"] == "pro"
    assert json.loads(json.dumps(cd)) == cd

    commit = store.log()[0]
    md = commit.to_dict()
    assert md["parents"] == list(commit.parents)
    assert json.loads(json.dumps(md)) == md


def test_with_retry_gives_up_after_a_bounded_number_of_attempts() -> None:
    attempts = 0

    def always_conflicts() -> None:
        nonlocal attempts
        attempts += 1
        raise ConflictError("branch moved")

    with pytest.raises(ConflictError):
        agents._with_retry(always_conflicts)
    assert attempts == agents._RETRIES

    calls = 0

    def succeeds_on_the_third_try() -> str:
        nonlocal calls
        calls += 1
        if calls < 3:
            raise ConflictError("branch moved")
        return "ok"

    assert agents._with_retry(succeeds_on_the_third_try) == "ok"
    assert calls == 3
