# Examples

Runnable scripts, one per surface. Each is deterministic by default (the "agent"
is a fixed script, not a model) and ends by asserting the outcome, so CI keeps
them working.

| Script | Surface | Shows |
| --- | --- | --- |
| `support_agent.py` | the Python SDK | record beliefs with provenance, then `bisect` + `blame` find where a wrong one entered |
| `claude_agent.py` | the Python SDK | a support agent is told a past answer was wrong, then diagnoses and corrects it. This is the README GIF |
| `langgraph_memory.py` | `mnem-langgraph` | Mnemosyne as a LangGraph `BaseStore`: memory across runs, a hypothesis `branch`, `why` |
| `mcp_client.py` | `mnem-mcp` | drive the MCP server over stdio the way an MCP client would |

```bash
python examples/support_agent.py
python examples/claude_agent.py                        # replay, no API key
python examples/claude_agent.py --live                 # a real Claude tool-use loop; needs anthropic + an API key
python examples/langgraph_memory.py                    # needs mnemosyne-langgraph
python examples/mcp_client.py                          # needs mnemosyne-mcp on PATH
```
