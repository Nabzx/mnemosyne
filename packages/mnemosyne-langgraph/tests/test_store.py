"""Tests for MnemosyneStore (#58)."""

import asyncio

import mnem
import pytest

from mnemosyne_langgraph import MnemosyneStore


@pytest.fixture
def store(tmp_path: object) -> MnemosyneStore:
    return MnemosyneStore(str(mnem.init(tmp_path).root))


NS = ("memories", "u1")


def test_put_get_delete_round_trip(store: MnemosyneStore) -> None:
    store.put(NS, "plan", {"tier": "enterprise"})
    item = store.get(NS, "plan")
    assert item is not None
    assert item.namespace == NS
    assert item.key == "plan"
    assert item.value == {"tier": "enterprise"}

    store.put(NS, "plan", {"tier": "pro"})
    assert store.get(NS, "plan").value == {"tier": "pro"}

    store.delete(NS, "plan")
    assert store.get(NS, "plan") is None

    # every write is a commit
    assert len(store.history(limit=10)) == 3


def test_provenance_via_meta_reaches_blame(store: MnemosyneStore) -> None:
    store.put(NS, "plan", {"tier": "enterprise", "_meta": {"source": "ticket-4821"}})
    store.put(NS, "plan", {"tier": "pro", "_meta": {"source": "billing-8842", "step": "s31"}})

    blame = store.why(NS, "plan")
    assert blame.node.content["tier"] == "pro"
    assert blame.provenance.source == "billing-8842"
    assert blame.provenance.agent_step == "s31"


def test_search_filters_and_lists_namespaces(store: MnemosyneStore) -> None:
    store.put(("memories", "u1"), "plan", {"tier": "enterprise"})
    store.put(("memories", "u1"), "seats", {"count": 240})
    store.put(("memories", "u2"), "plan", {"tier": "pro"})
    store.put(("prefs", "u1"), "theme", {"mode": "dark"})

    hits = store.search(("memories",))
    assert {(h.namespace, h.key) for h in hits} == {
        (("memories", "u1"), "plan"),
        (("memories", "u1"), "seats"),
        (("memories", "u2"), "plan"),
    }

    filtered = store.search(("memories",), filter={"tier": "pro"})
    assert [(h.namespace, h.key) for h in filtered] == [(("memories", "u2"), "plan")]

    q = store.search(("memories",), query="enterprise")
    assert [h.key for h in q] == ["plan"]

    assert set(store.list_namespaces()) == {
        ("memories", "u1"),
        ("memories", "u2"),
        ("prefs", "u1"),
    }
    assert set(store.list_namespaces(prefix=("memories",))) == {
        ("memories", "u1"),
        ("memories", "u2"),
    }


def test_branch_isolates_an_experiment(store: MnemosyneStore) -> None:
    store.put(NS, "plan", {"tier": "enterprise"})
    store.branch("what-if")
    store.switch("what-if")
    store.put(NS, "plan", {"tier": "pro"})
    assert store.get(NS, "plan").value == {"tier": "pro"}

    store.switch("main")
    assert store.get(NS, "plan").value == {"tier": "enterprise"}


def test_a_graph_node_uses_it_as_memory(store: MnemosyneStore) -> None:
    from typing import TypedDict

    from langgraph.graph import START, StateGraph

    class State(TypedDict):
        tier: str

    from langgraph.store.base import BaseStore

    def recall_plan(state: State, *, store: BaseStore) -> State:
        store.put(NS, "plan", {"tier": "enterprise", "_meta": {"source": "ticket-1"}})
        return {"tier": store.get(NS, "plan").value["tier"]}

    graph = (
        StateGraph(State)
        .add_node("recall_plan", recall_plan)
        .add_edge(START, "recall_plan")
        .compile(store=store)
    )
    assert graph.invoke({"tier": ""})["tier"] == "enterprise"
    assert store.why(NS, "plan").provenance.source == "ticket-1"


def test_async_methods_do_not_need_a_running_loop(store: MnemosyneStore) -> None:
    async def go() -> None:
        await store.aput(NS, "plan", {"tier": "enterprise"})
        item = await store.aget(NS, "plan")
        assert item.value == {"tier": "enterprise"}
        hits = await store.asearch(("memories",), query="enterprise")
        assert [h.key for h in hits] == ["plan"]

    asyncio.run(go())
