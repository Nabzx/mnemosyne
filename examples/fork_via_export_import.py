"""Fork a store via `export`/`import`, edit the fork, merge the edit back -
all with `export`/`import`/`merge`, no agent-adapter-specific code at all
(#356, part of the registry/portability map #352).

    python examples/fork_via_export_import.py

`export` dumps every commit, ref, and HEAD as one JSON value; `import`
rebuilds a store from that JSON at a fresh path. This proves the round
trip is lossless - not just that content survives, but that history does:
a node merged in by the fork resolves back to the *exact same commit id*
it was written in originally, because content-addressed objects hash
identically regardless of which store file holds them.
"""

from __future__ import annotations

import os
import tempfile

import mnem


def run(original_dir: str, fork_dir: str) -> None:
    original = mnem.init(original_dir)
    original.add("deploy-plan", "canary rollout, 5% then 50% then 100%")
    original_commit = original.commit("record the rollout plan")

    exported = original.export()
    fork = mnem.Store.import_(fork_dir, exported)

    fork.new_branch("fork-edit")
    fork.checkout("fork-edit")
    fork.add("rollback-trigger", "error rate over 2% in the 5% stage")
    fork.commit("record the rollback trigger, discovered while forking")
    fork.checkout("main")

    merged = fork.merge("fork-edit")

    memory = fork.working_memory()
    assert merged.status == "fast-forwarded"
    assert set(memory) == {"deploy-plan", "rollback-trigger"}
    assert memory["deploy-plan"].content == "canary rollout, 5% then 50% then 100%"
    assert memory["rollback-trigger"].content == "error rate over 2% in the 5% stage"

    # history round-tripped too, not just content: the inherited node still
    # resolves to the commit it was ORIGINALLY written in, byte-for-byte -
    # proof export/import carried the real object graph, not a flattened copy.
    inherited_blame = fork.blame("deploy-plan")
    assert inherited_blame.commit == original_commit

    # the original store was never touched by any of this.
    assert set(original.working_memory()) == {"deploy-plan"}

    print(f"original store:  1 commit, HEAD {original_commit[:12]}")
    print(f"forked store:    merge status {merged.status!r}, commit {merged.commit[:12]}")
    print(f"forked memory:   {sorted(memory)}")
    print(f"'deploy-plan' still blames to the original commit: {inherited_blame.commit[:12]}")
    print("OK")


if __name__ == "__main__":
    with tempfile.TemporaryDirectory() as original_dir, tempfile.TemporaryDirectory() as fork_parent:
        run(original_dir, os.path.join(fork_parent, "fork"))
