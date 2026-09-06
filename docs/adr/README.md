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
| 0002 | On-disk object format | Proposed (Phase 0 map) |
| 0003 | The memory node model | Proposed (Phase 0 map) |
| 0004 | Rust core and Python SDK boundary | Proposed (Phase 0 map) |
