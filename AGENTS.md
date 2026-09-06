# Agent guide

How an engineering agent should work in this repo.

## Skills

### Issue tracker

Issues and specs live in this repo's GitHub Issues. See
[`docs/agents/issue-tracker.md`](docs/agents/issue-tracker.md).

### Domain docs

Single-context layout: [`CONTEXT.md`](CONTEXT.md) and [`docs/adr/`](docs/adr) at
the repo root. See [`docs/agents/domain.md`](docs/agents/domain.md).

### Wayfinder

Each phase opens with a `wayfinder:map` issue. Research and grilling tickets
hang off it and resolve into ADRs before build tickets start. The frontier is
the set of open child tickets with no open blocker.

## Invariants

These do not change without an ADR that supersedes this list.

### One maintainer

Mnemosyne has a single maintainer, Nabil Shah. Every commit, issue, and pull
request is authored by him. External pull requests are not merged; they are
declined politely and closed. No commit carries a co-author trailer.

### `mnem-core` is offline and model-free

Nothing in `mnem-core` opens a network connection or calls a model. The v1
product is deterministic. The semantic layer in v2 lives behind a trait, in a
separate crate, and its default implementation is a local model.

### The on-disk format is frozen within a major version

Once `docs/format/` is marked frozen for a version line, a change to the object
format requires a major version bump and a migration path. A store written by
one v1.x release is readable by every other v1.x release.

### A commit's id does not include its signature

An object's id is the BLAKE3 hash of its canonical bytes (ADR-0005). A commit's
hashed bytes are `{kind, parents, state, message, author, time}`. Signatures live
in a side table. Signing, re-signing, or adding a signature never changes any
id, and signing is never required by any operation.

### The human is always the gate in v2

When the review model lands, an approval is only ever given by a person or an
explicit policy they configured. A critic agent advises; it never approves or
vetoes on its own.

## House style

British English. Plain, short sentences. No marketing tone in commits, issues,
or ADRs. No em dashes. Commit messages are present tense and reference the
issue, for example `add the commit object and its content hash (#7)`.
