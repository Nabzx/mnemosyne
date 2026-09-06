# Research: agent memory data model survey (issue #5)

Feeds ADR-0003. Question: what exactly does Mnemosyne version, and what fields
does a memory node carry?

`CONTEXT.md` already sketches this: a memory node is content plus provenance
plus timestamps plus an optional embedding, and a claim is a node whose content
follows a schema. This survey pressure-tests that sketch against how other
systems model memory, and fills in the specifics for the grilling in #6.

## What other systems store

### mem0

`add()` extracts discrete **facts** from input text, checks each against
existing memories for semantic overlap, and emits a resolution event: ADD,
UPDATE, DELETE or NONE. A memory is a fact string plus a metadata dict (`user_id`
and arbitrary keys) plus extracted entities kept in a parallel collection.
Storage is hybrid: a vector store, a graph of entities and relations, and a
key-value store.

Takeaway: fact-oriented, with an extraction step, and update or delete happens in
place.

### Letta (MemGPT)

Memory is organised into **blocks**: a block is a labelled, persistent string
with a size limit, edited by the agent with `core_memory_append` and
`core_memory_replace`. Tiered into core (in context), recall (searchable
history) and archival (large store).

Takeaway: block-oriented, not fact-oriented. The unit is a mutable labelled
string the agent rewrites. Wrong shape for version control, since the whole
point of a block is that it is edited in place.

### Zep and Graphiti

A temporal knowledge graph. Every fact **edge** carries bi-temporal metadata:
`valid_from` and `valid_to` (event time, when the fact was true in the world)
and an ingestion time (when Zep learned it). When a fact is superseded the old
edge is **invalidated, not deleted**.

Takeaway: Zep already does "keep history, mark superseded" at the application
level, with a clean split between event time and record time. Mnemosyne should
do the same thing one layer down, in the store.

### LangGraph store

An item is `{namespace: text[], key: text, value: JSON, created_at, updated_at}`
with a primary key of `(namespace, key)`. That is the whole schema. The value is
an opaque JSON blob.

Takeaway: the minimal viable model is a keyed opaque value. Anything Mnemosyne
adds beyond this needs to earn its place.

### Generative Agents

A memory object is `{description, creation_time, last_access_time,
importance_score, embedding}`. Retrieval scores each object on recency,
importance and relevance.

Takeaway: importance and recency are **retrieval-scoring signals**, mutable and
derived, not facts about the memory. They do not belong in a versioned model.

### The 2026 versioned-memory papers

- **StateFuse** (arXiv 2607.05844): immutable history, explicit conflict
  objects, `claim_id` and `claim_ref` as exact and semantic correction handles,
  deterministic predicate contracts, and projection-time resolution that never
  rewrites replicated state.
- **MemTX** (arXiv 2607.23929): each record carries **evidence, permissions,
  provenance, and validity**. Writes are staged in snapshot-isolated
  transactions and admitted by a validate-and-commit pipeline. Irreversible tool
  calls are gated on in-flight belief state.
- **LatticeMind** (arXiv 2608.08236): explicit **status tracking** on items,
  symbolic conflict checks, and selective LLM reconciliation in one update loop.

Takeaway: provenance, evidence and validity are consistently first-class.
Contradiction is either an explicit object (StateFuse) or a status on the item
(LatticeMind); these two are in tension.

### CogCanvas ablation (arXiv 2601.00821)

A controlled ablation swapped only the stored representation in a fixed
retrieval pipeline: LLM-extracted typed artefacts versus verbatim conversation
chunks. **Verbatim won by 15.9 points on LoCoMo and 22.0 on LongMemEval-S.** The
mechanism is lossy distillation, not structure. Recommendation: structured
memory should augment verbatim text, not replace it.

Takeaway: do not force extraction or a schema. Store what the agent gives you.
Structure is optional and additive.

## Answers to the four questions

### 1. Freeform node, structured claim, or both?

**Both, with freeform as the default.** The unit is a **memory node** whose
`content` is freeform: text or an arbitrary JSON value, like LangGraph's opaque
value. A node may declare `content_kind = claim`, in which case its content
follows the claim schema. v0.1 ships only `note`.

This is the CogCanvas finding applied directly: verbatim by default, structure
as an optional overlay. It also avoids Letta's mistake of a mutable unit.

### 2. What does the claim schema need for semantic merge in v2?

- `subject`: the entity the claim is about.
- `predicate`: the attribute or relation.
- `value`: the asserted value.
- `confidence`: a number in 0 to 1, the agent's stated confidence.
- `evidence`: a list of references that support the assertion (distinct from
  provenance, see below). Optional.

