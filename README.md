<div align="center">

# Mnemosyne

**Version control for AI agent memory.**

[![Licence: Apache 2.0](https://img.shields.io/badge/licence-Apache_2.0-3b82f6.svg)](https://github.com/Nabzx/mnemosyne/blob/main/LICENSE)
&nbsp;[![CI](https://github.com/Nabzx/mnemosyne/actions/workflows/ci.yml/badge.svg)](https://github.com/Nabzx/mnemosyne/actions/workflows/ci.yml)
&nbsp;[![crates.io](https://img.shields.io/crates/v/mnem-git.svg)](https://crates.io/crates/mnem-git)
&nbsp;[![PyPI](https://img.shields.io/pypi/v/mnem-agents.svg)](https://pypi.org/project/mnem-agents/)
&nbsp;[![PyPI downloads](https://img.shields.io/pypi/dm/mnem-agents.svg)](https://pypi.org/project/mnem-agents/)
&nbsp;[![Docs](https://img.shields.io/badge/docs-nabzx.github.io-1f6feb.svg)](https://nabzx.github.io/mnemosyne/)
&nbsp;![Status: early development](https://img.shields.io/badge/status-early_development-f59e0b.svg)

</div>

<p align="center">
  <img src="https://raw.githubusercontent.com/Nabzx/mnemosyne/main/assets/demo.gif" alt="A support agent is told a past answer was wrong. It runs bisect to find the commit where the belief entered, blame to trace it to a misread billing note, then merges in a parallel branch that had the right answer, resolving a real conflict. The history keeps every line of it." width="900" />
</p>

An AI agent builds up memory as it works: facts it learns, decisions it makes. Frameworks store that as state it overwrites as it goes. Mnemosyne gives agent memory what Git gives code: **commits, branches, merge, blame and bisect.** A Rust core, a `mnem` CLI, and a Python SDK. Local, deterministic, no network, no model calls.

## The problem

Your agent runs for an hour, makes forty tool calls, and updates its memory the whole way. Then it gets something wrong, and all you have is the memory as it stands right now. You cannot see when the bad fact got in, what the agent knew before it went off track, or why it believes what it believes. Version control solved exactly this for code.

## What you get

| Command | What it does |
| --- | --- |
| `commit` · `log` | record memory as an immutable, content-addressed snapshot with its provenance, and walk the history |
| `show <commit>` | reconstruct the memory exactly as it stood at any past commit |
| `branch` · `checkout` · `diff` | fork memory for the price of a pointer to explore a hypothesis in isolation |
| `merge` | combine two lines of memory, surfacing conflicts as objects you inspect and resolve |
| `blame` | resolve any belief to the commit (through merges) and the observation that introduced it |
| `bisect` | binary-search a run for the first commit where a wrong belief appears |
| `export` · `import` | dump the whole history as one portable JSON file, and rebuild a store from it |

<p align="center"><img src="https://raw.githubusercontent.com/Nabzx/mnemosyne/main/assets/panel-agent.svg" alt="A Python agent loop: a Claude call, store.add with provenance, store.commit, and store.blame tracing a belief to its origin" width="820" /></p>

Every correctness property above - exact reconstruction, precise `bisect`, accurate `blame`, no lost writes on `merge` - holds at 100% across an 80-seed, 720-run sweep, plus a separate 50,000-case fuzz test on the merge algorithm, gated in CI on every change. Full detail: [the benchmark](https://github.com/Nabzx/mnemosyne/blob/main/docs/benchmark.md).

## Quickstart

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/Nabzx/mnemosyne/releases/latest/download/mnem-git-installer.sh | sh
# no Rust toolchain needed; PowerShell equivalent: mnem-git-installer.ps1 on the same release page
pip install mnem-agents     # the Python SDK; `import mnem`
```

Already have Rust? `cargo install mnem-git` builds the CLI from source instead.

Three commands, and an agent's memory is under version control:

```bash
mnem init ./agent-memory && cd ./agent-memory
mnem add customer-4821 "on the Enterprise plan" --source ticket-4821
mnem commit -m "open the case" --author agent
```

Now watch it catch a real mistake:

```bash
# an hour later, the agent misreads a billing note:
mnem add customer-4821 "downgraded to Pro last month" --source billing-note-8842 --step step-31
mnem commit -m "reconcile the plan tier" --author agent

# a wrong answer surfaces. find where it entered, and why:
mnem bisect --node customer-4821 --equals '"downgraded to Pro last month"'
mnem blame customer-4821
```

The Python SDK:

```python
import mnem

store = mnem.init("./agent-memory")
store.add("customer-4821", "on the Enterprise plan",
          provenance=mnem.Provenance(source="ticket-4821"))
store.commit("learn the plan tier", author="support-agent")

b = store.blame("customer-4821")       # which commit set this, and why
print(b.commit[:8], b.provenance.source)
```

Shell completions:

```bash
mnem completions zsh > ~/.zfunc/_mnem     # or bash, fish, powershell, elvish
```

More worked examples, one per surface (SDK, Claude, LangGraph, MCP), live in
[`examples/`](examples/) - the README GIF above is `examples/claude_agent.py`.

## Coming from Git

| Git | `mnem` |
| --- | --- |
| `git init` | `mnem init` |
| `git add` | `mnem add` |
| `git commit` | `mnem commit` |
| `git log` | `mnem log` |
| `git branch` | `mnem branch` |
| `git checkout` | `mnem checkout` |
| `git diff` | `mnem diff` |
| `git merge` | `mnem merge` |
| `git blame` | `mnem blame`, same idea |
| `git bisect` | `mnem bisect`, same idea |

What is missing on purpose, for now: `push` / `pull` / `clone` (the sync protocol between stores is Era 2), a staged hunk (`add -p`, staging is a whole node), and a text-merge conflict marker (a conflict is an object you resolve with `--resolve <id>=ours|theirs|base|delete` or `--strategy`, not an inline marker).

## The vision

Code needed version control, then a way to share and build on it together. AI agents need exactly the same thing, and it does not stop at memory: it reaches the whole agent, how agents work together, and eventually the physical machines they run on. Mnemosyne is building that, end to end:

1. **Version control** *(now)*: single-agent versioned memory, local and deterministic.
2. **The collaboration layer**: agents working together on shared memory, with a review step before a change lands, the same way a pull request works for code.
3. **The platform**: the whole agent, not just its memory, saved, shared and forked the way code is on GitHub today.
4. **The translation layer**: one agent definition, written once, running on any framework instead of rebuilt for each one.
5. **Physical AI**: robots are the future. The same version control and translation layer, applied to physical skills instead of software memory, so what one system learns can move to another.

We start with software agents. That is the easiest place to prove it works. But physical AI is the new phenomenon, and it is what this is ultimately built for.

## Prior work

A wave of 2026 research points at this idea (Git4Data, GitOfThoughts, StateFuse, MemTX, LatticeMind), each a paper or a prototype. One finding is worth stating plainly: versioned memory does not make an agent give better answers. What it gives you is history, audit, and safe merging. That is the whole pitch, and it is enough. The [benchmark](https://github.com/Nabzx/mnemosyne/blob/main/docs/benchmark.md) has the numbers, [why I built this](https://github.com/Nabzx/mnemosyne/blob/main/docs/why.md) has the longer version, and [why agent memory needs version control](https://github.com/Nabzx/mnemosyne/blob/main/docs/why-version-control.md) makes the general case.

Want the same story worked end to end, one command at a time, with real commit ids instead of a compressed summary? [Debugging a poisoned agent with bisect](https://github.com/Nabzx/mnemosyne/blob/main/docs/debugging-a-poisoned-agent.md) narrates exactly what the GIF above is doing.

## Contributing & licence

Developed by a single maintainer; issues and feedback welcome, but not open to external pull requests at this stage ([`CONTRIBUTING.md`](https://github.com/Nabzx/mnemosyne/blob/main/CONTRIBUTING.md)). Apache 2.0 ([`LICENSE`](https://github.com/Nabzx/mnemosyne/blob/main/LICENSE)).
