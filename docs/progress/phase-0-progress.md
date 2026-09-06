# Phase 0 progress

Bootstrap. Get to "clone and build" with the process scaffolding in place.

## Done

- Repo renamed from `YC27` to `mnemosyne`.
- Rust workspace: `mnem-core` (library, one placeholder function and a test) and
  `mnem-cli` (the `mnem` binary, `--version` and a declared command tree with
  every subcommand stubbed out).
- Python SDK skeleton: `mnem` exposes its version and raises a clear error on any
  API that is not built yet. A smoke test covers both.
- CI: a `rust` job (`fmt`, `clippy -D warnings`, `test`, `build`) and a `python`
  job (`ruff`, `pytest`) on every pull request and on push to `main`.
- Docs: `README.md`, `CONTEXT.md` (seed glossary), `ROADMAP.md`, `AGENTS.md`
  (with the one-maintainer and offline-core invariants), `CONTRIBUTING.md`,
  `docs/adr/0000-template.md`, ADR-0001, the ADR index, and the agent workflow
  docs.
- Issue and pull request templates, including the Wayfinder ticket template.
- Licence: Apache 2.0.

## Pending

- Labels, milestones `v0.1` to `v1.0`, and the project board.
- The Phase 0 Wayfinder map (#... ) and its child tickets:
  - ADR-0002, on-disk object format (`wayfinder:research`, then `wayfinder:grilling`)
  - ADR-0003, the memory node model (`wayfinder:research`, then `wayfinder:grilling`)
  - ADR-0004, Rust core and Python SDK boundary, including the Python-core fallback (`wayfinder:grilling`)
- Local verification of the Rust build. The scaffold was written without a Rust
  toolchain to hand; CI is the first real check.

## Notes

- The pyo3 and maturin binding was moved out of Phase 0 and into Phase 1, where
  the core first has something to bind. Phase 0 ships a pure-Python placeholder.
  ROADMAP updated to match.
- Phase 0 tag `v0.0.1` is cut once CI is green and the map is open.
