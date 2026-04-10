"""Load python-dateutil (PyPI), dateutil-rs v0, and dateutil-rs v1
for three-way benchmarks.

Build both native modules before running:
    maturin develop --manifest-path crates/dateutil-rs/Cargo.toml -F python
    maturin develop --manifest-path crates/dateutil-py/Cargo.toml -F python
"""

from types import SimpleNamespace

import pytest

# ---------------------------------------------------------------------------
# python-dateutil (PyPI baseline)
# ---------------------------------------------------------------------------


def _import_python_dateutil():
    """Import python-dateutil from site-packages (PyPI install)."""
    import dateutil.easter
    import dateutil.parser
    import dateutil.relativedelta
    import dateutil.rrule
    import dateutil.tz
    import dateutil.utils

    return SimpleNamespace(
        name="python-dateutil",
        easter=dateutil.easter,
        parser=dateutil.parser,
        relativedelta=dateutil.relativedelta,
        rrule=dateutil.rrule,
        tz=dateutil.tz,
        utils=dateutil.utils,
    )


# ---------------------------------------------------------------------------
# dateutil-rs v0 (python-dateutil compatible Rust port)
# ---------------------------------------------------------------------------


def _import_v0():
    """Import dateutil_rs v0 and wrap it in a namespace matching dateutil API."""
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
        name="v0",
        easter=dateutil_rs.easter,
        parser=dateutil_rs.parser,
        relativedelta=dateutil_rs.relativedelta,
        rrule=dateutil_rs.rrule,
        tz=dateutil_rs.tz,
        utils=dateutil_rs.utils,
    )


# ---------------------------------------------------------------------------
# dateutil-rs v1 (Rust-optimized core) — with API compatibility wrappers
# ---------------------------------------------------------------------------


def _import_v1():
    """Import dateutil_rs.v1 with compatibility wrappers for benchmark API.

    Note: dateutil_rs.v1.__init__ re-exports names like ``relativedelta`` and
    ``rrule`` (class) which shadow the submodule of the same name. We import
    everything directly from ``_native`` to avoid the ambiguity.
    """
    try:
        from dateutil_rs.v1._native import (
            DAILY,
            EASTER_JULIAN,
            EASTER_ORTHODOX,
            EASTER_WESTERN,
            HOURLY,
            MINUTELY,
            MONTHLY,
            SECONDLY,
            WEEKLY,
            YEARLY,
            FR,
            MO,
            SA,
            SU,
            TH,
            TU,
            WE,
            datetime_ambiguous as _v1_datetime_ambiguous,
            datetime_exists as _v1_datetime_exists,
            easter as _v1_easter_fn,
            gettz as _v1_gettz,
            isoparse as _v1_isoparse_fn,
            parse as _v1_parse_fn,
            relativedelta as _v1_rd_cls,
            resolve_imaginary as _v1_resolve_imaginary,
            rrule as _v1_rrule_cls,
            rruleset as _v1_rruleset_cls,
            rrulestr as _v1_rrulestr_fn,
            tzlocal as _v1_tzlocal,
            tzoffset as _v1_tzoffset,
            tzutc as _v1_tzutc,
        )
    except ImportError:
        return None

    # --- v1 relativedelta (direct use — __radd__/__rsub__/__mul__/from_diff in Rust) ---

    def _v1_relativedelta(dt1=None, dt2=None, **kwargs):
        """Factory matching python-dateutil's relativedelta(dt1=, dt2=, **kw) API."""
        if dt1 is not None and dt2 is not None:
            return _v1_rd_cls.from_diff(dt1, dt2)
        return _v1_rd_cls(**kwargs)

    v1_rd_module = SimpleNamespace(
        relativedelta=_v1_relativedelta,
        MO=MO,
        TU=TU,
        WE=WE,
        TH=TH,
        FR=FR,
        SA=SA,
        SU=SU,
    )

    # --- v1 rrule wrapper (convert tuple → list for byweekday) ---

    def _v1_rrule_compat(freq, **kwargs):
        if "byweekday" in kwargs and isinstance(kwargs["byweekday"], tuple):
            kwargs["byweekday"] = list(kwargs["byweekday"])
        if "bymonth" in kwargs and isinstance(kwargs["bymonth"], tuple):
            kwargs["bymonth"] = list(kwargs["bymonth"])
        if "bymonthday" in kwargs and isinstance(kwargs["bymonthday"], tuple):
            kwargs["bymonthday"] = list(kwargs["bymonthday"])
        return _v1_rrule_cls(freq, **kwargs)

    v1_rrule_module = SimpleNamespace(
        rrule=_v1_rrule_compat,
        rruleset=_v1_rruleset_cls,
        rrulestr=_v1_rrulestr_fn,
        YEARLY=YEARLY,
        MONTHLY=MONTHLY,
        WEEKLY=WEEKLY,
        DAILY=DAILY,
        HOURLY=HOURLY,
        MINUTELY=MINUTELY,
        SECONDLY=SECONDLY,
        MO=MO,
        TU=TU,
        WE=WE,
        TH=TH,
        FR=FR,
        SA=SA,
        SU=SU,
    )

    # --- v1 easter namespace ---

    v1_easter_module = SimpleNamespace(
        easter=_v1_easter_fn,
        EASTER_JULIAN=EASTER_JULIAN,
        EASTER_ORTHODOX=EASTER_ORTHODOX,
        EASTER_WESTERN=EASTER_WESTERN,
    )

    # --- v1 parser namespace ---

    v1_parser_module = SimpleNamespace(
        parse=_v1_parse_fn,
        isoparse=_v1_isoparse_fn,
    )

    # --- v1 tz namespace ---

    v1_tz_module = SimpleNamespace(
        tzutc=_v1_tzutc,
        tzoffset=_v1_tzoffset,
        tzlocal=_v1_tzlocal,
        gettz=_v1_gettz,
        UTC=_v1_tzutc(),
        datetime_exists=_v1_datetime_exists,
        datetime_ambiguous=_v1_datetime_ambiguous,
        resolve_imaginary=_v1_resolve_imaginary,
    )

    return SimpleNamespace(
        name="v1",
        easter=v1_easter_module,
        parser=v1_parser_module,
        relativedelta=v1_rd_module,
        rrule=v1_rrule_module,
        tz=v1_tz_module,
        utils=None,
    )


# ---------------------------------------------------------------------------
# Module-level imports (run once at collection time)
# ---------------------------------------------------------------------------

_python_dateutil = _import_python_dateutil()
_v0 = _import_v0()
_v1 = _import_v1()

# ---------------------------------------------------------------------------
# Parametrized fixture
# ---------------------------------------------------------------------------


@pytest.fixture(params=["python-dateutil", "v0", "v1"])
def du(request):
    """Fixture providing dateutil modules — parametrized for all three versions."""
    if request.param == "python-dateutil":
        return _python_dateutil

    if request.param == "v0":
        if _v0 is None:
            pytest.skip(
                "dateutil_rs v0 not installed (run: maturin develop --manifest-path crates/dateutil-rs/Cargo.toml -F python)"
            )
        return _v0

    # v1
    if _v1 is None:
        pytest.skip(
            "dateutil_rs v1 not installed (run: maturin develop --manifest-path crates/dateutil-py/Cargo.toml -F python)"
        )

    return _v1
