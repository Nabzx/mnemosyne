"""A Claude agent works a billing dispute and uses Mnemosyne to find its own mistake.

    python examples/claude_agent.py            # replay a recorded run, no API key needed
    python examples/claude_agent.py --demo     # the same run, paced and styled for the GIF
    ANTHROPIC_API_KEY=... python examples/claude_agent.py --live

Live mode runs a real Claude tool-use loop; the tools are thin wrappers over
`mnem.agents` and `Store`, and it needs the ``anthropic`` package. Replay mode
runs the same memory operations from a fixed script so CI keeps it working.
Either way the store is real: the commit ids, the ``bisect`` and the ``blame``
below are produced by Mnemosyne, not typed in.
"""

from __future__ import annotations

import argparse
import os
import sys
import tempfile
import time

import mnem
from mnem import agents

# Catppuccin-ish palette, matched to assets/demo.tape.
LILAC = "\033[38;2;203;166;247m"
BLUE = "\033[38;2;137;180;250m"
GREEN = "\033[38;2;166;227;161m"
DIM = "\033[38;2;127;132;156m"
BOLD = "\033[1m"
OFF = "\033[0m"

PACE = 0.0  # seconds between lines; raised by --demo


def line(text: str = "", *, pause: float = 1.0) -> None:
    print(text)
    if PACE:
        time.sleep(PACE * pause)


def user(text: str) -> None:
    line(f"\n{LILAC}{BOLD}you ▸{OFF}  {text}", pause=1.6)


def claude(text: str) -> None:
    line(f"\n{BLUE}{BOLD}claude{OFF}  {text}", pause=1.3)


def cont(text: str) -> None:
    line(f"        {text}", pause=1.0)


def tool(call: str) -> None:
    line(f"        {GREEN}●{OFF} {call}", pause=0.8)


def result(text: str) -> None:
    for row in text.splitlines():
        line(f"          {DIM}{row}{OFF}", pause=0.3)


def seed(store: mnem.Store) -> str:
    """The first session: the case is opened, then a billing note is misread.

    Returns the id of the commit that recorded the wrong plan tier.
    """
    agents.remember(
        store, "customer-4821", "on the Enterprise plan",
        source="ticket-4821", summary="open the case", time_ms=1_000,
    )
    agents.remember(
        store, "primary-contact", "alice@acme",
        step="step-4", summary="note the primary contact", time_ms=2_000,
    )
    return agents.remember(
        store, "customer-4821", "downgraded to Pro last month",
        source="billing-note-8842", step="step-31",
        summary="reconcile the plan tier from billing", time_ms=3_000,
    )


def replay(store: mnem.Store, bad_commit: str) -> None:
    line(f"{DIM}  (earlier: 3 commits from the first session on this case){OFF}", pause=0.6)

    user(
        "Customer on ticket 4821 says they were never on Pro, but your notes "
        "say Pro. Work out what went wrong."
    )

    claude('My current note for customer-4821 is "downgraded to Pro last month".')
    cont("Let me find the commit where that belief entered.")
    tool('bisect  customer-4821 == "downgraded to Pro last month"')
    boundary = store.bisect(
        lambda memory: (
            "customer-4821" in memory
            and memory["customer-4821"].content == "downgraded to Pro last month"
        )
    )
    blame = store.blame("customer-4821", boundary)
    result(
        f"{boundary[:12]}  reconcile the plan tier from billing\n"
        f"  customer-4821 set here, from {blame.provenance.agent_step}, "
        f"source {blame.provenance.source}"
    )

    claude("Now trace that commit to the observation behind it.")
    tool("blame  customer-4821")
    result(
        f'content: "{blame.node.content}"\n'
        f"step:    {blame.provenance.agent_step}\n"
        f"source:  {blame.provenance.source}"
    )

    claude(f"It came from {blame.provenance.source} at {blame.provenance.agent_step}.")
    cont("But ticket-4821, the original source of this case, says Enterprise.")
    cont("I misread the billing note. Correcting the record.")
    fixed = agents.remember(
        store, "customer-4821", "on the Enterprise plan",
        source="ticket-4821", note="fix: misread billing-note-8842 as a downgrade",
        summary="correct the plan tier", time_ms=6_000,
    )
    tool("remember  customer-4821 = \"on the Enterprise plan\"  (source: ticket-4821)")
    tool(f"commit    {fixed[:12]}  correct the plan tier")

    claude("Fixed. The history keeps both the mistake and the correction:")
    tool("log --oneline")
    result("\n".join(f"{c.id[:12]}  {c.message}" for c in store.log()))

    assert boundary == bad_commit
    assert blame.provenance.source == "billing-note-8842"
    assert store.working_memory()["customer-4821"].content == "on the Enterprise plan"
    line(f"\n{GREEN}OK{OFF}  the wrong belief was found, traced and corrected", pause=0)


