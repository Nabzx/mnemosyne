"""Tests for the conventional-commit check (ADR-0011)."""

from conventional_commits import check_message, check_subject


def test_accepts_a_plain_conventional_subject() -> None:
    assert check_subject("feat: add the log walk") == []
    assert check_subject("fix(core): reject a detached-HEAD commit") == []
    assert check_subject("docs(adr): record the ref model") == []
    assert check_subject("feat(sdk)!: rename Store.open") == []


def test_rejects_a_missing_or_wrong_type() -> None:
    assert check_subject("add the log walk")
    assert check_subject("Feature: add the log walk")
    assert check_subject("wip: poke at things")


def test_rejects_an_uppercase_description() -> None:
    assert check_subject("feat: Add the log walk")


def test_rejects_a_trailing_full_stop() -> None:
    assert any("full stop" in p for p in check_subject("feat: add the log walk."))


def test_rejects_an_overlong_subject() -> None:
    long = "feat: " + "x" * 80
    assert any("characters" in p for p in check_subject(long))


def test_rejects_closing_keywords_in_the_body() -> None:
    message = "fix(core): stop the panic\n\nCloses #12\n"
    assert any("closing keyword" in p for p in check_message(message))
    assert check_message("fix(core): stop the panic\n\nrefs #12\n") == []


def test_accepts_a_full_message() -> None:
    message = "feat(cli): wire init, add, commit and log to the core\n\nrefs #28\n"
    assert check_message(message) == []
