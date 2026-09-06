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
- The Phase 0 Wayfinder map (#1) with six child tickets (#2 to #7), sub-issue
  linked, with #4 blocked by #3 and #6 blocked by #5.
- The scaffold (#2) built on `chore/phase-0-scaffold`, merged via PR #8 with
  green CI.

## Pending

- Tag `v0.0.1` and cut the release (blocked in this session; to be done by the
  maintainer).
- The GitHub project board (needs the `project` token scope:
  `gh auth refresh -s project,read:project`).
- The Phase 0 frontier: #3 (research, object format), #5 (research, memory
  model), #7 (grill, Rust and Python boundary). #4 and #6 stay blocked until
  their research tickets close.

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
