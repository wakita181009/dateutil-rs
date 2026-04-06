import os
import sys

import pytest

# ---------------------------------------------------------------------------
# --rust flag: redirect implemented dateutil.* modules to dateutil_rs.*
# ---------------------------------------------------------------------------
# Modules that have been ported to Rust and can be tested via dateutil_rs.
# Add modules here as they are implemented in each phase.
_RUST_READY_MODULES = {
    "dateutil.easter": "dateutil_rs.easter",
    # Phase 2: "dateutil.relativedelta": "dateutil_rs.relativedelta",
    # Phase 2: "dateutil.parser": "dateutil_rs.parser",
    # Phase 3: "dateutil.tz": "dateutil_rs.tz",
    # Phase 4: "dateutil.rrule": "dateutil_rs.rrule",
}


def pytest_addoption(parser):
    parser.addoption(
        "--rust",
        action="store_true",
        default=False,
        help="Test against dateutil_rs Rust implementation instead of Python reference",
    )


def pytest_configure(config):
    if not config.getoption("--rust"):
        return

    try:
        import dateutil_rs  # noqa: F401
    except ImportError:
        pytest.exit("--rust requires dateutil_rs to be installed (run: uv sync)")

    for py_mod, rs_mod in _RUST_READY_MODULES.items():
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
