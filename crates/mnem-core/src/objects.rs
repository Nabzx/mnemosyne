//! The content-addressed object database (ADR-0002, ADR-0008).
//!
//! Objects live in one `redb` table, `objects`, keyed by the 32-byte
//! [`ObjectId`] and holding the canonical CBOR bytes. These functions operate
//! inside a transaction the caller owns; opening the store file is [`Store`]'s
//! job (issue #24).

use redb::{ReadableTable, TableDefinition};

use crate::codec;
use crate::error::{MnemError, Result};
use crate::id::ObjectId;
use crate::object::Object;

/// The object table: id to canonical bytes.
pub const OBJECTS: TableDefinition<'static, &[u8], &[u8]> = TableDefinition::new("objects");

/// Write an object into `txn` and return its id. Idempotent: writing an object
/// that is already stored does nothing.
pub fn put(txn: &redb::WriteTransaction, object: &Object) -> Result<ObjectId> {
    let bytes = codec::encode(object);
    let id = ObjectId::hash_canonical(&bytes);
    let mut table = txn.open_table(OBJECTS)?;
    if table.get(id.as_bytes().as_slice())?.is_none() {
        table.insert(id.as_bytes().as_slice(), bytes.as_slice())?;
    }
    Ok(id)
}

/// Read an object by id. `Ok(None)` if it is not stored.
pub fn get(txn: &redb::ReadTransaction, id: ObjectId) -> Result<Option<Object>> {
    let table = match txn.open_table(OBJECTS) {
        Ok(table) => table,
        Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    match table.get(id.as_bytes().as_slice())? {
        Some(guard) => Ok(Some(codec::decode(guard.value())?)),
        None => Ok(None),
    }
}

/// Read an object by id, erroring if it is not stored.
pub fn require(txn: &redb::ReadTransaction, id: ObjectId) -> Result<Object> {
    get(txn, id)?.ok_or(MnemError::NotFound(id))
}

/// Whether an object with this id is stored.
pub fn has(txn: &redb::ReadTransaction, id: ObjectId) -> Result<bool> {
    let table = match txn.open_table(OBJECTS) {
        Ok(table) => table,
        Err(redb::TableError::TableDoesNotExist(_)) => return Ok(false),
        Err(e) => return Err(e.into()),
    };
    Ok(table.get(id.as_bytes().as_slice())?.is_some())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::object::{ContentKind, MemoryNode, Provenance};

    fn db() -> redb::Database {
        redb::Database::builder()
            .create_with_backend(redb::backends::InMemoryBackend::new())
            .unwrap()
    }

    fn node(id: &str) -> Object {
        Object::MemoryNode(MemoryNode {
            id: id.to_string(),
            content: json!("a belief"),
            content_kind: ContentKind::Note,
            provenance: Provenance::default(),
            event_time: None,
        })
    }

    #[test]
    fn put_then_get_round_trips() {
        let db = db();
        let obj = node("n1");

        let id = {
            let txn = db.begin_write().unwrap();
            let id = put(&txn, &obj).unwrap();
            txn.commit().unwrap();
            id
        };

        let txn = db.begin_read().unwrap();
        assert_eq!(get(&txn, id).unwrap(), Some(obj));
        assert!(has(&txn, id).unwrap());
    }

    #[test]
    fn put_is_idempotent_and_content_addressed() {
        let db = db();
        let obj = node("n1");

        let txn = db.begin_write().unwrap();
        let first = put(&txn, &obj).unwrap();
        let second = put(&txn, &obj).unwrap();
        txn.commit().unwrap();

        assert_eq!(first, second);
        assert_eq!(first, ObjectId::hash_canonical(&codec::encode(&obj)));
    }

    #[test]
    fn missing_object_is_none_on_a_fresh_store() {
        let db = db();
        let txn = db.begin_read().unwrap();
        let missing = ObjectId::hash_canonical(b"nothing wrote this");
        assert_eq!(get(&txn, missing).unwrap(), None);
        assert!(!has(&txn, missing).unwrap());
        assert!(matches!(
            require(&txn, missing),
            Err(MnemError::NotFound(_))
        ));
    }

    #[test]
    fn two_objects_are_independent() {
        let db = db();
        let (a_id, b_id) = {
            let txn = db.begin_write().unwrap();
            let a = put(&txn, &node("a")).unwrap();
            let b = put(&txn, &node("b")).unwrap();
            txn.commit().unwrap();
            (a, b)
        };
        assert_ne!(a_id, b_id);

        let txn = db.begin_read().unwrap();
        assert!(has(&txn, a_id).unwrap());
        assert!(has(&txn, b_id).unwrap());
    }
}
