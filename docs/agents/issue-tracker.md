# Issue tracker: GitHub

Issues and specs for this repo live as GitHub issues. Use the `gh` CLI for all
operations. `gh` infers the repo from `git remote -v`.

## Conventions

- **Create**: `gh issue create --title "..." --body "..."`, heredoc for multi-line bodies. Issue titles are a plain description of the bug or job, not a conventional-commit line (that is for the PR).
- **Read**: `gh issue view <n> --comments`.
- **List**: `gh issue list --state open --label phase-1 --json number,title,labels`.
- **Comment**: `gh issue comment <n> --body "..."`.
- **Label**: `gh issue edit <n> --add-label "..."` / `--remove-label "..."`.
- **Close**: `gh issue close <n> --comment "..."`. A build issue is not closed on merge; it carries `pending-release` and is closed in a batch when its release is tagged (ADR-0011). Bugs and docs issues may close on merge.

## Issues are for bugs and maintainer work items

The issue tracker holds reproducible bugs (the `bug.yml` form) and
maintainer-created work items (wayfinder tickets, the `wayfinder.yml` form, and
the hardening epics). Feature requests, ideas, direction and questions go to
[Discussions](https://github.com/Nabzx/mnemosyne/discussions); blank issues are
disabled.

## Labels

**Phase and track**

- `phase-0` to `phase-7`: which phase the work belongs to. Every issue carries one.
- `track-a` / `track-b`: core and format, versus SDK, adapters and developer experience. Not used while there is one maintainer, but kept current.

**Wayfinder**

- `wayfinder:map`: the single map issue for a phase.
- `wayfinder:research`: an autonomous investigation. Output is an ADR recommendation.
- `wayfinder:prototype`: a throwaway spike that needs a human to judge the result.
- `wayfinder:grilling`: pressure-test a decision with the maintainer before it is locked.
- `wayfinder:task`: build work.
- `ready-for-agent`: the spec is settled; an implementation agent may build it.

**Priority** (bugs carry one)

- `p1`: serious. A core workflow is broken or a released guarantee is violated.
- `p2`: an ordinary defect with a workaround or a limited blast radius.
- `p3`: minor. Cosmetic, or a nice-to-have with no urgency.

**Triage state**

- `needs-repro`: plausible but not yet reproduced. On the `bug.yml` form by default.
- `needs-decision`: blocked on a design or architecture decision (an ADR or a grilling).
- `needs-research`: blocked on a `wayfinder:research` investigation.
- `regression`: worked in an earlier tagged release, broken now.
- `pending-release`: merged to `main`, closes when the release that contains it is tagged (ADR-0011).

**Area**

- `adr`: the issue needs or updates an ADR.
- `format`: the issue touches the on-disk format spec.
- `benchmark`: the issue concerns the substrate benchmark.
- `documentation`: docs only.

## Wayfinder

The **map** is one issue labelled `wayfinder:map`, holding three sections in its
body: Notes, Decisions so far, and Fog (what is still unknown).

**Child tickets** are GitHub sub-issues of the map, each labelled
`wayfinder:<type>` and `phase-N`.

**Blocking** uses GitHub's native issue dependencies. Add an edge with
`gh api --method POST repos/Nabzx/mnemosyne/issues/<child>/dependencies/blocked_by -F issue_id=<blocker-db-id>`,
where the blocker db id comes from `gh api repos/Nabzx/mnemosyne/issues/<n> --jq .id`.

**Frontier**: the map's open child tickets with no open blocker. Work them in
map order.

**Resolve** a research or grilling ticket: comment the answer, close it, then
append a one-line pointer to the map's Decisions so far, linking the resulting
ADR.
