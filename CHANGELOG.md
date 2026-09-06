# Changelog

All notable changes to Mnemosyne are recorded here, in the
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format. Versioning
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html); the on-disk
`format_version` runs on its own track (see
[ADR-0007](docs/adr/0007-versioning-and-release-policy.md)).

## [Unreleased]

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

[Unreleased]: https://github.com/Nabzx/mnemosyne/compare/v0.0.3...HEAD
[0.0.3]: https://github.com/Nabzx/mnemosyne/compare/v0.0.2...v0.0.3
[0.0.2]: https://github.com/Nabzx/mnemosyne/compare/v0.0.1...v0.0.2
[0.0.1]: https://github.com/Nabzx/mnemosyne/releases/tag/v0.0.1
