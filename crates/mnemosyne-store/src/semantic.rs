//! The Era 2 semantic-merge seam (issue #66; ADR-0018).
//!
//! The structural merge (ADR-0013, [`crate::merge`]) runs first and produces a
//! list of [`Conflict`] values. [`SemanticMerge`] is a second pass over the
//! conflicts it could not settle: an impl reads the node content on each side
//! and returns a [`Verdict`].
//!
//! Era 1 ships one impl, [`StructuralOnly`], which always defers back to the
//! caller, so `Store::merge` behaves exactly as before. A content-aware impl
//! that calls a model is Era 2 and lives in a separate crate (ADR-0004); this
//! module only fixes the shape it plugs into.

use crate::error::Result;
use crate::id::ObjectId;
use crate::merge::Conflict;
use crate::mergeflow::Resolution;
use crate::store::Store;

/// A content-aware resolver for the conflicts the structural merge left open
/// (ADR-0018). Called once per [`Conflict`].
pub trait SemanticMerge {
    /// Try to settle one structural conflict by reasoning about the nodes on
    /// each side. `store` is the read surface: `Store::node` loads the object
    /// on each side, and history is walkable for provenance.
    fn resolve(&self, store: &Store, conflict: &Conflict) -> Result<Verdict>;
}

/// What a [`SemanticMerge`] impl decided about one conflict (ADR-0018).
#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    /// The resolver settled it: apply this [`Resolution`] (ADR-0014).
    Resolved(Resolution),
    /// The two sides genuinely disagree and a human or another agent should see
    /// it. Carries the draft of the `Contradiction` object Era 2 stores.
    ///
    /// In Era 1 there is no stored `Contradiction` (that is `format_version` 2),
    /// so `Store::merge_with` treats this like [`Verdict::Unresolved`]: the
    /// conflict comes back to the caller.
    Contradiction(ContradictionDraft),
    /// The resolver has nothing to add. The merge falls through to the caller's
    /// `strategy`, or returns the conflict (the structural behaviour).
    Unresolved,
}

/// The in-memory precursor of the stored `Contradiction` object (ADR-0018).
///
/// `format_version` 2 turns this into a content-addressed object kind
/// referenced from the merge commit. Until then it is only ever returned from a
/// custom resolver and not persisted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContradictionDraft {
    /// The node id the two sides disagree on.
    pub id: String,
    /// The node object in the merge base, if present.
    pub base: Option<ObjectId>,
    /// The node object in ours, if present.
    pub ours: Option<ObjectId>,
    /// The node object in theirs, if present.
    pub theirs: Option<ObjectId>,
    /// Why the resolver judged the two sides incompatible.
    pub reason: String,
}

/// The Era 1 resolver: no content reasoning. Every structural conflict is
/// returned to the caller unchanged. `Store::merge` uses this.
#[derive(Debug, Clone, Copy, Default)]
pub struct StructuralOnly;

