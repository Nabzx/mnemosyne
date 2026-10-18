# `mnem bisect`

Find the first commit where a memory node reaches a given state.

## Synopsis

```
mnem bisect [OPTIONS] --node <NODE> [BAD]
```

## Description

Binary-searches the commit history between a known-good and a known-
bad commit to find the exact commit where `--node` first matches the
given predicate - `--equals <JSON>` for an exact content match,
`--present` for the node existing at all, `--absent` for it not
existing. No reading commits by hand one at a time: `bisect` does the
search. `BAD` is a positional argument, not a flag, and defaults to
HEAD if omitted; `--good` defaults to the history root if omitted.

## Options

| Argument / Flag | Description |
| --- | --- |
| `[BAD]` | A commit where the predicate is true (default: HEAD). |
| `--node <NODE>` | The node id the predicate is about. |
| `--equals <JSON>` | True when the node's content equals this JSON value. |
| `--absent` | True when the node is not in memory. |
| `--present` | True when the node is in memory. |
| `--good <GOOD>` | A commit where the predicate is false (default: the history root). |

## Examples

```
$ mnem bisect --node status --equals '"degraded"' --good 9e451eef0c04 b7a942a11824
d765122d6001  status degraded after a tool error
  status set here, no provenance recorded
```

`BAD` is positional - a common mistake is passing it as `--bad`, which
clap rejects outright rather than silently misreading it:

```
$ mnem bisect --node status --equals '"degraded"' --good 9e451eef0c04 --bad b7a942a11824
error: unexpected argument '--bad' found
```

## See also

- [`blame`](blame.md) - resolve a node to its introducing commit once
  you already know which one; `bisect` is for when you do not.
- [`log`](log.md) - the history `bisect` searches.
