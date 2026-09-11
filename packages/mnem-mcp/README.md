# mnem-mcp

An [MCP](https://modelcontextprotocol.io) server that exposes a
[Mnemosyne](https://github.com/Nabzx/mnemosyne) store to an agent: its memory as
tools and resources, so the agent records what it learns, and you can trace a
wrong belief later.

```bash
pip install mnem-mcp
mnem-mcp --store ./agent-memory  # speaks MCP over stdio
```

## Tools

`remember`, `revise`, `remember_many`, `forget`, `recall`, `recall_at`,
`history`, `why`, `when_did`, `whats_new`. With `--tools all`, also `branch`,
`switch` and `merge` (for a harness, not usually the model).

Every write tool is one commit. Provenance travels on `remember` as `source` /
`step` / `observation`.

## Resources

`mnem://memory`, `mnem://memory/{id}`, `mnem://log`, `mnem://commit/{id}`, so a
client keeps the current memory in context without a tool call every turn.

## How it binds

One store per server process, from `--store` or `$MNEM_STORE`: a single handle
is opened at startup and reused for every call (redb allows only one open
handle per file per process). This is not protocol state: the 2026-07-28 MCP
transport is stateless, and branch state still lives in the store's `HEAD`
file, which the handle re-reads on every operation. Concurrent writes
serialise and retry; a persistent loser returns the `conflict` code.

See [ADR-0016](https://github.com/Nabzx/mnemosyne/blob/main/docs/adr/0016-mcp-tools-and-the-adapter-contract.md).
