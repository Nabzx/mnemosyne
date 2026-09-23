# ADR-0022: Mem0 and Zep on the audit-query table

- Status: Accepted
- Date: 2026-09-23
- Issue: #282 (research + grilling, one combined ticket)

## Context

ADR-0017 scopes `docs/benchmark.md`'s audit-query table to a plain `dict`, a
JSONL log, and `mnem`, reasoning (via `docs/comparisons.md`) that "a shared
numeric table would measure the wrong axis" for tools like Mem0 and Zep that
solve retrieval, not versioning. That reasoning holds fully for an
*accuracy-style* benchmark - Mem0/Zep's actual home turf, where `mnem` has
nothing to submit, since it does no semantic retrieval at all.

It does not hold for the audit-query table itself. Those seven questions
("which observation set this belief", "does a merge conflict block until
resolved", ...) are `mnem`'s own axis, not an accuracy axis, and research
(`docs/research/issue-282-mem0-zep-audit-table.md`, read against the real
`mem0ai/mem0` and `getzep/graphiti` source) found real, verifiable answers for
Mem0 and Zep on every row - several of them genuinely qualified rather than a
clean yes or no.

## Decision

### This narrows ADR-0017's scope, it does not reverse it

ADR-0017's actual point - no numeric *accuracy/relevance* benchmark against
Mem0/Zep - stands, fully, unchanged. This ADR carves out one specific,
narrower thing: the audit-query table's seven structural questions may be
answered for Mem0 and Zep, sourced and qualified, outside `docs/benchmark.md`.
`docs/benchmark.md`'s own CI-gated numbers remain scoped exactly as ADR-0017
states - dict, JSONL, `mnem`, nothing else, still asserted on every build.

### The table lives in `docs/comparisons.md`, not `docs/benchmark.md`

`docs/benchmark.md` is terse, boolean-shaped, and CI-gated - claims the build
itself verifies every run. Mem0's and Zep's behaviour isn't `mnem`'s to
assert that way: it's a claim about someone else's software that needs
re-checking by hand if they ever change it. `docs/comparisons.md` (#237) is
already the written, sourced, qualifiable home for exactly this kind of
claim, and already carries the prose version of these same findings.

### Qualified answers are written inline, not pushed to footnotes

At least three of the seven rows need a qualification for at least one tool
(e.g. Mem0: "no built-in call, but the schema permits it manually"; Zep: "yes,
via the filter API, but `search()`'s own convenience wrapper always uses
now"). The nuance *is* the finding - a reader comparing tools should see it at
the point of the claim, not chase a footnote to learn the "yes" they just read
has a real caveat.

### Letta is not added to this table

Scoped to Mem0/Zep, matching how #282 was filed. Letta's comparison stays as
existing prose in `docs/comparisons.md`; adding a third column here is scope
this ticket didn't ask for and can be a separate decision later if there's a
real reason to numerically compare Letta specifically.

### The seven rows

| Question | Mem0 | Zep (Graphiti) |
| --- | --- | --- |
| what does it believe now | yes | yes |
| what did it believe at step t | no built-in call (the `history` table is scoped per `memory_id`, no global reconstruction; the schema permits a manual per-memory replay, nothing does it for you) | yes, via `SearchFilters`' `valid_at`/`invalid_at` date filters - but `search()`'s own convenience wrapper always uses "now" |
| when did belief X first go wrong (bisect) | no | no |
| which observation set X | shallow: `actor_id`/`role` per history event | real: `EntityEdge.episodes` references the originating observation(s) |
| what did step t change | qualified: a real diff (`old_memory`/`new_memory`), but scoped to one memory, no cross-memory "commit" | no explicit diff API; derivable by comparing two point-in-time filtered queries |
| merge two agents' memories, surfacing conflicts | no | no (a later fact invalidates an earlier one; not a flagged, resolvable conflict) | 
| storage after L steps | `O(keys)` live state (vector store) + `O(events)` append-only history | `O(all facts ever asserted)`, monotonically growing |

## Consequences

- `docs/comparisons.md` gains a concrete, sourced table alongside its existing
  prose - strengthens the same claims with specifics rather than duplicating
  them.
- `docs/benchmark.md` and its CI assertion are completely untouched; nothing
  about ADR-0017's actual mechanism changes.
- The qualified-answer format means this table needs re-verification by hand
  if Mem0 or Zep materially change their history/temporal APIs - an accepted,
  stated cost of comparing against software this project doesn't control,
  same as `docs/comparisons.md`'s existing Memoria/ByteRover sections already
  carry.
- Bisect is now the one row where the written record plainly states `mnem`,
  Mem0, and Zep are *all* behind a plain JSONL log's linear scan for two of
  the three real competitors - worth being honest about rather than only
  ever stating where `mnem` wins.

## Alternatives considered

**Fold this into `docs/benchmark.md` as a new subsection.** Rejected: that
page's whole point is CI-gated numbers the build itself verifies; Mem0/Zep's
behaviour is not something `mnem`'s test suite can check, and putting it there
would misrepresent it as build-verified the way the dict/JSONL/`mnem` rows
genuinely are.

**Force every cell to a strict yes/no.** Rejected: collapses real information.
Mem0's "no, but the schema permits it manually" and Zep's "yes, but only via
the low-level filter API" are different claims than a flat no/yes, and a
reader comparing tools loses exactly the detail that makes the table useful.

**Footnote the qualifications instead of writing them inline.** Rejected:
keeps the table's surface scannable at the cost of hiding the nuance that is
the actual finding - a reader skimming the table would come away with the
wrong impression of at least three rows.

**Add Letta as a third column.** Rejected for now: out of #282's filed scope,
and Letta's existing prose comparison in `docs/comparisons.md` already covers
it; a separate decision if there's a real reason to want it numerically.

**Treat this as reversing ADR-0017 rather than narrowing it.** Rejected:
ADR-0017's actual claim (no accuracy benchmark against Mem0/Zep) is not
challenged anywhere in this research or this ADR - it remains correct and
unchanged. Framing this as a reversal would overstate what changed.
