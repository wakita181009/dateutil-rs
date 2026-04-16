"""dateutil.zoneinfo — Stub for python-dateutil compatibility.

dateutil-rs uses the system IANA timezone database via gettz() and does not
bundle a timezone tarball.  This module provides the minimum API surface so
that code importing ``from dateutil import zoneinfo`` does not break.
"""

from __future__ import annotations

import warnings
from typing import Any


class ZoneInfoFile:
    """Minimal stub — always reports an empty zone set."""

    def __init__(self, zonefile_stream: Any = None) -> None:
        self.zones: dict[str, Any] = {}
        self.metadata: Any = None

    def get(self, name: str) -> Any:
        from dateutil.tz import gettz

        return gettz(name)


_CACHE: list[ZoneInfoFile] = []


def get_zonefile_instance(new_instance: bool = False) -> ZoneInfoFile:
    """Return a cached :class:`ZoneInfoFile` instance.

    Because dateutil-rs does not ship a tarball the instance always has
    ``zones == {}``, which causes upstream skipIf guards to skip tarball-
    dependent tests automatically.
    """
    if new_instance or not _CACHE:
        if new_instance:
            _CACHE.clear()
        _CACHE.append(ZoneInfoFile())
    return _CACHE[0]


def gettz(name: str) -> Any:
    """Look up a timezone by IANA name (delegates to ``dateutil.tz.gettz``)."""
    from dateutil.tz import gettz as _gettz

    return _gettz(name)


def gettz_db_metadata() -> Any:
    """Return metadata for the bundled timezone database.

    Always emits a ``DeprecationWarning`` because dateutil-rs does not ship a
    bundled tarball.
    """
    warnings.warn(
        "gettz_db_metadata is not supported in dateutil-rs",
        DeprecationWarning,
        stacklevel=2,
    )
    return None
