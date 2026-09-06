# Roadmap

One maintainer, working in linear phases. Every phase ends in a tagged build
that runs, even if narrow. There are no dates. A phase is done when its
definition of done is met, not before, and it does not grow past its goal.

The label taxonomy carries `track-a` (core and format) and `track-b` (SDK,
adapters, developer experience) so a second maintainer can slot in later without
reshaping the work. Until then the phases run in order.

## Ground rules

- Every change lands through a pull request with green CI, even though there is
  one maintainer. The trail is part of the point.
- An ADR is written before code for anything touching the on-disk format, the
  object model, a public API surface, the merge algorithm, the Rust and Python
  boundary, or a new dependency.
- Each phase opens with a Wayfinder map issue that names the decisions the phase
  depends on. Research and grilling tickets resolve them into ADRs before the
  build tickets start.

---

## Phase 0 — Bootstrap

Get to "clone and build" with the process scaffolding in place.

- [ ] Rust workspace: `mnem-core` (library) and `mnem-cli` (the `mnem` binary), both compiling, `mnem --version` working
- [ ] Python SDK skeleton: `mnem` installs and imports, exposes its version, raises a clear error on any unbuilt API
- [ ] CI on every pull request: `cargo fmt`, `cargo clippy -D warnings`, `cargo test`, `cargo build`, plus `ruff` and `pytest`
- [ ] Docs: `README.md`, `CONTEXT.md`, this file, `AGENTS.md`, `CONTRIBUTING.md`, the ADR template and ADR-0001, the agent workflow docs
- [ ] Labels, milestones `v0.1` to `v1.0`, and a project board
- [ ] The Phase 0 Wayfinder map and its child tickets: ADR-0002 (on-disk object format), ADR-0003 (the memory node model), ADR-0004 (Rust core and Python SDK boundary)

**Definition of done:** `cargo build` and `cargo test` pass in CI; `pip install -e .` gives a working `import mnem`; `mnem --version` runs. No features. Tag `v0.0.1`.

---

## Phase 1 — v0.1: it commits

The content-addressed object store and the commit graph.

- [ ] The object model from ADR-0002 and ADR-0003, implemented in `mnem-core`
- [ ] `mnem init`, `mnem add`, `mnem commit`, `mnem log`
- [ ] The Rust core bound into the Python SDK through maturin (the boundary set by ADR-0004)
- [ ] Python SDK: `Store.open`, `store.add`, `store.commit`, `store.log`
- [ ] The on-disk format written up in `docs/format/` and frozen for the v0.x line

**Definition of done:** an agent loop can persist its memory as a commit history and read it back, with round trips verified byte for byte. Tag `v0.1`.

---

## Phase 2 — v0.2: it branches and travels

Isolation and history.

- [ ] Branches, `mnem checkout`, and refs
- [ ] Time travel: materialise working memory at any past commit
- [ ] `mnem diff`, structural, between two commits
- [ ] The branch-per-hypothesis pattern in the SDK: `with store.branch("h1"): ...`

**Definition of done:** an agent forks memory, explores on a branch, and either keeps or discards it; time travel reconstructs past working memory exactly. Tag `v0.2`.

---

## Phase 3 — v0.3: it merges

Deterministic merge, the last purely mechanical piece.

- [ ] Three-way merge with a shared base
- [ ] Structural conflict detection and conflict objects
- [ ] A resolution API: take mine, take theirs, or supply a value
- [ ] A property and chaos harness proving merge is order-independent and associative where it must be, in the spirit of the ephor exactly-once tests

**Definition of done:** non-overlapping changes merge clean; overlapping changes produce conflict objects you can inspect and resolve; the harness runs in CI. Tag `v0.3`.

---

## Phase 4 — v0.4: it explains

The debugging layer, still deterministic.

- [ ] The provenance index in `mnem-core`
- [ ] `mnem blame <node>`: the introducing commit and its provenance
- [ ] `mnem bisect`: find the commit where a supplied predicate first holds
- [ ] A synthetic buggy-run fixture the tests bisect against

**Definition of done:** `blame` resolves any node to its origin; `bisect` finds the fault commit over the fixture. Tag `v0.4`.

---

## Phase 5 — v0.5: it plugs in

Make it usable from a real agent.

- [ ] The MCP server: memory operations exposed as tools
- [ ] The LangGraph adapter
- [ ] SDK hardening: errors, types, transactions
- [ ] Three worked examples in `examples/`, each a real agent

**Definition of done:** a LangGraph agent uses Mnemosyne as its memory with branch and blame working; an MCP-capable agent gets the same operations as tools. Tag `v0.5`.

---

## Phase 6 — v1.0: launch

- [ ] A benchmark measuring what v1 actually delivers: time-travel reproducibility, blame accuracy, merge correctness, against a no-version-control baseline, framed on the GitOfThoughts methodology
- [ ] A short docs site and a recorded terminal demo in the README
- [ ] `docs/format/` finalised for v1
- [ ] Published to crates.io and PyPI
- [ ] The v2 semantic layer declared behind a trait and a protocol, with no implementation
- [ ] Public launch: Show HN and the relevant communities

**Definition of done:** something you would hand a stranger, with a benchmark and published numbers. Tag `v1.0.0`.

---

## Beyond v1

Detailed only once v1 has shipped and has users.

### v2.0 — the multi-agent layer

- Semantic diff and merge over claims, with embeddings and an optional LLM adjudicator. Local model by default, an API optional and never used by CI.
- Contradiction objects: incompatible claims kept and surfaced, not dropped.
- The sync protocol: `remote`, `push`, `pull`, `fetch` between stores.
- The review model: an agent proposes a memory update, a reviewer (a policy, a critic agent, or a person) approves it before it lands in shared memory, with full provenance.

### v3.0 — the agent as a repository

- The whole agent, its identity, prompt, tools, memory schema, policy and evaluations, as one versioned, signed, forkable artefact.
- A registry to publish, discover, fork, and submit improvements to agents.

### Backlog

- CrewAI and AutoGen adapters
- A weight-level view: LoRA deltas as commits, model merging as merge
- A daemon for long-running stores
- Encryption at rest for the object database
