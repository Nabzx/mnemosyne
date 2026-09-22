# Why agent memory needs version control

An agent that runs for any length of time accumulates memory: facts it
learned, decisions it made, things a user told it. Every framework in
common use today stores that memory as state - a row in a database, a key
in a dict, a document in a vector store - and updates it in place. That's
the right choice for almost everything else an agent does. It's the wrong
choice for memory specifically, for one reason: state that's only ever
updated in place has no past tense. You can ask it what it believes. You
cannot ask it what it believed an hour ago, or why.

That gap doesn't matter until an agent gets something wrong. Then it's the
only thing that matters.

## The actual question, and why logging doesn't answer it

Say an agent tells a customer they're on the wrong plan. The real question
isn't "what does the agent believe now" - you already know that, it's the
wrong thing. The question is: *when did that belief enter, and what was it
based on.* That's a different question, and it needs a different kind of
data than most memory systems keep.

It's worth being precise about what "different" means here, because the
obvious fallback - just log everything - looks like it should be enough,
and isn't quite:

| Question you might ask of an agent's memory | a dict | a JSONL log | version control |
| --- | :---: | :---: | :---: |
| what does it believe now | yes | yes | yes |
| what did it believe at step *t* | no | yes | yes |
| when did belief *X* first go wrong | no | yes, linear scan | yes, binary search |
| which observation set *X* | no | no | **yes** |
| what did step *t* change | no | yes | yes |
| merge two agents' memories, surfacing conflicts | no | no | **yes** |

A dict answers almost nothing beyond right now. A log gets you the *time*
questions - what did it believe at some past point - but a log is a list of
events, not a structure, so it can tell you nothing about *why* a value is
what it is, and it has no idea what to do when two logs need to become one.
Finding the moment something went wrong means reading the log by hand or
grepping for the right string; there's no structure to search faster than
that. This table is not a claim, it's [asserted by a CI job on every
change](benchmark.md) - if the "yes" column stops being true, the build
fails before it ships.

Version control answers the same questions a JSONL log does, plus the two
a log structurally can't: what caused a specific belief, and what happens
when two independent lines of memory disagree. Not because version control
is a fashionable word to attach to an idea - because a commit graph is a
different data structure than a list of events, one built specifically to
answer "why" and "which of these two things is right," which is exactly
the shape of question an agent's memory eventually forces on you.

## A concrete version of the abstract problem

Take a support agent, working a billing dispute over a few sessions. Early
on it reads the original ticket and learns the customer is on the
Enterprise plan. Later, working the same case, it reads a billing note and
misreads it - records that the customer downgraded to Pro. Several steps
later, the customer disputes their bill, and the agent's current answer
("Pro") is wrong.

With only current state, there's nothing to do but re-read everything and
guess where it went wrong. With a commit history, this is two commands:

```bash
mnem bisect --node customer-4821 --equals '"downgraded to Pro last month"'
# -> the exact commit where that belief entered

mnem blame customer-4821
# -> content: "downgraded to Pro last month"
#    step:    step-31
#    source:  billing-note-8842
```

`bisect` finds the exact commit a wrong belief first appears in - a binary
search over the run, not a linear read. `blame` resolves the current value
back to the observation that produced it. Neither command is a special
case built for this scenario; they're the same two operations Git gives
code, aimed at belief instead of source lines. The full worked version of
this - including what happens when a second, independent line of memory
disagrees with the first, and how that conflict gets resolved rather than
silently overwritten - is in the project's own [README](https://github.com/Nabzx/mnemosyne#readme)
and [demo](https://github.com/Nabzx/mnemosyne/blob/main/examples/claude_agent.py).

## What this doesn't claim

Versioning an agent's memory does not make the agent more accurate. This
isn't a hedge - it's a settled finding worth taking seriously: GitOfThoughts
(arXiv 2606.14470) tested five memory backends and found none reliably
moves an agent's accuracy. Keeping a perfect history of a belief doesn't
make the belief more likely to be right in the first place.

What it changes is what happens *after* the agent is wrong, which every
agent eventually is. Instead of nothing to inspect, there's an exact commit
and an exact source. Instead of a silent overwrite when two lines of memory
disagree, there's a conflict, surfaced, with both sides visible before
anything is decided. That's a real, if unglamorous, difference: the
distance between "we don't know how this happened" and "here's exactly
what happened, and why" - the same distance version control closed for
code, a long time before anyone was asking whether Git made programmers
write better code. It didn't need to.

There's a real, measured cost to this, too, worth being upfront about
rather than pretending it's free: a commit is a durable write - about 12ms
on the reference hardware, because it's an actual `fsync`, not a database
row update. Against an LLM call measured in seconds, that's noise. It
wouldn't be, at a much higher write frequency than an agent's own reasoning
steps produce - which is a real, current limit, not a hidden one.

[`mnem`](https://github.com/Nabzx/mnemosyne) is one concrete implementation
of this argument - open-source, working, and benchmarked, not a proposal.
