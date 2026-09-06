"""Mnemosyne: version control for AI agent memory.

This is the Python SDK. Phase 1 binds the Rust core through the compiled
extension ``mnem._mnem`` (built with maturin, see ``docs/adr/0004``) and
re-exports it here. The ergonomic layer, context managers and dataclasses,
lands in a later ticket (#30); for now the surface is the binding itself.

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
    Store,
    StoreExistsError,
    StoreIoError,
    __version__,
    version,
)

__all__ = [
    "ConflictError",
    "CorruptStoreError",
    "FormatVersionError",
    "InvalidRefError",
    "MnemError",
    "NoStoreError",
    "NotFoundError",
    "Store",
    "StoreExistsError",
    "StoreIoError",
    "__version__",
    "version",
]
