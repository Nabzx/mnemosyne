# `mnem show`

Print the full memory at a commit, or working memory with no argument.

## Synopsis

```
mnem show [OPTIONS] [REV]
```

## Description

With no argument, prints working memory - everything
[`add`](add.md)/[`rm`](rm.md) have staged plus every earlier commit,
materialised as it stands right now. Given a branch name or commit id,
materialises memory exactly as it stood at that point in history
instead - real time travel, not a diff against the present. `--stat`
swaps the content for just the list of node ids that commit changed.

## Options

| Argument / Flag | Description |
| --- | --- |
| `[REV]` | A branch name or commit id. |
| `--stat` | Instead of the memory, list the node ids this commit changed. |

## Examples

Working memory right now:

```
$ mnem show
note
  "unrelated housekeeping"
order-1
  "shipped"
status
  "degraded"
```

Memory as it stood at an earlier commit - real time travel, not
re-deriving the past from a diff:

```
$ mnem show 9e451eef0c04
order-1
  "pending"
status
  "ok"
```

## See also

- [`log`](log.md) - find the commit id to point `show` at.
- [`diff`](diff.md) - the difference between two `show`-able points,
  instead of one point in full.
