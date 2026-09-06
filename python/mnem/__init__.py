"""Mnemosyne: version control for AI agent memory.

This is the Python SDK. Phase 0 ships a placeholder so the package installs
and imports. The real API lands in Phase 1, and the Rust core is bound in
through maturin at that point (see docs/adr/0004). Until then, importing
anything other than the version will raise.

See ROADMAP.md for the plan.
"""

from __future__ import annotations

__all__ = ["__version__", "version"]

__version__ = "0.0.1"


def version() -> str:
    """Return the installed Mnemosyne SDK version."""
    return __version__


def __getattr__(name: str) -> object:  # pragma: no cover - guard rail
    raise AttributeError(
        f"mnem.{name} is not available yet. Phase 0 ships the package skeleton "
        f"only; the Store API lands in Phase 1. See ROADMAP.md."
    )
