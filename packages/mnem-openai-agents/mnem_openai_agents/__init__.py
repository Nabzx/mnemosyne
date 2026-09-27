"""``mnem-openai-agents``: an OpenAI Agents SDK :class:`Session` backed by a
Mnemosyne store (ADR-0020).

Scaffolding only (#303) - see :mod:`mnem_openai_agents.session` for what
still raises ``NotImplementedError`` and which ticket fills it in.
"""

from .session import MnemosyneSession

__all__ = ["MnemosyneSession"]
