"""``mnem-langgraph``: a LangGraph :class:`~langgraph.store.base.BaseStore`
backed by a Mnemosyne store (ADR-0016).

Every write is a commit, so the agent's long-term memory has history. `branch`
and `why` are extra methods a graph node can call directly::

    from mnem_langgraph import MnemosyneStore

    store = MnemosyneStore("./agent-memory")
    store.put(("memories", "u1"), "plan", {"tier": "enterprise",
                                           "_meta": {"source": "ticket-4821"}})
    item = store.get(("memories", "u1"), "plan")
    blame = store.why(("memories", "u1"), "plan")   # which commit, and from where
"""

from .store import MnemosyneStore

__all__ = ["MnemosyneStore"]
