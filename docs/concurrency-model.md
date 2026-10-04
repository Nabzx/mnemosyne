# The concurrency model

**One open handle per store per process.** This is a real constraint of
`redb` (ADR-0008), confirmed directly, not a design choice this project
could lift without changing the storage engine: opening a second
`Store` (or an adapter wrapping one - `MnemosyneStore`,
`MnemosyneStorageBackend`, a `Session`, a `Memory`) on a path a first one
in the *same process* already holds raises

```
StoreIoError: Database already open. Cannot acquire lock.
```

## What this rules out

Two handles open **in the same process, at the same instant**, on the
same store path. A long-running process that opens a store once and
holds it for the process's lifetime is fine; a process that opens,
finishes, and lets the handle drop before a second one opens is fine.
What doesn't work is holding two live handles side by side - the classic
way to hit this by accident is constructing two adapter instances
against the same path in one script without ever letting the first go
out of scope.

## What this does not rule out

**Multiple agents, even across different frameworks, sharing one store**
- just not at the exact same instant from the same process. The normal
git-like pattern already works today: each agent's process opens the
store, does its work, and either closes (handle dropped) or holds for
its own run's lifetime; merging two agents' divergent work happens the
same way any two branches merge, with no special-casing for which
adapter or framework wrote which side (see
`examples/cross_framework_merge.py`). Sequential access across
processes - one agent's run finishes before another's starts, or
they're simply never open on the same path concurrently - is the real,
supported pattern.

## What would need a daemon

True concurrent access - two processes with the store open on the same
path at the same instant, each reading and writing without stepping
on the other - needs a server process mediating access, already noted
as a backlog item in `ROADMAP.md`. Nothing in the current substrate
provides this, and this document should not be read as implying it
does.

---

Related: each adapter's own README documents a "Sharing a store with
other adapters" section stating *what* id shape it owns
([`mnem-crewai`](https://github.com/Nabzx/mnemosyne/blob/main/packages/mnem-crewai/README.md#sharing-a-store-with-other-adapters),
[`mnem-langgraph`](https://github.com/Nabzx/mnemosyne/blob/main/packages/mnem-langgraph/README.md#sharing-a-store-with-other-adapters),
[`mnem-openai-agents`](https://github.com/Nabzx/mnemosyne/blob/main/packages/mnem-openai-agents/README.md#sharing-a-store-with-other-adapters),
[`mnem-autogen`](https://github.com/Nabzx/mnemosyne/blob/main/packages/mnem-autogen/README.md#sharing-a-store-with-other-adapters));
this document covers *when* a store can safely be opened at all.
