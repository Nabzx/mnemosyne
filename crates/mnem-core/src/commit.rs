//! Staging and the commit operation (issue #26).
//!
//! `stage` records a memory node for the next commit. `commit` builds the new
//! state from the parent's state plus everything staged, writes the `State` and
//! `Commit` objects, moves the current branch, and clears staging, all in one
//! `redb` write transaction (ADR-0008), so a commit is atomic.

use std::collections::BTreeMap;

use redb::ReadableTable;

use crate::codec;
use crate::error::{MnemError, Result};
use crate::head::Head;
use crate::id::ObjectId;
use crate::object::{Commit, MemoryNode, Object, State};
use crate::store::Store;
use crate::{objects, refs, staging};

fn load_object(txn: &redb::WriteTransaction, id: ObjectId) -> Result<Object> {
    let table = txn.open_table(objects::OBJECTS).map_err(err)?;
    let guard = table
        .get(id.as_bytes().as_slice())
        .map_err(err)?
        .ok_or(MnemError::NotFound(id))?;
    codec::decode(guard.value())
}

fn write_object(txn: &redb::WriteTransaction, object: &Object) -> Result<ObjectId> {
    let bytes = codec::encode(object);
    let id = ObjectId::hash_canonical(&bytes);
    let mut table = txn.open_table(objects::OBJECTS).map_err(err)?;
    if table.get(id.as_bytes().as_slice()).map_err(err)?.is_none() {
        table
            .insert(id.as_bytes().as_slice(), bytes.as_slice())
            .map_err(err)?;
    }
    Ok(id)
}

fn branch_tip(txn: &redb::WriteTransaction, branch: &str) -> Result<Option<ObjectId>> {
    let table = txn.open_table(refs::REFS).map_err(err)?;
    let tip = table
        .get(branch)
        .map_err(err)?
        .map(|guard| ObjectId::from_bytes(guard.value()));
    Ok(tip)
}

fn err<E: std::fmt::Display>(e: E) -> MnemError {
    MnemError::Io(e.to_string())
}

impl Store {
    /// Stage a memory node for the next commit. The node object is written to
    /// the store and its id is recorded in staging.
    pub fn stage(&self, node: &MemoryNode) -> Result<ObjectId> {
        let txn = self.begin_write()?;
        let node_id = node.id.clone();
        let object_id = write_object(&txn, &Object::MemoryNode(node.clone()))?;
        staging::put(&txn, &node_id, object_id)?;
        txn.commit().map_err(err)?;
        Ok(object_id)
    }

    /// The staged nodes, id to object id, sorted by id.
    pub fn staged(&self) -> Result<Vec<(String, ObjectId)>> {
        let txn = self.begin_read()?;
        staging::list(&txn)
    }

    /// Drop a node from staging. Returns whether it was staged.
    pub fn unstage(&self, node_id: &str) -> Result<bool> {
        let txn = self.begin_write()?;
        let existed = staging::remove(&txn, node_id)?;
        txn.commit().map_err(err)?;
        Ok(existed)
    }

