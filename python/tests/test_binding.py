"""Phase 0 smoke tests for the Python SDK skeleton."""

import mnem


def test_version_is_exposed() -> None:
    assert mnem.__version__ == "0.0.1"
    assert mnem.version() == "0.0.1"


def test_unbuilt_api_raises_clearly() -> None:
    try:
        _ = mnem.Store  # type: ignore[attr-defined]
    except AttributeError as error:
        assert "Phase 1" in str(error)
    else:  # pragma: no cover
        raise AssertionError("expected mnem.Store to be unavailable in Phase 0")
