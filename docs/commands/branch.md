# `mnem branch`

List, create, or delete branches.

## Synopsis

```
mnem branch [OPTIONS] [NAME] [START]
```

## Description

With no arguments, lists every branch, its tip commit, and that
commit's message, marking the current branch with a `*`. Given
`NAME`, creates a new branch there instead of listing - at `START` (a
branch name or commit id) if given, at HEAD otherwise. Creating a
branch is cheap: a new ref pointing at an existing commit, nothing
copied. `--delete <NAME>` removes a branch; it refuses to delete the
one currently checked out.

## Options

| Argument / Flag | Description |
| --- | --- |
| `[NAME]` | The branch to create. Omit to list branches. |
| `[START]` | Where the new branch starts: a branch name or commit id (default HEAD). |
| `-d, --delete <NAME>` | Delete this branch. |

## Examples

```
$ mnem branch
  escalation  fb8e2144d9f5  escalate ticket
  main  1d62639c0f62  seed
* resolution  ee0e860e1aa9  resolve ticket
```

Fork a hypothesis at the current commit, and switch to it in one step
with [`checkout -b`](checkout.md):

```
$ mnem checkout -b feature-b
Switched to branch feature-b
```

## See also

- [`checkout`](checkout.md) - switch to a branch, or create and
  switch in one step.
- [`status --all-branches`](status.md) - every branch with its tip's
  author and age, instead of just its message.
- [`inbox`](inbox.md) - which of these branches is unmerged.
