"""Tests for ``mnem.agents`` and the ``to_dict`` serialisers (#59)."""

import json
import threading
import time
from http.server import BaseHTTPRequestHandler, HTTPServer

import pytest

import mnem
from mnem import agents
from mnem._mnem import ConflictError


def test_remember_is_one_commit_with_provenance(tmp_path: object) -> None:
    store = mnem.init(tmp_path)
    commit = agents.remember(
        store,
        "customer-4821",
        "on the Enterprise plan",
        source="ticket-4821",
        step="step-3",
        author="support-agent",
        time_ms=1000,
    )

    assert store.head_commit() == commit
    assert store.staged() == []  # nothing left half-staged

    node = store.working_node("customer-4821")
    assert node.content == "on the Enterprise plan"
    assert node.provenance.source == "ticket-4821"
    assert node.provenance.agent_step == "step-3"

    [c] = store.log()
    assert c.message == "remember customer-4821"
    assert c.author == "support-agent"


def test_remember_many_is_a_single_commit(tmp_path: object) -> None:
    store = mnem.init(tmp_path)
    commit = agents.remember_many(
        store,
        [
            {"id": "plan", "content": "enterprise", "source": "ticket-1"},
            {"id": "seats", "content": 240},
            {"id": "renewal", "content": "2027-01"},
        ],
        summary="learn the account",
        time_ms=1000,
    )

    assert store.head_commit() == commit
    assert len(store.log()) == 1
    mem = store.working_memory()
    assert set(mem) == {"plan", "seats", "renewal"}
    assert mem["seats"].content == 240
    assert mem["plan"].provenance.source == "ticket-1"


def test_forget_tombstones_in_one_commit(tmp_path: object) -> None:
    store = mnem.init(tmp_path)
    agents.remember(store, "temp", "scratch note", time_ms=1000)
    agents.remember(store, "keep", "real belief", time_ms=2000)

    commit = agents.forget(store, "temp", time_ms=3000)
    assert store.head_commit() == commit
    assert store.working_node("temp") is None
    assert store.working_node("keep").content == "real belief"
    assert store.log()[0].message == "forget temp"


def test_default_commit_messages(tmp_path: object) -> None:
    store = mnem.init(tmp_path)
    agents.remember(store, "a", 1, time_ms=1000)
    agents.remember_many(store, [{"id": "b", "content": 2}], time_ms=2000)
    agents.remember_many(
        store, [{"id": "c", "content": 3}, {"id": "d", "content": 4}], time_ms=3000
    )
    messages = [c.message for c in store.log()]
    assert messages == ["remember 2 nodes", "remember 1 node", "remember a"]


def test_to_dict_is_json_serialisable(tmp_path: object) -> None:
    store = mnem.init(tmp_path)
    agents.remember(
        store, "plan", "enterprise", source="ticket-4821", step="s1", time_ms=1000
    )
    first = store.head_commit()
    agents.remember(store, "plan", "pro", observation="obs-7", time_ms=2000)

    blame = store.blame("plan")
    bd = blame.to_dict()
    assert json.loads(json.dumps(bd)) == bd
    assert bd["commit"] == blame.commit
    assert bd["node"]["content"] == "pro"
    assert bd["node"]["provenance"]["observation"] == "obs-7"

    [change] = store.diff(first, None)
    cd = change.to_dict()
    assert cd["kind"] == "modified"
    assert cd["old"]["content"] == "enterprise"
    assert cd["new"]["content"] == "pro"
    assert json.loads(json.dumps(cd)) == cd

    commit = store.log()[0]
    md = commit.to_dict()
    assert md["parents"] == list(commit.parents)
    assert json.loads(json.dumps(md)) == md


def test_notify_branch_posts_a_real_payload_to_a_real_server(tmp_path: object) -> None:
    received: list[dict] = []

    class Handler(BaseHTTPRequestHandler):
        def do_POST(self) -> None:
            length = int(self.headers["Content-Length"])
            received.append(json.loads(self.rfile.read(length)))
            self.send_response(200)
            self.end_headers()

        def log_message(self, *args: object) -> None:
            pass  # keep the test's own output clean

    server = HTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        store = mnem.init(tmp_path)
        agents.remember(store, "fact", "v1", time_ms=1000)
        commit_id = store.head_commit()
        store.new_branch("feature-x")

        agents.notify_branch(
            store, "feature-x", f"http://127.0.0.1:{server.server_port}/", author="bob"
        )
    finally:
        server.shutdown()

    assert received == [
        {
            "event": "branch_created",
            "branch": "feature-x",
            "author": "bob",
            "commit": commit_id,
        }
    ]


def test_notify_branch_raises_on_an_unreachable_webhook(tmp_path: object) -> None:
    store = mnem.init(tmp_path)
    agents.remember(store, "fact", "v1", time_ms=1000)
    store.new_branch("feature-x")

    with pytest.raises(OSError):
        agents.notify_branch(
            store, "feature-x", "http://127.0.0.1:1/", timeout=1.0
        )


def test_sync_from_directory_mirrors_a_remote_store_without_colliding(
    tmp_path: object,
) -> None:
    sender = mnem.init(tmp_path / "sender")
    agents.remember(sender, "plan", "canary rollout v1", time_ms=1000)

    watcher = mnem.init(tmp_path / "watcher")
    agents.remember(watcher, "own-fact", "belongs to the watcher only", time_ms=1000)

    watch_dir = tmp_path / "watch"
    watch_dir.mkdir()

    stop = threading.Event()
    thread = threading.Thread(
        target=agents.sync_from_directory,
        args=(watcher, watch_dir, "sender"),
        kwargs={"poll_interval": 0.1, "stop": stop},
        daemon=True,
    )
    thread.start()
    try:
        (watch_dir / "export1.json").write_text(sender.export())
        _wait_until(lambda: "sender:plan" in watcher.working_memory())

        mem = watcher.working_memory()
        assert set(mem) == {"own-fact", "sender:plan"}
        assert mem["sender:plan"].content == "canary rollout v1"
        assert mem["own-fact"].content == "belongs to the watcher only"

        agents.remember(sender, "plan", "canary rollout v2, widened", time_ms=2000)
        (watch_dir / "export2.json").write_text(sender.export())
        _wait_until(
            lambda: watcher.working_memory().get("sender:plan")
            and watcher.working_memory()["sender:plan"].content
            == "canary rollout v2, widened"
        )

        assert list(watch_dir.glob("*.json")) == []
    finally:
        stop.set()
        thread.join(timeout=5.0)
    assert not thread.is_alive()


def _wait_until(predicate: object, timeout: float = 5.0) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if predicate():
            return
        time.sleep(0.05)
    raise AssertionError("condition never became true")


def test_with_retry_gives_up_after_a_bounded_number_of_attempts() -> None:
    attempts = 0

    def always_conflicts() -> None:
        nonlocal attempts
        attempts += 1
        raise ConflictError("branch moved")

    with pytest.raises(ConflictError):
        agents._with_retry(always_conflicts)
    assert attempts == agents._RETRIES

    calls = 0

    def succeeds_on_the_third_try() -> str:
        nonlocal calls
        calls += 1
        if calls < 3:
            raise ConflictError("branch moved")
        return "ok"

    assert agents._with_retry(succeeds_on_the_third_try) == "ok"
    assert calls == 3
