"""Tests for add_events_to_memory (#591, ADR-0026) - a delta write, against
real google.adk.events.event.Event objects, not mocks."""

import asyncio

import mnem
import pytest
from google.adk.events.event import Event
from google.genai import types

from mnem_adk import MnemosyneMemoryService
from mnem_adk._ids import event_node_id
from mnem_adk.memory_service import _UNKNOWN_SESSION_ID


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


def test_a_delta_with_a_session_id_writes_scoped_nodes(
    service: MnemosyneMemoryService, store: mnem.Store
) -> None:
    _run(
        service.add_events_to_memory(
            app_name="myapp", user_id="u1", events=[_event("ev-1", "hi")], session_id="s1"
        )
    )
    assert store.working_node(event_node_id("myapp", "u1", "s1", "ev-1")) is not None


def test_no_session_id_falls_back_to_the_reserved_unknown_session(
    service: MnemosyneMemoryService, store: mnem.Store
) -> None:
    _run(
        service.add_events_to_memory(
            app_name="myapp", user_id="u1", events=[_event("ev-1", "hi")]
        )
    )
    expected = event_node_id("myapp", "u1", _UNKNOWN_SESSION_ID, "ev-1")
    assert store.working_node(expected) is not None


def test_an_empty_delta_is_a_noop(service: MnemosyneMemoryService, store: mnem.Store) -> None:
    before = len(store.log())
    _run(service.add_events_to_memory(app_name="myapp", user_id="u1", events=[], session_id="s1"))
    assert len(store.log()) == before


def test_a_second_delta_call_is_additive_not_a_full_resync(
    service: MnemosyneMemoryService, store: mnem.Store
) -> None:
    _run(
        service.add_events_to_memory(
            app_name="myapp", user_id="u1", events=[_event("ev-1", "a")], session_id="s1"
        )
    )
    _run(
        service.add_events_to_memory(
            app_name="myapp", user_id="u1", events=[_event("ev-2", "b")], session_id="s1"
        )
    )
    assert store.working_node(event_node_id("myapp", "u1", "s1", "ev-1")) is not None
    assert store.working_node(event_node_id("myapp", "u1", "s1", "ev-2")) is not None
    assert len(store.working_memory()) == 2


def test_custom_metadata_is_accepted_and_ignored(
    service: MnemosyneMemoryService, store: mnem.Store
) -> None:
    _run(
        service.add_events_to_memory(
            app_name="myapp",
            user_id="u1",
            events=[_event("ev-1", "hi")],
            session_id="s1",
            custom_metadata={"ttl_seconds": 60},
        )
    )
    assert store.working_node(event_node_id("myapp", "u1", "s1", "ev-1")) is not None
