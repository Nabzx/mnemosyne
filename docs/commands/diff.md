# `mnem diff`

Show what changed between two memory states.

## Synopsis

```
mnem diff [OPTIONS] [FROM] [TO]
```

## Description

Structural diff over two memory states, one line per changed node:
`+` added, `-` removed, `~` modified (shown with both the old and new
content). With no arguments, compares the HEAD commit against working
memory - "what have I staged that isn't committed yet". Given one
side, compares that commit against working memory; given both, a pure
historical comparison with no live state involved at all. `--stat`
drops the content and shows just the classified node ids; `--name-
only` drops the classification too, just the ids.

## Options

| Argument / Flag | Description |
| --- | --- |
| `[FROM]` | The `from` side: a branch or commit. Default: the HEAD commit. |
| `[TO]` | The `to` side: a branch or commit. Default: working memory. |
| `--stat` | One classified line per node, no content. |
| `--name-only` | Just the changed node ids. |

## Examples

```
$ mnem diff 9e451eef0c04 d765122d6001
~ order-1
  - "pending"
  + "shipped"
~ status
  - "ok"
  + "degraded"
```

Just the shape of the change, no content:

```
$ mnem diff 9e451eef0c04 d765122d6001 --stat
~ order-1
~ status
```

## See also

- [`changelog`](changelog.md) - the same comparison, narrated in
  plain English instead of a structured diff.
- [`show`](show.md) - one point in full, instead of the difference
  between two.
