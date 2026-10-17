# `mnem status`

The current branch, HEAD, and what is staged.

## Synopsis

```
mnem status [OPTIONS]
```

## Description

With no flags, prints the current branch and whatever
[`add`](add.md)/[`rm`](rm.md) have staged but not yet committed -
nothing you have not already seen echoed back by those commands, just
a single place to check it again.

`--all-branches` is a different, dashboard-shaped view over data that
already exists: every branch, its tip commit's real author, and how
long ago it last moved ("5s ago", "2h ago", "3d ago") - useful for
spotting a branch nobody has touched in a while, not a replacement for
[`inbox`](inbox.md)'s own narrower "what's unmerged" question.

## Options

| Flag | Description |
| --- | --- |
| `--all-branches` | A dashboard view: every branch, its tip's author, and how long ago it last moved - instead of just the current branch. |

## Examples

```
$ mnem status
On branch main
  be7a8e64a560  record the rollout plan
nothing staged
```

```
$ mnem status --all-branches
main            be7a8e64a560  alice   1m ago
feature-a       f1e2d3c4b5a6  bob     5s ago
```

## See also

- [`inbox`](inbox.md) - which branches are unmerged, not just when
  they last moved.
- [`branch`](branch.md) - list, create or delete the branches
  `--all-branches` surveys.
