"""dateutil_rs.utils - Utility functions (all Rust-accelerated)."""

from dateutil_rs._native import default_tzinfo, today, within_delta

__all__ = ["default_tzinfo", "today", "within_delta"]
