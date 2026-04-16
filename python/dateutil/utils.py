"""dateutil.utils - General convenience utilities for datetimes."""

import datetime

__all__ = [
    "default_tzinfo",
    "today",
    "within_delta",
]


def today(tzinfo: datetime.tzinfo | None = None) -> datetime.datetime:
    """Return the current day at midnight.

    Parameters
    ----------
    tzinfo : datetime.tzinfo or None
        The timezone to attach (also used to determine the current day).

    Returns
    -------
    datetime.datetime
        A datetime representing the current day at midnight.
    """
    dt = datetime.datetime.now(tzinfo)
    return datetime.datetime.combine(dt.date(), datetime.time(0, tzinfo=tzinfo))


def default_tzinfo(dt: datetime.datetime, tzinfo: datetime.tzinfo) -> datetime.datetime:
    """Set tzinfo on naive datetimes only.

    If *dt* already has a tzinfo, it is returned unchanged.

    Parameters
    ----------
    dt : datetime.datetime
        The datetime on which to replace the timezone.
    tzinfo : datetime.tzinfo
        The tzinfo to assign if *dt* is naive.

    Returns
    -------
    datetime.datetime
        An aware datetime.
    """
    if dt.tzinfo is not None:
        return dt
    return dt.replace(tzinfo=tzinfo)


def within_delta(
    dt1: datetime.datetime,
    dt2: datetime.datetime,
    delta: datetime.timedelta,
) -> bool:
    """Check whether two datetimes are within *delta* of each other.

    Parameters
    ----------
    dt1, dt2 : datetime.datetime
        The datetimes to compare.
    delta : datetime.timedelta
        The maximum allowed difference.

    Returns
    -------
    bool
    """
    delta = abs(delta)
    difference = dt1 - dt2
    return -delta <= difference <= delta
