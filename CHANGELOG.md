# Changelog

All notable changes to Mnemosyne are recorded here, in the
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format. Versioning
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html); the on-disk
`format_version` runs on its own track (see
[ADR-0007](docs/adr/0007-versioning-and-release-policy.md)).

## [Unreleased]

### Added

- Phase 0 scaffold: the Rust workspace (`mnem-core`, `mnem-cli`), the Python SDK
  skeleton, CI, and the full doc set.
- ADR-0001 to ADR-0007, fixing the object format, the memory node model, the
  Rust and Python boundary, commit identity, and this versioning policy.

## [0.0.1] - 2026-09-06

### Added

- Initial bootstrap tag. Workspace, CI and process scaffolding only; no
  features.

[Unreleased]: https://github.com/Nabzx/mnemosyne/compare/v0.0.1...HEAD
[0.0.1]: https://github.com/Nabzx/mnemosyne/releases/tag/v0.0.1
