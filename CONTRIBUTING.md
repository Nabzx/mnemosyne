# Contributing

## External contributions

Mnemosyne is developed by a single maintainer. External pull requests from other
people are not merged, with one carve-out: `docs/` and `examples/` are open to
real external PRs (typo fixes, clarity improvements, a new worked example) -
lower review risk, no correctness or security surface. Everywhere else
(`crates/`, `packages/`, the CLI) stays solo-maintained until the substrate has
shipped, the same bar this section already names below. A PR outside the
`docs/`/`examples/` carve-out is closed with a pointer to opening an issue
instead, not silently left open. Issues, bug reports, and design feedback are
always welcome and read.

The one exception is Dependabot, which is the maintainer's own dependency-update
tooling, configured in `.github/dependabot.yml`. The maintainer reviews each
Dependabot pull request and re-lands the change under their own commit, so the
history stays single-author. An open Dependabot pull request is not a stalled
external contribution - it is closed once triaged, whether that means
re-landing it (the usual case) or closing it with a reason if the bump isn't
wanted yet; it is not left open as a queue.

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
- `just commits` checks the format locally; the pre-commit hook and CI enforce it.
- The author of every commit is the maintainer.

### Docs move with the code

If a change alters behaviour, update `CONTEXT.md` (if a term shifts),
`ROADMAP.md` (if scope shifts), and the phase note in `docs/progress/`.

### Checks

`just ci` runs everything CI runs, in the same order: `cargo fmt --check`,
`cargo clippy -D warnings`, `cargo test`, the release build, `cargo deny check`
(bans, licences, advisories and sources), the Python lint and tests, the
adapter packages' lint/tests plus the worked `examples/`, the benchmark
regression gate, the repo-consistency checks (ADR index, ADR references, the
changelog), and the conventional-commit check. It must pass before a pull
request.

`just install-hooks` sets `.githooks` as the hooks path, so `git commit` runs a
fast pre-commit check (formatting and the Python lint).

Individual recipes: `just fmt`, `just test`, `just clippy`, `just msrv`, `just
deny`, `just py`, `just adapters`, `just benchmark`, `just checks`, `just
wheel`, `just commits`, `just coverage`. Run `just` to list them.

A `coverage` CI job also runs on every push, uploading an HTML report as a
build artefact - a report, not a gate. There is no coverage badge yet: a
live one needs an external service (Codecov or similar), which needs the
maintainer's own account and token, not something to add silently.

From Phase 3, the merge property harness runs in CI.

## Releasing

ADR-0007 sketches a release process it hands to "a `release` workflow"; what
actually shipped (ADR-0019, #173) is two independently-triggered workflows and
a couple of steps that stay manual on purpose. This section is the real
sequence, kept current here rather than by amending an Accepted ADR.

1. On a `chore/release-vX.Y.Z` branch: bump `[workspace.package] version` in
   the root `Cargo.toml`, move `CHANGELOG.md`'s `Unreleased` section to a
   dated `[X.Y.Z]` heading (fresh empty `Unreleased` after it), bump the
   hardcoded version asserts in `python/tests/test_binding.py`, and add a
   top entry to `docs/progress/plain-notes.md`. PR it, wait for green CI,
   merge.
2. On `main`: `git tag -a vX.Y.Z -m vX.Y.Z && git push origin vX.Y.Z`.
3. `cargo publish -p mnem-store` then `cargo publish -p mnem-git` (path
   dependency, so that order). Not automated in CI - see the "no `cargo
   publish` in any workflow" note below.
4. Create the GitHub Release for the tag as a **draft** (`gh release create
   vX.Y.Z --draft --notes-from-tag`, or the web UI) - do not publish it yet.
   ADR-0019's `create-release = false` means cargo-dist expects this draft to
   already exist; it does not create one.
5. `gh workflow run release.yml -f tag=vX.Y.Z` - cargo-dist builds the `mnem`
   binaries for all 5 targets, attaches them to the draft, and undrafts it
   once everything is attached.
6. Undrafting a GitHub Release fires the same `release: published` event a
   manual publish would. `release-pypi.yml` listens for exactly that event,
   so it should start on its own and publish `mnem-agents`/`mnem-mcp`/
   `mnem-langgraph` wheels to PyPI via trusted publishing, no manual
   `maturin`/`twine` step needed. **This chain (undraft -> auto-triggered
   PyPI publish) has not yet been exercised end-to-end** - #173's own test
   used `pr-run-mode = "upload"`, a different path. Check the Actions tab
   after step 5; if `release-pypi.yml` did not start, fall back to
   `gh workflow run release-pypi.yml -f ref=vX.Y.Z -f publish=true`.
7. Close whatever issues/epics the release covers and move on.

**No workflow touches crates.io** (`cargo publish`, `CARGO_REGISTRY_TOKEN`) -
step 3 is manual by design, and stays that way until there's a reason to
automate it.
