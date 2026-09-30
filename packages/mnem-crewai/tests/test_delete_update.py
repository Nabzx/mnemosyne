"""Tests for delete, update, count, list_categories, reset (#313, ADR-0021)."""

from datetime import datetime, timezone

import mnem
import pytest
from crewai.memory.types import MemoryRecord

from mnem_crewai import MnemosyneStorageBackend
from mnem_crewai.storage import _node_id


@pytest.fixture
def backend(tmp_path: object) -> MnemosyneStorageBackend:
    return MnemosyneStorageBackend(str(mnem.init(tmp_path).root))


def _seed(backend: MnemosyneStorageBackend) -> list[MemoryRecord]:
    """Several records across scopes and categories, real fixture data."""
    records = [
        MemoryRecord(
            content="enterprise plan",
            scope="/acme/billing",
            categories=["billing"],
            metadata={"tier": "enterprise"},
            created_at=datetime(2026, 1, 1, tzinfo=timezone.utc),
        ),
        MemoryRecord(
            content="support ticket 4821",
            scope="/acme/support",
            categories=["support", "billing"],
            metadata={"ticket": "4821"},
            created_at=datetime(2026, 3, 1, tzinfo=timezone.utc),
        ),
        MemoryRecord(
            content="other customer",
            scope="/other",
            categories=["billing"],
            metadata={"tier": "free"},
            created_at=datetime(2026, 6, 1, tzinfo=timezone.utc),
        ),
    ]
    backend.save(records)
    return records


def test_count_total_and_scoped(backend: MnemosyneStorageBackend) -> None:
    _seed(backend)
    assert backend.count() == 3
    assert backend.count(scope_prefix="/acme") == 2
    assert backend.count(scope_prefix="/other") == 1
    assert backend.count(scope_prefix="/nothing-here") == 0


def test_list_categories_aggregated_and_scoped(backend: MnemosyneStorageBackend) -> None:
    _seed(backend)
    assert backend.list_categories() == {"billing": 3, "support": 1}
    assert backend.list_categories(scope_prefix="/acme") == {"billing": 2, "support": 1}


def test_delete_by_record_ids(backend: MnemosyneStorageBackend) -> None:
    records = _seed(backend)
    deleted = backend.delete(record_ids=[records[0].id])
    assert deleted == 1
    assert backend.get_record(records[0].id) is None
    assert backend.count() == 2


def test_delete_by_category(backend: MnemosyneStorageBackend) -> None:
    _seed(backend)
    deleted = backend.delete(categories=["support"])
    assert deleted == 1
    assert backend.count() == 2


def test_delete_by_scope_prefix(backend: MnemosyneStorageBackend) -> None:
    _seed(backend)
    deleted = backend.delete(scope_prefix="/acme")
    assert deleted == 2
    assert backend.count() == 1
    assert backend.list_records()[0].content == "other customer"


def test_delete_by_older_than(backend: MnemosyneStorageBackend) -> None:
    _seed(backend)
    cutoff = datetime(2026, 4, 1, tzinfo=timezone.utc)
    deleted = backend.delete(older_than=cutoff)
    assert deleted == 2  # Jan and Mar records, not the June one
    assert backend.count() == 1


def test_delete_by_metadata_filter(backend: MnemosyneStorageBackend) -> None:
    _seed(backend)
    deleted = backend.delete(metadata_filter={"tier": "free"})
    assert deleted == 1
    assert backend.count() == 2


def test_delete_combines_criteria_with_and(backend: MnemosyneStorageBackend) -> None:
    _seed(backend)
    # scope /acme AND category billing - matches only the first record,
    # not the support/billing one under a different scope-only match
    deleted = backend.delete(scope_prefix="/acme", categories=["support"])
    assert deleted == 1
    assert backend.count() == 2


def test_delete_is_a_tombstone_not_a_hard_delete(
    backend: MnemosyneStorageBackend,
) -> None:
    records = _seed(backend)
    before_commit = backend._store.head_commit()
    backend.delete(record_ids=[records[0].id])

    assert backend.get_record(records[0].id) is None
    old_state = backend._store.state_at(before_commit)
    node_id = _node_id(records[0].scope, records[0].id)
    assert node_id in old_state  # still recoverable through history


def test_delete_no_match_is_a_real_noop(backend: MnemosyneStorageBackend) -> None:
    _seed(backend)
    log_before = len(backend._store.log())
    deleted = backend.delete(record_ids=["does-not-exist"])
    assert deleted == 0
    assert len(backend._store.log()) == log_before  # no new commit


def test_update_replaces_content_keeping_the_id(backend: MnemosyneStorageBackend) -> None:
    records = _seed(backend)
    updated = records[0].model_copy(update={"content": "downgraded to pro"})
    backend.update(updated)

    found = backend.get_record(records[0].id)
    assert found is not None
    assert found.content == "downgraded to pro"
    assert backend.count() == 3  # replaced, not added


def test_update_is_a_new_commit_old_value_recoverable(
    backend: MnemosyneStorageBackend,
) -> None:
    records = _seed(backend)
    before_commit = backend._store.head_commit()
    updated = records[0].model_copy(update={"content": "downgraded to pro"})
    backend.update(updated)

    node_id = _node_id(records[0].scope, records[0].id)
    old_state = backend._store.state_at(before_commit)
    assert old_state[node_id].content["content"] == "enterprise plan"


def test_update_moves_the_record_when_scope_changes(
    backend: MnemosyneStorageBackend,
) -> None:
    records = _seed(backend)
    old_node_id = _node_id(records[0].scope, records[0].id)
    moved = records[0].model_copy(update={"scope": "/acme/archive"})
    backend.update(moved)

    assert old_node_id not in backend._store.working_memory()
    found = backend.get_record(records[0].id)
    assert found is not None
    assert found.scope == "/acme/archive"
    assert backend.count() == 3  # still just a move, not a duplicate


def test_reset_scoped_empties_only_that_scope(backend: MnemosyneStorageBackend) -> None:
    _seed(backend)
    backend.reset(scope_prefix="/acme")
    assert backend.count(scope_prefix="/acme") == 0
    assert backend.count(scope_prefix="/other") == 1
    assert backend.count() == 1


def test_reset_everything_when_no_scope_given(backend: MnemosyneStorageBackend) -> None:
    _seed(backend)
    backend.reset()
    assert backend.count() == 0


def test_reset_preserves_history(backend: MnemosyneStorageBackend) -> None:
    _seed(backend)
    before_commit = backend._store.head_commit()
    log_before = len(backend._store.log())
    backend.reset()

    assert len(backend._store.log()) == log_before + 1  # one new commit
    old_state = backend._store.state_at(before_commit)
    assert len(old_state) == 3  # still recoverable at the prior commit
