# Retention

**There is no garbage collection anywhere in the engine today.** Not a
missing feature with a workaround - no code path exists anywhere in
`mnem-store` that removes a commit, a state, or a memory node once
written. The closest thing to a deletion primitive, `delete_branch`,
removes a name-to-commit pointer only; its own doc comment states the
property plainly:

> No force variant: with no garbage collection, a deleted branch's
> commits stay reachable by id.

The same is true of `rm` (a tombstone commit recording that a node was
removed from working memory, not an erasure of the commits that
created or changed it) and of every adapter's own `clear`/
`clear_session`/`pop_item` method - all tombstone commits, confirmed
directly against each adapter's own implementation, never a hard
delete.

## What this means in practice

**Every commit, once made, is retained for the life of the store.**
This is a real, already-true property of the current architecture -
not a promise about a future feature, not a policy decision someone
made and wrote down, just a consequence of there being no deletion
code at all. `log`, `blame`, and `bisect` can walk back to the first
commit a store ever had, for as long as the store file exists.

## What this is not

This document states an existing fact about the engine, not a
retention *policy*. Whether that fact is sufficient on its own to meet
a given regulatory retention requirement (for example, the EU AI
Act's Article 12 automatic event-recording and Article 26 minimum-
retention provisions) is a separate question this document does not
answer - it would need the primary legal text read against the
current architecture, which is real research, not a documentation
task. See [map #376](https://github.com/Nabzx/mnemosyne/issues/376)
for that open question.

Nor does "no GC exists" mean a store's files can never be deleted - a
caller can always remove the `.mnem/` directory itself, or the whole
store, at the filesystem level. What's absent is any in-engine
operation that selectively forgets part of a store's history while
keeping the rest.
