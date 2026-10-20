"""Tests for branch/switch/merge/why/bisect/history (#595, ADR-0026) -
mnem-adk's own extra methods, outside BaseMemoryService, following the
LangGraph/OpenAI Agents SDK/AutoGen precedent (ADR-0016, ADR-0020,
ADR-0023)."""

import asyncio

import mnem
import pytest
from google.adk.events.event import Event
from google.adk.sessions.session import Session
from google.genai import types

from mnem_adk import MnemosyneMemoryService
from mnem_adk._ids import event_node_id


def _run(coro):
    return asyncio.run(coro)


def _event(event_id: str, text: str, author: str = "user") -> Event:
    return Event(id=event_id, author=author, content=types.Content(role="user", parts=[types.Part(text=text)]))


def _ingest(service: MnemosyneMemoryService, event: Event) -> None:
    session = Session(id="s1", app_name="myapp", user_id="u1", events=[event])
    _run(service.add_session_to_memory(session))


@pytest.fixture
def store(tmp_path: object) -> mnem.Store:
    return mnem.init(tmp_path)


@pytest.fixture
def service(store: mnem.Store) -> MnemosyneMemoryService:
    return MnemosyneMemoryService(store)


def test_branch_creates_without_switching(
    service: MnemosyneMemoryService, store: mnem.Store
) -> None:
    _ingest(service, _event("ev-1", "a"))
    before = store.head()
    service.branch("hypothesis")
    assert store.head() == before
    assert any(name == "hypothesis" for name, _ in store.branches())


def test_switch_moves_head(service: MnemosyneMemoryService, store: mnem.Store) -> None:
    _ingest(service, _event("ev-1", "a"))
    service.branch("hypothesis")
    service.switch("hypothesis")
    assert store.head() == "ref: hypothesis"


def test_why_traces_a_node_to_its_commit_and_provenance(
    service: MnemosyneMemoryService, store: mnem.Store
) -> None:
    _ingest(service, _event("ev-1", "shipped", "lookup_order"))
    node_id = event_node_id("myapp", "u1", "s1", "ev-1")
    blame = service.why(node_id)
    assert blame.provenance.source == "lookup_order"


def test_history_matches_store_log(service: MnemosyneMemoryService, store: mnem.Store) -> None:
    _ingest(service, _event("ev-1", "a"))
    _ingest(service, _event("ev-2", "b"))
    assert [c.id for c in service.history()] == [c.id for c in store.log()]
    assert [c.id for c in service.history(limit=1)] == [c.id for c in store.log(limit=1)]


def _fork_and_diverge(service: MnemosyneMemoryService, store: mnem.Store) -> None:
    """Two branches each write a different event with the same id,
    producing a genuine add/add conflict on merge - all through the
    service's own ordinary API."""
    _ingest(service, _event("ev-shared", "base event"))
    fork_point = store.head()

    service.branch("hypothesis")
    service.switch("hypothesis")
    _ingest(service, _event("ev-1", "confirmed by phone"))

    service.switch(fork_point.removeprefix("ref: "))
    _ingest(service, _event("ev-1", "misread from billing note"))


def test_merge_surfaces_a_real_conflict_then_resolves_it(
    service: MnemosyneMemoryService, store: mnem.Store
) -> None:
    _fork_and_diverge(service, store)

    conflicted = service.merge("hypothesis")
    assert conflicted.status == "conflicts"
    assert len(conflicted.conflicts) == 1
    conflict = conflicted.conflicts[0]
    assert conflict.kind == "add/add"

    merged = service.merge("hypothesis", resolutions={conflict.id: "theirs"})
    assert merged.status == "merged"


def test_bisect_finds_the_commit_that_added_a_node(
    service: MnemosyneMemoryService, store: mnem.Store
) -> None:
    _ingest(service, _event("ev-1", "a"))
    _ingest(service, _event("ev-2", "b"))
    second_commit = store.head_commit()
    target_id = event_node_id("myapp", "u1", "s1", "ev-2")

    boundary = service.bisect(lambda memory: target_id in memory)
    assert boundary == second_commit
