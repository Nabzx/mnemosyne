"""The MCP server (ADR-0016).

The operations live in ``do_*`` functions that take an open :class:`mnem.Store`,
so they are testable without the protocol. :func:`build_server` wraps them as
MCP tools and resources, opening the store fresh on every call (the 2026-07-28
transport is stateless, so the server holds nothing between calls).
"""

from __future__ import annotations

import json
import os
from typing import Any

import mnem
from mcp.server import MCPServer
from mcp.server.mcpserver.exceptions import ToolError
from mnem import MnemError, agents

# MnemError subclass name -> the code an agent sees.
_CODES = {
    "NotFoundError": "not_found",
    "InvalidRefError": "invalid_ref",
    "ConflictError": "conflict",
    "CorruptStoreError": "corrupt_store",
    "NoStoreError": "no_store",
    "StoreIoError": "store_io",
    "FormatVersionError": "format_version",
}

_LOG_LIMIT = 20
_MISSING = object()


def error_code(exc: MnemError) -> str:
    return _CODES.get(type(exc).__name__, "error")


def resolve_store_path(explicit: str | None) -> str:
    """The store path: an explicit argument, then ``$MNEM_STORE``, then ``.``."""
    return explicit or os.environ.get("MNEM_STORE") or "."


# --- operations (take an open Store) ---------------------------------------


def do_remember(
    store: mnem.Store,
    id: str,
    content: Any,
    *,
    source: str | None = None,
    step: str | None = None,
    observation: str | None = None,
    summary: str | None = None,
) -> dict[str, Any]:
    commit = agents.remember(
        store, id, content, source=source, step=step,
        observation=observation, summary=summary,
    )
    return {"commit": commit}


def do_remember_many(
    store: mnem.Store, items: list[dict[str, Any]], *, summary: str | None = None
) -> dict[str, Any]:
    return {"commit": agents.remember_many(store, items, summary=summary)}


def do_forget(store: mnem.Store, id: str, *, summary: str | None = None) -> dict[str, Any]:
    return {"commit": agents.forget(store, id, summary=summary)}


def do_recall(store: mnem.Store, id: str | None = None) -> dict[str, Any] | None:
    if id is None:
        return {k: n.to_dict() for k, n in store.working_memory().items()}
    node = store.working_node(id)
    return node.to_dict() if node is not None else None


def do_recall_at(
    store: mnem.Store, commit: str, id: str | None = None
) -> dict[str, Any] | None:
    state = store.state_at(commit)
    if id is None:
        return {k: n.to_dict() for k, n in state.items()}
    node = state.get(id)
    return node.to_dict() if node is not None else None


def do_history(store: mnem.Store, limit: int = _LOG_LIMIT) -> list[dict[str, Any]]:
    return [c.to_dict() for c in store.log(limit=limit)]


def do_why(store: mnem.Store, id: str, at: str | None = None) -> dict[str, Any]:
    return store.blame(id, at).to_dict()


def do_when_did(
    store: mnem.Store,
    id: str,
    *,
    equals: Any = _MISSING,
    absent: bool = False,
    present: bool = False,
    good: str | None = None,
) -> dict[str, Any]:
    if absent:
        def predicate(state: dict[str, Any]) -> bool:
            return id not in state
    elif present:
        def predicate(state: dict[str, Any]) -> bool:
            return id in state
    elif equals is not _MISSING:
        def predicate(state: dict[str, Any]) -> bool:
            return id in state and state[id].content == equals
    else:
        raise ToolError("invalid_ref: when_did needs one of equals, absent or present")

    commit = store.bisect(predicate, good=good)
    result: dict[str, Any] = {"commit": commit}
    try:
        result["blame"] = store.blame(id, commit).to_dict()
    except MnemError:
        result["blame"] = None
    return result


def do_whats_new(store: mnem.Store, commit: str) -> dict[str, str]:
    return store.changed_by(commit)


def do_commit_view(store: mnem.Store, commit: str) -> dict[str, Any]:
    return {
        "changed": store.changed_by(commit),
        "state": {k: n.to_dict() for k, n in store.state_at(commit).items()},
    }


def do_branch(store: mnem.Store, name: str, start: str | None = None) -> dict[str, Any]:
    store.new_branch(name, start=start)
    return {"branch": name}


def do_switch(store: mnem.Store, target: str) -> dict[str, Any]:
    store.checkout(target)
    return {"head": store.head()}


def do_merge(
    store: mnem.Store,
    theirs: str,
    *,
    resolutions: dict[str, str] | None = None,
    strategy: str | None = None,
) -> dict[str, Any]:
    return store.merge(theirs, resolutions=resolutions, strategy=strategy).to_dict()


# --- the server ----------------------------------------------------------


