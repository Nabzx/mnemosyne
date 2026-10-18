# `mnem changelog`

Narrate what changed between two memory states in plain English.

## Synopsis

```
mnem changelog [FROM] [TO]
```

## Description

The same comparison [`diff`](diff.md) computes, reworded as one
sentence per changed node instead of a structured `+`/`-`/`~` dump -
"customer-4821 changed from X to Y" rather than a diff block. `diff`'s
own structured output is unchanged by this existing; `changelog` is a
rephrasing, not a second computation. Same `FROM`/`TO` defaults as
`diff`: no arguments compares HEAD against working memory.

## Options

| Argument | Description |
| --- | --- |
| `[FROM]` | The `from` side: a branch or commit. Default: the HEAD commit. |
| `[TO]` | The `to` side: a branch or commit. Default: working memory. |

## Examples

```
$ mnem changelog 9e451eef0c04 d765122d6001
order-1 changed from "pending" to "shipped"
status changed from "ok" to "degraded"
```

## See also

- [`diff`](diff.md) - the same comparison, as a structured dump
  rather than plain English.
