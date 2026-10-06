# Shared memory across frameworks

A practical guide to using one Mnemosyne store as shared memory for agents
built on different frameworks - when to give each agent its own branch,
how a real conflict surfaces and resolves, and what the concurrency model
means for how to actually structure this. Everything below is checked
against two real worked examples, not written from theory:
[`examples/cross_framework_merge.py`](https://github.com/Nabzx/mnemosyne/blob/main/examples/cross_framework_merge.py)
(two agents, one real conflict) and
[`examples/multi_agent_incident_retro.py`](https://github.com/Nabzx/mnemosyne/blob/main/examples/multi_agent_incident_retro.py)
(four agents, a sequential merge).

## Separate branches, or one shared branch?

**Give each agent its own branch when its work is provisional until
reviewed, or genuinely independent of the others' until merge time.**
Both worked examples do this: CrewAI's own branch in
`cross_framework_merge.py`, and all four agents' own branches in
`multi_agent_incident_retro.py`. A branch costs nothing but a pointer
(ADR-0012), so there is no real reason to default to a shared branch just
to save the call to `new_branch`.

**Write straight to a shared branch (usually `main`) when the agents'
writes are already known not to collide** - distinct node ids, no
overlapping scope, and no review step needed before the write counts.
LangGraph's branch in `cross_framework_merge.py` still uses its own
branch even though its write never collides with CrewAI's, because the
point of that example is to demonstrate branching generally; in a real
deployment where you already know two agents never touch the same ids,
writing directly to a shared branch is a reasonable simplification -
the merge algorithm (ADR-0013) still protects you if that assumption
ever turns out to be wrong, since a real collision would simply surface
as a conflict on the next merge rather than silently overwriting.

## How a conflict actually surfaces and resolves

A conflict is only possible when two branches both changed the **same
node id** since they diverged. This matters directly for the first
question above: if every agent sharing a store writes under a
genuinely distinct id scheme (see each adapter's own "Sharing a store
with other adapters" README section, and [the reserved-namespace
finding in #362](https://github.com/Nabzx/mnemosyne/issues/362)), two
agents on different frameworks structurally cannot conflict with each
other at all - which is exactly why `multi_agent_incident_retro.py`'s
four-way merge has no conflict to resolve, only four independent
findings landing cleanly in sequence.

When a real conflict does happen - two branches that both touch the
same node, the way `cross_framework_merge.py`'s two CrewAI branches
do - `merge()` returns a result whose `status` is `"conflicts"`, not
an exception and not a silent pick of one side:

```python
result = store.merge("their-branch", time_ms=...)
if result.status == "conflicts":
    for c in result.conflicts:
        print(c.ours.content, c.theirs.content)
    # decide, then call again with the resolution:
    store.merge(
        "their-branch",
        resolutions={c.id: "theirs" for c in result.conflicts},
        time_ms=...,
    )
```

Nothing is written until every conflict has a resolution - `"ours"`,
`"theirs"`, `"base"`, `"delete"`, or a `MemoryNode` to set directly
(ADR-0014). Which side should win is a decision for whatever is
driving the merge (a human, a review step, a rule), not something this
guide can answer generically - `cross_framework_merge.py` picks
`"theirs"` because the scenario's own later event (an escalation)
should win over an earlier one (a refund), which is a decision specific
to that story, not a default.

## What the concurrency model means for a real deployment

The one constraint that actually shapes how you'd structure this for
real: [redb allows exactly one open handle per store per process](concurrency-model.md).
Every agent in both worked examples opens its own backend, writes, and
lets the handle drop (the function returns) before the next agent opens
its own - never two handles open on the same store path at the same
instant in the same process. In practice, this means:

- **Separate processes, same machine, same store path**: fine, as long
  as they are not literally open at the identical instant. A short-lived
  process (open, write, exit) sharing a store with other short-lived
  processes is the pattern both worked examples model.
- **One long-running process holding the store open for its whole
  lifetime**: also fine, as long as no second process (or second
  in-process handle) ever opens that same path while the first is still
  open.
- **Two agents that need to write at genuinely the same instant, from
  the same process**: not supported by anything in the current
  substrate. This needs a daemon mediating access, already noted as a
  backlog item - don't structure a real deployment around an assumption
  that isn't true yet.

## See also

- [The concurrency model](concurrency-model.md) - the full one-handle
  constraint this guide's last section summarises.
- Each adapter's own README has a "Sharing a store with other adapters"
  section naming its real reserved id shape.
- [#362](https://github.com/Nabzx/mnemosyne/issues/362) - the research
  that found which adapter pairs structurally cannot collide, and which
  pair (the OpenAI Agents SDK and AutoGen) can guarantee one if given
  the same identifier string.
