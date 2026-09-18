//! Core object model and storage engine for Mnemosyne, version control for AI
//! agent memory.
//!
//! An agent's memory becomes an immutable, content-addressed history: every
//! change is a [`commit`], memory can [`branch`] and [`merge`], any belief can
//! be traced back to the commit and observation that produced it with
//! [`blame`], and a run can be binary-searched for where a wrong belief
//! entered with [`bisect`]. [`checkout`] and [`timetravel`] materialise memory
//! as it stood at any past commit; [`diff`] and the change [`index`] show what
//! one commit changed.
//!
//! This crate never touches the network and never calls a model: [`object`]
//! and [`codec`] define the on-disk shape, [`store`] and [`objects`] are the
//! content-addressed database, [`refs`] and [`head`] track branches. The
//! `SemanticMerge` seam for Era 2's content-aware merge lives in `semantic`,
//! with a no-op implementation; nothing here calls out to reason about
//! content.
//!
//! The on-disk format is specified in `docs/format/` and frozen for the
//! `0.0.x` line at `format_version` 1.

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
pub mod portable;
pub mod refs;
pub mod semantic;
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
pub use portable::{Export, PortableObject};
pub use semantic::{ContradictionDraft, SemanticMerge, StructuralOnly, Verdict};
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
