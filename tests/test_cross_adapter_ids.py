"""Cross-adapter node-id collision risk (#362, ADR-0016/0020/0021/0023).

Lives at the repo root, not in any one package's `tests/`, because this is
genuinely cross-adapter - it needs all four installed together, which no
single package's own CI job does today.

Real finding, verified directly, not assumed: collision across the four
adapters this project ships is **not structurally impossible**, contrary
to the hope "different separator characters make a literal-string
collision unlikely."

- The OpenAI Agents SDK and AutoGen adapters' own node-id schemes are
  byte-for-byte **identical** - `{name}:{seq:010d}` with a `{name}:_seq`
  counter. ADR-0023 explicitly reused ADR-0020's scheme directly (a
  deliberate, documented choice at the time), but nobody checked what
  that means if the same identifier string (a `session_id` for one, a
  `name` for the other) is ever used for both on one shared store: every
  item and the counter node collide, on every single write, not as a
  risk but as a certainty.
- LangGraph's own scheme has no reserved shape of its own at all - a
  caller-controlled `":".join([*namespace, key])` - so a caller's own
  namespace/key choice can reproduce *any* other adapter's exact reserved
  id, including another adapter's own `_seq` counter node specifically,
  which would corrupt that adapter's internal bookkeeping (not just sit
  alongside it harmlessly - `_read_seq` would try to interpret whatever
  LangGraph wrote there as an integer).
- CrewAI's `/`-rooted scheme is the most distinct of the four, but even
  it isn't structurally guaranteed safe against a `:`-based adapter if
  its own caller-supplied scope/record_id happen to contain the other's
  reserved characters.

The real, statable invariant these tests actually prove: collision is
avoided when every adapter sharing one store is given a genuinely
distinct identifier (session_id / name / scope) that doesn't equal
another adapter's own identifier and doesn't reproduce another adapter's
reserved `:_seq` suffix. That's a caller-discipline guarantee, not a
structural one - #365 documents the real contract this finding shows is
actually needed, not just a nice-to-have.
"""

from mnem_autogen.memory import _item_id as _ag_id
from mnem_autogen.memory import _seq_node_id as _ag_seq_id
from mnem_crewai.storage import _node_id as _cw_id
from mnem_langgraph.store import _node_id as _lg_id
from mnem_openai_agents.session import _item_id as _oa_id
from mnem_openai_agents.session import _seq_node_id as _oa_seq_id


def test_openai_agents_sdk_and_autogen_collide_on_a_shared_identifier() -> None:
    """A real, guaranteed collision - not a risk. Both adapters reuse the
    exact same scheme (ADR-0023 reused ADR-0020 directly), so the same
    identifier string produces identical ids for every item and the
    reserved counter node."""
    shared = "shared-name"
    assert _oa_id(shared, 0) == _ag_id(shared, 0)
    assert _oa_seq_id(shared) == _ag_seq_id(shared)


def test_openai_agents_sdk_and_autogen_do_not_collide_with_distinct_identifiers() -> None:
    assert _oa_id("session-a", 0) != _ag_id("agent-b", 0)
    assert _oa_seq_id("session-a") != _ag_seq_id("agent-b")


def test_langgraph_can_reproduce_another_adapters_reserved_shape() -> None:
    """LangGraph's own scheme has no reserved shape of its own - a
    caller-chosen namespace/key CAN match another adapter's exact
    reserved id, including its counter node specifically."""
    assert _lg_id(("agent-a",), "0000000000") == _oa_id("agent-a", 0)
    assert _lg_id(("agent-a",), "0000000000") == _ag_id("agent-a", 0)
    assert _lg_id(("agent-a",), "_seq") == _oa_seq_id("agent-a")
    assert _lg_id(("agent-a",), "_seq") == _ag_seq_id("agent-a")


def test_langgraph_avoids_collision_with_a_distinct_namespace_and_key() -> None:
    """The real, practical mitigation: a namespace/key that doesn't equal
    another adapter's own identifier or its exact reserved suffix."""
    safe = _lg_id(("langgraph-own-namespace",), "a-real-key")
    assert safe != _oa_id("langgraph-own-namespace", 0)
    assert safe != _oa_seq_id("langgraph-own-namespace")
    assert "/" not in safe  # would need this to literally match CrewAI's own shape


def test_crewai_is_distinct_from_the_colon_based_schemes_with_real_identifiers() -> None:
    """CrewAI's own scheme is rooted at "/" - distinct from the three
    `:`-based schemes as long as the caller's own scope/record_id don't
    themselves contain a `:`-based adapter's reserved shape."""
    cw = _cw_id("/acme/billing", "a-real-record-id")
    assert ":" not in cw
    assert cw != _oa_id("acme", 0)
    assert cw != _ag_id("acme", 0)
