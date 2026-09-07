# ADR-0016: The MCP tools and the adapter contract

- Status: Accepted
- Date: 2026-09-07
- Issue: #56 (grilling), research #55

## Context

Phase 5 (`v0.0.6`) is "it plugs in": an MCP server (`mnem-mcp`) and a LangGraph
adapter (`mnem-langgraph`), so a real agent uses Mnemosyne as its memory with
branch and blame working. This ADR fixes the MCP tool and resource surface, how
a store binds to a stateless server, and the contract the two adapters share.

ADR-0004 already settled the shape: adapters are **Python packages that depend
on the SDK, not the core, and live outside the Cargo workspace**. So `mnem-core`
stays offline and deterministic, `cargo deny` is not in play, and the network,
the async runtime and the protocol libraries all sit in the adapter packages.
This ADR does not change `format_version`: the adapters are new consumers of an
unchanged store.

The MCP spec revision assumed here is **2026-07-28**: a stateless transport (no
protocol sessions), the tools / resources / prompts primitives, stdio and
Streamable HTTP transports, and `ttlMs` / `cacheScope` on list and read results.

## Decision

### Packaging

Two new packages under `packages/` in this repo, each with its own
`pyproject.toml`, neither a Cargo member:

- `packages/mnem-mcp/` -> PyPI `mnemosyne-mcp`, import `mnem_mcp`, console
  script `mnem-mcp`.
- `packages/mnem-langgraph/` -> PyPI `mnemosyne-langgraph`, import
  `mnem_langgraph`.

Both pin `mnemosyne-agents ~= 0.0` and version independently: they are the
SDK's customers, not part of its release train. CI tests them against the built
wheel. They move to their own repositories only if an adapter grows its own
contributors.

### The `mnem.agents` module (the shared contract)

A new module in the SDK, consumed by both adapters and available to the CLI
later. It holds the ergonomics that both adapters would otherwise duplicate,
keeping ADR-0004's "the SDK holds the ergonomics" true.

```python
# mnem.agents
def remember(store, id, content, *, source=None, step=None, observation=None,
             summary=None, author="agent", time_ms=None) -> str            # commit id
def remember_many(store, items, *, summary=None, author="agent",
                  time_ms=None) -> str                                     # one commit
def forget(store, id, *, summary=None, author="agent", time_ms=None) -> str
```

Each is `add` (or `rm`) then `commit` in one call. `summary` defaults to a
generated message (`"remember {id}"`, `"remember 3 nodes"`, `"forget {id}"`).

The result dataclasses gain `to_dict()` for the JSON boundary: `Blame`,
`NodeChange`, `Commit`, `MemoryNode`, `Conflict`, `MergeResult`. The binding
already emits dicts; the SDK dataclasses round-trip to the same shape.

### The MCP tools

Nine agent-facing tools, named for how an agent reaches for them:

| Tool | Does | Arguments |
| --- | --- | --- |
| `remember` | record one fact, one commit | `id`, `content`, `source?`, `step?`, `observation?`, `summary?` |
| `revise` | same as `remember`; the name signals "this changes a belief" | same |
| `remember_many` | record several facts as one commit | `items: [{id, content, ...}]`, `summary?` |
| `forget` | tombstone one node, one commit | `id`, `summary?` |
| `recall` | read one node, or the whole current memory | `id?` |
| `recall_at` | the memory as of a past commit | `commit`, `id?` |
| `history` | recent commits, newest first | `limit?` |
| `why` | the commit and provenance that set a node | `id`, `at?` |
| `when_did` | first commit where a node reaches a value / is absent / is present | `id`, one of `equals` / `absent` / `present`, `good?` |
| `whats_new` | what a commit changed | `commit` |

`revise` and `remember` share one implementation and differ only in the
description string that the model reads.

`when_did` is the `--node/--equals/--absent/--present` helper form only,
matching the CLI (ADR-0015); the predicate-closure form has no JSON Schema.

