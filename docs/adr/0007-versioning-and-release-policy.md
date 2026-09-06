# ADR-0007: Versioning and release policy

- Status: Accepted
- Date: 2026-09-06
- Issue: #17 (grilling)

## Context

ADR-0004 fixed that the crate, the wheel and the CLI carry one version number and
move together. ADR-0002 said the on-disk format is "frozen within a major
version" and that a breaking format change ships `mnem migrate`. This ADR turns
that into a concrete policy: what the version scheme is, what a bump means, how
the format version relates to the software version, and how a release is cut.

The hard part is the second question. ADR-0002 and ADR-0003 call adding a new
object kind, such as `claim` in v2, "additive". But a v1.0 reader rejects
unknown kinds, so a store written by a later v1.x is not fully readable by v1.0.
If every format change of any kind forced a major software release, `claim`
support alone would make Mnemosyne 2.0.

## Decision

### Version scheme

**SemVer 2.0.0.** One number for the crate, the wheel and the CLI.

Under 1.0, the Cargo 0.x convention: in `0.MINOR.PATCH`, a bump of `MINOR` may
break the public API or the format, a bump of `PATCH` may not. This holds
through the whole v0.x roadmap. From v1.0, full SemVer applies.

### The format version is a separate track

`config` carries a `format_version` integer. It is **not** the software version.
The software version is an API contract; `format_version` is a data contract.

Each release declares the `format_version` range it can read and the single
version it writes. The mapping between a format change and a software bump:

| Format change | `format_version` | Software bump | Migration |
| --- | --- | --- | --- |
| none | unchanged | PATCH or MINOR per the API | none |
| additive (a new object kind, a new optional field) | +1 | MINOR | none. An older release cannot open a newer store, but a newer release opens any older store it declares support for. |
| breaking (an old reader would misread an existing object) | +1 | MAJOR | `mnem migrate` reads the old store and writes a new one. |

So `claim` support in v2 is an additive change: `format_version` goes up by one,
and it ships in a MINOR software release, not a major one. A change that alters
how an existing object is interpreted is major and gets a migration.

An older release that meets a `format_version` it does not know refuses the
store with a clear message naming the version it would need.

### Pre-release tags

SemVer pre-release identifiers: `vX.Y.Z-rc.N`, `vX.Y.Z-beta.N`, `vX.Y.Z-alpha.N`.
A hyphenated tag publishes as a GitHub pre-release, and as a pre-release version
on crates.io and PyPI, which is not installed by default. `mnem --version` prints
the full string including the identifier.

### The release process

A release is gated on, in order:

1. `main` is green.
2. The version in `Cargo.toml` is bumped to the intended `X.Y.Z` and matches the
   tag about to be pushed.
3. `CHANGELOG.md` has a dated section for `X.Y.Z`, moved down from `Unreleased`.
4. The tag `vX.Y.Z` is pushed.
5. A `release` workflow builds and publishes the crate to crates.io and the
   wheels to PyPI, and cuts a GitHub release with the changelog section as its
   notes.

The workflow refuses if the `Cargo.toml` version and the tag disagree. Building
the workflow is a Phase 6 task (#68); this ADR fixes the policy it enforces.

### Changelog

A hand-written `CHANGELOG.md` in the Keep a Changelog format. Any user-visible
change updates it in the same pull request, under an `Unreleased` section that is
dated and renamed to the version at release time. Commits are already small and
issue-referenced; a curated changelog reads better than a generated one, and
there is one maintainer to keep it honest.

### Deprecation

Post-1.0: a deprecated public API element (a CLI flag, an SDK method) is marked
deprecated in a MINOR release, keeps working with a warning for at least one
further MINOR, and may be removed only in the next MAJOR.

Pre-1.0: deprecation is best-effort. An element can be removed in a `0.x` MINOR
with a changelog note.

### Yanking

If a published crate or wheel is broken, `cargo yank` the crate version and mark
the PyPI release yanked. The version number is never reused. The fix goes out as
the next PATCH. The git tag stays, because history is immutable, and the GitHub
release is marked as broken.

## Consequences

- A v2 that adds `claim`, the Contradiction object and the embedding index is a
  MINOR software release, since all three are additive. Mnemosyne 2.0 is
  reserved for a genuine break.
- Two version numbers to keep straight: the software SemVer and
  `format_version`. The mapping table above is the single source of truth, and
  `mnem --version` will print both.
- An older release cannot open a store written by a newer one. This is accepted:
  a release never promised to read the future, and the error message is
  explicit.
- The changelog is manual work on every user-visible PR. Accepted for the
  quality it buys a reader.

## Alternatives considered

**Any format change is a MAJOR software bump.** Rejected. It conflates the data
contract with the API contract and turns every additive tweak into a major
release, which trains users to ignore major bumps.

**Additive format changes are a MINOR bump, with no separate `format_version`.**
Rejected. Without a `format_version` integer, an old release has no clean way to
detect that a store is too new, and would fail with a parse error rather than a
useful message.

**Generate the changelog from commits or PR labels.** Rejected. A generated
changelog is a list of commits, not a description of what changed for a user.
With one maintainer and small commits, hand-writing it is cheap.

**Independent version numbers for the crate, the wheel and the CLI.** Settled in
ADR-0004: rejected there, and this ADR inherits the one-number rule.
