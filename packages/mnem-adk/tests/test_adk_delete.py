"""Tests for delete_memory (#594, #574, ADR-0026) - mnem-adk's own extra
method, outside BaseMemoryService, confirming the no-re-ingestion decision
holds when a session carrying the deleted event is re-added."""

import asyncio

import mnem
import pytest
from google.adk.events.event import Event
from google.adk.sessions.session import Session
from google.genai import types

from mnem_adk import MnemosyneMemoryService
from mnem_adk._ids import event_node_id, tombstone_marker_id


def _run(coro):
    return asyncio.run(coro)


def _event(event_id: str, text: str) -> Event:
    return Event(id=event_id, author="user", content=types.Content(role="user", parts=[types.Part(text=text)]))


def _session(events) -> Session:
    return Session(id="s1", app_name="myapp", user_id="u1", events=events)


@pytest.fixture
def store(tmp_path: object) -> mnem.Store:
    return mnem.init(tmp_path)


@pytest.fixture
def service(store: mnem.Store) -> MnemosyneMemoryService:
    return MnemosyneMemoryService(store)


def test_delete_removes_the_node(service: MnemosyneMemoryService, store: mnem.Store) -> None:
    node_id = event_node_id("myapp", "u1", "s1", "ev-1")
    _run(service.add_session_to_memory(_session([_event("ev-1", "hello")])))
    assert store.working_node(node_id) is not None
    _run(service.delete_memory(node_id))
    assert store.working_node(node_id) is None


def test_delete_writes_a_tombstone_marker(
    service: MnemosyneMemoryService, store: mnem.Store
) -> None:
    node_id = event_node_id("myapp", "u1", "s1", "ev-1")
    _run(service.add_session_to_memory(_session([_event("ev-1", "hello")])))
    _run(service.delete_memory(node_id))
    assert store.working_node(tombstone_marker_id(node_id)) is not None


def test_re_adding_the_same_session_does_not_resurrect_a_deleted_event(
    service: MnemosyneMemoryService, store: mnem.Store
) -> None:
    node_id = event_node_id("myapp", "u1", "s1", "ev-1")
    session = _session([_event("ev-1", "hello")])
    _run(service.add_session_to_memory(session))
    _run(service.delete_memory(node_id))
    _run(service.add_session_to_memory(session))
    assert store.working_node(node_id) is None


def test_re_adding_via_add_events_to_memory_also_does_not_resurrect(
    service: MnemosyneMemoryService, store: mnem.Store
) -> None:
    node_id = event_node_id("myapp", "u1", "s1", "ev-1")
    _run(service.add_session_to_memory(_session([_event("ev-1", "hello")])))
    _run(service.delete_memory(node_id))
    _run(
        service.add_events_to_memory(
            app_name="myapp", user_id="u1", events=[_event("ev-1", "hello")], session_id="s1"
        )
    )
    assert store.working_node(node_id) is None


def test_deleting_one_event_does_not_affect_others_in_the_same_session(
    service: MnemosyneMemoryService, store: mnem.Store
) -> None:
    session = _session([_event("ev-1", "a"), _event("ev-2", "b")])
    _run(service.add_session_to_memory(session))
    _run(service.delete_memory(event_node_id("myapp", "u1", "s1", "ev-1")))
    _run(service.add_session_to_memory(session))
    assert store.working_node(event_node_id("myapp", "u1", "s1", "ev-1")) is None
    assert store.working_node(event_node_id("myapp", "u1", "s1", "ev-2")) is not None
