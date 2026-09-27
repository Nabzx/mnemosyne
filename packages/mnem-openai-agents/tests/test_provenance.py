"""Tests for on_tool_end provenance auto-capture (#307, ADR-0020)."""

import asyncio
from types import SimpleNamespace

import mnem
import pytest
from agents.lifecycle import RunHooksBase

from mnem_openai_agents import MnemosyneSession


def _run(coro):
    return asyncio.run(coro)


@pytest.fixture
def store(tmp_path: object) -> mnem.Store:
    return mnem.init(tmp_path)


@pytest.fixture
def session(store: mnem.Store) -> MnemosyneSession:
    return MnemosyneSession(store, "u1")


def _tool_context(call_id: str) -> SimpleNamespace:
    return SimpleNamespace(tool_call_id=call_id)


def test_hooks_is_a_real_run_hooks(session: MnemosyneSession) -> None:
    # RunHooks itself is a subscripted generic alias, unusable with
    # isinstance(); RunHooksBase (its unparametrized origin) works.
    assert isinstance(session.hooks, RunHooksBase)


def test_on_tool_end_populates_pending_provenance(session: MnemosyneSession) -> None:
    tool = SimpleNamespace(name="lookup_order")
    _run(session.hooks.on_tool_end(_tool_context("call_1"), None, tool, "shipped"))
    assert session._pending_provenance["call_1"].source == "lookup_order"
    assert session._pending_provenance["call_1"].observation == "shipped"


def test_add_items_attaches_matching_provenance_by_call_id(session: MnemosyneSession) -> None:
    tool = SimpleNamespace(name="lookup_order")
    _run(session.hooks.on_tool_end(_tool_context("call_1"), None, tool, "shipped"))

    items = [
        {"type": "function_call", "call_id": "call_1", "name": "lookup_order"},
        {"type": "function_call_output", "call_id": "call_1", "output": "shipped"},
    ]
    _run(session.add_items(items))

    from mnem_openai_agents.session import _read_items

    nodes = _read_items(session._store, "u1")
    assert [n.provenance.source for n in nodes] == ["lookup_order", "lookup_order"]
    assert [n.provenance.observation for n in nodes] == ["shipped", "shipped"]


def test_items_with_no_matching_call_id_get_no_provenance(session: MnemosyneSession) -> None:
    items = [{"role": "user", "content": "hi"}]
    _run(session.add_items(items))

    from mnem_openai_agents.session import _read_items

    nodes = _read_items(session._store, "u1")
    assert nodes[0].provenance.source is None
    assert nodes[0].provenance.observation is None


def test_multiple_tool_calls_in_one_batch_keep_distinct_provenance(
    session: MnemosyneSession,
) -> None:
    _run(
        session.hooks.on_tool_end(
            _tool_context("call_1"), None, SimpleNamespace(name="lookup_order"), "shipped"
        )
    )
    _run(
        session.hooks.on_tool_end(
            _tool_context("call_2"), None, SimpleNamespace(name="lookup_refund"), "denied"
        )
    )

    items = [
        {"type": "function_call_output", "call_id": "call_1", "output": "shipped"},
        {"type": "function_call_output", "call_id": "call_2", "output": "denied"},
    ]
    _run(session.add_items(items))

    from mnem_openai_agents.session import _read_items

    nodes = _read_items(session._store, "u1")
    assert [n.provenance.source for n in nodes] == ["lookup_order", "lookup_refund"]
    assert [n.provenance.observation for n in nodes] == ["shipped", "denied"]


def test_pending_provenance_is_consumed_not_reused(session: MnemosyneSession) -> None:
    tool = SimpleNamespace(name="lookup_order")
    _run(session.hooks.on_tool_end(_tool_context("call_1"), None, tool, "shipped"))

    _run(session.add_items([{"type": "function_call_output", "call_id": "call_1"}]))
    _run(session.add_items([{"type": "function_call_output", "call_id": "call_1"}]))

    from mnem_openai_agents.session import _read_items

    nodes = _read_items(session._store, "u1")
    assert nodes[0].provenance.source == "lookup_order"
    assert nodes[1].provenance.source is None  # not reused for the second write


def test_on_tool_end_with_no_tool_call_id_is_a_safe_no_op(session: MnemosyneSession) -> None:
    context_without_call_id = SimpleNamespace()  # no tool_call_id attribute at all
    tool = SimpleNamespace(name="lookup_order")
    _run(session.hooks.on_tool_end(context_without_call_id, None, tool, "shipped"))
    assert session._pending_provenance == {}
