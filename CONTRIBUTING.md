# Contributing

## External contributions

Mnemosyne is developed by a single maintainer. External pull requests from other
people are not merged. Issues, bug reports, and design feedback are welcome and
read.

The one exception is Dependabot, which is the maintainer's own dependency-update
tooling, configured in `.github/dependabot.yml`. The maintainer reviews each
Dependabot pull request and re-lands the change under their own commit, so the
history stays single-author.

This keeps the history clean and the design coherent while the project is
young. It may change once the substrate has shipped.

## How the maintainer works

### Issues first

Nothing is built without an issue. Each phase opens with a Wayfinder map issue
that names the open decisions. Research and grilling tickets resolve those into
ADRs. Build tickets are labelled `ready-for-agent` only once their spec is
settled.

### ADR before code

Write an ADR before writing code for anything that touches:

- the on-disk format
- the object model
- a public API surface (the CLI, the SDK, the MCP tools)
- the merge algorithm
- the Rust and Python boundary
- a new dependency

ADRs live in `docs/adr/`, numbered, following `docs/adr/0000-template.md`.

### Branches and commits

- One branch per issue: `feat/<slug>-<n>`, `research/<slug>-<n>`, `adr/<slug>-<n>`.
- One pull request per branch. CI must be green before merge.
- Commits are small and cover one logical change.
- Commit messages and pull request titles follow Conventional Commits
  (ADR-0011): `<type>(<scope>): <description>`, lowercase, imperative, British
  English, plain and concise, no marketing tone, no em dashes, no co-author
  trailer. Types: `feat`, `fix`, `docs`, `test`, `refactor`, `perf`, `build`,
  `ci`, `chore`, `revert`. Scope is a crate or area (`core`, `cli`, `sdk`,
  `format`, `adr`) when it sharpens the subject.
- Put `refs #<n>` in the body, not `closes #<n>`. Issues close when the release
  that contains them is tagged, not on merge (ADR-0011).
- `python scripts/conventional_commits.py --range main..HEAD` checks the format
  locally.
- The author of every commit is the maintainer.

### Docs move with the code

If a change alters behaviour, update `CONTEXT.md` (if a term shifts),
`ROADMAP.md` (if scope shifts), and the phase note in `docs/progress/`.

### Tests

- Rust: `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`.
- Python: `ruff check python`, `pytest python/tests`.
- From Phase 3, the merge property harness runs in CI.
