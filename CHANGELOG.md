# Changelog

All notable changes to Mnemosyne are recorded here, in the
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format. Versioning
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html); the on-disk
`format_version` runs on its own track (see
[ADR-0007](docs/adr/0007-versioning-and-release-policy.md)).

## [Unreleased]

### Changed

- README: the quickstart now installs the published packages (`cargo install
  mnem-git`, `pip install mnem-agents`) instead of the pre-publish build-from-
  source workaround, and gains a CLI-only walkthrough alongside the Python
  one. The status table catches up through `v0.0.7`. The badge row adds
  crates.io, PyPI and docs-site badges; the "Prior work" section links
  `docs/benchmark.md`, the evidence for the claim right above it.

### Fixed

- Every relative link and image path in a file that ships standalone to a
  registry (the root `README.md`, published as the `mnem-agents` PyPI
  description; `packages/mnem-mcp/README.md`; `packages/mnem-langgraph/README.md`)
  now points at an absolute `github.com`/`raw.githubusercontent.com` URL. The
  relative paths rendered as dead links, and the demo GIF and the agent-loop
  SVG did not render at all, on the PyPI project pages.
- `packages/mnem-mcp/README.md`: dropped the stale "not published yet" caveat,
  and corrected "it opens the store fresh each time" to describe what the code
  actually does (one handle opened at startup and reused; redb allows a single
  open handle per file per process).

## [0.0.7] - 2026-09-09

Era 1 complete: the substrate, with a benchmark and a docs site. `format_version` 1.

### Added

- ADR-0017 (the benchmark and its metrics): the `v0.0.7` benchmark measures
  operational properties (reconstruction, `bisect` precision, `blame` accuracy,
  merge correctness) at a hard 100% target and overhead (latency, `.mnem`
  growth) against a `dict` and a JSONL baseline, not accuracy. A seeded
  `benchmark.rs` plus `benchmarks/overhead.py`, feeding `docs/benchmark.md`.
- The benchmark harnesses: `crates/mnem-store/tests/benchmark.rs`
  (correctness metrics, seeded, `MNEM_BENCH_*` overrides) and `benchmarks/overhead.py` (the
  overhead comparison and the audit-query table). A new `benchmark` CI job runs
  the latter with a 2x-regression gate against `benchmarks/baseline.json`.
- `docs/benchmark.md`: the first published run. All eight correctness metrics
  hold over 720 seeded runs; a commit costs about 12 ms and single-digit KB;
  the audit-query table shows what version control buys over a dict and a JSONL
  log.
- A docs site (mdBook, `book.toml` + `docs/SUMMARY.md`), published to GitHub
  Pages by a `docs` workflow: the format spec, the benchmark, the ADRs and the
  research surveys, browsable.
- ADR-0018 (the Era 2 seam): the `SemanticMerge` trait, where a content-aware
  merge resolver attaches to the structural merge, and the named pieces of the
  sync protocol (a `Remote`, content-addressed exchange, a reviewable
  `Proposal`). The trait and its Era 1 impl `StructuralOnly` ship now in
  `mnem-store`; `Store::merge_with(theirs, resolver, ..)` runs a resolver over
  the conflicts the structural merge leaves open, and `Store::merge` is
  `merge_with(&StructuralOnly, ..)`. No behaviour change, no `format_version`
  change: the seam is proven before the Era 1 format freezes.
- `examples/claude_agent.py`: a support agent is told a past answer was wrong,
  then uses `bisect` and `blame` to trace it to a misread observation and
  commits a correction. Runs from a fixed script by default (CI checks it) or
  as a real Claude tool-use loop with `--live`. This is the new README demo.

### Changed

- **Crates and packages renamed for the first publish**, because `mnem-core`
  and `mnem-cli` were already taken on crates.io by an unrelated project. The
  core crate is now **`mnem-store`** (imported as `mnem_store`), the CLI crate
  is **`mnem-git`** (the binary is still `mnem`), and the binding crate is
  **`mnem-py`** (never published). On PyPI the SDK is **`mnem-agents`**
  (`import mnem`), the MCP server is **`mnem-mcp`**, and the LangGraph adapter
  is **`mnem-langgraph`**. The `mnem` binary and the `.mnem/` store directory
  are unchanged. No behaviour or `format_version` change. ADRs through 0018
  still refer to the core and CLI crates as `mnem-core` / `mnem-cli`.
