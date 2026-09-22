# Debugging a poisoned agent with bisect

This is a walkthrough, not a demo. Every command and every id below is real
output from [`examples/claude_agent.py`](https://github.com/Nabzx/mnemosyne/blob/main/examples/claude_agent.py) -
run it yourself with `python examples/claude_agent.py` and you'll see the
same lines, because the ids are content hashes and the script's inputs are
fixed. Nothing here is staged for the page.

## The setup

A support agent has been working a billing case for a customer, `4821`,
across a few sessions. Before the walkthrough starts, three things have
already happened:

1. It opened the case, reading the original ticket: the customer is on the
   **Enterprise** plan.
2. It noted a primary contact.
3. On a side branch, `direct-callback`, a second interaction confirmed the
   Enterprise plan again - this time by phone, independently of the
   original ticket.

Then, back on the main line, it read a billing note and misread it: the
customer had, it recorded, **downgraded to Pro**. That's the wrong belief.
It's now sitting in memory next to a phone-confirmed right one, and
nothing has reconciled them yet.

## The customer disputes the bill

```
you ▸  Customer on ticket 4821 says they were never on Pro, but your notes say Pro. Work out what went wrong.
```

The agent's current belief is wrong, and it knows it - but "wrong" isn't
useful on its own. What it needs is *when* the belief entered, because
that's what tells you whether it's a bad source, a bad read, or something
else.

```
claude  My current note for customer-4821 is "downgraded to Pro last month".
        A dozen updates have touched this case since - grep finds every
        mention of Pro, not which one is real. Let me bisect for the exact commit.
        ● bisect  customer-4821 == "downgraded to Pro last month"
          d3354bb514df  reconcile the plan tier from billing
            customer-4821 set here, from step-31, source billing-note-8842
```

`bisect` binary-searches the commit history for the first point where a
predicate becomes true - here, "does `customer-4821` equal this exact
string." It doesn't matter how long the history is; this is `O(log n)`
comparisons against the commit graph, not a read through every update
looking for a string match. One call, one answer: commit `d3354bb514df`,
step `step-31`.

## Tracing it to its source

A commit id tells you *when*. It doesn't yet tell you *why* - what the
agent was actually looking at when it wrote that belief.

```
claude  Now trace that commit to the observation behind it.
        ● blame  customer-4821
          content: "downgraded to Pro last month"
          step:    step-31
          source:  billing-note-8842
```

`blame` resolves the current value of a node back to the provenance
attached when it was written. Every `mnem.agents.remember()` call in this
project carries a `source` - here, `billing-note-8842` - so this isn't a
guess or a re-derivation. It's reading back exactly what was recorded at
the time, which is the entire reason provenance is captured on write and
not reconstructed after the fact.

## The twist: a second line already had the right answer

This is the part a simple "find the bad commit and fix it" story misses.
The agent didn't just misread a note - a second, independent interaction
on this same case had *already* gotten the right answer, on its own
branch, at the same time:

```
claude  It came from billing-note-8842 at step-31 - a misread.
        A second line on this case ran in parallel: direct-callback, confirmed by phone.
        ● merge  direct-callback
          conflict on customer-4821: ours "downgraded to Pro last month", theirs "on the Enterprise plan"
```

`merge` doesn't pick a side. Two branches touched the same node with
different values, so it stops and reports both - `ours`, from the misread
billing note; `theirs`, from the phone call - as a real conflict object,
not a silently-applied guess. Nothing is written to the store at this
point. That refusal to write anything until the conflict is explicitly
resolved is the whole point of the operation: a merge that quietly picked
one side would just be a second wrong answer with better manners.

## Resolving it

The two commits carry enough provenance to make the decision, not just
detect that there is one:

```
claude  Ours traces to the misread billing note. Theirs traces to a direct call. Keeping theirs.
        ● merge  direct-callback --resolve customer-4821=theirs
        ● commit    9d120f5ae3cf  merge direct-callback
```

Resolving a conflict is a normal commit like any other - it has its own
id, `9d120f5ae3cf`, and its own message explaining the reasoning, not just
"resolved."

## What the history actually looks like afterward

```
claude  Fixed and reconciled. The history keeps every line of it:
        ● log --oneline
          9d120f5ae3cf  merge direct-callback: trust the phone confirmation over the misread billing note
          d3354bb514df  reconcile the plan tier from billing
          27be6b7ef0dc  confirmed the plan by phone
          b62fe5fdede9  note the primary contact
          cbbebd81b55e  open the case

OK  the wrong belief was found, traced and reconciled
```

Every commit is still there - the misread, the phone confirmation, the
merge that reconciled them. Nothing got quietly overwritten or scrubbed
out to make the story look cleaner after the fact. If the same thing
happens again next month, on this case or another, the same two commands
find it the same way.

## Run it yourself

```bash
git clone https://github.com/Nabzx/mnemosyne.git && cd mnemosyne
python examples/claude_agent.py
```

The full source is in [`examples/claude_agent.py`](https://github.com/Nabzx/mnemosyne/blob/main/examples/claude_agent.py) -
about 90 lines for the whole scenario above, run for real by CI on every
change so it can't drift from what's shown here. `--live` replaces the
scripted dialogue with a real Claude tool-use loop over the same store, if
you'd rather watch the reasoning happen than read a fixed transcript of it.
