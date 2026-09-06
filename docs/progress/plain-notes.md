# Plain notes

What has been built, in plain words. One entry per release tag, newest first.
Written like meeting notes. If you have never seen the code, this is the page
for you.

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