impl SemanticMerge for StructuralOnly {
    fn resolve(&self, _store: &Store, _conflict: &Conflict) -> Result<Verdict> {
        Ok(Verdict::Unresolved)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::json;

    use super::*;
    use crate::merge::ConflictKind;
    use crate::mergeflow::MergeOutcome;
    use crate::object::{ContentKind, MemoryNode, Provenance};

    fn scratch() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let base = std::env::temp_dir().join(format!(
            "mnem-semantic-{}-{}-{:?}",
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

    fn no_res() -> BTreeMap<String, Resolution> {
        BTreeMap::new()
    }

    /// A base commit on `main`, then `main` and `feature` both edit `plan`.
    fn edit_edit() -> (std::path::PathBuf, Store) {
        let root = scratch();
        let store = Store::init(&root).unwrap();
        store.stage(&node("plan", "enterprise")).unwrap();
        store.commit("base", "agent", 1_000).unwrap();
        store.branch("feature", None).unwrap();

        store.stage(&node("plan", "team")).unwrap();
        store.commit("ours", "agent", 2_000).unwrap();

        store.checkout("feature", false).unwrap();
        store.stage(&node("plan", "pro")).unwrap();
        store.commit("theirs", "agent", 3_000).unwrap();

        store.checkout("main", false).unwrap();
        (root, store)
    }

    #[test]
    fn structural_only_leaves_every_conflict_for_the_caller() {
        let (root, store) = edit_edit();
        let outcome = store
            .merge_with(
                "feature",
                &StructuralOnly,
                &no_res(),
                None,
                None,
                "agent",
                4_000,
            )
            .unwrap();
        let MergeOutcome::Conflicts(conflicts) = outcome else {
            panic!("expected the conflict to come back, got {outcome:?}");
        };
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].id, "plan");
        assert_eq!(conflicts[0].kind, ConflictKind::EditEdit);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn merge_is_merge_with_structural_only() {
        let (root_a, store_a) = edit_edit();
        let a = store_a
            .merge("feature", &no_res(), None, None, "agent", 4_000)
            .unwrap();

        let (root_b, store_b) = edit_edit();
        let b = store_b
            .merge_with(
                "feature",
                &StructuralOnly,
                &no_res(),
                None,
                None,
                "agent",
                4_000,
            )
            .unwrap();

        assert_eq!(a, b);
        std::fs::remove_dir_all(&root_a).ok();
        std::fs::remove_dir_all(&root_b).ok();
    }

    /// A resolver that takes their side for every conflict, by reading content.
    struct AlwaysTheirs;
    impl SemanticMerge for AlwaysTheirs {
        fn resolve(&self, store: &Store, conflict: &Conflict) -> Result<Verdict> {
            // prove the store handle works: load their node
            let _ = store.node(conflict.theirs.unwrap())?;
            Ok(Verdict::Resolved(Resolution::Theirs))
        }
    }

    #[test]
    fn a_resolver_verdict_settles_the_merge() {
        let (root, store) = edit_edit();
        let outcome = store
            .merge_with(
                "feature",
                &AlwaysTheirs,
                &no_res(),
                None,
                None,
                "agent",
                4_000,
            )
            .unwrap();
        assert!(matches!(outcome, MergeOutcome::Merged(_)));
        assert_eq!(
            store.working_memory().unwrap()["plan"].content,
            json!("pro")
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// A resolver that flags every conflict as a contradiction.
    struct AlwaysContradiction;
    impl SemanticMerge for AlwaysContradiction {
        fn resolve(&self, _store: &Store, conflict: &Conflict) -> Result<Verdict> {
            Ok(Verdict::Contradiction(ContradictionDraft {
                id: conflict.id.clone(),
                base: conflict.base,
                ours: conflict.ours,
                theirs: conflict.theirs,
                reason: "test".to_string(),
            }))
        }
    }

    #[test]
    fn era_1_treats_a_contradiction_verdict_as_unresolved() {
        let (root, store) = edit_edit();
        let outcome = store
            .merge_with(
                "feature",
                &AlwaysContradiction,
                &no_res(),
                None,
                None,
                "agent",
                4_000,
            )
            .unwrap();
        let MergeOutcome::Conflicts(conflicts) = outcome else {
            panic!("expected the conflict back, got {outcome:?}");
        };
        assert_eq!(conflicts.len(), 1);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn an_explicit_resolution_beats_the_resolver() {
        let (root, store) = edit_edit();
        let mut res = BTreeMap::new();
        res.insert("plan".to_string(), Resolution::Ours);
        // AlwaysTheirs would take "pro"; the explicit Ours wins with "team".
        let outcome = store
            .merge_with("feature", &AlwaysTheirs, &res, None, None, "agent", 4_000)
            .unwrap();
        assert!(matches!(outcome, MergeOutcome::Merged(_)));
        assert_eq!(
            store.working_memory().unwrap()["plan"].content,
            json!("team")
        );
        std::fs::remove_dir_all(&root).ok();
    }
}
