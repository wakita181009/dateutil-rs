"""Load python-dateutil (PyPI) and dateutil-rs for two-way benchmarks.

Build the native module before running:
    maturin develop -F python
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
# dateutil-rs (Rust-optimized core)
# ---------------------------------------------------------------------------


def _import_dateutil_rs():
    """Import dateutil_rs with compatibility wrappers for benchmark API."""
    try:
        from dateutil_rs._native import (
            DAILY,
            EASTER_JULIAN,
            EASTER_ORTHODOX,
            EASTER_WESTERN,
            FR,
            HOURLY,
            MINUTELY,
            MO,
            MONTHLY,
            SA,
            SECONDLY,
            SU,
            TH,
            TU,
            WE,
            WEEKLY,
            YEARLY,
            datetime_ambiguous,
            datetime_exists,
            gettz,
            resolve_imaginary,
            tzlocal,
            tzoffset,
            tzutc,
        )
        from dateutil_rs._native import (
            easter as _easter_fn,
        )
        from dateutil_rs._native import (
            isoparse as _isoparse_fn,
        )
        from dateutil_rs._native import (
            parse as _parse_fn,
        )
        from dateutil_rs._native import (
            relativedelta as _rd_cls,
        )
        from dateutil_rs._native import (
            rrule as _rrule_cls,
        )
        from dateutil_rs._native import (
            rruleset as _rruleset_cls,
        )
        from dateutil_rs._native import (
            rrulestr as _rrulestr_fn,
        )
    except ImportError:
        return None

    rd_module = SimpleNamespace(
        relativedelta=_rd_cls,
        MO=MO,
        TU=TU,
        WE=WE,
        TH=TH,
        FR=FR,
        SA=SA,
        SU=SU,
    )

    rrule_module = SimpleNamespace(
        rrule=_rrule_cls,
        rruleset=_rruleset_cls,
        rrulestr=_rrulestr_fn,
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

    easter_module = SimpleNamespace(
        easter=_easter_fn,
        EASTER_JULIAN=EASTER_JULIAN,
        EASTER_ORTHODOX=EASTER_ORTHODOX,
        EASTER_WESTERN=EASTER_WESTERN,
    )

    parser_module = SimpleNamespace(
        parse=_parse_fn,
        isoparse=_isoparse_fn,
    )

    tz_module = SimpleNamespace(
        tzutc=tzutc,
        tzoffset=tzoffset,
        tzlocal=tzlocal,
        gettz=gettz,
        UTC=tzutc(),
        datetime_exists=datetime_exists,
        datetime_ambiguous=datetime_ambiguous,
        resolve_imaginary=resolve_imaginary,
    )

    return SimpleNamespace(
        name="dateutil-rs",
        easter=easter_module,
        parser=parser_module,
        relativedelta=rd_module,
        rrule=rrule_module,
        tz=tz_module,
        utils=None,
    )


# ---------------------------------------------------------------------------
# Module-level imports (run once at collection time)
# ---------------------------------------------------------------------------

_python_dateutil = _import_python_dateutil()
_dateutil_rs = _import_dateutil_rs()

# ---------------------------------------------------------------------------
# Parametrized fixture
# ---------------------------------------------------------------------------


@pytest.fixture(params=["python-dateutil", "dateutil-rs"])
def du(request):
    """Fixture providing dateutil modules — parametrized for both versions."""
    if request.param == "python-dateutil":
        return _python_dateutil

    if _dateutil_rs is None:
        pytest.skip("dateutil_rs not installed (run: maturin develop -F python)")
    return _dateutil_rs
