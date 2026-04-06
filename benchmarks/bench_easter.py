"""Benchmarks for dateutil.easter module."""


def test_easter_western_single(benchmark, du):
    """Single Easter calculation (Western method)."""
    benchmark(du.easter.easter, 2024, du.easter.EASTER_WESTERN)


def test_easter_orthodox_single(benchmark, du):
    """Single Easter calculation (Orthodox method)."""
    benchmark(du.easter.easter, 2024, du.easter.EASTER_ORTHODOX)


def test_easter_julian_single(benchmark, du):
    """Single Easter calculation (Julian method)."""
    benchmark(du.easter.easter, 2024, du.easter.EASTER_JULIAN)


def test_easter_western_range(benchmark, du):
    """Easter calculation for 1000 consecutive years (Western)."""
    easter = du.easter.easter
    WESTERN = du.easter.EASTER_WESTERN

    def compute():
        return [easter(y, WESTERN) for y in range(1583, 2583)]

    benchmark(compute)


def test_easter_all_methods_range(benchmark, du):
    """Easter calculation for 500 years x 3 methods."""
    easter = du.easter.easter
    JULIAN = du.easter.EASTER_JULIAN
    ORTHODOX = du.easter.EASTER_ORTHODOX
    WESTERN = du.easter.EASTER_WESTERN

    def compute():
        results = []
        for y in range(1583, 2083):
            for method in (JULIAN, ORTHODOX, WESTERN):
                results.append(easter(y, method))
        return results

    benchmark(compute)
