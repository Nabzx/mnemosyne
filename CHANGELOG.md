# Changelog

All notable changes to Mnemosyne are recorded here, in the
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format. Versioning
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html); the on-disk
`format_version` runs on its own track (see
[ADR-0007](docs/adr/0007-versioning-and-release-policy.md)).

## [Unreleased]

### Added

- `.github/ISSUE_TEMPLATE/feedback.yml`: a real Issues home for feature
  requests, ideas, and confusing bits - previously, the only real issue
  template (`bug.yml`) explicitly required confirming "this is a
  reproducible bug, not a feature request, idea or question", and
  everything else was redirected to Discussions instead
  (`blank_issues_enabled: false` meant there was no free-form path
  either). `README.md` and `CONTRIBUTING.md` now lead with the
  invitation to open an issue rather than with the pull-request
  restriction.

- `docs/shared-memory-across-frameworks.md`: the practical how-to for
  using one store as shared memory across agents on different
  frameworks - when to give each agent its own branch vs write
  straight to a shared one, how a real conflict surfaces and resolves,
  and what the concurrency model means for structuring a real
  deployment. Grounded in the two real worked examples
  (`cross_framework_merge.py`, `multi_agent_incident_retro.py`), not
  written from theory. Cross-linked from `docs/concurrency-model.md`
  and each adapter's own "Sharing a store with other adapters" README
  section. Not cross-linked from the main README's own adapter section,
  as the ticket originally specified - that section was removed in
  this session's own README rewrite, so the concurrency doc and each
  adapter's README are the more relevant anchors now (#368).

