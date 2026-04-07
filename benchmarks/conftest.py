"""Load both original (PyPI) and Rust dateutil for side-by-side benchmarks."""

from types import SimpleNamespace

import pytest


def _import_original():
    """Import python-dateutil from site-packages (PyPI install)."""
    import dateutil.easter
    import dateutil.parser
    import dateutil.relativedelta
    import dateutil.rrule
    import dateutil.tz
    import dateutil.utils

    return SimpleNamespace(
        easter=dateutil.easter,
        parser=dateutil.parser,
        relativedelta=dateutil.relativedelta,
        rrule=dateutil.rrule,
        tz=dateutil.tz,
        utils=dateutil.utils,
    )


def _import_rust():
    """Import dateutil_rs and wrap it in a namespace matching the dateutil API."""
    try:
        import dateutil_rs.easter
        import dateutil_rs.parser
        import dateutil_rs.relativedelta
        import dateutil_rs.rrule
        import dateutil_rs.tz
        import dateutil_rs.utils
    except ImportError:
        return None

    return SimpleNamespace(
        easter=dateutil_rs.easter,
        parser=dateutil_rs.parser,
        relativedelta=dateutil_rs.relativedelta,
        rrule=dateutil_rs.rrule,
        tz=dateutil_rs.tz,
        utils=dateutil_rs.utils,
    )


_original = _import_original()
_rust = _import_rust()


@pytest.fixture(params=["original", "rust"])
def du(request):
    """Fixture providing dateutil modules — parametrized for all versions."""
    if request.param == "original":
        return _original
    else:
        if _rust is None:
            pytest.skip("dateutil_rs not installed (run: maturin develop)")

        # Skip benchmarks for modules not yet implemented in Rust
        test_path = request.node.nodeid
        if "bench_rrule" in test_path and _rust.rrule is None:
            pytest.skip("dateutil_rs.rrule not yet implemented")
        if "bench_tz" in test_path and _rust.tz is None:
            pytest.skip("dateutil_rs.tz not yet implemented")

        return _rust
