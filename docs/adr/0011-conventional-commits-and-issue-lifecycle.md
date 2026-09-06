# ADR-0011: Conventional commits and the issue lifecycle

- Status: Accepted
- Date: 2026-09-06
- Issue: #100

## Context

The project has around fifty commits in a plain student voice, and issues close
the moment a pull request merges to `main`. Both were fine while there was no
release cadence. As the history grows and releases begin (ADR-0007, ADR-0010),
two gaps show:

- A plain subject line carries no machine-readable signal. A convention lets
  release notes group changes, lets a check cross-reference the hand-written
  changelog, and tells a reader the project is run to a standard.
- Closing an issue on merge to `main` conflates "in `main`" with "shipped". A
  store written by `v0.0.2` is the contract, not whatever is on `main`.

This ADR fixes the commit convention and the issue lifecycle. It does not change
the hand-written changelog (ADR-0007) or the versioning scheme (ADR-0010).

## Decision

### Conventional commits

Every commit message and every pull request title follows
[Conventional Commits 1.0.0](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

<body>

refs #<issue>
```

- **type**: one of `feat`, `fix`, `docs`, `test`, `refactor`, `perf`, `build`,
  `ci`, `chore`, `revert`.
- **scope**: optional, lowercase, a crate or area: `core`, `cli`, `sdk`,
  `format`, `adr`, `ci`. Use it when it sharpens the subject.
- **description**: lowercase, imperative, British English, concise, no trailing
  full stop. Aim for 72 characters.
- **body**: optional, plain student voice, wrapped. Explains why, not what.
- No co-author trailer, ever (unchanged, AGENTS.md).

Earlier commits are left as they are; the history has a small discontinuity at
this ADR.

### `refs #N`, not closing keywords

A commit or pull request that relates to an issue puts `refs #<n>` in the body.
GitHub's closing keywords (`closes`, `fixes`, `resolves`) are not used, so a
merge to `main` never closes an issue on its own.

### Issues close at release

An issue closes when the tag that contains its change is pushed, not when its
pull request merges. Between merge and release the issue carries the
`pending-release` label. A phase's map issue closes when its milestone tag is
pushed.

### Enforcement

`scripts/conventional_commits.py` validates the subject of every non-merge
commit in a pull request, and the pull request title, against the format above,
and rejects closing keywords. It has a unit test and runs in a CI `commits` job
on `pull_request` events.

## Consequences

- Release notes and `git log --oneline` become scannable by type.
- One more check to pass on a pull request. It is fast and local via `just ci`.
- The changelog stays hand-written; a later check can diff it against the
  commit types in a release range.
- The tracker now distinguishes "merged" from "released", at the cost of a
  batch issue-close step at each tag.

## Alternatives considered

**Keep the student voice.** Rejected. No machine signal, and a weaker read for
anyone judging whether the project is serious.

**Conventional pull request titles only, free-form commits.** Rejected. The
squash history would be clean but the branch history stays noisy, and the check
would only be half a check. Merges here are merge commits, not squashes, so the
individual commits land on `main`.

**Commitizen or semantic-release.** Rejected. Overkill for one maintainer, and
ADR-0007 already chose a hand-written changelog over a generated one.
