# `mnem approve`

Approve a branch: merge it in with the reviewer recorded as the merge
commit's author.

## Synopsis

```
mnem approve --reviewer <REVIEWER> <BRANCH>
```

## Description

A thin wrapper over [`merge`](merge.md): runs the real merge of
`BRANCH` into the current branch, with `--reviewer` recorded as the
resulting merge commit's author - so `log`/`blame` show who approved
the change, not just who wrote the branch's own commits. Inherits
`merge`'s own conflict handling unchanged: a conflicting approve fails
exactly the way a plain merge would, with the same instruction to
resolve and merge again. Not the ADR-gated propose/approve object a
future review model may settle on (that would hold a pending change
before anyone reacts to it); this is a thin, immediate wrapper over
what `merge` already does.

## Options

| Argument / Flag | Description |
| --- | --- |
| `<BRANCH>` | The branch to approve and merge in. |
| `--reviewer <REVIEWER>` | Who is approving. Recorded as the merge commit's author. |

## Examples

```
$ mnem approve feature-b --reviewer alice
[f7f38658a222] merge feature-b into main
$ mnem log -n 1
commit f7f38658a2224ce0764b1d1258929ac4dc973698cc259bd8214dafea7557d6f3
Author: alice
Date:   2026-10-18 17:29:19Z

    merge feature-b into main
```

## See also

- [`reject`](reject.md) - the alternative: record why a branch is
  rejected, without touching the store.
- [`merge`](merge.md) - what `approve` wraps; use it directly when no
  reviewer needs recording.
- [`inbox`](inbox.md) - branches waiting for an approve or reject.
