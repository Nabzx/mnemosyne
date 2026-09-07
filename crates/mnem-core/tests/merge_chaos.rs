//! Phase 3 definition of done (#49): the merge algebra holds under chaos.
//!
//! Two harnesses, both seeded so a failure reproduces exactly (the approach of
//! `tests/time_travel.rs`, #41):
//!
//! 1. [`merge_state_maps_algebra`] hammers the pure function
//!    ([`mnem_core::merge::merge_state_maps`]) with random base / ours / theirs
//!    maps and checks the invariants from `docs/research/issue-42-merge-survey.md`
//!    section 4: totality, no lost writes, clean-merge symmetry, idempotence and
//!    base identity.
//! 2. [`store_merge_converges`] drives the real [`mnem_core::Store::merge`] flow
//!    over random two-branch histories and checks that an `Ours`-strategy merge
//!    never conflicts, lands the state the pure function predicts, and that
//!    merging back the other way converges.
//!
//! CI runs these at the trial counts below. `docs/chaos-report.md` records a
//! larger manual sweep.

use std::collections::{BTreeMap, BTreeSet};

use mnem_core::merge::merge_state_maps;
use mnem_core::{
    ContentKind, MemoryNode, MergeOutcome, MergeStrategy, ObjectId, Provenance, Store,
};

/// Trials for the pure-function harness. Cheap: each is a handful of map walks.
/// Override with `MNEM_CHAOS_ALGEBRA_TRIALS` for a wider manual sweep.
const ALGEBRA_TRIALS: u64 = 600;
/// Trials for the store-flow harness. Each builds a store and writes commits, so
/// this is the slow one; kept small for CI. Override with
/// `MNEM_CHAOS_STORE_TRIALS` for the sweep recorded in `docs/chaos-report.md`.
const STORE_TRIALS: u64 = 24;

