# Changelog

All notable changes to Mnemosyne are recorded here, in the
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format. Versioning
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html); the on-disk
`format_version` runs on its own track (see
[ADR-0007](docs/adr/0007-versioning-and-release-policy.md)).

## [Unreleased]

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

[Unreleased]: https://github.com/Nabzx/mnemosyne/compare/v0.0.1...HEAD
[0.0.1]: https://github.com/Nabzx/mnemosyne/releases/tag/v0.0.1
