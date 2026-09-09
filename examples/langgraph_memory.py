"""A LangGraph agent whose long-term memory is a Mnemosyne store.

    python examples/langgraph_memory.py     # needs mnem-langgraph

Shows memory that outlives a single run, a hypothesis branch that never touches
the main line, and `why` tracing a stored fact to where it came from.
"""

from __future__ import annotations

import tempfile
from typing import TypedDict

from langgraph.graph import START, StateGraph
from langgraph.store.base import BaseStore
from mnem_langgraph import MnemosyneStore

import mnem

ACCOUNT = ("memories", "acme")


class State(TypedDict):
    note: str


def learn(state: State, *, store: BaseStore) -> State:
    store.put(
        ACCOUNT, "plan",
        {"tier": "enterprise", "_meta": {"source": "ticket-4821"}},
    )
    store.put(ACCOUNT, "seats", {"count": 240})
    return {"note": "learned the account"}


def recall(state: State, *, store: BaseStore) -> State:
    plan = store.get(ACCOUNT, "plan")
    return {"note": f"the account is on the {plan.value['tier']} plan"}


def run(store_dir: str) -> None:
    mnem.init(store_dir)
    store = MnemosyneStore(store_dir)

    learner = (
        StateGraph(State).add_node("learn", learn).add_edge(START, "learn").compile(store=store)
    )
    recaller = (
        StateGraph(State).add_node("recall", recall).add_edge(START, "recall").compile(store=store)
    )

    # one run learns; a separate run still remembers, because the store is the
    # long-term memory, not the run's state
    learner.invoke({"note": ""})
    remembered = recaller.invoke({"note": ""})["note"]
    print(remembered)

    # test "what if the account churned" on a branch; main is untouched
    store.branch("what-if-churn")
    store.switch("what-if-churn")
    store.put(ACCOUNT, "plan", {"tier": "cancelled"})
    print("on the branch, plan is:", store.get(ACCOUNT, "plan").value["tier"])
    store.switch("main")
    print("back on main, plan is: ", store.get(ACCOUNT, "plan").value["tier"])

    blame = store.why(ACCOUNT, "plan")
    print(f"'plan' was set by {blame.commit[:8]}, from {blame.provenance.source}")

    assert remembered == "the account is on the enterprise plan"
    assert store.get(ACCOUNT, "plan").value["tier"] == "enterprise"
    assert blame.provenance.source == "ticket-4821"
    print("\nOK")


if __name__ == "__main__":
    with tempfile.TemporaryDirectory() as directory:
        run(directory)
