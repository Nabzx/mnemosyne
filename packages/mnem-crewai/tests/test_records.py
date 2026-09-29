"""Tests for save, get_record, list_records (#312, ADR-0021)."""

from datetime import datetime, timezone

import mnem
import pytest
from crewai.memory.types import MemoryRecord

from mnem_crewai import MnemosyneStorageBackend


@pytest.fixture
def backend(tmp_path: object) -> MnemosyneStorageBackend:
    # mnem.open() allows only one handle per store per process (redb); the
    # temporary init()-time handle must drop before the backend opens its
    # own (#311's own lesson).
    return MnemosyneStorageBackend(str(mnem.init(tmp_path).root))


def test_save_empty_list_is_a_noop(backend: MnemosyneStorageBackend) -> None:
    backend.save([])
    assert backend.list_records() == []


def test_save_then_get_record_round_trips_including_embedding(
    backend: MnemosyneStorageBackend,
) -> None:
    record = MemoryRecord(
        content="the customer is on the Enterprise plan",
        scope="/a",
        categories=["billing"],
        source="ticket-4821",
        embedding=[0.1, 0.2, 0.3],
    )
    backend.save([record])

    found = backend.get_record(record.id)
    assert found is not None
    assert found.id == record.id
    assert found.content == record.content
    assert found.scope == record.scope
    assert found.categories == record.categories
    assert found.embedding == [0.1, 0.2, 0.3]


def test_get_record_missing_returns_none(backend: MnemosyneStorageBackend) -> None:
    assert backend.get_record("does-not-exist") is None


def test_save_is_one_commit_for_the_whole_batch(
    backend: MnemosyneStorageBackend,
) -> None:
    records = [MemoryRecord(content=str(i), scope="/a") for i in range(3)]
    before = len(backend._store.log())
    backend.save(records)
    assert len(backend._store.log()) == before + 1  # one commit, however many records
    assert len(backend.list_records()) == 3


def test_save_maps_source_to_provenance(backend: MnemosyneStorageBackend) -> None:
    record = MemoryRecord(content="x", scope="/a", source="ticket-4821")
    backend.save([record])

    blame = backend._store.blame(f"/a/{record.id}")
    assert blame.provenance.source == "ticket-4821"


def test_list_records_newest_first(backend: MnemosyneStorageBackend) -> None:
    early = MemoryRecord(content="early", scope="/a", created_at=datetime(2026, 1, 1, tzinfo=timezone.utc))
    late = MemoryRecord(content="late", scope="/a", created_at=datetime(2026, 6, 1, tzinfo=timezone.utc))
    backend.save([early, late])

    records = backend.list_records()
    assert [r.content for r in records] == ["late", "early"]


def test_list_records_scoped_to_a_subtree(backend: MnemosyneStorageBackend) -> None:
    backend.save(
        [
            MemoryRecord(content="a", scope="/a"),
            MemoryRecord(content="a/b", scope="/a/b"),
            MemoryRecord(content="other", scope="/other"),
        ]
    )

    scoped = backend.list_records(scope_prefix="/a")
    assert {r.content for r in scoped} == {"a", "a/b"}
    assert all(r.content != "other" for r in scoped)


def test_list_records_pagination(backend: MnemosyneStorageBackend) -> None:
    records = [
        MemoryRecord(
            content=str(i), scope="/a", created_at=datetime(2026, 1, i + 1, tzinfo=timezone.utc)
        )
        for i in range(5)
    ]
    backend.save(records)

    page = backend.list_records(limit=2, offset=1)
    all_records = backend.list_records()
    assert [r.content for r in page] == [r.content for r in all_records[1:3]]