`subject` and `predicate` together are what a v2 merge uses to detect that two
claims are about the same thing and might contradict. This mirrors StateFuse's
predicate contracts and MemTX's per-record validity and evidence.

### 3. How is provenance attached, and what does it point at?

**Every node carries provenance.** It is a structured record of how this node
came to exist:

- `agent_step`: which step of the run produced it.
- `observation`: a reference to the observation or input it was drawn from.
- `tool_call`: a reference to the tool call, if one produced it.
- `source`: an external source identifier, if any.

All fields are optional individually, since an agent loop may only know the
step. This is what `blame` reads.

**Provenance is not evidence.** Provenance answers "where did this record come
from" (a `blame` concern, every node). Evidence answers "what supports this
assertion" (a claim concern, optional). MemTX keeps both; so should we.

### 4. Do embeddings belong on the node, or in a side index?

**A side index.** A `redb` table in the store, keyed by content hash,
rebuildable, never part of the versioned object and never part of its content
hash. Reasons:

- Embeddings are a v2 concern (semantic merge, retrieval). v1 does not use them.
- A float array on every node bloats the object store and would pull the
  embedding model's identity into the content hash.
- The embedding model changes over time; history must not have to be rewritten
  when it does.
- mem0 and Generative Agents both keep vectors in a separate store.

## Recommendation for ADR-0003

**The memory node.** Fields:

| Field | v0.1 | Notes |
| --- | --- | --- |
| `id` | yes | a stable logical key, so updates across commits target the same node. Caller-provided, or generated if absent. Not content-derived. |
| `content` | yes | freeform: a string or an arbitrary JSON value |
| `content_kind` | yes, always `note` | `note` or `claim`. `claim` follows the claim schema. |
| `provenance` | yes | `{agent_step, observation, tool_call, source}`, all optional individually |
| `event_time` | yes | when the agent formed the node. The commit carries the record time. Zep's bi-temporal split. |

**The claim schema** (content of a `claim` node): `{subject, predicate, value,
confidence, evidence[]}`. Not built in v0.1, but defined now so ADR-0002's
self-describing objects can carry it additively at v2.

**Not on the node.** Embeddings (side index, v2). Importance and recency
(retrieval-scoring signals, mutable, out of the versioned model). Status
(asserted, superseded, contradicted): computed by the v2 merge layer, which
emits Contradiction objects rather than mutating nodes. Nodes stay immutable per
commit, matching StateFuse over LatticeMind on this point.

**Rationale.** The model is deliberately close to LangGraph's minimal item at
the core (`id` plus opaque `content`), with the two additions that version
control genuinely needs: `provenance` for `blame`, and `event_time` for an
honest history. The claim schema is defined but dormant until v2, so Phase 1
stays small and the format does not need a break to add it later.

## Open questions for the grilling (#6)

- `id`: caller-provided with a generated fallback, as recommended, or always
  generated. A content-derived id is out, since it would make "update a node"
  impossible.
- `content`: accept any JSON value plus text, or constrain it. This survey says
  accept anything; it is the agent's memory, not ours to shape.
- `event_time` in v0.1: include it now, as recommended, or rely on the commit
  timestamp until v2. One field, and `blame` reads better with it.
- Contradiction as an object (StateFuse) versus a status on the node
  (LatticeMind). This survey picks the object, to keep nodes immutable. Confirm,
  since it constrains the v2 design.
- How much provenance structure to mandate. This survey makes every field
  optional. Confirm that a node with an empty provenance is legal.

## Sources

- [State of AI Agent Memory 2026 (mem0)](https://mem0.ai/blog/state-of-ai-agent-memory-2026) and [mem0 update operations](https://docs.mem0.ai/v0x/core-concepts/memory-operations/update)
- [Letta memory blocks](https://www.letta.com/blog/memory-blocks/)
- [Zep temporal knowledge graph](https://www.getzep.com/ai-agents/temporal-knowledge-graph/) and [Graphiti](https://www.getzep.com/platform/graphiti/)
- [LangGraph stores](https://docs.langchain.com/oss/python/langgraph/stores)
- [Generative Agents (arXiv 2304.03442)](https://arxiv.org/pdf/2304.03442) and [the memory stream pattern](https://agentpatterns.ai/agent-design/generative-agents-memory-stream/)
- [StateFuse (arXiv 2607.05844)](https://arxiv.org/abs/2607.05844)
- [MemTX (arXiv 2607.23929)](https://arxiv.org/abs/2607.23929)
- [LatticeMind (arXiv 2608.08236)](https://arxiv.org/html/2608.08236)
- [CogCanvas: verbatim beats extracted artefacts (arXiv 2601.00821)](https://arxiv.org/abs/2601.00821)
