# ADR-0018: The Era 2 seam: the semantic merge trait and the sync protocol

- Status: Accepted
- Date: 2026-09-08
- Issue: #63 (grilling)

## Context

Era 1 (`0.0.x`) is the substrate: one agent, local, deterministic. Era 2 is the
collaboration layer: a semantic merge that reasons about contradiction, a sync
protocol between stores, and a review step before a change lands in shared
memory. Pull requests, for agent memory.

Era 2 is not built in `v0.0.7`. But `v0.0.7` freezes the Era 1 format
(`docs/format/`) and hands the substrate to strangers, so the places Era 2 plugs
in have to be right now, or Era 2 becomes a rewrite of `mnem-core` rather than a
new crate. This ADR fixes two seams:

- the `SemanticMerge` trait: where a content-aware resolver attaches to the
  structural merge from ADR-0013 and ADR-0014;
- the sync protocol shape: the named pieces of store-to-store exchange, without
  the transport or the wire format.

ADR-0004 keeps `mnem-core` offline and model-free. The trait lives in the core
(it is part of the merge API); every impl that calls a model lives in a separate
crate. The sync protocol is declared here and built in Era 2 under its own ADR.

## Decision

### The `SemanticMerge` trait

The structural merge runs first and unchanged (ADR-0013): it produces a merged
`State` map plus a list of `Conflict` values. `SemanticMerge` is a **second pass
over the conflicts the structural merge could not settle**:

```rust
pub trait SemanticMerge {
    /// Try to settle one structural conflict by reasoning about the nodes on
    /// each side. Called once per `Conflict` the structural merge produced.
    fn resolve(&self, store: &Store, conflict: &Conflict) -> Result<Verdict>;
}

pub enum Verdict {
    /// The resolver settled it: apply this resolution (ADR-0014).
    Resolved(Resolution),
    /// The two sides genuinely disagree and a human or another agent should
    /// see it. Carries the draft of the `Contradiction` object Era 2 stores.
    Contradiction(ContradictionDraft),
    /// The resolver has nothing to add: fall through to the caller's strategy,
    /// or leave the conflict for the caller (the structural behaviour).
    Unresolved,
}

pub struct ContradictionDraft {
    pub id: String,
    pub base: Option<ObjectId>,
    pub ours: Option<ObjectId>,
    pub theirs: Option<ObjectId>,
    pub reason: String,
}
```

`resolve` takes `&Store` so a resolver can load the node content on each side
(`Store::node`) and walk history (provenance, prior commits) to judge. A
narrower context type is a possible Era 2 refinement; `&Store` is the read
surface that already exists.

`Verdict` reuses ADR-0014's `Resolution` unchanged and adds exactly one case,
`Contradiction`, for "these disagree and the disagreement is worth keeping".

The core ships one impl:

```rust
pub struct StructuralOnly;
// resolve() always returns Verdict::Unresolved
```

`StructuralOnly` is the Era 1 behaviour: no content reasoning, every structural
conflict comes back to the caller.

### Where it attaches to `Store::merge`

`Store::merge` keeps its exact signature (ADR-0014, Accepted and now frozen). A
new method takes the resolver:

```rust
pub fn merge_with(
    &self,
    theirs: &str,
    resolver: &dyn SemanticMerge,
    resolutions: &BTreeMap<String, Resolution>,
    strategy: Option<MergeStrategy>,
    message: Option<&str>,
    author: &str,
    time_ms: i64,
) -> Result<MergeOutcome>;
```

`merge` is exactly `merge_with(theirs, &StructuralOnly, ..)`. The precedence per
conflicting id is: an explicit `resolutions` entry from the caller, then the
resolver's `Verdict`, then `strategy`, then unresolved. A caller stays in
control, the resolver is the automated middle, and `strategy` is the blunt
fallback.

`merge_with` is a Rust-only API in Era 1. The SDK's `store.merge(..)` is
unchanged; a Python-visible resolver is Era 2's, alongside the crate that
implements one.

### `Contradiction`: a stored object in `format_version` 2

ADR-0014 kept `Conflict` transient and deferred a stored form to Era 2's review
model. `Contradiction` is close to that stored form, and it is different in kind
from a `Conflict`: a `Conflict` is "the tool will not choose"; a `Contradiction`
is "a semantic resolver looked, and the claims are incompatible" (CONTEXT.md).

