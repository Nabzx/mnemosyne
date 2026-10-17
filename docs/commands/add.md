# `mnem add`

Stage a memory node for the next commit.

## Synopsis

```
mnem add [OPTIONS] <ID> [CONTENT]
```

## Description

Stages a memory node under `ID` with the given `CONTENT`, to be
recorded on the next [`commit`](commit.md). Staging the same `ID`
again replaces the staged content - it is an update, not a second
node. `CONTENT` is stored as a plain string by default; `--content-
json` parses it as a JSON value instead (a number, bool, object,
array, or `null`), and `--content-file` reads JSON content from a
file rather than the command line. Omitting `CONTENT` entirely reads
it from stdin as a plain string.

The three provenance flags (`--step`, `--source`, `--event-time`)
record how the node came to exist - which step of an agent's run
produced it, an external source identifier, and when the agent formed
it - queryable later through [`blame`](blame.md).

## Options

| Flag | Description |
| --- | --- |
| `<ID>` | The node's stable id. Staging the same id again is an update. |
| `[CONTENT]` | The content, as a plain string. Omit to read it from stdin. |
| `--content-file <FILE>` | Read the content from this file, parsed as JSON. |
| `--content-json <JSON>` | The content, parsed as JSON instead of stored as a plain string. |
| `--step <STEP>` | Provenance: which step of the run produced this. |
| `--source <SOURCE>` | Provenance: an external source identifier. |
| `--event-time <MS>` | When the agent formed this, in Unix milliseconds. |

## Examples

A plain string fact:

```
$ mnem add customer-4821 "wants a refund for the delayed order"
Staged customer-4821 as a1b2c3d4e5f6
```

Structured content, with provenance:

```
$ mnem add agent:policy --content-json '{"temperature": 0.2}' \
    --step "configure" --source "operator"
```

## See also

- [`rm`](rm.md) - the reverse: stage a deletion instead of a write.
- [`commit`](commit.md) - records what `add` staged.
- [`blame`](blame.md) - reads the provenance `add` recorded.
