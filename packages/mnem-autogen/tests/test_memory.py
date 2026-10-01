"""Tests for the MnemosyneMemory scaffolding (#317)."""

import asyncio

import mnem
import pytest
from autogen_core.memory import Memory, MemoryContent, MemoryMimeType

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
        lambda m: m.update_context(None),
        lambda m: m.query(""),
        lambda m: m.add(MemoryContent(content="x", mime_type=MemoryMimeType.TEXT)),
        lambda m: m.clear(),
        lambda m: m.close(),
    ],
)
def test_methods_not_yet_implemented(memory: MnemosyneMemory, call) -> None:
    with pytest.raises(NotImplementedError):
        _run(call(memory))
