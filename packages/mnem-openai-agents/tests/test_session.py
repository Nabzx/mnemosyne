"""Tests for the MnemosyneSession scaffolding (#303). get_items/add_items'
real behaviour is tested in test_items.py (#305)."""

import asyncio

import mnem
import pytest
from agents.memory.session import Session

from mnem_openai_agents import MnemosyneSession


@pytest.fixture
def session(tmp_path: object) -> MnemosyneSession:
    store = mnem.init(tmp_path)
    return MnemosyneSession(store, "u1")


def test_import() -> None:
    import mnem_openai_agents  # noqa: F401


def test_satisfies_session_protocol(session: MnemosyneSession) -> None:
    assert isinstance(session, Session)


def test_session_id_and_settings(session: MnemosyneSession) -> None:
    assert session.session_id == "u1"
    assert session.session_settings is None


@pytest.mark.parametrize(
    "call",
    [
        lambda s: s.pop_item(),
        lambda s: s.clear_session(),
    ],
)
def test_methods_not_yet_implemented(session: MnemosyneSession, call) -> None:
    with pytest.raises(NotImplementedError):
        asyncio.run(call(session))
