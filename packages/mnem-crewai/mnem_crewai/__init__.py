"""``mnem-crewai``: a CrewAI ``StorageBackend`` backed by a Mnemosyne store
(ADR-0021).

Scaffolding only (#310) - see :mod:`mnem_crewai.storage` for what still
raises ``NotImplementedError`` and which ticket fills it in.
"""

from .storage import MnemosyneStorageBackend

__all__ = ["MnemosyneStorageBackend"]