In Era 2, `Contradiction` becomes a fourth content-addressed object kind
(`format_version` 2, after the three in `docs/format/`), referenced from the
merge commit, so the disagreement is kept in history rather than dropped. Sketch
of the fields: the node `id`, the object id on each side, the resolver's
`reason`, and the merge commit it was found in. The exact shape, its encoding
row in `docs/format/`, and the `MergeOutcome` arm that carries contradictions
out of `merge_with` are Era 2's, under the `format_version` 2 ADR.
`ContradictionDraft` above is the in-memory precursor: what a resolver returns
before anything is stored.

In `v0.0.7`, `StructuralOnly` never returns `Verdict::Contradiction`, so no
contradiction is produced through `merge`. A custom resolver that returns one is
treated as `Unresolved` for now (the conflict comes back to the caller); the
`MergeOutcome` arm and the stored object land together in Era 2.

### The sync protocol shape

Era 2 lets two stores exchange history. This ADR names the pieces and defers the
rest:

- **A `Remote`**: a named pointer to another store, the way a branch is a named
  pointer to a commit. `mnem remote add <name> <location>`.
- **Content-addressed exchange**: objects are addressed by hash (ADR-0005), so
  syncing is a negotiation of "which object ids does each side hold", then a
  transfer of only the missing objects and the ref updates. Dedup is free and
  the transfer is minimal, the property that makes `git fetch` cheap.
- **A `Proposal`**: a cross-store change does not write a shared branch
  directly. It opens a `Proposal` (a source ref, a target ref, the commit range,
  a status) that a reviewer, human or agent, approves or rejects before it
  lands. This is the "pull request for memory" step.

What this ADR does **not** fix, all deferred to a dedicated Era 2 ADR: the
transport (HTTP, SSH, a file path), the wire format, the ref negotiation
protocol, the `Proposal` object's exact fields, authentication, and the
`mnem push` / `mnem pull` / `mnem review` command surface. Those depend on how
the Era 3 registry wants to serve stores, and picking them now in isolation is a
guess.

The commitment this ADR **does** make: a cross-store landing goes through a
reviewable `Proposal`. Era 2 is the review model, or it is just `rsync`.

### What stays out of `mnem-core`

The trait is in the core. `StructuralOnly` is in the core. Everything else Era 2
adds is not: a model-calling `SemanticMerge` impl is a separate crate (ADR-0004);
the sync transport touches the network and is a separate crate; the `Proposal`
review flow is CLI and orchestration. `cargo deny` still bans network, TLS and
async crates from the workspace core.

## Consequences

- Era 2's merge work is "write a crate that implements one trait", not "reopen
  `mnem-core`'s merge flow". `merge_with(&StructuralOnly, ..)` is exercised by
  the existing merge tests, so the seam is load-bearing from `v0.0.7`.
- `Store::merge` and `MergeOutcome` are untouched, so ADR-0014 stays frozen with
  the rest of the Era 1 surface.
- `format_version` 2's contents are now named in one place (`docs/format/` links
  here): the stored `Contradiction`, whatever sync adds to a commit or the refs,
  and the prolly-tree `State` if ADR-0017's trigger fires.
- The sync protocol is still an open design. That is deliberate: its shape
  depends on the registry, which is Era 3. Naming the pieces stops Era 1 from
  quietly closing a door, for example a non-content-addressed object id, which
  would break the cheap-sync property.
- A `SemanticMerge` impl that fails (a model call errors) surfaces as
  `MnemError`. Era 2 adds a variant for it; `#[non_exhaustive]` on the enum
  already allows that without a break.

## Alternatives considered

**A whole-`State` resolver** (given the three maps, return merged plus conflicts
plus contradictions). Rejected: it makes every semantic resolver re-implement
the structural walk from ADR-0013. The per-`Conflict` shape lets the structural
merge always run and the resolver be a clean second pass. `StructuralOnly` is
then a one-line impl.

**`merge` grows an `Option<&dyn SemanticMerge>` parameter.** Rejected: it breaks
a signature that is Accepted and, at `v0.0.7`, frozen. `merge_with` is additive.

**ADR-0018 is doc-only; the trait lands in Era 2.** Rejected: the point of #63
is that the seam is proven before the format freezes. A trait with a real impl
and test coverage now is proof; a paragraph is not.

**Fix the sync transport now** (say, HTTP with a `GET /objects` endpoint).
Rejected: the transport should follow the Era 3 registry's needs, which do not
exist yet. Naming the pieces (content-addressed exchange, a `Remote`, a
`Proposal`) protects the properties that matter without guessing the wire.

**No review step: sync writes straight to a shared branch.** Rejected: that is
`git push` to a shared `main` with no review, and the whole Era 2 pitch is the
review. The mechanism is deferred; the commitment is not.
