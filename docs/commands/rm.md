# `mnem rm`

Stage the deletion of a node from the next commit.

## Synopsis

```
mnem rm <ID>
```

## Description

Stages the removal of `ID` from working memory, to take effect on the
next [`commit`](commit.md). The node's own history is never touched
by this - every earlier commit that wrote it is still reachable, still
`blame`-able, and still shows up when [`show`](show.md) is pointed at
an older commit. `rm` only changes what working memory looks like
going forward.

## Options

| Argument | Description |
| --- | --- |
| `<ID>` | The node id to remove. |

## Examples

```
$ mnem rm customer-4821
$ mnem commit -m "customer-4821 is no longer relevant"
```

The removed fact's history survives at the commit that still has it -
`mnem log` names the commit before the removal:

```
$ mnem show 09a2c3b340af
customer-4821
  "wants a refund for the delayed order"
```

## See also

- [`add`](add.md) - the reverse.
- [`commit`](commit.md) - records what `rm` staged.
- [`blame`](blame.md) - still resolves a removed node's earlier
  history.
