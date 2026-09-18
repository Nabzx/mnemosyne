//! Exporting and importing a whole store as one JSON value (issue #174).
//!
//! [`Export`] is every object ever written (the `objects` table is
//! append-only and never rewritten, per `docs/format/`, so this is a
//! complete, GC-independent backup, not just what today's refs can reach),
//! every ref, and `HEAD`. An [`ObjectId`] renders as its 64-character hex
//! form here, never as a raw byte array, so the file reads cleanly with
//! `jq` or by hand.
//!
//! This is a view for portability (backup, moving a store between machines,
//! inspecting it with other tools), not a new on-disk format: it does not
//! touch `format_version`, and importing re-derives the store from scratch
//! through the same paths [`Store::init`] and a commit would use.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::config::SUPPORTED_FORMAT_VERSION;
use crate::error::{MnemError, Result};
use crate::head::Head;
use crate::id::ObjectId;
use crate::object::{Commit, MemoryNode, Object, State};
use crate::store::Store;
use crate::{codec, objects, refs};

/// A whole store, in a JSON-friendly, portable form. See the module docs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Export {
    /// The store's on-disk format version at export time.
    pub format_version: u32,
    /// The content hash algorithm. Always `blake3` in this release.
    pub hash_algo: String,
    /// The branch a fresh store of this config would attach `HEAD` to.
    /// Informational: importing always creates a store with the default
    /// config and then sets `HEAD` to match `head` below.
    pub default_branch: String,
    /// `HEAD`'s file contents, one line, no trailing newline: `ref: <branch>`
    /// or a bare commit id.
    pub head: String,
    /// Branch name to the hex id of the commit it points at.
    pub refs: BTreeMap<String, String>,
    /// Every stored object, keyed by its own hex id.
    pub objects: BTreeMap<String, PortableObject>,
}

/// One object, the same three kinds as [`Object`], with every [`ObjectId`]
/// written as hex instead of raw bytes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PortableObject {
    /// A memory node. Carries no object ids itself, so it is reused as-is.
    MemoryNode(MemoryNode),
    /// A state: node id to the hex id of that node's object.
    State {
        /// Node id to the hex object id of that node.
        nodes: BTreeMap<String, String>,
    },
    /// A commit, with its parents and state as hex ids.
    Commit {
        /// Parent commit ids, hex, in order.
        parents: Vec<String>,
        /// The hex id of this commit's state.
        state: String,
        /// The commit message.
        message: String,
        /// Who or what made the commit.
        author: String,
        /// The record time, Unix milliseconds.
        time: i64,
    },
}

fn parse_hex(s: &str) -> Result<ObjectId> {
    s.parse()
        .map_err(|e| MnemError::CorruptStore(format!("{s:?} is not a valid object id: {e}")))
}

impl PortableObject {
    fn from_object(object: &Object) -> Self {
        match object {
            Object::MemoryNode(node) => Self::MemoryNode(node.clone()),
            Object::State(state) => Self::State {
                nodes: state
                    .nodes
                    .iter()
                    .map(|(id, obj)| (id.clone(), obj.to_hex()))
                    .collect(),
            },
            Object::Commit(commit) => Self::Commit {
                parents: commit.parents.iter().map(ObjectId::to_hex).collect(),
                state: commit.state.to_hex(),
                message: commit.message.clone(),
                author: commit.author.clone(),
                time: commit.time,
            },
        }
    }

    fn into_object(self) -> Result<Object> {
        Ok(match self {
            Self::MemoryNode(node) => Object::MemoryNode(node),
            Self::State { nodes } => Object::State(State {
                nodes: nodes
                    .into_iter()
                    .map(|(id, hex)| Ok((id, parse_hex(&hex)?)))
                    .collect::<Result<_>>()?,
            }),
            Self::Commit {
                parents,
                state,
                message,
                author,
                time,
            } => Object::Commit(Commit {
                parents: parents
                    .iter()
                    .map(|s| parse_hex(s))
                    .collect::<Result<_>>()?,
                state: parse_hex(&state)?,
                message,
                author,
                time,
            }),
        })
    }
}

impl Store {
    /// Export the whole store: every object ever written, every ref, and
    /// `HEAD`, as a JSON-friendly [`Export`].
    pub fn export(&self) -> Result<Export> {
        let txn = self.begin_read()?;

        let mut object_map = BTreeMap::new();
        for id in objects::ids_with_prefix(&txn, "")? {
            let object = objects::require(&txn, id)?;
            object_map.insert(id.to_hex(), PortableObject::from_object(&object));
        }

        let refs_map = refs::list(&txn)?
            .into_iter()
            .map(|(name, id)| (name, id.to_hex()))
            .collect();

        Ok(Export {
            format_version: self.config().format_version,
            hash_algo: self.config().hash_algo.clone(),
            default_branch: self.config().default_branch.clone(),
            head: self.head()?.to_file_string().trim().to_string(),
            refs: refs_map,
            objects: object_map,
        })
    }