    /// Record everything staged as a commit on the current branch, and return
    /// its id.
    ///
    /// `time_ms` is the record time in Unix milliseconds; the caller supplies
    /// it. Fails if `HEAD` is detached. The first commit has no parent and
    /// creates the branch.
    pub fn commit(&self, message: &str, author: &str, time_ms: i64) -> Result<ObjectId> {
        let branch = match self.head()? {
            Head::Attached(name) => name,
            Head::Detached(_) => {
                return Err(MnemError::InvalidRef(
                    "cannot commit from a detached HEAD; check out a branch first".to_string(),
                ));
            }
        };

        let txn = self.begin_write()?;

        let parent = branch_tip(&txn, &branch)?;

        let mut nodes: BTreeMap<String, ObjectId> = match parent {
            Some(tip) => {
                let Object::Commit(commit) = load_object(&txn, tip)? else {
                    return Err(MnemError::CorruptStore(format!("{tip} is not a commit")));
                };
                let Object::State(state) = load_object(&txn, commit.state)? else {
                    return Err(MnemError::CorruptStore(format!(
                        "{} is not a state",
                        commit.state
                    )));
                };
                state.nodes
            }
            None => BTreeMap::new(),
        };

        for (node_id, object_id) in staging::list_in_write(&txn)? {
            nodes.insert(node_id, object_id);
        }

        let state_id = write_object(&txn, &Object::State(State { nodes }))?;

        let commit = Commit {
            parents: parent.into_iter().collect(),
            state: state_id,
            message: message.to_string(),
            author: author.to_string(),
            time: time_ms,
        };
        let commit_id = write_object(&txn, &Object::Commit(commit))?;

        // Move the branch, compare-and-swap against the parent tip (ADR-0009).
        {
            let mut table = txn.open_table(refs::REFS).map_err(err)?;
            let current = table
                .get(branch.as_str())
                .map_err(err)?
                .map(|guard| ObjectId::from_bytes(guard.value()));
            if current != parent {
                return Err(MnemError::Conflict(format!(
                    "branch {branch:?} moved during commit"
                )));
            }
            table
                .insert(branch.as_str(), commit_id.as_bytes())
                .map_err(err)?;
        }

        staging::clear(&txn)?;

        txn.commit().map_err(err)?;
        Ok(commit_id)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::object::{ContentKind, Provenance};

    fn tempdir() -> std::path::PathBuf {
        let base = std::env::temp_dir().join(format!(
            "mnem-commit-{}-{:?}",
            std::process::id(),
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
    fn first_commit_creates_the_branch_with_no_parent() {
        let root = tempdir();
        let store = Store::init(&root).unwrap();

        store.stage(&node("a", "first belief")).unwrap();
        let c1 = store.commit("start", "agent", 1_000).unwrap();

        // the branch now points at c1
        let txn = store.begin_read().unwrap();
        assert_eq!(refs::get(&txn, "main").unwrap(), Some(c1));
        assert_eq!(store.resolve_head(&txn).unwrap(), Some(c1));

        // c1 has no parent and one node in its state
        let Object::Commit(commit) = objects::require(&txn, c1).unwrap() else {
            panic!()
        };
        assert!(commit.parents.is_empty());
        let Object::State(state) = objects::require(&txn, commit.state).unwrap() else {
            panic!()
        };
        assert_eq!(state.nodes.len(), 1);
        assert!(state.nodes.contains_key("a"));

        // staging is now empty
        drop(txn);
        assert!(store.staged().unwrap().is_empty());

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_second_commit_carries_the_first_ones_nodes_and_chains() {
        let root = tempdir();
        let store = Store::init(&root).unwrap();

        store.stage(&node("a", "belief a")).unwrap();
        let c1 = store.commit("one", "agent", 1_000).unwrap();

        store.stage(&node("b", "belief b")).unwrap();
        store.stage(&node("a", "belief a, revised")).unwrap();
        let c2 = store.commit("two", "agent", 2_000).unwrap();

        let txn = store.begin_read().unwrap();
        let Object::Commit(commit) = objects::require(&txn, c2).unwrap() else {
            panic!()
        };
        assert_eq!(commit.parents, vec![c1]);

        let Object::State(state) = objects::require(&txn, commit.state).unwrap() else {
            panic!()
        };
        assert_eq!(state.nodes.len(), 2);

        // node "a" now points at the revised object, not the original
        let revised = state.nodes.get("a").copied().unwrap();
        let Object::MemoryNode(a) = objects::require(&txn, revised).unwrap() else {
            panic!()
        };
        assert_eq!(a.content, json!("belief a, revised"));

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn commit_is_refused_from_a_detached_head() {
        let root = tempdir();
        let store = Store::init(&root).unwrap();

        store.stage(&node("a", "x")).unwrap();
        let c1 = store.commit("one", "agent", 1_000).unwrap();
        store.set_head(&Head::Detached(c1)).unwrap();

        store.stage(&node("b", "y")).unwrap();
        assert!(matches!(
            store.commit("two", "agent", 2_000),
            Err(MnemError::InvalidRef(_))
        ));

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn unstage_drops_a_node_before_it_is_committed() {
        let root = tempdir();
        let store = Store::init(&root).unwrap();

        store.stage(&node("a", "keep")).unwrap();
        store.stage(&node("b", "drop")).unwrap();
        assert!(store.unstage("b").unwrap());
        assert!(!store.unstage("b").unwrap());

        let c1 = store.commit("one", "agent", 1_000).unwrap();
        let txn = store.begin_read().unwrap();
        let Object::Commit(commit) = objects::require(&txn, c1).unwrap() else {
            panic!()
        };
        let Object::State(state) = objects::require(&txn, commit.state).unwrap() else {
            panic!()
        };
        assert_eq!(state.nodes.keys().collect::<Vec<_>>(), vec!["a"]);

        std::fs::remove_dir_all(&root).ok();
    }
}
