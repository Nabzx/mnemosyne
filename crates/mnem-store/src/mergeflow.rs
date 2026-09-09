//! `Store::merge`: the merge flow (issue #47; ADR-0013, ADR-0014).
//!
//! Finds the merge base, detects fast-forward, runs the three-way merge
//! ([`merge_state_maps`](crate::merge::merge_state_maps)), applies the caller's
//! resolutions, and, when the merge is clean, writes the merged `State` and a
//! two-parent `Commit` in one write transaction. Stateless: a merge that still
//! has conflicts writes nothing, and the caller calls `merge` again with a
//! fuller resolution map.

use std::collections::{BTreeMap, BTreeSet};

use crate::codec;
use crate::error::{MnemError, Result};
use crate::head::Head;
use crate::id::ObjectId;
use crate::merge::{Conflict, StateMerge};
use crate::object::{Commit, MemoryNode, Object, State};
use crate::semantic::{SemanticMerge, StructuralOnly, Verdict};
use crate::store::Store;
use crate::{objects, refs};

/// How to resolve one conflicting node (ADR-0014).
#[derive(Debug, Clone, PartialEq)]
pub enum Resolution {
    /// Take our side's object (or the deletion, if we deleted it).
    Ours,
    /// Take their side's object (or the deletion).
    Theirs,
    /// Take the merge base's object, reverting both edits. Invalid for an
    /// add/add conflict, which has no base.
    Base,
    /// Drop the node from the merged state.
    Delete,
    /// Use this node as the resolution.
    Set(MemoryNode),
}

/// Resolve every conflict one way (ADR-0014). An explicit [`Resolution`]
/// overrides the strategy for that id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeStrategy {
    /// Every conflict resolves to our side.
    Ours,
    /// Every conflict resolves to their side.
    Theirs,
}

/// What [`Store::merge`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeOutcome {
    /// Theirs was already an ancestor of our branch; nothing to do.
    AlreadyUpToDate,
    /// Our branch had nothing theirs did not; the branch moved to this commit.
    FastForwarded(ObjectId),
    /// A merge commit was written.
    Merged(ObjectId),
    /// The merge has unresolved conflicts. Nothing was written.
    Conflicts(Vec<Conflict>),
}

impl Store {
    /// Merge `theirs` (a commit-ish) into the current branch.
    ///
    /// `resolutions` maps a conflicting node id to a [`Resolution`]; `strategy`
    /// resolves any conflict not in the map. Refused from a detached `HEAD` or
    /// with a dirty index (ADR-0013).
    ///
    /// This is [`merge_with`](Self::merge_with) with the Era 1 resolver
    /// ([`StructuralOnly`]): the structural merge is the whole merge.
    pub fn merge(
        &self,
        theirs: &str,
        resolutions: &BTreeMap<String, Resolution>,
        strategy: Option<MergeStrategy>,
        message: Option<&str>,
        author: &str,
        time_ms: i64,
    ) -> Result<MergeOutcome> {
        self.merge_with(
            theirs,
            &StructuralOnly,
            resolutions,
            strategy,
            message,
            author,
            time_ms,
        )
    }

