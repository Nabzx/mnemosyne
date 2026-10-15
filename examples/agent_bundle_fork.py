"""Fork a published agent bundle and edit it independently - the fork
gets a real new commit on top of the bundle's own history, not a
replacement of it (#452, part of the registry/portability map #352).

    python examples/agent_bundle_fork.py

Builds directly on #450's bundle shape and #356's own fork/export/import
proof (`fork_via_export_import.py`): fork a bundle, edit one field,
commit, then show the edited field `blame`s to the new commit while
every untouched field still `blame`s to the bundle's own original
commit - the same "history survives a fork" proof, retold for a whole
agent instead of a single memory fact. The CLI equivalent of this fork
step is `mnem registry fork` (#451); this uses the SDK's `export`/
`import` directly, matching every other example here.
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
]
REVISED_PROMPT = (
    "You are a support triage agent. Be terse, factual, and always cite the ticket id."
)


def run(original_dir: str, fork_dir: str) -> None:
    original = mnem.init(original_dir)
    original.add("agent:prompt", PROMPT)
    original.add("agent:policy", POLICY)
    original.add("agent:tools", TOOLS)
    bundle_commit = original.commit("package the agent as a bundle")

    exported = original.export()
    fork = mnem.Store.import_(fork_dir, exported)

    fork.add("agent:prompt", REVISED_PROMPT)
    edit_commit = fork.commit("sharpen the prompt: always cite the ticket id")

    log = fork.log()
    assert [commit.id for commit in log] == [edit_commit, bundle_commit]

    prompt_blame = fork.blame("agent:prompt")
    policy_blame = fork.blame("agent:policy")
    tools_blame = fork.blame("agent:tools")
    assert prompt_blame.commit == edit_commit
    assert policy_blame.commit == bundle_commit
    assert tools_blame.commit == bundle_commit

    memory = fork.working_memory()
    assert memory["agent:prompt"].content == REVISED_PROMPT
    assert memory["agent:policy"].content == POLICY
    assert memory["agent:tools"].content == TOOLS

    # the original bundle was never touched by any of this.
    assert original.working_memory()["agent:prompt"].content == PROMPT

    print(f"fork log: {[commit.id[:12] for commit in log]}")
    print(f"agent:prompt blames to the new edit commit: {prompt_blame.commit == edit_commit}")
    print(f"agent:policy still blames to the bundle commit: {policy_blame.commit == bundle_commit}")
    print(f"agent:tools  still blames to the bundle commit: {tools_blame.commit == bundle_commit}")
    print("OK")


if __name__ == "__main__":
    with tempfile.TemporaryDirectory() as original_parent, tempfile.TemporaryDirectory() as fork_parent:
        run(
            os.path.join(original_parent, "original"),
            os.path.join(fork_parent, "fork"),
        )
