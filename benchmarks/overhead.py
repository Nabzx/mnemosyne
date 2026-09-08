"""The v1 benchmark: overhead against a dict and a JSONL baseline (issue #64,
ADR-0017).

"No version control" is two things an agent author would otherwise write: a
dict they overwrite, and a JSONL file they append the whole memory to each step.
This measures what Mnemosyne costs over each (write latency, read latency,
`.mnem` growth) and asserts the audit-query table: which backend can answer
"when did X go wrong", "what set X", "memory as of step t", "merge two agents".

    python benchmarks/overhead.py

CI runs it at a small step count and fails if write latency or store growth
exceeds 2x `benchmarks/baseline.json`. The published numbers come from #65.
"""

from __future__ import annotations

import json
import os
import statistics
import sys
import tempfile
import time
from pathlib import Path

import mnem

HERE = Path(__file__).parent
BASELINE = HERE / "baseline.json"

KEYS = [f"k{i}" for i in range(100)]
VALUES = [f"v{i}" for i in range(20)]
STEPS = int(os.environ.get("MNEM_BENCH_STEPS", "256"))
REGRESSION_FACTOR = 2.0


class Lcg:
    """A tiny reproducible generator, matching the Rust harness's approach."""

    def __init__(self, seed: int) -> None:
        self.state = (seed * 0x9E3779B97F4A7C15 + 1) & 0xFFFFFFFFFFFFFFFF

    def below(self, n: int) -> int:
        self.state = (self.state * 6364136223846793005 + 1442695040888963407) & 0xFFFFFFFFFFFFFFFF
        return (self.state >> 33) % n


def workload(steps: int) -> list[list[tuple[str, str | None]]]:
    """`steps` commits, each a list of (key, value) ops; value None is a delete.
    Deterministic. One key ("k7") goes wrong at the midpoint and stays wrong."""
    rng = Lcg(1)
    fault_at = steps // 2
    plan: list[list[tuple[str, str | None]]] = []
    for step in range(steps):
        if step == fault_at:
            plan.append([("k7", "WRONG")])
            continue
        ops: list[tuple[str, str | None]] = []
        for _ in range(1 + rng.below(4)):
            key = KEYS[rng.below(len(KEYS))]
            if key == "k7" and step > fault_at:
                continue
            if rng.below(4) == 0:
                ops.append((key, None))
            else:
                ops.append((key, VALUES[rng.below(len(VALUES))]))
        plan.append(ops or [(KEYS[0], VALUES[0])])
    return plan


def _apply(memory: dict[str, str], ops: list[tuple[str, str | None]]) -> None:
    for key, value in ops:
        if value is None:
            memory.pop(key, None)
        else:
            memory[key] = value


# --- the three backends -------------------------------------------------


def run_dict(plan: list) -> dict:
    memory: dict[str, str] = {}
    writes = []
    for ops in plan:
        start = time.perf_counter()
        _apply(memory, ops)
        writes.append((time.perf_counter() - start) * 1000)
    start = time.perf_counter()
    _ = dict(memory)
    read_ms = (time.perf_counter() - start) * 1000
    return {"writes": writes, "read_ms": read_ms, "bytes": 0}


def run_jsonl(plan: list, path: Path) -> dict:
    memory: dict[str, str] = {}
    writes = []
    with path.open("w") as handle:
        for ops in plan:
            start = time.perf_counter()
            _apply(memory, ops)
            handle.write(json.dumps(memory) + "\n")
            handle.flush()
            os.fsync(handle.fileno())
            writes.append((time.perf_counter() - start) * 1000)
    start = time.perf_counter()
    with path.open() as handle:
        _ = json.loads(handle.readlines()[-1])
    read_ms = (time.perf_counter() - start) * 1000
    return {"writes": writes, "read_ms": read_ms, "bytes": path.stat().st_size}


def run_mnem(plan: list, store_dir: Path) -> dict:
    store = mnem.init(store_dir)
    writes = []
    obs = 0
    fault_commit = None
    commits: list[str] = []
    for step, ops in enumerate(plan):
        start = time.perf_counter()
        staged = False
        for key, value in ops:
            if value is None:
                try:
                    store.rm(key)
                    staged = True
                except mnem.InvalidRefError:
                    pass
            else:
                obs += 1
                store.add(key, value, provenance=mnem.Provenance(source=f"obs-{obs}",
                                                                 agent_step=f"step-{step}"))
                staged = True
        if staged:
            commit = store.commit(f"step {step}", author="bench", time_ms=1000 + step)
            commits.append(commit)
            if ops == [("k7", "WRONG")]:
                fault_commit = commit
        writes.append((time.perf_counter() - start) * 1000)

    start = time.perf_counter()
    _ = store.working_memory()
    read_ms = (time.perf_counter() - start) * 1000
    return {
        "writes": writes,
        "read_ms": read_ms,
        "bytes": _dir_size(store.root / ".mnem"),
        "store": store,
        "commits": commits,
        "fault_commit": fault_commit,
    }


