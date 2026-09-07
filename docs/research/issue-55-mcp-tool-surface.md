# Research: the MCP tool surface for memory (issue #55)

Feeds **ADR-0016** (the MCP tools and the adapter contract) and the Phase 5
build tickets #57 to #60. Phase 5 (`v0.0.6`) is "it plugs in": an MCP server and
a LangGraph adapter, so a real agent uses Mnemosyne as its memory with branch
and blame working, plus SDK hardening and three worked examples.

**The frame is already set by ADR-0004.** Adapters, meaning the MCP server
(`mnem-mcp`) and the framework adapters (`mnem-langgraph`), are Python packages
that depend on the **SDK**, not the core, and live **outside the Cargo
workspace**. So `mnem-core` stays offline and deterministic; `cargo deny` is not
in play here; the network, the async runtime and the protocol libraries all sit
in separate Python packages.

Six questions:

1. Which SDK operations become MCP **tools**, and which become MCP **resources**?
2. What is the write contract: one commit per call, or a stage/commit split?
3. How is a store bound to a server, given MCP 2026-07-28 is stateless?
4. What does the LangGraph adapter implement (`BaseStore`, `BaseCheckpointSaver`,
   or both), and how does its namespace/key model map onto Mnemosyne?
5. What is the shared **adapter contract**, and does the SDK need anything new?
6. Where do the adapter packages live, and how are they named and versioned?

---

## 1. Tools vs resources

MCP (spec revision **2026-07-28**) gives a server three primitives:

- **Tools**: model-invoked functions with a name, description and JSON Schema.
  Side effects allowed. This is where writes and searches go.
- **Resources**: readable context addressed by URI, pulled in by the client
  (app-controlled, not model-invoked). List and read results now carry `ttlMs`
  and `cacheScope`. This is where "the current memory" goes.
- **Prompts**: user-invoked templates. Not needed for `v0.0.6`.

### The tools

The agent-facing verbs, mapped from the SDK. Names are chosen for how an agent
would reach for them, not to mirror Git:

| Tool | SDK call(s) | Notes |
| --- | --- | --- |
| `remember` | `add` + `commit` | the common case: record one fact with provenance, in one commit. `{ id, content, source?, step?, observation? }` |
| `revise` | `add` + `commit` | same shape; a separate name so the model signals "this changes an existing belief" |
| `forget` | `rm` + `commit` | tombstone one node in a commit |
| `recall` | `working_node` / `working_memory` | read one node or the whole current memory. Also a resource (below); the tool form is for when the model decides it needs it |
| `recall_at` | `state_at` | the memory as of a past commit (time travel) |
| `history` | `log` | recent commits, newest first, `{ limit? }` |
| `why` | `blame` | resolve a node to the commit and provenance that set it |
| `when_did` | `bisect` | first commit where a node reaches a value, is absent, or is present |
| `whats_new` | `changed_by` | what a commit changed |

Branch, checkout and merge are **orchestration**, not in-loop reasoning. They
are exposed as tools too (`branch`, `switch`, `merge`, with the same resolution
surface as the CLI), but the ADR should mark them "for the harness, not the
model": an agent framework drives a hypothesis branch around a sub-task, and the
model rarely asks for one itself. A server flag (`--tools=core|all`) or MCP tool
annotations can hide them from a model that does not need them.

`bisect` on MCP is the `--node/--equals/--absent/--present` helper form only
(the predicate-closure form has no JSON-Schema representation), matching the CLI
(ADR-0015).

### The resources

| URI | Backed by | `cacheScope` |
| --- | --- | --- |
| `mnem://memory` | `working_memory` | per-store; short `ttlMs`, invalidated on any write |
| `mnem://memory/{node_id}` | `working_node` | per-store |
| `mnem://log` | `log(limit=N)` | per-store |
| `mnem://commit/{id}` | `state_at` + `changed_by` | immutable, long `ttlMs`, since a commit never changes |

Putting the current memory behind a resource means a client can keep it in
context for free, without spending a tool call every turn. The `recall` tool
stays for models that pull rather than get pushed.

### Not exposed

`init` (a deployment step, not an agent action), `format_version` / `root` /
`head` (introspection), `staged` / `unstage` / `staged_deletions` (there is no
staging on MCP, see §2), `resolve` (internal), `add_node` (the dataclass form).

---

## 2. The write contract: no staging over MCP

The SDK has a stage-then-commit split (`add` writes to staging, `commit` seals
it). Over MCP that split is a liability: a tool call is a discrete, retryable
unit, and a server that carries half-staged state between calls breaks the
stateless model (§3) and leaves an agent's memory in a limbo the model cannot
see.

