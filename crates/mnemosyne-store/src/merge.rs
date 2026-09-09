//! The structural three-way merge (issues #46, #47; ADR-0013, ADR-0014).
//!
//! [`merge_state_maps`] is the algorithm: one entry per node id, classified by
//! the base/ours/theirs object ids. It never looks inside a node's content;
//! that is Era 2. The `Store::merge` flow (fast-forward, the merge commit,
//! applying resolutions) builds on this in issue #47.

use std::collections::{BTreeMap, BTreeSet};

use crate::error::Result;
use crate::id::ObjectId;
use crate::store::Store;

/// How one node's change could not be merged automatically (ADR-0014).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictKind {
    /// Both sides changed the node to different objects.
    EditEdit,
    /// Ours deleted the node, theirs changed it.
    DeleteEdit,
    /// Ours changed the node, theirs deleted it.
    EditDelete,
    /// Both sides added the id, with different objects.
    AddAdd,
}

/// A node that the merge could not resolve on its own. A transient value, not a
/// stored object (ADR-0014). `base` / `ours` / `theirs` are the node object ids
/// on each side; the absent side is `None`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflict {
    /// The node id.
    pub id: String,
    /// Why it conflicts.
    pub kind: ConflictKind,
    /// The node object in the merge base, if present.
    pub base: Option<ObjectId>,
    /// The node object in ours, if present.
    pub ours: Option<ObjectId>,
    /// The node object in theirs, if present.
    pub theirs: Option<ObjectId>,
}

/// The result of merging two state maps against a base.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateMerge {
    /// The automatically merged part: node id to object id.
    pub merged: BTreeMap<String, ObjectId>,
    /// The nodes the merge could not resolve, in id order.
    pub conflicts: Vec<Conflict>,
}

impl StateMerge {
    /// Whether the merge is clean (no conflicts).
    pub fn is_clean(&self) -> bool {
        self.conflicts.is_empty()
    }
}

enum Outcome {
    /// Keep this object id (`None` means the node is absent from the result).
    Keep(Option<ObjectId>),
    /// Report a conflict of this kind.
    Conflict(ConflictKind),
}

/// The per-id three-way merge rule (ADR-0013's table). `b`, `o`, `t` are the
/// object ids in the base, ours and theirs.
fn classify(b: Option<ObjectId>, o: Option<ObjectId>, t: Option<ObjectId>) -> Outcome {
    // Both sides agree: unchanged, or a convergent change.
    if o == t {
        return Outcome::Keep(o);
    }
    // One side is unchanged from the base: take the other side's value.
    if b == o {
        return Outcome::Keep(t);
    }
    if b == t {
        return Outcome::Keep(o);
    }
    // Both sides changed, and to different things.
    match (b, o, t) {
        (Some(_), Some(_), Some(_)) => Outcome::Conflict(ConflictKind::EditEdit),
        (Some(_), None, Some(_)) => Outcome::Conflict(ConflictKind::DeleteEdit),
        (Some(_), Some(_), None) => Outcome::Conflict(ConflictKind::EditDelete),
        (None, Some(_), Some(_)) => Outcome::Conflict(ConflictKind::AddAdd),
        // Every other shape has o == t, b == o or b == t and was handled above.
        _ => unreachable!("classify: b={b:?} o={o:?} t={t:?}"),
    }
}

/// Merge `ours` and `theirs` against `base`, one node id at a time. The result
/// is deterministic: ids are visited in sorted order, so a clean merge produces
/// the same map regardless of which side is "ours".
pub fn merge_state_maps(
    base: &BTreeMap<String, ObjectId>,
    ours: &BTreeMap<String, ObjectId>,
    theirs: &BTreeMap<String, ObjectId>,
) -> StateMerge {
    let ids: BTreeSet<&String> = base
        .keys()
        .chain(ours.keys())
        .chain(theirs.keys())
        .collect();

    let mut merged = BTreeMap::new();
    let mut conflicts = Vec::new();

    for id in ids {
        let b = base.get(id).copied();
        let o = ours.get(id).copied();
        let t = theirs.get(id).copied();
        match classify(b, o, t) {
            Outcome::Keep(Some(object_id)) => {
                merged.insert(id.clone(), object_id);
            }
            Outcome::Keep(None) => {}
            Outcome::Conflict(kind) => conflicts.push(Conflict {
                id: id.clone(),
                kind,
                base: b,
                ours: o,
                theirs: t,
            }),
        }
    }

    StateMerge { merged, conflicts }
}

