# Research: the v1 benchmark design (issue #61)

Feeds **ADR-0017** (the benchmark and its metrics) and the Phase 6 build
tickets #64 (harness and baseline) and #65 (run it, publish `docs/benchmark.md`).
Phase 6 (`v0.0.7`) ships the substrate: something you would hand a stranger,
with numbers.

**The honest frame.** Mnemosyne does not claim an agent answers better because
its memory is versioned. The 2026 research says the same: GitOfThoughts tested
five memory backends (none, markdown, vector, graph, git) and found none
reliably moves accuracy. Its conclusion, which is also ours: "git's value is
the engineering trade-off at accuracy parity. It gives auditability, history,
and the ability to merge two agents' memories, at no cost to accuracy."

So the benchmark measures the **operational properties** and the **overhead**,
not answer quality. Accuracy parity is assumed and cited, not re-tested (that
needs models, an API budget, and it has been done).

Six questions:

1. What does the benchmark measure?
2. What synthetic workload produces the runs?
3. What are the baselines to compare against?
4. What is the metric for each property, and its expected value?
5. Where does the harness live, and does CI run it?
6. What is out of scope, and how is the v2 seam (#63) kept separate?

---

## 1. What the benchmark measures

Four operational properties, each turned from "it works" into a number, plus
overhead:

| Property | The claim | Measured as |
| --- | --- | --- |
| **Reconstruction** | `state_at(commit)` is the exact memory of that moment | over N seeded runs, the fraction of commits where `state_at` equals the working memory recorded at commit time; also the golden-vector bytes are still bit-identical |
| **Bisect precision** | `bisect` finds the exact commit a belief went wrong | plant a fault at a known commit `k` in a run of length `L`; report the exact-hit rate and the distribution of `found - k` |
| **Blame accuracy** | `blame` resolves a belief to the commit and observation that set it | plant provenance on every write; `blame` every node at `HEAD`; report the fraction resolving to the correct introducing commit and the correct `source` |
| **Merge correctness** | a structural merge never loses a write or leaves a node in limbo | re-run the chaos harness (#49) at a large trial count and report totality, no-lost-writes, symmetry and convergence |
| **Overhead** | version control has a bounded, small cost | write latency per commit, read latency for the working memory, and on-disk bytes after `L` steps, each as a multiple of the baseline |

The first three are deterministic (the operations are), so the expected number
is 100.00%. The benchmark's job is to *demonstrate* that at scale and publish
it, the way `docs/chaos-report.md` does for merge. A single failing seed would
be a real bug and would block the release.

## 2. The synthetic workload

A seeded generator, in the style of `tests/time_travel.rs` and
`tests/merge_chaos.rs` (a hand-rolled LCG, no property-test dependency):

- **A run** is `L` commits. Each commit stages 1 to 4 operations over a small
  key pool: set a key to a new value, or delete one. Every set carries
  provenance (`source = "obs-<n>"`, `agent_step = "step-<c>"`).
- **A planted fault**: at a chosen commit `k`, one key is set to a distinguished
  "wrong" value and left wrong for the rest of the run. This is the
  `buggy_run.rs` fixture, parameterised.
- **A branched run**: for the merge and blame-through-merge numbers, two
  branches off a base, each with a random op sequence, merged with a strategy.

Parameters swept: `L` in {16, 64, 256, 1024}; `k` at 10%, 50%, 90% of the run;
seeds 0..S. CI uses small `L` and `S`; the published sweep uses large ones.

## 3. The baselines

"No version control" is not one thing. Two baselines, both what an agent author
would otherwise write:

- **Baseline A: a dict.** The agent keeps `dict[str, Any]` and overwrites it.
  This is GitOfThoughts's "none". Zero history.
- **Baseline B: JSONL snapshots.** After every step the agent appends the whole
  memory as one JSON line to a file. A naive "keep history" approach.

The comparison is a table: for a set of **audit queries**, which backend can
answer it, and at what storage cost.

| Audit query | dict | JSONL | Mnemosyne |
| --- | --- | --- | --- |
| current memory | yes | yes (last line) | yes |
| memory as of step `t` | no | yes (line `t`) | yes (`state_at`) |
| when did key `X` first become value `V` | no | yes, O(L) scan | yes, O(log L) `bisect` |
| which observation set key `X`'s current value | no | no (no provenance) | yes (`blame`) |
| what did step `t` change | no | yes, diff two lines | yes (`changed_by`) |
| merge two agents' memories, surfacing conflicts | no | no | yes |
| storage after `L` steps of a 100-key memory | O(keys) | **O(L x keys)** | O(distinct values) content-addressed |

JSONL answers the time questions but at linear storage and with no provenance;
the dict answers almost nothing. Mnemosyne answers all of them, and its storage
is proportional to *distinct content* because objects are content-addressed
(re-writing the same value is free).

## 4. Metrics and expected values

| Metric | Definition | Expected | Fails the release if |
| --- | --- | --- | --- |
| `reconstruction_exact` | `state_at == recorded` over all commits, all seeds | 100.00% | any seed is not exact |
| `golden_bytes_stable` | the frozen format vectors still hash identically | pass | any drift |
| `bisect_exact` | `found == k` | 100.00% | any miss on a monotonic planted fault |
| `bisect_error_bits` | histogram of `\|found - k\|` | all zero | any non-zero |
| `blame_commit_acc` | correct introducing commit, linear history | 100.00% | below 100 |
| `blame_commit_acc_merge` | correct origin commit when the value came via a merge | 100.00% | below 100 |
| `blame_source_acc` | correct `source` string | 100.00% | below 100 |
| `merge_*` | the #49 invariants over a large sweep | all hold | any violation |
| `write_ms_p50` / `p99` | per-commit latency, 100-key memory | report; expect single-digit ms p50 | regression vs the last published number by > 2x |
| `read_ms_p50` | working-memory read latency | report | as above |
| `bytes_per_step` | `.mnem` growth per commit, vs baseline B | report the multiple; expect « JSONL | as above |

The correctness metrics have a hard target (100%). The overhead metrics are
**reported, not gated on an absolute**, but a large regression from the last
published `docs/benchmark.md` should fail CI, so the number cannot rot silently.

## 5. Where it lives, and CI

- **The correctness and precision harness**: `crates/mnem-core/tests/benchmark.rs`
  (or `benches/`, decided in the ADR), hand-rolled, seeded. CI runs it at a
  small trial count as an ordinary test; a `MNEM_BENCH_*` env override drives
  the large sweep.
- **The overhead-and-baseline comparison**: `benchmarks/overhead.py` (a new
  top-level dir, like `examples/`). It builds a dict, a JSONL file and a
  `.mnem` store over the same synthetic run, times the operations, measures
  sizes, and prints the audit-query table. Deterministic; CI runs it and
  asserts the audit table and a latency ceiling.
- **The output**: `docs/benchmark.md`, written by #65 from a full sweep, in the
  shape of `docs/chaos-report.md`: what was measured, the numbers, the
  reproduction command. Re-run and updated whenever the core changes.

No new dependency: timing is `std::time::Instant` in Rust and `time.perf_counter`
in Python, consistent with the project dropping `proptest` and `time` for
hand-rolled equivalents.

## 6. Out of scope, and the v2 seam

- **Accuracy.** Not measured. Cited from GitOfThoughts. Mnemosyne is a
  substrate, not a retrieval strategy; a vector index over `working_memory` is a
  wrapper someone else writes.
- **Real models.** The workload is synthetic and deterministic so the benchmark
  is a regression gate, not a paper.
- **The prolly tree.** The `bytes_per_step` and `read_ms` numbers are the
  evidence for whether the flat `State` is still enough (ADR-0012's reassess
  trigger). The benchmark surfaces the number; it does not decide.
- **The v2 seam (#63, ADR-0018).** Declaring the `SemanticMerge` trait and the
  sync-protocol shape is a separate design with its own grilling. It is not
  benchmarked (there is no implementation). This research does not touch it.

## Recommendations, in one place

- Measure operational properties and overhead, not accuracy. Cite GitOfThoughts
  for parity.
- Correctness metrics (`reconstruction`, `bisect_exact`, `blame_*`, `merge_*`)
  have a hard 100% target and gate the release.
- Overhead metrics are reported in `docs/benchmark.md` and gated only against a
  large regression from the last published run.
- Baselines: a dict ("none") and JSONL snapshots ("naive history"), compared on
  an audit-query table and storage.
- Two harnesses: a seeded Rust one for correctness, a Python one for
  overhead-vs-baseline. Both feed `docs/benchmark.md`. No new dependency.
- Synthetic, seeded workload parameterised on run length and fault position.

## Open questions for the grilling (#62)

- **The audit-query table.** Is it in the benchmark (asserted in CI) or only in
  `docs/benchmark.md` prose? Which queries are on it?
- **Gating overhead.** What is a "large regression": a fixed multiple (2x?) of
  the published p50, or a percentage, and is it p50 or p99?
- **Harness location.** `tests/benchmark.rs` (runs with `cargo test`) or
  `benches/` (needs a bench harness, and `cargo bench` is not in CI today)?
- **The Python baseline dir.** `benchmarks/` at the top level, or a script under
  `scripts/`?
- **Run length for the published sweep.** How large is `L`, and how many seeds,
  before the numbers are "the published numbers"?
- **Blame-through-merge.** Is a less-than-100% number here acceptable to
  publish with a caveat, or must it be 100% to ship `v0.0.7`?
- **A prolly-tree trigger.** Should the ADR name a concrete `bytes_per_step` or
  `read_ms` threshold that flips the reassessment, or keep it judgement?

## Sources

- [GitOfThoughts (arXiv 2606.14470)](https://arxiv.org/abs/2606.14470): the five-backend comparison, the study / ingest / test protocol, paired-bootstrap CIs, and the "engineering trade-off at accuracy parity" conclusion
- [GitOfThoughts HTML v2](https://arxiv.org/html/2606.14470v2): Table 2 (replay / audit / merge demonstrations), Table 3 (write / read latency per backend), Table 4 and 5 (accuracy CIs and the failed replication)
- [State of AI Agent Memory 2026 (Mem0)](https://mem0.ai/blog/state-of-ai-agent-memory-2026): the benchmark landscape
- [MemoryAgentBench (ICLR 2026)](https://github.com/HUST-AI-HYZ/MemoryAgentBench): incremental multi-turn memory evaluation, an accuracy-focused contrast
- `crates/mnem-core/tests/time_travel.rs`, `tests/merge_chaos.rs`, `tests/buggy_run.rs`: the seeded harnesses this benchmark extends
- `docs/chaos-report.md`: the output format `docs/benchmark.md` follows
