# Research: a command-reference template and category scheme (issue #549)

Feeds the decide ticket (#550), which locks the per-page template, the
category scheme, and the generated-versus-hand-written question before
any reference page (#551-556) or guide (#557-559) gets written.

Three questions:

1. What does git-scm.com/docs' own per-page template and category
   scheme actually look like?
2. What is mnem's own real command set, and how does a scaled-down
   version of git's categories fit it?
3. Hand-written pages or pages generated from each subcommand's own
   `--help` text: what is the real cost of each?

## 1. Git's real template and category scheme

Checked directly against git-scm.com/docs and a real command page
(`git-commit`), not from memory.

**The category scheme.** Commands are grouped by what a user is doing,
not alphabetically: Setup and Config, Getting and Creating Projects,
Basic Snapshotting, Branching and Merging, Sharing and Updating
Projects, Inspection and Comparison, Patching, Debugging, Email,
External Systems, Administration, Server Admin, and a large Plumbing
Commands category (50-plus low-level commands) kept separate from the
porcelain a user actually types. Alongside the command pages sits a
separate, non-command Guides layer: `gitattributes`, `gitcli`,
`giteveryday`, `gitfaq`, `gitglossary`, `githooks`, `gitignore`,
`gitmodules`, `gitrevisions`, `gitsubmodules`, `gittutorial`,
`gitworkflows` - conceptual documents, not tied to one command.

**The per-page template.** A real command page (`git-commit`) follows
a fixed order: NAME (a one-line description), SYNOPSIS (formal usage
syntax), DESCRIPTION (prose), OPTIONS (every flag), EXAMPLES (real
usage), then command-specific sections where the command needs them
(`git-commit` has its own COMMIT INFORMATION and DATE FORMATS
sections), then ENVIRONMENT AND CONFIGURATION VARIABLES, HOOKS, FILES,
SEE ALSO, and a standardised GIT footer naming the suite version. Not
every section appears on every page - HOOKS and FILES are specific to
commands that touch them - but the order is fixed, and SEE ALSO always
closes the page before the footer.

**Navigation.** A quick-reference cheat sheet, a complete alphabetical
command list, and a version switcher sit above the category browsing;
the Guides are reachable separately from the main command list, not
mixed into it.

## 2. mnem's real command set against a scaled-down scheme

mnem has twenty-six real subcommands today (every one's doc comment
read directly from `crates/mnem-git/src/main.rs`, not from memory of
building them): `init`, `add`, `rm`, `commit`, `log`, `status`,
`inbox`, `branch`, `checkout`, `show`, `blame`, `bisect`, `merge`,
`approve`, `reject`, `diff`, `changelog`, `similar`, `contradicts`,
`completions`, `export`, `import`, and `registry`'s own four
(`publish`, `list`, `fork`, `render`). Git's own category list assumes
a command set an order of magnitude larger - Email and Server Admin
alone have no mnem equivalent at all, and Plumbing-versus-porcelain is
not a real distinction here (every mnem command is a "porcelain"
command; there is no internal plumbing layer exposed to a user).

A scaled-down scheme that actually fits twenty-six commands, grouping
by what a user is doing the way git's own scheme does rather than by
implementation detail:

| Category | Commands |
| --- | --- |
| Getting started | `init`, `add`, `rm`, `commit`, `status` |
| History and inspection | `log`, `show`, `diff`, `blame`, `bisect`, `changelog`, `inbox` |
| Branching and merging | `branch`, `checkout`, `merge`, `approve`, `reject` |
| Portability | `export`, `import`, `registry` (and its own four subcommands) |
| Suggestion heuristics | `similar`, `contradicts` |

That fifth category - "suggestion heuristics" - has no real git
equivalent: `similar` and `contradicts` are both explicit, crude,
never-auto-resolving market-test spikes (#443, #449), a different kind
of command from everything else mnem ships, and worth keeping visibly
separate rather than folded into Inspection just because they both read
working memory.

`completions` does not fit any of the five categories above - it is
about the shell, not about memory - and fits git's own Guides layer
(it mirrors `gitcli`'s own territory: real cross-cutting CLI mechanics)
better than a sixth, one-command reference category.

Git's own Guides layer has a real, mostly-already-written mnem
equivalent: `gitglossary` maps onto `CONTEXT.md`'s own glossary
(already written, just not surfaced in the docs site), `gitworkflows`
maps onto the branch-per-hypothesis and review-gate patterns several
real examples (`cross_framework_merge.py`, `fork_via_export_import.py`,
`multi_agent_incident_retro.py`) already demonstrate but never collect
in one place, and `gitcli` maps onto the cross-cutting conventions -
`$MNEM_AUTHOR`/`$MNEM_REGISTRY` fallbacks, `NO_COLOR`, exit codes,
`--content-json` - that cut across every command rather than belonging
to one.

## 3. Hand-written pages or generated from `--help`

**Generated from `--help`.** Every subcommand's clap-derived `--help`
text is real, accurate by construction (it comes straight from the
same doc comments and `#[arg(...)]` attributes that drive the parser),
and can never drift out of sync with the CLI - a page regenerated on
every release is correct by definition. Checked directly against
several real `--help` outputs already produced while building `#447`,
`#448`, `#451` and `#453`: they give NAME, a compressed SYNOPSIS, and
OPTIONS for free, but never an EXAMPLES section - clap has no concept
of a worked example, and the one-line doc comments are deliberately
terse (the house style note in `main.rs`'s own module doc: "this crate
holds no operation semantics of its own, only argument parsing").

**Hand-written.** git's own pages earn their value almost entirely in
the sections `--help` text cannot produce at all: EXAMPLES, DISCUSSION,
SEE ALSO - cross-references and real worked scenarios, not restated
flag syntax. A hand-written mnem page could lift a real worked example
straight from an existing example script (`fork_via_export_import.py`
for `export`/`import`, the `cross_framework_merge.py` conflict for
`merge`) rather than inventing one, keeping the examples honest and
tied to something already tested.

**The real cost of each.** Generated pages risk nothing but add
nothing beyond what a terminal already shows; hand-written pages add
real value but drift the moment a flag changes and nobody updates the
page - a real risk this project has already hit once elsewhere
(`ROADMAP.md`'s own stale backlog line, #296). The two are not
mutually exclusive: NAME, SYNOPSIS and OPTIONS are exactly the part
`--help` already gets right and never drifts on, while EXAMPLES,
DISCUSSION and SEE ALSO are exactly the part worth writing by hand and
worth a CI check that each page's SYNOPSIS/OPTIONS still matches the
real `--help` output, so hand-written prose never silently rots next
to a changed flag.

## Recommendation

- **Template**: NAME and SYNOPSIS generated from the real `--help`
  output (verified by a CI check against the real binary, not just
  pasted in once); DESCRIPTION, EXAMPLES, and SEE ALSO hand-written,
  lifting real examples from existing example scripts where one
  already exists; OPTIONS listed from `--help` directly, one line of
  hand-written context added only where a flag's own one-liner needs
  it. No HOOKS or FILES section - mnem has no hook concept, and `FILES`
  collapses into a single pointer at `docs/format/` rather than
  repeating the on-disk layout on every page.
- **Categories**: the five in the table above, plus `completions`
  folded into a CLI-conventions guide rather than its own page.
- **Guides**: a glossary (linking `CONTEXT.md` directly, never
  redefining a term), a workflows guide (collecting patterns the real
  examples already prove), and a CLI-conventions guide (env vars,
  `NO_COLOR`, exit codes, `completions`).
- **Drift check**: whichever build ticket writes the first reference
  page should also add a small script (or extend `scripts/
  check_changelog.py`'s own sibling scripts) asserting each page's
  SYNOPSIS/OPTIONS block matches the real `--help` output in CI - not
  scoped here, but named so the decide ticket can place it.
