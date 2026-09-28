"""An OpenAI Agents SDK `Session` backed by Mnemosyne: a support agent looks
up an order by calling a real tool, and `why`/`bisect` trace the answer back
to that exact call - no caller code shaped the provenance by hand.

    python examples/openai_agents_memory.py     # needs mnem-openai-agents

The model here is a fixed script, not a live one, deterministic like every
other example so CI can assert on it without an API key. `Runner.run` and
the real tool still execute for real - only the "what should I do next"
decision is scripted.
"""

from __future__ import annotations

import asyncio
import tempfile

from agents import Agent, Model, ModelResponse, RunConfig, Runner, Usage, function_tool
from mnem_openai_agents import MnemosyneSession
from openai.types.responses import (
    ResponseFunctionToolCall,
    ResponseOutputMessage,
    ResponseOutputText,
)

import mnem

SESSION_ID = "ticket-4821"


@function_tool
def lookup_order(order_id: str) -> str:
    """Look up an order's shipping status.

    Args:
        order_id: The order id to look up.
    """
    return f"order {order_id}: shipped 2026-09-20, tracking ABC123"


def _text_message(text: str) -> ResponseOutputMessage:
    return ResponseOutputMessage(
        id="msg_1",
        role="assistant",
        status="completed",
        type="message",
        content=[ResponseOutputText(annotations=[], text=text, type="output_text")],
    )


def _tool_call(call_id: str, name: str, arguments: str) -> ResponseFunctionToolCall:
    return ResponseFunctionToolCall(
        call_id=call_id, name=name, arguments=arguments, type="function_call"
    )


class ScriptedModel(Model):
    """Returns pre-scripted turns in order - no live API, so the run stays
    deterministic the way every other example does. The real Runner still
    executes the real tool and fires the real hooks; only the model's own
    "what to do next" decision is fixed in advance."""

    def __init__(self, turns: list[list[object]]) -> None:
        self._turns = iter(turns)

    async def get_response(self, *args: object, **kwargs: object) -> ModelResponse:
        return ModelResponse(output=next(self._turns), usage=Usage(), response_id=None)

    def stream_response(self, *args: object, **kwargs: object):
        raise NotImplementedError("this example only exercises Runner.run, not streaming")


async def run(store_dir: str) -> None:
    store = mnem.init(store_dir)
    session = MnemosyneSession(store, SESSION_ID)

    model = ScriptedModel(
        [
            [_tool_call("call_1", "lookup_order", '{"order_id": "4821"}')],
            [_text_message("Order 4821 shipped on 2026-09-20, tracking ABC123.")],
        ]
    )
    agent = Agent(name="support-agent", model=model, tools=[lookup_order])

    result = await Runner.run(
        agent,
        "what's the status of order 4821?",
        session=session,
        hooks=session.hooks,  # turns on provenance auto-capture (#307)
        run_config=RunConfig(tracing_disabled=True),
    )

    print("the agent said:")
    print(f"  {result.final_output}")

    items = await session.get_items()
    commits = session.history()
    # commit ids carry a wall-clock timestamp - add_items has no time_ms
    # override to pin, unlike mnem.agents' own functions, since it must
    # match the Session protocol's fixed signature exactly - so these hex
    # prefixes differ between runs; nothing asserted below depends on them.
    print(f"\n{len(items)} items, one node each, across {len(commits)} commits:")
    for commit in commits:
        print(f"  {commit.id[:8]}  {commit.message}")

    # the tool's own output item, third in the batch (user prompt, tool
    # call, tool output, final message) - the node id scheme (README) is
    # {session_id}:{seq:010d}, so this is the one to trace.
    tool_output_id = f"{SESSION_ID}:{2:010d}"
    blame = session.why(tool_output_id)
    print(f"\nwhy {tool_output_id}:")
    print(f'  content: "{blame.node.content["output"]}"')
    print(f"  source:  {blame.provenance.source}")

    boundary = session.bisect(lambda memory: tool_output_id in memory)

    assert result.final_output == "Order 4821 shipped on 2026-09-20, tracking ABC123."
    assert len(items) == 4
    assert len(commits) == 3  # the prompt, the tool call+output pair, the final message
    assert blame.provenance.source == "lookup_order"
    assert blame.node.content["output"] == "order 4821: shipped 2026-09-20, tracking ABC123"
    assert boundary == blame.commit
    print("\nOK")


if __name__ == "__main__":
    with tempfile.TemporaryDirectory() as directory:
        asyncio.run(run(directory))
