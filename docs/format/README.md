# On-disk format

The layout of a `.mnem/` store: the object database, the refs, and the
provenance index.

This is empty until Phase 1. It is written alongside the Phase 1 build, marked
frozen for the v0.x line at the end of Phase 1, and finalised for v1 in Phase 6.
See ADR-0002.

Once frozen for a version line, a store written by one release in that line is
readable by every other release in it. A breaking change to the format needs a
major version bump and a migration path (see `AGENTS.md`).
