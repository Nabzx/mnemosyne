# `mnem checkout`

Move HEAD to a branch or commit.

## Synopsis

```
mnem checkout [OPTIONS] [REV]
```

## Description

Switches HEAD to `REV` - a branch name or commit id - and
materialises working memory to match. Refuses if there is anything
staged and uncommitted, unless `--discard` says to drop it. `-b
<NAME>` creates a new branch and switches to it in one step, with
`REV` (or HEAD, if omitted) as that branch's start point - the
quickest way to fork a hypothesis.

## Options

| Argument / Flag | Description |
| --- | --- |
| `[REV]` | A branch or commit to switch to. With `-b`, the new branch's start point. |
| `-b, --create <NAME>` | Create a branch and switch to it. |
| `--discard` | Drop uncommitted staged changes instead of refusing. |

## Examples

```
$ mnem checkout escalation
Switched to branch escalation
```

Fork and switch in one step:

```
$ mnem checkout -b feature-b
Switched to branch feature-b
```

## See also

- [`branch`](branch.md) - create a branch without switching to it.
- [`merge`](merge.md) - bring a branch's own commits back into the
  one you switched away from.
