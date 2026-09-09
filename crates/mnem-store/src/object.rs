//! The object model: the three kinds of thing a store holds.
//!
//! Every object is a self-describing map with a `kind` tag (ADR-0002), encoded
//! as deterministic CBOR (ADR-0008) and identified by the BLAKE3 hash of its
//! canonical bytes (ADR-0005). An [`Object`] is the tagged union; code that
//! knows which kind it holds works with [`MemoryNode`], [`State`] or [`Commit`]
//! directly.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::id::ObjectId;

/// Any object a store holds. Serialises with a `kind` tag, so an unknown kind
/// is a clean deserialisation error rather than a silent misread.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Object {
    /// A memory node.
    MemoryNode(MemoryNode),
    /// A state: the set of memory nodes visible at a commit.
    State(State),
    /// A commit.
    Commit(Commit),
}

impl Object {
    /// The kind name, matching the serialised `kind` tag.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::MemoryNode(_) => "memory_node",
            Self::State(_) => "state",
            Self::Commit(_) => "commit",
        }
    }
}

/// What a memory node's `content` is shaped as (ADR-0003). v0.1 only writes
/// and reads `Note`; `Claim` is defined so v2 can add it without a format break.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentKind {
    /// Freeform content.
    Note,
    /// Content shaped as a claim. Not built in v0.1.
    Claim,
}

/// How a memory node came to exist (ADR-0003). Read by `blame`. Every field is
/// optional and an entirely empty provenance is legal.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    /// Which step of the run produced the node.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub agent_step: Option<String>,
    /// The observation or input the node was drawn from.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub observation: Option<String>,
    /// The tool call that produced the node, if any.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tool_call: Option<String>,
    /// An external source identifier, if any.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub source: Option<String>,
    /// Free text for anything the fields above do not fit.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub note: Option<String>,
}

impl Provenance {
    /// True when no field is set.
    pub fn is_empty(&self) -> bool {
        self.agent_step.is_none()
            && self.observation.is_none()
            && self.tool_call.is_none()
            && self.source.is_none()
            && self.note.is_none()
    }
}

/// The versioned unit (ADR-0003).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryNode {
    /// The stable logical key. Successive writes with the same `id` are updates
    /// to one logical node.
    pub id: String,
    /// Freeform content: a string or any JSON value, stored verbatim. Finite
    /// numbers only.
    pub content: Value,
    /// The shape of `content`.
    pub content_kind: ContentKind,
    /// How the node came to exist. Omitted from the encoding when empty.
    #[serde(skip_serializing_if = "Provenance::is_empty", default)]
    pub provenance: Provenance,
    /// When the agent formed the node, Unix milliseconds. Distinct from a
    /// commit's record time.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub event_time: Option<i64>,
}

/// The set of memory nodes visible at a commit (ADR-0003). v0.1 is `flat`: a
/// sorted map from a node's `id` to that node's [`ObjectId`]. The prolly-tree
/// form arrives in Phase 2 as a second kind.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    /// Node id to node object id, kept sorted by key.
    pub nodes: BTreeMap<String, ObjectId>,
}

/// A commit (ADR-0005). Its hashed bytes are exactly these fields; the
/// signature, if any, lives in a side table and is not part of the id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Commit {
    /// Parent commit ids: empty for the first commit, one normally, two or more
    /// for a merge. Order is significant.
    pub parents: Vec<ObjectId>,
    /// The state this commit points at.
    pub state: ObjectId,
    /// Freeform commit message.
    pub message: String,
    /// Who or what made the commit. Opaque to the core.
    pub author: String,
    /// The record time, Unix milliseconds, set at commit.
    pub time: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_node() -> MemoryNode {
        MemoryNode {
            id: "customer-4821-plan".to_string(),
            content: Value::String("the customer is on the Enterprise plan".to_string()),
            content_kind: ContentKind::Note,
            provenance: Provenance {
                agent_step: Some("step-14".to_string()),
                source: Some("ticket-4821".to_string()),
                ..Default::default()
            },
            event_time: Some(1_757_160_000_000),
        }
    }

    #[test]
    fn object_kind_matches_the_tag() {
        assert_eq!(Object::MemoryNode(sample_node()).kind(), "memory_node");
        assert_eq!(Object::State(State::default()).kind(), "state");
    }

    #[test]
    fn node_round_trips_through_cbor() {
        let node = Object::MemoryNode(sample_node());
        let mut bytes = Vec::new();
        ciborium::into_writer(&node, &mut bytes).unwrap();
        let back: Object = ciborium::from_reader(bytes.as_slice()).unwrap();
        assert_eq!(node, back);
    }

    #[test]
    fn commit_round_trips_through_cbor() {
        let commit = Object::Commit(Commit {
            parents: vec![ObjectId::hash_canonical(b"parent")],
            state: ObjectId::hash_canonical(b"state"),
            message: "learn the plan tier from the support ticket".to_string(),
            author: "demo-agent".to_string(),
            time: 1_757_160_000_000,
        });
        let mut bytes = Vec::new();
        ciborium::into_writer(&commit, &mut bytes).unwrap();
        let back: Object = ciborium::from_reader(bytes.as_slice()).unwrap();
        assert_eq!(commit, back);
    }

    #[test]
    fn empty_provenance_is_skipped() {
        let node = MemoryNode {
            provenance: Provenance::default(),
            ..sample_node()
        };
        assert!(node.provenance.is_empty());
        let mut bytes = Vec::new();
        ciborium::into_writer(&Object::MemoryNode(node.clone()), &mut bytes).unwrap();
        let back: Object = ciborium::from_reader(bytes.as_slice()).unwrap();
        assert_eq!(Object::MemoryNode(node), back);
    }

    #[test]
    fn unknown_kind_is_rejected() {
        // A map with kind "wormhole" is not a known object.
        let mut bytes = Vec::new();
        ciborium::into_writer(
            &serde_json::json!({ "kind": "wormhole", "id": "x" }),
            &mut bytes,
        )
        .unwrap();
        let result: Result<Object, _> = ciborium::from_reader(bytes.as_slice());
        assert!(result.is_err());
    }
}
