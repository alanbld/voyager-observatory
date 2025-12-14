.PHONY: help test test-python test-rust test-cross coverage quality docs clean install-dev lint format check-format bootstrap

# Virtual environment auto-detection
VENV := .venv
PYTHON := $(shell if [ -d $(VENV) ]; then echo $(VENV)/bin/python; else echo python3; fi)
PYTEST := $(shell if [ -d $(VENV) ]; then echo $(VENV)/bin/pytest; else echo python3 -m pytest; fi)
COVERAGE := $(shell if [ -d $(VENV) ]; then echo $(VENV)/bin/coverage; else echo python3 -m coverage; fi)

# Default target
.DEFAULT_GOAL := help

help: ## Show this help message
	@echo "pm_encoder - Dual Engine Build System"
	@echo "======================================"
	@echo "Using: $(PYTHON)"
	@echo ""
	@echo "Main Commands:"
	@echo "  make test         - Run all tests (Python + Rust)"
	@echo "  make test-python  - Run Python test suite only"
	@echo "  make test-rust    - Run Rust test suite only"
	@echo "  make test-cross   - Cross-validate Python vs Rust output"
	@echo "  make version      - Show versions of both engines"
	@echo ""
	@echo "Python Commands:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -v "^test:" | grep -v "^version:" | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'

bootstrap: ## Set up development environment with uv
	@./scripts/bootstrap.sh

# Unified test target (both engines)
test: test-python test-rust ## Run all tests (Python + Rust)
	@echo ""
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@echo "  All tests passed! ✅"
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Python test suite (original target renamed)
test-python: ## Run Python test suite
	@echo "Running Python test suite..."
	@$(PYTHON) -m unittest discover -s tests -p 'test_*.py' -v

# Rust test suite
test-rust: ## Run Rust test suite
	@echo "Running Rust test suite..."
	@cd rust && cargo test

# Cross-engine validation
test-cross: ## Cross-validate Python vs Rust output
	@echo "Cross-validating Python vs Rust output..."
	@echo ""
	@echo "1. Python engine:"
	@./pm_encoder.py test_vectors/ -o /tmp/pm_py.txt 2>&1 | head -3
	@echo ""
	@echo "2. Rust engine:"
	@cd rust && cargo run --quiet -- ../test_vectors/ > /tmp/pm_rs.txt 2>&1 || echo "   Note: Rust not yet feature-complete (v0.1.0)"
	@echo ""
	@echo "3. Comparing outputs:"
	@if diff -q /tmp/pm_py.txt /tmp/pm_rs.txt >/dev/null 2>&1; then \
		echo "   ✅ Outputs match! Engines are synchronized."; \
	else \
		echo "   ⚠️  Outputs differ (expected for Rust v0.1.0)"; \
		echo "   Run 'diff /tmp/pm_py.txt /tmp/pm_rs.txt' to see differences"; \
	fi

test-quick: ## Run tests without verbose output
	@$(PYTHON) -m unittest discover -s tests -p 'test_*.py'

test-pytest: ## Run tests with pytest (requires bootstrap)
	@echo "Running tests with pytest..."
	@$(PYTEST) tests/ -v

coverage: ## Run tests with coverage report
	@echo "Running tests with coverage..."
	@$(COVERAGE) run -m unittest discover -s tests -p 'test_*.py'
	@$(COVERAGE) report -m
	@$(COVERAGE) html
	@echo "HTML coverage report generated in htmlcov/"

coverage-check: ## Check if coverage meets 95% threshold
	@$(COVERAGE) run -m unittest discover -s tests -p 'test_*.py' 2>&1 > /dev/null
	@$(COVERAGE) report --fail-under=95 --include="pm_encoder.py"

docs: ## Regenerate auto-synchronized documentation
	@echo "Regenerating documentation..."
	@$(PYTHON) scripts/doc_gen.py
	@echo "Documentation synchronized successfully"

quality: test coverage-check docs ## Run all quality checks
	@echo "==================================="
	@echo "All quality checks passed! ✅"
	@echo "==================================="

clean: ## Clean up generated files (both engines)
	@echo "Cleaning up Python artifacts..."
	@rm -rf htmlcov/
	@rm -rf .coverage
	@rm -rf __pycache__
	@rm -rf tests/__pycache__
	@rm -rf tests/.pytest_cache
	@rm -rf .pytest_cache
	@find . -type d -name "__pycache__" -exec rm -rf {} + 2>/dev/null || true
	@find . -type f -name "*.pyc" -delete
	@echo "Cleaning up Rust artifacts..."
	@cd rust && cargo clean
	@echo "Cleaning up test outputs..."
	@rm -f /tmp/pm_*.txt
	@echo "✅ Cleanup complete"

install-dev: ## Install dev dependencies (legacy, use 'make bootstrap' instead)
	@echo "Installing development dependencies..."
	@pip3 install coverage pytest pytest-cov
	@echo "Development dependencies installed"
	@echo ""
	@echo "TIP: For isolated environment, use 'make bootstrap' instead"

lint: ## Run basic Python syntax check
	@echo "Checking Python syntax..."
	@$(PYTHON) -m py_compile pm_encoder.py
	@$(PYTHON) -m py_compile tests/test_pm_encoder.py
	@$(PYTHON) -m py_compile tests/test_comprehensive.py
	@echo "Syntax check passed"

self-serialize: ## Test self-serialization
	@echo "Testing self-serialization..."
	@./pm_encoder.py . -o /tmp/pm_encoder_test.txt
	@echo "Self-serialization successful"
	@head -1 /tmp/pm_encoder_test.txt

version: ## Show versions of both engines
	@echo "Engine Versions:"
	@echo "  Python: $$(./pm_encoder.py --version 2>&1)"
	@echo "  Rust:   $$(cd rust && cargo run --quiet -- --version 2>&1 | head -1)"

ci: clean test coverage-check lint self-serialize ## Run full CI pipeline locally
	@echo "==================================="
	@echo "CI pipeline passed! ✅"
	@echo "==================================="

check-env: ## Check development environment status
	@echo "Environment Status:"
	@echo "  Python:   $(PYTHON)"
	@echo "  Pytest:   $(PYTEST)"
	@echo "  Coverage: $(COVERAGE)"
	@echo ""
	@if [ -d $(VENV) ]; then \
		echo "  ✅ Virtual environment found at $(VENV)"; \
	else \
		echo "  ⚠️  No virtual environment. Run 'make bootstrap' to create one."; \
	fi

track-metrics: ## Record daily research metrics
	@echo "📊 Recording daily metrics..."
	@$(PYTHON) scripts/track_metrics.py

research-snapshot: coverage track-metrics ## Generate coverage and record metrics
	@echo "🔬 Research snapshot complete!"
	@echo "   - Coverage report: htmlcov/index.html"
	@echo "   - Daily metrics: research/data/daily_snapshots.csv"
