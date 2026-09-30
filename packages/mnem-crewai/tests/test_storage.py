"""Tests for the MnemosyneStorageBackend scaffolding (#310). get_scope_info/
list_scopes' real behaviour is tested in test_scopes.py (#311); save/
get_record/list_records' in test_records.py (#312); delete/update/count/
list_categories/reset' in test_delete_update.py (#313)."""

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
        lambda b: b.search([0.1]),
    ],
)
def test_sync_methods_not_yet_implemented(backend: MnemosyneStorageBackend, call) -> None:
    with pytest.raises(NotImplementedError):
        call(backend)


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
