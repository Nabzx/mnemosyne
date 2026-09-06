"""Smoke tests for the raw binding, ``mnem._mnem`` (#29).

These check the binding is wired to the core and that errors cross the boundary
as the right exception type (ADR-0004). The behaviour itself is tested in the
Rust core; this is a thin layer. The agent-facing API is covered by
``test_sdk.py``.
"""

import json

import pytest

from mnem import _mnem


def test_version_is_exposed() -> None:
    assert _mnem.__version__ == "0.0.3"
    assert _mnem.version() == "0.0.3"


def test_commit_and_log_round_trip(tmp_path: object) -> None:
    store = _mnem.Store.init(str(tmp_path))
    assert store.head() == "ref: main"
    assert store.format_version() == 1

    store.add("customer-4821", json.dumps("on the Enterprise plan"), source="ticket-4821")
    staged = store.staged()
    assert [node_id for node_id, _ in staged] == ["customer-4821"]

    commit_id = store.commit("learn the plan tier", "demo-agent", 1_757_160_000_000)
    assert len(commit_id) == 64
    assert store.staged() == []

    history = store.log()
    assert len(history) == 1
    assert history[0]["id"] == commit_id
    assert history[0]["message"] == "learn the plan tier"
    assert history[0]["author"] == "demo-agent"
    assert history[0]["parents"] == []


def test_open_missing_store_raises_no_store_error(tmp_path: object) -> None:
    with pytest.raises(_mnem.NoStoreError):
        _mnem.Store.open(str(tmp_path))


def test_init_twice_raises_store_exists_error(tmp_path: object) -> None:
    _mnem.Store.init(str(tmp_path))
    with pytest.raises(_mnem.StoreExistsError):
        _mnem.Store.init(str(tmp_path))


def test_store_exceptions_share_a_base(tmp_path: object) -> None:
    with pytest.raises(_mnem.MnemError):
        _mnem.Store.open(str(tmp_path))


def test_bad_content_kind_is_a_value_error(tmp_path: object) -> None:
    store = _mnem.Store.init(str(tmp_path))
    with pytest.raises(ValueError, match="content_kind"):
        store.add("n", json.dumps("x"), content_kind="fact")
