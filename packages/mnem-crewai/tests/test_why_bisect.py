"""Tests for why/bisect (#316, ADR-0021) - extra methods outside the
StorageBackend protocol, same shape as the LangGraph and OpenAI Agents SDK
adapters' own (ADR-0016, ADR-0020)."""

import mnem
import pytest
from crewai.memory.types import MemoryRecord

from mnem_crewai import MnemosyneStorageBackend
from mnem_crewai.storage import _node_id


@pytest.fixture
def backend(tmp_path: object) -> MnemosyneStorageBackend:
    return MnemosyneStorageBackend(str(mnem.init(tmp_path).root))


def test_why_returns_the_commit_and_provenance_that_wrote_the_record(
    backend: MnemosyneStorageBackend,
) -> None:
    record = MemoryRecord(content="enterprise plan", scope="/acme", source="billing-agent")
    backend.save([record])

    blame = backend.why(record.id)

    assert blame.commit == backend._store.head_commit()
    assert blame.provenance.source == "billing-agent"
    assert blame.node.content["content"] == "enterprise plan"


def test_why_reflects_the_latest_update(backend: MnemosyneStorageBackend) -> None:
    record = MemoryRecord(content="enterprise plan", scope="/acme", source="billing-agent")
    backend.save([record])
    updated = record.model_copy(update={"content": "downgraded to pro", "source": "support-agent"})
    backend.update(updated)

    blame = backend.why(record.id)

    assert blame.commit == backend._store.head_commit()
    assert blame.provenance.source == "support-agent"
    assert blame.node.content["content"] == "downgraded to pro"


def test_why_raises_for_an_unknown_record_id(backend: MnemosyneStorageBackend) -> None:
    with pytest.raises(mnem.InvalidRefError):
        backend.why("does-not-exist")


def test_bisect_finds_the_commit_where_a_record_first_appears(
    backend: MnemosyneStorageBackend,
) -> None:
    first = MemoryRecord(content="first", scope="/acme")
    backend.save([first])
    boundary_commit = backend._store.head_commit()

    second = MemoryRecord(content="second", scope="/acme")
    backend.save([second])

    node_id = _node_id(second.scope, second.id)
    boundary = backend.bisect(lambda memory: node_id in memory)

    assert boundary == backend._store.head_commit()
    assert boundary != boundary_commit
