<div align="center">

# Mnemosyne

**Version control for AI agent memory.**

[![Licence: Apache 2.0](https://img.shields.io/badge/licence-Apache_2.0-3b82f6.svg)](LICENSE)
&nbsp;[![CI](https://github.com/Nabzx/mnemosyne/actions/workflows/ci.yml/badge.svg)](https://github.com/Nabzx/mnemosyne/actions/workflows/ci.yml)
&nbsp;![Status: early development](https://img.shields.io/badge/status-early_development-f59e0b.svg)
&nbsp;![Rust](https://img.shields.io/badge/core-Rust-b7410e.svg)
&nbsp;![Python](https://img.shields.io/badge/sdk-Python-3776ab.svg)

</div>

<p align="center">
  <img src="assets/demo.gif" alt="mnem: record memory, fork it on a branch, diff the two lines" width="860" />
</p>

An AI agent builds up memory as it works: facts it learns, decisions it makes, conclusions it reaches. Frameworks store that as state the agent overwrites as it goes.

Mnemosyne gives agent memory what Git gives code: commits, history, branches, merge, and blame. It is a Rust core with a `mnem` CLI and a Python SDK. Local, deterministic, no network, no model calls.

## The problem

Your agent runs for an hour, makes forty tool calls, and updates its memory the whole way. Then it gets something wrong.

You go to debug it, and all you have is the memory as it stands right now. You cannot see when a bad fact got in, what the agent knew before it went off track, or why it believes what it believes. You cannot fork the memory, try three approaches, and keep the best. And when two agents share one store, the last write wins.

Version control solved exactly this for code. Agent memory needs the same, with merges that understand contradiction, not line diffs.

## What you get

A command set modelled on Git and adapted to memory:

| Command | What it does |
| --- | --- |
| `commit` | record the current memory state as an immutable, content-addressed snapshot with its provenance |
| `branch` and `checkout` | fork memory cheaply to explore a hypothesis in isolation |
| `merge` | combine two lines of memory, surfacing conflicts you can inspect and resolve |
| `blame` | resolve any belief to the commit and the observation that introduced it |
| `bisect` | binary search a run to find the commit where a wrong belief first appeared |
| `log` and time travel | materialise working memory exactly as it stood at any past commit |

It ships as a Rust core, a `mnem` CLI, a Python SDK, and later an MCP server and a LangGraph adapter.

## Status

Early development, and numbered to say so: the whole current roadmap is `0.0.x` (ADR-0010). `v0.0.2` ships `commit`, `add` and `log`; `branch`, `merge` and `blame` are the next phases. `ROADMAP.md` says what lands when.

## Quickstart

```bash
# not published yet; build from source
cargo build --release -p mnem-cli      # the mnem binary
pip install -e ".[dev]"                # the python sdk
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

# try an idea on a branch, keep it or throw it away
with store.branch("assume-downgrade"):
    store.add("customer-4821", "the customer downgraded to Pro")
    store.commit("assume the downgrade", author="support-agent")

for c in store.log():
    print(c.id[:12], c.message)

# later phases
#   store.blame("customer-4821")   why does the agent believe this?  (Phase 4)
```

## The GitHub for AI agents

Git made source code collaborative. GitHub made it social: pull requests, review, forks, and a registry the whole ecosystem is built on. As software becomes agents, and agents start working in teams, they need that same stack underneath them. Mnemosyne is building it, in three eras.

1. **The substrate** (now). Single-agent versioned memory: commit, branch, merge, blame, bisect, time travel. Local and deterministic.
2. **The collaboration layer.** Semantic merge that reasons about contradiction, a sync protocol between stores, and a review step so one agent's memory update is checked before it lands in shared memory. Pull requests, for agent memory.
3. **The platform** (`1.0`). The whole agent, its prompt, tools, memory, policy and evaluations, as one versioned, signed, forkable artefact, with a registry to publish, discover and improve them. The GitHub for AI agents.

The later two are the point. Everything today is `0.0.x` groundwork (ADR-0010).

## How it works

- `mnem-core` (Rust): the object model, the content-addressed store, the commit graph, and every deterministic operation built on them. It never touches the network and never calls a model, and a CI check (`cargo deny`) bans every network and TLS crate from its dependency tree.
- `mnem` (Rust binary): the command line interface.
- `mnem` (Python): the SDK an agent calls.
- The store is a `.mnem/` directory, in the spirit of `.git/`. Its format is specified in full in [`docs/format/`](docs/format) and frozen for the `0.0.x` line at `format_version` 1.

Architecture decisions and their reasoning live in [`docs/adr/`](docs/adr).

## Prior work

A wave of 2026 research points straight at this idea: Git4Data, GitOfThoughts, StateFuse, MemTX and LatticeMind. Every one is a paper or a prototype. Mnemosyne is the engineered implementation they point towards, built to be depended on.

One finding is worth stating plainly. Versioned memory does not make an agent give better answers. What it gives you is history, audit, and safe merging. That is the whole pitch, and it is enough.

## Contributing

Mnemosyne is developed by a single maintainer. Issues and feedback are welcome. The codebase is not open to external pull requests at this stage. See [`CONTRIBUTING.md`](CONTRIBUTING.md).

## Licence

Apache 2.0. See [`LICENSE`](LICENSE).
