# ADR-0003: The memory node model

- Status: Accepted
- Date: 2026-09-06
- Issue: #6 (grilling), #5 (research)

## Context

A memory node is the unit Mnemosyne versions. The #5 survey looked at how mem0,
Letta, Zep, LangGraph, Generative Agents, and the 2026 versioned-memory papers
(StateFuse, MemTX, LatticeMind) model memory, and at the CogCanvas ablation.

The findings that shaped this decision:

- **CogCanvas** (arXiv 2601.00821): swapping only the stored representation in a
  fixed pipeline, verbatim chunks beat LLM-extracted typed artefacts by 15.9
  points on LoCoMo and 22.0 on LongMemEval-S. The mechanism is lossy
  distillation. Structure should augment verbatim text, not replace it.
- **LangGraph** proves the minimal viable model: a key and an opaque value.
- **Zep** splits event time (when a fact was true) from ingestion time (when it
  was learned), and invalidates superseded facts rather than deleting them.
- **MemTX** keeps provenance, evidence and validity as distinct first-class
  fields.
- **StateFuse** keeps contradictions as explicit objects over an immutable
  history; **LatticeMind** puts a mutable status on each item. These are in
  tension.
- **Letta**'s block is a mutable labelled string, which is the wrong shape for
  version control.

This ADR fixes the node model. It does not build the claim schema, the embedding
index, or anything in v2.

## Decision

### The memory node

A memory node has five fields.

| Field | Type | Required in v0.1 | Notes |
| --- | --- | --- | --- |
| `id` | string | yes | a stable logical key |
| `content` | string or any JSON value | yes | freeform, stored verbatim |
| `content_kind` | enum | yes, always `note` | `note` or `claim` |
| `provenance` | object | yes, may be empty | how the node came to exist |
| `event_time` | timestamp or null | no | when the agent formed the node |

**`id`** is caller-provided, with a generated fallback. An agent with a natural
key (an entity id, a fact key) supplies it, so successive writes to the same
fact share an `id` and `blame` and `diff` line up. An agent with no key gets a
generated `id`, and every write is a fresh node. `id` is never content-derived,
because that would make an update impossible.

**Update** means: same `id`, new `content`, recorded in a new commit. The old
version stays reachable through history. A node is **immutable within a commit**;
it carries no mutable field.

**`content`** is a string or any JSON value, unconstrained. Mnemosyne is a
version-control tool: it stores faithfully what the agent wrote and does not
reshape it. A soft size warning may be added later; there is no hard limit.

**`content_kind`** is present from v0.1 with `note` as the only value the v0.1
reader accepts. `claim` is named here and in the format spec but is rejected by a
v0.1 reader. Per ADR-0002, adding `claim` in v2 is then an additive change, not a
format break.

**`event_time`** is an optional timestamp: when the agent formed the belief, as
distinct from the commit's record time. If absent, `blame` falls back to the
commit time. It is one nullable field, and a temporal field added later would be
a semantically meaningful format change, so it goes in now.

### Provenance

Provenance answers "where did this record come from". It is read by `blame`.
Every node has a provenance object, which may be entirely empty.

| Field | Type | Notes |
| --- | --- | --- |
| `agent_step` | string or null | which step of the run produced it |
| `observation` | reference or null | the observation or input it was drawn from |
| `tool_call` | reference or null | the tool call, if one produced it |
| `source` | reference or null | an external source identifier |
| `note` | string or null | free text for anything the fields above do not fit |

An agent loop may only know the step, or nothing. Refusing a memory for lack of
provenance would be worse than storing it bare, so an empty provenance is legal.
`bisect` operates on node content and presence, not provenance, so it needs
nothing here.

Provenance is not evidence. Evidence (below) is claim-only and says what
supports the assertion. MemTX keeps both; so do we.

### The claim schema (defined, dormant until v2)

A node with `content_kind = claim` has `content` shaped as:

| Field | Type | Notes |
| --- | --- | --- |
| `subject` | reference | the entity the claim is about |
| `predicate` | string | the attribute or relation |
| `value` | any JSON value | the asserted value |
| `confidence` | float 0 to 1 or null | the agent's stated confidence |
| `evidence` | list of references | what supports the assertion. May be empty. |

This ADR fixes the field names and their types so ADR-0002's format can carry a
`claim` object additively. It **defers to a v2 grilling**: the `predicate`
vocabulary, how `subject` identity is resolved, and how `confidence` is
calibrated. Nothing is built or validated in v0.1, so a v2 grilling can still
widen a field.

### Status and contradiction

A memory node has no `status` field. Supersession is already visible in the
commit graph: a later commit with a new `content` for the same `id` supersedes
the earlier one. Contradiction becomes a separate **Contradiction object** in
v2, designed by a v2 grilling. This follows StateFuse over LatticeMind, and
keeps nodes consistent with ADR-0002's content-addressed state.

### Embeddings

Embeddings are not on the node and not in its content hash. They live in a side
index: a `redb` table keyed by content hash, with the `model_id` recorded
alongside so that changing the embedding model forces a rebuild rather than
silent drift. Not built in v0.1, since v1 has no semantic operations. A local
embedding model on an M1 produces roughly 1.5 to 4 KB per node, a few megabytes
for a large store, and the index is rebuildable, so the choice holds.

Retrieval-scoring signals (importance, recency) are not part of the versioned
model at all. They are mutable and derived, and belong to a retrieval layer if
one is ever built.

## Consequences

- v0.1 needs only `id`, `content`, `content_kind = note`, `provenance` and an
  optional `event_time`. Small.
- v2 adds `claim`, the Contradiction object, and the embedding index without a
  format break, because the object model was built to absorb them.
- An agent can write arbitrary JSON, so the store faithfully reflects whatever
  the agent's memory actually was, garbage included. That is the correct
  behaviour for version control, but it means Mnemosyne offers no schema
  guarantees about `content`.
- `blame` quality depends on the agent populating provenance. An agent that
  passes nothing gets `blame` that resolves only to the commit.
- Pinning the claim schema shape now carries a small risk of pinning a field
  we later regret. Mitigated by deferring all claim *semantics* to v2 and
  building nothing against the schema until then.

## Alternatives considered

**Letta-style mutable blocks.** Rejected. The unit is designed to be edited in
place, which defeats version control.

**mem0-style forced fact extraction.** Rejected. CogCanvas shows extraction
loses 15 to 22 points against verbatim, and it puts an LLM call on the write
path, which v1 forbids.

**A `status` field on the node (LatticeMind).** Rejected. A mutable field fights
ADR-0002's content-addressed state, and supersession is already in the graph.
Contradiction as a separate object (StateFuse) keeps nodes immutable.

**Embeddings on the node.** Rejected. It would pull the embedding model's
identity into the content hash and force history rewrites when the model
changes.

**Omit `content_kind` until claims arrive.** Rejected. Adding a discriminator
later is the format break ADR-0002's self-describing objects exist to avoid.

**Rely on the commit timestamp instead of `event_time`.** Rejected for v0.1.
A temporal field added later is a meaningful semantic change; one nullable field
now is cheap.
