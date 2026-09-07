#!/usr/bin/env python3
"""Check that a user-facing change updates the changelog (ADR-0007).

If a pull request changes crate source, the Python package, or the format spec,
``CHANGELOG.md`` must be in the same diff. Docs, tests, CI and tooling changes
are exempt.

Usage::

    check_changelog.py --pr            # CI: diff $PR_BASE_SHA..HEAD
    check_changelog.py --range A..B
"""

from __future__ import annotations

import argparse
import os
import subprocess
import sys

USER_FACING_PREFIXES = (
    "crates/mnem-core/src/",
    "crates/mnem-cli/src/",
    "crates/mnem-py/src/",
    "python/mnem/",
    "packages/",
    "docs/format/",
)
EXEMPT_SUFFIXES = ("/tests/", "test_")


def is_user_facing(path: str) -> bool:
    if path == "README.md":
        return True
    if any(seg in path for seg in EXEMPT_SUFFIXES):
        return False
    return path.startswith(USER_FACING_PREFIXES)


def changed_files(rev_range: str) -> list[str]:
    out = subprocess.run(
        ["git", "diff", "--name-only", rev_range],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    return [line for line in out.splitlines() if line]


def check(files: list[str]) -> list[str]:
    facing = sorted(p for p in files if is_user_facing(p))
    if facing and "CHANGELOG.md" not in files:
        return [
            "user-facing files changed but CHANGELOG.md did not: "
            + ", ".join(facing[:5])
            + (" ..." if len(facing) > 5 else "")
        ]
    return []


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--pr", action="store_true")
    group.add_argument("--range")
    args = parser.parse_args(argv)

    if args.pr:
        base = os.environ.get("PR_BASE_SHA")
        if not base:
            print("--pr needs $PR_BASE_SHA", file=sys.stderr)
            return 2
        rev_range = f"{base}..HEAD"
    else:
        rev_range = args.range

    problems = check(changed_files(rev_range))
    for problem in problems:
        print(f"  - {problem}")
    if problems:
        print("changelog not updated")
        return 1
    print("changelog: ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