def build_server(store_path: str | None = None, tools: str = "core") -> MCPServer:
    """An :class:`MCPServer` bound to one store.

    ``tools="all"`` additionally exposes ``branch`` / ``switch`` / ``merge``,
    which are for an agent harness rather than the model.

    One store handle is opened here and reused: redb allows a single open
    handle per file per process, so "open per call" is not possible. This is
    not protocol state (ADR-0016's stateless requirement is about the MCP
    transport carrying no session id); branch state still lives in the store's
    ``HEAD`` file, which the handle re-reads on every operation.
    """
    mcp = MCPServer(
        "mnem",
        instructions="Mnemosyne: the agent's memory, under version control. "
        "Record what you learn with `remember` (include where it came from); "
        "trace a wrong belief with `why` and `when_did`.",
    )

    store = mnem.open(resolve_store_path(store_path))

    def opened() -> mnem.Store:
        return store

    def guard(action: Any) -> Any:
        try:
            return action()
        except MnemError as exc:
            raise ToolError(f"{error_code(exc)}: {exc}") from exc

    @mcp.tool()
    def remember(
        id: str,
        content: Any,
        source: str | None = None,
        step: str | None = None,
        observation: str | None = None,
        summary: str | None = None,
    ) -> dict[str, Any]:
        """Record one fact in memory as a commit, with where it came from."""
        return guard(lambda: do_remember(
            opened(), id, content, source=source, step=step,
            observation=observation, summary=summary,
        ))

    @mcp.tool()
    def revise(
        id: str,
        content: Any,
        source: str | None = None,
        step: str | None = None,
        observation: str | None = None,
        summary: str | None = None,
    ) -> dict[str, Any]:
        """Change an existing belief. Same as `remember`; the name says intent."""
        return guard(lambda: do_remember(
            opened(), id, content, source=source, step=step,
            observation=observation, summary=summary,
        ))

    @mcp.tool()
    def remember_many(
        items: list[dict[str, Any]], summary: str | None = None
    ) -> dict[str, Any]:
        """Record several facts as one commit. Each item has id, content, and
        optionally source / step / observation."""
        return guard(lambda: do_remember_many(opened(), items, summary=summary))

    @mcp.tool()
    def forget(id: str, summary: str | None = None) -> dict[str, Any]:
        """Drop a memory from the next snapshot. Past snapshots keep it."""
        return guard(lambda: do_forget(opened(), id, summary=summary))

    @mcp.tool()
    def recall(id: str | None = None) -> dict[str, Any] | None:
        """Read one memory, or the whole current memory with no id. A single
        memory comes back with an ``id`` field; the whole memory as
        ``{node_id: memory}``."""
        return guard(lambda: do_recall(opened(), id))

    @mcp.tool()
    def recall_at(commit: str, id: str | None = None) -> dict[str, Any] | None:
        """Read memory as it stood at a past commit."""
        return guard(lambda: do_recall_at(opened(), commit, id))

    @mcp.tool()
    def history(limit: int = _LOG_LIMIT) -> list[dict[str, Any]]:
        """The recent commits, newest first."""
        return guard(lambda: do_history(opened(), limit))

    @mcp.tool()
    def why(id: str, at: str | None = None) -> dict[str, Any]:
        """The commit and provenance that set a memory's current value."""
        return guard(lambda: do_why(opened(), id, at))

    @mcp.tool()
    def when_did(
        id: str,
        equals: Any = None,
        absent: bool = False,
        present: bool = False,
        good: str | None = None,
    ) -> dict[str, Any]:
        """The first commit where a memory equals a value, is absent, or is
        present. Pass exactly one of equals / absent / present."""
        kwargs: dict[str, Any] = {"absent": absent, "present": present, "good": good}
        if equals is not None:
            kwargs["equals"] = equals
        return guard(lambda: do_when_did(opened(), id, **kwargs))

    @mcp.tool()
    def whats_new(commit: str) -> dict[str, str]:
        """The memories a commit added, changed or removed."""
        return guard(lambda: do_whats_new(opened(), commit))

    @mcp.resource("mnem://memory")
    def memory_resource() -> str:
        return json.dumps(guard(lambda: do_recall(opened())))

    @mcp.resource("mnem://memory/{node_id}")
    def node_resource(node_id: str) -> str:
        return json.dumps(guard(lambda: do_recall(opened(), node_id)))

    @mcp.resource("mnem://log")
    def log_resource() -> str:
        return json.dumps(guard(lambda: do_history(opened())))

    @mcp.resource("mnem://commit/{commit}")
    def commit_resource(commit: str) -> str:
        return json.dumps(guard(lambda: do_commit_view(opened(), commit)))

    if tools == "all":

        @mcp.tool()
        def branch(name: str, start: str | None = None) -> dict[str, Any]:
            """Fork memory onto a new branch (harness tool)."""
            return guard(lambda: do_branch(opened(), name, start))

        @mcp.tool()
        def switch(target: str) -> dict[str, Any]:
            """Move HEAD to a branch or commit (harness tool)."""
            return guard(lambda: do_switch(opened(), target))

        @mcp.tool()
        def merge(
            theirs: str,
            resolutions: dict[str, str] | None = None,
            strategy: str | None = None,
        ) -> dict[str, Any]:
            """Merge a branch into the current one (harness tool)."""
            return guard(lambda: do_merge(
                opened(), theirs, resolutions=resolutions, strategy=strategy,
            ))

    return mcp
