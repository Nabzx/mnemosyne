# Research: search_memory ranking parity with InMemoryMemoryService (issue #573)

Feeds ADR-0026. Confirms `InMemoryMemoryService.search_memory`'s real
ranking algorithm against its actual source at `v2.12.0`, and settles
whether `mnem-adk` should match it or degrade, against both existing
precedents for this exact question (ADR-0021, ADR-0023).

## The real algorithm, read directly from `in_memory_memory_service.py`

Not paraphrased from the earlier map notes - fetched fresh from the
`v2.12.0` tag:

1. Tokenise the query: NFC-normalise, `re.findall(r'\w+', text)`,
   lowercase - one plain token set.
2. Tokenise each candidate event's text the same way, then widen it:
   for any token that mixes scripts, split it on Latin/non-Latin
   script-run boundaries too (so a CJK sentence embedding a Latin
   word still exposes that word as its own token).
3. Score an event by counting query tokens it matches. A query token
   matches if it is in the event's token set outright, **or** - a
   real detail the map's earlier notes did not carry - if it is
   non-ASCII, by plain substring search over the event's lowercased
   text. That second path exists because CJK text carries no spaces,
   so the token-set widening in step 2 cannot be exhaustive the way
   whitespace-delimited ASCII tokenising already is.
4. Drop any event scoring zero. Sort what is left by score,
   descending; ties keep insertion order, because the sort key reads
   only the score. Keep the top 10 (`_MAX_SEARCH_RESULTS`).

Scoping is `(app_name, user_id)` only, confirmed again directly in
`_user_key` - consistent with #571. The returned `MemoryEntry` here
never sets `id` at all, only `content`, `author` and `timestamp` -
confirming #570's "memory is a corpus" finding from the write side
now also on the read side.

## Which precedent actually applies: neither transplants whole

ADR-0023 (AutoGen) and ADR-0021 (CrewAI) already settled this
question twice, but they point in different directions, and ADK's
case is closer to neither:

- **AutoGen's `ListMemory.query()` ignores its own argument outright**
  - its own docstring says so. Matching that reference's real
  behaviour meant building no ranking at all. That precedent does not
  transplant here: `InMemoryMemoryService.search_memory` is not an
  unranked reference implementation, it has a real, specific ranking
  algorithm, and the ADR-0023 principle ("match the reference, don't
  invent") now points at building that ranking, not skipping it.
- **CrewAI's `search()` takes a vector, not a string** - there is no
  keyword to match against, so a degraded keyword fallback would
  answer a different question than the one asked. ADK's
  `search_memory(*, app_name, user_id, query: str)` is the opposite
  shape: a plain string, no embedder anywhere in the contract. There
  is no vector to be faithful to here either.

Both ADRs share one underlying rule, not a specific algorithm: match
whatever the reference implementation's contract actually expects,
rather than inventing capability nothing in the ecosystem asked for.
For ADK, the reference contract expects keyword-overlap ranking
specifically, because that is what `InMemoryMemoryService` - the
service every `load_memory`/`preload_memory` sample in #570 runs
against by default - actually does.

## Recommendation for ADR-0026

Match `InMemoryMemoryService`'s algorithm exactly: same tokenising
(NFC, `\w+`, lowercase), same Latin/non-Latin script-run widening,
same ASCII-token-set-membership-or-non-ASCII-substring matching rule,
same zero-score exclusion, same stable descending sort, same top-10
cap. Any caller that swaps `mnem-adk` in for the reference service
during development should see the same ranking on the same corpus -
that parity is the actual bar here, not an approximation of it.

Related: #568 (map), #569 (confirmed `InMemoryMemoryService`'s
behaviour exists and is real), #570 (the `load_memory`/`preload_memory`
call sites this ranking ultimately feeds), #571 (the `(app_name,
user_id)` scope this search already shares).