- The README demo GIF now shows the `claude_agent.py` session rather than a
  raw `mnem` CLI walkthrough.
- `docs/format/`: the on-disk format is declared **final for Era 1** at
  `format_version` 1. The next change is `format_version` 2 in Era 2.

## [0.0.6] - 2026-09-08

Era 1, "it plugs in". `format_version` 1.

### Added

- `mnem` CLI: coloured output when stdout is a terminal, covering commit ids,
  branch names, `diff` and `show --stat` markers, and the `blame` / `bisect`
  provenance. Output stays plain when piped, redirected, or when `NO_COLOR` is
  set.
- ADR-0016 (the MCP tools and the adapter contract): the `mnem-mcp` tool and
  resource surface, a no-staging one-commit-per-call write contract, stateless
  one-store-per-process binding, a shared `mnem.agents` helper module, and the
  LangGraph `BaseStore` mapping. Packaged under `packages/`; stdio only for
  `v0.0.6`.
- Python SDK: `mnem.agents` with `remember` / `remember_many` / `forget`, each a
  stage-then-commit in one call with a bounded retry on a lost write race. The
  `Blame` / `NodeChange` / `Commit` / `MemoryNode` / `Conflict` / `MergeResult`
  / `Provenance` dataclasses gain `to_dict()` for the JSON boundary. `Store`'s
  docstring documents that a handle is one-per-thread.
- `mnemosyne-mcp` (`packages/mnem-mcp/`): an MCP server exposing a store as
  tools (`remember`, `recall`, `why`, `when_did`, ...) and resources
  (`mnem://memory`, `mnem://log`, `mnem://commit/{id}`). `mnem-mcp --store PATH`
  speaks MCP over stdio; `--tools all` adds `branch` / `switch` / `merge`.
- `mnemosyne-langgraph` (`packages/mnem-langgraph/`): `MnemosyneStore`, a
  LangGraph `BaseStore` whose every `put` is a commit. Namespace plus key maps
  to a node id, a reserved `_meta` key carries provenance, `search` is a plain
  filter, and `branch` / `why` / `history` are extra methods for a graph node.
  Sync, with async methods that run in a worker thread.
- `examples/`: three runnable, self-asserting scripts, one per surface: a
  support agent using the SDK (`support_agent.py`), a LangGraph agent using
  `MnemosyneStore` (`langgraph_memory.py`), and an MCP client driving `mnem-mcp`
  over stdio (`mcp_client.py`). CI runs all three.

### Changed

- README: rebuilt and trimmed. A colour demo GIF leads the page, running the
  full arc (record, branch, the bug, then `bisect` and `blame` finding it),
  followed by a compact command table, an SVG panel of a Claude agent loop
  calling the SDK, and a one-block quickstart.

## [0.0.5] - 2026-09-07

Era 1, "it explains". `format_version` 1.

### Added

- ADR-0015 (the provenance index, blame and bisect): `blame` is a first-parent
  walk to the commit that changed a node's value (following the contributing
  parent through merges); `bisect` binary-searches the first-parent chain with a
  state predicate; a `commit_nodes` reverse index (`commit -> changed node ids`)
  is built in the commit transaction. The forward index and the reflog are
  deferred with named triggers.
- `mnem-core`: the `commit_nodes` index. `commit` and `merge` record each
  commit's change set (`ChangeKind` per node id) against its first parent, in
  the commit's own write transaction. `Store::changed_by` reads it, recomputing
  on a miss; `Store::rebuild_index` rewrites the table from history.
- `mnem-core`: `Store::blame(node_id, at)` returning a `Blame` (commit, node,
  effective time). Walks to the commit that set the node's current value,
  following the contributing parent through merges; errors if the node is absent
  at `at`.
- `mnem-core`: `Store::bisect(bad, good, predicate)` — binary-search the
  first-parent chain for the first commit where a state predicate holds. `bad`
  defaults to `HEAD`, `good` to the chain's root. Predicate constructors
  `bisect::node_content_is` / `node_absent` / `node_present`.
