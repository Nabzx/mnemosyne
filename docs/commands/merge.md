# `mnem merge`

Merge another branch or commit into the current branch.

## Synopsis

```
mnem merge [OPTIONS] <THEIRS>
```

## Description

A deterministic three-way merge of `THEIRS` into the current branch,
per node id. A clean merge (no two sides changed the same node
incompatibly) lands as a normal commit, or fast-forwards if the
current branch has no new commits of its own. A real conflict - the
same node edited two different ways since the common ancestor - is
surfaced as a typed conflict object, not silently decided for you:
nothing is written until every conflict has an explicit answer, via
`--resolve <id>=<side>` (repeatable, one conflict at a time) or
`--strategy ours|theirs` (every conflict the same way).

`--dry-run` previews the outcome - clean, fast-forward, or which
nodes would conflict - without writing anything; it does not combine
with `--resolve` or `--strategy`, since previewing and resolving are
different questions. `--dry-run --html <file>` additionally renders
any conflicts as a static two-column HTML page, base/ours/theirs side
by side, for reading outside a terminal.

## Options

| Flag | Description |
| --- | --- |
| `<THEIRS>` | A branch name or commit id to merge in. |
| `--resolve <ID=SIDE>` | Resolve one conflict: `<id>=<ours\|theirs\|base\|delete>`. Repeatable. |
| `--strategy <SIDE>` | Resolve every conflict one way: `ours` or `theirs`. |
| `-m, --message <MESSAGE>` | Override the merge commit message. |
| `--author <AUTHOR>` | Who or what made the merge. Falls back to `$MNEM_AUTHOR`, then `"unknown"`. |
| `--dry-run` | Show what merging would do, without writing anything. |
| `--html <FILE>` | With `--dry-run`: if there are conflicts, also render them as an HTML page. |

## Examples

Previewing a real conflict before touching anything:

```
$ mnem merge conflict-a --dry-run
(dry run) would conflict in 1 node(s):
  order-10 (edit/edit)
    base:   "shipped"
    ours:   "refunded"
    theirs: "returned"
Error: resolve with `--resolve <id>=<side>` (or `--strategy ours|theirs`) and merge for real
```

The same conflict, resolved one node at a time:

```
$ mnem merge escalation --resolve case-4821=theirs --author alice
[44b432580778] merge escalation into resolution
```

## See also

- [`approve`](approve.md) - the same merge, with a named reviewer
  recorded as its author.
- [`inbox`](inbox.md) - branches waiting to be merged.
- [`diff`](diff.md) - compare two branches without merging either.
