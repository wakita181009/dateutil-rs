.PHONY: bench bench-save bench-help setup-zoneinfo

BENCH_DIR  := benchmarks
BENCH_ARGS := --benchmark-enable --benchmark-only --benchmark-sort=fullname

bench-help: ## Show benchmark usage
	@echo "Usage:"
	@echo "  make bench            Run benchmarks (original vs local, side-by-side)"
	@echo "  make bench-save       Run and save results to .benchmarks/"
	@echo "  make setup-zoneinfo   Copy zoneinfo data from python-dateutil (auto-runs)"

setup-zoneinfo: ## Copy zoneinfo data from installed python-dateutil if missing
	@test -f src/dateutil/zoneinfo/dateutil-zoneinfo.tar.gz \
		|| python -c "import shutil; from pathlib import Path; import dateutil.zoneinfo as z; \
		   shutil.copy2(Path(z.__file__).parent / 'dateutil-zoneinfo.tar.gz', \
		   'src/dateutil/zoneinfo/dateutil-zoneinfo.tar.gz')" \
		&& echo "Copied dateutil-zoneinfo.tar.gz" \
		|| echo "Already exists"

bench: setup-zoneinfo ## Run benchmarks comparing original python-dateutil vs local
	python -m pytest $(BENCH_DIR) $(BENCH_ARGS) \
		--benchmark-group-by=func

bench-save: setup-zoneinfo ## Run benchmarks and save results as JSON
	python -m pytest $(BENCH_DIR) $(BENCH_ARGS) \
		--benchmark-group-by=func \
		--benchmark-save=comparison
