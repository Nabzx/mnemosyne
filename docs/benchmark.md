# Benchmark report (issue #65)

What the substrate delivers, measured. Re-run and updated whenever the core
changes. The design is ADR-0017; the harnesses are
`crates/mnem-core/tests/benchmark.rs` (correctness) and
`benchmarks/overhead.py` (overhead and the audit table).

**Mnemosyne makes no accuracy claim.** The 2026 research is settled on this:
GitOfThoughts (arXiv 2606.14470) tested five memory backends and found none
reliably moves an agent's accuracy. The value of version control is history,
audit, and safe merging, at accuracy parity. So this benchmark measures those.

## The one-line version

| Question you might ask of an agent's memory | a dict | a JSONL log | Mnemosyne |
| --- | :---: | :---: | :---: |
| what does it believe now | yes | yes | yes |
| what did it believe at step `t` | no | yes | yes |
| when did belief `X` first go wrong | no | yes, O(L) | yes, O(log L) |
| which observation set `X` | no | no | **yes** |
| what did step `t` change | no | yes | yes |
| merge two agents' memories, surfacing conflicts | no | no | **yes** |
| storage after `L` steps | O(keys) | O(L x keys) | O(commits) |

A dict answers almost nothing. A JSONL snapshot log answers the *time* questions
but has no provenance and cannot merge. Mnemosyne answers all seven, and
`benchmarks/overhead.py` asserts this table on every CI run.

## Correctness and precision

Every one of these is deterministic, so the target is exact. The harness fails
the build on any miss.

| Metric | What it checks | Result |
| --- | --- | --- |
| `reconstruction_exact` | `state_at(commit)` equals the working memory recorded at that commit | **100.00%** |
| `golden_bytes_stable` | the frozen canonical encoding still hashes identically | **true** |
| `bisect_exact` | `bisect` returns the exact commit a planted monotonic fault began | **100.00%** |
| `bisect_error_max` | the largest `\|found - k\|` seen | **0** |
| `blame_commit_acc` | `blame` resolves to the correct introducing commit, linear history | **100.00%** |
| `blame_commit_acc_merge` | correct origin commit when the value arrived on a merged-in side | **100.00%** |
| `blame_source_acc` | correct provenance `source` string | **100.00%** |
| `merge_invariants` | symmetry, no invented ids, no double-listing (the full sweep is `merge_chaos.rs`) | **true** |

**Sweep**: 80 seeds x run lengths {16, 64, 256} x fault positions {10%, 50%,
90%} = 720 synthetic runs, about 80,000 commits reconstructed and blamed, 720
bisects, 720 blame-through-merge checks. The `overhead.py` run adds a
1024-commit run whose audit assertion is a `bisect` and a `blame` at that
length. No failing seed.

A larger sweep (the `just bench-sweep` target, thousands of seeds up to length
1024) is possible but takes hours: each commit is a durable write (an `fsync`),
which is the wall-clock floor. Since the metrics are deterministic, more seeds
buy input-shape coverage, not confidence.

## Overhead

Against the two baselines an agent author would otherwise write: a `dict` they
overwrite, and a JSONL file they append the whole memory to each step. Median of
several runs on an idle machine, a 100-key memory, values from a pool of 20.

| Steps | backend | write p50 | write p99 | read (full) | bytes / step |
| --- | --- | --- | --- | --- | --- |
| 256 | dict | ~0 ms | | ~0 ms | 0 |
| 256 | JSONL | 0.03 ms | | 0.19 ms | 830 |
| 256 | **Mnemosyne** | **12 ms** | **24 ms** | **1.5 ms** | **10,320** |
| 1024 | JSONL | 0.04 ms | | 0.65 ms | 955 |
| 1024 | **Mnemosyne** | **12 ms** | **24 ms** | **1.5 ms** | **7,470** |

Reading:

- **Write latency is ~12 ms**, flat with run length, and it is the durable
  write: a commit `fsync`s a new `State`, a `Commit` and an index entry. A dict
  is free; a JSONL append is a small `fsync`. For an agent making one memory
  update per reasoning step, 12 ms sits against an LLM call measured in seconds.
- **Read is ~1.5 ms** for the whole working memory, flat with run length. Well
  under the ~25 ms mark that would prompt reassessing the flat `State`
  (ADR-0012, ADR-0017).
- **Storage is single-digit KB per commit** and falls as the run lengthens
  (10.3 KB/step at 256, 7.5 KB/step at 1024) as redb's fixed overhead amortises.
  It is 8 to 12x a flat JSONL snapshot at these lengths: content-addressing
  bounds the cost of repeated *values*, but the per-commit `State` and `Commit`
  objects are the floor. This is the number a prolly-tree `State` would later
  share down, and the trigger for revisiting it is a real store's `.mnem`
  exceeding ~50 MB.

The CI `benchmark` job fails if `write_ms_p50` or `bytes_per_step` exceeds 2x
`benchmarks/baseline.json`, so a regression (a double `fsync`, an O(L^2)) cannot
land quietly.

## Reproducing

```
# the correctness metrics (CI runs a small count on every push)
cargo test -p mnem-core --test benchmark

# the published correctness sweep (~20 min: fsync-bound)
MNEM_BENCH_RUNS=80 MNEM_BENCH_LENGTHS=16,64,256 \
  cargo test -p mnem-core --test benchmark --release -- --nocapture

# the overhead comparison and the audit table
python benchmarks/overhead.py                 # 256 steps, the CI scale
MNEM_BENCH_STEPS=1024 python benchmarks/overhead.py

# both, via just
just benchmark        # the CI-scale run
just bench-sweep      # the published scale
```

## Sources

- [GitOfThoughts (arXiv 2606.14470)](https://arxiv.org/abs/2606.14470): five memory backends, no accuracy movement, "the engineering trade-off at accuracy parity"
- ADR-0017: the metrics, the targets, the gating
- `docs/chaos-report.md`: the merge invariants at a 50,000-case sweep, summarised here as `merge_invariants`
