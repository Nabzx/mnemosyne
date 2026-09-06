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


def next_number(have: set[str]) -> str:
    """The next ADR in sequence. A research ticket names the ADR it feeds
    before that ADR is written, so one forward reference is allowed."""
    highest = max((int(n) for n in have), default=0)
    return f"{highest + 1:04d}"


def check() -> list[str]:
    have = existing_numbers(ADR_DIR)
    allowed = have | {"0000", next_number(have)}
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
