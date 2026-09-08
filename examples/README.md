# Examples

Three runnable scripts, one per surface. Each is deterministic (the "agent" is a
fixed script, not a model) and ends by asserting the outcome, so CI keeps them
working.

| Script | Surface | Shows |
| --- | --- | --- |
| `support_agent.py` | the Python SDK | record beliefs with provenance, then `bisect` + `blame` find where a wrong one entered |
| `langgraph_memory.py` | `mnem-langgraph` | Mnemosyne as a LangGraph `BaseStore`: memory across runs, a hypothesis `branch`, `why` |
| `mcp_client.py` | `mnem-mcp` | drive the MCP server over stdio the way an MCP client would |

```bash
python examples/support_agent.py
python examples/langgraph_memory.py     # needs mnemosyne-langgraph
python examples/mcp_client.py           # needs mnemosyne-mcp on PATH
```
