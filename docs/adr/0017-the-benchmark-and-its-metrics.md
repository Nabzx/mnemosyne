# ADR-0017: The benchmark and its metrics

- Status: Accepted
- Date: 2026-09-08
- Issue: #62 (grilling), research #61

## Context

Phase 6 (`v0.0.7`) ships the substrate: something you would hand a stranger,
with numbers. This ADR fixes what the benchmark measures, the targets, where the
harnesses live, and how a regression fails CI. Build tickets #64 (the plumbing)
and #65 (the numbers, `docs/benchmark.md`) follow it.

**Mnemosyne makes no accuracy claim.** The 2026 research is settled on this:
GitOfThoughts (arXiv 2606.14470) tested five memory backends (none, markdown,
vector, graph, git) and found none reliably moves an agent's accuracy. Its
conclusion is ours: the value of version control is "auditability, history, and
the ability to merge two agents' memories, at no cost to accuracy". So the
benchmark measures the **operational properties** and the **overhead**. Accuracy
parity is cited, not re-tested; that needs models and an API budget, and it has
been done.

Nothing here changes `format_version`.

## Decision

### What is measured

**Correctness and precision** (deterministic, so the target is exact):

| Metric | Definition | Target |
| --- | --- | --- |
| `reconstruction_exact` | `state_at(commit)` equals the working memory recorded at commit time, over every commit of every seed | `100.00%` |
| `golden_bytes_stable` | the frozen format vectors still hash identically (reuses the `golden_vectors` assertion) | `true` |
| `bisect_exact` | `bisect` returns the exact commit `k` where a monotonic planted fault began | `100.00%` |
| `bisect_error_max` | the largest `\|found - k\|` seen | `0` |
| `blame_commit_acc` | `blame` resolves to the correct introducing commit, linear history | `100.00%` |
| `blame_commit_acc_merge` | correct origin commit when the value arrived on the merged-in side | `100.00%` |
| `blame_source_acc` | correct provenance `source` string | `100.00%` |
| `merge_invariants` | the #49 chaos invariants (totality, no lost writes, symmetry, convergence) at the bench trial count | `true` |

Every one of these gates the release. A single failing seed is a bug and blocks
`v0.0.7`.

**Overhead** (reported, gated only against regression):

| Metric | Definition |
| --- | --- |
| `write_ms_p50`, `write_ms_p99` | latency of one commit, a 100-key memory |
| `read_ms_p50` | latency of a full working-memory read |
| `bytes_per_step` | `.mnem` growth per commit, and the multiple over baseline B |

### The workload

A seeded generator, hand-rolled (a linear congruential generator, the style of
`tests/time_travel.rs` and `tests/merge_chaos.rs`; no property-test dependency):

- a **run** is `L` commits; each stages 1 to 4 operations over a **12-key pool**,
  **75% set / 25% delete**;
- every set carries provenance: `source = "obs-<n>"`, `agent_step = "step-<c>"`;
- values are drawn from a small pool, so content-addressed dedup shows up in
  `bytes_per_step`;
- a **planted fault**: at commit `k` (at 10%, 50% and 90% of the run) one
  distinguished key is set to a "wrong" value and left wrong to the end, so the
  `bisect` predicate `key == wrong_value` is genuinely monotonic;
- a **branched run** for `blame_commit_acc_merge`: two branches off a base, the
  blamed key's final value written on the merged-in side, single merge base (no
  criss-cross, ADR-0013).

### The baselines

"No version control" is two things an agent author would otherwise write:

- **Baseline A, a dict.** `dict[str, Any]`, overwritten in place. GitOfThoughts's
  "none". Zero history.
- **Baseline B, JSONL snapshots.** After every step, the whole memory appended as
  one JSON line. A naive "keep history" approach.

The comparison is an **audit-query table**, asserted in CI: for each query, which
backend can answer it.

| Query | dict | JSONL | Mnemosyne |
| --- | --- | --- | --- |
| current memory | yes | yes | yes |
| memory as of step `t` | no | yes | yes |
| when did key `X` first become value `V` | no | yes, O(L) | yes, O(log L) |
| which observation set `X`'s current value | no | no | yes |
| what did step `t` change | no | yes | yes |
| merge two agents' memories, surfacing conflicts | no | no | yes |

`benchmarks/overhead.py` runs each query against each backend and asserts this
matrix, so a broken `bisect` or `blame` fails the benchmark, not just a test.

