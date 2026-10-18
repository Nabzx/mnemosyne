# `mnem reject`

Reject a branch: record why, without touching the store.

## Synopsis

```
mnem reject --reviewer <REVIEWER> --reason <REASON> <BRANCH>
```

## Description

Prints who rejected `BRANCH` and why - the store is never opened, so
there is nothing to roll back if a reviewer says no. The companion to
[`approve`](approve.md): the two give a reviewer a single pair of
commands for the same decision, one of which writes a real commit and
one of which writes nothing at all.

## Options

| Argument / Flag | Description |
| --- | --- |
| `<BRANCH>` | The branch being rejected. |
| `--reviewer <REVIEWER>` | Who is rejecting. |
| `--reason <REASON>` | Why the branch is rejected. |

## Examples

```
$ mnem reject escalation --reviewer alice --reason "resolution already shipped a fix"
Rejected escalation (alice): resolution already shipped a fix
```

## See also

- [`approve`](approve.md) - the alternative: merge the branch in with
  the reviewer recorded.
- [`inbox`](inbox.md) - branches waiting for an approve or reject.
