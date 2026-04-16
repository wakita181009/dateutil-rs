.PHONY: bench bench-save bench-help build build-v0 build-v1

PYTHON := $(shell uv run python -c "import sys; print(f'cpython-3{sys.version_info[1]}')")
ARCH   := $(shell uv run python -c "import sysconfig; print(sysconfig.get_config_var('SOABI').split('-',1)[1])")
SO_EXT := _native.$(PYTHON)-$(ARCH).so

# ---------------------------------------------------------------------------
# Build
# ---------------------------------------------------------------------------

build: ## Build unified native module (v0 + v1) via maturin
	maturin develop --release

build-v1-standalone: ## Build standalone v1 cdylib for isolated benchmarks
	PYO3_PYTHON=$$(uv run python -c "import sys; print(sys.executable)") \
	cargo rustc -p dateutil-py -F python,standalone --release --crate-type cdylib \
		-- -C link-arg=-undefined -C link-arg=dynamic_lookup
	cp target/release/libdateutil_py.dylib python/dateutil/$(SO_EXT)

# ---------------------------------------------------------------------------
# Benchmarks
# ---------------------------------------------------------------------------

BENCH_DIR  := benchmarks
BENCH_ARGS := --benchmark-enable --benchmark-only --benchmark-sort=fullname

bench-help: ## Show benchmark usage
	@echo "Usage:"
	@echo "  make build                  Build unified native module (v0 + v1)"
	@echo "  make build-v1-standalone    Build standalone v1 cdylib for benchmarks"
	@echo "  make bench                  Run 3-way benchmarks (python-dateutil vs v0 vs v1)"
	@echo "  make bench-save             Run and save results to .benchmarks/"
	@echo ""
	@echo "Run a specific module:"
	@echo "  make bench BENCH_FILE=bench_easter.py"
	@echo "  make bench BENCH_FILE=bench_parser.py"
	@echo "  make bench BENCH_FILE=bench_relativedelta.py"
	@echo "  make bench BENCH_FILE=bench_rrule.py"
	@echo "  make bench BENCH_FILE=bench_tz.py"

BENCH_FILE ?=

bench: ## Run benchmarks (dateutil Rust implementation)
	uv run pytest $(BENCH_DIR)/$(BENCH_FILE) $(BENCH_ARGS)

bench-save: ## Run benchmarks and save results as JSON
	uv run pytest $(BENCH_DIR)/$(BENCH_FILE) $(BENCH_ARGS) \
		--benchmark-save=comparison
