//! Phase 1 definition of done (#31).
//!
//! Persist a fixed synthetic agent run, drop the store so the database file is
//! closed, reopen it from disk, and assert the stored memory came back
//! byte-identical. Black box: only the public `mnem-core` API, plus a raw read
//! of the object table to compare bytes.

use std::collections::BTreeMap;

use mnem_core::codec;
use mnem_core::objects::{self, OBJECTS};
use mnem_core::{ContentKind, MemoryNode, Object, ObjectId, Provenance, Store};
use redb::ReadableTable;
use serde_json::json;

fn scratch_dir(tag: &str) -> std::path::PathBuf {
    let base = std::env::temp_dir().join(format!(
        "mnem-{tag}-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&base).unwrap();
    base
}

fn note(
    id: &str,
    content: serde_json::Value,
    provenance: Provenance,
    event_time: Option<i64>,
) -> MemoryNode {
    MemoryNode {
        id: id.to_string(),
        content,
        content_kind: ContentKind::Note,
        provenance,
        event_time,
    }
}

/// A support agent triaging one ticket over three commits: a mix of fresh
/// nodes and updates, string / nested / unicode content, and provenance
/// ranging from empty to every field set.
fn synthetic_run(store: &Store) -> Vec<ObjectId> {
    let mut commits = Vec::new();

    store
        .stage(&note(
            "customer-4821",
            json!("checkout fails on Safari"),
            Provenance {
                agent_step: Some("intake".to_string()),
                source: Some("ticket-4821".to_string()),
                ..Default::default()
            },
            None,
        ))
        .unwrap();
    store
        .stage(&note(
            "case-4821-status",
            json!("triaging"),
            Provenance::default(),
            None,
        ))
        .unwrap();
    commits.push(
        store
            .commit("open the case", "support-agent", 1_757_000_000_000)
            .unwrap(),
    );

    store
        .stage(&note(
            "case-4821-cause",
            json!({ "area": "payments", "confidence": 0.62, "symptoms": ["3ds timeout", "retry loop"] }),
            Provenance {
                agent_step: Some("diagnose".to_string()),
                tool_call: Some("search-logs".to_string()),
                ..Default::default()
            },
            Some(1_757_000_500_000),
        ))
        .unwrap();
    store
        .stage(&note(
            "customer-4821",
            json!("checkout fails on Safari 17 only"),
            Provenance {
                agent_step: Some("diagnose".to_string()),
                observation: Some("reproduced on Safari 17.4".to_string()),
                source: Some("ticket-4821".to_string()),
                ..Default::default()
            },
            None,
        ))
        .unwrap();
    commits.push(
        store
            .commit("add the diagnosis", "support-agent", 1_757_001_000_000)
            .unwrap(),
    );

    store
        .stage(&note(
            "case-4821-status",
            json!("resolved"),
            Provenance::default(),
            None,
        ))
        .unwrap();
    store
        .stage(&note(
            "case-4821-resolution",
            json!("shipped a 3DS retry cap; customer confirmed"),
            Provenance {
                agent_step: Some("resolve".to_string()),
                observation: Some("customer replied to the ticket".to_string()),
                tool_call: Some("deploy".to_string()),
                source: Some("ticket-4821".to_string()),
                note: Some("regression test added".to_string()),
            },
            Some(1_757_002_000_000),
        ))
        .unwrap();
    store
        .stage(&note(
            "case-4821-closing-note",
            json!("customer wrote: \u{201e}danke \u{2014} works now \u{2705}\u{201c}"),
            Provenance::default(),
            None,
        ))
        .unwrap();
    commits.push(
        store
            .commit(
                "record the fix and close",
                "support-agent",
                1_757_002_500_000,
            )
            .unwrap(),
    );

    commits
}

/// Every row of the object table: id bytes to stored bytes.
fn object_bytes(store: &Store) -> BTreeMap<Vec<u8>, Vec<u8>> {
    let txn = store.begin_read().unwrap();
    let table = txn.open_table(OBJECTS).unwrap();
    let mut out = BTreeMap::new();
    for row in table.iter().unwrap() {
        let (key, value) = row.unwrap();
        out.insert(key.value().to_vec(), value.value().to_vec());
    }
    out
}

#[test]
fn a_fixed_run_reloads_byte_identical() {
    let root = scratch_dir("round-trip");

    let expected_commits;
    let before;
    {
        let store = Store::init(&root).unwrap();
        expected_commits = synthetic_run(&store);
        before = object_bytes(&store);
    } // Store dropped here: the redb database is closed.

    let store = Store::open(&root).unwrap();
    let after = object_bytes(&store);

    // 1. every object's on-disk bytes survived close and reopen unchanged.
    assert_eq!(
        before, after,
        "the object store is not byte-identical after reload"
    );
    assert!(
        after.len() >= 3 + 3 + 5,
        "expected at least 3 commits, 3 states and 5 nodes"
    );

    // 2. content addressing still holds and the codec is stable.
    for (key, bytes) in &after {
        let id = ObjectId::from_bytes(key.as_slice().try_into().expect("32-byte key"));
        assert_eq!(
            ObjectId::hash_canonical(bytes),
            id,
            "the stored id is not the hash of the stored bytes"
        );
        let decoded = codec::decode(bytes).expect("stored bytes decode to an object");
        assert_eq!(
            &codec::encode(&decoded),
            bytes,
            "re-encoding a decoded object does not reproduce the stored bytes"
        );
    }

    // 3. the commit history reloads exactly, newest first.
    let log = store.log_head(false, None).unwrap();
    let ids: Vec<ObjectId> = log.iter().map(|(id, _)| *id).collect();
    let newest_first: Vec<ObjectId> = expected_commits.iter().rev().copied().collect();
    assert_eq!(ids, newest_first);
    assert_eq!(log[0].1.message, "record the fix and close");
    assert_eq!(log[0].1.author, "support-agent");
    assert_eq!(log[0].1.time, 1_757_002_500_000);
    assert_eq!(log[2].1.parents, Vec::<ObjectId>::new());
    assert_eq!(log[1].1.parents, vec![expected_commits[0]]);
    assert_eq!(log[0].1.parents, vec![expected_commits[1]]);

    // 4. HEAD state holds every node id, each pointing at its latest value.
    let txn = store.begin_read().unwrap();
    let head = store
        .resolve_head(&txn)
        .unwrap()
        .expect("HEAD resolves to a commit");
    assert_eq!(head, expected_commits[2]);
    let Object::Commit(tip) = objects::require(&txn, head).unwrap() else {
        panic!("HEAD is not a commit");
    };
    let Object::State(state) = objects::require(&txn, tip.state).unwrap() else {
        panic!("the commit does not point at a state");
    };
    let names: Vec<&str> = state.nodes.keys().map(String::as_str).collect();
    assert_eq!(
        names,
        vec![
            "case-4821-cause",
            "case-4821-closing-note",
            "case-4821-resolution",
            "case-4821-status",
            "customer-4821",
        ]
    );

    // 5. specific nodes reload with the exact content and provenance written.
    let customer = require_node(&txn, state.nodes["customer-4821"]);
    assert_eq!(customer.content, json!("checkout fails on Safari 17 only"));
    assert_eq!(
        customer.provenance.observation.as_deref(),
        Some("reproduced on Safari 17.4")
    );
    assert_eq!(customer.event_time, None);

    let cause = require_node(&txn, state.nodes["case-4821-cause"]);
    assert_eq!(
        cause.content,
        json!({ "area": "payments", "confidence": 0.62, "symptoms": ["3ds timeout", "retry loop"] })
    );
    assert_eq!(cause.event_time, Some(1_757_000_500_000));

    let closing = require_node(&txn, state.nodes["case-4821-closing-note"]);
    assert_eq!(
        closing.content,
        json!("customer wrote: \u{201e}danke \u{2014} works now \u{2705}\u{201c}")
    );
    assert!(closing.provenance.is_empty());

    // 6. the first commit's state kept the original value of an updated node.
    let Object::Commit(first) = objects::require(&txn, expected_commits[0]).unwrap() else {
        panic!("first commit is not a commit");
    };
    let Object::State(first_state) = objects::require(&txn, first.state).unwrap() else {
        panic!("first commit has no state");
    };
    assert_eq!(first_state.nodes.len(), 2);
    let customer_v1 = require_node(&txn, first_state.nodes["customer-4821"]);
    assert_eq!(customer_v1.content, json!("checkout fails on Safari"));

    std::fs::remove_dir_all(&root).ok();
}

fn require_node(txn: &redb::ReadTransaction, id: ObjectId) -> MemoryNode {
    match objects::require(txn, id).unwrap() {
        Object::MemoryNode(node) => node,
        other => panic!("{id} is a {}, not a memory node", other.kind()),
    }
}
