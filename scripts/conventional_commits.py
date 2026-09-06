#!/usr/bin/env python3
"""Check that commits follow Conventional Commits (ADR-0011).

Every non-merge commit subject, and the pull request title, must read
``<type>(<scope>): <description>`` with a lowercase type, a lowercase imperative
description and no trailing full stop. GitHub closing keywords are rejected in
favour of ``refs #<n>``.

Usage::

    conventional_commits.py --range BASE..HEAD
    conventional_commits.py --pr                 # CI: PR base range plus PR title
    conventional_commits.py --message "feat: do the thing"
"""

from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys

TYPES = (
    "feat",
    "fix",
    "docs",
    "test",
    "refactor",
    "perf",
    "build",
    "ci",
    "chore",
    "revert",
)

SUBJECT_RE = re.compile(
    r"^(?P<type>" + "|".join(TYPES) + r")"
    r"(?P<scope>\([a-z0-9][a-z0-9-]*\))?"
    r"(?P<breaking>!)?"
    r": (?P<description>[a-z].*)$"
)

CLOSING_RE = re.compile(
    r"\b(clos(?:e|es|ed)|fix(?:e[sd])?|resolv(?:e|es|ed))\s+#\d+\b",
    re.IGNORECASE,
)

MAX_SUBJECT = 72


def check_subject(subject: str) -> list[str]:
    """Problems with a single subject line, empty if it is fine."""
    if not SUBJECT_RE.match(subject):
        return [
            (
                "subject must be '<type>(<scope>): <description>' with a lowercase "
                f"type from {', '.join(TYPES)} and a lowercase description"
            )
        ]
    problems = []
    if subject.endswith("."):
        problems.append("subject ends with a full stop")
    if len(subject) > MAX_SUBJECT:
        problems.append(f"subject is {len(subject)} characters, over {MAX_SUBJECT}")
    return problems


def check_body(body: str) -> list[str]:
    if CLOSING_RE.search(body):
        return ["uses a GitHub closing keyword; use 'refs #<n>' instead (ADR-0011)"]
    return []


def check_message(message: str) -> list[str]:
    lines = message.strip().splitlines()
    if not lines:
        return ["empty message"]
    return check_subject(lines[0].strip()) + check_body("\n".join(lines[1:]))


def commits_in_range(rev_range: str) -> list[tuple[str, str]]:
    """(short sha, full message) for each non-merge commit in ``rev_range``."""
    out = subprocess.run(
        ["git", "log", "--no-merges", "-z", "--format=%H%n%B", rev_range],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    commits = []
    for block in out.split("\0"):
        block = block.strip("\n")
        if not block:
            continue
        sha, _, message = block.partition("\n")
        commits.append((sha[:12], message))
    return commits


def _report(label: str, problems: list[str]) -> bool:
    if not problems:
        return True
    print(f"  {label}")
    for problem in problems:
        print(f"    - {problem}")
    return False


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--range", help="a git rev range, e.g. origin/main..HEAD")
    group.add_argument("--pr", action="store_true", help="use $PR_BASE_SHA..HEAD and $PR_TITLE")
    group.add_argument("--message", help="check a single message")
    args = parser.parse_args(argv)

    if args.message is not None:
        problems = check_message(args.message)
        _report(f'"{args.message.splitlines()[0]}"', problems)
        return 1 if problems else 0

    rev_range = args.range
    title = None
    if args.pr:
        base = os.environ.get("PR_BASE_SHA")
        if not base:
            print("--pr needs $PR_BASE_SHA", file=sys.stderr)
            return 2
        rev_range = f"{base}..HEAD"
        title = os.environ.get("PR_TITLE")

    ok = True
    for sha, message in commits_in_range(rev_range):
        subject = message.splitlines()[0] if message.strip() else ""
        ok &= _report(f"{sha}  {subject}", check_message(message))

    if title is not None:
        ok &= _report(f'PR title  "{title}"', check_subject(title.strip()))

    if ok:
        print("conventional commits: ok")
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