- `mnem` CLI: `blame <node-id> [<commit>]`, `bisect --node <id>
  (--equals <json> | --absent | --present) [--good <commit>] [<bad>]` (which also
  prints the boundary's `blame`), and `show <commit> --stat`.
- Python SDK: `store.blame(node_id, at)` returning a `Blame` (with a
  `provenance` shortcut), `store.bisect(predicate, *, bad, good)` taking a
  callable over `{id: MemoryNode}`, and `store.changed_by(commit)`.
- The buggy-run fixture (`crates/mnem-core/tests/buggy_run.rs`): a synthetic
  support-agent run where a misread observation writes a wrong plan tier;
  `bisect` lands on that commit and `blame` surfaces the observation. Phase 4's
  definition of done.

## [0.0.4] - 2026-09-07

Era 1, "it merges". `format_version` 1.

### Added

- `mnem-core`: `Store::ancestors` / `is_ancestor` / `merge_bases` / `merge_base`
  (commit-graph queries for Phase 3's merge; `merge_base` refuses a criss-cross
  history).
- ADR-0013 (the deterministic merge algorithm): a structural three-way merge
  over the `State` map, a single required merge base, fast-forward, and a
  stateless merge call. ADR-0014 (the conflict object and resolution API):
  transient `Conflict` values and `ours` / `theirs` / `base` / `delete` /
  `set` resolutions.
- `mnem-core`: the merge algorithm. `merge::merge_state_maps` (the per-id
  three-way table), `Conflict` / `ConflictKind` / `StateMerge`, and
  `Store::merge_states`.
- `mnem-core`: `Store::merge` (the merge flow): merge base, fast-forward,
  `Resolution` (`Ours` / `Theirs` / `Base` / `Delete` / `Set`), `MergeStrategy`,
  and `MergeOutcome`. Stateless: a conflicted merge writes nothing.
- `mnem` CLI: `merge <theirs>` with `--resolve <id>=<ours|theirs|base|delete>`,
  `--strategy ours|theirs`, `--message` and `--author`. A conflicted merge
  prints each conflict's three sides and exits non-zero.
- Python SDK: `store.merge(theirs, resolutions=..., strategy=...)` returning a
  `MergeResult` (`status` / `commit` / `conflicts`), with `Conflict` dataclasses
  carrying the base, ours and theirs nodes.
- The merge chaos harness (`crates/mnem-core/tests/merge_chaos.rs`): seeded
  random histories check totality, no-lost-writes, clean-merge symmetry,
  idempotence, base identity and convergence. CI runs a small trial count;
  `docs/chaos-report.md` records a 50,000-case sweep. Phase 3's definition of
  done.

### Changed

- README: the demo GIF now shows branching and `diff`, not just `commit` / `log`.

## [0.0.3] - 2026-09-07

Era 1, "it branches and travels". `format_version` 1.

### Added

- ADR-0012 (the branch and checkout model): branches stay pointer-only, working
  memory is a view, `mnem rm` gains a local tombstone, and the prolly-tree state
  form is deferred with named triggers.
- `mnem-core`: `Store::branch` / `branches` / `delete_branch`, and
  `resolve_commitish` (a branch name or an unambiguous commit id prefix).
- `mnem-core`: `Store::checkout` (move `HEAD`, refused on a dirty index),
  `working_memory` / `working_node` (the view), and `rm` (stage a tombstone in
  the new local `staging_tombstones` table). `commit` and `unstage` account for
  tombstones; staging a node lifts its tombstone.
- `mnem-core`: `Store::state_at` / `state_map_at` (time travel): reconstruct the
  memory at any past commit, read-only, no `HEAD` move.
- `mnem-core`: `Store::diff(DiffTarget, DiffTarget)` returning `Vec<NodeChange>`
  (added / removed / modified), and `Store::node(object_id)`.
- `mnem` CLI: `branch`, `checkout`, `show`, `diff`, `status`, `rm` (ADR-0012).
  `commit` now accepts a tombstone-only commit.
- Python SDK: `with store.branch("h1"):` context manager, plus `new_branch`,
  `branches`, `delete_branch`, `checkout`, `rm`, `working_memory`,
  `working_node`, `state_at`, `diff` (returning `NodeChange` dataclasses),
  `head_commit` and `resolve`.
- The time-travel property test: over 40 pseudo-random histories, `state_at` of
  every commit equals the working memory recorded when that commit was made.

### Changed

- README: a `mnem` demo GIF, a concrete developer-facing problem statement, and
  the roadmap reframed as "the GitHub for AI agents".

## [0.0.2] - 2026-09-06

Era 1, "it commits". `format_version` 1.

### Added

- Phase 1 substrate core: the object model, the canonical CBOR codec, the
  content-addressed object store, refs and `HEAD`, staging, `commit`, and the
  `log` walk in `mnem-core`.
- The `mnem` CLI: `init`, `add`, `commit`, `log`.
- The Python binding (`mnem._mnem`, pyo3 + maturin) and the agent-facing SDK
  (`mnem.Store`, the `Provenance` / `MemoryNode` / `Commit` dataclasses).
- The round-trip test: a fixed synthetic run reloaded from disk byte for byte.
- `docs/format/`: the full on-disk format specification and golden vectors,
  frozen for the `0.0.x` line at `format_version` 1, pinned by a test.
- CI `wheel` job: builds a release wheel, checks it bundles the extension and
  the type stubs, and runs the SDK tests against the installed wheel.
- ADR-0008 (object encoding and the store engine), ADR-0009 (the ref model),
  ADR-0010 (what 1.0 means, and the 0.0.x roadmap), ADR-0011 (conventional
  commits and the issue lifecycle).
- CI `commits` job: `scripts/conventional_commits.py` checks every commit in a
  pull request and the pull request title.
- CI `deny` job: `cargo deny check bans` over `deny.toml`, which bans every
  network, TLS and async-runtime crate from the workspace tree. This is the
  offline-core check ADR-0004 promised.
- GitHub issue forms (`bug.yml`, `wayfinder.yml`), discussion templates, and a
  priority and triage-state label set. Discussions enabled; blank issues off;
  non-bugs routed to Discussions.
- A `justfile` (`just ci` runs the full check set locally), a `.githooks`
  pre-commit hook (`just install-hooks`), and a `CODEOWNERS` file.
- CI `checks` job: `scripts/check_adr_index.py`, `check_adr_refs.py` and
  `check_changelog.py` keep the ADR index current, ADR references live, and a
  user-facing change tied to a changelog entry.
- `clippy.toml` (MSRV pin, test allowances), workspace-level clippy and rust
  lints, and a path-based PR auto-labeler.

### Changed

- Versioning scheme (ADR-0010): the whole current roadmap is `0.0.x` initial
  development, and `1.0` is reserved for the agent-as-repository platform. The
  six roadmap milestones are renumbered `v0.0.2` through `v0.0.7`.
- Commit convention (ADR-0011): commits and pull request titles now follow
  Conventional Commits. Issues reference `refs #<n>` and close at release, not
  on merge.

## [0.0.1] - 2026-09-06

### Added

- Initial bootstrap tag. Workspace, CI and process scaffolding only; no
  features.

[Unreleased]: https://github.com/Nabzx/mnemosyne/compare/v0.0.7...HEAD
[0.0.7]: https://github.com/Nabzx/mnemosyne/compare/v0.0.6...v0.0.7
[0.0.6]: https://github.com/Nabzx/mnemosyne/compare/v0.0.5...v0.0.6
[0.0.5]: https://github.com/Nabzx/mnemosyne/compare/v0.0.4...v0.0.5
[0.0.4]: https://github.com/Nabzx/mnemosyne/compare/v0.0.3...v0.0.4
[0.0.3]: https://github.com/Nabzx/mnemosyne/compare/v0.0.2...v0.0.3
[0.0.2]: https://github.com/Nabzx/mnemosyne/compare/v0.0.1...v0.0.2
[0.0.1]: https://github.com/Nabzx/mnemosyne/releases/tag/v0.0.1
