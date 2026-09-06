"""Mnemosyne: version control for AI agent memory.

This is the Python SDK. It binds the Rust core through the compiled extension
``mnem._mnem`` (built with maturin, see ``docs/adr/0004``) and wraps it in an
ergonomic, agent-facing API: :class:`Store`, the :class:`MemoryNode`,
:class:`Provenance` and :class:`Commit` dataclasses, and the exception
hierarchy rooted at :class:`MnemError`.

    import mnem

    store = mnem.init("./agent-memory")
    store.add("customer-4821", "on the Enterprise plan", provenance=mnem.Provenance(source="ticket-4821"))
    store.commit("learn the plan tier", author="support-agent")
    for c in store.log():
        print(c.id[:12], c.message)

See ``ROADMAP.md`` for the plan.
"""

from __future__ import annotations

from ._mnem import (
    ConflictError,
    CorruptStoreError,
    FormatVersionError,
    InvalidRefError,
    MnemError,
    NoStoreError,
    NotFoundError,
    StoreExistsError,
    StoreIoError,
    __version__,
    version,
)
from ._sdk import Commit, MemoryNode, Provenance, Store, init, open

__all__ = [
    "Commit",
    "ConflictError",
    "CorruptStoreError",
    "FormatVersionError",
    "InvalidRefError",
    "MemoryNode",
    "MnemError",
    "NoStoreError",
    "NotFoundError",
    "Provenance",
    "Store",
    "StoreExistsError",
    "StoreIoError",
    "__version__",
    "init",
    "open",
    "version",
]
