"""An offline integration test driving a real ADK Runner through
load_memory and preload_memory against MnemosyneMemoryService (#596,
ADR-0026). A scripted model, no network call - the same offline
discipline examples/claude_agent.py already uses.

Productionises #584's own throwaway spike (which only drove
load_memory) into a real CI test, and adds preload_memory, confirming
#570's own finding that the two really do compose on one agent."""

import asyncio
from collections.abc import AsyncGenerator

import mnem
import pytest
from google.adk.agents import LlmAgent
from google.adk.models import BaseLlm
from google.adk.models.llm_response import LlmResponse
from google.adk.runners import Runner
from google.adk.sessions import InMemorySessionService
from google.adk.tools import load_memory, preload_memory
from google.genai import types

from mnem_adk import MnemosyneMemoryService


def _run(coro):
    return asyncio.run(coro)


@pytest.fixture
def store(tmp_path: object) -> mnem.Store:
    return mnem.init(tmp_path)


@pytest.fixture
def memory_service(store: mnem.Store) -> MnemosyneMemoryService:
    return MnemosyneMemoryService(store)


async def _seed_memory(memory_service: MnemosyneMemoryService, session_service) -> None:
    """A plain scripted turn, ingested into memory - the fact later turns
    draw on."""

    class _AckLlm(BaseLlm):
        model: str = "scripted"

        async def generate_content_async(
            self, llm_request, stream: bool = False
        ) -> AsyncGenerator[LlmResponse, None]:
            yield LlmResponse(
                content=types.Content(
                    role="model", parts=[types.Part(text="Noted: your favourite colour is teal.")]
                )
            )

    agent = LlmAgent(name="seed_agent", model=_AckLlm())
    runner = Runner(
        app_name="spike", agent=agent, session_service=session_service, memory_service=memory_service
    )
    session = await session_service.create_session(app_name="spike", user_id="u1")
    async for _event in runner.run_async(
        user_id="u1",
        session_id=session.id,
        new_message=types.Content(
            role="user", parts=[types.Part(text="My favourite colour is teal.")]
        ),
    ):
        pass

    fresh = await session_service.get_session(app_name="spike", user_id="u1", session_id=session.id)
    await memory_service.add_session_to_memory(fresh)


def test_preload_memory_injects_the_stored_fact_automatically(
    memory_service: MnemosyneMemoryService,
) -> None:
    session_service = InMemorySessionService()
    _run(_seed_memory(memory_service, session_service))

    seen_past_conversations = []

    class _ObservingLlm(BaseLlm):
        model: str = "scripted"

        async def generate_content_async(
            self, llm_request, stream: bool = False
        ) -> AsyncGenerator[LlmResponse, None]:
            for content in llm_request.contents:
                for part in content.parts or []:
                    if part.text and "<PAST_CONVERSATIONS>" in part.text:
                        seen_past_conversations.append(part.text)
            yield LlmResponse(
                content=types.Content(role="model", parts=[types.Part(text="teal, got it.")])
            )

    agent = LlmAgent(name="recall_agent", model=_ObservingLlm(), tools=[preload_memory])
    runner = Runner(
        app_name="spike", agent=agent, session_service=session_service, memory_service=memory_service
    )
    session = _run(session_service.create_session(app_name="spike", user_id="u1"))

    async def go():
        async for _event in runner.run_async(
            user_id="u1",
            session_id=session.id,
            new_message=types.Content(
                role="user", parts=[types.Part(text="What is my favourite colour?")]
            ),
        ):
            pass

    _run(go())

    assert seen_past_conversations, "preload_memory never injected a <PAST_CONVERSATIONS> block"
    assert "teal" in seen_past_conversations[0]


def test_load_memory_tool_call_round_trips(memory_service: MnemosyneMemoryService) -> None:
    session_service = InMemorySessionService()
    _run(_seed_memory(memory_service, session_service))

    class _RecallLlm(BaseLlm):
        model: str = "scripted"
        turn: int = 0

        async def generate_content_async(
            self, llm_request, stream: bool = False
        ) -> AsyncGenerator[LlmResponse, None]:
            self.turn += 1
            if self.turn == 1:
                yield LlmResponse(
                    content=types.Content(
                        role="model",
                        parts=[
                            types.Part(
                                function_call=types.FunctionCall(
                                    id="call-1",
                                    name="load_memory",
                                    args={"query": "favourite colour"},
                                )
                            )
                        ],
                    )
                )
            else:
                found = "unknown"
                for content in llm_request.contents:
                    for part in content.parts or []:
                        if part.function_response and part.function_response.name == "load_memory":
                            found = str(part.function_response.response)
                yield LlmResponse(
                    content=types.Content(
                        role="model",
                        parts=[types.Part(text=f"Your favourite colour is teal (memory: {found}).")],
                    )
                )

    agent = LlmAgent(name="recall_agent", model=_RecallLlm(), tools=[load_memory])
    runner = Runner(
        app_name="spike", agent=agent, session_service=session_service, memory_service=memory_service
    )
    session = _run(session_service.create_session(app_name="spike", user_id="u1"))

    final_text = None

    async def go():
        nonlocal final_text
        async for event in runner.run_async(
            user_id="u1",
            session_id=session.id,
            new_message=types.Content(
                role="user", parts=[types.Part(text="What is my favourite colour?")]
            ),
        ):
            if event.content and event.content.parts:
                for part in event.content.parts:
                    if part.text:
                        final_text = part.text

    _run(go())

    assert final_text is not None
    assert "memory:" in final_text.lower()
    assert "teal" in final_text.lower()
