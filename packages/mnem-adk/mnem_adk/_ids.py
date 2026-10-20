"""The node id and scope scheme locked in #571 (ADR-0026).

Scoped by a flat ``{app_name}:{user_id}:`` prefix, not a ``/``-path:
``search_memory``'s own real signature takes no ``session_id``, and ADK's
own guide states the scoping rule directly. An ingested event honours
``Event.id`` literally; a direct ``add_memory`` write with its own id uses
that id; one with none gets a zero-padded sequence number, backed by a
reserved counter node excluded from search and content iteration - the
same idiom ADR-0020 and ADR-0023 already use.
"""

from __future__ import annotations

import mnem

_MEMORY_SEGMENT = "_memory"
_SEQ_SUFFIX = "_seq"


def scope_prefix(app_name: str, user_id: str) -> str:
    """The ``{app_name}:{user_id}:`` prefix every node under this scope
    starts with."""
    return f"{app_name}:{user_id}:"


def event_node_id(app_name: str, user_id: str, session_id: str, event_id: str) -> str:
    """The id for one ingested ``Event``, honouring its own real id."""
    return f"{scope_prefix(app_name, user_id)}{session_id}:{event_id}"


def session_prefix(app_name: str, user_id: str, session_id: str) -> str:
    """The prefix every event node under one session starts with."""
    return f"{scope_prefix(app_name, user_id)}{session_id}:"


def memory_node_id(app_name: str, user_id: str, memory_id: str) -> str:
    """The id for a direct ``add_memory`` entry that already has its own id."""
    return f"{scope_prefix(app_name, user_id)}{_MEMORY_SEGMENT}:{memory_id}"


def memory_seq_node_id(app_name: str, user_id: str) -> str:
    """The reserved counter node's id for this scope's own sequence ids."""
    return f"{scope_prefix(app_name, user_id)}{_MEMORY_SEGMENT}:{_SEQ_SUFFIX}"


def next_memory_node_id(app_name: str, user_id: str, seq: int) -> str:
    """The sortable id for the ``seq``-th ``add_memory`` entry with no id of
    its own. Zero-padded to 10 digits so working memory's ``BTreeMap``
    lexicographic order equals insertion order past 9 entries (ADR-0020)."""
    return f"{scope_prefix(app_name, user_id)}{_MEMORY_SEGMENT}:{seq:010d}"


def read_memory_seq(store: mnem.Store, app_name: str, user_id: str) -> int:
    """The next sequence number to assign, reading the reserved counter
    node fresh - 0 if it does not exist yet."""
    node = store.working_node(memory_seq_node_id(app_name, user_id))
    return 0 if node is None else node.content


def is_reserved(node_id: str) -> bool:
    """Whether ``node_id`` is a reserved internal node (the sequence
    counter) that search and content iteration must skip."""
    return node_id.endswith(f":{_MEMORY_SEGMENT}:{_SEQ_SUFFIX}")
