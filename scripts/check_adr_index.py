#!/usr/bin/env python3
"""Check the ADR index is current (ADR-0001).

Every ``docs/adr/NNNN-*.md`` file (bar the template) must have a row in
``docs/adr/README.md``, and every row must link to a file that exists.
"""

from __future__ import annotations

import pathlib
import re
import sys

ADR_DIR = pathlib.Path("docs/adr")
FILE_RE = re.compile(r"^(\d{4})-[a-z0-9-]+\.md$")
ROW_RE = re.compile(r"^\|\s*\[(\d{4})\]\(([^)]+)\)\s*\|")


def files_by_number(adr_dir: pathlib.Path) -> dict[str, str]:
    out = {}
    for path in sorted(adr_dir.glob("*.md")):
        m = FILE_RE.match(path.name)
        if m and m.group(1) != "0000":
            out[m.group(1)] = path.name
    return out


def index_rows(index_text: str) -> dict[str, str]:
    out = {}
    for line in index_text.splitlines():
        m = ROW_RE.match(line.strip())
        if m:
            out[m.group(1)] = m.group(2)
    return out


def check(adr_dir: pathlib.Path) -> list[str]:
    index_path = adr_dir / "README.md"
    if not index_path.exists():
        return [f"{index_path} is missing"]
    files = files_by_number(adr_dir)
    rows = index_rows(index_path.read_text())
    problems = []
    for number, name in files.items():
        if number not in rows:
            problems.append(f"ADR {number} ({name}) has no row in {index_path}")
        elif rows[number] != name:
            problems.append(
                f"ADR {number} row links to '{rows[number]}', file is '{name}'"
            )
    for number, link in rows.items():
        if number not in files:
            problems.append(f"index row for ADR {number} links to '{link}', which does not exist")
    return problems


def main() -> int:
    problems = check(ADR_DIR)
    for problem in problems:
        print(f"  - {problem}")
    if problems:
        print("ADR index is out of date")
        return 1
    print("ADR index: ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
