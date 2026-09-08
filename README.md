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
  <img src="assets/demo.gif" alt="A support agent is told a past answer was wrong. It runs bisect to find the commit where the belief entered, blame to trace it to a misread billing note, then commits a correction. The history keeps both." width="900" />
</p>

An AI agent builds up memory as it works: facts it learns, decisions it makes. Frameworks store that as state it overwrites as it goes. Mnemosyne gives agent memory what Git gives code: **commits, branches, merge, blame and bisect.** A Rust core, a `mnem` CLI, and a Python SDK. Local, deterministic, no network, no model calls.

## The problem

Your agent runs for an hour, makes forty tool calls, and updates its memory the whole way. Then it gets something wrong, and all you have is the memory as it stands right now. You cannot see when the bad fact got in, what the agent knew before it went off track, or why it believes what it believes. Version control solved exactly this for code.

## What you get

| Command | What it does |
| --- | --- |
| `commit` · `log` | record memory as an immutable, content-addressed snapshot with its provenance, and walk the history |
| `show <commit>` | reconstruct the memory exactly as it stood at any past commit |
| `branch` · `checkout` · `diff` | fork memory for the price of a pointer to explore a hypothesis in isolation |
| `merge` | combine two lines of memory, surfacing conflicts as objects you inspect and resolve |
| `blame` | resolve any belief to the commit (through merges) and the observation that introduced it |
| `bisect` | binary-search a run for the first commit where a wrong belief appears |

<p align="center"><img src="assets/panel-agent.svg" alt="A Python agent loop: a Claude call, store.add with provenance, store.commit, and store.blame tracing a belief to its origin" width="820" /></p>

## Quickstart

```bash
cargo build --release -p mnem-cli      # the mnem binary (not published yet)
pip install -e ".[dev]"                # the mnem python sdk
```

```python
import mnem

store = mnem.init("./agent-memory")
store.add("customer-4821", "on the Enterprise plan",
          provenance=mnem.Provenance(source="ticket-4821"))
store.commit("learn the plan tier", author="support-agent")

b = store.blame("customer-4821")       # which commit set this, and why
print(b.commit[:8], b.provenance.source)
```

## The GitHub for AI agents

Git made source code collaborative; GitHub made it social. As software becomes agents working in teams, they need the same stack underneath. Mnemosyne is building it in three eras:

1. **The substrate** *(now)*: single-agent versioned memory, local and deterministic.
2. **The collaboration layer**: semantic merge that reasons about contradiction, a sync protocol, and a review step before a memory update lands in shared memory. Pull requests, for agent memory.
3. **The platform** *(`1.0`)*: the whole agent (prompt, tools, memory, policy, evals) as one versioned, signed, forkable artefact, with a registry.

The later two are the point. Everything today is `0.0.x` groundwork ([ADR-0010](docs/adr/0010-what-1-0-means.md)).

## Status

| Tag | What landed |
| --- | --- |
| `v0.0.2` | the object store, `commit` · `add` · `log`, the Python binding |
| `v0.0.3` | `branch` · `checkout` · `diff`, time travel |
| `v0.0.4` | deterministic three-way `merge` with conflict objects |
| `v0.0.5` | `blame` · `bisect`, the provenance index |

Next: a LangGraph adapter and an MCP server. See [`ROADMAP.md`](ROADMAP.md) and [`CHANGELOG.md`](CHANGELOG.md).

## How it works

- **`mnem-core` (Rust)**: the object model, the content-addressed store, the commit graph. Never touches the network or a model; a `cargo deny` check enforces it.
- **`mnem`**: the CLI (Rust) and the SDK (Python) built on the core.
- **The store** is a `.mnem/` directory, format specified in [`docs/format/`](docs/format) and frozen for `0.0.x` at `format_version` 1. Decisions live in [`docs/adr/`](docs/adr).

## Prior work

A wave of 2026 research points at this idea (Git4Data, GitOfThoughts, StateFuse, MemTX, LatticeMind), each a paper or a prototype. One finding is worth stating plainly: versioned memory does not make an agent give better answers. What it gives you is history, audit, and safe merging. That is the whole pitch, and it is enough.

## Contributing & licence

Developed by a single maintainer; issues and feedback welcome, but not open to external pull requests at this stage ([`CONTRIBUTING.md`](CONTRIBUTING.md)). Apache 2.0 ([`LICENSE`](LICENSE)).