def _dir_size(path: Path) -> int:
    return sum(f.stat().st_size for f in path.rglob("*") if f.is_file())


# --- the audit-query table -------------------------------------------


def audit_table(plan: list, mnem_result: dict) -> dict[str, dict[str, bool]]:
    store = mnem_result["store"]
    commits = mnem_result["commits"]
    fault_commit = mnem_result["fault_commit"]
    a_past_commit = commits[len(commits) // 2]

    queries: dict[str, dict[str, bool]] = {}

    queries["current memory"] = {"dict": True, "jsonl": True, "mnem": True}

    # memory as of a past commit
    mnem_ok = isinstance(store.state_at(a_past_commit), dict)
    queries["memory as of step t"] = {"dict": False, "jsonl": True, "mnem": mnem_ok}

    # when did k7 first become "WRONG"
    found = store.bisect(lambda s: "k7" in s and s["k7"].content == "WRONG")
    mnem_ok = found == fault_commit
    queries["when did key X become value V"] = {"dict": False, "jsonl": True, "mnem": mnem_ok}

    # which observation set k7's current value
    blame = store.blame("k7")
    mnem_ok = blame.commit == fault_commit and blame.provenance.source is not None
    queries["which observation set key X"] = {"dict": False, "jsonl": False, "mnem": mnem_ok}

    # what did a step change
    mnem_ok = isinstance(store.changed_by(a_past_commit), dict)
    queries["what did step t change"] = {"dict": False, "jsonl": True, "mnem": mnem_ok}

    # merge two agents' memories with conflicts surfaced
    store.new_branch("other")
    store.checkout("other")
    store.add("k0", "from-other")
    store.commit("other work", author="bench", time_ms=99999)
    store.checkout("main")
    result = store.merge("other")
    mnem_ok = result.status in {"merged", "fast-forwarded", "conflicts"}
    queries["merge two agents, surfacing conflicts"] = {
        "dict": False, "jsonl": False, "mnem": mnem_ok,
    }

    return queries


EXPECTED_TABLE = {
    "current memory": {"dict": True, "jsonl": True, "mnem": True},
    "memory as of step t": {"dict": False, "jsonl": True, "mnem": True},
    "when did key X become value V": {"dict": False, "jsonl": True, "mnem": True},
    "which observation set key X": {"dict": False, "jsonl": False, "mnem": True},
    "what did step t change": {"dict": False, "jsonl": True, "mnem": True},
    "merge two agents, surfacing conflicts": {"dict": False, "jsonl": False, "mnem": True},
}


def pct(values: list[float], p: float) -> float:
    ordered = sorted(values)
    idx = min(len(ordered) - 1, int(p * len(ordered)))
    return ordered[idx]


def main() -> int:
    plan = workload(STEPS)

    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)
        d = run_dict(plan)
        j = run_jsonl(plan, tmp_path / "memory.jsonl")
        m = run_mnem(plan, tmp_path / "store")
        table = audit_table(plan, m)

    write_p50 = statistics.median(m["writes"])
    write_p99 = pct(m["writes"], 0.99)
    bytes_per_step = m["bytes"] / STEPS
    jsonl_bytes_per_step = j["bytes"] / STEPS

    print("== mnemosyne benchmark (overhead) ==")
    print(f"steps={STEPS} keys={len(KEYS)}")
    print(f"{'backend':<10} {'write_p50_ms':>12} {'read_ms':>10} {'bytes/step':>12}")
    for name, r, size in (
        ("dict", d, 0.0),
        ("jsonl", j, jsonl_bytes_per_step),
        ("mnem", m, bytes_per_step),
    ):
        w = statistics.median(r["writes"])
        print(f"{name:<10} {w:>12.3f} {r['read_ms']:>10.3f} {size:>12.1f}")
    print(f"\nmnem write_p99_ms  {write_p99:.3f}")
    print(f"mnem is {bytes_per_step / jsonl_bytes_per_step:.2f}x JSONL per step "
          f"(content-addressed)")

    print("\naudit queries (can the backend answer it?):")
    for query, answers in table.items():
        marks = "  ".join(f"{b}:{'y' if answers[b] else 'n'}" for b in ("dict", "jsonl", "mnem"))
        print(f"  {query:<40} {marks}")

    # the audit table must match
    assert table == EXPECTED_TABLE, f"audit table changed:\n{json.dumps(table, indent=2)}"

    # the regression gate
    baseline = json.loads(BASELINE.read_text())
    ok = True
    for metric, measured in (("write_ms_p50", write_p50), ("bytes_per_step", bytes_per_step)):
        limit = baseline[metric] * REGRESSION_FACTOR
        status = "ok" if measured <= limit else "REGRESSED"
        if measured > limit:
            ok = False
        print(f"gate {metric}: {measured:.3f} vs {limit:.3f} ({baseline[metric]:.3f} x "
              f"{REGRESSION_FACTOR})  {status}")

    print("\nOK" if ok else "\nFAILED")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
