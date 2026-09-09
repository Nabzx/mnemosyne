//! The commit-nodes index (issue #51, ADR-0015).
//!
//! A reverse map: for each commit, which node ids it changed relative to its
//! first parent, and how (`Added` / `Modified` / `Removed`). It is derived from
//! `diff(parents[0], commit)` and written inside the commit's own write
//! transaction (`Store::commit`, `Store::merge`), so an entry is always
//! consistent with its commit.
//!
//! It is **never the source of truth**. Every reader
//! ([`Store::changed_by`](crate::Store::changed_by), `blame`) falls back to
//! recomputing the change set from the two states, so a missing or stale entry
//! is safe. [`Store::rebuild_index`](crate::Store::rebuild_index) rewrites the
//! whole table from history.

use std::collections::BTreeMap;

use redb::ReadableTable;
use serde::{Deserialize, Serialize};

use crate::error::{MnemError, Result};
use crate::id::ObjectId;
use crate::object::Object;
use crate::objects;
use crate::store::Store;

/// `commit id -> the node ids it changed, each tagged with how`.
pub(crate) const COMMIT_NODES: redb::TableDefinition<'static, [u8; 32], &[u8]> =
    redb::TableDefinition::new("commit_nodes");

/// How a commit changed one node id, relative to its first parent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    /// The node id was absent in the first parent's state.
    Added,
    /// The node id was present in the first parent, pointing at a different object.
    Modified,
    /// The node id was present in the first parent and is gone in this commit.
    Removed,
}

/// The change set that turns `from` into `to`, as a sorted map. The same
/// classification as [`crate::diff`], in map form.
pub(crate) fn changes_between(
    from: &BTreeMap<String, ObjectId>,
    to: &BTreeMap<String, ObjectId>,
) -> BTreeMap<String, ChangeKind> {
    let mut out = BTreeMap::new();
    for (id, new) in to {
        match from.get(id) {
            None => {
                out.insert(id.clone(), ChangeKind::Added);
            }
            Some(old) if old != new => {
                out.insert(id.clone(), ChangeKind::Modified);
            }
            Some(_) => {}
        }
    }
    for id in from.keys() {
        if !to.contains_key(id) {
            out.insert(id.clone(), ChangeKind::Removed);
        }
    }
    out
}

/// Record a commit's change set. Called inside the commit write transaction.
pub(crate) fn put(
    txn: &redb::WriteTransaction,
    commit: ObjectId,
    changes: &BTreeMap<String, ChangeKind>,
) -> Result<()> {
    let mut bytes = Vec::new();
    ciborium::into_writer(changes, &mut bytes).map_err(|e| MnemError::Io(e.to_string()))?;
    let mut table = txn.open_table(COMMIT_NODES)?;
    table.insert(commit.as_bytes(), bytes.as_slice())?;
    Ok(())
}

