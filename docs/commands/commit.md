# `mnem commit`

Record the staged memory state as a commit.

## Synopsis

```
mnem commit [OPTIONS] --message <MESSAGE>
```

## Description

Records everything currently staged by [`add`](add.md) and
[`rm`](rm.md) as a new commit on the current branch, with `--message`
as the commit message. `--author` records who or what made the
commit; it falls back to the `$MNEM_AUTHOR` environment variable, then
to `"unknown"` - the same fallback shape `--reviewer`, `registry
publish`, and every other author-recording flag in mnem use.

A commit's id is the content hash of its own bytes (ADR-0005), so the
same staged state, message, author and time always produces the same
id - committing is deterministic, not timestamped-and-random the way
many systems' ids are.

## Options

| Flag | Description |
| --- | --- |
| `-m, --message <MESSAGE>` | The commit message. |
| `--author <AUTHOR>` | Who or what made the commit. Falls back to `$MNEM_AUTHOR`, then `"unknown"`. |

## Examples

```
$ mnem add plan "canary rollout, 5% then 50% then 100%"
Staged plan as f1e2d3c4b5a6
$ mnem commit -m "record the rollout plan" --author alice
[a1b2c3d4e5f6] record the rollout plan
```

Setting `$MNEM_AUTHOR` once instead of passing `--author` every time:

```
$ export MNEM_AUTHOR=my-agent
$ mnem commit -m "update after tool call"
```

## See also

- [`add`](add.md), [`rm`](rm.md) - what gets committed.
- [`log`](log.md) - reads back what `commit` wrote.
- [`blame`](blame.md) - resolves a node to the commit that introduced
  it.