- `examples/multi_agent_incident_retro.py`: four agents on four
  different frameworks (CrewAI, LangGraph, the OpenAI Agents SDK,
  AutoGen) each investigate one incident on their own branch, merged
  sequentially into one shared store - extends #364's two-agent demo
  to prove `merge` holds up past the simplest pairwise case. The real
  point: every finding still `blame`s to the commit the agent that
  wrote it made, even after three more merges land on top of it -
  history stays legible at 4-way scale, not just 2-way. Confirmed
  directly while building this, not assumed: the OpenAI Agents SDK and
  AutoGen adapters auto-sequence their own node ids per session/name
  (`{identifier}:{seq:010d}`, plus a reserved `:_seq` counter node),
  unlike CrewAI/LangGraph's caller-chosen ids - given deliberately
  distinct identifiers here, per each adapter's own "Sharing a store"
  README note (#365), avoiding the guaranteed collision #362 found
  for this pair. Wired into the `adapters-crewai` job, which already
  has all four adapters in one venv (#371).

- `docs/research/issue-339-naming-collision.md`: a Reddit commenter
  flagged one same-named project (`mnemosyne-oss/mnemosyne`,
  NousResearch-backed, 3,336 stars); checked directly, and the real
  picture is bigger - at least eight unrelated GitHub projects share
  the name "Mnemosyne" in the agent-memory space, plus a ninth,
  unrelated claim on the plain `mnemosyne` PyPI name. None of them use
  commit/branch/merge/blame/bisect language - the overlap is real at
  the name and category level, not the feature level. The technical
  cost of a rename is lower than expected: no PyPI or crates.io
  collision on this project's real package names (`mnem-agents`,
  `mnem-git`, ...), and the CLI binary is already `mnem`, not
  `mnemosyne`. Feeds the real open decision (rename or not), not yet
  made (#339).

- `docs/retention.md`: there is no garbage collection anywhere in the
  engine - confirmed directly, not assumed (`delete_branch`'s own doc
  comment states a deleted branch's commits stay reachable by id; `rm`
  and every adapter's `clear`/`clear_session`/`pop_item` stage a
  tombstone commit, never a hard delete). Every commit, once made, is
  retained for the life of the store - a real, already-true property,
  stated as a fact about the engine, explicitly not a claim about
  meeting any specific regulatory retention requirement (that's left to
  map #376's own open research question). Cross-linked from
  `docs/why.md` and `docs/why-version-control.md` (#379).

- `docs/concurrency-model.md`: states plainly what was previously only a
  test gotcha (#311) - redb allows exactly one open handle per store per
  process, what that rules out (two handles open at the same instant in
  the same process), and what it doesn't (sequential access across
  processes, including multiple agents or frameworks sharing one store
  over time, same as `examples/cross_framework_merge.py` already proves)
  (#363). Each of the four adapters' own README gains a "Sharing a store
  with other adapters" section stating its own reserved id shape and
  cross-linking back to this doc (#365) - using #362's actual findings:
  CrewAI's `/`-rooted scheme is distinct from the others; LangGraph has
  no reserved shape of its own and can reproduce another adapter's exact
  id; the OpenAI Agents SDK and AutoGen adapters' schemes are
  byte-for-byte identical, so sharing an identifier string between them
  is a guaranteed collision, not a risk. Also fixed a stale line in
  `mnem-openai-agents`'s own README claiming no worked example exists -
  `examples/openai_agents_memory.py` has existed and run in CI all
  session.

- `examples/fork_via_export_import.py`: fork a store with `export`/
  `import`, edit the fork on a branch, `merge` the edit back into the
  fork's own main - no new convention, `export`/`import`/`merge` are
  already real, general-purpose primitives. The merged-in node still
  `blame`s to the *exact* original commit id, proving the round trip
  carries the real object graph, not a flattened copy (content-addressed
  objects hash identically regardless of which store file holds them).
  Wired into the `adapters` job - core CLI, not adapter-specific (#356).

- `examples/cross_framework_merge.py`: two different framework adapters -
  CrewAI and LangGraph - writing their own branches of one shared store.
  A real conflict surfaces and resolves between two of CrewAI's own
  branches, while an independent LangGraph branch merges in cleanly
  alongside it, proving `merge` needs no adapter-specific code to work
  across frameworks sharing one store (#364). The original design tried
  to force CrewAI and LangGraph onto one literal, colliding node id;
  checked directly against each adapter's real validated public API
  (not just its internal id-building helper), this turned out to be
  structurally impossible for this pair specifically - LangGraph's own
  `put()` rejects an empty namespace, so its real output always contains
  a `:` that CrewAI's `/`-rooted scheme never does. Runs in the
  `adapters-crewai` job, which already has both adapters in one venv.

- `tests/test_cross_adapter_ids.py`: verified, not assumed, whether the
  four adapters' own node-id schemes can collide on a shared store. Real
  finding: the OpenAI Agents SDK and AutoGen adapters' schemes are
  byte-for-byte identical (ADR-0023 reused ADR-0020's scheme directly),
  so the same identifier string used for both on one store guarantees a
  collision on every write, not just risks one - and LangGraph's own
  scheme has no reserved shape at all, so it can reproduce any other
  adapter's exact reserved id, including another adapter's own `_seq`
  counter node specifically. The real, statable invariant: distinct
  identifiers per adapter avoid it; nothing structural guarantees it.
  Runs in the `adapters-crewai` job, which needed all four installed in
  one combined `uv pip install` call - splitting crewai into its own
  call first (the job's previous style) silently resolved an
  incompatible protobuf gencode/runtime pair, found the hard way and
  fixed by combining the install, not by hand-pinning protobuf.

- `packages/mnem-autogen`: scaffolding for an AutoGen `Memory` backed by
  Mnemosyne (ADR-0023). `MnemosyneMemory` subclasses `autogen_core.memory.
  Memory` directly (a real ABC, unlike CrewAI's structural `StorageBackend`
  Protocol) and imports cleanly, but every method still raises
  `NotImplementedError` - real behaviour lands one ticket at a time under
  epic #232 (#317). Shares the plain `adapters` CI job (no dependency
  conflict with mnem-mcp/mnem-langgraph/mnem-openai-agents, unlike
  mnem-crewai's genuine `mcp` pin conflict, #310) and is covered on
  Windows from day one for the same reason.
- `mnem-autogen`: the node id scheme and write/read primitives (ADR-0023) -
  a sortable `{name}:{seq:010d}` id per item, reusing the OpenAI Agents SDK
  adapter's own scheme directly (ADR-0020), and a `{name}:_seq` counter
  node bumped in the same commit as every write, excluded from any content
  read. `add()`/`query()`/`update_context()` still raise
  `NotImplementedError` - nothing calls these primitives yet (#318).
- `mnem-autogen`: `add`/`query`/`update_context` now do real work
  (ADR-0023) - `add` is one commit per call, one node per `MemoryContent`;
  `query` ignores its own argument and returns everything chronologically,
  matching `ListMemory`'s real behaviour exactly rather than a degraded
  implementation; `update_context` formats byte-identically to
  `ListMemory`'s own output, verified directly against it for the same
  content. `MemoryContent.content`'s own `bytes`/`Image` cases get real,
  lossless handling (pydantic's own `model_dump(mode="json")` silently
  mis-serializes `bytes` and has no handling for `Image` at all - found and
  worked around, not assumed safe). `metadata` round-trips fully opaque,
  no reserved-key interpretation. `clear`/`close` are next (#320).
- `mnem-autogen`: `clear`/`close` now do real work (ADR-0023) - `clear` is
  one batched tombstone commit forgetting every node under the instance's
  namespace (including the `_seq` counter, so the next `add()` starts
  fresh), never a hard delete - history from before stays recoverable
  through `mnem log`/`blame`. `close` actually releases the store handle
  (`mnem.Store` has no close method of its own; redb's handle is released
  when the last Python reference drops), unlike `ListMemory`'s true no-op
  - redb allows only one open handle per store per process, so this is
  what lets another instance open the same path afterward. Every `Memory`
  method is now real; a worked example is the only thing left (#321).
- `mnem-autogen`: `why(item_id)`/`bisect(predicate)` (ADR-0023) - extra
  methods outside the `Memory` protocol, the same shape as the LangGraph,
  OpenAI Agents SDK, and CrewAI adapters' own. A new worked example,
  `examples/autogen_memory.py` - a real `AssistantAgent`, `update_context`
  demonstrably injecting stored memories before inference (verified
  directly against the model client's own call history, not just trusted
  to have happened), then `why`/`bisect` tracing a memory to the commit
  that wrote it - wired into CI via a new `examples` extra
  (`autogen-agentchat`/`autogen-ext`, needed only by the example, kept out
  of the package's own dependencies). This closes out the `mnem-autogen`
  build (#321).
- `packages/mnem-openai-agents`: scaffolding for an OpenAI Agents SDK
  `Session` backed by Mnemosyne (ADR-0020). `MnemosyneSession` imports
  cleanly and satisfies the `Session` protocol structurally, but every
  method still raises `NotImplementedError` - real behaviour lands one
  ticket at a time under epic #232 (#303).
- `mnem-openai-agents`: the node id scheme and reserved sequence-counter
  node the rest of the adapter is built on (ADR-0020) - a sortable
  `{session_id}:{seq:010d}` id per item, a `{session_id}:_seq` counter
  bumped in the same commit as every write, excluded from any content
  read. No `Session` protocol methods wired up to it yet (#304).
- `mnem-openai-agents`: `MnemosyneSession.get_items`/`add_items` now do
  real work (ADR-0020) - `add_items` is one Mnemosyne commit per call,
  one node per item; `get_items` is a keyed range read under the
  session's prefix, `limit` (or `session_settings.limit` if unset)
  taking the tail, in chronological order. Item content is stored as
  the raw `TResponseInputItem` dict, unchanged (#305).
- `mnem-openai-agents`: `MnemosyneSession.pop_item`/`clear_session` now
  do real work (ADR-0020) - both are tombstone commits via
  `mnem.agents.forget`/a batched equivalent, never a hard delete, so a
  popped item or a cleared session stays recoverable through the
  store's ordinary history (#306).
- `mnem-openai-agents`: provenance auto-capture (ADR-0020) - passing
  `hooks=session.hooks` to `Runner.run` populates a real
  `Provenance.source`/`observation` on every node written as a result
  of a tool call, no per-call payload to shape by hand. README updated
  to state the behaviour and the one line of caller wiring it needs
  (#307).
- `mnem-openai-agents`: `MnemosyneSession.branch`/`switch`/`merge`/
  `why`/`bisect`/`history` (ADR-0020), thin wrappers over the
  underlying `Store` for code that wants Mnemosyne's own
  differentiators, not just `Session`-protocol drop-in compatibility.
  Store-wide except `why`, matching the LangGraph adapter's own extra
  methods (#308).
- `examples/openai_agents_memory.py`: a worked example for
  `mnem-openai-agents` - a real `Runner.run` tool-calling turn (a
  scripted model, so it stays deterministic and needs no API key, but
  the real tool and the real hooks still execute), then `why`/`bisect`
  trace the answer to the exact call that produced it. Wired into CI;
  `mnem-openai-agents` now listed in the main README's adapter list
  alongside `mnem-mcp`/`mnem-langgraph`. This closes out the
  `mnem-openai-agents` build (#309).
- `packages/mnem-crewai`: scaffolding for a CrewAI `StorageBackend` backed
  by Mnemosyne (ADR-0021). `MnemosyneStorageBackend` imports cleanly and
  satisfies the `StorageBackend` protocol structurally, but every method
  still raises `NotImplementedError` - real behaviour lands one ticket at
  a time under epic #232 (#310). CI runs it in a separate job from
  `mnem-mcp`/`mnem-langgraph`/`mnem-openai-agents`: every crewai release
  with this protocol pins `mcp` to the 1.x line, incompatible with
  `mnem-mcp`'s own `mcp >= 2.2` floor in one venv.
- `mnem-crewai`: the node id scheme (`{scope}/{record_id}`, CrewAI's own
  scope path honored literally) and a real scope tree-walk
  (`list_scopes`/`get_scope_info`) over it (ADR-0021). `save`/`search`
  and the rest of the protocol still raise `NotImplementedError` (#311).
- `mnem-crewai`: `save`/`get_record`/`list_records` now do real work
  (ADR-0021) - `save` is one Mnemosyne commit per call, one node per
  record, `MemoryRecord.source` mapped straight to `Provenance.source`;
  `get_record` and `list_records` (newest-first, optionally scoped to
  a subtree) round-trip the full record, embedding included. `delete`/
  `update`/`count` are next (#312).
- `mnem-crewai`: `delete`/`update`/`count`/`list_categories`/`reset`
  now do real work (ADR-0021) - `delete` matches on any combination of
  `scope_prefix`/`categories`/`record_ids`/`older_than`/
  `metadata_filter` (all given criteria AND together) and tombstones,
  never a hard delete; `update` replaces a record by id as a new
  commit, moving it to a new node if its own `scope` changed rather
  than leaving a stale duplicate behind; `importance`/`last_accessed`
  round-trip through all of the above, never independently
  recomputed. `search` is next, the last protocol method (#313).
- `mnem-crewai`: `search` now does real work (ADR-0021) - exact, unindexed
  cosine similarity over every embedding in the scanned scope, respecting
  `scope_prefix`/`categories`/`metadata_filter`/`min_score`/`limit`, ranked
  identically to a hand-computed cosine similarity over the same vectors.
  README documents the scale ceiling (a few thousand records per scope)
  and points to CrewAI's own LanceDB/Qdrant backends beyond it. Every sync
  method is now real; only the async wrappers remain (#314).
- `mnem-crewai`: `asave`/`asearch`/`adelete` now do real work (ADR-0021) -
  each wraps its sync counterpart in `asyncio.to_thread`, matching the
  LangGraph adapter's existing precedent, rather than a native async
  re-implementation. Every `StorageBackend` protocol method is now real;
  only a worked example is left (#315).
- `mnem-crewai`: `why(record_id)`/`bisect(predicate)` (ADR-0021) - extra
  methods outside the `StorageBackend` protocol, the same shape as the
  LangGraph and OpenAI Agents SDK adapters' own, needed to do anything
  with the history every write already leaves behind. A new worked
  example, `examples/crewai_memory.py` - a real `Memory` instance,
  `remember`/`recall` round-tripping through `MnemosyneStorageBackend`,
  then `why`/`bisect` tracing a record to the commit that wrote it -
  wired into CI. This closes out the `mnem-crewai` build (#316).

### Fixed

- `mnem-crewai`: corrected ADR-0021's claim that private-record filtering
  "happens in the adapter's `search`/`list_records` implementations" -
  the real `StorageBackend.search` protocol never passes a requester
  identity to filter by, and neither of CrewAI's own reference backends
  (LanceDB, Qdrant) filter on `private` themselves; that enforcement is
  CrewAI's own `RecallFlow`'s job, after calling `storage.search()`.
  `private` is round-tripped only, exactly like `importance`/
  `last_accessed` (ADR-0025, narrowing ADR-0021).

### Changed

- README: the top pitch line and the "What you get" table now lead
  with `blame`/`bisect` ahead of `commit`/`branch`/`merge` - real
  feedback from the 2026-09-29 Reddit launch ("nobody has ever wanted
  to branch their agent's memory, everybody has wanted to know which
  turn a wrong fact came in on"). Ordering and emphasis only, no
  wording or claims changed on any row. The Quickstart demo and the
  README GIF already led with bisect/blame ahead of merge, so neither
  needed touching (#340).

- `redb` 2 -> 4. A real breaking change, not routine: `Database::
  begin_read`/`begin_write` moved from inherent methods to the new
  `ReadableDatabase` trait, needing one import in `store.rs` (used in
  production code) and one each in `objects.rs`/`refs.rs`'s own test
  modules (only their tests construct a raw `redb::Database` directly).
  Verified thoroughly before landing, given this crate underpins the
  frozen on-disk format: the full Rust suite passes unchanged,
  including the golden-vector byte-for-byte check, the round-trip and
  time-travel tests, and the 50,000-case merge fuzz sweep: none of
  4.x's real behavioural changes (all gated behind opt-in experimental
  feature flags this project does not enable) touch what this project
  actually stores. `cargo clippy`, `cargo deny`, and the full Python
  SDK test suite against a rebuilt extension also pass unchanged.
  Re-lands #330 as a maintainer commit, not a blind Dependabot merge.
- **MSRV raised from 1.85 to 1.90**, a real compatibility commitment,
  not incidental to the `redb` bump above: checked every `redb`
  release directly (not assumed) and found the entire 3.x/4.x line
  requires at least Rust 1.89, with the specific 4.3.0 this project
  now pins needing 1.90 - there is no partial upgrade that stays on
  1.85. Updated everywhere the old MSRV was pinned: `Cargo.toml`'s
  `rust-version`, `clippy.toml`'s `msrv` (so `incompatible_msrv` still
  catches a real break locally), `rust-toolchain.toml`'s comment, and
  the `msrv` CI job's toolchain pin. Verified `cargo check --all
  --all-features` passes clean against a real, locally-installed
  1.90.0 toolchain, not just a newer one that happens to also work.
- `README.md`/`ROADMAP.md`: added a real, checked finding to Era 3's own
  scope - every framework this project has an adapter for (CrewAI,
  AutoGen, LangGraph, the OpenAI Agents SDK) derives a tool's JSON schema
  from a real Python callable's type hints and docstring the same way,
  confirmed directly against each one's own source - so an agent
  definition's tool *shape* translates between frameworks by
  reformatting, not retraining. Noted as part of Era 3's own registry
  idea (map #352), not yet scoped or estimated. Also noted, honestly
  separated as a harder, different kind of problem: the same versioning
  idea applied to physical robot skills is a robotics/ML research
  question, not a data-format one - added to the Backlog as a long-term
  direction, explicitly not scoped.

### Fixed

- `mnem-openai-agents`: `MnemosyneSession` took a path and opened its own
  store handle (#303), contradicting ADR-0020's own decision that it
  takes an already-open `Store` shared across every session a host
  application constructs. Fixed while building #304, before any adapter
  behaviour depended on the wrong shape.

- `.github/workflows/release.yml` had drifted from what `cargo-dist`
  (pinned at 0.32.0) actually generates - caused by hand-editing action
  versions inside it while re-landing Dependabot bumps. Regenerated for
  real via `dist generate --mode ci`, not hand-reverted. CONTRIBUTING.md
  now states plainly that this file is autogenerated and Dependabot PRs
  touching it get closed without re-landing, so this doesn't recur.

### Fixed

- `ROADMAP.md`'s Backlog listed "CrewAI and AutoGen adapters" as
  unplaced. Both now have real, Accepted ADRs (ADR-0021, ADR-0023) and
  are tracked in epic #232 - removed from the backlog, noted where
  they actually live.

### Changed

- `ROADMAP.md`: applied the dating policy decided in #247. The
  now-inaccurate "there are no dates" line is fixed (Era 1 stays
  dateless, Era 2/3 use the estimate policy). Era 2 and Era 3 each
  explicitly state they have no estimate yet and why - neither's own
  precondition (Era 1 shipped with users; real design work starting)
  has happened, so there's nothing real to estimate against, checked
  against Era 1's actual observed pace (six phases, four days) rather
  than assumed.

### Added

- `ROADMAP.md`: a dating policy for Era 2/3 - Era 1's already-shipped
  phases stay dateless, Era 2/3 get soft, revisable estimates in
  months (tightening to weeks as a phase nears), not quarters. Era 1
  stays as-is; the actual dates land in Era 2/3 in a follow-up ticket.

### Changed

- CONTRIBUTING.md: `docs/` and `examples/` are now open to real external
  PRs (typo fixes, clarity, a new worked example) - lower review risk,
  no correctness/security surface. Everywhere else stays solo-maintained
  until the substrate-shipped bar this section already stated. A PR
  outside that carve-out is closed with a pointer to opening an issue,
  not left open. `examples/README.md` gains a one-line pointer to the
  on-ramp.

### Added

- [ADR-0024](docs/adr/0024-the-mem0-migration-path.md): the Mem0
  migration path's contract. `mnem.agents.import_mem0(...)` in the SDK
  (not a script or a new package), `mem0:`-namespaced node ids,
  skip-and-log for empty content, a real pagination loop over
  `get_all`. No code yet - the build ticket follows.

### Added

- `docs/research/issue-253-mem0-migration-survey.md`: source-level
  research on Mem0's real read API (`get_all`'s scope/pagination
  constraints, the exact per-item shape) feeding the ADR that will fix
  a Mem0 migration/import tool's scope - not a peer adapter, since
  Mem0 is a memory store, not an agent framework.

### Fixed

- `docs/comparisons.md`'s Letta section described a retired API
  (block-history/checkpoint-rollback, now on an `archive` branch) as
  current. Corrected to describe the active product (`letta-code`),
  whose agent memory is a real git repository used as sync plumbing,
  not a versioning feature - no `log`, `branch`, or `merge`.

### Added

- `docs/research/issue-252-letta-survey.md`: source-level research
  finding Letta's active product (`letta-code`) has pivoted away from
  the block/checkpoint model `docs/comparisons.md` described - its
  current memory model is a real git repository used as internal sync
  plumbing, with no Python API surface for it. Feeds the ADR deciding
  whether an adapter makes sense at all.

### Changed

- CONTRIBUTING.md: the Dependabot paragraph now states plainly that an
  open Dependabot pull request is triaged (re-landed or closed with a
  reason), never left open as a queue.

### Changed

- CI: `astral-sh/setup-uv` 5 -> 10.1.0, `actions/checkout` 4.4.0 ->
  7.0.1, `actions/labeler` 5.0.0 -> 7.0.0, `actions/upload-artifact`
  4.6.2 -> 7.0.1, `actions/deploy-pages` 4.0.5 -> 5.0.1. Re-lands 5
  previously-untriaged Dependabot pull requests (#78, #79, #188, #189,
  #190) as maintainer commits, each SHA re-verified against its real
  tag ref before applying.

### Added

- [ADR-0023](docs/adr/0023-the-autogen-adapter.md): the `mnem-autogen`
  adapter's contract. `Memory` implemented to match `ListMemory`'s own
  real behaviour exactly (query ignores its argument, returns everything
  chronologically), one store per instance, metadata left fully opaque.
  No code yet - the build ticket follows.

### Added

- `docs/research/issue-251-autogen-survey.md`: source-level research on
  AutoGen's `Memory` protocol, feeding the ADR that will fix a
  `mnem-autogen` adapter's contract.

### Added

- `docs/comparisons.md`: a sourced Mem0/Zep table answering the
  audit-query table's seven questions against each tool's real source
  ([ADR-0022](docs/adr/0022-mem0-and-zep-on-the-audit-query-table.md)),
  alongside the existing prose comparison.

### Added

- [ADR-0022](docs/adr/0022-mem0-and-zep-on-the-audit-query-table.md): the
  audit-query table's seven questions verified against real Mem0/Zep
  behaviour, narrowing (not reversing) ADR-0017's scope. The table itself
  lands in `docs/comparisons.md` in a follow-up build ticket (#283).

### Added

- `docs/research/issue-282-mem0-zep-audit-table.md`: source-level research
  verifying Mem0's and Zep's real behaviour against the audit-query table's
  seven questions, feeding the ADR that will narrow ADR-0017's scope.

### Added

- [ADR-0021](docs/adr/0021-the-crewai-adapter.md): the `mnem-crewai`
  adapter's contract. `StorageBackend` implemented for real (one node per
  memory record, `/`-delimited scope-path node ids, unindexed but exact
  cosine-similarity search with a stated scale ceiling, `source`/`private`
  passed straight through). No code yet - the build ticket follows.

### Added

- `docs/research/issue-250-crewai-survey.md`: source-level research on
  CrewAI's `StorageBackend` protocol, feeding the ADR that will fix a
  `mnem-crewai` adapter's contract.

- [ADR-0020](docs/adr/0020-the-openai-agents-sdk-adapter.md): the
  `mnem-openai-agents` adapter's contract. `Session` implemented narrowly
  (one node per transcript item, a shared store with a session-id prefix,
  provenance auto-captured via `on_tool_end`); `branch`/`merge`/`blame`/
  `bisect` live on the adapter class outside the protocol, mirroring the
  LangGraph adapter's precedent. No code yet - the build ticket follows.

### Added

- `docs/research/issue-249-openai-agents-survey.md`: source-level research
  on the OpenAI Agents SDK's `Session` protocol, feeding the ADR that will
  fix a `mnem-openai-agents` adapter's contract.

### Added

- The README now states the benchmark's actual correctness figures (100%
  across an 80-seed, 720-run sweep, plus a separate 50,000-case merge fuzz
  test) instead of only linking to `docs/benchmark.md`.

### Added

- CI: a `coverage` job runs `cargo llvm-cov` on every push, uploading an
  HTML report as a build artefact (`just coverage` locally). Currently
  81.65% line coverage workspace-wide. A report, not a gate - no failing
  threshold. No badge yet; a live one needs a maintainer account on an
  external service (Codecov or similar), which is a separate decision.

### Added

- A PyPI-downloads badge to the README's badge row (verified live before
  adding it - `pypistats.org`-backed, no auth needed).

### Added

- `docs/debugging-a-poisoned-agent.md`: a step-by-step walkthrough of
  `examples/claude_agent.py`'s bisect/blame/merge scenario, using its real,
  reproducible output (the ids are content hashes, deterministic from the
  script's fixed inputs) rather than a compressed or fabricated example.
  Linked from the README's "Prior work" section.

### Added

- `docs/why-version-control.md`: a technical piece making the general case
  for versioned agent memory, aimed at a reader who hasn't seen the
  project - leans on the benchmark's audit-query table and the
  bisect/blame billing-note example rather than abstract argument. Linked
  from the README's "Prior work" section.

### Added

- `docs/why.md`: the "why I built this" essay drafted back in #175 -
  redrafted fresh against the current state of the project (v0.0.8, the
  merge-conflict demo, `docs/comparisons.md`) rather than a stale copy,
  since the original file was never committed and no longer exists.
  Linked from the README's "Prior work" section.

### Added

- `docs/comparisons.md`: a written comparison against Mem0, Zep, Letta,
  Memoria and ByteRover CLI, checked against each project's actual source
  (not its README) - what's real about each one's history/versioning
  claims, and what's actually different about `mnem`'s. Linked from a new
  README FAQ entry.

### Added

- `packages/mnem-mcp/README.md`: a Claude Code section (`claude mcp add`
  and a project-scoped `.mcp.json`), alongside the existing Claude
  Desktop config. Both verified working end to end.

### Changed

- README demo GIF: retimed from 24.3s down to 15.0s without cutting any
  content (`--demo` mode's `PACE` multiplier lowered from 1.0 to 0.55,
  scaling every line's pause proportionally; `Set TypingSpeed` tightened
  in the tape). A real, well-regarded comparable (atuin's README GIF)
  runs 12.3s; this keeps the same full merge-conflict scenario the demo
  just gained while landing much closer to that pace.

### Changed

- README demo GIF: theme changed from Catppuccin Mocha to Aardvark Blue.
  Tested 5 candidates against the real recording first - most read as
  near-identical to Mocha; Aardvark Blue's saturated navy was the most
  distinct without clashing against the demo's fixed lilac/blue/green
  accent colours (which are hardcoded truecolor, independent of the
  tape's theme - a theme swap alone never touches them).

### Changed

- The README demo GIF now shows a real merge conflict, not just
  commit/bisect/blame: a second line of the same case ran in parallel on
  a branch and independently got the right answer, and the agent resolves
  a genuine conflict in its favour rather than silently overwriting
  anything. Re-recorded with a tighter lead-in and a deliberate hold on
  the final line so the payoff reads clearly whether a viewer catches the
  loop from the start or mid-cycle.

### Changed

- README Quickstart: the 3 commands that get memory under version control
  are now their own block, ahead of the fuller bisect/blame walkthrough -
  previously the two were one undifferentiated 7-command sequence.

### Changed

- README restructured: "The GitHub for AI agents" and "Status" merge
  into one new "The vision" section, five stages (version control, the
  collaboration layer, the platform, the translation layer, physical
  AI) ending on robots as the end goal and software agents as the
  easiest place to prove the idea first. The "How it works" and "FAQ"
  sections are removed; everything else is unchanged.

- README quickstart leads with the cargo-dist shell installer (no Rust
  toolchain needed); `cargo install mnem-git` is now the "already have
  Rust" alternative.

## [0.0.8] - 2026-09-21

Prebuilt `mnem` binaries, `export`/`import` (CLI and SDK), and a round of
pre-launch polish across CI, docs, and package metadata.

### Added

- Python SDK: `store.export()` / `mnem.Store.import_(path, data)` - the
  same whole-store JSON backup/restore `mnem export`/`import` gave the
  CLI in #174, now callable without shelling out to the binary.
- `mnem add --content-json <JSON>`: stage a number, bool, object, array, or
  null inline, without writing it to a temp file first (`--content-file`
  was the only prior way to store non-string content from the CLI).
- CI: a `windows-python` job builds the `mnem-py` extension and runs the SDK
  and adapter tests (`packages/mnem-mcp`, `packages/mnem-langgraph`) on
  Windows. #184's Windows job only ever covered the plain Rust build; a real
  Windows wheel has shipped to PyPI on every release since #165 without
  anything testing it first.
- ADR-0019 (prebuilt `mnem` binaries): `cargo-dist` builds `mnem` for linux
  (x86_64, aarch64), macOS (x86_64, aarch64) and Windows (x86_64), plus a
  shell and a PowerShell installer, and attaches them to the GitHub
  Release. Triggered by hand (`workflow_dispatch`), same as the PyPI
  workflow; assumes the release already exists as a draft and undrafts it
  once the binaries are attached. No Homebrew tap yet.
- `mnem export` / `mnem import`: the whole store (every object ever
  written, every ref, HEAD) as one portable JSON file, and the reverse.
  Object ids render as hex, so the file reads cleanly with `jq` or by
  hand. A hand-edited or corrupted export is a clean error on import,
  not a silent misread: every object's claimed id is re-hashed and
  checked. Does not touch `format_version`; it is a view for backup and
  portability, not a new on-disk format.
- CI: `.github/workflows/release-pypi.yml` (named `release.yml` until #173
  gave the Rust binaries their own workflow of that name). A published
  GitHub Release now builds
  `mnem-agents` wheels for linux (x86_64, aarch64), macOS (universal2) and
  Windows (x64), plus the sdist and the two adapter wheels, and publishes
  all of it to PyPI via trusted publishing. `workflow_dispatch` reruns it
  for an existing tag, to backfill a release that shipped without this
  (v0.0.7 currently has only a macOS wheel).
- CI: a `windows-latest` job runs `cargo test` and `cargo build --release`
  on every push and pull request. The core and the CLI are now actually
  tested on Windows, not just assumed to work there.
- `mnem completions <shell>`: prints a shell completion script (bash, zsh,
  fish, powershell, elvish) via a new `clap_complete` dependency.
- `deny.toml`: a `[licenses]` policy, allow-listing every permissive licence
  actually in the dependency tree. The `deny` CI job now runs `cargo deny
  check` (bans, licences, advisories, sources), not just `check bans`.
- README: a "Coming from Git" command cheat-sheet, and an FAQ answering the
  usual "how is this different from a vector store / mem0 / Zep / a
  checkpointer / a JSONL log" questions up front.
- `packages/mnem-mcp/README.md`: a Claude Desktop `claude_desktop_config.json`
  snippet, linked from the main README.
- `CITATION.cff`.
- `homepage` on the published crate metadata (`mnem-store`, `mnem-git`);
  crates.io showed "Not provided" despite a real docs site existing.
- `deny.toml`: explicit `[advisories]` and `[sources]` tables, written down
  rather than left to cargo-deny's implicit defaults.

### Changed

- README: the CLI quickstart is now the full arc the demo GIF tells (record a
  fact, misread one an hour later, `bisect` and `blame` find it), not just
  `init`/`add`/`commit`/`blame`.
- The crate-level doc comments for `mnem-store` and `mnem-git` (the public
  docs.rs landing page and the crates.io description) now describe what each
  crate does, rather than the phase history of how it was built.
- The `Development Status` classifier on all three PyPI packages
  (`mnem-agents`, `mnem-mcp`, `mnem-langgraph`) moves from `2 - Pre-Alpha` to
  `3 - Alpha`.
- README: the quickstart now installs the published packages (`cargo install
  mnem-git`, `pip install mnem-agents`) instead of the pre-publish build-from-
  source workaround, and gains a CLI-only walkthrough alongside the Python
  one. The status table catches up through `v0.0.7`. The badge row adds
  crates.io, PyPI and docs-site badges; the "Prior work" section links
  `docs/benchmark.md`, the evidence for the claim right above it.
- README: links to shell completions (`mnem completions <shell>`, #171) and
  to `examples/`, neither of which was surfaced there before.
- `CONTRIBUTING.md`'s Checks section now names all 9 recipes `just ci` runs,
  not 5.
- `CONTEXT.md` and `mnem-store`'s `State` doc comment no longer claim a
  prolly-tree form "arrives in Phase 2" - Era 1 is fully shipped and it was
  never built; both now point at ADR-0002/0012's deferral instead.

### Fixed

- `mnem-langgraph`: `MnemosyneStore.delete()` on a key that was never
  written no longer raises `InvalidRefError` - `BaseStore`'s delete is
  idempotent by convention, matching a dict's `.pop(key, None)`.
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

[Unreleased]: https://github.com/Nabzx/mnemosyne/compare/v0.0.8...HEAD
[0.0.8]: https://github.com/Nabzx/mnemosyne/compare/v0.0.7...v0.0.8
[0.0.7]: https://github.com/Nabzx/mnemosyne/compare/v0.0.6...v0.0.7
[0.0.6]: https://github.com/Nabzx/mnemosyne/compare/v0.0.5...v0.0.6
[0.0.5]: https://github.com/Nabzx/mnemosyne/compare/v0.0.4...v0.0.5
[0.0.4]: https://github.com/Nabzx/mnemosyne/compare/v0.0.3...v0.0.4
[0.0.3]: https://github.com/Nabzx/mnemosyne/compare/v0.0.2...v0.0.3
[0.0.2]: https://github.com/Nabzx/mnemosyne/compare/v0.0.1...v0.0.2
[0.0.1]: https://github.com/Nabzx/mnemosyne/releases/tag/v0.0.1
