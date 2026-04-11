"""Shared fixtures for dateutil_rs tests."""

import pytest


@pytest.fixture
def utc():
    """Return dateutil_rs.tzutc() instance."""
    from dateutil_rs import tzutc

    return tzutc()


@pytest.fixture
def eastern():
    """Return US/Eastern timezone via gettz."""
    from dateutil_rs import gettz

    return gettz("US/Eastern")


@pytest.fixture
def tokyo():
    """Return Asia/Tokyo timezone via gettz."""
    from dateutil_rs import gettz

    return gettz("Asia/Tokyo")
