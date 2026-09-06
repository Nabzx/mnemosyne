"""Smoke tests for the Python binding (#29).

These check the binding is wired to the core and that errors cross the boundary
as the right exception type (ADR-0004). The behaviour itself is tested in the
Rust core; this is a thin layer.
"""

import json

import pytest

import mnem


def test_version_is_exposed() -> None:
    assert mnem.__version__ == "0.0.1"
    assert mnem.version() == "0.0.1"


def test_commit_and_log_round_trip(tmp_path: object) -> None:
    store = mnem.Store.init(str(tmp_path))
    assert store.head() == "ref: main"
    assert store.format_version() == 1

    store.add("customer-4821", json.dumps("on the Enterprise plan"), source="ticket-4821")
    assert store.staged() == [
        ("customer-4821", store.staged()[0][1]),
    ]

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
    with pytest.raises(mnem.NoStoreError):
        mnem.Store.open(str(tmp_path))


def test_init_twice_raises_store_exists_error(tmp_path: object) -> None:
    mnem.Store.init(str(tmp_path))
    with pytest.raises(mnem.StoreExistsError):
        mnem.Store.init(str(tmp_path))


def test_store_exceptions_share_a_base(tmp_path: object) -> None:
    with pytest.raises(mnem.MnemError):
        mnem.Store.open(str(tmp_path))
