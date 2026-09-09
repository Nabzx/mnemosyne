"""Drive the mnem-mcp server from an MCP client, over stdio.

    python examples/mcp_client.py     # needs mnem-mcp on PATH

Spawns `mnem-mcp` as a subprocess and talks to it the way an MCP-capable agent
(Claude Desktop, an SDK client) would: record a belief, misread one, then use
`when_did` to find the commit where the wrong belief entered.
"""

from __future__ import annotations

import asyncio
import json
import tempfile

from mcp import Client, StdioServerParameters

import mnem


def data(result: object) -> object:
    structured = result.structured_content
    if isinstance(structured, dict) and list(structured) != ["result"]:
        return structured
    return json.loads(result.content[0].text) if result.content else None


async def run(store_dir: str) -> None:
    mnem.init(store_dir)
    params = StdioServerParameters(command="mnem-mcp", args=["--store", store_dir])

    async with Client(params, raise_exceptions=True) as client:
        await client.call_tool(
            "remember",
            {"id": "customer-4821", "content": "on the Enterprise plan",
             "source": "ticket-4821"},
        )
        bad = data(await client.call_tool(
            "remember",
            {"id": "customer-4821", "content": "downgraded to Pro",
             "source": "billing-note-8842", "step": "step-31"},
        ))["commit"]

        now = data(await client.call_tool("recall", {"id": "customer-4821"}))
        print("the agent now believes:", now["content"])

        found = data(await client.call_tool(
            "when_did", {"id": "customer-4821", "equals": "downgraded to Pro"},
        ))
        source = found["blame"]["node"]["provenance"]["source"]
        print(f"that belief entered at {found['commit'][:8]}, from {source}")

        assert found["commit"] == bad
        assert source == "billing-note-8842"

    print("\nOK")


if __name__ == "__main__":
    with tempfile.TemporaryDirectory() as directory:
        asyncio.run(run(directory))
