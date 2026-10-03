"""Tests for the MnemosyneMemory scaffolding (#317). add/query/update_context's
real behaviour is tested in test_memory_ops.py (#319); clear/close's in
test_clear_close.py (#320). Every Memory method is now real."""

import mnem
import pytest
from autogen_core.memory import Memory

from mnem_autogen import MnemosyneMemory


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
