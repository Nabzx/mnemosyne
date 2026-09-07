//! `bisect`: the first commit on a range where a predicate holds (issue #53,
//! ADR-0015).
//!
//! Binary-search the first-parent chain between `good` (predicate false) and
//! `bad` (predicate true). The predicate is any function of the reconstructed
//! memory at a commit. Monotonicity is assumed, as in `git bisect`: false up to
//! a boundary, true from there on. Linear history only — merge commits on the
//! chain are evaluated like any other, second parents are not descended into.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::error::{MnemError, Result};
use crate::id::ObjectId;
use crate::object::MemoryNode;
use crate::store::Store;

/// A [`Store::bisect`] predicate: node `id`'s content equals `value`.
pub fn node_content_is(
    id: impl Into<String>,
    value: Value,
) -> impl Fn(&BTreeMap<String, MemoryNode>) -> bool {
    let id = id.into();
    move |state| state.get(&id).is_some_and(|node| node.content == value)
}

/// A [`Store::bisect`] predicate: node `id` is not in memory.
pub fn node_absent(id: impl Into<String>) -> impl Fn(&BTreeMap<String, MemoryNode>) -> bool {
    let id = id.into();
    move |state| !state.contains_key(&id)
}

/// A [`Store::bisect`] predicate: node `id` is in memory.
pub fn node_present(id: impl Into<String>) -> impl Fn(&BTreeMap<String, MemoryNode>) -> bool {
    let id = id.into();
    move |state| state.contains_key(&id)
}

