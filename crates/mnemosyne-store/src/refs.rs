//! The ref store (ADR-0009).
//!
//! A ref is a branch: a mutable `name -> commit id` pointer, held in the `refs`
//! table. These functions operate inside a transaction the caller owns, the
//! same shape as [`objects`](crate::objects). `HEAD` is handled separately, in
//! [`head`](crate::head), because it is a text file.

use redb::{ReadableTable, TableDefinition};

use crate::error::{MnemError, Result};
use crate::id::ObjectId;

/// The ref table: branch name to commit id.
pub const REFS: TableDefinition<'static, &str, [u8; 32]> = TableDefinition::new("refs");

/// Check a branch name against the ADR-0009 rules.
pub fn validate_name(name: &str) -> Result<()> {
    let invalid = |why: &str| Err(MnemError::InvalidRef(format!("{name:?}: {why}")));

    if name.is_empty() {
        return invalid("empty");
    }
    if name == "HEAD" {
        return invalid("HEAD is reserved");
    }
    if name.starts_with('/') || name.ends_with('/') {
        return invalid("must not start or end with '/'");
    }
    if name.starts_with('.') || name.ends_with('.') {
        return invalid("must not start or end with '.'");
    }
    if name.contains("..") {
        return invalid("must not contain '..'");
    }
    for c in name.chars() {
        if c.is_whitespace() || c.is_control() {
            return invalid("must not contain whitespace or control characters");
        }
        if !(c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | '/')) {
            return invalid("only letters, digits, '-', '_', '.', '/' are allowed");
        }
    }
    Ok(())
}

/// Every ref, sorted by name.
pub fn list(txn: &redb::ReadTransaction) -> Result<Vec<(String, ObjectId)>> {
    let table = match txn.open_table(REFS) {
        Ok(table) => table,
        Err(redb::TableError::TableDoesNotExist(_)) => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    let mut out = Vec::new();
    for entry in table.iter()? {
        let (name, id) = entry?;
        out.push((name.value().to_string(), ObjectId::from_bytes(id.value())));
    }
    Ok(out)
}

/// The commit a ref points at, or `None` if the ref does not exist.
pub fn get(txn: &redb::ReadTransaction, name: &str) -> Result<Option<ObjectId>> {
    let table = match txn.open_table(REFS) {
        Ok(table) => table,
        Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let target = table.get(name)?.map(|guard| guard.value());
    Ok(target.map(ObjectId::from_bytes))
}

/// Create or move a ref, unconditionally. The name is validated.
pub fn set(txn: &redb::WriteTransaction, name: &str, target: ObjectId) -> Result<()> {
    validate_name(name)?;
    let mut table = txn.open_table(REFS)?;
    table.insert(name, target.as_bytes())?;
    Ok(())
}

/// Move a ref only if it currently points where `expected` says (ADR-0009's
/// compare-and-swap). `expected` of `None` means the ref must not exist yet.
/// A mismatch is [`MnemError::Conflict`].
pub fn compare_and_set(
    txn: &redb::WriteTransaction,
    name: &str,
    expected: Option<ObjectId>,
    target: ObjectId,
) -> Result<()> {
    validate_name(name)?;
    let mut table = txn.open_table(REFS)?;
    let current = table.get(name)?.map(|guard| guard.value());
    let current = current.map(ObjectId::from_bytes);
    if current != expected {
        return Err(MnemError::Conflict(format!(
            "ref {name:?}: expected {expected:?}, found {current:?}"
        )));
    }
    table.insert(name, target.as_bytes())?;
    Ok(())
}

/// Delete a ref. Returns whether it existed. This is the primitive; the policy
/// (refusing to delete the branch `HEAD` is on, or the last branch) lives in
/// the command layer.
pub fn delete(txn: &redb::WriteTransaction, name: &str) -> Result<bool> {
    let mut table = txn.open_table(REFS)?;
    let existed = table.remove(name)?.is_some();
    Ok(existed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> redb::Database {
        redb::Database::builder()
            .create_with_backend(redb::backends::InMemoryBackend::new())
            .unwrap()
    }

    fn id(seed: &[u8]) -> ObjectId {
        ObjectId::hash_canonical(seed)
    }

    #[test]
    fn set_get_list_delete() {
        let db = db();
        let (a, b) = (id(b"a"), id(b"b"));

        {
            let txn = db.begin_write().unwrap();
            set(&txn, "main", a).unwrap();
            set(&txn, "feature/x", b).unwrap();
            txn.commit().unwrap();
        }

        {
            let txn = db.begin_read().unwrap();
            assert_eq!(get(&txn, "main").unwrap(), Some(a));
            assert_eq!(get(&txn, "nope").unwrap(), None);
            assert_eq!(
                list(&txn).unwrap(),
                vec![("feature/x".to_string(), b), ("main".to_string(), a)]
            );
        }

        let txn = db.begin_write().unwrap();
        assert!(delete(&txn, "main").unwrap());
        assert!(!delete(&txn, "main").unwrap());
        txn.commit().unwrap();
    }

    #[test]
    fn compare_and_set_enforces_the_expected_value() {
        let db = db();
        let (a, b, c) = (id(b"a"), id(b"b"), id(b"c"));

        let txn = db.begin_write().unwrap();
        compare_and_set(&txn, "main", None, a).unwrap();
        compare_and_set(&txn, "main", Some(a), b).unwrap();
        assert!(matches!(
            compare_and_set(&txn, "main", Some(a), c),
            Err(MnemError::Conflict(_))
        ));
        assert!(matches!(
            compare_and_set(&txn, "main", None, c),
            Err(MnemError::Conflict(_))
        ));
        txn.commit().unwrap();
    }

    #[test]
    fn list_is_empty_on_a_fresh_store() {
        let db = db();
        let txn = db.begin_read().unwrap();
        assert!(list(&txn).unwrap().is_empty());
        assert_eq!(get(&txn, "main").unwrap(), None);
    }

    #[test]
    fn name_rules() {
        for ok in ["main", "feature/x", "a-b_c.d", "café", "v1.2"] {
            assert!(validate_name(ok).is_ok(), "{ok} should be valid");
        }
        for bad in [
            "", "HEAD", "/x", "x/", ".x", "x.", "a..b", "a b", "a\tb", "a~b", "a:b",
        ] {
            assert!(validate_name(bad).is_err(), "{bad:?} should be invalid");
        }
    }
}
