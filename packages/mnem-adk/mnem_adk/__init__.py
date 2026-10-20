"""``mnem-adk``: a Google ADK ``BaseMemoryService`` backed by a Mnemosyne
store (ADR-0026).

Scaffolding only (#586) - see :mod:`mnem_adk.memory_service` for what still
raises ``NotImplementedError`` and which ticket fills it in.
"""

from .memory_service import MnemosyneMemoryService

__all__ = ["MnemosyneMemoryService"]
