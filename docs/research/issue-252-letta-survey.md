# Research: a Letta adapter (issue #252)

Feeds the ADR that fixes a `mnem-letta` adapter's contract - or, depending on
what this research finds, the ADR that explains why no adapter is the right
call right now. Unlike #249-#251, the premise itself needed checking first:
issue #252 was filed on the strength of `docs/comparisons.md`'s Letta section
("agent templates carry numbered versions with a documented rollback
endpoint, and a `block_history` mechanism chains a hash over each change to a
memory block"). That description is no longer accurate.

## 1. The premise has gone stale: Letta pivoted since that research was done

`letta-ai/letta` (the repo `docs/comparisons.md` researched) now ships only
policy and legal files at its root - no source at all. Its own README:

> Letta (f.k.a. MemGPT) is actively developed. The current source code lives
> in `letta-ai/letta-code`... The `archive` branch contains the retired
> Letta V1 API server.

The block-history/checkpoint-rollback API `docs/comparisons.md` described is
the **retired V1 server**, now on an `archive` branch, not the active
product. This needs its own correction to `docs/comparisons.md` regardless of
what this ticket concludes about an adapter - a separate, smaller finding,
noted here and worth its own ticket.

## 2. What Letta is today: `letta-code`, a CLI tool, and its memory model is a real git repo

`letta-ai/letta-code` (Apache-2.0, npm `@letta-ai/letta-code`, Bun/TypeScript)
describes itself as "a CLI tool for interacting with stateful Letta agents
from the terminal" - the same category as Claude Code, not a Python agent
framework. Its own source docstring, verbatim:

> Git operations for git-backed agent memory. When memFS is enabled, the
> agent's memory is stored in a git repo on the server at
> `$LETTA_MEMFS_BASE_URL/v1/git/$AGENT_ID/state.git`... clone on first run,
> pull on startup, commit memory writes, post-turn push for clean pending
> commits.

This is not an analogy or a hash-chain-as-marketing-claim (which is roughly
what the retired V1 `block_history` was). It is a literal git repository,
cloned, pulled, and pushed like any other, holding the agent's memory as real
files at `~/.letta/agents/<agentId>/memory/`. `src/agent/memory-git.ts` is
2,092 lines with real credential handling, commit signing, retry-on-transient-
network-error, and Windows-specific credential plumbing - substantial,
production infrastructure, not a toy.

**But it is plumbing, not a versioning feature.** The exported surface
(`commitMemoryWrite`, `pushMemory`, `pullMemory`, `cloneMemoryRepo`,
`getMemoryHeadRevision`, `getMemoryGitStatus`) has no `log`, no `checkout-at-
revision`, no `branch`, no `merge`. `pushToMemoryRepository`'s non-fast-
forward handling (`isNonFastForwardPushError`) just returns the raw error
text as a failure - it does not merge, does not retry with a rebase, does not
produce anything resembling `mnem`'s `Conflict` object. Confirmed in Letta's
own documentation (`src/skills/builtin/migrating-memory/SKILL.md`): the
*recommended, first-party workflow* for moving memory between agents is
`cd ~/.letta/agents/$LETTA_AGENT_ID/memory && git add ... && git commit -m
"..." && git push` - literally telling the user to drive raw git themselves.
The same doc states plainly: "the legacy block-level CLI commands have been
removed."

**This is the sharpest positioning point found in any adapter research so
far.** Letta gives a user a real git repository and tells them to use real
git by hand if they want history. `mnem` gives `blame`, `bisect`, a typed
merge-conflict object, and a real object model on top of the same idea. Same
substrate, completely different level of actual capability.

## 3. There is no Python API for the current memory model

The `python/` directory in `letta-code` is a **packaging shim only**:
`wheel_backend.py`'s own docstring - "Wheel-only backend... There is
deliberately no sdist: installing must never compile or fetch Node/npm... The
launcher is Python ABI independent, but its payload is platform specific."
`pip install letta` gets you the compiled CLI binary via a Python entry point
(`letta = "letta_code:main"`), not a memory API. There is no Python object
resembling a `Session`, `Memory`, or `StorageBackend` for this adapter to
implement - the pattern every prior adapter (#249-#251) followed doesn't
apply here at all.

A separate PyPI package, `letta-client` (v1.12.1, last released 2026-06-02),
exists - "the official Python library for the letta API." This is almost
certainly a REST client generated against Letta Cloud's hosted API (the same
scope as the TS `@letta-ai/letta-client` package `letta-code` itself depends
on for `AgentState`/`CreateBlock` types), not the git-backed memfs system
specifically - `CreateBlock` being a live type import suggests the
block-oriented Cloud API is still reachable there even though the CLI's own
local-first path has moved past it. Not independently verified against a live
server in this research pass (no credentials, no running instance) - a real
open item, not assumed.

## Recommendation

Two honest paths, not a single clear winner - this research surfaces the
fork rather than resolves it:

**(A) No adapter for now; fix the stale `docs/comparisons.md` claim instead.**
There is no clean Python integration surface matching any prior adapter's
shape. The interesting technical opportunity - `mnem`'s object model layered
on the *same* git repo Letta already syncs - is a contribution to Letta's own
CLI, not an `mnem`-side adapter package, and is a materially different kind
of undertaking (upstream PR to someone else's TypeScript codebase) than
anything ADR-0020/0021/0023 scoped.

**(B) A thin Python package against `letta-client`'s REST API**, scoped
honestly to whatever that client actually exposes (verified against a real
Letta Cloud account before any contract is fixed, not assumed from package
metadata) - if it's block-CRUD only, this would be a `mnem`-agents-adjacent
sync helper more than a `Session`/`Memory`-shaped adapter, and would need its
own real research pass first to fix scope.

This research recommends (A) as the honest default - fix the stale
comparisons claim now, defer the adapter question until either Letta ships a
real Python-side memory API, or there's a concrete reason to build the
REST-client-only version B describes. But this is exactly the kind of call
that belongs to the grilling, not to this doc alone.

## Open questions for the grilling

- **Build nothing, or build the thin REST-client version (B)?** If (B): what
  does `letta-client` actually expose today, verified against a live
  account - and is a block-CRUD sync helper worth building at all given the
  CLI itself has moved past blocks?
- **The stale `docs/comparisons.md` Letta section**: fix it as part of this
  ticket's own close-out, or file it as its own separate, smaller ticket?
  Either way it needs fixing - it currently describes retired software as
  current.
- **Is the "contribute upstream to Letta" angle worth naming publicly**
  anywhere (a discussion post, an issue on `letta-ai/letta-code`), even
  though it's explicitly out of scope for this project's own adapter work?

## Sources

- `letta-ai/letta` (cloned at research time, current `main`): `README.md`
  (confirms the pivot and the `archive` branch).
- `letta-ai/letta-code` (cloned at research time, Apache-2.0, npm
  `@letta-ai/letta-code` v0.33.0): `src/agent/memory-git.ts` (the git-backed
  memory implementation, `commitMemoryWrite`/`pushMemory`/`pullMemory`/
  `isNonFastForwardPushError`), `src/agent/memory-filesystem.ts` ("With
  git-backed memory, most sync/hash logic is removed"),
  `src/skills/builtin/migrating-memory/SKILL.md` ("the legacy block-level CLI
  commands have been removed", the `git add`/`commit`/`push` recommended
  workflow), `python/wheel_backend.py` (confirms the Python package is a
  binary-distribution shim, not an API).
- `pypi.org/pypi/letta-client/json` (live PyPI metadata, checked at research
  time: v1.12.1, released 2026-06-02) - existence confirmed, contents not
  independently verified against a live server.
- `docs/comparisons.md` (#237's existing Letta section - the claim this
  research found to be stale).
