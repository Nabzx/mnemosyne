"""Tests for the agent-facing SDK, ``mnem`` (#30).

The SDK is pure ergonomics over the binding (ADR-0004); these check the
wrapping, not the store behaviour, which the Rust core covers.
"""

import pytest

import mnem


def test_an_agent_loop_persists_and_reads_back_its_memory(tmp_path: object) -> None:
    store = mnem.init(tmp_path)
    assert isinstance(store, mnem.Store)
    assert store.root == tmp_path
    assert store.format_version == 1

    first = store.add(
        "customer-4821",
        {"plan": "enterprise", "seats": 40},
        provenance=mnem.Provenance(source="ticket-4821", agent_step="triage"),
    )
    assert len(first) == 64
    assert store.staged() == [("customer-4821", first)]

    store.commit("learn the plan tier", author="support-agent", time_ms=1_757_160_000_000)
    assert store.staged() == []

    store.add_node(mnem.MemoryNode(id="customer-4821", content={"plan": "pro", "seats": 40}))
    store.commit("record the downgrade", author="support-agent", time_ms=1_757_170_000_000)

    history = store.log()
    assert [c.message for c in history] == ["record the downgrade", "learn the plan tier"]
    assert all(isinstance(c, mnem.Commit) for c in history)
    assert history[0].parents == (history[1].id,)
    assert history[1].parents == ()

    assert store.log(limit=1)[0].message == "record the downgrade"


def test_open_finds_the_store_from_a_subdirectory(tmp_path: object) -> None:
    mnem.init(tmp_path)
    sub = tmp_path / "runs" / "run-7"  # type: ignore[operator]
    sub.mkdir(parents=True)

    store = mnem.open(sub)
    assert store.root == tmp_path
    assert store.head() == "ref: main"


def test_commit_author_falls_back_to_the_environment(
    tmp_path: object, monkeypatch: pytest.MonkeyPatch
) -> None:
    store = mnem.init(tmp_path)
    store.add("n", "x")

    monkeypatch.setenv("MNEM_AUTHOR", "env-agent")
    store.commit("no explicit author", time_ms=1)
    assert store.log()[0].author == "env-agent"


def test_unstage_reports_whether_the_node_was_staged(tmp_path: object) -> None:
    store = mnem.init(tmp_path)
    store.add("n", "x")
    assert store.unstage("n") is True
    assert store.unstage("n") is False


def test_missing_store_raises_the_typed_exception(tmp_path: object) -> None:
    with pytest.raises(mnem.NoStoreError):
        mnem.open(tmp_path)


def test_repr_names_the_root(tmp_path: object) -> None:
    store = mnem.init(tmp_path)
    assert "mnem.Store" in repr(store)
