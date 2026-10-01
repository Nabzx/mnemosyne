"""A CrewAI ``Memory`` instance backed by Mnemosyne: `remember`/`recall`
round-trip through `MnemosyneStorageBackend`, then `why`/`bisect` (extra
methods outside the `StorageBackend` protocol, ADR-0021) trace a specific
record back to the commit that wrote it.

    python examples/crewai_memory.py     # needs mnem-crewai

Deterministic like every other example, so CI can assert on it without an
API key: ``embedder`` is a tiny local bag-of-words hash (not a real
embedding model - just enough for `search`'s cosine similarity to rank
genuinely different content sensibly), and ``llm`` is a fake that raises if
ever called, proving CrewAI's own documented "pass all fields explicitly to
skip LLM analysis" path is honored for real, not just not-exercised.

``OTEL_SDK_DISABLED`` is set before importing ``crewai`` for a real,
non-obvious reason found while writing this: outside a `Crew`/`Task`
execution context, `Memory`'s internal span-parent lookup (for its OTel
event tracing) always misses and falls back only after a flat 5-second
wait per call - `CREWAI_TRACING_ENABLED` (the opt-in "send traces to
CrewAI's backend" flag) does not gate this, since it is a separate,
always-on internal mechanism. Setting the OTel SDK's own standard
kill-switch avoids it entirely; a real crew running inside `Crew.kickoff()`
would not need this.
"""

from __future__ import annotations

import os

os.environ["OTEL_SDK_DISABLED"] = "true"

import hashlib
import tempfile

from crewai.llms.base_llm import BaseLLM
from crewai.memory import Memory
from crewai.memory.storage.factory import set_memory_storage_factory
from mnem_crewai import MnemosyneStorageBackend

import mnem

_VOCAB_BUCKETS = 16


class _NeverCalledLLM(BaseLLM):
    """Passing scope/categories/importance explicitly to `remember()` is
    supposed to skip LLM analysis entirely (CrewAI's own documented
    behaviour) - this raises if that claim is ever wrong."""

    def call(self, *args: object, **kwargs: object) -> str:
        raise AssertionError("remember() should not need the LLM: all fields were explicit")


def _fake_embedder(texts: list[str]) -> list[list[float]]:
    """A local, deterministic stand-in for a real embedding model: each
    word hashes into one of 16 buckets, the embedding is the bucket-count
    vector. Enough for a real, if crude, cosine similarity ranking between
    genuinely different content - not a real embedding model, no network
    call, no API key."""
    vectors = []
    for text in texts:
        vector = [0.0] * _VOCAB_BUCKETS
        for word in text.lower().split():
            bucket = int(hashlib.sha256(word.encode()).hexdigest(), 16) % _VOCAB_BUCKETS
            vector[bucket] += 1.0
        vectors.append(vector)
    return vectors


def run(store_dir: str) -> None:
    mnem.init(store_dir)
    backend = MnemosyneStorageBackend(store_dir)
    # A one-time, process-wide call (ADR-0021) - not scoped to this one
    # Memory instance the way MnemosyneSession/MnemosyneStore are.
    set_memory_storage_factory(lambda spec: backend)

    memory = Memory(storage="mnem", embedder=_fake_embedder, llm=_NeverCalledLLM(model="fake"))

    plan = memory.remember(
        "the acme account is on the enterprise plan",
        scope="/acme/billing",
        categories=["billing"],
        importance=0.8,
        source="billing-agent",
    )
    plan_commit = backend._store.head_commit()
    ticket = memory.remember(
        "support ticket 4821 was resolved by refunding a late fee",
        scope="/acme/billing",
        categories=["billing", "support"],
        importance=0.5,
        source="support-agent",
    )
    ticket_commit = backend._store.head_commit()

    hits = memory.recall("what plan is the acme account on", depth="shallow")
    print("recall ranked:")
    for hit in hits:
        print(f"  {hit.score:.3f}  {hit.record.content}")

    blame = backend.why(plan.id)
    print(f"\nwhy {plan.id}:")
    print(f"  commit:  {blame.commit[:8]}")
    print(f"  source:  {blame.provenance.source}")
    print(f'  content: "{blame.node.content["content"]}"')

    ticket_node_suffix = f"/{ticket.id}"
    boundary = backend.bisect(
        lambda working_memory: any(nid.endswith(ticket_node_suffix) for nid in working_memory)
    )

    assert hits[0].record.content == plan.content
    assert blame.commit == plan_commit
    assert blame.provenance.source == "billing-agent"
    assert boundary == ticket_commit
    print("\nOK")


if __name__ == "__main__":
    with tempfile.TemporaryDirectory() as directory:
        run(directory)