def live(store: mnem.Store) -> None:
    try:
        import anthropic
    except ImportError:
        sys.exit("live mode needs the anthropic package: pip install anthropic")

    tools = [
        {
            "name": "remember",
            "description": "Record or update a belief as one commit, with where it came from.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "node_id": {"type": "string"},
                    "content": {"type": "string"},
                    "source": {"type": "string"},
                    "step": {"type": "string"},
                    "note": {"type": "string"},
                },
                "required": ["node_id", "content"],
            },
        },
        {
            "name": "blame",
            "description": "Resolve a belief to the commit and observation that introduced it.",
            "input_schema": {
                "type": "object",
                "properties": {"node_id": {"type": "string"}},
                "required": ["node_id"],
            },
        },
        {
            "name": "bisect",
            "description": "Find the first commit where a belief took a given value.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "node_id": {"type": "string"},
                    "equals": {"type": "string"},
                },
                "required": ["node_id", "equals"],
            },
        },
    ]

    def dispatch(name: str, args: dict) -> str:
        if name == "remember":
            commit = agents.remember(
                store, args["node_id"], args["content"],
                source=args.get("source"), step=args.get("step"), note=args.get("note"),
            )
            return f"committed {commit[:12]}"
        if name == "blame":
            b = store.blame(args["node_id"])
            return (
                f'{b.commit[:12]} "{b.node.content}" step={b.provenance.agent_step} '
                f"source={b.provenance.source}"
            )
        if name == "bisect":
            want = args["equals"]
            found = store.bisect(
                lambda m: args["node_id"] in m and m[args["node_id"]].content == want
            )
            return f"first at {found[:12]}"
        return f"unknown tool {name}"

    client = anthropic.Anthropic()
    model = os.environ.get("MNEM_DEMO_MODEL", "claude-sonnet-4-5")
    messages = [{
        "role": "user",
        "content": (
            "Customer on ticket 4821 is disputing their bill. They say they were "
            "never on Pro, but your memory says Pro. Work out what went wrong, "
            "then correct the record."
        ),
    }]
    system = (
        "You are a support agent whose memory is version-controlled with Mnemosyne. "
        "When a past answer looks wrong, use bisect and blame to find the observation "
        "that caused it, then remember the correction with its real source."
    )
    for _ in range(8):
        reply = client.messages.create(
            model=model, max_tokens=1024,
            system=system, tools=tools, messages=messages,
        )
        for block in reply.content:
            if block.type == "text" and block.text.strip():
                claude(block.text.strip())
            elif block.type == "tool_use":
                tool(f"{block.name}  {block.input}")
                out = dispatch(block.name, block.input)
                result(out)
        messages.append({"role": "assistant", "content": reply.content})
        if reply.stop_reason != "tool_use":
            break
        messages.append({
            "role": "user",
            "content": [
                {
                    "type": "tool_result",
                    "tool_use_id": block.id,
                    "content": dispatch(block.name, block.input),
                }
                for block in reply.content
                if block.type == "tool_use"
            ],
        })


def main() -> None:
    global PACE
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--demo", action="store_true", help="pace and style for recording")
    parser.add_argument("--live", action="store_true", help="run a real Claude tool-use loop")
    args = parser.parse_args()
    if args.demo:
        PACE = 1.0

    print(f"{DIM}  mnemosyne · a support agent finds its own mistake{OFF}")
    with tempfile.TemporaryDirectory() as directory:
        store = mnem.init(directory)
        bad_commit = seed(store)
        if args.live:
            live(store)
        else:
            replay(store, bad_commit)


if __name__ == "__main__":
    main()
