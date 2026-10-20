"""Tests for the MnemosyneMemoryService scaffolding (#586). Real
behaviour for each BaseMemoryService method is tested in its own
ticket's test module: add_session_to_memory in test_adk_add_session.py
(#590), add_events_to_memory in test_adk_add_events.py (#591), add_memory
in test_adk_add_memory.py (#592), search_memory in test_adk_search.py
(#593)."""

import mnem
import pytest
from google.adk.memory import BaseMemoryService

from mnem_adk import MnemosyneMemoryService


@pytest.fixture
def store(tmp_path: object) -> mnem.Store:
    return mnem.init(tmp_path)


@pytest.fixture
def service(store: mnem.Store) -> MnemosyneMemoryService:
    return MnemosyneMemoryService(store)


def test_import() -> None:
    import mnem_adk  # noqa: F401


def test_is_a_base_memory_service(service: MnemosyneMemoryService) -> None:
    assert isinstance(service, BaseMemoryService)
