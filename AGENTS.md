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

Nothing in `mnem-core` opens a network connection or calls a model. The
substrate is deterministic. `deny.toml` bans every network, TLS and
async-runtime crate from the workspace tree, and the `deny` CI job enforces it;
this is the check ADR-0004 promised. The semantic layer (Era 2) lives behind a
trait, in a separate crate, and its default implementation is a local model.

### The on-disk format changes only by the rules in ADR-0007

`config` carries a `format_version` integer, on its own track from the software
SemVer. An additive format change (a new object kind, a new optional field)
bumps `format_version`. A change that would make an older reader misread an
existing object bumps `format_version` and ships `mnem migrate`. A newer release
opens any older store it declares support for; an older release refuses a newer
store with a clear message. The software bump that carries a `format_version`
change is a MINOR from `0.1.0` on, and a `0.0.x` bump while the roadmap is still
in `0.0.x` (ADR-0010).

### A commit's id does not include its signature

An object's id is the BLAKE3 hash of its canonical bytes (ADR-0005). A commit's
hashed bytes are `{kind, parents, state, message, author, time}`. Signatures live
in a side table. Signing, re-signing, or adding a signature never changes any
id, and signing is never required by any operation.

### The human is always the gate in Era 2

When the review model lands, an approval is only ever given by a person or an
explicit policy they configured. A critic agent advises; it never approves or
vetoes on its own.

## House style

British English. Plain, short sentences. No marketing tone in commits, issues,
or ADRs. No em dashes.

Commit messages and pull request titles follow Conventional Commits (ADR-0011):
`<type>(<scope>): <description>`, lowercase, imperative, concise, for example
`feat(core): add the commit object and its content hash`. The body, when
present, stays plain and explains why. Reference an issue with `refs #<n>` in
the body, never a closing keyword; issues close when their release is tagged,
not on merge. `scripts/conventional_commits.py` enforces the format in CI.
