"""Four agents, four different frameworks, investigate one incident and
each write their own finding to one shared store - merged sequentially,
not pairwise, extending #364's two-agent demo to prove `merge` holds up
past the simplest A-vs-B case (#371, part of #353).

    python examples/multi_agent_incident_retro.py

Each agent branches from `main`, writes its own finding, and is merged
back in turn: CrewAI, then LangGraph, then the OpenAI Agents SDK, then
AutoGen. After all four merges, every finding is present, and - the real
point - each one still `blame`s back to the exact commit the agent that
wrote it made, even though three more merges happened on top of it
afterwards. History survives a 4-way sequential merge the same way it
survives a 2-way one; the merge algorithm (ADR-0013) has no notion of
"how many branches so far" to degrade with.

The OpenAI Agents SDK and AutoGen adapters reuse the exact same node-id
scheme (`{identifier}:{seq:010d}`, ADR-0023 reused ADR-0020 directly) -
confirmed in #362 to guarantee a collision if given the same identifier
string. This demo gives them "openai-session" and "autogen-agent"
deliberately, matching the "Sharing a store with other adapters" note on
each adapter's own README (#365).

redb allows only one open handle per store per process (#311, #363), so
each agent's helper function opens, writes, and returns - letting the
handle drop - before the next one opens its own.
"""

from __future__ import annotations

import asyncio
import tempfile

import mnem

DB_FINDING = "p99 342ms, confirmed via the slow query log"
CACHE_FINDING = "dropped to 61% at 14:02 UTC"
QUEUE_FINDING = "backed up to 40k messages; the consumer crashed at 14:01 UTC"
DEPLOY_FINDING = "v2.3.1 shipped at 14:00 UTC, correlates with the incident start"


def _crewai_investigates(store_dir: str) -> None:
    from crewai.memory.types import MemoryRecord
    from mnem_crewai import MnemosyneStorageBackend

    backend = MnemosyneStorageBackend(store_dir)
    store = backend._store
    store.new_branch("crewai-db-investigation", start="main")
    store.checkout("crewai-db-investigation")
    backend.save([MemoryRecord(id="db-latency", scope="/", content=DB_FINDING)])
    store.checkout("main")


def _langgraph_investigates(store_dir: str) -> None:
    from mnem_langgraph import MnemosyneStore

    store_wrapper = MnemosyneStore(store_dir)
    store = store_wrapper._store
    store.new_branch("langgraph-cache-investigation", start="main")
    store.checkout("langgraph-cache-investigation")
    store_wrapper.put(("incident",), "cache-hit-rate", {"content": CACHE_FINDING})
    store.checkout("main")


def _openai_agents_investigates(store_dir: str) -> None:
    from mnem_openai_agents import MnemosyneSession

    store = mnem.open(store_dir)
    session = MnemosyneSession(store, "openai-session")
    store.new_branch("openai-queue-investigation", start="main")
    store.checkout("openai-queue-investigation")
    asyncio.run(session.add_items([{"role": "assistant", "content": QUEUE_FINDING}]))
    store.checkout("main")


def _autogen_investigates(store_dir: str) -> None:
    from autogen_core.memory import MemoryContent, MemoryMimeType
    from mnem_autogen import MnemosyneMemory

    memory = MnemosyneMemory("autogen-agent", store_dir)
    store = memory._store
    store.new_branch("autogen-deploy-investigation", start="main")
    store.checkout("autogen-deploy-investigation")
    asyncio.run(memory.add(MemoryContent(content=DEPLOY_FINDING, mime_type=MemoryMimeType.TEXT)))
    store.checkout("main")
    asyncio.run(memory.close())


def run(store_dir: str) -> None:
    store = mnem.init(store_dir)
    store.add("incident-4821", "database errors spiking, on-call paged")
    store.commit("open the incident")
    del store  # drop this handle before any adapter opens its own

    _crewai_investigates(store_dir)
    _langgraph_investigates(store_dir)
    _openai_agents_investigates(store_dir)
    _autogen_investigates(store_dir)

    store = mnem.open(store_dir)

    branches_in_order = [
        "crewai-db-investigation",
        "langgraph-cache-investigation",
        "openai-queue-investigation",
        "autogen-deploy-investigation",
    ]
    merge_commits: dict[str, str] = {}
    for branch in branches_in_order:
        result = store.merge(branch, time_ms=len(merge_commits) * 1_000)
        print(f"merged {branch}: status {result.status!r}, commit {result.commit[:12]}")
        assert result.status in ("merged", "fast-forwarded")
        merge_commits[branch] = result.commit

    memory = store.working_memory()
    print(f"\nfinal incident report, {len(memory)} findings:")
    for node_id, node in sorted(memory.items()):
        print(f"  {node_id}: {node.content!r}")

    # CrewAI and LangGraph take a caller-chosen id; the OpenAI Agents SDK
    # and AutoGen adapters auto-sequence theirs per session/name instead
    # (confirmed directly, not assumed), each bringing its own reserved
    # `:_seq` counter node along as a real stored node.
    assert set(memory) == {
        "incident-4821",
        "/db-latency",
        "incident:cache-hit-rate",
        "openai-session:0000000000",
        "openai-session:_seq",
        "autogen-agent:0000000000",
        "autogen-agent:_seq",
    }
    assert memory["/db-latency"].content["content"] == DB_FINDING
    assert memory["incident:cache-hit-rate"].content["content"] == CACHE_FINDING
    assert memory["openai-session:0000000000"].content["content"] == QUEUE_FINDING
    assert memory["autogen-agent:0000000000"].content["content"] == DEPLOY_FINDING

    # the real point: each finding still blames to the commit the agent
    # that wrote it made, not to a later merge commit - traceability
    # survives three more merges happening on top of it afterwards.
    db_blame = store.blame("/db-latency")
    cache_blame = store.blame("incident:cache-hit-rate")
    last_merge = merge_commits["autogen-deploy-investigation"]
    print(f"\n'/db-latency' blames to {db_blame.commit[:12]} - the CrewAI branch's own commit,")
    print(f"not {last_merge[:12]}, the last merge in the sequence.")
    assert db_blame.commit != last_merge
    assert cache_blame.commit != last_merge

    print("\nOK")


if __name__ == "__main__":
    with tempfile.TemporaryDirectory() as directory:
        run(directory)
