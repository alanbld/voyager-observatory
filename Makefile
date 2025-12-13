.PHONY: help test coverage quality docs clean install-dev lint format check-format bootstrap

# Virtual environment auto-detection
VENV := .venv
PYTHON := $(shell if [ -d $(VENV) ]; then echo $(VENV)/bin/python; else echo python3; fi)
PYTEST := $(shell if [ -d $(VENV) ]; then echo $(VENV)/bin/pytest; else echo python3 -m pytest; fi)
COVERAGE := $(shell if [ -d $(VENV) ]; then echo $(VENV)/bin/coverage; else echo python3 -m coverage; fi)

# Default target
.DEFAULT_GOAL := help

help: ## Show this help message
	@echo "pm_encoder Development Commands"
	@echo "================================"
	@echo "Using: $(PYTHON)"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

bootstrap: ## Set up development environment with uv
	@./scripts/bootstrap.sh

test: ## Run the full test suite
	@echo "Running test suite..."
	@$(PYTHON) -m unittest discover -s tests -p 'test_*.py' -v

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

clean: ## Clean up generated files
	@echo "Cleaning up..."
	@rm -rf htmlcov/
	@rm -rf .coverage
	@rm -rf __pycache__
	@rm -rf tests/__pycache__
	@rm -rf tests/.pytest_cache
	@rm -rf .pytest_cache
	@find . -type d -name "__pycache__" -exec rm -rf {} + 2>/dev/null || true
	@find . -type f -name "*.pyc" -delete
	@echo "Cleanup complete"

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

version: ## Show current version
	@./pm_encoder.py --version

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
