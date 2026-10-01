"""``mnem-crewai``: a CrewAI ``StorageBackend`` backed by a Mnemosyne store
(ADR-0021). Every protocol method is real - see :mod:`mnem_crewai.storage`
for the mapping and ``why``/``bisect``, the two extra methods outside the
protocol itself.
"""

from .storage import MnemosyneStorageBackend

__all__ = ["MnemosyneStorageBackend"]
