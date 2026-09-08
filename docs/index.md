# Mnemosyne

**Version control for AI agent memory.**

An AI agent builds up memory as it works: facts it learns, decisions it makes.
Frameworks store that as state it overwrites as it goes. Mnemosyne gives agent
memory what Git gives code: commits, branches, merge, blame and bisect. A Rust
core, a `mnem` CLI, and a Python SDK. Local, deterministic, no network, no model
calls.

This is the reference documentation. For the project overview, the quickstart
and the demo, see the [README on GitHub](https://github.com/Nabzx/mnemosyne).

## What is here

- **On-disk format**: the `.mnem/` store, specified in enough detail to write a
  reader in another language. Final for Era 1 at `format_version` 1.
- **Benchmark report**: what the substrate delivers, measured. Reconstruction,
  `bisect` precision, `blame` accuracy, merge correctness, and the overhead
  against a dict and a JSONL log.
- **Merge chaos report**: the merge invariants over a 50,000-case seeded sweep.
- **Architecture decisions**: every decision that shaped Mnemosyne, and why it
  went the way it did. Accepted before the code.
- **Research**: the surveys that fed the decisions.
- **Progress notes**: what shipped in each release, in plain language.

Use the sidebar.

## The three eras

1. **The substrate** *(now, `0.0.x`)*: single-agent versioned memory. Local and
   deterministic.
2. **The collaboration layer**: semantic merge that reasons about contradiction,
   a sync protocol between stores, and a review step. Pull requests, for agent
   memory.
3. **The platform** *(`1.0`)*: the whole agent as one versioned, signed,
   forkable artefact, with a registry. The GitHub for AI agents.

See ADR-0010.
