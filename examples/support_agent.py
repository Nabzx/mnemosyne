"""A support agent works a billing case, gets a plan tier wrong, and Mnemosyne
finds where the wrong belief entered.

    python examples/support_agent.py

The "agent" here is a fixed script, not a model. The point is the memory
operations: `mnem.agents.remember` records each conclusion as a commit with its
provenance, then `bisect` and `blame` trace a wrong answer back to its cause.
"""

from __future__ import annotations

import tempfile

import mnem
from mnem import agents


def run(store_dir: str) -> None:
    store = mnem.init(store_dir)

    # the case opens; the agent records what the ticket says, and where from
    agents.remember(
        store, "customer-4821", "on the Enterprise plan",
        source="ticket-4821", summary="open the case",
    )
    agents.remember(store, "primary-contact", "alice@acme", step="step-4")
    agents.remember(store, "open-tickets", "2", summary="count the open tickets")

    # an hour of work later, the agent misreads a billing note
    bad_commit = agents.remember(
        store, "customer-4821", "downgraded to Pro last month",
        source="billing-note-8842", step="step-31",
        summary="reconcile the plan tier from billing",
    )
    agents.remember(store, "discount-band", "pro-tier", summary="apply the discount")

    print("the run so far:")
    for commit in store.log():
        print(f"  {commit.id[:8]}  {commit.message}")

    # the agent gives a wrong answer. find the commit where "plan == Pro" began.
    boundary = store.bisect(
        lambda memory: (
            "customer-4821" in memory
            and memory["customer-4821"].content == "downgraded to Pro last month"
        )
    )
    blame = store.blame("customer-4821", boundary)

    print()
    print(f"the wrong belief entered at {boundary[:8]}")
    print(f"  written at {blame.provenance.agent_step}, "
          f"from observation {blame.provenance.source}")

    assert boundary == bad_commit
    assert blame.provenance.source == "billing-note-8842"
    assert blame.node.content == "downgraded to Pro last month"
    print("\nOK")


if __name__ == "__main__":
    with tempfile.TemporaryDirectory() as directory:
        run(directory)