**Harness tools.** `branch`, `switch` and `merge` (with the CLI's
`--resolve` / `--strategy` resolution surface) are implemented but hidden from
the model by default. A `mnem-mcp --tools=core|all` flag (default `core`)
controls visibility. An agent framework that drives a hypothesis branch around a
sub-task passes `--tools=all`; a bare model does not see them.

### The MCP resources

| URI | Backed by | Cache |
| --- | --- | --- |
| `mnem://memory` | `working_memory` | short `ttlMs`, `cacheScope` per-store, invalidated on any write |
| `mnem://memory/{node_id}` | `working_node` | as above |
| `mnem://log` | `log(limit)` | short `ttlMs`, per-store |
| `mnem://commit/{id}` | `state_at` + `changed_by` | long `ttlMs` (a commit is immutable) |

The resources let a client keep the current memory in context without a tool
call every turn. `recall` stays as a tool for models that pull rather than get
pushed.

### The write contract: no staging over MCP

The SDK's stage-then-commit split does not cross the MCP boundary. A tool call
is a discrete, retryable unit; a server that carried half-staged state between
calls would break the stateless model and leave memory in a limbo the model
cannot see. **Every write tool is exactly one commit**, atomic (the core's
commit is one write transaction, ADR-0008). `remember_many` is the batch form:
several `stage` calls then one `commit`.

`staged`, `unstage`, `staged_deletions`, `init`, `resolve`, `add_node`,
`format_version`, `root` and `head` are not exposed.

### Store binding

MCP 2026-07-28 is stateless. The server binds **one store per process**: the
path comes from `mnem-mcp --store <path>`, with a `MNEM_STORE` environment
fallback. Every tool call does `mnem.open` (a cheap redb handle), acts, and
returns. An agent runtime launches one `mnem-mcp` per agent, pointed at that
agent's memory.

Multi-tenant selection (a `store` argument per call, or a routing header) and
per-identity stores (post-authentication) are deferred to the collaboration
layer.

**Concurrency.** redb is one writer, many readers. Two concurrent writes to one
store serialise on the write transaction; the loser is retried by the SDK
(bounded, below) and, if it still fails, the tool returns the `busy` error code.
"Which branch am I on" is the store's `HEAD` file, not server state, so `switch`
followed by another call is consistent.

### Transport

**stdio only for `v0.0.6`.** It is the local, single-agent case the definition
of done describes, needs no authentication, and is what an agent runtime spawns.
The `mcp` Python SDK's FastMCP API writes the server once; Streamable HTTP
(remote, multi-client, the 2026-07-28 authorization surface) is later
configuration and belongs with the collaboration layer.

### The LangGraph adapter

`mnem_langgraph.MnemosyneStore` implements LangGraph's **`BaseStore`**
(cross-thread long-term memory). Not `BaseCheckpointSaver`: per-superstep graph
state is a different shape and a larger surface, deferred with a trigger
(someone wants a rewindable agent run).

Mapping:

| `BaseStore` | Mnemosyne |
| --- | --- |
| `namespace: tuple[str, ...]` | an id prefix: the node id is `":".join(namespace) + ":" + key` |
| `key: str` | the node id within the namespace |
| `value: dict` | the node content, minus a reserved `_meta` key |
| `put(ns, key, value)` | `agents.remember`, with `Provenance` lifted from `value["_meta"]` if present |
| `get(ns, key)` | `working_node` |
| `delete(ns, key)` | `agents.forget` |
| `search(ns, query?)` | a plain substring filter over node ids and stringified content within the namespace; `query` omitted lists the namespace |
| `list_namespaces()` | the set of id prefixes present |

- **Namespace is a prefix, not a branch.** Namespaces scope memory (per user,
  per topic); they are not lines of history. Branching stays an explicit
  `store.branch("hypothesis")` call.
- **Provenance travels in a reserved `_meta` key** of the opaque `value` dict
  (`{"...": ..., "_meta": {"source": ..., "step": ...}}`), which the adapter
  lifts into `Provenance` and strips from stored content. Absent `_meta` means
  empty provenance, which is legal (ADR-0003).
- **`search` is a plain filter, not semantic.** Documented as such, with a
  pointer to wrapping a vector store for ranking. Mnemosyne is not a retrieval
  layer.
