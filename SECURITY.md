# Security policy

## Reporting a vulnerability

Do not open a public issue for a security problem.

Use GitHub's private vulnerability reporting on this repository
(Security tab, "Report a vulnerability"), or email the maintainer at
nabilshahx@gmail.com with "mnemosyne security" in the subject.

You should get an acknowledgement within a few days. Once a fix is ready, a
patched release is cut and the advisory is published.

## Supported versions

Mnemosyne is pre-1.0. Only the latest tagged release is supported. There are no
backports.

Once `1.0.0` ships, the latest `1.x` release is supported, and the on-disk
format guarantees in `AGENTS.md` apply.

## Scope

In scope: anything that lets an untrusted store, commit, or memory node cause
`mnem` to execute code, escape the store directory, corrupt an unrelated store,
or misreport `verify` or `fsck`.

Out of scope for now: denial of service from a deliberately huge store, and the
Era 2 semantic layer (embeddings, semantic merge), which does not exist yet.
