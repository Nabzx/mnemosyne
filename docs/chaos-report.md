# Merge chaos report (issue #49)

Phase 3's definition of done: the merge is not just implemented, it is
*hammered*. This is the first report. It will be re-run and updated whenever the
merge algorithm or the store flow changes.

The harness is `crates/mnem-store/tests/merge_chaos.rs`. It is seeded (a
hand-rolled LCG, the same approach as `tests/time_travel.rs`), so every run is
deterministic and a failing seed reproduces exactly. No property-test
dependency is pulled in (ADR-0004: the core stays lean and offline).

## What it checks

The invariants are the ones named in the research
(`docs/research/issue-42-merge-survey.md`, section 4) and locked by ADR-0013 and
ADR-0014.

### 1. The pure function — `merge_state_maps(base, ours, theirs)`

Each trial builds a random base map (six possible keys, each present ~2/3 of the
time, values drawn from a pool of four object ids) and diverges it twice into
`ours` and `theirs` with a few random set / delete ops. Then:

| Invariant | Assertion |
| --- | --- |
| **Totality** | every id across `base ∪ ours ∪ theirs` is in `merged` **or** in `conflicts`, never both, never neither |
| **No lost writes** | for every non-conflict id the merged value is exactly what the per-id rule dictates (`o` if `o == t`; `t` if `b == o`; `o` if `b == t`) |
| **No invented ids** | nothing in `merged` or `conflicts` that was not in an input |
| **Conflict ordering** | conflicts come back in id order, no duplicates, each carrying the true `(base, ours, theirs)` triple |
| **Clean-merge symmetry** | swapping `ours` and `theirs` gives the *identical* merged map and the *identical* conflict id set (the kinds mirror: edit/delete ↔ delete/edit) |
| **Idempotence** | `merge_state_maps(base, ours, ours)` is clean and returns `ours` unchanged |
| **Base identity** | `merge_state_maps(base, ours, base)` is clean and returns `ours`; `merge_state_maps(base, base, theirs)` is clean and returns `theirs` |

### 2. The store flow — `Store::merge`

Each trial initialises a real store, writes a base commit, branches, and makes
one or two random commits on each side. Then:

- an **`Ours`-strategy merge never surfaces a conflict** — the outcome is always
  `Merged`, `FastForwarded` or `AlreadyUpToDate`;
- a real merge commit **has exactly two parents**, ours first;
- **convergence**: merging `main` back into `feature` afterwards fast-forwards
  (or is already up to date) onto *exactly* the same memory state — the two
  branches agree node for node;
- **flow-level idempotence**: re-merging the already-merged branch is a no-op
  (`AlreadyUpToDate`).

## Results

CI runs the harness on every push at a small trial count (600 pure, 24 store) —
a few seconds. The counts are overridable with `MNEM_CHAOS_ALGEBRA_TRIALS` and
`MNEM_CHAOS_STORE_TRIALS` for a manual sweep.

| Sweep | Trials | Result |
| --- | --- | --- |
| CI (every push) | 600 pure + 24 store | pass |
| Manual, this report | **50,000 pure + 250 store** | **pass, no failing seed** |

```
MNEM_CHAOS_ALGEBRA_TRIALS=50000 MNEM_CHAOS_STORE_TRIALS=250 \
  cargo test -p mnem-store --test merge_chaos --release
# test merge_state_maps_algebra ... ok
# test store_merge_converges ... ok
# test result: ok. 2 passed; 0 failed; finished in 37.85s
```

Every one of the 50,000 pure cases classified every id and lost no write; every
one of the 250 store histories converged when merged both ways. No seed has
failed since the harness was written.

## Reproducing a failure

If CI ever reports a failing seed `N`, run it alone:

```
MNEM_CHAOS_ALGEBRA_TRIALS=<N+1> cargo test -p mnem-store --test merge_chaos -- merge_state_maps_algebra
```

The assertion message names the seed and the id, and the LCG is pure, so the
case is fully determined by `N`.
