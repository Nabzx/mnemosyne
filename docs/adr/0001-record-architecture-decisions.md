# ADR-0001: Record architecture decisions

- Status: Accepted
- Date: 2026-09-06
- Issue: #1

## Context

Mnemosyne is being built slowly and deliberately, one phase at a time, with the
design settled before the code. The reasoning behind each choice needs to
survive, both for the maintainer returning to an area months later and for
anyone reading the repo to judge whether the project is serious.

The design space here is unusually live. A wave of 2026 research (Git4Data,
GitOfThoughts, StateFuse, MemTX, LatticeMind) is exploring the same ground, and
findings from those papers feed directly into decisions. Those inputs need a
home.

## Decision

Every architecture decision is recorded as a numbered Markdown file in
`docs/adr/`, following `0000-template.md`.

An ADR is written, and reaches Accepted, before code is written for anything
that touches: the on-disk format, the object model, a public API surface, the
merge algorithm, the Rust and Python boundary, or a new dependency.

ADRs are immutable once Accepted. A later decision that changes an earlier one
is a new ADR that marks the old one Superseded.

## Consequences

- The repo carries its own reasoning. A reviewer can follow why the format looks
  the way it does without asking.
- Research is not lost. A `wayfinder:research` ticket produces an ADR
  recommendation, which becomes an ADR.
- There is a small tax on every structural change: the ADR comes first.
- The ADR index in `README.md` for this folder has to be kept current.

## Alternatives considered

**A design doc that is edited in place.** Rejected. It loses the history of what
was tried and dropped, which is exactly the part worth keeping here.

**Decisions recorded only in issues and pull requests.** Rejected. They are hard
to find later and get buried under implementation chatter. An ADR is a stable,
linkable artefact.
