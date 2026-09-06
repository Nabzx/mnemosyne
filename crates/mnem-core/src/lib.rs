//! Core object model and storage engine for Mnemosyne.
//!
//! Mnemosyne is version control for AI agent memory. This crate holds the parts
//! that never touch the network and never call a model: the object model, the
//! on-disk store, the commit graph, and the deterministic operations built on
//! them.
//!
//! Phase 1 is building this out. So far: the object model ([`object`]), object
//! identity ([`id`]), the canonical encoding ([`codec`]), and the
//! content-addressed object database ([`objects`]). The store file, the ref
//! store, the commit and the log land in the tickets after this one. See
//! `ROADMAP.md`.

#![forbid(unsafe_code)]

pub mod codec;
pub mod error;
pub mod id;
pub mod object;
pub mod objects;

pub use error::{MnemError, Result};
pub use id::ObjectId;
pub use object::{Commit, ContentKind, MemoryNode, Object, Provenance, State};

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
