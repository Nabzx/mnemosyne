//! Core object model and storage engine for Mnemosyne.
//!
//! Mnemosyne is version control for AI agent memory. This crate holds the
//! parts that never touch the network and never call a model: the object
//! model, the on-disk store, the commit graph, and the deterministic
//! operations built on top of them.
//!
//! Nothing here is implemented yet. Phase 0 ships the workspace and the
//! shared vocabulary; the object store lands in Phase 1. See `ROADMAP.md`.

#![forbid(unsafe_code)]

/// The crate version, taken from `Cargo.toml` at build time.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Returns the Mnemosyne core version.
///
/// This is a placeholder so the workspace has something to compile and
/// test in Phase 0. It is replaced by real API in Phase 1.
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
