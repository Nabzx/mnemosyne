//! Checkout, working memory, and node deletion (issue #37, ADR-0012).
//!
//! `checkout` moves `HEAD` and nothing else. Working memory is a view, never
//! stored: the `HEAD` commit's state, minus staged tombstones, with `staging`
//! overlaid. `rm` stages a tombstone.

use std::collections::BTreeMap;

use crate::error::{MnemError, Result};
use crate::head::Head;
use crate::id::ObjectId;
use crate::object::MemoryNode;
use crate::store::Store;
use crate::{objects, refs, staging};

/// What a [`Store::checkout`] did, for the caller to report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Checkout {
    /// `HEAD` is now attached to this branch.
    SwitchedToBranch(String),
    /// `HEAD` is now detached at this commit.
    DetachedAt(ObjectId),
    /// The target was already the current position; nothing changed.
    AlreadyThere,
}

impl Store {
    /// The `nodes` map of the `HEAD` commit's state, or empty when `HEAD` has no
    /// commit yet.
    fn head_state_map(&self, txn: &redb::ReadTransaction) -> Result<BTreeMap<String, ObjectId>> {
        match self.resolve_head(txn)? {
            Some(tip) => self.state_map_at_in(txn, tip),
            None => Ok(BTreeMap::new()),
        }
    }

    /// Move `HEAD` to `target` (a branch name or a commit-ish). A branch target
    /// attaches `HEAD`; a commit target detaches it.
    ///
    /// Refused when `staging` or the tombstone set is non-empty, unless
    /// `discard` is set (then both are cleared). One atomic `HEAD` write, no
    /// lock.
    pub fn checkout(&self, target: &str, discard: bool) -> Result<Checkout> {
        let txn = self.begin_read()?;

        let new_head = if refs::get(&txn, target)?.is_some() {
            Head::Attached(target.to_string())
        } else {
            Head::Detached(self.resolve_commitish_in(&txn, target)?)
        };

        if self.head()? == new_head {
            return Ok(Checkout::AlreadyThere);
        }

        let staged = staging::list(&txn)?;
        let tombs = staging::tombstones(&txn)?;
        drop(txn);

        if !staged.is_empty() || !tombs.is_empty() {
            if !discard {
                let mut pending: Vec<String> = staged.into_iter().map(|(id, _)| id).collect();
                pending.extend(tombs);
                pending.sort();
                return Err(MnemError::InvalidRef(format!(
                    "checkout refused: {} pending change(s) ({}); commit them or pass discard",
                    pending.len(),
                    pending.join(", ")
                )));
            }
            let w = self.begin_write()?;
            staging::clear(&w)?;
            staging::clear_tombstones(&w)?;
            w.commit()?;
        }

        self.set_head(&new_head)?;
        Ok(match new_head {
            Head::Attached(name) => Checkout::SwitchedToBranch(name),
            Head::Detached(id) => Checkout::DetachedAt(id),
        })
    }

    /// The current working memory: the `HEAD` commit's state, minus staged
    /// tombstones, with `staging` overlaid. Computed on every call.
    pub fn working_memory(&self) -> Result<BTreeMap<String, MemoryNode>> {
        let txn = self.begin_read()?;

        let mut out: BTreeMap<String, MemoryNode> = BTreeMap::new();
        for (id, object_id) in self.head_state_map(&txn)? {
            out.insert(id, objects::require_node(&txn, object_id)?);
        }
        for id in staging::tombstones(&txn)? {
            out.remove(&id);
        }
        for (id, object_id) in staging::list(&txn)? {
            out.insert(id, objects::require_node(&txn, object_id)?);
        }
        Ok(out)
    }

    /// One node from working memory, or `None` if it is absent or tombstoned.
    pub fn working_node(&self, id: &str) -> Result<Option<MemoryNode>> {
        let txn = self.begin_read()?;

        if let Some((_, object_id)) = staging::list(&txn)?.into_iter().find(|(sid, _)| sid == id) {
            return Ok(Some(objects::require_node(&txn, object_id)?));
        }
        if staging::tombstones(&txn)?.iter().any(|t| t == id) {
            return Ok(None);
        }
        match self.head_state_map(&txn)?.get(id) {
            Some(&object_id) => Ok(Some(objects::require_node(&txn, object_id)?)),
            None => Ok(None),
        }
    }

