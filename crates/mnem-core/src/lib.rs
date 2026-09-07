//! Core object model and storage engine for Mnemosyne.
//!
//! Mnemosyne is version control for AI agent memory. This crate holds the parts
//! that never touch the network and never call a model: the object model, the
//! on-disk store, the commit graph, and the deterministic operations built on
//! them.
//!
//! Phase 1's core: the object model ([`object`]), object identity ([`id`]), the
//! canonical encoding ([`codec`]), the content-addressed object database
//! ([`objects`]), the store lifecycle ([`store`]), the ref store ([`refs`]) and
//! [`head`], staging plus [`commit`], and the [`log`] walk. Phase 2 adds
//! [`branch`] and, next, checkout and time travel. See `ROADMAP.md`.

#![forbid(unsafe_code)]

pub mod bisect;
pub mod blame;
pub mod branch;
pub mod checkout;
pub mod codec;
pub mod commit;
pub mod config;
pub mod diff;
pub mod error;
pub mod graph;
pub mod head;
pub mod id;
pub mod index;
pub mod log;
pub mod merge;
pub mod mergeflow;
pub mod object;
pub mod objects;
pub mod refs;
pub mod staging;
pub mod store;
pub mod timetravel;

pub use blame::Blame;
pub use checkout::Checkout;
pub use config::Config;
pub use diff::{DiffTarget, NodeChange};
pub use error::{MnemError, Result};
pub use head::Head;
pub use id::ObjectId;
pub use index::ChangeKind;
pub use merge::{Conflict, ConflictKind, StateMerge};
pub use mergeflow::{MergeOutcome, MergeStrategy, Resolution};
pub use object::{Commit, ContentKind, MemoryNode, Object, Provenance, State};
pub use store::Store;

/// The crate version, taken from `Cargo.toml` at build time.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Returns the Mnemosyne core version.
pub fn version() -> &'static str {
    VERSION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_reported() {
        assert_eq!(version(), env!("CARGO_PKG_VERSION"));
        assert!(!version().is_empty());
    }
}
