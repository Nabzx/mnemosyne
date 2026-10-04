"""An AutoGen ``AssistantAgent`` whose memory is a Mnemosyne store:
`update_context` injects stored memories before every inference, then
`why`/`bisect` (extra methods outside the `Memory` protocol, ADR-0023)
trace a specific memory back to the commit that wrote it.

    python examples/autogen_memory.py     # needs mnem-autogen[examples]

Deterministic like every other example: the model is a
``ReplayChatCompletionClient`` - AutoGen's own real deterministic test
client, no API key, no network - returning one scripted reply. The real
``AssistantAgent`` and ``MnemosyneMemory`` still run for real; only the
model's own response is scripted.
"""

from __future__ import annotations

import asyncio
import tempfile

from autogen_agentchat.agents import AssistantAgent
from autogen_core.memory import MemoryContent, MemoryMimeType
from autogen_ext.models.replay import ReplayChatCompletionClient
from mnem_autogen import MnemosyneMemory
from mnem_autogen.memory import _item_id

import mnem

NAME = "support-agent"


async def run(store_dir: str) -> None:
    mnem.init(store_dir)
    memory = MnemosyneMemory(NAME, store_dir)

    await memory.add(
        MemoryContent(
            content="the acme account is on the enterprise plan",
            mime_type=MemoryMimeType.TEXT,
        )
    )
    plan_commit = memory._store.head_commit()

    await memory.add(
        MemoryContent(
            content="support ticket 4821 was resolved by refunding a late fee",
            mime_type=MemoryMimeType.TEXT,
        )
    )
    ticket_commit = memory._store.head_commit()

    model_client = ReplayChatCompletionClient(
        ["The acme account is on the enterprise plan, and ticket 4821 is already resolved."]
    )
    agent = AssistantAgent(name="support_agent", model_client=model_client, memory=[memory])

    result = await agent.run(task="what's the status of the acme account and ticket 4821?")
    final_message = result.messages[-1].content
    print("the agent said:")
    print(f"  {final_message}")

    # update_context runs automatically before every inference - this is
    # the real SystemMessage it injected, read back from the model
    # client's own call history, not just trusted to have happened.
    injected = model_client.create_calls[0]["messages"][-1].content
    print("\nmemory injected before inference:")
    print(f"  {injected!r}")

    plan_id = _item_id(NAME, 0)
    blame = memory.why(plan_id)
    print(f"\nwhy {plan_id}:")
    print(f"  commit:  {blame.commit[:8]}")
    print(f'  content: "{blame.node.content["content"]}"')

    ticket_id = _item_id(NAME, 1)
    boundary = memory.bisect(lambda working_memory: ticket_id in working_memory)

    assert (
        final_message
        == "The acme account is on the enterprise plan, and ticket 4821 is already resolved."
    )
    assert "the acme account is on the enterprise plan" in injected
    assert "support ticket 4821 was resolved" in injected
    assert blame.commit == plan_commit
    assert blame.node.content["content"] == "the acme account is on the enterprise plan"
    assert boundary == ticket_commit
    print("\nOK")


if __name__ == "__main__":
    with tempfile.TemporaryDirectory() as directory:
        asyncio.run(run(directory))
