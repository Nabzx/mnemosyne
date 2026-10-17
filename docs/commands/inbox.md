# `mnem inbox`

List every branch not yet merged into `main`, with its tip commit.

## Synopsis

```
mnem inbox
```

## Description

A read-only view over branches that already exist: every branch whose
tip is not an ancestor of `main`, with that tip's commit id, author
and message. No new object model behind this - it is `branch` plus
`log` plus an ancestry check, read together. Nothing here is a real
pending-proposal object (that is the ADR-gated review model, #419-422,
still unsettled); `inbox` only answers "what is unmerged right now".

## Options

`mnem inbox` takes no options beyond `-h`/`--help`.

## Examples

```
$ mnem inbox
feature-a  4e214d3b688c  bob  sketch a faster retry backoff
```

With nothing unmerged:

```
$ mnem inbox
Nothing pending - every branch is merged into main.
```

## See also

- [`branch`](branch.md) - list every branch, merged or not.
- [`merge`](merge.md), [`approve`](approve.md) - land one of `inbox`'s
  own listed branches.
