//! Phase 2 definition of done (#41): time travel is exact.
//!
//! Over many pseudo-random histories, `state_at(commit)` must equal the working
//! memory that existed the instant that commit was made. A seeded LCG stands in
//! for a property-test generator, so the run is deterministic and needs no
//! extra dependency.

use std::collections::BTreeMap;

use mnem_core::{ContentKind, MemoryNode, ObjectId, Provenance, Store};

/// A small linear congruential generator. Not for cryptography; for shaping a
/// varied but reproducible sequence of operations.
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
    let base = std::env::temp_dir().join(format!(
        "mnem-time-travel-{}-{seed}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&base).unwrap();
    base
}

fn note(id: &str, content: &str) -> MemoryNode {
    MemoryNode {
        id: id.to_string(),
        content: serde_json::Value::String(content.to_string()),
        content_kind: ContentKind::Note,
        provenance: Provenance::default(),
        event_time: None,
    }
}

#[test]
fn state_at_matches_working_memory_at_commit_time() {
    const KEYS: [&str; 6] = ["alpha", "beta", "gamma", "delta", "epsilon", "zeta"];

    for seed in 0..40u64 {
        let root = scratch(seed);
        let store = Store::init(&root).unwrap();
        let mut rng = Lcg::new(seed);

        let mut expected: Vec<(ObjectId, BTreeMap<String, MemoryNode>)> = Vec::new();

        let commits = 3 + rng.below(8);
        for c in 0..commits {
            let ops = 1 + rng.below(4);
            for _ in 0..ops {
                let key = KEYS[rng.below(KEYS.len() as u64) as usize];
                match rng.below(3) {
                    0 | 1 => {
                        let value = format!("s{seed}-c{c}-{}", rng.next_u64());
                        store.stage(&note(key, &value)).unwrap();
                    }
                    _ => {
                        // may fail when the key is not in memory; that is a
                        // valid no-op for the generator.
                        store.rm(key).ok();
                    }
                }
            }

            let id = store
                .commit(&format!("commit {c}"), "agent", i64::try_from(c).unwrap())
                .unwrap();
            expected.push((id, store.working_memory().unwrap()));
        }

        for (id, snapshot) in &expected {
            assert_eq!(
                &store.state_at(*id).unwrap(),
                snapshot,
                "seed {seed}: state_at({id}) disagrees with the working memory recorded at commit time"
            );
        }

        std::fs::remove_dir_all(&root).ok();
    }
}
