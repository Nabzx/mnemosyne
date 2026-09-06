# Phase 0 progress

Bootstrap. Get to "clone and build" with the process scaffolding in place.

## Done

- Repo renamed from `YC27` to `mnemosyne`.
- Rust workspace: `mnem-core` (library, one placeholder function and a test) and
  `mnem-cli` (the `mnem` binary, `--version` and a declared command tree with
  every subcommand stubbed out). Rust CI (`fmt`, `clippy -D warnings`, `test`,
  `build --release`) green on `main`.
- Python SDK skeleton: `mnem` exposes its version and raises a clear error on any
  API that is not built yet. A smoke test covers both. Python CI (`ruff`,
  `pytest`) green on `main`.
- CI: a `rust` job and a `python` job on every pull request and on push to
  `main`.
- Docs: `README.md`, `CONTEXT.md` (seed glossary), `ROADMAP.md`, `AGENTS.md`
  (one-maintainer and offline-core invariants), `CONTRIBUTING.md`,
  `docs/adr/0000-template.md`, ADR-0001, the ADR index, the agent workflow docs,
  and the `docs/format`, `docs/research`, `examples` placeholders.
- Issue and pull request templates, including the Wayfinder ticket template.
- Licence: Apache 2.0.
- Labels: `phase-0` to `phase-7`, the `wayfinder:*` set, `ready-for-agent`,
  `track-a` / `track-b`, `adr`, `format`, `benchmark`, `spike`.
- Milestones: `v0.1` to `v1.0`.
- The Phase 0 Wayfinder map (#1), now with ten child tickets: #2 to #7 plus #15
  to #18 (commit hashing research and ADR-0005, ADR-0007 versioning, repo
  hygiene).
- Maps for Phases 1 to 6 (#9 to #14), each a kept-open epic with a checklist
  comment linking its children.
- The full backlog: 68 issues in total. 57 granular one-PR-sized child tickets
  across the six phase maps, sub-issue linked, with 48 `blocked_by` dependency
  edges. Every task ticket is scoped to something a single pull request could
  close.
- The scaffold (#2) built on `chore/phase-0-scaffold`, merged via PR #8 with
  green CI.
- Tag `v0.0.1` pushed.
- The GitHub project ([Mnemosyne roadmap](https://github.com/users/Nabzx/projects/1)),
  linked to the repo, with all open issues added.
- Decision tickets closed: #3 and #15 (research), #4, #5, #6, #7, #16 (research
  and grilling). ADR-0002 (object format), ADR-0003 (memory node model), ADR-0004
  (core and SDK boundary) and ADR-0005 (commit identity) all Accepted.
- Repo hygiene (#18): `SECURITY.md`, `CODE_OF_CONDUCT.md`, `.github/dependabot.yml`
  (cargo, pip, github-actions), `rust-toolchain.toml`, and an `msrv` CI job at
  Rust 1.85 (the floor set by `clap`'s `edition2024` transitive requirement).
- **ADR-0007, versioning and release policy: Accepted** (#17). SemVer 2.0.0;
  `format_version` on its own integer track; additive format changes are a MINOR
  software release, breaking ones a MAJOR with `mnem migrate`; hand-written
  `CHANGELOG.md`; a gated release workflow (built in Phase 6).

**Phase 0 is complete.** Definition of done met: `cargo build` and `cargo test`
green in CI, `import mnem` works, `mnem --version` runs, `v0.0.1` tagged.
ADR-0001 to ADR-0005 and ADR-0007 Accepted.

## Pending, not blocking

- The `v0.0.1` GitHub release. The tag is pushed; the release page was blocked
  by a session safety check. Cut it from the tag in the web UI, or add a Bash
  permission rule for `gh release`.
- Project board views. The project has all items; add a "group by Milestone"
  view and a "group by Status" board in the UI.

## Next

Phase 1 (`v0.1`, "it commits"), map #9. First: research #19 (object encoding and
the store engine), then ADR-0008 (#20) and ADR-0009 (#21), then the build tickets
starting at #22 (object types).

## Notes

- The pyo3 and maturin binding was moved from Phase 0 to Phase 1, where the core
  first has something to bind. Phase 0 ships a pure-Python placeholder. ROADMAP
  updated to match.
- Two CI fixes were needed for the Python job: the runner's system interpreter is
  externally managed, so `uv pip install --system` fails, and `setup-uv` with
  `python-version` already creates the `.venv`, so a second `uv venv` clashes.
  Settled on: `setup-uv` with `python-version`, then `uv pip install` and
  `uv run` straight into that venv.
- The initial commit on `main` shows a red CI run: it predates the Cargo and
  pyproject files. `main` has been green since PR #8 merged.