impl Store {
    /// The first commit between `good` and `bad` (both commit-ishes) where
    /// `predicate` holds.
    ///
    /// `bad` defaults to `HEAD`, `good` to the root of `bad`'s first-parent
    /// chain. `good` must be on that chain, `predicate` must be false at `good`
    /// and true at `bad`. The predicate is assumed monotonic between them.
    pub fn bisect(
        &self,
        bad: Option<&str>,
        good: Option<&str>,
        predicate: impl Fn(&BTreeMap<String, MemoryNode>) -> bool,
    ) -> Result<ObjectId> {
        let bad_id = match bad {
            Some(spec) => self.resolve_commitish(spec)?,
            None => self
                .head_commit()?
                .ok_or_else(|| MnemError::InvalidRef("cannot bisect: HEAD has no commit".into()))?,
        };

        // newest-first first-parent chain from `bad`
        let chain: Vec<ObjectId> = self
            .log(bad_id, true, None)?
            .into_iter()
            .map(|(id, _)| id)
            .collect();

        let good_id = match good {
            Some(spec) => self.resolve_commitish(spec)?,
            None => *chain
                .last()
                .expect("a commit has at least itself in its log"),
        };

        if good_id == bad_id {
            return Err(MnemError::InvalidRef(
                "bisect: good and bad are the same commit".into(),
            ));
        }
        let Some(good_pos) = chain.iter().position(|id| *id == good_id) else {
            return Err(MnemError::InvalidRef(format!(
                "bisect: {good_id} is not on {bad_id}'s first-parent history"
            )));
        };

        if predicate(&self.state_at(good_id)?) {
            return Err(MnemError::InvalidRef(format!(
                "bisect: the predicate is already true at good ({good_id})"
            )));
        }
        if !predicate(&self.state_at(bad_id)?) {
            return Err(MnemError::InvalidRef(format!(
                "bisect: the predicate is false at bad ({bad_id})"
            )));
        }

        // candidates are (good, bad], oldest-first
        let mut candidates: Vec<ObjectId> = chain[..good_pos].to_vec();
        candidates.reverse();

        // lower bound: the first candidate where the predicate holds. The last
        // candidate is `bad`, where it holds, so the search always lands.
        let mut lo = 0usize;
        let mut hi = candidates.len();
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if predicate(&self.state_at(candidates[mid])?) {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }
        Ok(candidates[lo])
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::object::{ContentKind, Provenance};

    fn scratch() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let base = std::env::temp_dir().join(format!(
            "mnem-bisect-{}-{}-{:?}",
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

    fn node(id: &str, content: &str) -> MemoryNode {
        MemoryNode {
            id: id.to_string(),
            content: json!(content),
            content_kind: ContentKind::Note,
            provenance: Provenance::default(),
            event_time: None,
        }
    }

    /// A history of `n` commits; `plan` flips to "pro" at commit index `bad_at`.
    fn history(store: &Store, n: usize, bad_at: usize) -> Vec<ObjectId> {
        let mut ids = Vec::new();
        store.stage(&node("plan", "enterprise")).unwrap();
        store.stage(&node("seats", "10")).unwrap();
        ids.push(store.commit("c0", "agent", 1_000).unwrap());
        for i in 1..n {
            if i == bad_at {
                store.stage(&node("plan", "pro")).unwrap();
            } else {
                store.stage(&node("seats", &format!("{}", 10 + i))).unwrap();
            }
            ids.push(
                store
                    .commit(&format!("c{i}"), "agent", 1_000 + i as i64 * 1_000)
                    .unwrap(),
            );
        }
        ids
    }

    #[test]
    fn bisect_finds_the_flip_commit() {
        let root = scratch();
        let store = Store::init(&root).unwrap();
        let ids = history(&store, 12, 7);

        let found = store
            .bisect(None, None, node_content_is("plan", json!("pro")))
            .unwrap();
        assert_eq!(found, ids[7]);

        // with an explicit good that is still well before the flip
        let found = store
            .bisect(
                Some(&ids[11].to_hex()),
                Some(&ids[2].to_hex()),
                node_content_is("plan", json!("pro")),
            )
            .unwrap();
        assert_eq!(found, ids[7]);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn bisect_on_the_boundary_cases() {
        let root = scratch();
        let store = Store::init(&root).unwrap();
        let ids = history(&store, 6, 1); // flips at the very first step

        // the flip is the second commit
        let found = store
            .bisect(None, None, node_content_is("plan", json!("pro")))
            .unwrap();
        assert_eq!(found, ids[1]);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn bisect_rejects_a_non_monotonic_or_impossible_range() {
        let root = scratch();
        let store = Store::init(&root).unwrap();
        let ids = history(&store, 6, 3);

        // predicate already true at good
        assert!(matches!(
            store.bisect(None, Some(&ids[4].to_hex()), node_content_is("plan", json!("pro"))),
            Err(MnemError::InvalidRef(m)) if m.contains("already true at good")
        ));

        // predicate never true (false at bad)
        assert!(matches!(
            store.bisect(None, None, node_content_is("plan", json!("never"))),
            Err(MnemError::InvalidRef(m)) if m.contains("false at bad")
        ));

        // good == bad
        assert!(matches!(
            store.bisect(Some(&ids[3].to_hex()), Some(&ids[3].to_hex()), node_absent("x")),
            Err(MnemError::InvalidRef(m)) if m.contains("same commit")
        ));

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn bisect_with_an_absence_predicate() {
        let root = scratch();
        let store = Store::init(&root).unwrap();

        store.stage(&node("plan", "enterprise")).unwrap();
        store.stage(&node("temp", "scratch")).unwrap();
        store.commit("c0", "agent", 1_000).unwrap();
        for i in 1..8 {
            store.stage(&node("seats", &format!("{i}"))).unwrap();
            store
                .commit(&format!("c{i}"), "agent", 1_000 + i * 1_000)
                .unwrap();
        }
        store.rm("temp").unwrap();
        let dropped = store.commit("drop temp", "agent", 20_000).unwrap();
        for i in 1..4 {
            store.stage(&node("x", &format!("{i}"))).unwrap();
            store
                .commit(&format!("after{i}"), "agent", 20_000 + i * 1_000)
                .unwrap();
        }

        let found = store.bisect(None, None, node_absent("temp")).unwrap();
        assert_eq!(found, dropped);

        std::fs::remove_dir_all(&root).ok();
    }
}
