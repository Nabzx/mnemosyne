//! Phase 4 definition of done (#54): a synthetic run where a wrong belief
//! enters at a known commit, and `bisect` finds it while `blame` explains it.
//!
//! This is the "it explains" story end to end: a support agent builds up an
//! account picture over a run, misreads one observation and writes a wrong plan
//! tier, then keeps working. Later, `bisect` on "plan == pro" lands exactly on
//! the bad commit, and `blame plan` there names the observation that caused it.

use mnem_core::bisect::{node_content_is, node_present};
use mnem_core::{ContentKind, MemoryNode, Provenance, Store};

fn scratch() -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let base = std::env::temp_dir().join(format!(
        "mnem-buggy-run-{}-{}-{:?}",
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

fn note(id: &str, content: &str) -> MemoryNode {
    MemoryNode {
        id: id.to_string(),
        content: serde_json::Value::String(content.to_string()),
        content_kind: ContentKind::Note,
        provenance: Provenance::default(),
        event_time: None,
    }
}

fn from_observation(id: &str, content: &str, observation: &str, step: &str) -> MemoryNode {
    MemoryNode {
        id: id.to_string(),
        content: serde_json::Value::String(content.to_string()),
        content_kind: ContentKind::Note,
        provenance: Provenance {
            agent_step: Some(step.to_string()),
            observation: Some(observation.to_string()),
            ..Default::default()
        },
        event_time: Some(1_700_000_000_000),
    }
}

/// The run. Returns (store, root, the commit where the wrong belief entered).
fn buggy_run() -> (Store, std::path::PathBuf, mnem_core::ObjectId) {
    let root = scratch();
    let store = Store::init(&root).unwrap();
    let mut t = 1_000i64;
    let mut tick = || {
        t += 1_000;
        t
    };

    // c0: the account opens
    store.stage(&note("account", "acme-corp")).unwrap();
    store.stage(&note("plan", "enterprise")).unwrap();
    store
        .commit("open the account", "support-agent", tick())
        .unwrap();

    // a few commits of ordinary, correct work
    store.stage(&note("primary-contact", "alice@acme")).unwrap();
    store
        .commit("learn the primary contact", "support-agent", tick())
        .unwrap();

    store.stage(&note("open-tickets", "2")).unwrap();
    store
        .commit("count the open tickets", "support-agent", tick())
        .unwrap();

    store.stage(&note("seat-count", "240")).unwrap();
    store
        .commit("record the seat count", "support-agent", tick())
        .unwrap();

    store.stage(&note("renewal", "2027-01")).unwrap();
    store
        .commit("note the renewal date", "support-agent", tick())
        .unwrap();

    // THE BUG: a misread billing note. The agent thinks the account downgraded.
    store
        .stage(&from_observation(
            "plan",
            "pro",
            "billing-note-8842",
            "step-31",
        ))
        .unwrap();
    let bad = store
        .commit("update the plan tier from billing", "support-agent", tick())
        .unwrap();

    // work continues, some of it leaning on the now-wrong plan
    store.stage(&note("discount-band", "pro-tier")).unwrap();
    store
        .commit("apply the pro-tier discount band", "support-agent", tick())
        .unwrap();

    store.stage(&note("open-tickets", "3")).unwrap();
    store
        .commit("a new ticket comes in", "support-agent", tick())
        .unwrap();

    store
        .stage(&note("primary-contact", "alice@acme, bob@acme"))
        .unwrap();
    store
        .commit("add a second contact", "support-agent", tick())
        .unwrap();

    store
        .stage(&note("escalation", "billing mismatch flagged"))
        .unwrap();
    store
        .commit("escalate the billing mismatch", "support-agent", tick())
        .unwrap();

    (store, root, bad)
}

#[test]
fn bisect_lands_on_the_bad_commit_and_blame_explains_it() {
    let (store, root, bad) = buggy_run();

    // bisect for "plan is pro" finds exactly the commit that introduced it
    let found = store
        .bisect(
            None,
            None,
            node_content_is("plan", serde_json::json!("pro")),
        )
        .unwrap();
    assert_eq!(
        found, bad,
        "bisect did not land on the commit that set plan=pro"
    );

    // blame at HEAD resolves the current wrong value to the same commit
    let blame = store.blame("plan", None).unwrap();
    assert_eq!(blame.commit, bad);
    assert_eq!(blame.node.content, serde_json::json!("pro"));
    assert_eq!(
        blame.node.provenance.observation.as_deref(),
        Some("billing-note-8842"),
        "blame did not surface the misread observation"
    );
    assert_eq!(blame.node.provenance.agent_step.as_deref(), Some("step-31"));
    assert_eq!(blame.time, 1_700_000_000_000); // the node's event_time

    // the reverse index agrees the bad commit changed `plan`
    assert_eq!(
        store.changed_by(bad).unwrap().get("plan"),
        Some(&mnem_core::ChangeKind::Modified)
    );

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn bisect_finds_where_a_belief_first_appeared() {
    let (store, root, _bad) = buggy_run();

    // discount-band did not exist until after the bug; bisect on its presence
    // finds the commit that first added it
    let first_present = store
        .bisect(None, None, node_present("discount-band"))
        .unwrap();
    let blame = store.blame("discount-band", None).unwrap();
    assert_eq!(first_present, blame.commit);

    std::fs::remove_dir_all(&root).ok();
}
