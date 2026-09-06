# Issue tracker: GitHub

Issues and specs for this repo live as GitHub issues. Use the `gh` CLI for all
operations. `gh` infers the repo from `git remote -v`.

## Conventions

- **Create**: `gh issue create --title "..." --body "..."`, heredoc for multi-line bodies.
- **Read**: `gh issue view <n> --comments`.
- **List**: `gh issue list --state open --label phase-1 --json number,title,labels`.
- **Comment**: `gh issue comment <n> --body "..."`.
- **Label**: `gh issue edit <n> --add-label "..."` / `--remove-label "..."`.
- **Close**: `gh issue close <n> --comment "..."`.

## Labels

- `phase-0` to `phase-7`: which phase the work belongs to. Every issue carries one.
- `wayfinder:map`: the single map issue for a phase.
- `wayfinder:research`: an autonomous investigation. Output is an ADR recommendation.
- `wayfinder:prototype`: a throwaway spike that needs a human to judge the result.
- `wayfinder:grilling`: pressure-test a decision with the maintainer before it is locked.
- `wayfinder:task`: build work.
- `ready-for-agent`: the spec is settled; an implementation agent may build it.
- `track-a` / `track-b`: core and format, versus SDK, adapters and developer experience. Not used while there is one maintainer, but kept current.
- `adr`: the issue needs or updates an ADR.
- `format`: the issue touches the on-disk format spec.
- `benchmark`: the issue concerns the v1 benchmark.

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
