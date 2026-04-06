"""Load both original (PyPI) and local dateutil for side-by-side benchmarks."""

import shutil
import sys
from pathlib import Path
from types import SimpleNamespace

import pytest

_PROJECT_SRC = str(Path(__file__).resolve().parent.parent / "src")


def _ensure_zoneinfo_data():
    """Copy dateutil-zoneinfo.tar.gz from PyPI python-dateutil if missing locally."""
    local_zoneinfo = (
        Path(_PROJECT_SRC) / "dateutil" / "zoneinfo" / "dateutil-zoneinfo.tar.gz"
    )
    if local_zoneinfo.exists():
        return
    import dateutil.zoneinfo as _zi

    src_tar = Path(_zi.__file__).parent / "dateutil-zoneinfo.tar.gz"
    if src_tar.exists():
        shutil.copy2(src_tar, local_zoneinfo)


_ensure_zoneinfo_data()


def _import_dateutil(use_local: bool):
    """Import a complete dateutil snapshot from either local src/ or site-packages.

    Returns a SimpleNamespace with attributes: easter, parser, relativedelta,
    rrule, tz, utils — each a module object.
    """
    # 1. Save and clear all cached dateutil modules
    saved = {}
    for key in list(sys.modules):
        if key == "dateutil" or key.startswith("dateutil."):
            saved[key] = sys.modules.pop(key)

    # 2. Adjust sys.path
    had_src = _PROJECT_SRC in sys.path
    if use_local:
        if not had_src:
            sys.path.insert(0, _PROJECT_SRC)
    else:
        while _PROJECT_SRC in sys.path:
            sys.path.remove(_PROJECT_SRC)

    # 3. Import fresh
    import dateutil.easter
    import dateutil.parser
    import dateutil.relativedelta
    import dateutil.rrule
    import dateutil.tz
    import dateutil.utils

    ns = SimpleNamespace(
        easter=dateutil.easter,
        parser=dateutil.parser,
        relativedelta=dateutil.relativedelta,
        rrule=dateutil.rrule,
        tz=dateutil.tz,
        utils=dateutil.utils,
    )

    # 4. Remove newly loaded modules from cache
    for key in list(sys.modules):
        if key == "dateutil" or key.startswith("dateutil."):
            del sys.modules[key]

    # 5. Restore original sys.path and module cache
    if had_src and _PROJECT_SRC not in sys.path:
        sys.path.insert(0, _PROJECT_SRC)
    elif not had_src and _PROJECT_SRC in sys.path:
        sys.path.remove(_PROJECT_SRC)
    sys.modules.update(saved)

    return ns


# Pre-load both Python versions once at collection time
_original = _import_dateutil(use_local=False)
_local = _import_dateutil(use_local=True)


def _import_rust():
    """Import dateutil_rs and wrap it in a namespace matching the dateutil API."""
    try:
        import dateutil_rs.easter
        import dateutil_rs.parser
        import dateutil_rs.relativedelta
        import dateutil_rs.utils
    except ImportError:
        return None

    return SimpleNamespace(
        easter=dateutil_rs.easter,
        parser=dateutil_rs.parser,
        relativedelta=dateutil_rs.relativedelta,
        rrule=None,
        tz=None,
        utils=dateutil_rs.utils,
    )


_rust = _import_rust()


@pytest.fixture(params=["original", "local", "rust"])
def du(request):
    """Fixture providing dateutil modules — parametrized for all versions."""
    if request.param == "original":
        return _original
    elif request.param == "local":
        return _local
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