### Where it lives

- **`crates/mnem-core/tests/benchmark.rs`**: the correctness and precision
  harness. Runs with `cargo test` at a small trial count; `MNEM_BENCH_RUNS` /
  `MNEM_BENCH_LENGTHS` env overrides drive the published sweep.
- **`benchmarks/overhead.py`**: a new top-level directory (mirroring
  `examples/`). Builds a dict, a JSONL file and a `.mnem` store over the same
  synthetic run, times the operations with `time.perf_counter`, measures sizes,
  asserts the audit-query table, and checks the 2x gate.
- **`benchmarks/baseline.json`**: `{ "write_ms_p50": ..., "bytes_per_step": ... }`.
  The regression gate reads this; `docs/benchmark.md` is the prose. Updated
  deliberately, in its own commit, when a change to the numbers is intended.
- **`docs/benchmark.md`**: written by #65 from a full sweep, in the shape of
  `docs/chaos-report.md`: a one-line summary (the audit-query matrix), what was
  measured, the results table, and the exact reproduction command. Re-run and
  updated whenever the core changes.

Timing uses `std::time::Instant` and `time.perf_counter`; no new dependency,
consistent with the project dropping `proptest` and `time` for hand-rolled
equivalents.

### CI

- The `rust` job already runs `benchmark.rs` (it is a test) at the small trial
  count: `L` in {16, 64}, 40 seeds.
- A new **`benchmark`** job: Python plus the SDK, runs `benchmarks/overhead.py`,
  asserts the audit-query table, and fails if `write_ms_p50` or `bytes_per_step`
  exceeds **2x** the value in `benchmarks/baseline.json`. `read_ms_p50` is
  reported, not gated (it is dominated by content decode).

### The published sweep

`docs/benchmark.md` reports:

- correctness: `L` in {16, 64, 256, 1024}, **2000 seeds** (a few minutes, like
  the 50k-case merge-chaos sweep);
- overhead: `L = 1024`, 100 keys, the median of 5 runs.

### The prolly-tree trigger

ADR-0012 and ADR-0013 defer the prolly-tree `State` form with a "reassess when a
real store hits a ceiling" trigger. The benchmark now produces the number, so
the trigger gets a soft edge: **the flat `State` is reconsidered when
`read_ms_p50` for a full working-memory read exceeds roughly 25 ms, or a real
store's `.mnem` exceeds roughly 50 MB.** This is a prompt to revisit, not an
obligation to rewrite.

## Consequences

- The correctness metrics are a release gate that runs on every push, so
  "time travel is exact", "bisect is precise" and "blame is accurate" stop being
  claims and become CI.
- The overhead numbers are checked in and cannot rot silently; a real regression
  (a per-node fsync, an accidental O(L^2)) fails the `benchmark` job.
- `docs/benchmark.md` is the artefact for the launch: honest, reproducible, and
  it says plainly what version control does and does not buy.
- The `bytes_per_step` and `read_ms_p50` numbers are the evidence for the
  prolly-tree decision, without this ADR making it.
- Two more files to keep current on a core change (`benchmark.rs` is automatic;
  `baseline.json` and `docs/benchmark.md` need a deliberate re-run).

## Alternatives considered

**Benchmark accuracy with real models.** Rejected. It needs an API budget, it is
not a regression gate (too slow, too noisy), and GitOfThoughts already showed no
memory backend reliably helps. Mnemosyne is a substrate, not a retrieval
strategy.

**A `criterion` bench harness.** Rejected. `criterion` is a real dependency and
`cargo bench` is not in CI. A hand-rolled seeded harness in `tests/` is the
project's established pattern and runs on every push for free.

**Gate the overhead on a fixed absolute** (`write_ms_p50 < 10`). Rejected. CI
runners vary; a fixed ceiling either flaps or is set so loose it catches
nothing. A multiple of the last published number tracks the real trend.

**A third baseline (a vector store, SQLite).** Rejected. A vector store is a
retrieval axis this benchmark does not measure; SQLite is JSONL with indices,
the same storage story. Two baselines make the point.

**Publish `blame_commit_acc_merge` below 100% with a caveat.** Rejected. The
generator's merges are single-base, so blame's contributing-parent walk is
well-defined and exact. A failure there is a bug, and it blocks the release like
any other correctness miss.