**Every write tool is one commit.** `remember` / `revise` / `forget` each do
`add` or `rm` then `commit` in a single SDK call sequence, atomically (the
core's commit is one write transaction, ADR-0008). The commit message is the
tool's `summary` argument, or a generated one (`"remember {id}"`); the author is
the server's configured agent name.

Batch writes, meaning "record these five facts as one commit", are a real case
(an agent finishing a step). One option is a `remember_many` tool taking a list,
one commit. The ADR should decide whether that lands in `v0.0.6` or waits.

---

## 3. Binding a store to a server

MCP 2026-07-28 is **stateless**: no protocol sessions, no `Mcp-Session-Id`, any
request answerable by any server instance. So the server cannot "hold" a store
handle across a session the way a CLI process does.

Options, cheapest first:

- **(a) One store per server process.** The store path is a launch argument
  (`mnem-mcp --store ./agent-memory`) or env var. Every tool call opens the
  store fresh (`mnem.open` is cheap, a redb handle), acts, closes. Stateless,
  trivial, correct. This is the `v0.0.6` recommendation: an agent runtime
  launches one `mnem-mcp` per agent, pointed at that agent's memory.
- **(b) Store selected per call.** A `store` argument on every tool, or the
  `Mcp-Name`-style header carrying it. Needed only for a multi-tenant server
  hosting many agents' memories behind one process. Defer.
- **(c) Store per MCP client identity** (post-auth). Era 2, the collaboration
  layer, where a store is shared and access-controlled.

**Concurrency.** redb gives one writer, many readers (ADR-0008). Two concurrent
`remember` calls to the same store serialise on the write transaction; the loser
retries. The server should surface a clean "busy, retry" rather than block
indefinitely, which is a small SDK hardening item (#59).

**Branch state.** `mnem.open` resolves `HEAD` from the store's `HEAD` file, so
"which branch am I on" is store state, not server state, which is consistent
with statelessness. A `switch` tool writes `HEAD`; the next call sees it.

---

## 4. The LangGraph adapter

LangGraph persistence has two interfaces:

- **`BaseCheckpointSaver`**: saves the full graph state after every node, keyed
  by thread. `put` / `get_tuple` / `list`. This is *execution* state (the
  channels, the pending writes), not beliefs.
- **`BaseStore`**: cross-thread, long-lived memory. Namespaced by a tuple of
  strings, `key` a string, `value` an arbitrary dict. `get` / `put` / `search` /
  `delete` / `list_namespaces`, plus async forms.

**The Phase 5 DoD, "a LangGraph agent uses Mnemosyne as its memory with branch
and blame working", is `BaseStore`.** Recommendation: implement `BaseStore`
first.

The mapping is close to 1:1:

| `BaseStore` | Mnemosyne |
| --- | --- |
| `namespace: tuple[str, ...]` | a prefix on the node id (`":".join(namespace)`), or a branch, per the ADR |
| `key: str` | the node id (within the namespace) |
| `value: dict` | `MemoryNode.content` (JSON) |
| `put(ns, key, value)` | `remember`: add + commit, provenance from `value["_meta"]` if present |
| `get(ns, key)` | `working_node` |
| `search(ns, query)` | `working_memory` filtered. Mnemosyne has no vector search; the ADR decides whether `search` is prefix/substring only or out of scope for `v0.0.6` |
| `delete(ns, key)` | `forget` |
| `list_namespaces()` | the set of id prefixes seen |

`branch` and `blame` are not on the `BaseStore` interface. They are exposed as
extra methods on the adapter class (`store.branch("hypothesis")`,
`store.why(key)`), which a graph node calls directly. That satisfies "branch and
blame working" without fighting the interface.

`BaseCheckpointSaver` (every superstep is a commit) is a compelling second
adapter, since it makes a whole agent run rewindable, but it is a different
shape and a bigger surface. Name it in the ADR as deferred, with a trigger.

---

## 5. The adapter contract, and SDK gaps

The MCP server and the LangGraph adapter should share one thin **contract
module** so #57 and #58 do not diverge:

- **`remember(id, content, *, source=None, step=None, observation=None,
  summary=None) -> commit_id`**: the atomic add+commit. Both adapters call this.
- **`forget(id, *, summary=None) -> commit_id`**.
- A stable mapping from `Blame` / `NodeChange` / `Commit` to plain JSON dicts
  (the MCP tool results and the adapter return values).
- One error taxonomy surfaced as JSON (`not_found`, `invalid_ref`, `conflict`,
  `busy`).

Where this lives: a small `mnem` SDK addition (`mnem.agents` or top-level
helpers), *not* in each adapter. That keeps "the SDK holds the ergonomics"
(ADR-0004) true, and means the CLI could use the same `remember` helper later.

**SDK hardening (#59) that this surfaces:**

- `remember` / `forget` atomic helpers (above).
- A retry-on-write-conflict wrapper, or a clear `ConflictError` the caller can
  catch (`compare_and_set` already raises one in the core).
- `to_dict()` on `Blame`, `NodeChange`, `Commit`, `MemoryNode` for the JSON
  boundary (the binding already emits dicts; the SDK dataclasses should
  round-trip).
- Confirm thread-safety of a `Store` handle, or document "one handle per thread"
  and have the adapters open per-call.

---

## 6. Packaging

ADR-0004 says adapters live outside the workspace. Two readings:

- **A `packages/` (or `adapters/`) directory in this repo**, each its own
  `pyproject.toml`, not a Cargo member, published to PyPI separately. One repo,
  one issue tracker, and CI can test them against the built wheel. **Recommended
  for `v0.0.6`**: a separate repo per adapter is overhead the project does not
  need yet.
- **Separate repos** (`Nabzx/mnem-mcp`, `Nabzx/mnem-langgraph`). Cleaner
  dependency story, but three repos to keep in sync for a solo maintainer.
  Revisit if an adapter grows its own contributors.

Names: PyPI `mnemosyne-mcp` and `mnemosyne-langgraph` (the SDK is
`mnemosyne-agents`); import names `mnem_mcp` / `mnem_langgraph`. The `mnem-mcp`
console script is the server entry point.

Versioning: the adapters pin `mnemosyne-agents ~= 0.0` and version
independently. They are the SDK's customers, not part of its release train.

---

## Transport and auth

- **stdio only for `v0.0.6`.** It is the local, single-agent case the DoD
  describes, needs no auth, and is what an agent runtime spawns. Streamable HTTP
  (remote, multi-client) brings the 2026-07-28 authorization story and belongs
  with the collaboration layer.
- The `mcp` Python SDK's FastMCP API covers both; the server is written once and
  gains HTTP later by configuration.

## Recommendations, in one place

- **Tools**: `remember` / `revise` / `forget` / `recall` / `recall_at` /
  `history` / `why` / `when_did` / `whats_new`; `branch` / `switch` / `merge`
  behind a "harness, not model" annotation.
- **Resources**: `mnem://memory`, `mnem://memory/{id}`, `mnem://log`,
  `mnem://commit/{id}`, so the current memory is pull-free context.
- **Writes**: no staging over MCP; every write tool is one atomic commit.
- **Store binding**: one store per server process, path from a launch argument;
  stateless, open-per-call. Multi-tenant selection deferred.
- **LangGraph**: implement `BaseStore` first; `branch` / `why` as extra adapter
  methods; `BaseCheckpointSaver` deferred with a trigger.
- **Contract**: a shared `remember` / `forget` / `to_dict` helper set in the
  **SDK**, consumed by both adapters.
- **Packaging**: a `packages/` directory in this repo, PyPI `mnemosyne-mcp` /
  `mnemosyne-langgraph`, versioned independently.
- **Transport**: stdio only for `v0.0.6`.

## Open questions for the grilling (#56)

- **Write granularity.** Is `remember_many` (batch, one commit) in `v0.0.6`, or
  is one-fact-one-commit enough to ship?
- **The `_meta` channel.** How does provenance travel through `BaseStore.put`'s
  opaque `value` dict: a reserved `_meta` key, a separate `put` argument the
  adapter adds, or dropped (provenance only via the MCP `remember` tool)?
- **`search`.** Prefix/substring filter over `working_memory`, or "not in
  `v0.0.6`, raise `NotImplementedError`"? Mnemosyne is explicitly not a retrieval
  layer (README, prior work).
- **Namespace.** Does a `BaseStore` namespace map to an id prefix, or to a
  Mnemosyne branch? (Prefix is simpler; branch is more powerful and matches
  "branch working".)
- **Harness tools.** Hide `branch` / `merge` from the model by default, or
  expose everything and trust the framework?
- **One repo or many.** `packages/` here, or `Nabzx/mnem-mcp` +
  `mnem-langgraph` from the start?
- **Checkpointer.** Confirm `BaseCheckpointSaver` is deferred, and name its
  trigger (someone wants a rewindable run).

## Sources

- [MCP 2026-07-28 specification](https://blog.modelcontextprotocol.io/posts/2026-07-28/): stateless transport, tools/resources/prompts, `ttlMs` / `cacheScope`, header routing
- [modelcontextprotocol.io](https://modelcontextprotocol.io): the primitives, stdio vs Streamable HTTP
- [modelcontextprotocol/python-sdk](https://github.com/modelcontextprotocol/python-sdk) and [`mcp` on PyPI](https://pypi.org/project/mcp/): FastMCP, `mcp run` / `mcp dev`
- [LangGraph persistence: checkpointers and BaseStore](https://medium.com/@gmshakil786/langgraph-persistence-explained-from-scratch-part-2-production-backends-human-in-the-loop-and-cf20f34ade4a) and [BaseStore for long-term memory](https://www.abstractalgorithms.dev/langgraph-memory-and-state-persistence): the two interfaces, namespace/key/value model
- [langgraph-checkpoint-aws](https://pypi.org/project/langgraph-checkpoint-aws/): a worked custom-backend example (`BaseCheckpointSaver` subclass)
- ADR-0004 (the core / SDK boundary): adapters depend on the SDK, live outside the workspace
- ADR-0015 (blame and bisect): the `--node/--equals/--absent/--present` predicate helpers the MCP `when_did` tool reuses
