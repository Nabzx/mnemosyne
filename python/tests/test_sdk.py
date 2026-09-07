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


def test_branch_context_manager_isolates_an_experiment(tmp_path: object) -> None:
    store = mnem.init(tmp_path)
    store.add("customer", "on Enterprise")
    store.add("status", "triaging")
    store.commit("open", author="agent", time_ms=1)

    with store.branch("assume-downgrade"):
        assert store.head() == "ref: assume-downgrade"
        store.add("customer", "downgraded to Pro")
        store.rm("status")
        changes = store.diff()
        assert {c.id: c.kind for c in changes} == {
            "customer": "modified",
            "status": "removed",
        }
        store.commit("try it", author="agent", time_ms=2)
        assert sorted(store.working_memory()) == ["customer"]

    # back on main, main untouched, the branch kept
    assert store.head() == "ref: main"
    assert sorted(store.working_memory()) == ["customer", "status"]
    assert [name for name, _ in store.branches()] == ["assume-downgrade", "main"]


def test_time_travel_and_cross_branch_diff(tmp_path: object) -> None:
    store = mnem.init(tmp_path)
    store.add("plan", "enterprise")
    store.commit("one", author="agent", time_ms=1)
    c1 = store.head_commit()

    store.new_branch("pro")
    store.checkout("pro")
    store.add("plan", "pro")
    store.commit("two", author="agent", time_ms=2)

    assert store.state_at(c1)["plan"].content == "enterprise"
    assert store.working_node("plan").content == "pro"

    changes = store.diff("main", "pro")
    assert len(changes) == 1
    assert changes[0].kind == "modified"
    assert changes[0].old.content == "enterprise"
    assert changes[0].new.content == "pro"


def test_checkout_refuses_a_dirty_index(tmp_path: object) -> None:
    store = mnem.init(tmp_path)
    store.add("a", "1")
    store.commit("one", author="agent", time_ms=1)
    store.new_branch("other")
    store.add("b", "2")
    with pytest.raises(mnem.InvalidRefError):
        store.checkout("other")
    store.checkout("other", discard=True)
    assert store.staged() == []


def test_merge_clean_fast_forward_and_up_to_date(tmp_path: object) -> None:
    store = mnem.init(tmp_path)
    store.add("plan", "enterprise")
    store.commit("base", author="agent", time_ms=1)
    store.new_branch("feature")

    store.checkout("feature")
    store.add("region", "eu")
    store.commit("feature work", author="agent", time_ms=2)

    store.checkout("main")
    ff = store.merge("feature")
    assert ff.status == "fast-forwarded"
    assert ff.ok
    assert store.working_node("region").content == "eu"

    again = store.merge("feature")
    assert again.status == "up-to-date"


def test_merge_conflict_then_resolution(tmp_path: object) -> None:
    store = mnem.init(tmp_path)
    store.add("plan", "enterprise")
    store.commit("base", author="agent", time_ms=1)
    store.new_branch("feature")

    store.checkout("feature")
    store.add("plan", "pro")
    store.commit("theirs", author="agent", time_ms=2)

    store.checkout("main")
    store.add("plan", "team")
    store.commit("ours", author="agent", time_ms=3)

    conflicted = store.merge("feature")
    assert not conflicted.ok
    assert conflicted.status == "conflicts"
    assert conflicted.commit is None
    assert [c.id for c in conflicted.conflicts] == ["plan"]
    assert conflicted.conflicts[0].kind == "edit/edit"
    assert conflicted.conflicts[0].ours.content == "team"
    assert conflicted.conflicts[0].theirs.content == "pro"

    merged = store.merge("feature", resolutions={"plan": "theirs"})
    assert merged.status == "merged"
    assert merged.commit is not None
    assert store.working_node("plan").content == "pro"


def test_merge_set_resolution_with_a_memory_node(tmp_path: object) -> None:
    store = mnem.init(tmp_path)
    store.add("plan", "enterprise")
    store.commit("base", author="agent", time_ms=1)
    store.new_branch("feature")

    store.checkout("feature")
    store.add("plan", "pro")
    store.commit("theirs", author="agent", time_ms=2)

    store.checkout("main")
    store.add("plan", "team")
    store.commit("ours", author="agent", time_ms=3)

    pick = mnem.MemoryNode(id="plan", content="starter")
    merged = store.merge("feature", resolutions={"plan": pick})
    assert merged.status == "merged"
    assert store.working_node("plan").content == "starter"


def test_blame_resolves_a_node_to_its_introducing_commit(tmp_path: object) -> None:
    store = mnem.init(tmp_path)
    store.add("plan", "enterprise", provenance=mnem.Provenance(source="ticket-1"))
    c1 = store.commit("one", author="agent", time_ms=1000)
    store.add("plan", "pro", provenance=mnem.Provenance(observation="obs-7"))
    c2 = store.commit("two", author="agent", time_ms=2000)
    store.add("note", "unrelated")
    store.commit("three", author="agent", time_ms=3000)

    blame = store.blame("plan")
    assert blame.commit == c2
    assert blame.node.content == "pro"
    assert blame.provenance.observation == "obs-7"
    assert blame.time == 2000  # falls back to the commit time

    # and at an explicit past commit
    assert store.blame("plan", c1).commit == c1

    with pytest.raises(mnem.InvalidRefError):
        store.blame("ghost")


def test_bisect_finds_the_first_commit_where_a_predicate_holds(tmp_path: object) -> None:
    store = mnem.init(tmp_path)
    store.add("plan", "enterprise")
    store.commit("c0", author="agent", time_ms=1000)
    ids = []
    for i in range(1, 10):
        if i == 6:
            store.add("plan", "pro")  # the wrong belief enters here, and stays
        store.add("seats", str(i))
        ids.append(store.commit(f"c{i}", author="agent", time_ms=1000 + i * 1000))

    boundary = store.bisect(
        lambda state: "plan" in state and state["plan"].content == "pro"
    )
    assert boundary == ids[5]  # the i == 6 commit


def test_changed_by_lists_a_commits_change_set(tmp_path: object) -> None:
    store = mnem.init(tmp_path)
    store.add("plan", "enterprise")
    store.add("owner", "alice")
    store.commit("one", author="agent", time_ms=1000)

    store.add("plan", "pro")
    store.add("region", "eu")
    store.rm("owner")
    c2 = store.commit("two", author="agent", time_ms=2000)

    assert store.changed_by(c2) == {
        "plan": "modified",
        "region": "added",
        "owner": "removed",
    }
