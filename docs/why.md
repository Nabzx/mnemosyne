# Why I built this

An agent runs for an hour. It reads a ticket, makes a note, reads a billing
record, makes another. Forty tool calls later it tells a customer something
wrong. You go looking for why, and there is nothing to look at - just
whatever the agent believes right now. Not what it believed an hour ago,
not which of those forty calls put the wrong idea in, not why it trusted
the source it trusted. The memory is a single mutable blob, and the moment
it goes wrong is the moment its own history disappears.

That's the whole problem. Not "agents need memory" - most of them already
have some. The problem is that the memory has no past tense.

## The frame that already exists

Software had exactly this problem, and solved it decades ago. A codebase is
also just a mutable blob until you put it under version control - then
every change is a commit, you can check out any past state, `blame`
resolves a line back to whoever wrote it and why, and `bisect` finds the
exact commit a bug entered without anyone reading the diff by hand. None of
that makes the code better. It makes the code *accountable* - you can
always answer "when did this happen, and why."

`mnem` is that idea, pointed at what an agent remembers instead of what a
repo contains. Every write is a commit. `branch` forks a hypothesis for the
price of a pointer. `blame` resolves any current belief to the exact commit
and the observation that produced it. `bisect` binary-searches a whole run
to find the first commit where a wrong belief appears - no reading logs by
hand, no grepping for "Pro" across forty tool calls to find the one that
mattered.

The part that took the longest to get right, and the part I think actually
matters most, is `merge`. Two lines of memory - two branches, two agents,
two sessions on the same case - can each learn something true, and when
they meet, they can disagree. A support case gets confirmed by phone on one
line while a billing note gets misread on another; both are real commits,
both look confident, and only one is right. `merge` doesn't pick a winner
for you. It surfaces the conflict as an object - here's what each side
believes, and why - and writes nothing until something, a human or a
policy, actually resolves it. That's the one piece of this that doesn't
exist anywhere else I've found: real memory tools don't have branches to
reconcile in the first place, and the closer prior art I've since found
either doesn't block on the disagreement or resolves it with a plain text
merge marker instead of a typed answer. The comparison is written down,
with the actual source read before any claim was made about it, not just
each project's README: [`docs/comparisons.md`](comparisons.md).

## What this doesn't claim

Worth being upfront about, because it would be easy to oversell: versioned
memory does not make an agent give better answers. A 2026 wave of research
into roughly this idea - Git4Data, GitOfThoughts, StateFuse, MemTX,
LatticeMind - converges on the same honest finding, and the benchmark here
backs it: `mnem` doesn't move accuracy. What it gives you is history,
audit, and safe merging. That's the whole pitch, and on reflection I think
it's enough - the actual cost of a wrong belief in production isn't that
the model was wrong once, it's that nobody could tell *when* it became
wrong, or trust the fix wasn't just another guess. That's a solved
problem for code. It shouldn't still be an unsolved one for memory.

The benchmark is public and adversarial, not a curated demo: every
correctness property - exact reconstruction, precise `bisect`, accurate
`blame`, no lost writes on `merge` - holds at 100% across an 80-seed,
720-run sweep, plus a separate 50,000-case fuzz test on the merge algorithm
specifically, gated in CI on every change. If any of that regresses, the
build fails before it ships.

## What's deliberately not here yet

- **No shared memory between agents yet.** Everything today is one agent,
  one store, local and offline - no network calls, ever, even to a model.
  A real sync protocol between stores is the next thing, not this one.
- **No semantic merge.** `merge` today is structural: it catches that two
  branches disagree, it doesn't understand *why* two claims contradict
  each other. The seam for a content-aware resolver is already built into
  the core and proven against the current format - not implemented yet,
  on purpose, until there's a second store to actually merge with.
- **MCP is stdio only.** The right shape for one agent talking to one
  local store; a remote/multi-tenant transport is a different, harder
  problem for whenever shared memory exists to justify it.
- **The Python wheels aren't fully cross-platform yet.** The `mnem` binary
  itself installs anywhere without a Rust toolchain; the Python package
  publish pipeline for every platform is mid-rollout as I write this.

None of these are secret gaps. They're the next things, in order, and
naming them here is the same discipline the rest of the project runs on -
say what's actually true, not what would sound better.

## Where this goes

The substrate above - commit, branch, merge, blame, bisect, all of it - is
Era 1, and it's what exists today. Era 2 is the collaboration layer: a
sync protocol between stores, and a real review step before an update
lands in shared memory - a proposal, a diff, an approval, the same shape a
pull request has for code. Era 3 is the actual destination: the whole
agent - its prompt, tools, memory, policy, evaluations - versioned and
signed as one forkable artefact, with a registry to publish, discover, and
fork them. GitHub, eventually, but for agents instead of code.

Era 1 had to be right before any of that was worth building. A
collaboration layer over an object model that quietly loses data on merge
isn't worth having. So this is where I started, and where I've been most
careful: [the on-disk format](format/README.md), [the merge
algorithm](adr/0013-deterministic-merge-algorithm.md), [the provenance
index that makes `blame` possible](adr/0015-provenance-index-blame-and-bisect.md),
[the seam Era 2 will build on](adr/0018-the-era-2-seam.md).

It's open-source, Apache-2.0, and installable today - `pip install
mnem-agents`, or a one-line binary install with no Rust toolchain required.
If you're running agents in production and any of this sounds like a
problem you actually have, I'd like to hear about it.
