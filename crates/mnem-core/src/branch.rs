//! Branch operations (issue #36, ADR-0012).
//!
//! A branch is a row in the `refs` table (ADR-0009): a name pointing at a
//! commit. Creating one is an O(1) write and copies nothing. These build on the
//! [`refs`](crate::refs) primitives.

use crate::error::{MnemError, Result};
use crate::head::Head;
use crate::id::ObjectId;
use crate::refs;
use crate::store::Store;

impl Store {
    /// Create a branch. `start` is a commit-ish (a branch name or a commit id
    /// prefix); `None` means where `HEAD` currently resolves.
    ///
    /// Fails if `name` is invalid (ADR-0009 rules) or already exists, or if
    /// `start` is `None` and `HEAD` has no commit yet.
    pub fn branch(&self, name: &str, start: Option<&str>) -> Result<()> {
        refs::validate_name(name)?;

        let target = match start {
            Some(spec) => self.resolve_commitish(spec)?,
            None => {
                let txn = self.begin_read()?;
                self.resolve_head(&txn)?.ok_or_else(|| {
                    MnemError::InvalidRef(
                        "cannot create a branch: HEAD does not point at a commit yet".to_string(),
                    )
                })?
            }
        };

        let txn = self.begin_write()?;
        refs::compare_and_set(&txn, name, None, target).map_err(|e| match e {
            MnemError::Conflict(_) => {
                MnemError::InvalidRef(format!("branch {name:?} already exists"))
            }
            other => other,
        })?;
        txn.commit()?;
        Ok(())
    }

    /// Every branch, name to tip commit, sorted by name.
    pub fn branches(&self) -> Result<Vec<(String, ObjectId)>> {
        let txn = self.begin_read()?;
        refs::list(&txn)
    }

    /// Delete a branch. Returns whether it existed. Refuses the branch `HEAD`
    /// is currently on. No force variant: with no garbage collection, a
    /// deleted branch's commits stay reachable by id.
    pub fn delete_branch(&self, name: &str) -> Result<bool> {
        if let Head::Attached(current) = self.head()? {
            if current == name {
                return Err(MnemError::InvalidRef(format!(
                    "cannot delete {name:?}: it is the current branch"
                )));
            }
        }
        let txn = self.begin_write()?;
        let existed = refs::delete(&txn, name)?;
        txn.commit()?;
        Ok(existed)
    }
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
            "mnem-branch-{}-{}-{:?}",
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

    fn seeded() -> (std::path::PathBuf, Store, ObjectId) {
        let root = scratch();
        let store = Store::init(&root).unwrap();
        store.stage(&node("a")).unwrap();
        let c1 = store.commit("one", "agent", 1_000).unwrap();
        (root, store, c1)
    }

    #[test]
    fn branch_creates_a_ref_at_head() {
        let (root, store, c1) = seeded();
        store.branch("feature", None).unwrap();
        let branches = store.branches().unwrap();
        assert_eq!(branches, vec![("feature".into(), c1), ("main".into(), c1)]);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn branch_takes_a_start_point() {
        let (root, store, c1) = seeded();
        store.stage(&node("b")).unwrap();
        let c2 = store.commit("two", "agent", 2_000).unwrap();

        // by another branch name
        store.branch("from-main", Some("main")).unwrap();
        // by a full commit id
        store.branch("at-c1", Some(&c1.to_hex())).unwrap();
        // by a short prefix
        store
            .branch("at-c1-short", Some(&c1.to_hex()[..8]))
            .unwrap();

        let by_name: std::collections::HashMap<_, _> =
            store.branches().unwrap().into_iter().collect();
        assert_eq!(by_name["from-main"], c2);
        assert_eq!(by_name["at-c1"], c1);
        assert_eq!(by_name["at-c1-short"], c1);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn branch_refuses_a_duplicate_or_bad_name() {
        let (root, store, _) = seeded();
        store.branch("dup", None).unwrap();
        assert!(matches!(
            store.branch("dup", None),
            Err(MnemError::InvalidRef(m)) if m.contains("already exists")
        ));
        assert!(store.branch("has space", None).is_err());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn branch_refuses_when_head_is_unborn() {
        let root = scratch();
        let store = Store::init(&root).unwrap();
        assert!(matches!(
            store.branch("early", None),
            Err(MnemError::InvalidRef(m)) if m.contains("no commit yet") || m.contains("not point at a commit")
        ));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn delete_branch_drops_the_ref_but_not_the_current_one() {
        let (root, store, _) = seeded();
        store.branch("scratch", None).unwrap();
        assert!(store.delete_branch("scratch").unwrap());
        assert!(!store.delete_branch("scratch").unwrap());
        assert!(matches!(
            store.delete_branch("main"),
            Err(MnemError::InvalidRef(m)) if m.contains("current branch")
        ));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn resolve_commitish_covers_names_ids_and_errors() {
        let (root, store, c1) = seeded();
        assert_eq!(store.resolve_commitish("main").unwrap(), c1);
        assert_eq!(store.resolve_commitish(&c1.to_hex()).unwrap(), c1);
        assert_eq!(store.resolve_commitish(&c1.to_hex()[..6]).unwrap(), c1);

        assert!(store.resolve_commitish("nope").is_err()); // not a name, not hex
        assert!(store.resolve_commitish("abc").is_err()); // too short
        assert!(store.resolve_commitish("ffffffff").is_err()); // no such commit

        // a prefix that matches the state object (not a commit) resolves to nothing
        let txn = store.begin_read().unwrap();
        let tip = store.resolve_head(&txn).unwrap().unwrap();
        let crate::object::Object::Commit(commit) = crate::objects::require(&txn, tip).unwrap()
        else {
            panic!("not a commit");
        };
        drop(txn);
        assert!(store.resolve_commitish(&commit.state.to_hex()).is_err());

        std::fs::remove_dir_all(&root).ok();
    }
}
