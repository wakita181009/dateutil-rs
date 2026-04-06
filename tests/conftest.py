import os
import sys

import pytest

# ---------------------------------------------------------------------------
# --rust flag: redirect all dateutil.* modules to dateutil_rs.*
# ---------------------------------------------------------------------------
# dateutil_rs provides Rust-accelerated modules (easter, relativedelta) and
# delegates unported modules (parser, rrule, tz) to python-dateutil.
# This mapping lets the existing test suite run against dateutil_rs unchanged.
# Only redirect modules whose dateutil_rs wrappers are self-contained.
# parser/tz/rrule have internal sub-imports (dateutil.parser._parser, etc.)
# that break when the top-level module is redirected, so leave them as-is.
# Modules redirected to dateutil_rs when --rust is used.
# Only modules with self-contained wrappers (no internal sub-imports).
# utils is NOT redirected because freezegun can't patch the re-exported today().
_MODULE_MAP = {
    "dateutil.easter": "dateutil_rs.easter",
    "dateutil.relativedelta": "dateutil_rs.relativedelta",
}


def pytest_addoption(parser):
    parser.addoption(
        "--rust",
        action="store_true",
        default=False,
        help="Test against dateutil_rs (Rust + python-dateutil hybrid)",
    )


def pytest_configure(config):
    if not config.getoption("--rust"):
        return

    try:
        import dateutil_rs
    except ImportError:
        pytest.exit("--rust requires dateutil_rs to be installed (run: uv sync)")

    for py_mod, rs_mod in _MODULE_MAP.items():
        rust_module = __import__(rs_mod, fromlist=[""])
        sys.modules[py_mod] = rust_module


# Configure pytest to ignore xfailing tests
# See: https://stackoverflow.com/a/53198349/467366
def pytest_collection_modifyitems(items):
    for item in items:
        marker_getter = getattr(item, "get_closest_marker", None)

        # Python 3.3 support
        if marker_getter is None:
            marker_getter = item.get_marker

        marker = marker_getter("xfail")

        # Need to query the args because conditional xfail tests still have
        # the xfail mark even if they are not expected to fail
        if marker and (not marker.args or marker.args[0]):
            item.add_marker(pytest.mark.no_cover)


def set_tzpath():
    """
    Sets the TZPATH variable if it's specified in an environment variable.
    """
    tzpath = os.environ.get("DATEUTIL_TZPATH", None)

    if tzpath is None:
        return

    path_components = tzpath.split(":")

    print(f"Setting TZPATH to {path_components}")

    from dateutil import tz

    tz.TZPATHS.clear()
    tz.TZPATHS.extend(path_components)


set_tzpath()
