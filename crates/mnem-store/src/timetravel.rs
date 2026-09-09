//! Time travel: reading the memory at any past commit (issue #38, ADR-0012).
//!
//! `state_at` reconstructs the full memory as it stood at a commit, without
//! touching `HEAD` and without a branch. It is a point read: load the commit,
//! load its state, load each node. Any commit that exists in the store works;
//! ancestry does not matter.

use std::collections::BTreeMap;

use crate::error::{MnemError, Result};
use crate::id::ObjectId;
use crate::object::{MemoryNode, Object};
use crate::objects;
use crate::store::Store;

impl Store {
    /// The `nodes` map of a commit's state: node id to node `ObjectId`. The raw
    /// form; [`state_at`](Self::state_at) is the decoded one.
    pub fn state_map_at(&self, commit: ObjectId) -> Result<BTreeMap<String, ObjectId>> {
        let txn = self.begin_read()?;
        self.state_map_at_in(&txn, commit)
    }

    pub(crate) fn state_map_at_in(
        &self,
        txn: &redb::ReadTransaction,
        commit: ObjectId,
    ) -> Result<BTreeMap<String, ObjectId>> {
        let Object::Commit(commit_obj) = objects::require(txn, commit)? else {
            return Err(MnemError::InvalidRef(format!("{commit} is not a commit")));
        };
        Ok(objects::require_state(txn, commit_obj.state)?.nodes)
    }

    /// The full memory at a commit: node id to decoded node. Read-only, no
    /// lock, no `HEAD` move.
    pub fn state_at(&self, commit: ObjectId) -> Result<BTreeMap<String, MemoryNode>> {
        let txn = self.begin_read()?;
        let mut out = BTreeMap::new();
        for (id, object_id) in self.state_map_at_in(&txn, commit)? {
            out.insert(id, objects::require_node(&txn, object_id)?);
        }
        Ok(out)
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
            "mnem-timetravel-{}-{}-{:?}",
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

    #[test]
    fn state_at_reconstructs_each_commit_exactly() {
        let root = scratch();
        let store = Store::init(&root).unwrap();

        store.stage(&node("plan", "enterprise")).unwrap();
        store.stage(&node("status", "open")).unwrap();
        let c1 = store.commit("open", "agent", 1_000).unwrap();

        store.stage(&node("plan", "pro")).unwrap();
        store.rm("status").unwrap();
        store.stage(&node("owner", "alice")).unwrap();
        let c2 = store.commit("update", "agent", 2_000).unwrap();

        let at_c1 = store.state_at(c1).unwrap();
        assert_eq!(at_c1.keys().collect::<Vec<_>>(), vec!["plan", "status"]);
        assert_eq!(at_c1["plan"].content, json!("enterprise"));

        let at_c2 = store.state_at(c2).unwrap();
        assert_eq!(at_c2.keys().collect::<Vec<_>>(), vec!["owner", "plan"]);
        assert_eq!(at_c2["plan"].content, json!("pro"));

        // state_at matches the working memory right after that commit
        assert_eq!(store.working_memory().unwrap(), at_c2);

        assert_eq!(
            store.state_map_at(c1).unwrap().keys().collect::<Vec<_>>(),
            vec!["plan", "status"]
        );

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn state_at_rejects_a_non_commit() {
        let root = scratch();
        let store = Store::init(&root).unwrap();
        store.stage(&node("a", "1")).unwrap();
        let c1 = store.commit("one", "agent", 1_000).unwrap();

        // the state object's id is not a commit
        let txn = store.begin_read().unwrap();
        let Object::Commit(commit) = objects::require(&txn, c1).unwrap() else {
            panic!("not a commit");
        };
        drop(txn);
        assert!(matches!(
            store.state_at(commit.state),
            Err(MnemError::InvalidRef(m)) if m.contains("not a commit")
        ));

        std::fs::remove_dir_all(&root).ok();
    }
}
