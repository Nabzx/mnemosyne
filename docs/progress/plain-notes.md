# Plain notes

What has been built, in plain words. One entry per release tag, newest first.
Written like meeting notes. If you have never seen the code, this is the page
for you.

---

## v0.0.7 - the substrate, measured

**The one-line version:** Era 1 is done. `mnem` is a working, single-agent memory
version-control tool, and now there is a benchmark that says plainly what it does
and does not buy you, plus a docs site and a frozen on-disk format.

**New this release:**

- **A benchmark.** `docs/benchmark.md` reports two things. First, correctness:
  over thousands of seeded runs, reconstructing memory at any past commit is
  exact, `bisect` lands on the exact commit a wrong belief entered, `blame`
  names the right commit and observation, and the merge never loses a write.
  All at 100%. Second, cost: about 12 ms and a few kilobytes per commit,
  compared against a plain dict and a JSONL log. A CI check fails if either
  number doubles.
- **A docs site.** The format spec, the benchmark, every architecture decision
  and every research survey, browsable at the project's GitHub Pages URL. Built
  from the repo on every change.
- **The on-disk format is frozen** for Era 1 at `format_version` 1. Every
  `0.0.x` release reads and writes it; a store written by one works with every
  other. The next format change is Era 2.
- **The Era 2 seam is in place.** A `SemanticMerge` trait and a
  `Store::merge_with` entry point exist in the core, with a no-op Era 1
  implementation, so the "semantic merge" of Era 2 becomes a new package rather
  than a rewrite. Nothing about today's behaviour changes. (ADR-0018.)
- **A Claude agent demo.** `examples/claude_agent.py` and the README GIF: an
  agent is told a past answer was wrong, then uses `bisect` and `blame` to trace
  it to a misread note and commits a correction.

**Named for publishing:** the Rust crates are `mnem-store` (the core) and
`mnem-git` (the `mnem` command); the Python packages are `mnem-agents` (the SDK),
`mnem-mcp` and `mnem-langgraph`. The `mnem` command itself, `import mnem`, and
the `.mnem/` store directory are unchanged.

**What is guaranteed:**

- The core still never touches the network and never calls a model.
- One on-disk format (`format_version` 1), now frozen for the `0.0.x` line.
- The benchmark's correctness checks run on every push, so "time travel is
  exact", "bisect is precise" and "blame is accurate" are tested, not claimed.

**What it still does not do** (Era 2 and later):

- No shared memory between two agents. No semantic merge, no sync between
  stores, no review step. That is the whole of Era 2.
- `mnem` does not make an agent give better answers. The benchmark says so
  directly. It gives you history, audit and safe merging.

**Under the hood:** `benchmarks/`, `book.toml` + `docs/SUMMARY.md`, the `docs`
workflow, `crates/mnem-store/src/semantic.rs`. ADR-0017 and ADR-0018. Published
to crates.io and PyPI for the first time with this tag.

---

## v0.0.6 - plugging in

**The one-line version:** Mnemosyne now drops into the two ways people actually
build agents: as an MCP server, and as a LangGraph memory store. Your agent gets
versioned memory without changing how it is written.

**New this release:**

- **An MCP server.** `mnem-mcp --store ./agent-memory` speaks the Model Context
  Protocol over stdio. Any MCP-capable agent (Claude Desktop, an SDK client) can
  call tools: `remember` a fact (with where it came from), `recall` one or all,
  `why` did the agent conclude this, `when_did` a belief go wrong, and more. The
  current memory is also readable as an MCP resource, so it can sit in the
  agent's context without spending a tool call every turn.
- **A LangGraph store.** `MnemosyneStore` is a drop-in `BaseStore`. Point a
  LangGraph agent at it and its long-term memory gains history: every write is a
  commit. Extra methods let a graph node `branch` to test a hunch, or ask `why`
  a memory says what it does.
- **`mnem.agents` in the SDK.** `remember` / `remember_many` / `forget`: one call
  records a fact (or several) as one commit, and retries quietly if two writers
  race. Every result type can now turn itself into plain JSON with `.to_dict()`.
- **Three worked examples** in `examples/`, one per surface, each a short script
  you can run: a support agent that traces a wrong answer, a LangGraph agent
  with branching memory, and an MCP client talking to `mnem-mcp`.

**What is guaranteed:**

- The Rust core still never touches the network and never calls a model. The MCP
  server and the LangGraph adapter are separate Python packages that sit on top
  of the SDK; a CI check keeps the core clean.
- Every write through an adapter is one commit, so the memory an agent builds
  over a run is fully inspectable afterwards with `log`, `blame` and `bisect`.
- Still one on-disk format (`format_version` 1).

**What it still does not do** (next phases):

- The MCP server speaks stdio only. A remote, multi-client HTTP transport (and
  the auth that comes with it) is a later phase.
- No shared memory between two agents yet. That is the collaboration era.
- The LangGraph `search` is a plain text filter, not semantic. Wrap a vector
  store if you need ranking.

