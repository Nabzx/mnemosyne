# ADR-0004: The Rust core and Python SDK boundary

- Status: Accepted
- Date: 2026-09-06
- Issue: #7 (grilling)

## Context

The stack was set at plan time: a Rust core (`mnem-core`), a `mnem` command line
tool, and a Python SDK (`mnem`) bound through pyo3 and maturin. This ADR fixes
the boundary between those pieces, and pre-commits to a fallback so that choice
is not made under pressure later.

The Rust core is the ambitious choice for a solo maintainer. The value is a real
CLI binary, a performance story against Git4Data and DoltDB, and the discipline
that a typed core imposes. The risk is that the maintainer stalls on Rust and
the whole project stalls with it.

## Decision

### `mnem-core`

The core holds:

- the object model (ADR-0003)
- the store (ADR-0002)
- the commit graph
- every deterministic operation on them: `commit`, `branch`, `checkout`,
  `merge`, `blame`, `bisect`, `diff`, time travel

The core does **not** hold, and a CI check enforces the first of these:

- any network-capable dependency
- any model or LLM call
- CLI argument parsing or output formatting
- any Python or FFI types
- any knowledge of an agent framework
- anything semantic: embeddings, retrieval, semantic merge. Semantic work lands
  in v2 in a separate `mnem-semantic` crate, behind a trait.

If a behaviour is worth a test, it lives in the core with a Rust test.

### The Python SDK

`mnem` (the Python package) is pure ergonomics over the core: Pythonic names,
exceptions, dataclasses, context managers. It carries no operation semantics.

The one carve-out is Python-idiom composition, such as the
`with store.branch("h1"):` context manager, which is only "call `branch`, call
`checkout`, on exit `checkout` back or `merge`". The SDK gets thin smoke tests;
the behaviour it composes is tested in the core.

### Consumers of the core

`mnem-cli` (the `mnem` binary) and `mnem-py` (the pyo3 binding) are **peer
consumers of `mnem-core`**. Neither depends on the other. The core's consumer
count stays at two.

The MCP server (`mnem-mcp`) and the framework adapters (`mnem-langgraph` and
later) are Python packages that depend on the SDK, not on the core. They are the
SDK's customers, the same as any agent author.

### Packaging

The Python wheel ships the extension module only. The `mnem` CLI binary is not
in the wheel; it is installed with `cargo install mnem` or from a release. This
is revisited at v1.0 if users ask for a bundled CLI.

### Errors across the boundary

The core defines one error enum, `MnemError`, with named variants. The binding
maps each variant to a Python exception in a hierarchy rooted at
`MnemError(Exception)`:

| `MnemError` variant | Python exception |
| --- | --- |
| `NotFound` | `NotFoundError` |
| `InvalidRef` | `InvalidRefError` |
| `Conflict` | `ConflictError` |
| `CorruptStore` | `CorruptStoreError` |
| `FormatVersion` | `FormatVersionError` |
| `StoreExists` | `StoreExistsError` |
| `NoStore` | `NoStoreError` |
| `Io` | `StoreIoError` |

Adding a variant means adding a row here. No stringly-typed errors cross the
boundary. `StoreExists` and `NoStore` were added with `Store::init`/`open` in
issue #24 and their rows added here per this rule, which the ADR carves out from
the usual immutability. In Python the base class is named `MnemError`; the
mapping and hierarchy are exercised by the binding's tests (#29).

### Versioning

One version number, held in the Cargo workspace `[workspace.package] version`
and read into the wheel at build time by maturin. A release is one git tag
`vX.Y.Z`. A release job publishes the crate and the wheel from that tag and
refuses if the two versions disagree. The semver policy, in particular what a
minor versus a patch means for the on-disk format, is ADR-0007's job (#17).

### The fallback: a tracer bullet

Phase 1 starts with the minimum vertical slice in Rust: `Store::init`, write one
object, read it back, one commit, `log` (issues #23 to #27).

- If that slice works and is pleasant to extend, Rust continues.
- If it hits a genuine wall (a `redb` or `pyo3` limitation, or the graph code
  fighting the borrow checker past reasonable effort), the maintainer switches
  `mnem-core` to Python for v0.1, keeps the same public API, and records the
  switch as an ADR that supersedes the "core is Rust" part of this one.

The public API (the CLI surface and the SDK surface) is defined first and is
language-neutral, so a switch does not change what users see. The go or no-go is
made at the tracer-bullet checkpoint, early and cheap, and it is the
maintainer's call alone. No agonising over partial progress below the
checkpoint; a clear decision at it.

## Consequences

- The core is a deep module with a small, typed surface, testable without any
  Python or network in the loop.
- The SDK is small enough that most of its cost is documentation and stubs, not
  logic.
- Two consumers of the core, both thin. Adapters sit a layer further out and
  cannot reach past the SDK.
- The wheel is simple: no per-platform CLI binary, no entry-point plumbing.
  Users who want the CLI take one extra step.
- The Rust bet has an explicit, early exit. If it fails it fails in Phase 1 at
  low cost, not at v0.5.
- A Python fallback core would lose the performance story and the CLI-as-a-single-
  binary story, and would need its own packaging. That cost is accepted as the
  price of not stalling.

## Alternatives considered

**A Python core from the start.** Rejected as the default. It gives up the
performance story, the single-binary CLI, and the discipline of a typed core.
Kept as the named fallback.

**The SDK holds orchestration logic.** Rejected. Logic in two languages means
two test suites for one behaviour and drift between them.

**Bundle the CLI in the wheel.** Rejected for now. Per-platform wheels and
entry-point plumbing for an audience that mostly wants the library. Revisit at
v1.0.

**Adapters consume `mnem-core` directly through their own binding.** Rejected.
It multiplies bindings and means adapters see different errors and ergonomics
from what agent authors see.

**Independent version numbers for the crate, wheel and CLI.** Rejected. Three
numbers for one release is a source of confusion and mismatched bug reports.
