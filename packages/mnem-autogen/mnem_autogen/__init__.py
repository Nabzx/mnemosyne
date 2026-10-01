"""``mnem-autogen``: an AutoGen ``Memory`` backed by a Mnemosyne store
(ADR-0023).

Scaffolding only (#317) - see :mod:`mnem_autogen.memory` for what still
raises ``NotImplementedError`` and which ticket fills it in.
"""

from .memory import MnemosyneMemory

__all__ = ["MnemosyneMemory"]
