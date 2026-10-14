"""Package an agent - a prompt, a policy, a tool list - as one portable
bundle using `export`/`import` directly, with zero new engine code
(#450, part of the registry/portability map #352).

    python examples/agent_bundle_export_import.py

A throwaway spike ahead of #354/#355's own decided node-id convention:
`agent:prompt`, `agent:policy`, and `agent:tools` below are picked for
this demo only, not a real convention any build ticket is bound to.
`export` already dumps every commit, ref, and HEAD as one JSON value
regardless of what the node ids happen to be, so "an agent, exported"
needs nothing beyond committing the right nodes under ids a human
judges sensible - the same proof `fork_via_export_import.py` already
gave for a plain memory store, applied to an agent-shaped one instead.
"""

from __future__ import annotations

import os
import tempfile

import mnem

PROMPT = "You are a support triage agent. Be terse and factual."
POLICY = {"temperature": 0.2, "max_steps": 5}
TOOLS = [
    {
        "name": "search_tickets",
        "description": "Search support tickets by keyword",
        "parameters": {"query": "string"},
    },
    {
        "name": "escalate",
        "description": "Escalate a ticket to a human",
        "parameters": {"ticket_id": "string"},
    },
]


def run(original_dir: str, bundle_dir: str) -> None:
    original = mnem.init(original_dir)
    original.add("agent:prompt", PROMPT)
    original.add("agent:policy", POLICY)
    original.add("agent:tools", TOOLS)
    original.commit("package the agent as a bundle")

    exported = original.export()
    bundle = mnem.Store.import_(bundle_dir, exported)

    memory = bundle.working_memory()
    assert set(memory) == {"agent:prompt", "agent:policy", "agent:tools"}
    assert memory["agent:prompt"].content == PROMPT
    assert memory["agent:policy"].content == POLICY
    assert memory["agent:tools"].content == TOOLS

    print(f"bundle: {sorted(memory)}")
    print(f"prompt round-tripped: {memory['agent:prompt'].content == PROMPT}")
    print(f"policy round-tripped: {memory['agent:policy'].content == POLICY}")
    print(f"tools round-tripped:  {memory['agent:tools'].content == TOOLS}")
    print("OK")


if __name__ == "__main__":
    with tempfile.TemporaryDirectory() as original_dir, tempfile.TemporaryDirectory() as bundle_parent:
        run(original_dir, os.path.join(bundle_parent, "bundle"))
