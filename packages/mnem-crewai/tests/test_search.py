"""Tests for search via real cosine similarity (#314, ADR-0021/ADR-0025)."""

import math

import mnem
import pytest
from crewai.memory.types import MemoryRecord

from mnem_crewai import MnemosyneStorageBackend


@pytest.fixture
def backend(tmp_path: object) -> MnemosyneStorageBackend:
    return MnemosyneStorageBackend(str(mnem.init(tmp_path).root))


def _hand_cosine(a: list[float], b: list[float]) -> float:
    dot = sum(x * y for x, y in zip(a, b, strict=True))
    return dot / (math.sqrt(sum(x * x for x in a)) * math.sqrt(sum(y * y for y in b)))


def _seed(backend: MnemosyneStorageBackend) -> list[MemoryRecord]:
    """Three 2-D embeddings with a clean, hand-computable ranking against
    the query [1.0, 0.0]: aligned > diagonal > orthogonal."""
    records = [
        MemoryRecord(
            content="aligned",
            scope="/acme/billing",
            categories=["billing"],
            metadata={"tier": "enterprise"},
            embedding=[1.0, 0.0],
        ),
        MemoryRecord(
            content="orthogonal",
            scope="/acme/support",
            categories=["support"],
            metadata={"tier": "free"},
            embedding=[0.0, 1.0],
        ),
        MemoryRecord(
            content="diagonal",
            scope="/other",
            categories=["billing"],
            metadata={"tier": "enterprise"},
            embedding=[1.0, 1.0],
        ),
    ]
    backend.save(records)
    return records


def test_search_ranks_identically_to_hand_computed_cosine(
    backend: MnemosyneStorageBackend,
) -> None:
    _seed(backend)
    query = [1.0, 0.0]
    results = backend.search(query)

    assert [r.content for r, _score in results] == ["aligned", "diagonal", "orthogonal"]
    for record, score in results:
        assert score == pytest.approx(_hand_cosine(query, record.embedding))


def test_search_respects_scope_prefix(backend: MnemosyneStorageBackend) -> None:
    _seed(backend)
    results = backend.search([1.0, 0.0], scope_prefix="/acme")
    assert {r.content for r, _score in results} == {"aligned", "orthogonal"}


def test_search_respects_categories(backend: MnemosyneStorageBackend) -> None:
    _seed(backend)
    results = backend.search([1.0, 0.0], categories=["support"])
    assert [r.content for r, _score in results] == ["orthogonal"]


def test_search_respects_metadata_filter(backend: MnemosyneStorageBackend) -> None:
    _seed(backend)
    results = backend.search([1.0, 0.0], metadata_filter={"tier": "free"})
    assert [r.content for r, _score in results] == ["orthogonal"]


def test_search_respects_min_score(backend: MnemosyneStorageBackend) -> None:
    _seed(backend)
    # cosine(aligned) = 1.0, cosine(diagonal) ~= 0.707, cosine(orthogonal) = 0.0
    results = backend.search([1.0, 0.0], min_score=0.5)
    assert [r.content for r, _score in results] == ["aligned", "diagonal"]


def test_search_respects_limit(backend: MnemosyneStorageBackend) -> None:
    _seed(backend)
    results = backend.search([1.0, 0.0], limit=1)
    assert [r.content for r, _score in results] == ["aligned"]


def test_search_excludes_records_with_no_embedding(
    backend: MnemosyneStorageBackend,
) -> None:
    _seed(backend)
    backend.save([MemoryRecord(content="no embedding at all", scope="/acme/billing")])
    results = backend.search([1.0, 0.0])
    assert "no embedding at all" not in {r.content for r, _score in results}


def test_search_does_not_filter_private_records(backend: MnemosyneStorageBackend) -> None:
    """Private-enforcement is CrewAI's own RecallFlow's job, not the storage
    layer's - the real StorageBackend.search protocol never passes a
    requester identity to filter by (ADR-0025)."""
    backend.save(
        [
            MemoryRecord(
                content="private note",
                scope="/acme",
                embedding=[1.0, 0.0],
                private=True,
                source="agent-a",
            )
        ]
    )
    results = backend.search([1.0, 0.0])
    assert "private note" in {r.content for r, _score in results}


def test_search_empty_store_returns_empty(backend: MnemosyneStorageBackend) -> None:
    assert backend.search([1.0, 0.0]) == []
