# Domain docs

How an engineering agent should consume this repo's domain documentation.

## Before working in an area, read these

- [`CONTEXT.md`](../../CONTEXT.md) at the repo root: the glossary.
- [`docs/adr/`](../adr): the ADRs that touch the area you are about to change.
- [`docs/format/`](../format): the on-disk format spec, once it exists.

## Use the glossary's vocabulary

When your output names a domain concept, in an issue title, an ADR, a
hypothesis, a test name, use the term as defined in `CONTEXT.md`. Do not drift
to a synonym the glossary lists under _Avoid_.

If the concept you need is not in the glossary, that is a signal. Either you are
inventing language the project does not use, in which case reconsider, or there
is a real gap, in which case note it.

## Flag ADR conflicts

If your change would contradict an Accepted ADR, say so openly rather than
quietly working around it:

> Contradicts ADR-0002 (on-disk object format), but worth reopening because ...

An Accepted ADR is changed only by a new ADR that supersedes it.
