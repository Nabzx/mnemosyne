# `mnem export`

Dump the whole store (every commit, ref and HEAD) as one JSON value.

## Synopsis

```
mnem export [OPTIONS]
```

## Description

Dumps the whole content-addressed object graph as one portable JSON
value - every commit, ref, and HEAD - not a flattened snapshot.
[`import`](import.md) is the exact reverse. Because objects are
content-addressed (ADR-0005), a node that survives the round trip
resolves to the **same commit id** it was written under originally -
`blame` on the imported copy still names the real original commit,
not a new one invented for the copy. With no `--output`, writes to
stdout.

## Options

| Flag | Description |
| --- | --- |
| `-o, --output <FILE>` | Write here instead of stdout. |

## Examples

```
$ mnem export -o bundle.json
Exported 4 object(s) to bundle.json
```

## See also

- [`import`](import.md) - the reverse: create a new store from an
  export.
- [`registry`](registry.md) - `export`/`import` wrapped under a
  name, in a shared folder.
