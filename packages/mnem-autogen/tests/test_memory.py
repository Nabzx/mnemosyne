"""Tests for the MnemosyneMemory scaffolding (#317). add/query/update_context's
real behaviour is tested in test_memory_ops.py (#319)."""

import asyncio

import mnem
import pytest
from autogen_core.memory import Memory

from mnem_autogen import MnemosyneMemory


def _run(coro):
    return asyncio.run(coro)


@pytest.fixture
def memory(tmp_path: object) -> MnemosyneMemory:
    mnem.init(tmp_path)
    return MnemosyneMemory("test-memory", str(tmp_path))


def test_import() -> None:
    import mnem_autogen  # noqa: F401


def test_is_a_real_memory_subclass(memory: MnemosyneMemory) -> None:
    assert isinstance(memory, Memory)


def test_name_round_trips() -> None:
    mem = MnemosyneMemory.__new__(MnemosyneMemory)
    mem._name = "my-agent"
    assert mem.name == "my-agent"


@pytest.mark.parametrize(
    "call",
    [
        lambda m: m.clear(),
        lambda m: m.close(),
    ],
)
def test_methods_not_yet_implemented(memory: MnemosyneMemory, call) -> None:
    with pytest.raises(NotImplementedError):
        _run(call(memory))
