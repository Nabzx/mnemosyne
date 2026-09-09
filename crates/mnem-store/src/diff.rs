//! Structural diff between two memory states (issue #39, ADR-0012).
//!
//! A merge over the two states' `id -> ObjectId` maps: each node id is
//! classified as added, removed or modified (a different `ObjectId`). Carries
//! ids and hashes only; a caller that wants content loads the changed nodes.

use std::collections::BTreeMap;

use crate::error::Result;
use crate::id::ObjectId;
use crate::store::Store;

/// One side of a [`Store::diff`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffTarget {
    /// The state at a commit.
    Commit(ObjectId),
    /// The current working memory.
    Working,
    /// The empty state (an unborn `HEAD`).
    Empty,
}

/// How one node changed between two states. `id` is the node id; `old` and
/// `new` are the node object ids in the `from` and `to` states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeChange {
    /// Present only in the `to` state.
    Added {
        /// The node id.
        id: String,
        /// The node object in the `to` state.
        new: ObjectId,
    },
    /// Present only in the `from` state.
    Removed {
        /// The node id.
        id: String,
        /// The node object in the `from` state.
        old: ObjectId,
    },
    /// Present in both, pointing at different objects.
    Modified {
        /// The node id.
        id: String,
        /// The node object in the `from` state.
        old: ObjectId,
        /// The node object in the `to` state.
        new: ObjectId,
    },
}

impl NodeChange {
    /// The node id this change is about.
    pub fn id(&self) -> &str {
        match self {
            Self::Added { id, .. } | Self::Removed { id, .. } | Self::Modified { id, .. } => id,
        }
    }
}

impl Store {
    /// The changes that turn `from` into `to`, in node-id order.
    pub fn diff(&self, from: DiffTarget, to: DiffTarget) -> Result<Vec<NodeChange>> {
        let txn = self.begin_read()?;
        let from_map = self.state_map_for(&txn, from)?;
        let to_map = self.state_map_for(&txn, to)?;
        Ok(diff_maps(&from_map, &to_map))
    }

    fn state_map_for(
        &self,
        txn: &redb::ReadTransaction,
        target: DiffTarget,
    ) -> Result<BTreeMap<String, ObjectId>> {
        match target {
            DiffTarget::Commit(id) => self.state_map_at_in(txn, id),
            DiffTarget::Working => self.working_state_map(txn),
            DiffTarget::Empty => Ok(BTreeMap::new()),
        }
    }
}

fn diff_maps(
    from: &BTreeMap<String, ObjectId>,
    to: &BTreeMap<String, ObjectId>,
) -> Vec<NodeChange> {
    let mut out = Vec::new();
    let mut ids: Vec<&String> = from.keys().chain(to.keys()).collect();
    ids.sort_unstable();
    ids.dedup();
    for id in ids {
        match (from.get(id), to.get(id)) {
            (Some(&old), None) => out.push(NodeChange::Removed {
                id: id.clone(),
                old,
            }),
            (None, Some(&new)) => out.push(NodeChange::Added {
                id: id.clone(),
                new,
            }),
            (Some(&old), Some(&new)) if old != new => out.push(NodeChange::Modified {
                id: id.clone(),
                old,
                new,
            }),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::object::{ContentKind, MemoryNode, Provenance};

    fn scratch() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let base = std::env::temp_dir().join(format!(
            "mnem-diff-{}-{}-{:?}",
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
    fn diff_classifies_added_removed_and_modified() {
        let root = scratch();
        let store = Store::init(&root).unwrap();

        store.stage(&node("keep", "v1")).unwrap();
        store.stage(&node("change", "v1")).unwrap();
        store.stage(&node("gone", "v1")).unwrap();
        let c1 = store.commit("one", "agent", 1_000).unwrap();

        store.stage(&node("change", "v2")).unwrap();
        store.stage(&node("fresh", "v1")).unwrap();
        store.rm("gone").unwrap();
        let c2 = store.commit("two", "agent", 2_000).unwrap();

        let changes = store
            .diff(DiffTarget::Commit(c1), DiffTarget::Commit(c2))
            .unwrap();
        let ids: Vec<&str> = changes.iter().map(NodeChange::id).collect();
        assert_eq!(ids, vec!["change", "fresh", "gone"]);
        assert!(matches!(changes[0], NodeChange::Modified { .. }));
        assert!(matches!(changes[1], NodeChange::Added { .. }));
        assert!(matches!(changes[2], NodeChange::Removed { .. }));

        // reverse is the mirror image
        let back = store
            .diff(DiffTarget::Commit(c2), DiffTarget::Commit(c1))
            .unwrap();
        assert!(matches!(back[1], NodeChange::Removed { .. })); // fresh
        assert!(matches!(back[2], NodeChange::Added { .. })); // gone

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn diff_against_working_memory_and_empty() {
        let root = scratch();
        let store = Store::init(&root).unwrap();
        store.stage(&node("a", "v1")).unwrap();
        let c1 = store.commit("one", "agent", 1_000).unwrap();

        store.stage(&node("a", "v2")).unwrap();
        store.stage(&node("b", "v1")).unwrap();

        let staged = store
            .diff(DiffTarget::Commit(c1), DiffTarget::Working)
            .unwrap();
        assert_eq!(staged.len(), 2);
        assert!(matches!(staged[0], NodeChange::Modified { .. })); // a
        assert!(matches!(staged[1], NodeChange::Added { .. })); // b

        let from_nothing = store
            .diff(DiffTarget::Empty, DiffTarget::Commit(c1))
            .unwrap();
        assert_eq!(from_nothing.len(), 1);
        assert!(matches!(from_nothing[0], NodeChange::Added { .. }));

        std::fs::remove_dir_all(&root).ok();
    }
}
