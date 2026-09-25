# Roadmap

One maintainer, working in linear phases. Every phase ends in a tagged build
that runs, even if narrow. Era 1 carries no dates; Era 2 and Era 3 carry a
soft, revisable estimate once one exists (the dating policy below, #247). A
phase is done when its definition of done is met, not before, and it does not
grow past its goal.

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

**Dating policy (decided, #247):** Era 1's phases above stay dateless - they're
already shipped, tagged, and self-evidently ordered by their own history; a
retrofitted date would be trivia, not information. Era 2 and Era 3, once
detailed, carry a soft, explicitly revisable estimate each - a real sense of
pace, not a deadline. Nothing about "done when its definition of done is met,
not before" changes because of them: an estimate that slips is revised, not
defended. Granularity is months while a phase is still some way off, tightening
to weeks once it's imminent - not quarters, which round away real information
without buying any real precision back.

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

- [x] Rust workspace: `mnem-store` and `mnem-git`, both compiling, `mnem --version` working
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

- [x] The object model from ADR-0002 and ADR-0003, implemented in `mnem-store`
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
- [x] The branch-per-hypothesis pattern in the SDK: `with store.branch("h1"): ...`

**Definition of done:** an agent forks memory, explores on a branch, and either keeps or discards it; time travel reconstructs past working memory exactly. Tag `v0.0.3`.

---

## Phase 3 (v0.0.4): it merges

Deterministic merge, the last purely mechanical piece.

- [x] Three-way merge with a shared base
- [x] Structural conflict detection and conflict objects
- [x] A resolution API: take mine, take theirs, or supply a value
- [x] A property and chaos harness proving merge is order-independent and associative where it must be, in the spirit of the ephor exactly-once tests

**Definition of done:** non-overlapping changes merge clean; overlapping changes produce conflict objects you can inspect and resolve; the harness runs in CI. Tag `v0.0.4`.

---

## Phase 4 (v0.0.5): it explains

The debugging layer, still deterministic.

- [x] The provenance index in `mnem-store`
- [x] `mnem blame <node>`: the introducing commit and its provenance
- [x] `mnem bisect`: find the commit where a supplied predicate first holds
- [x] A synthetic buggy-run fixture the tests bisect against

**Definition of done:** `blame` resolves any node to its origin; `bisect` finds the fault commit over the fixture. Tag `v0.0.5`.

---

## Phase 5 (v0.0.6): it plugs in

Make it usable from a real agent.

- [x] The MCP server: memory operations exposed as tools
- [x] The LangGraph adapter
- [x] SDK hardening: errors, types, transactions
- [x] Three worked examples in `examples/`, each a real agent

**Definition of done:** a LangGraph agent uses Mnemosyne as its memory with branch and blame working; an MCP-capable agent gets the same operations as tools. Tag `v0.0.6`.

---

## Phase 6 (v0.0.7): ship the substrate

- [x] A benchmark measuring what the substrate actually delivers: time-travel reproducibility, blame accuracy, merge correctness, against a no-version-control baseline, framed on the GitOfThoughts methodology
- [x] A short docs site and a recorded terminal demo in the README
- [x] `docs/format/` finalised for the substrate
- [x] Published to crates.io and PyPI
- [x] The Era 2 semantic layer declared behind a trait and a protocol, with no implementation
- [ ] Public launch: Show HN and the relevant communities

**Definition of done:** something you would hand a stranger, with a benchmark and published numbers. Era 1 is complete. Tag `v0.0.7`.

---

## Era 2: the collaboration layer

Detailed once Era 1 has shipped and has users. Stays in the `0.0.x` line.

**No estimate yet.** This era's own precondition - Era 1 shipped with real
users - hasn't happened, and none of its actual design work (a Wayfinder map,
the research and grilling tickets that resolve into ADRs) has started. An
estimate lands once that precondition is met and the map exists, per the
dating policy above - not invented ahead of it. Era 1's own six phases took
four days end to end, but that was already-researched, mechanically-specified
work with known reference designs (a three-way merge, `blame`, `bisect`);
none of that pace evidence transfers to semantic merge, contradiction
objects, or a sync protocol, none of which have been designed yet.

- Semantic diff and merge over claims, with embeddings and an optional LLM adjudicator. Local model by default, an API optional and never used by CI.
- Contradiction objects: incompatible claims kept and surfaced, not dropped.
- The sync protocol: `remote`, `push`, `pull`, `fetch` between stores.
- The review model: an agent proposes a memory update, a reviewer (a policy, a critic agent, or a person) approves it before it lands in shared memory, with full provenance.

## Era 3: the platform, and 1.0

**No estimate yet**, for the same reason as Era 2, more so: there isn't even a
phase-level sketch here yet, just the two bullets below. An estimate lands
once real design work starts, not before.

- The whole agent, its identity, prompt, tools, memory schema, policy and evaluations, as one versioned, signed, forkable artefact.
- A registry to publish, discover, fork, and submit improvements to agents.
- This is `1.0.0` (ADR-0010). When `0.1.0` is cut on the way there, or whether the line goes straight from `0.0.x` to `1.0.0`, is decided when Era 3 is in view.

## Backlog

Not yet placed in an era.

- A weight-level view: LoRA deltas as commits, model merging as merge
- A daemon for long-running stores
- Encryption at rest for the object database

CrewAI and AutoGen adapters were here; both now have real, Accepted ADRs
(ADR-0021, ADR-0023) and a build ticket ahead of them - tracked in epic #232,
not an unplaced backlog item anymore.
