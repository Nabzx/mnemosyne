# ADR-0009: The ref model

- Status: Accepted
- Date: 2026-09-06
- Issue: #21 (grilling)

## Context

ADR-0002 and ADR-0005 fixed that `HEAD` is a plain-text file and the source of
truth for the current pointer, and ADR-0008 fixed that named refs live in a
`redb` table `refs: &str -> [u8; 32]`. This ADR fixes the semantics: what a ref
is, how it is named, what `HEAD` contains, and what happens on a concurrent
update or a branch deletion.

The design space is small and well understood from git and jujutsu. The
decisions below take git's model and drop the parts that exist only because
git's refs are files on a filesystem.

## Decision

### A ref is a branch

In v1 a ref is a branch: a mutable `name -> commit id` pointer. There are no tags
(immovable pointers) in v1. Nothing needs them yet; they are a plausible later
addition.

### Flat namespace

Refs are keyed by their bare name in the `refs` table: `main`, `hypothesis-x`.
There is no `refs/heads/` hierarchy. Git's hierarchy separates heads from tags
and remotes; v1 has only heads. If tags or remotes arrive (v2 sync, v3), a
prefix convention is an additive change, not a format break.

### Naming rules

A branch name is a non-empty string that:

- contains only Unicode letters, digits, `-`, `_`, `.`, `/`
- does not start or end with `/` or `.`
- contains no `..`, no whitespace, and no control characters

Names are case-sensitive. `HEAD` is reserved and is not a branch name.

### `HEAD`

`HEAD` is a one-line plain-text file.

- **Attached**: `ref: <branch-name>`. `HEAD` follows the branch; a commit moves
  the branch and `HEAD` with it.
- **Detached**: `<64-hex-commit-id>`. `HEAD` points straight at a commit;
  committing from here is refused until a branch is created.
- **Fresh store, no commits**: `ref: main`. `main` does not exist in `refs` yet.
  The first commit creates it.

### The default branch

`main`. Overridable by a `default_branch` key in `config` for anyone who wants
something else.

### Ref updates are compare-and-swap

A `mnem commit` moves a branch inside the same `redb` write transaction that
writes the objects (ADR-0008), so the move is atomic. The move is also a
compare-and-swap: the transaction asserts the branch still points where `HEAD`
resolved it to when the commit started. On a mismatch the commit fails with "the
branch moved under you" and writes nothing.

This costs nothing inside the transaction. Single-agent v1 will almost never hit
it. It is the primitive v2's concurrent agents need, so it goes in now.

### Branch deletion

`mnem branch -d <name>` removes the `refs` row.

- Deleting the branch `HEAD` points at is refused.
- Deleting the last branch of a store that has a commit is refused.
- Commits that only the deleted branch reached become unreachable but are not
  removed (no GC in v0.x, ADR-0008). They are recoverable by hash.

### Reflog: reserved, not built

A reflog (a per-ref history of the commits a ref has pointed at) fits the
product and is cheap: append `(ref, old, new, time, op)` to a `reflog` table on
every ref move. It is not needed for the Phase 1 definition of done, and it
interacts with the eventual GC design, so it is deferred. It lands as its own
ticket in Phase 2, or alongside `blame` and `bisect` in Phase 4.

## Consequences

- The ref layer is a thin wrapper over one `redb` table plus a text file. Small.
- Compare-and-swap on every commit means the concurrency story is already
  correct when v2 arrives; no retrofit.
- Without a reflog, recovering a mistakenly deleted branch needs its tip hash.
  Since there is no GC, the commits are still there; the user just needs to know
  the hash. The reflog closes that gap when it lands.
- A flat namespace means adding tags later needs a naming convention (a `tags/`
  prefix, or a second table). That is a Phase-7 concern and additive.

## Alternatives considered

**Lightweight tags in v1.** Rejected. Nothing uses them, and they add naming and
deletion rules for no v1 benefit.

**A git-style `refs/heads/` hierarchy.** Rejected for v1. It exists to separate
ref categories git has and v1 does not.

**No compare-and-swap; last write wins.** Rejected. It is free to add now and
painful to retrofit once v2 has concurrent writers, and "last write silently
wins" is the wrong default for a tool whose job is not losing history.

**Build the reflog in v0.1.** Rejected. Not on the Phase 1 critical path, and
better designed together with GC.

**`HEAD` stored in `redb` rather than a text file.** Settled in ADR-0002:
rejected there, because `HEAD` is the one pointer a person most often reads or
sets by hand.
