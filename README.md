<div align="center">

# Mnemosyne

**Version control for AI agent memory.**

[![Licence: Apache 2.0](https://img.shields.io/badge/licence-Apache_2.0-3b82f6.svg)](LICENSE)
&nbsp;[![CI](https://github.com/Nabzx/mnemosyne/actions/workflows/ci.yml/badge.svg)](https://github.com/Nabzx/mnemosyne/actions/workflows/ci.yml)
&nbsp;![Status: early development](https://img.shields.io/badge/status-early_development-f59e0b.svg)
&nbsp;![Rust](https://img.shields.io/badge/core-Rust-b7410e.svg)
&nbsp;![Python](https://img.shields.io/badge/sdk-Python-3776ab.svg)

</div>

AI is moving from single prompts to agents that work for hours, and from single agents to teams of them. Those agents build up memory as they go: facts they learn, notes they keep, conclusions they reach. Today that memory is an unversioned blob. It gets overwritten, it cannot be audited, and when two agents share it they overwrite each other.

Mnemosyne treats agent memory the way Git treats code. Every change is a commit. You branch to test an idea and merge it back. You trace any belief to the moment it was formed, and you can rewind to what the agent knew at any point in its run.

Git made collaborative software possible. As software becomes agents, and agents start working in teams, they need the same foundation underneath them. Mnemosyne is building it.

## The problem

An agent's memory is its working understanding of the task: what it has read, what it has decided, what it now believes. Frameworks persist this as a flat store that the agent rewrites in place.

That is fine until something goes wrong. Then you cannot ask when a wrong belief entered, or what evidence it rests on, or what the agent knew before it went off track. You cannot let the agent try three approaches on isolated copies and keep the best. And the moment a second agent writes to the same store, one set of changes silently wins.

These are the problems version control solved for source code forty years ago. Agent memory needs the same treatment, with merge semantics that understand contradiction rather than line diffs.

## What Mnemosyne gives you

The v1 command set, modelled on Git and adapted to memory:

| Command | What it does |
| --- | --- |
| `commit` | record the current memory state as an immutable, content-addressed snapshot with its provenance |
| `branch` and `checkout` | fork memory cheaply to explore a hypothesis in isolation |
| `merge` | combine two lines of memory, surfacing conflicts you can inspect and resolve |
| `blame` | resolve any belief to the commit and the observation that introduced it |
| `bisect` | binary search a run to find the commit where a wrong belief first appeared |
| `log` and time travel | materialise working memory exactly as it stood at any past commit |

It ships as a Rust core, a `mnem` command line tool, a Python SDK, an MCP server, and a LangGraph adapter. v1 is fully local and deterministic. No network, no model calls.

## Status

Early development. v0.1 is in progress. The API shown below is the target for v0.1, not a released interface. `ROADMAP.md` says what lands in each phase.

## Quickstart

```bash
cargo install mnem              # the cli
pip install mnemosyne-agents    # the python sdk
```

```python
import mnem

store = mnem.init("./agent-memory")
store.add(
    "customer-4821",
    "the customer is on the Enterprise plan",
    provenance=mnem.Provenance(source="ticket-4821"),
)
store.commit("learn the plan tier from the support ticket", author="support-agent")

for c in store.log():
    print(c.id[:12], c.message)

# v0.2 and beyond
#   with store.branch("assume-downgrade"): ...   try an idea off the main line
#   store.blame("customer-4821")                 why does the agent believe this?
```

## Where this goes

Mnemosyne is built in three stages, and the later two are the point.

1. **v1, the substrate.** Single-agent versioned memory: commit, branch, merge, blame, bisect, time travel. Local and deterministic.
2. **v2, the multi-agent layer.** Semantic merge that reasons about contradiction, a sync protocol between stores, and a review step so one agent's memory update is checked before it lands in shared memory. Git plus a review queue, for agents.
3. **v3, the agent as a repository.** The whole agent, its prompt, tools, memory, policy and evaluations, as one versioned, signed, forkable artefact, with a registry to publish, discover and improve them.

## How it works

- `mnem-core` (Rust): the object model, the content-addressed store, the commit graph, and every deterministic operation built on them. It never touches the network and never calls a model.
- `mnem` (Rust binary): the command line interface.
- `mnem` (Python): the SDK an agent calls.
- The store is a `.mnem/` directory, in the spirit of `.git/`. Its format is specified in [`docs/format/`](docs/format) and frozen within a major version.

Architecture decisions and their reasoning live in [`docs/adr/`](docs/adr).

## Prior work

A wave of 2026 research points straight at this idea: Git4Data, GitOfThoughts, StateFuse, MemTX and LatticeMind. Every one is a paper or a prototype. Mnemosyne is the engineered implementation they point towards, built to be depended on.

One finding is worth stating plainly. Versioned memory does not make an agent give better answers. What it gives you is history, audit, and safe merging. That is the whole pitch, and it is enough.

## Contributing

Mnemosyne is developed by a single maintainer. Issues and feedback are welcome. The codebase is not open to external pull requests at this stage. See [`CONTRIBUTING.md`](CONTRIBUTING.md).

## Licence

Apache 2.0. See [`LICENSE`](LICENSE).
