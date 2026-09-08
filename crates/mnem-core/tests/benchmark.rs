//! The v1 benchmark: the correctness and precision metrics (issue #64,
//! ADR-0017).
//!
//! Every metric here is deterministic, so the target is exact (100% / true / 0).
//! A single failing seed is a bug and blocks the release. This runs as an
//! ordinary test at a small trial count; `MNEM_BENCH_RUNS` and
//! `MNEM_BENCH_LENGTHS` drive the published sweep, which `docs/benchmark.md`
//! reports.
//!
//! The overhead metrics (latency, `.mnem` growth) are a separate Python harness,
//! `benchmarks/overhead.py`, because the baselines are a dict and a JSONL file.

use std::collections::BTreeMap;

use serde_json::{json, Value};

use mnem_core::merge::merge_state_maps;
use mnem_core::{codec, id::ObjectId, ContentKind, MemoryNode, Object, Provenance, Store};

/// A small linear congruential generator: varied but reproducible, no dependency.
struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(1))
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

fn scratch(tag: &str, seed: u64) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let base = std::env::temp_dir().join(format!(
        "mnem-bench-{tag}-{}-{seed}-{}-{:?}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&base).unwrap();
    base
}

const KEYS: [&str; 12] = [
    "plan", "seats", "owner", "renewal", "region", "tier", "status", "contact", "sla", "quota",
    "billing", "notes",
];
const VALUES: [&str; 6] = ["alpha", "beta", "gamma", "delta", "epsilon", "zeta"];

fn runs() -> u64 {
    env_u64("MNEM_BENCH_RUNS", 6)
}

fn lengths() -> Vec<usize> {
    match std::env::var("MNEM_BENCH_LENGTHS") {
        Ok(s) => s.split(',').filter_map(|p| p.trim().parse().ok()).collect(),
        Err(_) => vec![16, 48],
    }
}

fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn node(id: &str, value: &str, source: &str, step: &str) -> MemoryNode {
    MemoryNode {
        id: id.to_string(),
        content: Value::String(value.to_string()),
        content_kind: ContentKind::Note,
        provenance: Provenance {
            agent_step: Some(step.to_string()),
            observation: Some(format!("in-{source}")),
            source: Some(source.to_string()),
            ..Default::default()
        },
        event_time: None,
    }
}

/// One synthetic run. Returns the commit ids in order, the working memory
/// recorded right after each commit, and, per key still live at the end, the
/// commit and `source` that last set it.
struct Run {
    commits: Vec<ObjectId>,
    snapshots: Vec<BTreeMap<String, MemoryNode>>,
    set_at: BTreeMap<String, (ObjectId, String)>,
    planted_key: String,
    planted_at: usize,
}

fn generate(store: &Store, rng: &mut Lcg, length: usize, fault_fraction: f64) -> Run {
    let planted_key = KEYS[rng.below(KEYS.len() as u64) as usize].to_string();
    let planted_at = ((length as f64) * fault_fraction) as usize;
    let planted_at = planted_at.clamp(1, length.saturating_sub(1));

    let mut commits = Vec::with_capacity(length);
    let mut snapshots = Vec::with_capacity(length);
    let mut set_at: BTreeMap<String, (ObjectId, String)> = BTreeMap::new();
    let mut obs = 0u64;

    for c in 0..length {
        // the planted fault: set the key to a distinguished wrong value, forever
        if c == planted_at {
            obs += 1;
            let src = format!("obs-{obs}");
            store
                .stage(&node(&planted_key, "WRONG", &src, &format!("step-{c}")))
                .unwrap();
            let commit = commit_now(store, c);
            set_at.insert(planted_key.clone(), (commit, src));
            commits.push(commit);
            snapshots.push(store.working_memory().unwrap());
            continue;
        }

        let ops = 1 + rng.below(4);
        for _ in 0..ops {
            let key = KEYS[rng.below(KEYS.len() as u64) as usize];
            if key == planted_key && c > planted_at {
                continue; // never disturb the planted key after it goes wrong
            }
            if rng.below(4) == 0 {
                let _ = store.rm(key);
                set_at.remove(key);
            } else {
                obs += 1;
                let src = format!("obs-{obs}");
                let value = VALUES[rng.below(VALUES.len() as u64) as usize];
                store
                    .stage(&node(key, value, &src, &format!("step-{c}")))
                    .unwrap();
                set_at.insert(key.to_string(), (ObjectId::from_bytes([0; 32]), src));
            }
        }
        let commit = commit_now(store, c);
        // fill in the real commit id for keys set this commit
        for (_, entry) in set_at.iter_mut() {
            if entry.0 == ObjectId::from_bytes([0; 32]) {
                entry.0 = commit;
            }
        }
        commits.push(commit);
        snapshots.push(store.working_memory().unwrap());
    }

    Run {
        commits,
        snapshots,
        set_at,
        planted_key,
        planted_at,
    }
}

fn commit_now(store: &Store, c: usize) -> ObjectId {
    store
        .commit(&format!("step {c}"), "bench", 1_000 + c as i64)
        .unwrap_or_else(|_| {
            // a commit with nothing staged (all ops were failed rm): reuse HEAD
            store.head_commit().unwrap().unwrap()
        })
}

struct Tally {
    hits: u64,
    total: u64,
}

impl Tally {
    fn new() -> Self {
        Self { hits: 0, total: 0 }
    }
    fn record(&mut self, ok: bool) {
        self.total += 1;
        self.hits += u64::from(ok);
    }
    fn pct(&self) -> f64 {
        if self.total == 0 {
            100.0
        } else {
            100.0 * self.hits as f64 / self.total as f64
        }
    }
}

#[test]
fn the_v1_benchmark_correctness_metrics_hold() {
    let n = runs();
    let ls = lengths();

    let mut reconstruction = Tally::new();
    let mut bisect_exact = Tally::new();
    let mut bisect_error_max = 0usize;
    let mut blame_commit = Tally::new();
    let mut blame_source = Tally::new();
    let mut blame_commit_merge = Tally::new();

    for seed in 0..n {
        for &length in &ls {
            for fault in [0.1_f64, 0.5, 0.9] {
                let root = scratch("run", seed);
                let store = Store::init(&root).unwrap();
                let mut rng = Lcg::new(seed ^ ((length as u64) << 8) ^ (fault * 1000.0) as u64);
                let run = generate(&store, &mut rng, length, fault);

                // reconstruction: state_at(c) == the working memory recorded then
                for (i, commit) in run.commits.iter().enumerate() {
                    reconstruction.record(store.state_at(*commit).unwrap() == run.snapshots[i]);
                }

                // bisect: find the exact commit where the planted fault began
                let key = run.planted_key.clone();
                let found = store
                    .bisect(None, None, move |mem: &BTreeMap<String, MemoryNode>| {
                        mem.get(&key).map(|node| &node.content) == Some(&json!("WRONG"))
                    })
                    .unwrap();
                let want = run.commits[run.planted_at];
                bisect_exact.record(found == want);
                if found != want {
                    let fi = run.commits.iter().position(|c| *c == found).unwrap_or(0);
                    bisect_error_max = bisect_error_max.max(fi.abs_diff(run.planted_at));
                }

                // blame: every live key resolves to the commit and source that set it
                let final_memory = run.snapshots.last().cloned().unwrap_or_default();
                for key in final_memory.keys() {
                    let blame = store.blame(key, None).unwrap();
                    if let Some((commit, source)) = run.set_at.get(key) {
                        blame_commit.record(blame.commit == *commit);
                        blame_source.record(
                            blame.node.provenance.source.as_deref() == Some(source.as_str()),
                        );
                    }
                }

                std::fs::remove_dir_all(&root).ok();
            }

            // blame through a merge: the blamed key's value comes from theirs
            let root = scratch("merge", seed);
            let store = Store::init(&root).unwrap();
            let mut rng = Lcg::new(seed ^ 0xB1A3E);
            store.stage(&node("base", "x", "obs-0", "step-0")).unwrap();
            store.commit("base", "bench", 1).unwrap();
            store.branch("feature", None).unwrap();

            store
                .stage(&node("owner", VALUES[rng.below(6) as usize], "obs-o", "s"))
                .unwrap();
            store.commit("ours", "bench", 2).unwrap();

            store.checkout("feature", false).unwrap();
            store
                .stage(&node("plan", "settled", "obs-theirs", "s"))
                .unwrap();
            let theirs = store.commit("theirs", "bench", 3).unwrap();

            store.checkout("main", false).unwrap();
            let outcome = store
                .merge("feature", &BTreeMap::new(), None, None, "bench", 4)
                .unwrap();
            assert!(
                matches!(outcome, mnem_core::MergeOutcome::Merged(_)),
                "seed {seed}: expected a clean merge, got {outcome:?}"
            );
            let blame = store.blame("plan", None).unwrap();
            blame_commit_merge.record(blame.commit == theirs);
            std::fs::remove_dir_all(&root).ok();
        }
    }

    // condensed merge invariants (the full sweep is tests/merge_chaos.rs)
    let merge_ok = merge_invariants_hold(n.max(200));

    // golden bytes: one canonical object still hashes to its pinned value
    let golden_ok = golden_bytes_stable();

    println!("== mnemosyne benchmark (correctness) ==");
    println!("runs={n} lengths={ls:?}");
    println!("reconstruction_exact    {:.2}%", reconstruction.pct());
    println!("golden_bytes_stable     {golden_ok}");
    println!("bisect_exact            {:.2}%", bisect_exact.pct());
    println!("bisect_error_max        {bisect_error_max}");
    println!("blame_commit_acc        {:.2}%", blame_commit.pct());
    println!("blame_commit_acc_merge  {:.2}%", blame_commit_merge.pct());
    println!("blame_source_acc        {:.2}%", blame_source.pct());
    println!("merge_invariants        {merge_ok}");

    assert_eq!(
        reconstruction.pct(),
        100.0,
        "reconstruction_exact regressed"
    );
    assert!(golden_ok, "golden_bytes_stable regressed");
    assert_eq!(bisect_exact.pct(), 100.0, "bisect_exact regressed");
    assert_eq!(bisect_error_max, 0, "bisect_error_max regressed");
    assert_eq!(blame_commit.pct(), 100.0, "blame_commit_acc regressed");
    assert_eq!(
        blame_commit_merge.pct(),
        100.0,
        "blame_commit_acc_merge regressed"
    );
    assert_eq!(blame_source.pct(), 100.0, "blame_source_acc regressed");
    assert!(merge_ok, "merge_invariants regressed");
}

fn merge_invariants_hold(iterations: u64) -> bool {
    fn oid(n: u64) -> ObjectId {
        ObjectId::from_bytes([(n & 0xFF) as u8; 32])
    }
    fn map(rng: &mut Lcg) -> BTreeMap<String, ObjectId> {
        let mut m = BTreeMap::new();
        for k in KEYS {
            if rng.below(3) != 0 {
                m.insert(k.to_string(), oid(1 + rng.below(4)));
            }
        }
        m
    }
    for seed in 0..iterations {
        let mut rng = Lcg::new(seed ^ 0x0DE);
        let base = map(&mut rng);
        let ours = map(&mut rng);
        let theirs = map(&mut rng);
        let forward = merge_state_maps(&base, &ours, &theirs);
        let backward = merge_state_maps(&base, &theirs, &ours);

        // symmetry: swapping ours/theirs gives the identical merged map
        if forward.merged != backward.merged {
            return false;
        }
        let all: std::collections::BTreeSet<&String> = base
            .keys()
            .chain(ours.keys())
            .chain(theirs.keys())
            .collect();
        let conflict_ids: std::collections::BTreeSet<&String> =
            forward.conflicts.iter().map(|c| &c.id).collect();
        for id in forward.merged.keys().chain(conflict_ids.iter().copied()) {
            // no invented ids
            if !all.contains(id) {
                return false;
            }
        }
        for id in &conflict_ids {
            // no id is both resolved and a conflict
            if forward.merged.contains_key(*id) {
                return false;
            }
            // a conflicting id is genuinely divergent on all three sides
            let (b, o, t) = (base.get(*id), ours.get(*id), theirs.get(*id));
            if o == t || b == o || b == t {
                return false;
            }
        }
    }
    true
}

fn golden_bytes_stable() -> bool {
    let obj = Object::MemoryNode(MemoryNode {
        id: "canonical".to_string(),
        content: json!({ "plan": "Enterprise", "seats": 40 }),
        content_kind: ContentKind::Note,
        provenance: Provenance {
            source: Some("ticket-1".to_string()),
            ..Default::default()
        },
        event_time: Some(1_757_000_000_000),
    });
    let id = ObjectId::hash_canonical(&codec::encode(&obj));
    let stable = id.to_hex() == PINNED_CANONICAL_HASH;
    if !stable {
        eprintln!("canonical hash is now {}", id.to_hex());
    }
    stable
}

/// The BLAKE3 hash of the canonical CBOR of one fixed object. If the encoder or
/// the object model drifts, this changes and the golden vectors in
/// `docs/format/` are stale (`tests/golden_vectors.rs` is the full check).
const PINNED_CANONICAL_HASH: &str =
    "a197a6c57aa46723469daf377bcc2336b42ac95f155dda456f5fa36f4b98bb11";
