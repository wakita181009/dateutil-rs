.PHONY: bench bench-save bench-help

BENCH_DIR  := benchmarks
BENCH_ARGS := --benchmark-enable --benchmark-only --benchmark-sort=fullname

bench-help: ## Show benchmark usage
	@echo "Usage:"
	@echo "  make bench            Run benchmarks (original vs rust, side-by-side)"
	@echo "  make bench-save       Run and save results to .benchmarks/"

bench: ## Run benchmarks comparing original python-dateutil vs rust
	python -m pytest $(BENCH_DIR) $(BENCH_ARGS) \
		--benchmark-group-by=func

bench-save: ## Run benchmarks and save results as JSON
	python -m pytest $(BENCH_DIR) $(BENCH_ARGS) \
		--benchmark-group-by=func \
		--benchmark-save=comparison
