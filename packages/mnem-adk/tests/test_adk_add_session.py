"""Tests for add_session_to_memory (#590, ADR-0026) - against a real
google.adk.sessions.session.Session, not a mock."""

import mnem
import pytest
from google.adk.events.event import Event
from google.adk.sessions.session import Session
from google.genai import types

from mnem_adk import MnemosyneMemoryService
from mnem_adk._ids import event_node_id


def _run(coro):
    import asyncio

    return asyncio.run(coro)


def _event(event_id: str, text: str, *, author: str = "user") -> Event:
    return Event(
        id=event_id,
        author=author,
        content=types.Content(role=author, parts=[types.Part(text=text)]),
    )


def _session(app_name: str = "myapp", user_id: str = "u1", session_id: str = "s1", events=None) -> Session:
    return Session(id=session_id, app_name=app_name, user_id=user_id, events=events or [])


@pytest.fixture
def store(tmp_path: object) -> mnem.Store:
    return mnem.init(tmp_path)


@pytest.fixture
def service(store: mnem.Store) -> MnemosyneMemoryService:
    return MnemosyneMemoryService(store)


def test_a_session_with_one_event_writes_one_node(
    service: MnemosyneMemoryService, store: mnem.Store
) -> None:
    session = _session(events=[_event("ev-1", "hello")])
    _run(service.add_session_to_memory(session))
    node_id = event_node_id("myapp", "u1", "s1", "ev-1")
    assert store.working_node(node_id) is not None


def test_a_session_with_several_events_is_one_commit(
    service: MnemosyneMemoryService, store: mnem.Store
) -> None:
    session = _session(events=[_event("ev-1", "a"), _event("ev-2", "b"), _event("ev-3", "c")])
    before = len(store.log())
    _run(service.add_session_to_memory(session))
    after = len(store.log())
    assert after == before + 1
    assert len(store.working_memory()) == 3


def test_an_empty_session_is_a_noop(service: MnemosyneMemoryService, store: mnem.Store) -> None:
    before = len(store.log())
    _run(service.add_session_to_memory(_session(events=[])))
    assert len(store.log()) == before


def test_a_session_with_only_contentless_events_is_a_noop(
    service: MnemosyneMemoryService, store: mnem.Store
) -> None:
    before = len(store.log())
    session = _session(events=[Event(id="ev-1", author="user")])
    _run(service.add_session_to_memory(session))
    assert len(store.log()) == before


def test_re_adding_the_same_session_is_idempotent(
    service: MnemosyneMemoryService, store: mnem.Store
) -> None:
    session = _session(events=[_event("ev-1", "hello")])
    _run(service.add_session_to_memory(session))
    _run(service.add_session_to_memory(session))
    assert len(store.working_memory()) == 1
    node = store.working_node(event_node_id("myapp", "u1", "s1", "ev-1"))
    assert types.Content.model_validate(node.content).parts[0].text == "hello"


def test_different_sessions_do_not_collide(
    service: MnemosyneMemoryService, store: mnem.Store
) -> None:
    _run(service.add_session_to_memory(_session(session_id="s1", events=[_event("ev-1", "a")])))
    _run(service.add_session_to_memory(_session(session_id="s2", events=[_event("ev-1", "b")])))
    assert len(store.working_memory()) == 2
