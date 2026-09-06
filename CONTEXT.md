# Mnemosyne

Version control for AI agent memory. An agent's memory is stored as a
content-addressed history: every change is a commit, memory can be branched
and merged, and any belief can be traced back to what produced it.

This file is the glossary. When an issue, a commit, an ADR or a test names one
of these concepts, use the term as defined here. Do not drift to the synonyms
under _Avoid_. New terms are added lazily, as decisions settle, not upfront.

## Language

### The unit of memory

**Memory node**:
The versioned unit. Its `id` (a stable logical key), its `content` (a string or
any JSON value, stored verbatim), its `content_kind` (`note` or `claim`), its
provenance, and an optional `event_time`. Immutable within a commit: an update
is the same `id` with new content in a new commit. See ADR-0003.
_Avoid_: fact, record, entry, item, memory (on its own)

**Provenance**:
How a memory node came to exist: `agent_step`, `observation`, `tool_call`,
`source`, and a free-text `note`. Every field is optional and an empty
provenance is legal. It is what `blame` reads. Distinct from evidence.
_Avoid_: source, origin, metadata, trace

**Claim**:
A memory node with `content_kind = claim`, whose content is shaped as `subject`,
`predicate`, `value`, `confidence`, `evidence`. Defined in ADR-0003 but dormant:
not built or validated until v2, where semantic merge reasons over it.
_Avoid_: belief, assertion, fact

**Evidence**:
The references on a claim that support its assertion. Claim-only, optional.
Distinct from provenance, which is about the record, not the assertion.
_Avoid_: provenance, source, support

### Version control

**Commit**:
An immutable, content-addressed, signed snapshot of the memory state, with a
message and one or more parent pointers. Identified by the hash of its content.
_Avoid_: checkpoint, snapshot, revision, save

**Store**:
The on-disk `.mnem/` directory: a single `redb` file for objects and refs, a
plain-text `HEAD`, and a plain-text `config`. One store belongs to one agent or
one shared memory. See ADR-0002.
_Avoid_: repo, repository, database, vault

**State**:
The content-addressed set of memory nodes visible at a commit. A commit points
at one state. It is `flat` (a sorted map) in v0.1 and gains a `prolly` form in
Phase 2. Distinct from working memory, which is the mutable form the agent
reads and writes.
_Avoid_: snapshot, tree, index, the memory

**Working memory**:
The current, mutable memory state the agent reads and writes, materialised from
a commit. The equivalent of a working tree.
_Avoid_: HEAD state, context, context window, RAM, live memory

**Branch**:
A named line of memory development. Cheap to create. Used to hold a hypothesis
apart from the main line until it is kept or dropped.
_Avoid_: fork (reserved for v3), timeline, thread, world

**Merge**:
Combining two branches into one memory state. In v1 the merge is deterministic
and structural. In v2 it is semantic and reasons about contradiction.
_Avoid_: reconcile, sync, integrate

**Conflict**:
A structural merge collision: two branches changed the same memory node in
different ways and the tool will not choose between them. Resolved by hand or by
a supplied policy.
_Avoid_: clash, contradiction (that is a separate, semantic thing in v2)

**Contradiction** (v2):
A first-class object emitted when semantic merge finds two claims that assert
incompatible things about the same subject and cannot be auto-resolved. It is
kept, not silently dropped.
_Avoid_: conflict, disagreement

### Operations

**Blame**:
Resolving, for a given memory node, the commit and the provenance that
introduced it.
_Avoid_: annotate, trace back

**Bisect**:
Binary searching a range of commits to find the one where a supplied predicate
first becomes true, for example "this wrong belief is present".
_Avoid_: search, hunt

**Time travel**:
Materialising working memory exactly as it stood at a chosen past commit.
_Avoid_: rewind, rollback, restore, replay

### Integration

**Adapter**:
The binding between Mnemosyne and one agent framework (LangGraph, CrewAI) or one
protocol (MCP). Adapters live outside `mnem-core` and depend on the SDK.
_Avoid_: plugin, connector, integration, driver

**SDK**:
The Python package `mnem`. The interface an agent uses directly. Wraps the Rust
core.
_Avoid_: client, library, wrapper
