# Research: Mem0 and Zep against the audit-query table (issue #282)

Feeds the ADR that supersedes ADR-0017's scoping decision (`docs/benchmark.md`'s
audit-query table stays scoped to dict/JSONL/`mnem`) by adding Mem0 and Zep as
verified columns - not an accuracy benchmark (ADR-0017's own point stands: that
axis measures the wrong thing for a retrieval-first tool), but the table's
seven structural questions, which are `mnem`'s actual axis and are answerable
from Mem0's and Zep's own source, not their marketing pages.

Read against real source, not docs: `mem0ai/mem0` (Apache-2.0,
`mem0/memory/{storage,main}.py`) and `getzep/graphiti` (Apache-2.0,
`graphiti_core/{edges,graphiti}.py`, `search/search_filters.py`) - Graphiti is
the open-source temporal-graph engine Zep Cloud is built on; its repo is what's
actually inspectable, same reasoning `docs/comparisons.md` already used citing
Graphiti for Zep.

## The seven questions, verified

**1. What does it believe now?** Both: yes, trivially - `Memory.get_all()`
(Mem0) and a `search()` call with no time filter (Graphiti) both read current
state. Not interesting on its own; every memory system clears this bar.

**2. What did it believe at step t?**

- **Mem0: no built-in call.** `storage.py`'s `history` table is scoped to one
  `memory_id` at a time (`get_history(memory_id)` - `WHERE memory_id = ?`).
  There is no API that reconstructs the *whole* memory set as it stood at an
  arbitrary past time; a caller would have to walk every `memory_id`'s own
  history table and manually take the latest `new_memory` before a cutoff for
  each one. The schema doesn't forbid this (it's the same trick a raw JSONL
  replay uses), but nothing in the SDK does it for you.
- **Zep (Graphiti): yes, via explicit filter construction.** `EntityEdge`
  carries real bi-temporal fields - `valid_at`, `invalid_at`, `expired_at`
  (`edges.py`) - and `SearchFilters` (`search_filters.py`) exposes `valid_at`/
  `invalid_at` as first-class `DateFilter` lists with comparison operators
  (`gt`/`lt`/`gte`/`lte`). A caller who constructs `valid_at <= t AND
  (invalid_at IS NULL OR invalid_at > t)` gets a real point-in-time
  reconstruction. Not a single `state_at(t)` call the way `mnem`'s is - the
  caller builds the filter - but genuinely supported by the data model, not
  approximated. `Graphiti.search()`'s convenience wrapper itself always uses
  "now" as its reference time (stated in its own docstring), so this only
  works through the lower-level filter API, not the one-liner.

**3. When did belief X first go wrong (bisect, ideally sublinear)?** Both:
**no**. Neither exposes anything resembling a binary search over history.
Mem0's `history` table and Graphiti's temporal edges are both linearly
queryable by a caller willing to write the loop themselves, but there is no
built-in bisect-shaped primitive in either - this is the one row where both
tools are flatly behind a plain JSONL log's `O(L)` linear scan, since neither
even offers that as a single call.

**4. Which observation set X (provenance)?**

- **Mem0: a shallow but real tag.** Every `history` row carries `actor_id` and
  `role` (`storage.py`'s schema) - who or what triggered the change. No
  structured source/step/observation object, just an actor identifier per
  event.
- **Zep (Graphiti): a real reference, not just a tag.** `EntityEdge.episodes:
  list[str]` (`edges.py`) holds the UUIDs of the episodic node(s) that support
  a fact - an actual link back to the raw observation(s) that established it,
  closer in spirit to `mnem`'s `Provenance` than Mem0's actor tag is.

**5. What did step t change (diff)?** Both: **qualified yes, scoped to one
item, not a global step.** Mem0's `old_memory`/`new_memory` pair on a history
row is a real diff, but only for that one `memory_id`'s own trail - there is
no cross-memory "commit" concept, so "what did step t change" has no global
answer the way `mnem whats_new(commit)` does (a commit can touch many nodes
atomically). Graphiti has no explicit diff API at all; a caller would compare
two point-in-time `SearchFilters` queries themselves - technically derivable
from question 2's answer, not a first-class call.

**6. Merge two agents' memories, surfacing conflicts?** Both: **no** -
reconfirms `docs/comparisons.md`'s existing finding (#237). Mem0 has no
branch/fork concept at all. Graphiti's graph is a single shared structure per
`group_id`; two divergent views can't be reconciled as a flagged, resolvable
conflict - a later fact simply invalidates an earlier one via `expired_at`,
which is a real form of history but not a merge.

**7. Storage after L steps?**

- **Mem0**: current state lives in whatever vector-store backend is
  configured (Qdrant, etc.), roughly `O(keys)`, updated in place; the SQLite
  `history` table is append-only and grows with every event, `O(events)`.
  Structurally this is close to `mnem`'s own current-state-plus-full-history
  split, not meaningfully worse on this axis alone.
- **Zep (Graphiti)**: the graph grows monotonically - a superseded fact's edge
  gets `expired_at` set, it isn't deleted - so storage is `O(all facts ever
  asserted)`, closer to a JSONL log's `O(L)` character than a dict's `O(keys)`.

## Recommendation

Both are real "qualified yes / no" tables, not clean yes/no rows - exactly the
nuance issue #282 itself flagged as an open question. Recommend: **qualified
answers, not forced booleans** (e.g. "no built-in call, but the schema
permits it manually" is a materially different claim than a flat "no", and
collapsing it loses real information a technical reader would want). Recommend
the table lives in `docs/comparisons.md` (a written comparison, room for the
qualification text a cell needs) rather than `docs/benchmark.md` (CI-gated,
terse, boolean-shaped, and ADR-0017's own point - this isn't a number the
build should assert on, since Mem0's and Zep's behaviour isn't `mnem`'s to
verify on every CI run the way its own metrics are).

## Open questions for the grilling

- **Where does the table live?** `docs/comparisons.md` (recommended above) or
  a new subsection of `docs/benchmark.md`, kept separate from the CI-gated
  numbers?
- **Format for a qualified answer** - a short parenthetical in the cell (as
  used above), or a table of qualifiers plus footnotes, given at least three
  of the seven rows need one for at least one of the two tools?
- **Does this table also gain Letta**, the third real comparison already in
  `docs/comparisons.md`'s prose (\"closest of the mainstream memory tools\" per
  the earlier research), or stay scoped to Mem0/Zep as #282 was filed?
- **The ADR that supersedes ADR-0017**: does it fully replace ADR-0017's
  scoping paragraph, or add a narrower amendment specifically carving out the
  structural-question table from the numeric-benchmark scope ADR-0017 still
  correctly states for accuracy-style comparisons?

## Sources

- `mem0ai/mem0` (cloned at research time, Apache-2.0): `mem0/memory/storage.py`
  (the `history` table schema, `get_history`), `mem0/memory/main.py` (`history`,
  `get`, `get_all` as the only public read surface).
- `getzep/graphiti` (cloned at research time, Apache-2.0):
  `graphiti_core/edges.py` (`valid_at`/`invalid_at`/`expired_at`/`episodes`
  fields), `graphiti_core/graphiti.py` (`retrieve_episodes`, `search`'s
  "current date and time" docstring note), `graphiti_core/search/
  search_filters.py` (`SearchFilters`' `DateFilter` lists).
- [ADR-0017](../adr/0017-the-benchmark-and-its-metrics.md) (the scoping
  decision this research proposes narrowing, not reversing).
- `docs/comparisons.md` (#237's existing Mem0/Zep prose, which this table
  makes concrete and numeric without repeating).
