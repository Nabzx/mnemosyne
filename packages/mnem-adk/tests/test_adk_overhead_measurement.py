"""The real per-turn overhead measurement behind the README's own "Known
limits" section (#599) - MnemosyneMemoryService against
InMemoryMemoryService, same workload, same process, printed so a CI log
carries the real numbers rather than an estimate frozen in prose.

No strict regression gate here (unlike benchmarks/overhead.py's own
2x-baseline check) - there is no established baseline yet for this
adapter; the loose ceiling below only catches a gross regression, not a
tuning target."""

import asyncio
import statistics
import time

import mnem
import pytest
from google.adk.events.event import Event
from google.adk.memory import InMemoryMemoryService
from google.adk.sessions.session import Session
from google.genai import types

from mnem_adk import MnemosyneMemoryService

N_TURNS = 50
LOOSE_CEILING_MS = 200.0


def _event(i: int) -> Event:
    return Event(
        id=f"ev-{i}",
        author="user",
        content=types.Content(
            role="user", parts=[types.Part(text=f"turn {i} favourite colour teal")]
        ),
    )


async def _median_add_ms(service, app_name: str, user_id: str) -> float:
    times = []
    for i in range(N_TURNS):
        session = Session(id=f"s{i}", app_name=app_name, user_id=user_id, events=[_event(i)])
        start = time.perf_counter()
        await service.add_session_to_memory(session)
        times.append((time.perf_counter() - start) * 1000)
    return statistics.median(times)


async def _median_search_ms(service, app_name: str, user_id: str) -> float:
    times = []
    for _ in range(N_TURNS):
        start = time.perf_counter()
        await service.search_memory(app_name=app_name, user_id=user_id, query="favourite colour")
        times.append((time.perf_counter() - start) * 1000)
    return statistics.median(times)


@pytest.mark.filterwarnings("ignore")
def test_measured_overhead_against_in_memory_memory_service(tmp_path: object) -> None:
    async def run():
        in_memory = InMemoryMemoryService()
        add_in_memory = await _median_add_ms(in_memory, "bench", "u1")
        search_in_memory = await _median_search_ms(in_memory, "bench", "u1")

        store = mnem.init(tmp_path)
        mnemosyne = MnemosyneMemoryService(store)
        add_mnemosyne = await _median_add_ms(mnemosyne, "bench", "u1")
        search_mnemosyne = await _median_search_ms(mnemosyne, "bench", "u1")

        add_overhead = add_mnemosyne - add_in_memory
        search_overhead = search_mnemosyne - search_in_memory
        print(
            f"\nadd_session_to_memory: InMemory={add_in_memory:.4f}ms  "
            f"mnem-adk={add_mnemosyne:.4f}ms  overhead={add_overhead:.4f}ms/turn"
        )
        print(
            f"search_memory:         InMemory={search_in_memory:.4f}ms  "
            f"mnem-adk={search_mnemosyne:.4f}ms  overhead={search_overhead:.4f}ms/turn"
        )
        return add_overhead, search_overhead

    add_overhead, search_overhead = asyncio.run(run())
    assert 0 <= add_overhead < LOOSE_CEILING_MS
    assert 0 <= search_overhead < LOOSE_CEILING_MS
