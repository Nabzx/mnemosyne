# ADR-0010: What 1.0 means, and the 0.0.x roadmap

- Status: Accepted
- Date: 2026-09-06
- Issue: #95

## Context

ADR-0007 set the version scheme: one SemVer number for the crate, the wheel and
the CLI, with a separate `format_version` integer for the data contract. It was
written against a roadmap that reached 1.0 at the end of a six-phase substrate
build, and it said the 0.x convention "holds through the whole v0.x roadmap".

Since then the ambition has settled. The end state is not the single-agent
substrate; it is the agent as a versioned, signed, forkable artefact with a
registry to publish and improve agents, what earlier docs call v3. The substrate
and the multi-agent collaboration layer are both groundwork for that. Numbering
the substrate's first working build 0.1.0, a sixth of the way to 1.0, oversells
how far along the project is.

This ADR fixes what 1.0 means and how versions are numbered until then. It does
not change the `format_version` track or the release process from ADR-0007.

## Decision

**1.0 is the platform.** Mnemosyne 1.0.0 is the release where an agent, its
prompt, tools, memory, policy and evaluations are versioned and signed as one
artefact, with a registry to publish, discover and fork them. Nothing smaller is
1.0.

**Three eras, all pre-1.0 except the last.**

- **Era 1, the substrate.** Single-agent versioned memory: commit, branch,
  merge, blame, bisect, time travel. Local and deterministic. This is the
  current six-phase roadmap.
- **Era 2, the collaboration layer.** Semantic merge over claims, a sync
  protocol between stores, and a review step before an update lands in shared
  memory.
- **Era 3, the platform.** The agent as a repository, plus the registry. Ships
  as 1.0.0.

Where older ADRs, research notes and issues say "v1", "v2" or "v3", read Era 1,
Era 2 and Era 3. None of them meant a software major version.

**The roadmap is 0.0.x.** Every milestone from here until the eras are
substantially done is a `0.0.x` tag. Phase 1 is `v0.0.2`, following Phase 0's
`v0.0.1`; each later phase is the next patch, through `v0.0.7`. Era 2's work
continues in the same range.

`0.0.x` is initial development. Per SemVer, anything may change on any bump: the
public API and the on-disk format included. ADR-0007's stricter promise, that a
PATCH bump may not break the API or the format and a MINOR may, takes effect at
`0.1.0`. When to cut `0.1.0`, or whether to move straight from `0.0.x` to
`1.0.0`, is decided when Era 3 is in view and the API is worth stabilising.

**The format version track is unchanged.** `format_version` still starts at 1
and still increments by exactly the rules in ADR-0007's table, so an older
release always meets a store it cannot read with a clear message. The only
difference while in `0.0.x` is that a `format_version` increment ships in a
`0.0.x` bump rather than a MINOR one.

## Consequences

- The version number matches the honest state of the project: barely started,
  on a long road.
- One renumber, once: the six roadmap milestones become `v0.0.2` through
  `v0.0.7`. The `ROADMAP.md` table, the `README.md` arc, the GitHub milestones
  and the phase label descriptions are updated in the same change as this ADR.
- ADR-0007 stays Accepted. This ADR narrows one of its clauses for the `0.0.x`
  period and schedules when the rest applies; it supersedes nothing.
- Until `0.1.0`, the `format_version` integer, not the software version, is the
  stability signal. A reader who sees `0.0.4` knows not to build anything
  load-bearing on the format yet.

## Alternatives considered

**Keep 0.MINOR per phase (Phase 1 = 0.1.0 ... Phase 6 = 0.6.0), push 1.0 out in
the narrative only.** Least churn, but 0.6.0 for a single-agent substrate with
no multi-agent story still reads as most of the way to done. Rejected.

**Two 0.x eras: 0.0.x for the substrate, 0.1.x onward for collaboration.**
A cleaner boundary, but it commits now to cutting 0.1.0 at a point that is still
far off and better decided later. Rejected for the looser 0.0.x range.

**A dated amendment to ADR-0007 instead of a new ADR.** ADRs are immutable once
Accepted (ADR-0001). A new ADR is the mechanism.
