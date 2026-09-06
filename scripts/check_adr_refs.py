#!/usr/bin/env python3
"""Check that every ADR referenced in the tree exists (ADR-0001).

Scans tracked ``.md`` and ``.rs`` files for ``ADR-NNNN`` and fails if any
number has no matching file in ``docs/adr/``.
"""

from __future__ import annotations

import pathlib
import re
import subprocess
import sys

ADR_DIR = pathlib.Path("docs/adr")
REF_RE = re.compile(r"ADR-(\d{4})")
FILE_RE = re.compile(r"^(\d{4})-")


def existing_numbers(adr_dir: pathlib.Path) -> set[str]:
    return {m.group(1) for p in adr_dir.glob("*.md") if (m := FILE_RE.match(p.name))}


def tracked_files() -> list[pathlib.Path]:
    out = subprocess.run(
        ["git", "ls-files", "*.md", "*.rs"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    return [pathlib.Path(line) for line in out.splitlines() if line]


def referenced_numbers(paths: list[pathlib.Path]) -> dict[str, set[str]]:
    """number -> set of files that mention it."""
    out: dict[str, set[str]] = {}
    for path in paths:
        try:
            text = path.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        for number in REF_RE.findall(text):
            out.setdefault(number, set()).add(str(path))
    return out


def upcoming_numbers(have: set[str], count: int = 2) -> set[str]:
    """The next few ADR numbers. A research ticket names the ADRs it feeds
    before they are written, and a ticket can split into two (an algorithm ADR
    and an API ADR), so a short forward window is allowed."""
    highest = max((int(n) for n in have), default=0)
    return {f"{highest + i:04d}" for i in range(1, count + 1)}


def check() -> list[str]:
    have = existing_numbers(ADR_DIR)
    allowed = have | {"0000"} | upcoming_numbers(have)
    refs = referenced_numbers(tracked_files())
    problems = []
    for number, where in sorted(refs.items()):
        if number not in allowed:
            sites = ", ".join(sorted(where)[:3])
            problems.append(f"ADR-{number} is referenced ({sites}) but no file exists")
    return problems


def main() -> int:
    problems = check()
    for problem in problems:
        print(f"  - {problem}")
    if problems:
        print("dangling ADR references")
        return 1
    print("ADR references: ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