fn trials(var: &str, default: u64) -> u64 {
    std::env::var(var)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

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

fn scratch(seed: u64) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let base = std::env::temp_dir().join(format!(
        "mnem-merge-chaos-{}-{seed}-{}-{:?}",
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

const KEYS: [&str; 6] = ["alpha", "beta", "gamma", "delta", "epsilon", "zeta"];

// --- 1. the pure function ---------------------------------------------------

/// A handful of distinct object ids to draw from, so convergent edits (both
/// sides land on the same id) happen often enough to matter.
fn oid(n: u64) -> ObjectId {
    ObjectId::from_bytes([(n & 0xFF) as u8; 32])
}

/// A random map: each key present with ~2/3 probability, value from a small pool.
fn random_map(rng: &mut Lcg) -> BTreeMap<String, ObjectId> {
    let mut m = BTreeMap::new();
    for key in KEYS {
        if rng.below(3) != 0 {
            m.insert(key.to_string(), oid(1 + rng.below(4)));
        }
    }
    m
}

/// Mutate `from` by a few random set / delete ops into a new map.
fn diverge(rng: &mut Lcg, from: &BTreeMap<String, ObjectId>) -> BTreeMap<String, ObjectId> {
    let mut m = from.clone();
    let ops = rng.below(4);
    for _ in 0..ops {
        let key = KEYS[rng.below(KEYS.len() as u64) as usize];
        if rng.below(4) == 0 {
            m.remove(key);
        } else {
            m.insert(key.to_string(), oid(1 + rng.below(4)));
        }
    }
    m
}

#[test]
fn merge_state_maps_algebra() {
    for seed in 0..trials("MNEM_CHAOS_ALGEBRA_TRIALS", ALGEBRA_TRIALS) {
        let mut rng = Lcg::new(seed);
        let base = random_map(&mut rng);
        let ours = diverge(&mut rng, &base);
        let theirs = diverge(&mut rng, &base);

        let result = merge_state_maps(&base, &ours, &theirs);

        let all_ids: BTreeSet<&String> = base
            .keys()
            .chain(ours.keys())
            .chain(theirs.keys())
            .collect();
        let conflict_ids: BTreeSet<&String> = result.conflicts.iter().map(|c| &c.id).collect();

        // Totality + no lost writes: every id is in `merged` xor `conflicts`,
        // never both, and its value is exactly what the per-id rule dictates.
        for id in &all_ids {
            let b = base.get(*id).copied();
            let o = ours.get(*id).copied();
            let t = theirs.get(*id).copied();
            let in_merged = result.merged.get(*id).copied();
            let in_conflict = conflict_ids.contains(*id);

            let expected: Result<Option<ObjectId>, ()> = if o == t {
                Ok(o)
            } else if b == o {
                Ok(t)
            } else if b == t {
                Ok(o)
            } else {
                Err(())
            };

            match expected {
                Ok(want) => {
                    assert!(!in_conflict, "seed {seed}: id {id} resolves yet conflicts");
                    assert_eq!(
                        in_merged, want,
                        "seed {seed}: id {id} merged to the wrong side"
                    );
                }
                Err(()) => {
                    assert!(in_conflict, "seed {seed}: id {id} should conflict");
                    assert_eq!(
                        in_merged, None,
                        "seed {seed}: conflicted id {id} also merged"
                    );
                    let c = result.conflicts.iter().find(|c| &c.id == *id).unwrap();
                    assert_eq!(
                        (c.base, c.ours, c.theirs),
                        (b, o, t),
                        "seed {seed}: conflict {id} carries the wrong sides"
                    );
                }
            }
        }
        // no id is invented
        for id in result.merged.keys().chain(conflict_ids.iter().copied()) {
            assert!(all_ids.contains(id), "seed {seed}: merge invented id {id}");
        }
        // conflicts are in id order and unique
        let ids: Vec<&String> = result.conflicts.iter().map(|c| &c.id).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(
            ids, sorted,
            "seed {seed}: conflicts out of order or duplicated"
        );

        // Clean-merge symmetry: swapping ours/theirs gives the identical merged
        // map (always) and the identical conflict id set (kinds mirror).
        let swapped = merge_state_maps(&base, &theirs, &ours);
        assert_eq!(
            result.merged, swapped.merged,
            "seed {seed}: merged map depends on side order"
        );
        let swapped_ids: BTreeSet<&String> = swapped.conflicts.iter().map(|c| &c.id).collect();
        assert_eq!(
            conflict_ids, swapped_ids,
            "seed {seed}: conflict set depends on side order"
        );
        assert_eq!(result.is_clean(), swapped.is_clean());

        // Idempotence: merging a side with itself is a clean no-op.
        let idem = merge_state_maps(&base, &ours, &ours);
        assert!(idem.is_clean(), "seed {seed}: merge(O, O) conflicts");
        assert_eq!(idem.merged, ours, "seed {seed}: merge(O, O) changed O");

        // Base identity: against the base, the other side wins cleanly.
        let against_base = merge_state_maps(&base, &ours, &base);
        assert!(
            against_base.is_clean(),
            "seed {seed}: merge(O, B) conflicts"
        );
        assert_eq!(against_base.merged, ours, "seed {seed}: merge(O, B) != O");
        let base_against = merge_state_maps(&base, &base, &theirs);
        assert!(
            base_against.is_clean(),
            "seed {seed}: merge(B, T) conflicts"
        );
        assert_eq!(base_against.merged, theirs, "seed {seed}: merge(B, T) != T");
    }
}

// --- 2. the store flow -----------------------------------------------------

fn note(id: &str, value: u64) -> MemoryNode {
    MemoryNode {
        id: id.to_string(),
        content: serde_json::Value::String(format!("v{value}")),
        content_kind: ContentKind::Note,
        provenance: Provenance::default(),
        event_time: None,
    }
}

/// Apply a random sequence of stage / rm ops and commit, on the current branch.
/// Returns whether a commit was actually made (a run of failed `rm`s makes none).
fn commit_random(store: &Store, rng: &mut Lcg, tag: &str, time: i64) -> bool {
    let ops = 1 + rng.below(4);
    for _ in 0..ops {
        let key = KEYS[rng.below(KEYS.len() as u64) as usize];
        if rng.below(4) == 0 {
            let _ = store.rm(key);
        } else {
            store.stage(&note(key, rng.below(4))).unwrap();
        }
    }
    if store.staged().unwrap().is_empty() && store.staged_deletions().unwrap().is_empty() {
        return false;
    }
    store.commit(tag, "agent", time).unwrap();
    true
}

fn state_values(state: &BTreeMap<String, MemoryNode>) -> BTreeMap<String, serde_json::Value> {
    state
        .iter()
        .map(|(k, v)| (k.clone(), v.content.clone()))
        .collect()
}

#[test]
fn store_merge_converges() {
    for seed in 0..trials("MNEM_CHAOS_STORE_TRIALS", STORE_TRIALS) {
        let root = scratch(seed);
        let store = Store::init(&root).unwrap();
        let mut rng = Lcg::new(seed ^ 0xC0FFEE);

        // a base commit, then two divergent branches
        store.stage(&note("alpha", 0)).unwrap();
        store.stage(&note("beta", 0)).unwrap();
        store.commit("base", "agent", 1_000).unwrap();
        store.branch("feature", None).unwrap();

        let ours_moved = commit_random(&store, &mut rng, "ours-1", 2_000)
            | commit_random(&store, &mut rng, "ours-2", 2_100);

        store.checkout("feature", false).unwrap();
        let theirs_moved = commit_random(&store, &mut rng, "theirs-1", 3_000)
            | commit_random(&store, &mut rng, "theirs-2", 3_100);

        store.checkout("main", false).unwrap();

        // An Ours-strategy merge must never surface a conflict.
        let outcome = store
            .merge(
                "feature",
                &BTreeMap::new(),
                Some(MergeStrategy::Ours),
                None,
                "agent",
                4_000,
            )
            .unwrap();
        assert!(
            !matches!(outcome, MergeOutcome::Conflicts(_)),
            "seed {seed}: Ours strategy still conflicted: {outcome:?}"
        );

        if let MergeOutcome::Merged(commit_id) = outcome {
            // The merge commit has both tips as parents, ours first.
            assert!(
                ours_moved && theirs_moved,
                "seed {seed}: two-parent merge without divergence"
            );
            let history = store.log(commit_id, false, Some(1)).unwrap();
            let (_, merge_commit) = &history[0];
            assert_eq!(
                merge_commit.parents.len(),
                2,
                "seed {seed}: merge is not two-parent"
            );
        }

        let main_state = state_values(&store.working_memory().unwrap());

        // Merging back the other way must converge: feature fast-forwards (or is
        // already up to date) onto exactly the same state.
        store.checkout("feature", false).unwrap();
        let back = store
            .merge("main", &BTreeMap::new(), None, None, "agent", 5_000)
            .unwrap();
        assert!(
            matches!(
                back,
                MergeOutcome::FastForwarded(_) | MergeOutcome::AlreadyUpToDate
            ),
            "seed {seed}: merge back did not converge: {back:?}"
        );
        let feature_state = state_values(&store.working_memory().unwrap());
        assert_eq!(
            main_state, feature_state,
            "seed {seed}: branches disagree after merging both ways"
        );

        // Idempotence at the flow level: merging feature into main again is a no-op.
        store.checkout("main", false).unwrap();
        let again = store
            .merge("feature", &BTreeMap::new(), None, None, "agent", 6_000)
            .unwrap();
        assert_eq!(
            again,
            MergeOutcome::AlreadyUpToDate,
            "seed {seed}: re-merging an already-merged branch was not a no-op"
        );

        std::fs::remove_dir_all(&root).ok();
    }
}
