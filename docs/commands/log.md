# `mnem log`

Show the commit history from HEAD, newest first.

## Synopsis

```
mnem log [OPTIONS]
```

## Description

Walks the commit graph from the current branch's tip, newest first.
With no flags, prints each commit in full: its id, author, date, and
message. `--oneline` compresses that to one line per commit - the id
and the first line of the message, the fastest way to scan recent
history. `--first-parent` follows only the first parent of each merge
commit, skipping the commits a merge brought in from the other side -
useful once a history has real merges in it.

## Options

| Flag | Description |
| --- | --- |
| `--first-parent` | Follow only the first parent of each commit. |
| `-n, --max-count <N>` | Show at most this many commits. |
| `--oneline` | One line per commit. |

## Examples

```
$ mnem log --oneline
b7a942a11824 housekeeping
d765122d6001 status degraded after a tool error
2f3ddeee0c3e order-1 shipped
9e451eef0c04 seed order and status
```

The full form, limited to the two most recent commits:

```
$ mnem log -n 2
commit b7a942a11824e0a4c3ea633e7f5f486d79cb7a1921949a684fb2477d30a67afe
Author: alice
Date:   2026-10-17 18:29:55Z

    housekeeping

commit d765122d6001f1caf57b6b86fe256202fce232dba90bb8e7323c518ebe1fef2e
Author: agent
Date:   2026-10-17 18:29:47Z

    status degraded after a tool error
```

## See also

- [`show`](show.md) - the full memory at one of these commits.
- [`blame`](blame.md), [`bisect`](bisect.md) - find a specific commit
  in this history rather than scanning it by eye.
- [`inbox`](inbox.md) - branches whose own commits never show up here
  because they have not merged into the current branch yet.
