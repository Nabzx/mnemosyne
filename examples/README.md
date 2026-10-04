# Examples

Runnable scripts, one per surface. Each is deterministic by default (the "agent"
is a fixed script, not a model) and ends by asserting the outcome, so CI keeps
them working.

A new worked example is a good first contribution - this directory is open to
external PRs (see `CONTRIBUTING.md`).

| Script | Surface | Shows |
| --- | --- | --- |
| `support_agent.py` | the Python SDK | record beliefs with provenance, then `bisect` + `blame` find where a wrong one entered |
| `claude_agent.py` | the Python SDK | a support agent is told a past answer was wrong, then diagnoses and corrects it. This is the README GIF |
| `langgraph_memory.py` | `mnem-langgraph` | Mnemosyne as a LangGraph `BaseStore`: memory across runs, a hypothesis `branch`, `why` |
| `mcp_client.py` | `mnem-mcp` | drive the MCP server over stdio the way an MCP client would |
| `openai_agents_memory.py` | `mnem-openai-agents` | a real `Runner.run` tool-calling turn, provenance auto-capture, then `why` + `bisect` trace the answer to the exact tool call |
| `crewai_memory.py` | `mnem-crewai` | a real `Memory` instance, `remember`/`recall` round-tripping through `MnemosyneStorageBackend`, then `why` + `bisect` trace a record to the commit that wrote it |
| `autogen_memory.py` | `mnem-autogen` | a real `AssistantAgent`, `update_context` demonstrably injecting stored memories before inference, then `why` + `bisect` trace a memory to the commit that wrote it |
| `cross_framework_merge.py` | `mnem-crewai` + `mnem-langgraph` | two different framework adapters writing their own branches of one shared store; a real conflict between two CrewAI branches is surfaced and resolved, while an independent LangGraph branch merges in cleanly alongside it |

```bash
python examples/support_agent.py
python examples/claude_agent.py                        # replay, no API key
python examples/claude_agent.py --live                 # a real Claude tool-use loop; needs anthropic + an API key
python examples/langgraph_memory.py                    # needs mnem-langgraph
python examples/mcp_client.py                          # needs mnem-mcp on PATH
python examples/openai_agents_memory.py                # needs mnem-openai-agents
python examples/crewai_memory.py                       # needs mnem-crewai
python examples/autogen_memory.py                      # needs mnem-autogen[examples]
python examples/cross_framework_merge.py                # needs mnem-crewai, mnem-langgraph
```