    /// Merge `theirs` into the current branch, running `resolver` over the
    /// conflicts the structural merge leaves open (ADR-0018).
    ///
    /// Per conflicting id the precedence is: an explicit `resolutions` entry,
    /// then the resolver's [`Verdict`], then `strategy`, then unresolved. A
    /// [`Verdict::Contradiction`] is treated as [`Verdict::Unresolved`] in
    /// Era 1: the stored `Contradiction` object is `format_version` 2.
    #[allow(clippy::too_many_arguments)]
    pub fn merge_with(
        &self,
        theirs: &str,
        resolver: &dyn SemanticMerge,
        resolutions: &BTreeMap<String, Resolution>,
        strategy: Option<MergeStrategy>,
        message: Option<&str>,
        author: &str,
        time_ms: i64,
    ) -> Result<MergeOutcome> {
        let Head::Attached(branch) = self.head()? else {
            return Err(MnemError::InvalidRef(
                "cannot merge from a detached HEAD; check out a branch first".to_string(),
            ));
        };
        if !self.staged()?.is_empty() || !self.staged_deletions()?.is_empty() {
            return Err(MnemError::InvalidRef(
                "cannot merge with a dirty index; commit or unstage first".to_string(),
            ));
        }

        let theirs_tip = self.resolve_commitish(theirs)?;

        let ours_tip = {
            let txn = self.begin_read()?;
            refs::get(&txn, &branch)?
        };
        let Some(ours_tip) = ours_tip else {
            // an unborn branch: adopt theirs wholesale
            let txn = self.begin_write()?;
            refs::compare_and_set(&txn, &branch, None, theirs_tip)?;
            txn.commit()?;
            return Ok(MergeOutcome::FastForwarded(theirs_tip));
        };

        if theirs_tip == ours_tip || self.is_ancestor(theirs_tip, ours_tip)? {
            return Ok(MergeOutcome::AlreadyUpToDate);
        }
        if self.is_ancestor(ours_tip, theirs_tip)? {
            let txn = self.begin_write()?;
            refs::compare_and_set(&txn, &branch, Some(ours_tip), theirs_tip)?;
            txn.commit()?;
            return Ok(MergeOutcome::FastForwarded(theirs_tip));
        }

        let base = self.merge_base(ours_tip, theirs_tip)?;
        let state_merge = self.merge_states(base, ours_tip, theirs_tip)?;

        let Resolved {
            merged,
            set_nodes,
            unresolved,
        } = resolve(self, resolver, state_merge, resolutions, strategy)?;
        if !unresolved.is_empty() {
            return Ok(MergeOutcome::Conflicts(unresolved));
        }

        let ours_branch_label = branch.clone();
        let default_message = format!("merge {theirs} into {ours_branch_label}");
        let message = message.unwrap_or(&default_message);

        // The merge commit is indexed against its first parent, `ours`.
        let ours_map = self.state_map_at(ours_tip)?;
        let changes = crate::index::changes_between(&ours_map, &merged);

        let txn = self.begin_write()?;
        for node in &set_nodes {
            objects::put(&txn, &Object::MemoryNode(node.clone()))?;
        }
        let state_id = objects::put(&txn, &Object::State(State { nodes: merged }))?;
        let commit = Commit {
            parents: vec![ours_tip, theirs_tip],
            state: state_id,
            message: message.to_string(),
            author: author.to_string(),
            time: time_ms,
        };
        let commit_id = objects::put(&txn, &Object::Commit(commit))?;
        crate::index::put(&txn, commit_id, &changes)?;
        refs::compare_and_set(&txn, &branch, Some(ours_tip), commit_id)?;
        txn.commit()?;

        Ok(MergeOutcome::Merged(commit_id))
    }
}

/// The output of [`resolve`].
struct Resolved {
    /// The final merged map.
    merged: BTreeMap<String, ObjectId>,
    /// `Set` resolution nodes that still need writing to the object store.
    set_nodes: Vec<MemoryNode>,
    /// Conflicts left with no resolution.
    unresolved: Vec<Conflict>,
}

/// Apply `resolutions`, then `resolver`, then `strategy` to the conflicts
/// (ADR-0018). The first that settles an id wins.
fn resolve(
    store: &Store,
    resolver: &dyn SemanticMerge,
    state_merge: StateMerge,
    resolutions: &BTreeMap<String, Resolution>,
    strategy: Option<MergeStrategy>,
) -> Result<Resolved> {
    let conflict_ids: BTreeSet<&str> = state_merge
        .conflicts
        .iter()
        .map(|c| c.id.as_str())
        .collect();
    for id in resolutions.keys() {
        if !conflict_ids.contains(id.as_str()) {
            return Err(MnemError::InvalidRef(format!(
                "resolution given for {id:?}, which is not a conflict"
            )));
        }
    }

    let strategy_pick = |s: MergeStrategy| match s {
        MergeStrategy::Ours => Resolution::Ours,
        MergeStrategy::Theirs => Resolution::Theirs,
    };

    let mut merged = state_merge.merged;
    let mut set_nodes = Vec::new();
    let mut unresolved = Vec::new();

    for conflict in state_merge.conflicts {
        let picked = match resolutions.get(&conflict.id).cloned() {
            Some(explicit) => Some(explicit),
            None => match resolver.resolve(store, &conflict)? {
                Verdict::Resolved(resolution) => Some(resolution),
                // Era 1: no stored Contradiction (format_version 2). Fall
                // through to the strategy, or hand the conflict back.
                Verdict::Contradiction(_) | Verdict::Unresolved => strategy.map(strategy_pick),
            },
        };

        match picked {
            None => unresolved.push(conflict),
            Some(Resolution::Ours) => apply_side(&mut merged, &conflict.id, conflict.ours),
            Some(Resolution::Theirs) => apply_side(&mut merged, &conflict.id, conflict.theirs),
            Some(Resolution::Base) => match conflict.base {
                Some(object_id) => {
                    merged.insert(conflict.id, object_id);
                }
                None => {
                    return Err(MnemError::InvalidRef(format!(
                        "cannot resolve {:?} to base: an add/add conflict has no base",
                        conflict.id
                    )));
                }
            },
            Some(Resolution::Delete) => {
                merged.remove(&conflict.id);
            }
            Some(Resolution::Set(node)) => {
                let object_id =
                    ObjectId::hash_canonical(&codec::encode(&Object::MemoryNode(node.clone())));
                merged.insert(conflict.id.clone(), object_id);
                set_nodes.push(node);
            }
        }
    }

    Ok(Resolved {
        merged,
        set_nodes,
        unresolved,
    })
}

