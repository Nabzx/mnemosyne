# Research: a Mem0 migration/import path (issue #253)

Feeds the ADR that fixes what this actually is - an import tool, a written
guide, or both - not an adapter contract; Mem0 is a memory store, not an
agent framework, so there is no `Session`/`Memory`/`StorageBackend`-shaped
object to implement the way #249-#251 had.

Builds on #282's already-verified `mem0ai/mem0` source reading (Apache-2.0),
extended here with the specific question this ticket needs answered: what
does reading a caller's existing Mem0 data actually look like, end to end.

## 1. The real read API: `get_all`, scoped and capped, no cursor

`Memory.get_all(filters, top_k=20, show_expired=False)` (`mem0/memory/
main.py`) is the only bulk-read entry point. Two real constraints an
importer has to design around:

- **`filters` must name a scope up front** - at least one of `user_id`,
  `agent_id`, `run_id` - enforced by a `ValueError` if none is given. There
  is no "list every memory in the store" call; a caller has to already know
  which scopes exist. An importer cannot discover what it can migrate on its
  own - it needs the user to supply the scope identifiers themselves (their
  own `user_id` etc. from their Mem0 usage).
- **`top_k` is a cap, not a cursor.** Default 20, no offset/cursor parameter
  anywhere in the signature. A real migration needs an explicit, large
  `top_k` (or repeated calls if a caller's memory count could exceed any
  single value trusted) - silently defaulting to 20 would migrate a small,
  arbitrary slice and call it done.

## 2. What a memory item actually contains on read

`_get_all_from_vector_store` (same file) builds each returned item as:

```python
MemoryItem(
    id=mem.id,
    memory=mem.payload.get("data", ""),
    hash=mem.payload.get("hash"),
    created_at=mem.payload.get("created_at"),
    updated_at=mem.payload.get("updated_at"),
)
```

plus, if present in the payload: `user_id`, `agent_id`, `run_id`,
`actor_id`, `role`, `attributed_to`, `expiration_date` (promoted to
top-level fields), and anything else bucketed into a `metadata` dict. No
`embedding` field is returned to the caller at all - vectors stay internal
to the configured vector store, so an importer never needs to handle them.

**The null/empty-content question a real external contributor raised on
#253 (`samvallad33`'s comment) has a concrete, sourced answer now**:
`mem.payload.get("data", "")` - **Mem0's own code silently defaults to an
empty string when a record's `data` key is missing, not an error.** This
confirms the concern was real, not hypothetical: a malformed or edge-case
Mem0 record can and does produce empty content through Mem0's own official
read path. An importer has to decide explicitly what to do with that,
rather than assume it can't happen.

## 3. History: current state only, confirmed again

#282 already established Mem0's `history` table is scoped per `memory_id`,
no project-wide event log. `get_all` reads current state from the vector
store, not history - so an import, whatever form it takes, brings across
**current beliefs, not Mem0's own edit trail**. This isn't a limitation of
the importer; it's a fact about what data exists on the Mem0 side to bring
across, exactly as issue #253's own body already anticipated.

## Recommendation

Both a tool and a guide, not a choice between them - they answer different
questions. A script (or a `mnem.agents` function, better matching this
project's existing SDK-ergonomics pattern than a `scripts/` one-off) that
takes a Mem0 `Memory` instance plus explicit scope filters and `remember`s
each item into a target `mnem` store, one node per Mem0 memory id, with
`user_id`/`agent_id`/`run_id`/`actor_id` mapped into `Provenance` the same
way every adapter's provenance mapping has worked. A short migration-guide
doc explaining the real, honest limitation up front: this imports current
state as a fresh `mnem` history, not a ported-over timeline, because Mem0
never captured one.

The empty-content question resolves cleanly given the evidence: skip and
log, not import-as-empty-string and not hard-fail the whole batch. A
migration tool importing someone else's real data should be conservative
about what it writes, and a loud skip is more useful to someone debugging
a partial migration than a silent empty node or an all-or-nothing failure.

This is a recommendation the grilling should still confirm, particularly
the "skip vs. import-empty vs. fail" three-way choice, since a real external
contributor already has a stake in this exact question.

## Open questions for the grilling

- **Skip, import-as-empty, or fail the batch on empty content?** Leaning
  skip-and-log per the reasoning above - confirm.
- **Where does the tool live**: a `mnem.agents.import_mem0(...)` SDK
  function (versioned with the SDK, testable, matches ADR-0016's "the SDK
  holds the ergonomics" precedent), a standalone `scripts/` entry, or a
  small dedicated package (`packages/mnem-mem0-import` or similar)?
- **Node id scheme**: Mem0's own `memory_id` used directly as the `mnem`
  node id, or namespaced (e.g. `mem0:{memory_id}`) to make the import's
  origin visible in the id itself?
- **Scope discovery UX**: does the tool require the caller to pass
  `user_id`/`agent_id`/`run_id` explicitly every time (simple, matches
  Mem0's own API constraint), or attempt some convenience layer on top
  (there's no Mem0 API to discover scopes, so this would mean asking the
  caller to enumerate their own known scopes some other way)?
- **`top_k` / completeness**: a single large default, or a real paging loop
  that keeps raising `top_k` (or otherwise confirms completeness) until a
  call returns fewer results than requested?

## Sources

- `mem0ai/mem0` (cloned for #282, re-read here, Apache-2.0):
  `mem0/memory/main.py` (`get_all`, `_get_all_from_vector_store`'s exact
  per-item field construction, the `mem.payload.get("data", "")` default),
  `mem0/memory/storage.py` (the per-`memory_id` `history` table, already
  verified in #282).
- The real external comment on issue #253 from `samvallad33` (empty-
  embedding rejection question) - the concrete finding above is a direct,
  sourced answer to it, reframed correctly around content rather than
  embeddings (Mem0's `get_all` never returns embeddings to begin with).
