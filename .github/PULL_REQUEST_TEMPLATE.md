## What this does

One or two sentences. Reference the issue in the body: `refs #<n>`.

## Why

The reason, or a pointer to the ADR or map that called for it.

## Checklist

- [ ] Title and commits follow Conventional Commits (ADR-0011); `refs #<n>`, not `closes #<n>`
- [ ] Advances a labelled issue
- [ ] An ADR covers anything touching the on-disk format, the object model, a public API, the merge algorithm, the Rust and Python boundary, or a new dependency
- [ ] `cargo fmt`, `cargo clippy`, `cargo test` pass locally
- [ ] `ruff check` and `pytest` pass locally
- [ ] Commits are small, British English, no co-author trailer
- [ ] Docs updated (`CONTEXT.md`, `ROADMAP.md`, `docs/progress/`, `CHANGELOG.md`) if behaviour changed
