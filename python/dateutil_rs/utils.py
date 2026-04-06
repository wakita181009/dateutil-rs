"""dateutil_rs.utils - Utility functions.

within_delta is Rust-accelerated; others delegate to python-dateutil.
"""

from dateutil.utils import default_tzinfo, today
from dateutil_rs._native import within_delta

__all__ = ["default_tzinfo", "today", "within_delta"]
