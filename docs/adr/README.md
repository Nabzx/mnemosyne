# Architecture decisions

Numbered records of the decisions that shape Mnemosyne, and why they went the
way they did. Format: [`0000-template.md`](0000-template.md).

An ADR is Accepted before code is written for anything touching the on-disk
format, the object model, a public API surface, the merge algorithm, the Rust
and Python boundary, or a new dependency. Accepted ADRs are immutable; a later
change is a new ADR that supersedes the old one.

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
| [0010](0010-what-1.0-means.md) | What 1.0 means, and the 0.0.x roadmap | Accepted |
