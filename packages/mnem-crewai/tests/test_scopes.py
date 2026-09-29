"""Tests for the node id scheme and scope tree-walk (#311, ADR-0021)."""

from datetime import datetime, timezone

import mnem
import pytest
from crewai.memory.types import MemoryRecord

from mnem_crewai import MnemosyneStorageBackend
from mnem_crewai.storage import _content_to_record, _node_id, _record_to_content, _scope_of


@pytest.fixture
def backend(tmp_path: object) -> MnemosyneStorageBackend:
    # mnem.open() allows only one handle per store per process (redb), so
    # the temporary init()-time handle must drop before the backend opens
    # its own - matching the langgraph adapter's own proven test fixture
    # pattern, not a separately-held `store` fixture on the same path.
    return MnemosyneStorageBackend(str(mnem.init(tmp_path).root))


def _write(backend: MnemosyneStorageBackend, **kwargs: object) -> MemoryRecord:
    """Write a record directly through the backend's own store handle
    (save() isn't built yet, #312) - the same node id and content shape
    the real save() will use."""
    record = MemoryRecord(**kwargs)
    backend._store.add(_node_id(record.scope, record.id), _record_to_content(record))
    backend._store.commit(f"write {record.id}")
    return record


def test_node_id_root_scope() -> None:
    assert _node_id("/", "abc") == "/abc"


def test_node_id_nested_scope() -> None:
    assert _node_id("/company/team", "abc") == "/company/team/abc"


def test_scope_of_round_trips_with_node_id() -> None:
    for scope in ["/", "/a", "/a/b/c"]:
        node_id = _node_id(scope, "some-id")
        assert _scope_of(node_id) == scope


def test_record_content_round_trips_including_embedding() -> None:
    record = MemoryRecord(content="hello", embedding=[0.1, 0.2, 0.3])
    restored = _content_to_record(_record_to_content(record))
    assert restored.content == "hello"
    assert restored.embedding == [0.1, 0.2, 0.3]  # excluded from model_dump() by default


def test_list_scopes_immediate_children_only(backend: MnemosyneStorageBackend) -> None:
    _write(backend, content="a", scope="/a")
    _write(backend, content="a/b", scope="/a/b")
    _write(backend, content="a/b/c", scope="/a/b/c")

    assert backend.list_scopes("/") == ["/a"]
    assert backend.list_scopes("/a") == ["/a/b"]
    assert backend.list_scopes("/a/b") == ["/a/b/c"]
    assert backend.list_scopes("/a/b/c") == []


def test_list_scopes_implies_ancestor_scopes_with_no_direct_record(
    backend: MnemosyneStorageBackend,
) -> None:
    # only a record at /a/b/c - /a and /a/b are still real scopes
    _write(backend, content="deep", scope="/a/b/c")

    assert backend.list_scopes("/") == ["/a"]
    assert backend.list_scopes("/a") == ["/a/b"]
    assert backend.list_scopes("/a/b") == ["/a/b/c"]


def test_list_scopes_multiple_siblings(backend: MnemosyneStorageBackend) -> None:
    _write(backend, content="x", scope="/a")
    _write(backend, content="y", scope="/b")
    _write(backend, content="z", scope="/b/c")

    assert backend.list_scopes("/") == ["/a", "/b"]
    assert backend.list_scopes("/b") == ["/b/c"]


def test_get_scope_info_counts_subtree_not_just_exact_scope(
    backend: MnemosyneStorageBackend,
) -> None:
    _write(backend, content="1", scope="/a")
    _write(backend, content="2", scope="/a")
    _write(backend, content="3", scope="/a/b")
    _write(backend, content="4", scope="/a/b/c")

    info = backend.get_scope_info("/a")
    assert info.record_count == 4  # /a's own 2, plus /a/b's 1, plus /a/b/c's 1
    assert info.child_scopes == ["/a/b"]


def test_get_scope_info_categories_aggregated(backend: MnemosyneStorageBackend) -> None:
    _write(backend, content="1", scope="/a", categories=["billing"])
    _write(backend, content="2", scope="/a/b", categories=["support", "billing"])

    info = backend.get_scope_info("/a")
    assert info.categories == ["billing", "support"]


def test_get_scope_info_date_range(backend: MnemosyneStorageBackend) -> None:
    early = datetime(2026, 1, 1, tzinfo=timezone.utc)
    late = datetime(2026, 6, 1, tzinfo=timezone.utc)
    _write(backend, content="1", scope="/a", created_at=early)
    _write(backend, content="2", scope="/a", created_at=late)

    info = backend.get_scope_info("/a")
    assert info.oldest_record == early
    assert info.newest_record == late


def test_get_scope_info_on_empty_scope(backend: MnemosyneStorageBackend) -> None:
    info = backend.get_scope_info("/nothing-here")
    assert info.record_count == 0
    assert info.categories == []
    assert info.oldest_record is None
    assert info.newest_record is None
    assert info.child_scopes == []
