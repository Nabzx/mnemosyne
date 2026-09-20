"""Tests for the mnem-mcp server (#57)."""

import asyncio
import json

import mnem
import pytest
from mcp.server.mcpserver.exceptions import ToolError

from mnem_mcp import server


@pytest.fixture
def store_path(tmp_path: object) -> str:
    return str(mnem.init(tmp_path).root)


def call(srv: object, name: str, args: dict) -> object:
    result = asyncio.run(srv.call_tool(name, args))
    sc = result.structured_content
    if isinstance(sc, dict) and list(sc) == ["result"]:
        return sc["result"]  # MCP wraps non-object returns (lists, scalars)
    if sc is not None:
        return sc
    return json.loads(result.content[0].text) if result.content else None


def read(srv: object, uri: str) -> object:
    contents = asyncio.run(srv.read_resource(uri))
    return json.loads(contents[0].content)


# --- the operations, direct ---------------------------------------------


def test_operations_cover_the_belief_lifecycle(store_path: str) -> None:
    store = mnem.open(store_path)

    server.do_remember(store, "plan", "enterprise", source="ticket-1", step="s1")
    server.do_remember(store, "plan", "pro", observation="obs-7")
    server.do_remember_many(
        store, [{"id": "seats", "content": 240}, {"id": "owner", "content": "alice"}]
    )

    assert server.do_recall(store, "plan")["content"] == "pro"
    assert set(server.do_recall(store)) == {"plan", "seats", "owner"}

    why = server.do_why(store, "plan")
    assert why["node"]["content"] == "pro"
    assert why["node"]["provenance"]["observation"] == "obs-7"

    found = server.do_when_did(store, "plan", equals="pro")
    assert found["commit"] == why["commit"]
    assert found["blame"]["node"]["provenance"]["observation"] == "obs-7"

    server.do_forget(store, "owner")
    assert server.do_recall(store, "owner") is None
    assert len(server.do_history(store)) == 4  # 2 remember + 1 many + 1 forget


def test_operations_raise_typed_errors(store_path: str) -> None:
    store = mnem.open(store_path)
    server.do_remember(store, "a", 1)
    with pytest.raises(mnem.InvalidRefError):
        server.do_why(store, "ghost")
    with pytest.raises(ToolError):
        server.do_when_did(store, "a")  # no equals / absent / present


def test_error_code_covers_every_mnem_error(store_path: str) -> None:
    # Every _CODES entry, exercised directly against the exception type
    # rather than by contriving a live scenario for each - a real scenario is
    # covered separately below where one exists (NoStoreError, CorruptStoreError).
    for name, code in [
        ("NotFoundError", "not_found"),
        ("InvalidRefError", "invalid_ref"),
        ("ConflictError", "conflict"),
        ("CorruptStoreError", "corrupt_store"),
        ("NoStoreError", "no_store"),
        ("StoreIoError", "store_io"),
        ("FormatVersionError", "format_version"),
    ]:
        exc = getattr(mnem, name)("boom")
        assert server.error_code(exc) == code


def test_do_branch_and_do_switch(store_path: str) -> None:
    store = mnem.open(store_path)
    server.do_remember(store, "plan", "enterprise")
    assert store.head() == "ref: main"

    assert server.do_branch(store, "exp") == {"branch": "exp"}
    assert store.head() == "ref: main"  # branching alone does not switch HEAD
    assert server.do_switch(store, "exp") == {"head": "ref: exp"}
    assert store.head() == "ref: exp"

    server.do_remember(store, "plan", "trial")
    assert server.do_recall(store, "plan")["content"] == "trial"

    server.do_switch(store, "main")
    assert server.do_recall(store, "plan")["content"] == "enterprise"  # main untouched


def test_do_merge_round_trip_through_a_real_conflict(store_path: str) -> None:
    store = mnem.open(store_path)
    server.do_remember(store, "plan", "enterprise")
    server.do_branch(store, "exp")
    server.do_switch(store, "exp")
    server.do_remember(store, "plan", "trial")
    server.do_switch(store, "main")
    server.do_remember(store, "plan", "pro")

    conflicted = server.do_merge(store, "exp")
    assert conflicted["status"] == "conflicts"
    assert conflicted["conflicts"][0]["id"] == "plan"

    resolved = server.do_merge(store, "exp", resolutions={"plan": "ours"})
    assert resolved["status"] == "merged"
    assert server.do_recall(store, "plan")["content"] == "pro"


def test_no_store_error_on_an_uninitialised_path(tmp_path: object) -> None:
    with pytest.raises(mnem.NoStoreError):
        mnem.open(str(tmp_path))


def test_corrupt_store_error_on_a_mangled_head(store_path: str) -> None:
    import pathlib

    head = pathlib.Path(store_path) / ".mnem" / "HEAD"
    head.write_text("not a ref or a commit id")
    # `open()` itself does not validate HEAD; the error surfaces lazily on
    # the first operation that resolves it.
    with pytest.raises(mnem.CorruptStoreError):
        mnem.open(store_path).head()


# --- through the protocol ---------------------------------------------


def test_tools_are_registered_and_gated(tmp_path: object) -> None:
    core_path = str(mnem.init(tmp_path / "core").root)
    core = server.build_server(store_path=core_path)
    names = {t.name for t in asyncio.run(core.list_tools())}
    assert "remember" in names and "why" in names and "when_did" in names
    assert {"branch", "switch", "merge"}.isdisjoint(names)
    del core

    full_path = str(mnem.init(tmp_path / "full").root)
    full = server.build_server(store_path=full_path, tools="all")
    assert {"branch", "switch", "merge"} <= {t.name for t in asyncio.run(full.list_tools())}


def test_a_round_trip_through_the_server(store_path: str) -> None:
    srv = server.build_server(store_path=store_path)

    commit = call(srv, "remember", {"id": "plan", "content": "enterprise", "source": "t1"})["commit"]
    assert call(srv, "recall", {"id": "plan"})["content"] == "enterprise"
    assert call(srv, "history", {})[0]["id"] == commit

    mem = read(srv, "mnem://memory")
    assert mem["plan"]["provenance"]["source"] == "t1"
    assert read(srv, "mnem://memory/plan")["content"] == "enterprise"


def test_a_failing_tool_raises_a_tool_error(store_path: str) -> None:
    srv = server.build_server(store_path=store_path)
    with pytest.raises(ToolError):
        asyncio.run(srv.call_tool("why", {"id": "nothing"}))


def test_store_path_falls_back_to_the_env(monkeypatch: pytest.MonkeyPatch, store_path: str) -> None:
    monkeypatch.setenv("MNEM_STORE", store_path)
    srv = server.build_server()  # no explicit path
    call(srv, "remember", {"id": "x", "content": 1})
    assert call(srv, "recall", {"id": "x"})["content"] == 1