    /// Stage the deletion of a node from the next commit's state.
    ///
    /// If the node is only a pending add (staged, not in the `HEAD` state), this
    /// just unstages it. If it is in neither place, it is an error.
    pub fn rm(&self, id: &str) -> Result<()> {
        let (staged, in_head) = {
            let txn = self.begin_read()?;
            let staged = staging::list(&txn)?.iter().any(|(sid, _)| sid == id);
            let in_head = self.head_state_map(&txn)?.contains_key(id);
            (staged, in_head)
        };
        if !staged && !in_head {
            return Err(MnemError::InvalidRef(format!("no node {id:?} in memory")));
        }

        let txn = self.begin_write()?;
        staging::remove(&txn, id)?;
        if in_head {
            staging::tombstone(&txn, id)?;
        }
        txn.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::head::Head;
    use crate::object::{ContentKind, Provenance};

    fn scratch() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let base = std::env::temp_dir().join(format!(
            "mnem-checkout-{}-{}-{:?}",
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
    fn checkout_switches_branches_and_detaches() {
        let root = scratch();
        let store = Store::init(&root).unwrap();
        store.stage(&node("a", "1")).unwrap();
        let c1 = store.commit("one", "agent", 1_000).unwrap();
        store.branch("feature", None).unwrap();

        assert_eq!(
            store.checkout("feature", false).unwrap(),
            Checkout::SwitchedToBranch("feature".into())
        );
        assert_eq!(store.head().unwrap(), Head::Attached("feature".into()));
        assert_eq!(
            store.checkout("feature", false).unwrap(),
            Checkout::AlreadyThere
        );

        assert_eq!(
            store.checkout(&c1.to_hex()[..8], false).unwrap(),
            Checkout::DetachedAt(c1)
        );
        assert_eq!(store.head().unwrap(), Head::Detached(c1));

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn checkout_refuses_a_dirty_index_unless_discarded() {
        let root = scratch();
        let store = Store::init(&root).unwrap();
        store.stage(&node("a", "1")).unwrap();
        store.commit("one", "agent", 1_000).unwrap();
        store.branch("feature", None).unwrap();

        store.stage(&node("b", "2")).unwrap();
        assert!(matches!(
            store.checkout("feature", false),
            Err(MnemError::InvalidRef(m)) if m.contains("pending change")
        ));

        store.checkout("feature", true).unwrap();
        assert!(store.staged().unwrap().is_empty());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn working_memory_is_head_state_minus_tombstones_plus_staging() {
        let root = scratch();
        let store = Store::init(&root).unwrap();
        store.stage(&node("keep", "v1")).unwrap();
        store.stage(&node("drop", "v1")).unwrap();
        store.commit("one", "agent", 1_000).unwrap();

        store.stage(&node("keep", "v2")).unwrap(); // update
        store.stage(&node("fresh", "v1")).unwrap(); // add
        store.rm("drop").unwrap(); // tombstone

        let wm = store.working_memory().unwrap();
        assert_eq!(wm.keys().collect::<Vec<_>>(), vec!["fresh", "keep"]);
        assert_eq!(wm["keep"].content, json!("v2"));
        assert_eq!(store.working_node("drop").unwrap(), None);
        assert_eq!(
            store.working_node("keep").unwrap().unwrap().content,
            json!("v2")
        );

        // a commit applies the tombstone and clears both staging tables
        store.commit("two", "agent", 2_000).unwrap();
        assert!(store.staged().unwrap().is_empty());
        let after = store.working_memory().unwrap();
        assert_eq!(after.keys().collect::<Vec<_>>(), vec!["fresh", "keep"]);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn rm_of_a_pending_add_just_unstages_and_rm_of_nothing_errors() {
        let root = scratch();
        let store = Store::init(&root).unwrap();
        store.stage(&node("a", "1")).unwrap();
        store.commit("one", "agent", 1_000).unwrap();

        store.stage(&node("pending", "1")).unwrap();
        store.rm("pending").unwrap();
        assert!(store.staged().unwrap().is_empty());
        assert!(!store.working_memory().unwrap().contains_key("pending"));

        assert!(matches!(
            store.rm("ghost"),
            Err(MnemError::InvalidRef(m)) if m.contains("no node")
        ));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn staging_a_node_lifts_its_tombstone() {
        let root = scratch();
        let store = Store::init(&root).unwrap();
        store.stage(&node("a", "1")).unwrap();
        store.commit("one", "agent", 1_000).unwrap();

        store.rm("a").unwrap();
        assert_eq!(store.working_node("a").unwrap(), None);
        store.stage(&node("a", "2")).unwrap();
        assert_eq!(
            store.working_node("a").unwrap().unwrap().content,
            json!("2")
        );

        std::fs::remove_dir_all(&root).ok();
    }
}
