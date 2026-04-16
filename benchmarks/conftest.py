"""Benchmarks for dateutil (Rust implementation).

Build the native module before running:
    maturin develop -F python
"""

from types import SimpleNamespace

import pytest

from dateutil._native import (
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
    easter,
    gettz,
    isoparse,
    parse,
    relativedelta,
    resolve_imaginary,
    rrule,
    rruleset,
    rrulestr,
    tzlocal,
    tzoffset,
    tzutc,
)

_du = SimpleNamespace(
    name="dateutil",
    easter=SimpleNamespace(
        easter=easter,
        EASTER_JULIAN=EASTER_JULIAN,
        EASTER_ORTHODOX=EASTER_ORTHODOX,
        EASTER_WESTERN=EASTER_WESTERN,
    ),
    parser=SimpleNamespace(
        parse=parse,
        isoparse=isoparse,
    ),
    relativedelta=SimpleNamespace(
        relativedelta=relativedelta,
        MO=MO,
        TU=TU,
        WE=WE,
        TH=TH,
        FR=FR,
        SA=SA,
        SU=SU,
    ),
    rrule=SimpleNamespace(
        rrule=rrule,
        rruleset=rruleset,
        rrulestr=rrulestr,
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
    ),
    tz=SimpleNamespace(
        tzutc=tzutc,
        tzoffset=tzoffset,
        tzlocal=tzlocal,
        gettz=gettz,
        UTC=tzutc(),
        datetime_exists=datetime_exists,
        datetime_ambiguous=datetime_ambiguous,
        resolve_imaginary=resolve_imaginary,
    ),
)


@pytest.fixture
def du():
    """Fixture providing dateutil modules for benchmarks."""
    return _du
