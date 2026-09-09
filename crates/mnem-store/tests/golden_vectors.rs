//! Golden vectors for the on-disk format (#32, ADR-0008).
//!
//! Each case is a canonical object, its exact CBOR bytes, and its `ObjectId`.
//! The bytes and ids are pinned here and copied into `docs/format/golden-vectors.md`.
//! If the encoder changes in a way that moves any byte, this test fails and the
//! change is deliberate: it is a `format_version` bump (ADR-0007, ADR-0010).
//!
//! Run `cargo test -p mnem-store --test golden_vectors -- --nocapture --ignored print`
//! to print the current vectors when adding a case.

use mnem_store::codec;
use mnem_store::{Commit, ContentKind, MemoryNode, Object, ObjectId, Provenance, State};
use serde_json::json;

fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(char::from_digit((b >> 4) as u32, 16).unwrap());
        s.push(char::from_digit((b & 0x0f) as u32, 16).unwrap());
    }
    s
}

fn note_minimal() -> Object {
    Object::MemoryNode(MemoryNode {
        id: "greeting".to_string(),
        content: json!("hello"),
        content_kind: ContentKind::Note,
        provenance: Provenance::default(),
        event_time: None,
    })
}

fn note_full() -> Object {
    Object::MemoryNode(MemoryNode {
        id: "customer-4821".to_string(),
        content: json!({ "plan": "enterprise", "seats": 40 }),
        content_kind: ContentKind::Note,
        provenance: Provenance {
            agent_step: Some("triage".to_string()),
            observation: Some("ticket-4821 body".to_string()),
            tool_call: Some("read-ticket".to_string()),
            source: Some("ticket-4821".to_string()),
            note: Some("first contact".to_string()),
        },
        event_time: Some(1_757_000_000_000),
    })
}

fn state_empty() -> Object {
    Object::State(State::default())
}

fn state_one() -> Object {
    let mut nodes = std::collections::BTreeMap::new();
    nodes.insert("greeting".to_string(), id_of(&note_minimal()));
    Object::State(State { nodes })
}

fn commit_root() -> Object {
    Object::Commit(Commit {
        parents: vec![],
        state: id_of(&state_one()),
        message: "first commit".to_string(),
        author: "agent".to_string(),
        time: 1_757_000_000_000,
    })
}

fn commit_child() -> Object {
    Object::Commit(Commit {
        parents: vec![id_of(&commit_root())],
        state: id_of(&state_empty()),
        message: "second".to_string(),
        author: "agent".to_string(),
        time: 1_757_000_060_000,
    })
}

fn id_of(object: &Object) -> ObjectId {
    ObjectId::hash_canonical(&codec::encode(object))
}

fn vectors() -> Vec<(&'static str, Object)> {
    vec![
        ("note_minimal", note_minimal()),
        ("note_full", note_full()),
        ("state_empty", state_empty()),
        ("state_one", state_one()),
        ("commit_root", commit_root()),
        ("commit_child", commit_child()),
    ]
}

/// The frozen vectors: name, id hex, CBOR hex. Format version 1. Mirrored in
/// `docs/format/golden-vectors.md`. Do not edit by hand; regenerate with the
/// `print` test and only when a `format_version` bump is intended.
const GOLDEN: &[(&str, &str, &str)] = &[
    (
        "note_minimal",
        "5107a6e4f686efec31950bf3171e7b4affd3c83697e8ab5182dc4a5f867cce10",
        "a4626964686772656574696e67646b696e646b6d656d6f72795f6e6f646567636f6e74656e746568656c6c6f6c636f6e74656e745f6b696e64646e6f7465",
    ),
    (
        "note_full",
        "85ff5bf72e74962be19991ba2f67edeb71212d80fc17a9abc83f0aedf228f8ec",
        "a66269646d637573746f6d65722d34383231646b696e646b6d656d6f72795f6e6f646567636f6e74656e74a264706c616e6a656e746572707269736565736561747318286a6576656e745f74696d651b00000199155c62006a70726f76656e616e6365a5646e6f74656d666972737420636f6e7461637466736f757263656b7469636b65742d3438323169746f6f6c5f63616c6c6b726561642d7469636b65746a6167656e745f73746570667472696167656b6f62736572766174696f6e707469636b65742d3438323120626f64796c636f6e74656e745f6b696e64646e6f7465",
    ),
    (
        "state_empty",
        "d6f3970d8ebe9b9ed1a48d4433b2872b324aa84e97bab1cfbfa50a112a620295",
        "a2646b696e64657374617465656e6f646573a0",
    ),
    (
        "state_one",
        "d1cdd28f6540e9e7a344554add2bd056bbf3717e7680833582fc463435446991",
        "a2646b696e64657374617465656e6f646573a1686772656574696e6758205107a6e4f686efec31950bf3171e7b4affd3c83697e8ab5182dc4a5f867cce10",
    ),
    (
        "commit_root",
        "925570c5e44700e4e36d74cffeaa010cbf3caeecb364a383f955ff835d088814",
        "a6646b696e6466636f6d6d69746474696d651b00000199155c62006573746174655820d1cdd28f6540e9e7a344554add2bd056bbf3717e7680833582fc46343544699166617574686f72656167656e74676d6573736167656c666972737420636f6d6d697467706172656e747380",
    ),
    (
        "commit_child",
        "fa02c3c4d1d9bdad0adb3f8bddd3ed9f36810e6158ef40d5467e38a579d4d447",
        "a6646b696e6466636f6d6d69746474696d651b00000199155d4c606573746174655820d6f3970d8ebe9b9ed1a48d4433b2872b324aa84e97bab1cfbfa50a112a62029566617574686f72656167656e74676d657373616765667365636f6e6467706172656e7473815820925570c5e44700e4e36d74cffeaa010cbf3caeecb364a383f955ff835d088814",
    ),
];

#[test]
#[ignore = "run with --ignored to print the current vectors"]
fn print() {
    for (name, object) in vectors() {
        let bytes = codec::encode(&object);
        let id = ObjectId::hash_canonical(&bytes);
        println!(
            "{name}\n  id:   {}\n  cbor: {}",
            id.to_hex(),
            to_hex(&bytes)
        );
    }
}

#[test]
fn the_format_doc_lists_the_same_vectors() {
    let doc = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/format/golden-vectors.md"
    ))
    .expect("read docs/format/golden-vectors.md");

    for (name, id, cbor) in GOLDEN {
        assert!(
            doc.contains(id),
            "{name}: id missing from golden-vectors.md"
        );
        assert!(
            doc.contains(cbor),
            "{name}: cbor missing from golden-vectors.md"
        );
    }
}

#[test]
fn vectors_match_the_frozen_bytes() {
    let by_name: std::collections::HashMap<_, _> = vectors().into_iter().collect();
    for (name, want_id, want_cbor) in GOLDEN {
        let object = by_name
            .get(name)
            .unwrap_or_else(|| panic!("no vector named {name}"));
        let bytes = codec::encode(object);
        let id = ObjectId::hash_canonical(&bytes);
        assert_eq!(&to_hex(&bytes), want_cbor, "{name}: CBOR bytes moved");
        assert_eq!(&id.to_hex(), want_id, "{name}: ObjectId moved");

        // the object also decodes from its frozen bytes and re-encodes identically
        let round = codec::encode(&codec::decode(&bytes).unwrap());
        assert_eq!(
            to_hex(&round),
            *want_cbor,
            "{name}: decode then encode is not stable"
        );
    }
}