/// Read a commit's recorded change set, if the index holds it.
pub(crate) fn get(
    txn: &redb::ReadTransaction,
    commit: ObjectId,
) -> Result<Option<BTreeMap<String, ChangeKind>>> {
    let table = match txn.open_table(COMMIT_NODES) {
        Ok(table) => table,
        Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    match table.get(commit.as_bytes())? {
        Some(guard) => {
            let map = ciborium::from_reader(guard.value())
                .map_err(|e| MnemError::CorruptStore(format!("bad commit_nodes entry: {e}")))?;
            Ok(Some(map))
        }
        None => Ok(None),
    }
}

impl Store {
    /// The node ids a commit changed relative to its first parent, tagged with
    /// how. Reads the index; on a miss, recomputes from the two states (a
    /// missing entry is never wrong, only slower).
    pub fn changed_by(&self, commit: ObjectId) -> Result<BTreeMap<String, ChangeKind>> {
        let txn = self.begin_read()?;
        if let Some(map) = get(&txn, commit)? {
            return Ok(map);
        }

        let Object::Commit(commit_obj) = objects::require(&txn, commit)? else {
            return Err(MnemError::InvalidRef(format!("{commit} is not a commit")));
        };
        let to = objects::require_state(&txn, commit_obj.state)?.nodes;
        let from = match commit_obj.parents.first() {
            Some(parent) => self.state_map_at_in(&txn, *parent)?,
            None => BTreeMap::new(),
        };
        Ok(changes_between(&from, &to))
    }

    /// Rebuild the whole `commit_nodes` table from history: walk every stored
    /// commit and rewrite its change set. Used on a format upgrade, when the
    /// table is missing, or in tests. Returns the number of commits indexed.
    pub fn rebuild_index(&self) -> Result<usize> {
        // Collect every commit id and its state up front, from a read txn.
        let entries: Vec<(ObjectId, BTreeMap<String, ObjectId>, Vec<ObjectId>)> = {
            let txn = self.begin_read()?;
            let table = txn.open_table(objects::OBJECTS)?;
            let mut out = Vec::new();
            for entry in table.iter()? {
                let (key, value) = entry?;
                let bytes: [u8; ObjectId::LEN] = key.value().try_into().map_err(|_| {
                    MnemError::CorruptStore("an object key is not 32 bytes".to_string())
                })?;
                let id = ObjectId::from_bytes(bytes);
                if let Object::Commit(commit) = crate::codec::decode(value.value())? {
                    let state = objects::require_state(&txn, commit.state)?.nodes;
                    out.push((id, state, commit.parents));
                }
            }
            out
        };

        let by_id: BTreeMap<ObjectId, &BTreeMap<String, ObjectId>> =
            entries.iter().map(|(id, state, _)| (*id, state)).collect();

        let txn = self.begin_write()?;
        {
            let mut table = txn.open_table(COMMIT_NODES)?;
            let keys: Vec<[u8; 32]> = table
                .iter()?
                .map(|e| e.map(|(k, _)| k.value()))
                .collect::<std::result::Result<_, _>>()?;
            for key in keys {
                table.remove(key)?;
            }
        }
        let empty = BTreeMap::new();
        for (id, state, parents) in &entries {
            let from = match parents.first().and_then(|p| by_id.get(p)) {
                Some(parent_state) => *parent_state,
                None => &empty,
            };
            put(&txn, *id, &changes_between(from, state))?;
        }
        txn.commit()?;
        Ok(entries.len())
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
            "mnem-index-{}-{}-{:?}",
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
    fn commit_records_its_change_set() {
        let root = scratch();
        let store = Store::init(&root).unwrap();

        store.stage(&node("plan", "enterprise")).unwrap();
        store.stage(&node("owner", "alice")).unwrap();
        let c1 = store.commit("one", "agent", 1_000).unwrap();

        store.stage(&node("plan", "pro")).unwrap();
        store.stage(&node("region", "eu")).unwrap();
        store.rm("owner").unwrap();
        let c2 = store.commit("two", "agent", 2_000).unwrap();

        let first = store.changed_by(c1).unwrap();
        assert_eq!(first["plan"], ChangeKind::Added);
        assert_eq!(first["owner"], ChangeKind::Added);

        let second = store.changed_by(c2).unwrap();
        assert_eq!(second["plan"], ChangeKind::Modified);
        assert_eq!(second["region"], ChangeKind::Added);
        assert_eq!(second["owner"], ChangeKind::Removed);

        // the entry is really in the table, not just recomputed
        let txn = store.begin_read().unwrap();
        assert!(get(&txn, c2).unwrap().is_some());

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn changed_by_recomputes_when_the_entry_is_missing() {
        let root = scratch();
        let store = Store::init(&root).unwrap();
        store.stage(&node("a", "1")).unwrap();
        let c1 = store.commit("one", "agent", 1_000).unwrap();

        // wipe the index
        {
            let txn = store.begin_write().unwrap();
            {
                let mut table = txn.open_table(COMMIT_NODES).unwrap();
                let keys: Vec<[u8; 32]> = table
                    .iter()
                    .unwrap()
                    .map(|e| e.unwrap().0.value())
                    .collect();
                for key in keys {
                    table.remove(key).unwrap();
                }
            }
            txn.commit().unwrap();
        }
        {
            let txn = store.begin_read().unwrap();
            assert!(get(&txn, c1).unwrap().is_none());
        }
        // still correct, via the fallback
        assert_eq!(store.changed_by(c1).unwrap()["a"], ChangeKind::Added);

        // and rebuild puts it back
        assert_eq!(store.rebuild_index().unwrap(), 1);
        let txn = store.begin_read().unwrap();
        assert!(get(&txn, c1).unwrap().is_some());

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_merge_commit_is_indexed_against_its_first_parent() {
        let root = scratch();
        let store = Store::init(&root).unwrap();
        store.stage(&node("plan", "enterprise")).unwrap();
        store.commit("base", "agent", 1_000).unwrap();
        store.branch("feature", None).unwrap();

        store.stage(&node("owner", "alice")).unwrap();
        store.commit("ours", "agent", 2_000).unwrap();

        store.checkout("feature", false).unwrap();
        store.stage(&node("region", "eu")).unwrap();
        store.commit("theirs", "agent", 3_000).unwrap();

        store.checkout("main", false).unwrap();
        let outcome = store
            .merge("feature", &BTreeMap::new(), None, None, "agent", 4_000)
            .unwrap();
        let crate::MergeOutcome::Merged(merge_id) = outcome else {
            panic!("expected a clean merge, got {outcome:?}");
        };

        // relative to the first parent (ours), the merge brought in `region`
        let changed = store.changed_by(merge_id).unwrap();
        assert_eq!(changed.get("region"), Some(&ChangeKind::Added));
        assert!(!changed.contains_key("owner"), "owner was already on ours");

        std::fs::remove_dir_all(&root).ok();
    }
}
