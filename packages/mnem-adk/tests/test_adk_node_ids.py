"""Tests for the node id and scope helpers (#588, #571, ADR-0026).

Named test_adk_node_ids.py, not test_ids.py or test_node_ids.py: no
tests/ directory here has an __init__.py, so pytest collects every
package's tests as bare top-level modules in the shared `adapters`
CI job - a same-named file in two packages is a real "import file
mismatch" collection error, not a lint nitpick. mnem-openai-agents
already has test_ids.py and mnem-autogen already has
test_node_ids.py; found both the hard way. Prefixing with adk_ keeps
this file unique regardless of what any other adapter is named.
"""

import mnem

from mnem_adk._ids import (
    event_node_id,
    is_reserved,
    memory_node_id,
    memory_seq_node_id,
    next_memory_node_id,
    read_memory_seq,
    scope_prefix,
    session_prefix,
)


def test_scope_prefix_is_app_and_user_only() -> None:
    assert scope_prefix("myapp", "u1") == "myapp:u1:"


def test_event_node_id_honours_the_event_id_literally() -> None:
    assert event_node_id("myapp", "u1", "s1", "ev-abc") == "myapp:u1:s1:ev-abc"


def test_session_prefix_is_a_prefix_of_its_own_event_ids() -> None:
    prefix = session_prefix("myapp", "u1", "s1")
    assert event_node_id("myapp", "u1", "s1", "ev-abc").startswith(prefix)


def test_different_sessions_do_not_share_a_prefix() -> None:
    assert session_prefix("myapp", "u1", "s1") != session_prefix("myapp", "u1", "s2")


def test_different_users_do_not_share_a_scope() -> None:
    assert scope_prefix("myapp", "u1") != scope_prefix("myapp", "u2")


def test_memory_node_id_with_an_explicit_id() -> None:
    assert memory_node_id("myapp", "u1", "mem-1") == "myapp:u1:_memory:mem-1"


def test_next_memory_node_id_is_zero_padded() -> None:
    assert next_memory_node_id("myapp", "u1", 3) == "myapp:u1:_memory:0000000003"


def test_memory_seq_node_id_is_reserved() -> None:
    seq_id = memory_seq_node_id("myapp", "u1")
    assert is_reserved(seq_id)


def test_event_and_explicit_memory_node_ids_are_not_reserved() -> None:
    assert not is_reserved(event_node_id("myapp", "u1", "s1", "ev-1"))
    assert not is_reserved(memory_node_id("myapp", "u1", "mem-1"))
    assert not is_reserved(next_memory_node_id("myapp", "u1", 0))


def test_read_memory_seq_is_zero_before_anything_is_written(tmp_path: object) -> None:
    store = mnem.init(tmp_path)
    assert read_memory_seq(store, "myapp", "u1") == 0


def test_read_memory_seq_reads_the_counter_node(tmp_path: object) -> None:
    store = mnem.init(tmp_path)
    store.add(memory_seq_node_id("myapp", "u1"), 5)
    store.commit("bump counter", author="test")
    assert read_memory_seq(store, "myapp", "u1") == 5


def test_read_memory_seq_is_scoped_per_app_and_user(tmp_path: object) -> None:
    store = mnem.init(tmp_path)
    store.add(memory_seq_node_id("myapp", "u1"), 5)
    store.commit("bump counter for u1", author="test")
    assert read_memory_seq(store, "myapp", "u2") == 0