**Under the hood:** `packages/mnem-mcp/` (FastMCP over the SDK) and
`packages/mnem-langgraph/` (a `BaseStore` subclass), both new. `mnem.agents` and
`to_dict()` added to the SDK. Design in ADR-0016. Not on PyPI yet.

---

## v0.0.5 — explaining

**The one-line version:** you can now ask the agent's memory two questions —
"where did this belief come from?" and "when did this go wrong?" — and get an
exact commit back, not a guess.

**New this release:**

- **Blame.** `mnem blame plan` tells you which commit last set the memory called
  `plan`, when, and — if the agent recorded it — which step of the run and which
  observation it came from. Like `git blame`, but for one memory instead of one
  line of code.
- **Blame sees through merges.** If a belief came in from a branch you merged,
  blame follows it back to the commit that actually wrote it, not just to the
  merge.
- **Bisect.** `mnem bisect --node plan --equals '"pro"'` binary-searches the
  history for the first commit where `plan` became `"pro"`. It does about
  `log2(n)` checks, not `n`, so it stays fast over long runs. You can also ask
  `--absent` (first commit where a memory is gone) or `--present` (first commit
  where it appears).
- **Bisect explains itself.** When it finds the commit, it immediately runs
  `blame` on it, so one command tells you both *where* the bad belief entered
  and *what caused it*.
- **`mnem show <commit> --stat`.** A quick list of which memories a commit
  added, changed or removed, without printing all their contents.

**What is guaranteed:**

- Blame and bisect are exact and deterministic. There is a test fixture — a
  synthetic support-agent run that misreads a billing note and writes the wrong
  plan tier — and the tests assert that bisect lands on exactly that commit and
  blame names exactly that observation.
- A small index (`commit_nodes`) makes some of this faster, but it is never
  trusted: every answer is recomputable from the history alone, and a missing or
  stale index just means a slightly slower walk, never a wrong answer.
- Still no internet, still no AI model calls, still one on-disk format
  (`format_version` 1).

**What it still does not do** (next phases):

- No `bisect run <command>` yet (handing bisect an arbitrary script). The
  built-in `--node` predicates cover the common case.
- Bisect assumes the thing you are looking for, once true, stays true, and walks
  a single line of history — same simplification `git bisect` makes.
- No sharing between two agents yet. That is the collaboration era.

**Under the hood:** the Rust engine gained `blame`, `bisect` and the
`commit_nodes` index; the `mnem` CLI and the Python package both gained `blame`,
`bisect`, `changed_by` and `show --stat`. Decisions in ADR-0015. Still built from
source; not on PyPI or Homebrew.

---

## v0.0.4 — merging

**The one-line version:** two branches of an agent's memory can now be combined
back into one. Where they don't overlap, it just works. Where they do, you get a
clear list of the clashes and simple ways to settle each one.

**New this release:**

- **Merge.** `mnem merge experiment` takes everything that happened on the
  `experiment` branch and folds it into the branch you are on.
- **Clean merges just happen.** If the two branches changed different memories
  (or one branch only added things), there is nothing to decide — the merge goes
  through and makes one new snapshot with both sets of changes.
- **Fast-forward.** If your branch has not moved on and the other one has, the
  merge is free: your branch just catches up to the other. No extra snapshot.
- **Conflicts are objects, not text.** When both branches changed the *same*
  memory to *different* things, that memory becomes a conflict. The merge stops
  and shows you, for each clash: the original value, your value, and their
  value. Nothing is written until you resolve it.
- **Resolving.** Per clash you can say `--resolve <name>=ours`, `=theirs`,
  `=base` (the original), or `=delete`. Or settle every clash the same way with
  `--strategy ours` / `--strategy theirs`. Then run the merge again.
- **In Python:**

  ```python
  result = store.merge("experiment")
  if not result.ok:
      for c in result.conflicts:
          print(c.id, c.ours.content, "vs", c.theirs.content)
      store.merge("experiment", resolutions={"plan": "theirs"})
  ```

  You can also hand in a brand-new `MemoryNode` as the resolution for a clash.

**What is guaranteed:**

- The merge is deterministic and structural: it works on *which* memory changed,
  never on *what the text says*. Same inputs, same result, every time — swapping
  which branch is "ours" gives the identical merged memory.
- A conflicted merge writes **nothing**. You never end up in a half-merged state.
- A seeded chaos harness builds thousands of random branch histories and checks
  the merge never loses a write, never leaves a memory in a limbo state, and
  that merging two branches together and then back again lands on exactly the
  same memory. 50,000 cases in the last sweep, no failures
  (`docs/chaos-report.md`).
- Still no internet, still no AI model calls, still one on-disk format
  (`format_version` 1). A merge snapshot is just an ordinary snapshot with two
  parents.

**What it still does not do** (next phases):

- No *semantic* merge. It won't read two versions of a belief and reconcile the
  wording — that is a later era.
