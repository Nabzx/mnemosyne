# `mnem blame`

Resolve a memory node to the commit and provenance that introduced it.

## Synopsis

```
mnem blame <NODE_ID> [REV]
```

## Description

Resolves `NODE_ID`'s current content back to the exact commit that
wrote it - who or what made that commit, when, and the provenance
recorded alongside it (which step of a run produced it, an external
source identifier), if any was given to [`add`](add.md) at the time.
Defaults to blaming as of HEAD; given `REV`, blames as of that branch
or commit instead, the same time-travel [`show`](show.md) does.

## Options

| Argument | Description |
| --- | --- |
| `<NODE_ID>` | The node id to blame. |
| `[REV]` | Blame as of this branch or commit (default: HEAD). |

## Examples

```
$ mnem blame status
d765122d6001  2026-10-17 18:29:47Z
  commit:  status degraded after a tool error
  content: "degraded"
  (no provenance recorded)
```

## See also

- [`bisect`](bisect.md) - find *which* commit introduced a state,
  when you do not already know it; `blame` is for when you do.
- [`add`](add.md) - where the provenance `blame` reads comes from.
