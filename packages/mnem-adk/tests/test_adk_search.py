"""Tests for search_memory (#593, #573, ADR-0026) - ranking matched
against InMemoryMemoryService's own real algorithm, scope isolation, and
reserved-node exclusion."""

import asyncio

import mnem
import pytest
from google.adk.events.event import Event
from google.adk.sessions.session import Session
from google.genai import types

from mnem_adk import MnemosyneMemoryService


def _run(coro):
    return asyncio.run(coro)


def _event(event_id: str, text: str, *, author: str = "user") -> Event:
    return Event(
        id=event_id,
        author=author,
        content=types.Content(role=author, parts=[types.Part(text=text)]),
    )


@pytest.fixture
def store(tmp_path: object) -> mnem.Store:
    return mnem.init(tmp_path)


@pytest.fixture
def service(store: mnem.Store) -> MnemosyneMemoryService:
    return MnemosyneMemoryService(store)


async def _ingest(service, app_name, user_id, session_id, events) -> None:
    session = Session(id=session_id, app_name=app_name, user_id=user_id, events=events)
    await service.add_session_to_memory(session)


def test_a_matching_query_finds_the_memory(service: MnemosyneMemoryService) -> None:
    _run(_ingest(service, "myapp", "u1", "s1", [_event("ev-1", "My favourite colour is teal.")]))
    result = _run(service.search_memory(app_name="myapp", user_id="u1", query="favourite colour"))
    assert len(result.memories) == 1
    assert result.memories[0].content.parts[0].text == "My favourite colour is teal."


def test_a_non_matching_query_finds_nothing(service: MnemosyneMemoryService) -> None:
    _run(_ingest(service, "myapp", "u1", "s1", [_event("ev-1", "My favourite colour is teal.")]))
    result = _run(service.search_memory(app_name="myapp", user_id="u1", query="pizza toppings"))
    assert result.memories == []


def test_results_are_scoped_to_app_name_and_user_id(service: MnemosyneMemoryService) -> None:
    _run(_ingest(service, "myapp", "u1", "s1", [_event("ev-1", "teal is the colour")]))
    _run(_ingest(service, "myapp", "u2", "s1", [_event("ev-1", "teal is also here")]))
    result = _run(service.search_memory(app_name="myapp", user_id="u1", query="teal"))
    assert len(result.memories) == 1


def test_results_are_scoped_across_sessions_within_one_user(
    service: MnemosyneMemoryService,
) -> None:
    _run(_ingest(service, "myapp", "u1", "s1", [_event("ev-1", "teal colour")]))
    _run(_ingest(service, "myapp", "u1", "s2", [_event("ev-1", "teal again")]))
    result = _run(service.search_memory(app_name="myapp", user_id="u1", query="teal"))
    assert len(result.memories) == 2


def test_reserved_counter_node_is_excluded_from_results(
    service: MnemosyneMemoryService, store: mnem.Store
) -> None:
    from mnem_adk._ids import memory_seq_node_id

    store.add(memory_seq_node_id("myapp", "u1"), 3)
    store.commit("bump counter", author="test")
    result = _run(service.search_memory(app_name="myapp", user_id="u1", query="3"))
    assert result.memories == []


def test_more_matching_words_ranks_higher(service: MnemosyneMemoryService) -> None:
    _run(
        _ingest(
            service,
            "myapp",
            "u1",
            "s1",
            [
                _event("ev-1", "teal"),
                _event("ev-2", "teal colour favourite"),
            ],
        )
    )
    result = _run(
        service.search_memory(app_name="myapp", user_id="u1", query="teal colour favourite")
    )
    assert [m.content.parts[0].text for m in result.memories] == [
        "teal colour favourite",
        "teal",
    ]


def test_at_most_ten_results(service: MnemosyneMemoryService) -> None:
    events = [_event(f"ev-{i}", "teal") for i in range(15)]
    _run(_ingest(service, "myapp", "u1", "s1", events))
    result = _run(service.search_memory(app_name="myapp", user_id="u1", query="teal"))
    assert len(result.memories) == 10


def test_provenance_source_becomes_the_returned_author(service: MnemosyneMemoryService) -> None:
    _run(
        _ingest(
            service, "myapp", "u1", "s1", [_event("ev-1", "teal colour", author="support-bot")]
        )
    )
    result = _run(service.search_memory(app_name="myapp", user_id="u1", query="teal"))
    assert result.memories[0].author == "support-bot"
