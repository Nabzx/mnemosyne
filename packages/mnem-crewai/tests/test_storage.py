"""Tests for the MnemosyneStorageBackend scaffolding (#310). get_scope_info/
list_scopes' real behaviour is tested in test_scopes.py (#311); save/
get_record/list_records' in test_records.py (#312); delete/update/count/
list_categories/reset' in test_delete_update.py (#313); search's in
test_search.py (#314); the async wrappers' in test_async.py (#315). Every
``StorageBackend`` protocol method is real now - only the worked example
(#316) is left."""

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
