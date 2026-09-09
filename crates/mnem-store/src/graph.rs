//! Commit-graph queries: ancestry and the merge base (issue #45).
//!
//! A commit's `parents` are the only edges. Histories are short and `State` is
//! small, so these are plain breadth-first walks with a visited set; no
//! generation numbers or bloom filters.

use std::collections::{BTreeSet, HashSet, VecDeque};

use crate::error::{MnemError, Result};
use crate::id::ObjectId;
use crate::object::Object;
use crate::objects;
use crate::store::Store;

impl Store {
    /// Every commit reachable from `start`, `start` included.
    pub fn ancestors(&self, start: ObjectId) -> Result<BTreeSet<ObjectId>> {
        let txn = self.begin_read()?;
        let mut seen: BTreeSet<ObjectId> = BTreeSet::new();
        let mut queue: VecDeque<ObjectId> = VecDeque::from([start]);
        while let Some(id) = queue.pop_front() {
            if !seen.insert(id) {
                continue;
            }
            let Object::Commit(commit) = objects::require(&txn, id)? else {
                return Err(MnemError::CorruptStore(format!("{id} is not a commit")));
            };
            for parent in commit.parents {
                if !seen.contains(&parent) {
                    queue.push_back(parent);
                }
            }
        }
        Ok(seen)
    }

    /// Whether `ancestor` is `descendant` or one of its ancestors.
    pub fn is_ancestor(&self, ancestor: ObjectId, descendant: ObjectId) -> Result<bool> {
        if ancestor == descendant {
            return Ok(true);
        }
        let txn = self.begin_read()?;
        let mut seen: HashSet<ObjectId> = HashSet::new();
        let mut queue: VecDeque<ObjectId> = VecDeque::from([descendant]);
        while let Some(id) = queue.pop_front() {
            if !seen.insert(id) {
                continue;
            }
            let Object::Commit(commit) = objects::require(&txn, id)? else {
                return Err(MnemError::CorruptStore(format!("{id} is not a commit")));
            };
            for parent in commit.parents {
                if parent == ancestor {
                    return Ok(true);
                }
                if !seen.contains(&parent) {
                    queue.push_back(parent);
                }
            }
        }
        Ok(false)
    }

    /// The lowest common ancestors of `a` and `b`: common ancestors with no
    /// other common ancestor as a descendant. Usually one; a criss-cross
    /// history can give several. Ordered by commit time (newest first), then id.
    pub fn merge_bases(&self, a: ObjectId, b: ObjectId) -> Result<Vec<ObjectId>> {
        let common: BTreeSet<ObjectId> = self
            .ancestors(a)?
            .intersection(&self.ancestors(b)?)
            .copied()
            .collect();

        // Drop any common ancestor that is a proper ancestor of another common
        // ancestor: the nearer one dominates it.
        let mut lcas = common.clone();
        for &c in &common {
            for anc in self.ancestors(c)? {
                if anc != c {
                    lcas.remove(&anc);
                }
            }
        }

        let mut out: Vec<ObjectId> = lcas.into_iter().collect();
        let txn = self.begin_read()?;
        let time_of = |id: ObjectId| -> Result<i64> {
            match objects::require(&txn, id)? {
                Object::Commit(commit) => Ok(commit.time),
                _ => Err(MnemError::CorruptStore(format!("{id} is not a commit"))),
            }
        };
        let mut keyed: Vec<(i64, ObjectId)> = Vec::with_capacity(out.len());
        for id in out.drain(..) {
            keyed.push((time_of(id)?, id));
        }
        keyed.sort_by(|x, y| {
            y.0.cmp(&x.0)
                .then_with(|| y.1.as_bytes().cmp(x.1.as_bytes()))
        });
        Ok(keyed.into_iter().map(|(_, id)| id).collect())
    }

