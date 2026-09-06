//! Walking the commit graph (issue #27).
//!
//! [`Store::log`] returns the commits reachable from a starting commit, newest
//! first. The first-parent walk follows only `parents[0]` of each commit and
//! gives a linear history; the full walk visits every parent, deduplicates, and
//! orders by record time then id so the result is deterministic.

use std::collections::HashSet;

use crate::error::{MnemError, Result};
use crate::id::ObjectId;
use crate::object::{Commit, Object};
use crate::objects;
use crate::store::Store;

impl Store {
    /// The commits reachable from `start`, newest first.
    ///
    /// With `first_parent`, only `parents[0]` of each commit is followed.
    /// `limit` caps the number returned.
    pub fn log(
        &self,
        start: ObjectId,
        first_parent: bool,
        limit: Option<usize>,
    ) -> Result<Vec<(ObjectId, Commit)>> {
        let txn = self.begin_read()?;

        let load = |id: ObjectId| -> Result<Commit> {
            match objects::require(&txn, id)? {
                Object::Commit(commit) => Ok(commit),
                other => Err(MnemError::CorruptStore(format!(
                    "{id} is a {}, not a commit",
                    other.kind()
                ))),
            }
        };

        let mut out: Vec<(ObjectId, Commit)> = Vec::new();

        if first_parent {
            let mut current = Some(start);
            while let Some(id) = current {
                if limit.is_some_and(|n| out.len() >= n) {
                    break;
                }
                let commit = load(id)?;
                current = commit.parents.first().copied();
                out.push((id, commit));
            }
        } else {
            let mut seen: HashSet<ObjectId> = HashSet::new();
            let mut stack = vec![start];
            while let Some(id) = stack.pop() {
                if !seen.insert(id) {
                    continue;
                }
                let commit = load(id)?;
                for parent in &commit.parents {
                    if !seen.contains(parent) {
                        stack.push(*parent);
                    }
                }
                out.push((id, commit));
            }
            out.sort_by(|a, b| {
                b.1.time
                    .cmp(&a.1.time)
                    .then_with(|| b.0.as_bytes().cmp(a.0.as_bytes()))
            });
            if let Some(n) = limit {
                out.truncate(n);
            }
        }

        Ok(out)
    }

    /// [`Store::log`] starting from where `HEAD` resolves. `Ok(vec![])` when
    /// `HEAD` is attached to a branch that has no commit yet.
    pub fn log_head(
        &self,
        first_parent: bool,
        limit: Option<usize>,
    ) -> Result<Vec<(ObjectId, Commit)>> {
        let start = {
            let txn = self.begin_read()?;
            self.resolve_head(&txn)?
        };
        match start {
            Some(id) => self.log(id, first_parent, limit),
            None => Ok(Vec::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::codec;
    use crate::object::{ContentKind, MemoryNode, Provenance, State};

    fn tempdir() -> std::path::PathBuf {
        let base = std::env::temp_dir().join(format!(
            "mnem-log-{}-{:?}",
            std::process::id(),
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

    /// Write a raw commit object into the store, outside `Store::commit`, so a
    /// merge can be constructed before `branch` and `checkout` exist.
    fn put_raw_commit(store: &Store, parents: Vec<ObjectId>, time: i64) -> ObjectId {
        let txn = store.begin_write().unwrap();
        let state_bytes = codec::encode(&Object::State(State::default()));
        let state_id = ObjectId::hash_canonical(&state_bytes);
        let commit = Object::Commit(Commit {
            parents,
            state: state_id,
            message: format!("raw {time}"),
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
    fn first_parent_walk_is_linear_and_newest_first() {
        let root = tempdir();
        let store = Store::init(&root).unwrap();

        store.stage(&node("a")).unwrap();
        let c1 = store.commit("one", "agent", 1_000).unwrap();
        store.stage(&node("b")).unwrap();
        let c2 = store.commit("two", "agent", 2_000).unwrap();
        store.stage(&node("c")).unwrap();
        let c3 = store.commit("three", "agent", 3_000).unwrap();

        let ids: Vec<_> = store
            .log(c3, true, None)
            .unwrap()
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        assert_eq!(ids, vec![c3, c2, c1]);

        assert_eq!(store.log(c3, true, Some(2)).unwrap().len(), 2);
        assert_eq!(store.log_head(true, None).unwrap().len(), 3);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn full_walk_visits_every_parent_once() {
        let root = tempdir();
        let store = Store::init(&root).unwrap();

        store.stage(&node("a")).unwrap();
        let c1 = store.commit("one", "agent", 1_000).unwrap();
        store.stage(&node("b")).unwrap();
        let c2 = store.commit("two", "agent", 2_000).unwrap();

        // a hand-built merge of c2 and c1
        let merge = put_raw_commit(&store, vec![c2, c1], 3_000);

        let full: Vec<_> = store
            .log(merge, false, None)
            .unwrap()
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        assert_eq!(full, vec![merge, c2, c1]);
        // c1 appears once even though both merge->c2->c1 and merge->c1 reach it
        assert_eq!(full.iter().filter(|id| **id == c1).count(), 1);

        // first-parent from the merge skips the second parent's unique history
        let fp: Vec<_> = store
            .log(merge, true, None)
            .unwrap()
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        assert_eq!(fp, vec![merge, c2, c1]);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn log_head_is_empty_on_a_fresh_store() {
        let root = tempdir();
        let store = Store::init(&root).unwrap();
        assert!(store.log_head(true, None).unwrap().is_empty());
        assert!(store.log_head(false, None).unwrap().is_empty());
        std::fs::remove_dir_all(&root).ok();
    }
}