fn apply_side(merged: &mut BTreeMap<String, ObjectId>, id: &str, side: Option<ObjectId>) {
    match side {
        Some(object_id) => {
            merged.insert(id.to_string(), object_id);
        }
        None => {
            merged.remove(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::merge::ConflictKind;
    use crate::object::{ContentKind, Provenance};

    fn scratch() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let base = std::env::temp_dir().join(format!(
            "mnem-mergeflow-{}-{}-{:?}",
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

    /// A base commit, then diverge `main` and `feature`.
    fn diverged() -> (std::path::PathBuf, Store) {
        let root = scratch();
        let store = Store::init(&root).unwrap();
        store.stage(&node("plan", "enterprise")).unwrap();
        store.stage(&node("owner", "alice")).unwrap();
        store.commit("base", "agent", 1_000).unwrap();
        store.branch("feature", None).unwrap();
        (root, store)
    }

    #[test]
    fn a_clean_merge_writes_a_two_parent_commit() {
        let (root, store) = diverged();

        // main changes `owner`
        store.stage(&node("owner", "bob")).unwrap();
        let ours = store.commit("ours", "agent", 2_000).unwrap();

        // feature changes `plan` and adds `region`
        store.checkout("feature", false).unwrap();
        store.stage(&node("plan", "pro")).unwrap();
        store.stage(&node("region", "eu")).unwrap();
        let theirs = store.commit("theirs", "agent", 3_000).unwrap();

        store.checkout("main", false).unwrap();
        let outcome = store
            .merge("feature", &no_res(), None, None, "agent", 4_000)
            .unwrap();
        let MergeOutcome::Merged(merge_id) = outcome else {
            panic!("expected a clean merge, got {outcome:?}");
        };

        let wm = store.working_memory().unwrap();
        assert_eq!(wm["owner"].content, json!("bob"));
        assert_eq!(wm["plan"].content, json!("pro"));
        assert_eq!(wm["region"].content, json!("eu"));

        let merge_obj = {
            let txn = store.begin_read().unwrap();
            objects::require(&txn, merge_id).unwrap()
        };
        let Object::Commit(commit) = merge_obj else {
            panic!("not a commit");
        };
        assert_eq!(commit.parents, vec![ours, theirs]);
        assert_eq!(commit.message, "merge feature into main");

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn fast_forward_and_already_up_to_date() {
        let root = scratch();
        let store = Store::init(&root).unwrap();
        store.stage(&node("a", "1")).unwrap();
        store.commit("one", "agent", 1_000).unwrap();
        store.branch("feature", None).unwrap();

        store.checkout("feature", false).unwrap();
        store.stage(&node("b", "1")).unwrap();
        let ahead = store.commit("two", "agent", 2_000).unwrap();

        store.checkout("main", false).unwrap();
        assert_eq!(
            store
                .merge("feature", &no_res(), None, None, "agent", 3_000)
                .unwrap(),
            MergeOutcome::FastForwarded(ahead)
        );
        // main now equals feature
        assert_eq!(
            store
                .merge("feature", &no_res(), None, None, "agent", 4_000)
                .unwrap(),
            MergeOutcome::AlreadyUpToDate
        );

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_conflict_writes_nothing_then_a_resolution_completes_it() {
        let (root, store) = diverged();

        store.stage(&node("plan", "team")).unwrap();
        store.commit("ours", "agent", 2_000).unwrap();

        store.checkout("feature", false).unwrap();
        store.stage(&node("plan", "pro")).unwrap();
        store.commit("theirs", "agent", 3_000).unwrap();

        store.checkout("main", false).unwrap();
        let before = store.head_commit().unwrap();

        let outcome = store
            .merge("feature", &no_res(), None, None, "agent", 4_000)
            .unwrap();
        let MergeOutcome::Conflicts(conflicts) = outcome else {
            panic!("expected a conflict");
        };
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].id, "plan");
        assert_eq!(conflicts[0].kind, ConflictKind::EditEdit);
        // nothing was written
        assert_eq!(store.head_commit().unwrap(), before);

        // resolve to theirs
        let mut res = BTreeMap::new();
        res.insert("plan".to_string(), Resolution::Theirs);
        let MergeOutcome::Merged(_) = store
            .merge("feature", &res, None, None, "agent", 5_000)
            .unwrap()
        else {
            panic!("expected a merge");
        };
        assert_eq!(
            store.working_memory().unwrap()["plan"].content,
            json!("pro")
        );

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn strategy_set_and_stale_resolutions() {
        let (root, store) = diverged();

        store.stage(&node("plan", "team")).unwrap();
        store.rm("owner").unwrap(); // ours deletes owner
        store.commit("ours", "agent", 2_000).unwrap();

        store.checkout("feature", false).unwrap();
        store.stage(&node("plan", "pro")).unwrap();
        store.stage(&node("owner", "carol")).unwrap(); // theirs edits owner -> delete/edit
        store.commit("theirs", "agent", 3_000).unwrap();

        store.checkout("main", false).unwrap();

        // a stale resolution (an id that is not a conflict) is an error
        let mut stale = BTreeMap::new();
        stale.insert("plan".to_string(), Resolution::Theirs);
        stale.insert("region".to_string(), Resolution::Ours); // not a conflict
        assert!(matches!(
            store.merge("feature", &stale, None, None, "agent", 4_000),
            Err(MnemError::InvalidRef(m)) if m.contains("not a conflict")
        ));

        // per-conflict resolutions: plan to theirs, owner (delete/edit) via Set
        let mut res = BTreeMap::new();
        res.insert("plan".to_string(), Resolution::Theirs);
        res.insert("owner".to_string(), Resolution::Set(node("owner", "dave")));
        let MergeOutcome::Merged(_) = store
            .merge("feature", &res, None, None, "agent", 5_000)
            .unwrap()
        else {
            panic!("expected a merge");
        };
        let wm = store.working_memory().unwrap();
        assert_eq!(wm["plan"].content, json!("pro"));
        assert_eq!(wm["owner"].content, json!("dave"));

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn strategy_ours_resolves_every_conflict_one_way() {
        let (root, store) = diverged();

        store.stage(&node("plan", "team")).unwrap();
        store.rm("owner").unwrap();
        store.commit("ours", "agent", 2_000).unwrap();

        store.checkout("feature", false).unwrap();
        store.stage(&node("plan", "pro")).unwrap();
        store.stage(&node("owner", "carol")).unwrap();
        store.commit("theirs", "agent", 3_000).unwrap();

        store.checkout("main", false).unwrap();
        let MergeOutcome::Merged(_) = store
            .merge(
                "feature",
                &no_res(),
                Some(MergeStrategy::Ours),
                None,
                "agent",
                4_000,
            )
            .unwrap()
        else {
            panic!("expected a merge");
        };
        let wm = store.working_memory().unwrap();
        assert_eq!(wm["plan"].content, json!("team"));
        assert!(!wm.contains_key("owner")); // ours deleted it

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn merge_refuses_a_dirty_index_or_detached_head() {
        let (root, store) = diverged();
        store.stage(&node("plan", "x")).unwrap();
        assert!(matches!(
            store.merge("feature", &no_res(), None, None, "a", 1),
            Err(MnemError::InvalidRef(m)) if m.contains("dirty index")
        ));
        std::fs::remove_dir_all(&root).ok();
    }
}
