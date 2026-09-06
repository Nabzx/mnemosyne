# Roadmap

One maintainer, working in linear phases. Every phase ends in a tagged build
that runs, even if narrow. There are no dates. A phase is done when its
definition of done is met, not before, and it does not grow past its goal.

Mnemosyne is built in three eras (ADR-0010). **Era 1, the substrate**, is
single-agent versioned memory and is the six phases below. **Era 2, the
collaboration layer**, adds semantic merge, a sync protocol and a review step.
**Era 3, the platform**, is the agent as a versioned, signed, forkable artefact
plus a registry, and ships as `1.0.0`. Everything before Era 3 is `0.0.x`
initial development: assume nothing about API or format stability, and read the
`format_version` integer, not the software version, for the data contract.

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

## The tracker

Every phase is a kept-open map issue with its child tickets in a checklist
comment. The full backlog is filed ahead of time.

| Phase | Map | Milestone |
| --- | --- | --- |
| 0 | [#1](https://github.com/Nabzx/mnemosyne/issues/1) | v0.0.1 |
| 1 | [#9](https://github.com/Nabzx/mnemosyne/issues/9) | v0.0.2 |
| 2 | [#10](https://github.com/Nabzx/mnemosyne/issues/10) | v0.0.3 |
| 3 | [#11](https://github.com/Nabzx/mnemosyne/issues/11) | v0.0.4 |
| 4 | [#12](https://github.com/Nabzx/mnemosyne/issues/12) | v0.0.5 |
| 5 | [#13](https://github.com/Nabzx/mnemosyne/issues/13) | v0.0.6 |
| 6 | [#14](https://github.com/Nabzx/mnemosyne/issues/14) | v0.0.7 |

Phases 1 to 6 are Era 1. Era 2 and Era 3 are sketched at the end of this file
and detailed once Era 1 has shipped.

Board: [Mnemosyne roadmap](https://github.com/users/Nabzx/projects/1).

---

## Phase 0: Bootstrap: done

Get to "clone and build" with the process scaffolding in place.

- [x] Rust workspace: `mnem-core` and `mnem-cli`, both compiling, `mnem --version` working
- [x] Python SDK skeleton: installs, imports, raises a clear error on any unbuilt API
- [x] CI on every pull request: `cargo fmt`, `cargo clippy -D warnings`, `cargo test`, `cargo build`, an `msrv` job, plus `ruff` and `pytest`
- [x] Docs: `README.md`, `CONTEXT.md`, this file, `AGENTS.md`, `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, `CHANGELOG.md`, the ADR template, the agent workflow docs
- [x] Labels, the roadmap milestones, and a project board
- [x] The Phase 0 Wayfinder map (#1) and the full backlog, plus the Phase 1 to 6 maps
- [x] ADR-0001 to ADR-0005 and ADR-0007 Accepted

**Definition of done:** `cargo build` and `cargo test` pass in CI; `pip install -e .` gives a working `import mnem`; `mnem --version` runs. No features. Tagged `v0.0.1`. Met.

---

## Phase 1 (v0.0.2): it commits

The content-addressed object store and the commit graph.

- [x] The object model from ADR-0002 and ADR-0003, implemented in `mnem-core`
- [x] `mnem init`, `mnem add`, `mnem commit`, `mnem log`
- [x] The Rust core bound into the Python SDK through maturin (the boundary set by ADR-0004)
- [x] Python SDK: `Store.open`, `store.add`, `store.commit`, `store.log`
- [x] The on-disk format written up in `docs/format/` and frozen for the `0.0.x` line

**Definition of done:** an agent loop can persist its memory as a commit history and read it back, with round trips verified byte for byte. Tag `v0.0.2`.

---

## Phase 2 (v0.0.3): it branches and travels

Isolation and history.

- [x] Branches, `mnem checkout`, and refs
- [x] Time travel: materialise working memory at any past commit
- [x] `mnem diff`, structural, between two commits
- [ ] The branch-per-hypothesis pattern in the SDK: `with store.branch("h1"): ...`

**Definition of done:** an agent forks memory, explores on a branch, and either keeps or discards it; time travel reconstructs past working memory exactly. Tag `v0.0.3`.

---

## Phase 3 (v0.0.4): it merges

Deterministic merge, the last purely mechanical piece.

- [ ] Three-way merge with a shared base
- [ ] Structural conflict detection and conflict objects
- [ ] A resolution API: take mine, take theirs, or supply a value
- [ ] A property and chaos harness proving merge is order-independent and associative where it must be, in the spirit of the ephor exactly-once tests

**Definition of done:** non-overlapping changes merge clean; overlapping changes produce conflict objects you can inspect and resolve; the harness runs in CI. Tag `v0.0.4`.

---

## Phase 4 (v0.0.5): it explains

The debugging layer, still deterministic.

- [ ] The provenance index in `mnem-core`
- [ ] `mnem blame <node>`: the introducing commit and its provenance
- [ ] `mnem bisect`: find the commit where a supplied predicate first holds
- [ ] A synthetic buggy-run fixture the tests bisect against

**Definition of done:** `blame` resolves any node to its origin; `bisect` finds the fault commit over the fixture. Tag `v0.0.5`.

---

## Phase 5 (v0.0.6): it plugs in

Make it usable from a real agent.

- [ ] The MCP server: memory operations exposed as tools
- [ ] The LangGraph adapter
- [ ] SDK hardening: errors, types, transactions
- [ ] Three worked examples in `examples/`, each a real agent

**Definition of done:** a LangGraph agent uses Mnemosyne as its memory with branch and blame working; an MCP-capable agent gets the same operations as tools. Tag `v0.0.6`.

---

## Phase 6 (v0.0.7): ship the substrate

- [ ] A benchmark measuring what the substrate actually delivers: time-travel reproducibility, blame accuracy, merge correctness, against a no-version-control baseline, framed on the GitOfThoughts methodology
- [ ] A short docs site and a recorded terminal demo in the README
- [ ] `docs/format/` finalised for the substrate
- [ ] Published to crates.io and PyPI
- [ ] The Era 2 semantic layer declared behind a trait and a protocol, with no implementation
- [ ] Public launch: Show HN and the relevant communities

**Definition of done:** something you would hand a stranger, with a benchmark and published numbers. Era 1 is complete. Tag `v0.0.7`.

---

## Era 2: the collaboration layer

Detailed once Era 1 has shipped and has users. Stays in the `0.0.x` line.

- Semantic diff and merge over claims, with embeddings and an optional LLM adjudicator. Local model by default, an API optional and never used by CI.
- Contradiction objects: incompatible claims kept and surfaced, not dropped.
- The sync protocol: `remote`, `push`, `pull`, `fetch` between stores.
- The review model: an agent proposes a memory update, a reviewer (a policy, a critic agent, or a person) approves it before it lands in shared memory, with full provenance.

## Era 3: the platform, and 1.0

- The whole agent, its identity, prompt, tools, memory schema, policy and evaluations, as one versioned, signed, forkable artefact.
- A registry to publish, discover, fork, and submit improvements to agents.
- This is `1.0.0` (ADR-0010). When `0.1.0` is cut on the way there, or whether the line goes straight from `0.0.x` to `1.0.0`, is decided when Era 3 is in view.

## Backlog

Not yet placed in an era.

- CrewAI and AutoGen adapters
- A weight-level view: LoRA deltas as commits, model merging as merge
- A daemon for long-running stores
- Encryption at rest for the object database
