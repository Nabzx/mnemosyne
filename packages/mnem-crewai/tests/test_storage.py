"""Tests for the MnemosyneStorageBackend scaffolding (#310)."""

import asyncio

import mnem
import pytest
from crewai.memory.storage.backend import StorageBackend

from mnem_crewai import MnemosyneStorageBackend


@pytest.fixture
def backend(tmp_path: object) -> MnemosyneStorageBackend:
    mnem.init(tmp_path)
    return MnemosyneStorageBackend(str(tmp_path))


def test_import() -> None:
    import mnem_crewai  # noqa: F401


def test_satisfies_storage_backend_protocol(backend: MnemosyneStorageBackend) -> None:
    assert isinstance(backend, StorageBackend)


@pytest.mark.parametrize(
    "call",
    [
        lambda b: b.save([]),
        lambda b: b.search([0.1]),
        lambda b: b.delete(),
        lambda b: b.get_record("x"),
        lambda b: b.list_records(),
        lambda b: b.get_scope_info("/"),
        lambda b: b.list_scopes(),
        lambda b: b.list_categories(),
        lambda b: b.count(),
        lambda b: b.reset(),
    ],
)
def test_sync_methods_not_yet_implemented(backend: MnemosyneStorageBackend, call) -> None:
    with pytest.raises(NotImplementedError):
        call(backend)


def test_update_not_yet_implemented(backend: MnemosyneStorageBackend) -> None:
    from crewai.memory.types import MemoryRecord

    with pytest.raises(NotImplementedError):
        backend.update(MemoryRecord(content="x"))


@pytest.mark.parametrize(
    "call",
    [
        lambda b: b.asave([]),
        lambda b: b.asearch([0.1]),
        lambda b: b.adelete(),
    ],
)
def test_async_methods_not_yet_implemented(backend: MnemosyneStorageBackend, call) -> None:
    with pytest.raises(NotImplementedError):
        asyncio.run(call(backend))
