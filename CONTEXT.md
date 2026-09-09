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
not built or validated until Era 2, where semantic merge reasons over it.
_Avoid_: belief, assertion, fact

**Evidence**:
The references on a claim that support its assertion. Claim-only, optional.
Distinct from provenance, which is about the record, not the assertion.
_Avoid_: provenance, source, support

### Version control

**Commit**:
An immutable snapshot of the state, with `{kind, parents, state, message,
author, time}`. Identified by the BLAKE3 hash of its canonical bytes, which do
not include any signature. May be signed, but a signature lives in a side table
and never changes the id. See ADR-0005.
_Avoid_: checkpoint, snapshot, revision, save

**Signature**:
An ed25519 attestation over `{commit id, signer, signed_at}`, held in the store's
`signatures` side table. Optional and additive: a commit may have none, one, or
several, added at any time. Checked by `mnem verify`.
_Avoid_: seal, stamp, certificate

**Store**:
The on-disk `.mnem/` directory: a single `redb` file for objects and refs, a
plain-text `HEAD`, and a plain-text `config`. One store belongs to one agent or
one shared memory. See ADR-0002.
_Avoid_: repo, repository, database, vault

**State**:
The content-addressed set of memory nodes visible at a commit. A commit points
at one state. It is `flat` (a sorted map) now and gains a `prolly` form in
Phase 2. Distinct from working memory, which is the mutable form the agent
reads and writes.
_Avoid_: snapshot, tree, index, the memory

**Working memory**:
The current, mutable memory state the agent reads and writes, materialised from
a commit. The equivalent of a working tree.
_Avoid_: HEAD state, context, context window, RAM, live memory

**Branch**:
A ref: a mutable `name -> commit id` pointer, held in the `refs` table. Cheap to
create. Used to hold a hypothesis apart from the main line until it is kept or
dropped. The default is `main`. See ADR-0009.
_Avoid_: fork (reserved for Era 3), timeline, thread, world

**HEAD**:
The one-line plain-text file naming the current position: `ref: <branch>` when
attached, a bare commit id when detached. The source of truth for where the
next commit lands. See ADR-0009.
_Avoid_: current, tip, cursor

**Merge**:
Combining two branches into one memory state. In the substrate the merge is
deterministic and structural. In Era 2 it is semantic and reasons about
contradiction.
_Avoid_: reconcile, sync, integrate

**Conflict**:
A structural merge collision: two branches changed the same memory node in
different ways and the tool will not choose between them. Resolved by hand or by
a supplied policy.
_Avoid_: clash, contradiction (that is a separate, semantic thing in Era 2)

**Contradiction** (Era 2):
A first-class object emitted when semantic merge finds two claims that assert
incompatible things about the same subject and cannot be auto-resolved. It is
kept, not silently dropped.
_Avoid_: conflict, disagreement

**Semantic merge** (Era 2):
The content-aware second pass over the conflicts the structural merge leaves
open: a `SemanticMerge` impl reads the nodes on each side and either resolves
the conflict or emits a `Contradiction`. The trait is defined in the core
(ADR-0018); every impl that calls a model lives in a separate crate.
_Avoid_: smart merge, AI merge, auto-merge

### Collaboration (Era 2)

**Remote**:
A named pointer to another store, the way a branch is a named pointer to a
commit. The thing `mnem push` and `mnem pull` act on. Shape declared in
ADR-0018, built in Era 2.
_Avoid_: origin, upstream, peer, server

**Proposal**:
A cross-store change offered for review before it lands in shared memory: a
source ref, a target ref, the commit range, and a status. The "pull request for
memory". Declared in ADR-0018, built in Era 2.
_Avoid_: pull request, merge request, change set, patch

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

**Core**:
The `mnemosyne-store` Rust crate: the object model, the store, the commit graph, and
every deterministic operation on them. No network, no model calls, no CLI or
Python or framework concerns. Its two consumers are the CLI and the binding.
See ADR-0004.
_Avoid_: engine, kernel, backend, library

**SDK**:
The Python package `mnem`, bound to the core through pyo3. Pure ergonomics: it
adds Pythonic names, exceptions and context managers, and holds no operation
semantics. The interface an agent uses directly.
_Avoid_: client, library, wrapper

**Adapter**:
The binding between Mnemosyne and one agent framework (LangGraph, CrewAI) or one
protocol (MCP). A Python package that depends on the SDK, not the core. Lives
outside the workspace.
_Avoid_: plugin, connector, integration, driver