- **Sync and async.** The sync methods call the SDK directly; the async methods
  (`aget`, `aput`, `asearch`, ...) wrap them in `asyncio.to_thread` so the
  event loop is not blocked.
- **Extra methods.** `store.branch(name)`, `store.switch(name)`,
  `store.why(namespace, key)` and `store.history(limit)` are on the adapter
  class, not the interface, for graph nodes to call directly. This is what
  "branch and blame working" in the definition of done means.

### SDK hardening (#59)

Four items, all in `v0.0.6`:

1. `mnem.agents` (`remember` / `remember_many` / `forget`) and the `to_dict()`
   methods, above.
2. A bounded retry-on-`ConflictError` wrapper for the atomic write helpers
   (a few attempts with a short backoff), then a raised `ConflictError` the
   caller maps to `busy`.
3. `to_dict()` round-trips on `Blame`, `NodeChange`, `Commit`, `MemoryNode`,
   `Conflict`, `MergeResult`.
4. `Store` thread-safety: documented as "one handle per thread"; the adapters
   open a handle per call, which the stateless MCP design already requires.

### Errors across the boundary

One taxonomy, mechanically derived from the core's `MnemError` hierarchy, as
lowercase string codes on MCP tool errors and adapter exceptions: `not_found`,
`invalid_ref`, `conflict`, `corrupt_store`, `no_store`, `store_io`, plus a
synthetic `busy` for write-lock contention.

### The worked examples (#60)

Three, one per surface, adjustable by #60:

1. **SDK** - a support agent that records beliefs with provenance, gets a plan
   tier wrong, and uses `bisect` + `blame` to find the misread observation (the
   running example, the `buggy_run` fixture as a real script).
2. **LangGraph** - an agent using `MnemosyneStore` as `BaseStore` across two
   threads, with a hypothesis `branch` around a sub-task.
3. **MCP** - a client script driving `mnem-mcp` over stdio through
   `remember` / `recall` / `why`.

## Consequences

- The core and the SDK are untouched except for one additive module
  (`mnem.agents`) and `to_dict()` methods. No `format_version` change.
- `mnem-mcp` is a thin FastMCP server: nine tools and four resources over the
  SDK, no state of its own. It is testable by driving it over stdio.
- The LangGraph adapter is a `BaseStore` subclass plus a handful of extra
  methods. An agent author swaps their store backend and gets history, blame
  and branching, with `search` degrading to a filter.
- The stateless, one-store-per-process design means an agent runtime scales
  memory the way it scales agents: one `mnem-mcp` each.
- Provenance survives the LangGraph path only if callers populate `_meta`.
  Documented; the MCP `remember` tool makes it a first-class argument instead.
- Two more packages to release. They version independently and pin a loose SDK
  range, so a patch SDK release does not force an adapter release.

## Alternatives considered

**Separate repositories for the adapters now.** Rejected for `v0.0.6`: three
repos to keep in sync for a solo maintainer, for no gain while the adapters are
small. The `packages/` layout keeps one tracker and lets CI test against the
wheel. Revisited when an adapter gets its own contributors.

**Expose the stage/commit split over MCP.** Rejected: it makes a tool call
stateful, fights the 2026-07-28 stateless transport, and gives a model a way to
leave memory half-written that it cannot then see.

**Map a `BaseStore` namespace to a Mnemosyne branch.** Rejected: namespaces
scope memory, they are not versions. Branch-per-namespace would multiply
histories and make the common "memories per user" case strange. Branching is an
explicit call.

**Implement `BaseCheckpointSaver` in `v0.0.6`.** Rejected: it is graph
execution state, not beliefs, and a larger surface. It is a compelling second
adapter (a rewindable run) but it waits for a real ask.

**A semantic `search`.** Rejected: Mnemosyne is not a retrieval layer (README,
prior work). A plain filter keeps the adapter a drop-in; ranking is the caller's
vector store.

**Streamable HTTP transport now.** Rejected for `v0.0.6`: it brings the
authorization surface and multi-client concerns that belong with the
collaboration layer. FastMCP adds it later by configuration, no server rewrite.
