# `mnem import`

Create a new store from an export (the reverse of `export`).

## Synopsis

```
mnem import <FILE> [PATH]
```

## Description

Reads an [`export`](export.md)ed JSON file and creates a fresh store
from it at `PATH` (the current directory if omitted), refusing if a
store already exists there. The round trip is lossless at the object
level, not just the content level: a node imported this way still
`blame`s to its real original commit id, because content-addressed
objects hash identically regardless of which store file holds them.

## Options

| Argument | Description |
| --- | --- |
| `<FILE>` | The exported JSON file. |
| `[PATH]` | Where to create the store. Refuses if one already exists there. |

## Examples

```
$ mnem import bundle.json ./clone
Imported 4 object(s) into clone/.mnem
```

## See also

- [`export`](export.md) - the reverse: dump a store to JSON.
- [`registry`](registry.md) - `fork` is `import` under a name, read
  from a shared folder instead of an arbitrary path.
