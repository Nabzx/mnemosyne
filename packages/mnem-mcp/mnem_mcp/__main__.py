"""The ``mnem-mcp`` console script: an MCP server over stdio."""

from __future__ import annotations

import argparse

from .server import build_server


def main() -> None:
    parser = argparse.ArgumentParser(
        prog="mnem-mcp",
        description="MCP server for a Mnemosyne store (stdio).",
    )
    parser.add_argument(
        "--store",
        metavar="PATH",
        help="the store directory (default: $MNEM_STORE, then the current directory)",
    )
    parser.add_argument(
        "--tools",
        choices=["core", "all"],
        default="core",
        help="'all' also exposes branch / switch / merge (default: core)",
    )
    args = parser.parse_args()
    build_server(store_path=args.store, tools=args.tools).run("stdio")


if __name__ == "__main__":
    main()