    /// The single merge base of `a` and `b`. Errors when there is none
    /// (disjoint histories) or more than one (a criss-cross history):
    /// Phase 3's merge requires an unambiguous base (ADR-0013).
    pub fn merge_base(&self, a: ObjectId, b: ObjectId) -> Result<ObjectId> {
        match self.merge_bases(a, b)?.as_slice() {
            [] => Err(MnemError::InvalidRef(format!(
                "{a} and {b} have no common ancestor"
            ))),
            [only] => Ok(*only),
            many => Err(MnemError::InvalidRef(format!(
                "{a} and {b} have {} merge bases (criss-cross history); merge one side first",
                many.len()
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::codec;
    use crate::object::{Commit, ContentKind, MemoryNode, Provenance, State};

    fn scratch() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let base = std::env::temp_dir().join(format!(
            "mnem-graph-{}-{}-{:?}",
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

    fn node(id: &str) -> MemoryNode {
        MemoryNode {
            id: id.to_string(),
            content: json!("x"),
            content_kind: ContentKind::Note,
            provenance: Provenance::default(),
            event_time: None,
        }
    }

    /// A raw commit with the given parents, so merges can be built before
    /// `Store::merge` exists.
    fn put_merge(store: &Store, parents: Vec<ObjectId>, time: i64) -> ObjectId {
        let txn = store.begin_write().unwrap();
        let state_bytes = codec::encode(&Object::State(State::default()));
        let state_id = ObjectId::hash_canonical(&state_bytes);
        let commit = Object::Commit(Commit {
            parents,
            state: state_id,
            message: format!("merge {time}"),
            author: "test".to_string(),
            time,
        });
        let commit_bytes = codec::encode(&commit);
        let commit_id = ObjectId::hash_canonical(&commit_bytes);
        {
            let mut table = txn.open_table(objects::OBJECTS).unwrap();
            table
                .insert(state_id.as_bytes().as_slice(), state_bytes.as_slice())
                .unwrap();
            table
                .insert(commit_id.as_bytes().as_slice(), commit_bytes.as_slice())
                .unwrap();
        }
        txn.commit().unwrap();
        commit_id
    }

    #[test]
    fn merge_base_of_two_branches_off_one_commit() {
        let root = scratch();
        let store = Store::init(&root).unwrap();
        store.stage(&node("a")).unwrap();
        let base = store.commit("base", "agent", 1_000).unwrap();

        store.branch("feature", None).unwrap();
        store.stage(&node("b")).unwrap();
        let ours = store.commit("ours", "agent", 2_000).unwrap();

        store.checkout("feature", false).unwrap();
        store.stage(&node("c")).unwrap();
        let theirs = store.commit("theirs", "agent", 3_000).unwrap();

        assert_eq!(store.merge_base(ours, theirs).unwrap(), base);
        assert_eq!(store.merge_bases(ours, theirs).unwrap(), vec![base]);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn merge_base_is_the_ancestor_when_one_side_leads() {
        let root = scratch();
        let store = Store::init(&root).unwrap();
        store.stage(&node("a")).unwrap();
        let c1 = store.commit("one", "agent", 1_000).unwrap();
        store.stage(&node("b")).unwrap();
        let c2 = store.commit("two", "agent", 2_000).unwrap();

        assert_eq!(store.merge_base(c1, c2).unwrap(), c1);
        assert!(store.is_ancestor(c1, c2).unwrap());
        assert!(!store.is_ancestor(c2, c1).unwrap());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn criss_cross_history_has_two_merge_bases() {
        let root = scratch();
        let store = Store::init(&root).unwrap();
        store.stage(&node("a")).unwrap();
        let root_c = store.commit("root", "agent", 1_000).unwrap();
        store.branch("side", None).unwrap();

        store.stage(&node("x")).unwrap();
        let main_a = store.commit("main a", "agent", 2_000).unwrap();
        store.checkout("side", false).unwrap();
        store.stage(&node("y")).unwrap();
        let side_a = store.commit("side a", "agent", 3_000).unwrap();

        // each side merges the other: a criss-cross
        let main_m = put_merge(&store, vec![main_a, side_a], 4_000);
        let side_m = put_merge(&store, vec![side_a, main_a], 5_000);

        let bases = store.merge_bases(main_m, side_m).unwrap();
        assert_eq!(bases.len(), 2);
        assert!(bases.contains(&main_a) && bases.contains(&side_a));
        assert!(!bases.contains(&root_c));

        assert!(matches!(
            store.merge_base(main_m, side_m),
            Err(MnemError::InvalidRef(m)) if m.contains("criss-cross")
        ));
        std::fs::remove_dir_all(&root).ok();
    }
}
