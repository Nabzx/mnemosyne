//! `blame`: which commit gave a memory node its current value (issue #52,
//! ADR-0015).
//!
//! A walk from a starting commit towards the root. At each commit, if a parent
//! already holds the exact node object being blamed, the value predates this
//! commit and the walk follows that parent (the first parent that matches, so a
//! value that came in on the merged-in side is followed to where it was
//! actually written). If no parent holds it, this commit introduced it.
//!
//! Read-only. Provenance is read straight off the returned node
//! ([`MemoryNode::provenance`](crate::object::MemoryNode)); an empty provenance
//! is legal and means `blame` resolved only to the commit.

use crate::error::{MnemError, Result};
use crate::id::ObjectId;
use crate::object::{MemoryNode, Object};
use crate::objects;
use crate::store::Store;

/// The result of [`Store::blame`].
#[derive(Debug, Clone, PartialEq)]
pub struct Blame {
    /// The commit that gave the node its current value.
    pub commit: ObjectId,
    /// The node object as written at that commit.
    pub node: MemoryNode,
    /// The node's `event_time`, or the commit's record time if it has none.
    pub time: i64,
}

impl Store {
    /// Resolve `node_id` to the commit, and the node, that introduced its
    /// current value as of `at` (a commit-ish; `None` is `HEAD`).
    ///
    /// Errors if `node_id` is not in memory at `at`: `blame` explains a value
    /// that exists, not when something was deleted.
    pub fn blame(&self, node_id: &str, at: Option<&str>) -> Result<Blame> {
        let txn = self.begin_read()?;

        let start = match at {
            Some(spec) => self.resolve_commitish_in(&txn, spec)?,
            None => self
                .resolve_head(&txn)?
                .ok_or_else(|| MnemError::InvalidRef("cannot blame: HEAD has no commit".into()))?,
        };

        let target = self
            .state_map_at_in(&txn, start)?
            .get(node_id)
            .copied()
            .ok_or_else(|| {
                MnemError::InvalidRef(format!("blame: {node_id:?} is not in memory at {start}"))
            })?;

        let mut current = start;
        loop {
            let Object::Commit(commit) = objects::require(&txn, current)? else {
                return Err(MnemError::CorruptStore(format!(
                    "{current} is not a commit"
                )));
            };

            // the first parent that already holds this exact node object
            let mut inherited = None;
            for parent in &commit.parents {
                if self.state_map_at_in(&txn, *parent)?.get(node_id).copied() == Some(target) {
                    inherited = Some(*parent);
                    break;
                }
            }

            match inherited {
                // a parent already holds this exact value: follow it back
                Some(parent) => current = parent,
                // no parent holds it (including the root, with no parents):
                // this commit introduced it
                None => {
                    let node = objects::require_node(&txn, target)?;
                    let time = node.event_time.unwrap_or(commit.time);
                    return Ok(Blame {
                        commit: current,
                        node,
                        time,
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::json;

    use super::*;
    use crate::object::{ContentKind, Provenance};

    fn scratch() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let base = std::env::temp_dir().join(format!(
            "mnem-blame-{}-{}-{:?}",
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
    fn blame_finds_the_commit_that_last_set_the_value() {
        let root = scratch();
        let store = Store::init(&root).unwrap();

        store.stage(&node("plan", "enterprise")).unwrap();
        store.stage(&node("owner", "alice")).unwrap();
        let c1 = store.commit("one", "agent", 1_000).unwrap();

        store.stage(&node("plan", "pro")).unwrap();
        let c2 = store.commit("two", "agent", 2_000).unwrap();

        store.stage(&node("note", "unrelated")).unwrap();
        store.commit("three", "agent", 3_000).unwrap();

        // plan was last set at c2, owner at c1
        assert_eq!(store.blame("plan", None).unwrap().commit, c2);
        assert_eq!(store.blame("owner", None).unwrap().commit, c1);
        // and at an explicit past commit
        assert_eq!(store.blame("plan", Some(&c1.to_hex())).unwrap().commit, c1);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn blame_reads_provenance_and_event_time() {
        let root = scratch();
        let store = Store::init(&root).unwrap();

        let mut n = node("plan", "pro");
        n.provenance = Provenance {
            agent_step: Some("step-14".into()),
            observation: Some("obs-7".into()),
            ..Default::default()
        };
        n.event_time = Some(1_555_000_000_000);
        store.stage(&n).unwrap();
        let c1 = store.commit("one", "agent", 9_000).unwrap();

        let blame = store.blame("plan", None).unwrap();
        assert_eq!(blame.commit, c1);
        assert_eq!(blame.node.provenance.agent_step.as_deref(), Some("step-14"));
        assert_eq!(blame.time, 1_555_000_000_000); // event_time wins over commit time

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn blame_errors_when_the_node_is_absent() {
        let root = scratch();
        let store = Store::init(&root).unwrap();
        store.stage(&node("a", "1")).unwrap();
        store.commit("one", "agent", 1_000).unwrap();

        assert!(matches!(
            store.blame("ghost", None),
            Err(MnemError::InvalidRef(m)) if m.contains("not in memory")
        ));

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn blame_follows_a_value_through_a_merge_to_its_real_origin() {
        let root = scratch();
        let store = Store::init(&root).unwrap();

        store.stage(&node("plan", "enterprise")).unwrap();
        store.commit("base", "agent", 1_000).unwrap();
        store.branch("feature", None).unwrap();

        // ours does unrelated work
        store.stage(&node("owner", "alice")).unwrap();
        store.commit("ours", "agent", 2_000).unwrap();

        // theirs sets the value we will blame
        store.checkout("feature", false).unwrap();
        store.stage(&node("plan", "pro")).unwrap();
        let theirs = store.commit("theirs", "agent", 3_000).unwrap();

        store.checkout("main", false).unwrap();
        let outcome = store
            .merge("feature", &BTreeMap::new(), None, None, "agent", 4_000)
            .unwrap();
        assert!(matches!(outcome, crate::MergeOutcome::Merged(_)));

        // blame follows `plan` past the merge commit to where theirs wrote it
        assert_eq!(store.blame("plan", None).unwrap().commit, theirs);

        std::fs::remove_dir_all(&root).ok();
    }
}
