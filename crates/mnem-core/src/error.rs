//! [`MnemError`], the one error type the core returns.
//!
//! The Python SDK maps each variant to a Python exception (ADR-0004). Adding a
//! variant here means adding a row to that mapping table.

use crate::id::ObjectId;

/// Anything that can go wrong in the core.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum MnemError {
    /// An object was asked for by id and is not in the store.
    #[error("object {0} not found")]
    NotFound(ObjectId),

    /// A ref name does not resolve, or is not a legal name.
    #[error("invalid ref: {0}")]
    InvalidRef(String),

    /// A ref moved under a compare-and-swap update (ADR-0009).
    #[error("the branch moved under you: {0}")]
    Conflict(String),

    /// An object's stored bytes could not be decoded, or an object references
    /// an id that is not present.
    #[error("corrupt store: {0}")]
    CorruptStore(String),

    /// The store's `format_version` is one this release does not support.
    #[error("unsupported format version {found}, this release supports up to {supported}")]
    FormatVersion {
        /// The version found in the store's `config`.
        found: u32,
        /// The highest version this release can read.
        supported: u32,
    },

    /// An error from the underlying storage.
    #[error("storage error: {0}")]
    Io(String),
}

/// The core's result type.
pub type Result<T> = std::result::Result<T, MnemError>;

impl From<redb::Error> for MnemError {
    fn from(e: redb::Error) -> Self {
        Self::Io(e.to_string())
    }
}

impl From<redb::DatabaseError> for MnemError {
    fn from(e: redb::DatabaseError) -> Self {
        Self::Io(e.to_string())
    }
}

impl From<redb::TransactionError> for MnemError {
    fn from(e: redb::TransactionError) -> Self {
        Self::Io(e.to_string())
    }
}

impl From<redb::TableError> for MnemError {
    fn from(e: redb::TableError) -> Self {
        Self::Io(e.to_string())
    }
}

impl From<redb::StorageError> for MnemError {
    fn from(e: redb::StorageError) -> Self {
        Self::Io(e.to_string())
    }
}

impl From<redb::CommitError> for MnemError {
    fn from(e: redb::CommitError) -> Self {
        Self::Io(e.to_string())
    }
}