    /// Create a new store at `root` from an [`Export`], the reverse of
    /// [`Store::export`]. Refuses if `root` already holds a store, the same
    /// rule as [`Store::init`].
    ///
    /// Every object's claimed id is re-hashed from its own bytes and checked
    /// against the key it was filed under, so a hand-edited export is a
    /// clean [`MnemError::CorruptStore`], not a silent misread. Rebuilds the
    /// derived `commit_nodes` index before returning, so `blame` and
    /// `bisect` are immediately fast, not just eventually correct.
    pub fn import(root: impl AsRef<Path>, export: &Export) -> Result<Self> {
        if export.hash_algo != "blake3" {
            return Err(MnemError::CorruptStore(format!(
                "export uses hash_algo {:?}, this release only reads blake3",
                export.hash_algo
            )));
        }
        if export.format_version > SUPPORTED_FORMAT_VERSION {
            return Err(MnemError::FormatVersion {
                found: export.format_version,
                supported: SUPPORTED_FORMAT_VERSION,
            });
        }

        let store = Self::init(root)?;

        let txn = store.begin_write()?;
        for (hex_id, portable) in &export.objects {
            let claimed = parse_hex(hex_id)?;
            let object = portable.clone().into_object()?;
            let bytes = codec::encode(&object);
            let actual = ObjectId::hash_canonical(&bytes);
            if actual != claimed {
                return Err(MnemError::CorruptStore(format!(
                    "object {hex_id} does not hash to its own id (got {actual}); \
                     the export is corrupt or was hand-edited"
                )));
            }
            let mut table = txn.open_table(objects::OBJECTS)?;
            table.insert(claimed.as_bytes().as_slice(), bytes.as_slice())?;
        }
        for (name, hex_id) in &export.refs {
            refs::set(&txn, name, parse_hex(hex_id)?)?;
        }
        txn.commit()?;

        store.set_head(&Head::parse(&format!("{}\n", export.head))?)?;
        store.rebuild_index()?;

        Ok(store)
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
            "mnem-portable-{}-{}-{:?}",
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
            provenance: Provenance {
                source: Some("ticket-4821".to_string()),
                ..Default::default()
            },
            event_time: None,
        }
    }

    /// A small history: two commits, a branch, on the original store.
    fn seeded() -> (std::path::PathBuf, Store) {
        let root = scratch();
        let store = Store::init(&root).unwrap();
        store.stage(&node("plan", "enterprise")).unwrap();
        store.commit("open the case", "agent", 1_000).unwrap();
        store.branch("feature", None).unwrap();
        store.stage(&node("plan", "pro")).unwrap();
        store.commit("try pro", "agent", 2_000).unwrap();
        (root, store)
    }

    #[test]
    fn object_ids_are_hex_strings_not_byte_arrays() {
        let (root, store) = seeded();
        let export = store.export().unwrap();
        let json = serde_json::to_value(&export).unwrap();
        for key in export.objects.keys() {
            assert_eq!(key.len(), 64, "not a 64-char hex id: {key}");
            assert!(key.chars().all(|c| c.is_ascii_hexdigit()));
        }
        // every id embedded inside an object is a JSON string, never an array
        let text = json.to_string();
        assert!(
            !text.contains("],\"kind\""),
            "an id leaked as an array: {text}"
        );

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn export_then_import_round_trips_working_memory_and_history() {
        let (root, store) = seeded();
        let export = store.export().unwrap();

        let restored_root = scratch();
        let restored = Store::import(&restored_root, &export).unwrap();

        assert_eq!(
            restored.working_memory().unwrap()["plan"].content,
            json!("pro")
        );
        assert_eq!(restored.branches().unwrap().len(), 2);
        let tip = restored.head_commit().unwrap().unwrap();
        assert_eq!(restored.log(tip, false, None).unwrap().len(), 2);

        // blame still works: the derived index was rebuilt on import.
        let blame = restored.blame("plan", None).unwrap();
        assert_eq!(blame.node.content, json!("pro"));

        std::fs::remove_dir_all(&root).ok();
        std::fs::remove_dir_all(&restored_root).ok();
    }

    #[test]
    fn export_then_reexport_is_byte_identical() {
        let (root, store) = seeded();
        let export = store.export().unwrap();

        let restored_root = scratch();
        let restored = Store::import(&restored_root, &export).unwrap();
        let reexport = restored.export().unwrap();

        assert_eq!(export, reexport);

        std::fs::remove_dir_all(&root).ok();
        std::fs::remove_dir_all(&restored_root).ok();
    }

    #[test]
    fn a_tampered_object_id_is_a_corrupt_store_error() {
        let (root, store) = seeded();
        let mut export = store.export().unwrap();
        // flip a byte in some object's claimed key without touching its body
        let key = export.objects.keys().next().unwrap().clone();
        let object = export.objects.remove(&key).unwrap();
        let mut tampered = key.clone();
        tampered.replace_range(0..1, if &key[0..1] == "0" { "1" } else { "0" });
        export.objects.insert(tampered, object);

        let restored_root = scratch();
        let result = Store::import(&restored_root, &export);
        assert!(matches!(result, Err(MnemError::CorruptStore(_))));

        std::fs::remove_dir_all(&root).ok();
        std::fs::remove_dir_all(&restored_root).ok();
    }

    #[test]
    fn import_refuses_an_existing_store() {
        let (root, store) = seeded();
        let export = store.export().unwrap();
        // importing into the same root mnem init already used must refuse
        let result = Store::import(&root, &export);
        assert!(matches!(result, Err(MnemError::StoreExists(_))));
        std::fs::remove_dir_all(&root).ok();
    }
}
