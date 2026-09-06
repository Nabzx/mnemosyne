# Phase 1 progress

v0.1, "it commits". The content-addressed object store and the commit graph.

## Decisions (all Accepted)

- **ADR-0008** object encoding and the store engine: `redb`, CBOR restricted to
  RFC 8949 §4.2 plus our 64-bit-float rule, `ciborium` plus a thin pass.
- **ADR-0009** the ref model: branches only, flat namespace, one-line text
  `HEAD`, compare-and-swap ref moves, reflog reserved.

## `mnem-core`, built

| Ticket | Module | What |
| --- | --- | --- |
| #22 | `object`, `id` | `Object` (kind-tagged), `MemoryNode`, `State`, `Commit`, `Provenance`; `ObjectId` (BLAKE3, hex, CBOR byte string) |
| #23 | `codec`, `objects`, `error` | canonical CBOR encode/decode; `put`/`get`/`has` over a `redb` txn; `MnemError` |
| #24 | `store`, `config` | `Store::init`/`open` (upward discovery, nested-store guard), `config` parsing with the `format_version` gate |
| #25 | `refs`, `head` | the `refs` table with compare-and-swap; `Head::{Attached,Detached}`; `Store::head`/`set_head`/`resolve_head` |
| #26 | `commit`, `staging` | persistent `staging`; `Store::stage`/`staged`/`unstage`/`commit` (one write txn per commit) |
| #27 | `log` | `Store::log`/`log_head`, first-parent and full-graph walks |

## ADR-0004 tracer-bullet checkpoint

The minimum vertical slice (#23 to #27) is working, ~43 tests, no walls hit.
Friction: a `redb` borrow-lifetime quirk (fixed by inlining a helper) and a
clippy version drift (fixed by pinning CI's `rust` job to 1.93.0). **The Rust
core continues; the Python fallback is not needed.**

## `mnem` CLI, built (#28)

`init`, `add`, `commit`, `log`, each a thin wrapper over the core. Verified end
to end:

```
$ mnem init
$ mnem add customer-4821 "the customer is on the Enterprise plan" --source ticket-4821
$ mnem commit -m "learn the plan tier"
$ mnem add customer-4821 "the customer downgraded to Pro"
$ mnem commit -m "record the downgrade"
$ mnem log --oneline
ddb2a9ea60a3 record the downgrade
45ee8a6b98b1 learn the plan tier
```

## `mnem-py`, the Python binding (#29)

A `pyo3` `cdylib` (`_mnem`) over `mnem-core`, built by maturin as an abi3
wheel (`abi3-py310`, one wheel for CPython 3.10 and up). It exposes `Store`
(`init`, `open`, `add`, `commit`, `staged`, `unstage`, `log`, `head`,
`format_version`, `root`) and maps every `MnemError` variant onto a Python
exception hierarchy rooted at `mnem.MnemError` (ADR-0004). `pyproject.toml`
moved from hatchling to the maturin backend; the version is read from the Cargo
workspace at build time. `python/mnem/__init__.py` re-exports the binding; the
ergonomic layer is #30.

The `rust` CI job excludes `mnem-py` from `cargo test`/`build` (its test binary
would need libpython to link); the `python` job builds the real extension
through maturin and runs the binding's smoke tests. The behaviour itself stays
tested in the core.

## Left in Phase 1

- #30 the Python SDK (ergonomic layer over the binding)
- #31 the round-trip test (the Phase 1 definition of done)
- #32 write and freeze `docs/format/` for the v0.x line
- #33 CI: build the wheel and test the SDK against it
