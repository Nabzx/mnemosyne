# `mnem init`

Create a new memory store in a directory (default: the current one).

## Synopsis

```
mnem init [PATH]
```

## Description

Creates a `.mnem/` directory at `PATH` (or the current directory if
`PATH` is omitted) holding a fresh, empty store: an object database, no
commits, `HEAD` pointing at an unborn `main` branch. Refuses if a store
already exists there. Nothing else in mnem works until a store exists
- every other command looks for `.mnem/` starting at the current
directory.

## Options

| Argument | Description |
| --- | --- |
| `[PATH]` | Where to create the store. Omit to use the current directory. |

## Examples

Create a store in the current directory:

```
$ mnem init
Initialised an empty Mnemosyne store in ./.mnem
```

Create one somewhere specific:

```
$ mnem init ./my-agent
Initialised an empty Mnemosyne store in my-agent/.mnem
```

## See also

- [`add`](add.md), [`commit`](commit.md) - the next two steps after
  `init`.
- [On-disk format](../format/README.md) - what `.mnem/` actually
  contains.
