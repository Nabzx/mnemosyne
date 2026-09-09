"""``mnemosyne-agents-mcp``: an MCP server that exposes a Mnemosyne store as tools and
resources (ADR-0016).

Run it with the ``mnemosyne-agents-mcp`` console script, or::

    from mnemosyne_mcp import build_server
    build_server(store_path="./agent-memory").run("stdio")
"""

from .server import build_server

__all__ = ["build_server"]