impl Store {
    /// Three-way merge the states of two commits against `base`, without writing
    /// anything. The `Store::merge` flow (#47) builds on this.
    pub fn merge_states(
        &self,
        base: ObjectId,
        ours: ObjectId,
        theirs: ObjectId,
    ) -> Result<StateMerge> {
        Ok(merge_state_maps(
            &self.state_map_at(base)?,
            &self.state_map_at(ours)?,
            &self.state_map_at(theirs)?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oid(byte: u8) -> ObjectId {
        ObjectId::from_bytes([byte; 32])
    }

    fn map(pairs: &[(&str, u8)]) -> BTreeMap<String, ObjectId> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), oid(*v)))
            .collect()
    }

    #[test]
    fn every_row_of_the_table() {
        // Each id exercises one row of ADR-0013's table. `opt` is base/ours/
        // theirs as Options; absence means the node is not in that map.
        struct Row {
            id: &'static str,
            b: Option<u8>,
            o: Option<u8>,
            t: Option<u8>,
        }
        let rows = [
            Row {
                id: "keep",
                b: Some(1),
                o: Some(1),
                t: Some(1),
            },
            Row {
                id: "ours-edit",
                b: Some(1),
                o: Some(2),
                t: Some(1),
            },
            Row {
                id: "theirs-edit",
                b: Some(1),
                o: Some(1),
                t: Some(2),
            },
            Row {
                id: "converge-edit",
                b: Some(1),
                o: Some(2),
                t: Some(2),
            },
            Row {
                id: "edit-edit",
                b: Some(1),
                o: Some(2),
                t: Some(3),
            },
            Row {
                id: "ours-del",
                b: Some(1),
                o: None,
                t: Some(1),
            },
            Row {
                id: "theirs-del",
                b: Some(1),
                o: Some(1),
                t: None,
            },
            Row {
                id: "both-del",
                b: Some(1),
                o: None,
                t: None,
            },
            Row {
                id: "delete-edit",
                b: Some(1),
                o: None,
                t: Some(3),
            },
            Row {
                id: "edit-delete",
                b: Some(1),
                o: Some(2),
                t: None,
            },
            Row {
                id: "ours-add",
                b: None,
                o: Some(9),
                t: None,
            },
            Row {
                id: "theirs-add",
                b: None,
                o: None,
                t: Some(8),
            },
            Row {
                id: "converge-add",
                b: None,
                o: Some(7),
                t: Some(7),
            },
            Row {
                id: "add-add",
                b: None,
                o: Some(5),
                t: Some(6),
            },
        ];

        let mut base = BTreeMap::new();
        let mut ours = BTreeMap::new();
        let mut theirs = BTreeMap::new();
        for r in &rows {
            if let Some(v) = r.b {
                base.insert(r.id.to_string(), oid(v));
            }
            if let Some(v) = r.o {
                ours.insert(r.id.to_string(), oid(v));
            }
            if let Some(v) = r.t {
                theirs.insert(r.id.to_string(), oid(v));
            }
        }

        let result = merge_state_maps(&base, &ours, &theirs);
        let m = &result.merged;

        assert_eq!(m.get("keep"), Some(&oid(1)));
        assert_eq!(m.get("ours-edit"), Some(&oid(2)));
        assert_eq!(m.get("theirs-edit"), Some(&oid(2)));
        assert_eq!(m.get("converge-edit"), Some(&oid(2)));
        assert_eq!(m.get("ours-del"), None);
        assert_eq!(m.get("theirs-del"), None);
        assert_eq!(m.get("both-del"), None);
        assert_eq!(m.get("ours-add"), Some(&oid(9)));
        assert_eq!(m.get("theirs-add"), Some(&oid(8)));
        assert_eq!(m.get("converge-add"), Some(&oid(7)));

        let kinds: BTreeMap<&str, ConflictKind> = result
            .conflicts
            .iter()
            .map(|c| (c.id.as_str(), c.kind))
            .collect();
        assert_eq!(kinds.get("edit-edit"), Some(&ConflictKind::EditEdit));
        assert_eq!(kinds.get("delete-edit"), Some(&ConflictKind::DeleteEdit));
        assert_eq!(kinds.get("edit-delete"), Some(&ConflictKind::EditDelete));
        assert_eq!(kinds.get("add-add"), Some(&ConflictKind::AddAdd));
        assert_eq!(result.conflicts.len(), 4);
        // conflicts are in id order
        let ids: Vec<&str> = result.conflicts.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["add-add", "delete-edit", "edit-delete", "edit-edit"]
        );
    }

    #[test]
    fn a_clean_merge_is_order_independent() {
        let base = map(&[("a", 1), ("b", 1)]);
        let ours = map(&[("a", 2), ("b", 1), ("c", 3)]);
        let theirs = map(&[("a", 1), ("b", 4)]);

        let forward = merge_state_maps(&base, &ours, &theirs);
        let backward = merge_state_maps(&base, &theirs, &ours);
        assert!(forward.is_clean() && backward.is_clean());
        assert_eq!(forward.merged, backward.merged);
        assert_eq!(forward.merged, map(&[("a", 2), ("b", 4), ("c", 3)]));
    }

    #[test]
    fn conflict_carries_all_three_sides() {
        let base = map(&[("x", 1)]);
        let ours = map(&[("x", 2)]);
        let theirs = map(&[("x", 3)]);
        let result = merge_state_maps(&base, &ours, &theirs);
        assert_eq!(
            result.conflicts,
            vec![Conflict {
                id: "x".to_string(),
                kind: ConflictKind::EditEdit,
                base: Some(oid(1)),
                ours: Some(oid(2)),
                theirs: Some(oid(3)),
            }]
        );
    }
}
