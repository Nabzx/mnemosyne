"""Tests for the async wrappers (#315, ADR-0021) - asave/asearch/adelete
wrap their sync counterpart in asyncio.to_thread, so each must produce the
same result as calling sync directly, and must not block the event loop
while the sync call runs."""

import asyncio
import time

import mnem
import pytest
from crewai.memory.types import MemoryRecord

from mnem_crewai import MnemosyneStorageBackend


@pytest.fixture
def backend(tmp_path: object) -> MnemosyneStorageBackend:
    return MnemosyneStorageBackend(str(mnem.init(tmp_path).root))


def _seed() -> list[MemoryRecord]:
    return [
        MemoryRecord(content="aligned", scope="/acme", categories=["billing"], embedding=[1.0, 0.0]),
        MemoryRecord(content="orthogonal", scope="/acme", categories=["support"], embedding=[0.0, 1.0]),
    ]


def test_asave_matches_save(backend: MnemosyneStorageBackend) -> None:
    records = _seed()
    asyncio.run(backend.asave(records))
    assert {r.content for r in backend.list_records()} == {"aligned", "orthogonal"}


def test_asearch_matches_search(backend: MnemosyneStorageBackend) -> None:
    backend.save(_seed())
    sync_results = backend.search([1.0, 0.0])
    async_results = asyncio.run(backend.asearch([1.0, 0.0]))
    assert [(r.content, s) for r, s in async_results] == [
        (r.content, s) for r, s in sync_results
    ]


def test_adelete_matches_delete(backend: MnemosyneStorageBackend) -> None:
    backend.save(_seed())
    deleted = asyncio.run(backend.adelete(categories=["support"]))
    assert deleted == 1
    assert {r.content for r in backend.list_records()} == {"aligned"}


def test_async_calls_do_not_block_the_event_loop(
    backend: MnemosyneStorageBackend, monkeypatch: pytest.MonkeyPatch
) -> None:
    """A real asyncio.gather test: a "slow" search (an artificial
    time.sleep injected into the sync path) runs concurrently with a
    coroutine that ticks a counter every 10ms. If asearch blocked the
    event loop (e.g. called the sync path directly instead of via
    asyncio.to_thread), the counter would not advance while it ran.
    """
    backend.save(_seed())
    real_search = backend.search

    def slow_search(*args: object, **kwargs: object) -> object:
        time.sleep(0.2)
        return real_search(*args, **kwargs)

    monkeypatch.setattr(backend, "search", slow_search)

    ticks = 0

    async def tick_counter() -> None:
        nonlocal ticks
        for _ in range(15):
            await asyncio.sleep(0.01)
            ticks += 1

    async def go() -> list[object]:
        return await asyncio.gather(backend.asearch([1.0, 0.0]), tick_counter())

    results, _ = asyncio.run(go())
    assert [r.content for r, _score in results] == ["aligned", "orthogonal"]
    assert ticks > 5  # the event loop kept ticking while the "slow" search ran in a thread
