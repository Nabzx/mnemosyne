"""Tests for the consistency-check scripts (#104)."""

from check_adr_index import index_rows
from check_adr_refs import next_number
from check_changelog import check as changelog_check
from check_changelog import is_user_facing


def test_next_number_is_the_sequel() -> None:
    assert next_number({"0001", "0009", "0011"}) == "0012"
    assert next_number(set()) == "0001"


def test_index_rows_parses_the_table() -> None:
    text = (
        "| ADR | Title | Status |\n"
        "| --- | --- | --- |\n"
        "| [0001](0001-record-architecture-decisions.md) | Record | Accepted |\n"
        "| [0011](0011-conventional-commits-and-issue-lifecycle.md) | CC | Accepted |\n"
    )
    rows = index_rows(text)
    assert rows["0001"] == "0001-record-architecture-decisions.md"
    assert rows["0011"] == "0011-conventional-commits-and-issue-lifecycle.md"


def test_user_facing_paths() -> None:
    assert is_user_facing("crates/mnem-core/src/commit.rs")
    assert is_user_facing("python/mnem/_sdk.py")
    assert is_user_facing("docs/format/README.md")
    assert is_user_facing("README.md")
    assert not is_user_facing("crates/mnem-core/tests/round_trip.rs")
    assert not is_user_facing("docs/adr/0011-x.md")
    assert not is_user_facing(".github/workflows/ci.yml")
    assert not is_user_facing("scripts/check_changelog.py")


def test_changelog_check_flags_a_missing_update() -> None:
    assert changelog_check(["crates/mnem-cli/src/main.rs"])
    assert changelog_check(["crates/mnem-cli/src/main.rs", "CHANGELOG.md"]) == []
    assert changelog_check(["docs/adr/0012-x.md", "README.md", "CHANGELOG.md"]) == []
    assert changelog_check([".github/workflows/ci.yml"]) == []
