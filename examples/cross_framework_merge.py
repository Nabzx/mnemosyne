"""Two different framework adapters - CrewAI and LangGraph - writing to
their own branches of one shared store, which then merge together:
CrewAI's own branch surfaces a real conflict against another CrewAI
branch, LangGraph's own independent branch merges in cleanly alongside
it. Proves `merge`'s own per-node-id algorithm (ADR-0013) needs zero new
code to work across adapters sharing one store: it has no awareness of
which adapter wrote a node, and the two adapters' own real id schemes
never collide with each other (#362).

    python examples/cross_framework_merge.py     # needs mnem-crewai, mnem-langgraph

Checked directly before writing this, not assumed: CrewAI's real id
scheme always has a `/` prefix (`{scope.rstrip("/")}/{record_id}`), and
LangGraph's own `put()` *requires* a non-empty namespace (confirmed
against its real validation, `_validate_namespace`), which always inserts
at least one `:` via `":".join([*namespace, key])`. The two can never
produce an identical literal node id through their real public
interfaces - a stronger, better-grounded claim than "unlikely to
collide," since it rules collision out structurally for this pair
specifically (not true of every pair - #362 found a real, guaranteed
collision between the OpenAI Agents SDK and AutoGen adapters instead).

redb allows only one open handle per store per process (#311, #363), so
each adapter's own backend is opened, used, and dropped before the next
one opens - each helper function below returns before the next starts,
letting the handle's refcount hit zero.
"""

from __future__ import annotations

import tempfile

import mnem

CASE_RECORD_ID = "support/case-4821"
CASE_NODE_ID = f"/{CASE_RECORD_ID}"


def _crewai_seed_and_diverge(store_dir: str) -> None:
    """CrewAI opens the shared store, logs the original fact on `main`,
    then updates it two different ways on two branches - the real
    conflict this example surfaces."""
    from crewai.memory.types import MemoryRecord
    from mnem_crewai import MnemosyneStorageBackend

    backend = MnemosyneStorageBackend(store_dir)
    backend.save(
        [MemoryRecord(id=CASE_RECORD_ID, scope="/", content="ticket opened: shipping delay")]
    )
    store = backend._store

    store.new_branch("crewai-resolution")
    store.checkout("crewai-resolution")
    backend.update(
        MemoryRecord(id=CASE_RECORD_ID, scope="/", content="ticket resolved: refunded shipping cost")
    )
    store.checkout("main")

    store.new_branch("crewai-escalation", start="main")
    store.checkout("crewai-escalation")
    backend.update(
        MemoryRecord(id=CASE_RECORD_ID, scope="/", content="ticket escalated: compensation requested")
    )
    store.checkout("main")


def _langgraph_track_independently(store_dir: str) -> None:
    """LangGraph opens the same shared store and adds its own tracking
    note, under its own namespace, on its own branch - independent of
    CrewAI's own record, proving the two coexist in one store without
    stepping on each other."""
    from mnem_langgraph import MnemosyneStore

    store_wrapper = MnemosyneStore(store_dir)
    store = store_wrapper._store
    store.new_branch("langgraph-tracking", start="main")
    store.checkout("langgraph-tracking")
    store_wrapper.put(("tracking",), "case-4821-sla", {"content": "SLA clock started"})
    store.checkout("main")


def run(store_dir: str) -> None:
    mnem.init(store_dir)
    _crewai_seed_and_diverge(store_dir)
    _langgraph_track_independently(store_dir)

    store = mnem.open(store_dir)

    print("merging crewai-resolution into main:")
    clean = store.merge("crewai-resolution", time_ms=1_000)
    print(f"  status: {clean.status}  commit: {clean.commit[:12] if clean.commit else None}")

    print("\nmerging crewai-escalation into main:")
    conflicted = store.merge("crewai-escalation", time_ms=2_000)
    ours = conflicted.conflicts[0].ours.content
    theirs = conflicted.conflicts[0].theirs.content
    print(f"  conflict on {CASE_NODE_ID}:")
    print(f'    ours   (the resolution, already landed): "{ours["content"]}"')
    print(f'    theirs (the escalation, from main):       "{theirs["content"]}"')

    print("\nkeeping theirs - the escalation is the more recent real decision:")
    merged = store.merge(
        "crewai-escalation",
        resolutions={CASE_NODE_ID: "theirs"},
        message="merge crewai-escalation: compensation request supersedes the earlier refund",
        time_ms=3_000,
    )
    print(f"  commit: {merged.commit[:12]}")

    print("\nmerging langgraph-tracking into main (a different framework, no conflict):")
    tracking_merge = store.merge("langgraph-tracking", time_ms=4_000)
    print(f"  status: {tracking_merge.status}  commit: {tracking_merge.commit[:12]}")

    final_case = store.working_node(CASE_NODE_ID).content
    final_tracking = store.working_node("tracking:case-4821-sla").content

    assert clean.status == "fast-forwarded"
    assert conflicted.status == "conflicts"
    assert ours["content"] == "ticket resolved: refunded shipping cost"
    assert theirs["content"] == "ticket escalated: compensation requested"
    assert merged.status == "merged"
    assert final_case["content"] == "ticket escalated: compensation requested"
    assert tracking_merge.status == "merged"
    assert final_tracking["content"] == "SLA clock started"
    print("\nOK")


if __name__ == "__main__":
    with tempfile.TemporaryDirectory() as directory:
        run(directory)
