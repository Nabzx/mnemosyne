# Architecture decisions

Numbered records of the decisions that shape Mnemosyne, and why they went the
way they did. Format: [`0000-template.md`](0000-template.md).

An ADR is Accepted before code is written for anything touching the on-disk
format, the object model, a public API surface, the merge algorithm, the Rust
and Python boundary, or a new dependency. Accepted ADRs are immutable; a later
change is a new ADR that supersedes the old one.

> Naming note: ADRs 0001 to 0018 call the core crate `mnem-core`, the CLI crate
> `mnem-cli`, the binding `mnem-py`, and the MCP package `mnemosyne-mcp`. Before
> the first crates.io and PyPI publish these were renamed to `mnemosyne-store`,
> `mnemosyne-git`, `mnemosyne-py` and `mnemosyne-agents-mcp` (the old names were
> taken). The `mnem` binary, `import mnem`, and the `.mnem/` store directory are
> unchanged. See the `CHANGELOG.md` entry under Unreleased.

| ADR | Title | Status |
| --- | --- | --- |
| [0001](0001-record-architecture-decisions.md) | Record architecture decisions | Accepted |
| [0002](0002-on-disk-object-format.md) | On-disk object format | Accepted |
| [0003](0003-memory-node-model.md) | The memory node model | Accepted |
| [0004](0004-core-sdk-boundary.md) | The Rust core and Python SDK boundary | Accepted |
| [0005](0005-commit-identity.md) | Commit identity, hashing and signing | Accepted |
| [0007](0007-versioning-and-release-policy.md) | Versioning and release policy | Accepted |
| [0008](0008-object-encoding-and-store-engine.md) | Object encoding and the store engine | Accepted |
| [0009](0009-ref-model.md) | The ref model | Accepted |
| [0010](0010-what-1-0-means.md) | What 1.0 means, and the 0.0.x roadmap | Accepted |
| [0011](0011-conventional-commits-and-issue-lifecycle.md) | Conventional commits and the issue lifecycle | Accepted |
| [0012](0012-branch-and-checkout-model.md) | The branch and checkout model | Accepted |
| [0013](0013-deterministic-merge-algorithm.md) | The deterministic merge algorithm | Accepted |
| [0014](0014-conflict-object-and-resolution-api.md) | The conflict object and the resolution API | Accepted |
| [0015](0015-provenance-index-blame-and-bisect.md) | The provenance index, blame and bisect | Accepted |
| [0016](0016-mcp-tools-and-the-adapter-contract.md) | The MCP tools and the adapter contract | Accepted |
| [0017](0017-the-benchmark-and-its-metrics.md) | The benchmark and its metrics | Accepted |
| [0018](0018-the-era-2-seam.md) | The Era 2 seam: the semantic merge trait and the sync protocol | Accepted |
