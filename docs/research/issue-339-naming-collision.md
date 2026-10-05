# Research: the Mnemosyne naming collision (issue #339)

Feeds a decision, most likely an ADR per #339's own scoping - a rename would
touch the project's display name everywhere it appears, which is squarely
inside [ADR-0011](../adr/0011-conventional-commits-and-issue-lifecycle.md)'s
own "needs an ADR" trigger list for a public-surface change.

A Reddit commenter on the 2026-09-29 launch post flagged one specific
project, `mnemosyne-oss/mnemosyne`, as a same-named prior work. Checked
directly rather than taken on the commenter's word, and the real picture is
bigger than one collision.

Four questions, per #339's own framing:

1. What does `mnemosyne-oss/mnemosyne` actually do, and how much does it
   really overlap?
2. Is this a single collision, or a wider pattern?
3. What would a rename actually cost?
4. Is "Mnemosyne" the real differentiator, or is the git-verb positioning?

---

## 1. `mnemosyne-oss/mnemosyne`: substantive, but on a different axis

Confirmed via the GitHub API and its own README, not assumed from the
Reddit comment:

- **3,336 stars, 286 forks, pushed 2026-10-03** (two days before this
  research) - a large, actively maintained project, not a stale namesake.
- **Backed by NousResearch** (the Hermes model family's own lab) - its own
  topics list `hermes`, `hermes-agent`, `nousresearch`; its Discord badge
  points at the NousResearch server.
- **ProductHunt-launched, benchmarked on BEAM (ICLR 2026)**, with its own
  `mnemosyne-memory` PyPI package.
- **Positioning**: "Zero-cloud AI memory that works everywhere. SQLite-backed.
  One pure-Python dependency." Local-first, no external services, same
  general pitch shape as this project's own "local, deterministic, no
  network, no model calls."

Its own README's "Why Mnemosyne?" table compares itself against Mem0,
Letta, Honcho, SuperMemory, Hindsight, and ChromaDB - the same competitive
set this project's own `docs/comparisons.md` already addresses (Mem0, Zep,
Letta). Its own CLI has `remember`/`recall`/`sleep` (consolidation),
`export`/`import` (a backup/restore pair, not a content-addressed object
graph), and a `sync` command (bidirectional sync between instances) - but
**no commit, branch, merge, blame, or bisect anywhere** in the README, the
CLI reference, or the Python API. It is a retrieval and consolidation
memory layer, scored on recall benchmarks (BEAM, LongMemEval), not a
version-control system.

This confirms the axis this project's own former FAQ already drew in
general terms ("why not mem0, Zep, or a memory-layer product? those manage
*what* an agent remembers... `mnem` manages the *history*") applies here
too, specifically. The overlap is real at the category and name level; it
is not real at the feature level.

## 2. Not one collision - a crowded, non-distinctive name

A plain web search for "mnemosyne agent memory github" surfaces at least
eight distinct, unrelated GitHub projects named `mnemosyne` or a close
variant, all in the agent-memory space:

- `mnemosyne-oss/mnemosyne` (above)
- `rand/mnemosyne` - "agentic memory and orchestration... persistent
  semantic memory across sessions" for Claude Code
- `nikolaef43/mnemosyne` and `kirocop/mnemosyne` - both "The Zero-Dependency,
  Sub-Millisecond AI Memory System for Hermes Agents"
- `onfire7777/Mnemosyne` - explicitly claims "THIS IS THE ORIGINAL MNEMOSYNE
  PROJECT"
- `nianpangzhi233/Mnemosyne-AI-Memory` - "Bionic memory... GraphRAG,
  predictive memory, dream consolidation"
- `28naem-del/mnemosyne` - "Cognitive Memory OS for AI Agents"
- this project, `Nabzx/mnemosyne`

Separately, the plain package name `mnemosyne` on PyPI belongs to yet
another, unrelated project ("a Python package for memory-related
utilities") - a ninth, independent claim on the name.

**"Mnemosyne" (the Greek personification of memory) is an obvious name for
any agent-memory project**, which is almost certainly why so many
unrelated teams landed on it independently. This is a real, structural
finding the original ticket framing did not anticipate: fixing "the"
collision with `mnemosyne-oss` would not fix the deeper problem, since at
least seven other same- or near-named projects exist in the identical
category regardless.

One mitigating factor, also checked directly rather than assumed: this
project's own GitHub description - "Version control for AI agent memory.
Commit, branch, merge, blame and time-travel what your agents know." -
appears distinctly in the same search results, and reads unambiguously
different from every other result's own description. The name collides;
the actual positioning, once a reader's eyes reach the description line,
does not.

## 3. The real cost of a rename is lower than it first looks

Checked what actually carries the name "Mnemosyne" as a *technical*
identifier, not just a display name:

- **No PyPI collision on the real package names.** This project's own
  packages are `mnem-agents`, `mnem-mcp`, `mnem-langgraph`,
  `mnem-openai-agents`, `mnem-crewai`, `mnem-autogen` - none is `mnemosyne`
  or `mnemosyne-*`. The plain `mnemosyne` PyPI name is taken by a third,
  unrelated party (confirmed directly via the PyPI JSON API), and would
  have been unavailable to this project regardless of the rename question.
- **No crates.io collision.** The Rust crate is `mnem-git`, not
  `mnemosyne`.
- **The CLI binary is already `mnem`, not `mnemosyne`.** A user's actual
  typed commands (`mnem init`, `mnem commit`, `mnem merge`, ...) never
  contain the contested name at all.

What a rename would actually touch is narrower than the ticket's own
framing worried: the **project's display name and GitHub repo slug**
(`Nabzx/mnemosyne` - GitHub redirects the old slug automatically, lowering
this cost further), the docs site title and URL
(`nabzx.github.io/mnemosyne`), and every README/docs/ADR prose reference to
"Mnemosyne" the name - a real but mechanical find-and-replace, not a
package-republishing or user-migration problem. No existing user's
`pip install`, `cargo install`, or CLI invocation would break.

## 4. Is the name the differentiator, or is the positioning?

The evidence above answers this directly: across every project found that
shares the name, **none** describes itself with commit/branch/merge/blame/
bisect language. The git-verb positioning is this project's own, real,
and - as far as this research found - unique within the set of projects
that happen to share its name. The name "Mnemosyne" itself carries no part
of that differentiation; it is a generic, independently-rediscovered label
for "a thing that remembers," not a brand asset tied to the actual
technical claim.

## What this research does not decide

Whether to rename, and if so to what, is the real open decision - a
product and branding call for the project's own maintainer to make, not
one this research should make unilaterally. This document states the
facts a decision needs: the overlap is real and wider than one project, it
is category-and-name-deep rather than feature-deep, and the technical cost
of a rename is lower than the ticket's own original framing assumed.
