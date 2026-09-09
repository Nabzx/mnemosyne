//! Canonical CBOR encoding for objects (ADR-0008).
//!
//! An object is encoded as CBOR restricted to a deterministic profile, so that
//! the same logical object always produces the same bytes, and therefore the
//! same [`ObjectId`](crate::id::ObjectId). The profile is RFC 8949 §4.2 plus
//! Mnemosyne's own rules, both written up in `docs/format/`:
//!
//! - map keys are sorted bytewise by their encoded form
//! - integers are in shortest form (ciborium's default)
//! - every float is encoded as a 64-bit CBOR float, never 16 or 32 bit
//! - no indefinite-length items, no CBOR tags
//!
//! The encode path goes through [`ciborium::value::Value`] so the canonical
//! pass can reorder maps before the bytes are written.

use ciborium::value::Value;

use crate::error::{MnemError, Result};
use crate::object::Object;

/// Encode an object to its canonical bytes.
pub fn encode(object: &Object) -> Vec<u8> {
    let mut value = Value::serialized(object).expect("an Object always serialises to a CBOR value");
    canonicalise(&mut value);
    let mut bytes = Vec::new();
    ciborium::into_writer(&value, &mut bytes).expect("a CBOR value always writes");
    bytes
}

/// Decode canonical bytes back to an object. An unknown `kind` or a malformed
/// body is a [`MnemError::CorruptStore`].
pub fn decode(bytes: &[u8]) -> Result<Object> {
    ciborium::from_reader(bytes).map_err(|e| MnemError::CorruptStore(format!("bad object: {e}")))
}

/// Recursively put a CBOR value into the canonical profile: sort every map's
/// entries by the encoded form of their key, and recurse into nested maps and
/// arrays. Integers and floats are left to the writer, which already emits
/// shortest-form integers and 64-bit floats.
fn canonicalise(value: &mut Value) {
    match value {
        Value::Map(entries) => {
            for (_, v) in entries.iter_mut() {
                canonicalise(v);
            }
            entries.sort_by_cached_key(|(key, _)| encoded_key(key));
        }
        Value::Array(items) => {
            for item in items.iter_mut() {
                canonicalise(item);
            }
        }
        _ => {}
    }
}

fn encoded_key(key: &Value) -> Vec<u8> {
    let mut bytes = Vec::new();
    ciborium::into_writer(key, &mut bytes).expect("a map key always writes");
    bytes
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::id::ObjectId;
    use crate::object::{Commit, ContentKind, MemoryNode, Provenance};

    fn node(content: serde_json::Value) -> Object {
        Object::MemoryNode(MemoryNode {
            id: "n1".to_string(),
            content,
            content_kind: ContentKind::Note,
            provenance: Provenance {
                agent_step: Some("step-1".to_string()),
                ..Default::default()
            },
            event_time: Some(1_757_000_000_000),
        })
    }

    #[test]
    fn round_trips() {
        let obj = node(json!({ "plan": "Enterprise", "seats": 40 }));
        let bytes = encode(&obj);
        assert_eq!(decode(&bytes).unwrap(), obj);
    }

    #[test]
    fn encoding_is_idempotent() {
        let obj = node(json!("just a string"));
        let once = encode(&obj);
        let twice = encode(&decode(&once).unwrap());
        assert_eq!(once, twice);
    }

    #[test]
    fn content_map_key_order_does_not_change_the_bytes() {
        // Two memory nodes whose content maps carry the same pairs in a
        // different source order must encode identically.
        let a = node(json!({ "b": 2, "a": 1, "c": 3 }));
        let b = node(json!({ "c": 3, "a": 1, "b": 2 }));
        assert_eq!(encode(&a), encode(&b));
        assert_eq!(
            ObjectId::hash_canonical(&encode(&a)),
            ObjectId::hash_canonical(&encode(&b))
        );
    }

    #[test]
    fn a_commit_encodes_stably() {
        let commit = Object::Commit(Commit {
            parents: vec![ObjectId::hash_canonical(b"p")],
            state: ObjectId::hash_canonical(b"s"),
            message: "first".to_string(),
            author: "agent".to_string(),
            time: 1_757_000_000_000,
        });
        assert_eq!(encode(&commit), encode(&decode(&encode(&commit)).unwrap()));
    }

    #[test]
    fn garbage_bytes_are_corrupt_store() {
        assert!(matches!(
            decode(&[0xff, 0x00, 0x13, 0x37]),
            Err(MnemError::CorruptStore(_))
        ));
    }
}
