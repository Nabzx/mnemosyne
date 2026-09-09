//! The staging area (issue #26).
//!
//! `mnem add` and `mnem commit` are separate processes, so the set of memory
//! nodes waiting to be committed is persisted in its own `redb` table, keyed by
//! node id. Staging holds the node's [`ObjectId`]; the node object itself is
//! already in the `objects` table by the time it is staged.

use redb::{ReadableTable, TableDefinition};

use crate::error::Result;
use crate::id::ObjectId;

/// The staging table: node id to node object id.
pub const STAGING: TableDefinition<'static, &str, [u8; 32]> = TableDefinition::new("staging");

/// The staged-deletion set: node ids the next commit drops from the state
/// (ADR-0012). Local working state, like [`STAGING`]; not part of the portable
/// history and does not travel with a store.
pub const STAGING_TOMBSTONES: TableDefinition<'static, &str, ()> =
    TableDefinition::new("staging_tombstones");

/// Record a staged node. Overwrites an earlier staging of the same id.
pub fn put(txn: &redb::WriteTransaction, node_id: &str, object_id: ObjectId) -> Result<()> {
    let mut table = txn.open_table(STAGING)?;
    table.insert(node_id, object_id.as_bytes())?;
    Ok(())
}

/// Remove a node from staging. Returns whether it was staged.
pub fn remove(txn: &redb::WriteTransaction, node_id: &str) -> Result<bool> {
    let mut table = txn.open_table(STAGING)?;
    let existed = table.remove(node_id)?.is_some();
    Ok(existed)
}

/// Every staged node, sorted by id.
pub fn list(txn: &redb::ReadTransaction) -> Result<Vec<(String, ObjectId)>> {
    let table = match txn.open_table(STAGING) {
        Ok(table) => table,
        Err(redb::TableError::TableDoesNotExist(_)) => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    collect(&table)
}

/// The same, read from within a write transaction (the table is created if
/// absent, which is harmless).
pub fn list_in_write(txn: &redb::WriteTransaction) -> Result<Vec<(String, ObjectId)>> {
    let table = txn.open_table(STAGING)?;
    collect(&table)
}

fn collect<T: ReadableTable<&'static str, [u8; 32]>>(table: &T) -> Result<Vec<(String, ObjectId)>> {
    let mut out = Vec::new();
    for entry in table.iter()? {
        let (id, object_id) = entry?;
        out.push((
            id.value().to_string(),
            ObjectId::from_bytes(object_id.value()),
        ));
    }
    Ok(out)
}

/// Empty the staging table.
pub fn clear(txn: &redb::WriteTransaction) -> Result<()> {
    let mut table = txn.open_table(STAGING)?;
    let ids: Vec<String> = {
        let mut ids = Vec::new();
        for entry in table.iter()? {
            ids.push(entry?.0.value().to_string());
        }
        ids
    };
    for id in ids {
        table.remove(id.as_str())?;
    }
    Ok(())
}

/// Add a node id to the staged-deletion set.
pub fn tombstone(txn: &redb::WriteTransaction, node_id: &str) -> Result<()> {
    let mut table = txn.open_table(STAGING_TOMBSTONES)?;
    table.insert(node_id, ())?;
    Ok(())
}

/// Remove a node id from the staged-deletion set. Returns whether it was set.
pub fn untombstone(txn: &redb::WriteTransaction, node_id: &str) -> Result<bool> {
    let mut table = txn.open_table(STAGING_TOMBSTONES)?;
    let existed = table.remove(node_id)?.is_some();
    Ok(existed)
}

/// Every staged deletion, sorted by id.
pub fn tombstones(txn: &redb::ReadTransaction) -> Result<Vec<String>> {
    let table = match txn.open_table(STAGING_TOMBSTONES) {
        Ok(table) => table,
        Err(redb::TableError::TableDoesNotExist(_)) => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    collect_names(&table)
}

/// The same, from within a write transaction.
pub fn tombstones_in_write(txn: &redb::WriteTransaction) -> Result<Vec<String>> {
    let table = txn.open_table(STAGING_TOMBSTONES)?;
    collect_names(&table)
}

fn collect_names<T: ReadableTable<&'static str, ()>>(table: &T) -> Result<Vec<String>> {
    let mut out = Vec::new();
    for entry in table.iter()? {
        out.push(entry?.0.value().to_string());
    }
    Ok(out)
}

/// Empty the staged-deletion set.
pub fn clear_tombstones(txn: &redb::WriteTransaction) -> Result<()> {
    let mut table = txn.open_table(STAGING_TOMBSTONES)?;
    let ids: Vec<String> = {
        let mut ids = Vec::new();
        for entry in table.iter()? {
            ids.push(entry?.0.value().to_string());
        }
        ids
    };
    for id in ids {
        table.remove(id.as_str())?;
    }
    Ok(())
}