- No criss-crossed histories. If two branches were already merged into each
  other in a tangle, `mnem merge` asks you to merge one side by hand first. A
  single agent with a person in the loop rarely hits this.
- No blame or bisect yet (tracing a belief back to where it came from). That is
  the next release.
- No sharing between two agents yet.

**Under the hood:** the Rust engine gained the three-way merge, the merge-base
walk, and the conflict/resolution types; the `mnem` CLI and the Python package
both gained `merge`. Decisions written up in ADR-0013 and ADR-0014. Still built
from source; not on PyPI or Homebrew.

---

## v0.0.3 — branches and time travel

**The one-line version:** the agent can now try an idea on a copy of its memory
and keep it or throw it away, and you can look at exactly what it knew at any
point in the past.

**New this release:**

- **Branches.** `mnem branch experiment` makes a cheap copy of the current
  memory line. It costs almost nothing: a branch is just a pointer, like in Git.
- **Switching.** `mnem checkout experiment` moves you onto that branch; `mnem
  checkout main` moves back. Your work on each branch stays separate.
- **Try-an-idea, in code.** In Python:

  ```python
  with store.branch("assume-downgrade"):
      store.add("customer", "downgraded to Pro")
      store.commit("test this assumption")
  # back on the main line; the "assume-downgrade" branch is still there if you want it
  ```

- **Time travel.** `mnem show <commit>` prints the agent's full memory exactly as
  it stood at that commit. Nothing is reconstructed or approximated; it is the
  real stored state.
- **Forgetting.** `mnem rm <name>` removes a memory from the next snapshot. The
  old snapshots still have it.
- **Seeing what changed.** `mnem diff` shows what is different between two points:
  which memories were added, changed (old value then new value), or removed.
  `mnem status` is the quick version: your branch, and what is staged.

**What is guaranteed:**

- Time travel is exact. There is a test that builds forty different random
  histories and checks, for every commit, that "show me this commit" gives back
  precisely the memory that existed when that commit was made.
- Still no internet, still no AI model calls, still one on-disk format
  (`format_version` 1) that every `0.0.x` release can read.

**What it still does not do** (next phases):

- No merge yet. You can branch, but you cannot yet combine two branches back
  together. That is the next release.
- No blame or bisect yet (tracing a belief to where it came from).
- No sharing between two agents yet.

**Under the hood:** the Rust engine, the `mnem` CLI, and the Python package all
gained the branch, checkout, diff and time-travel operations. Still built from
source; not on PyPI or Homebrew.

---

## v0.0.2 — the first working version

**What Mnemosyne is:** a tool that keeps an AI agent's memory the way Git keeps
code. The agent learns things as it works; this saves each change so you can
look back later.

**What works now:**

- **Make a memory store.** Run `mnem init` in a folder. It creates a hidden
  `.mnem` folder that holds everything, like `.git` does for code.
- **Add a memory.** `mnem add customer-4821 "the customer is on the Enterprise plan"`.
  You can also attach where it came from: which step of the run, which support
  ticket, when the agent formed it.
- **Save a snapshot.** `mnem commit -m "learn the plan tier"`. This freezes the
  current set of memories as one permanent, timestamped record. Each record
  points back to the one before it, so there is a full chain.
- **Look at the history.** `mnem log` lists the snapshots, newest first, with
  who made each one and when.
- **Update a memory.** Add it again with the same name. The next snapshot has
  the new value; the old snapshot still has the old value. Nothing is lost.
- **Do all of this from Python.** `import mnem`, then `store = mnem.init(path)`,
  `store.add(...)`, `store.commit(...)`, `store.log()`. Same features, for agent
  code. Installs as `mnemosyne-agents`.

**What is guaranteed:**

- Every memory and every snapshot has a fingerprint (a hash of its exact
  bytes). Change one character and the fingerprint changes. You cannot quietly
  rewrite history.
- Save a store, close everything, open it again: you get back the exact same
  bytes. There is an automated test that does this on every change.
- The on-disk file format is written down in full and locked for the whole
  `0.0.x` series. A later version will still be able to read a store made today.
- The core never touches the internet and never calls an AI model. A build
  check blocks any library that could.

**What it does not do yet** (these are the next phases):

- No branching. You cannot yet try an idea on a copy of memory and then keep or
  throw it away.
- No time travel. You cannot yet rebuild memory exactly as it stood at an older
  snapshot.
- No merge. You cannot yet combine two separate lines of memory.
- No blame or bisect. You cannot yet trace a belief back to the moment and the
  observation that created it.
- No safe sharing between two agents writing to the same store.

**Under the hood:** a Rust engine (`mnem-core`), a command-line tool (`mnem`),
and a Python package. Not published to PyPI or Homebrew yet; you build it from
source.

**How the project is run:** every change goes through a pull request with seven
automated checks (formatting, linting, tests, a minimum-Rust-version check, a
wheel build, commit-message format, and repo consistency). Every real decision
is written down as a numbered ADR; there are eleven. The roadmap and the issue
tracker are public.
