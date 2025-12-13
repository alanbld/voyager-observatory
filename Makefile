.PHONY: help test coverage quality docs clean install-dev lint format check-format

# Default target
.DEFAULT_GOAL := help

help: ## Show this help message
	@echo "pm_encoder Development Commands"
	@echo "================================"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

test: ## Run the full test suite
	@echo "Running test suite..."
	@python3 -m unittest discover -s tests -p 'test_*.py' -v

test-quick: ## Run tests without verbose output
	@python3 -m unittest discover -s tests -p 'test_*.py'

coverage: ## Run tests with coverage report
	@echo "Running tests with coverage..."
	@python3 -m coverage run -m unittest discover -s tests -p 'test_*.py'
	@python3 -m coverage report -m
	@python3 -m coverage html
	@echo "HTML coverage report generated in htmlcov/"

coverage-check: ## Check if coverage meets 98% threshold
	@python3 -m coverage run -m unittest discover -s tests -p 'test_*.py' 2>&1 > /dev/null
	@python3 -m coverage report --fail-under=98

docs: ## Regenerate auto-synchronized documentation
	@echo "Regenerating documentation..."
	@python3 scripts/doc_gen.py
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
	@find . -type d -name "__pycache__" -exec rm -rf {} + 2>/dev/null || true
	@find . -type f -name "*.pyc" -delete
	@echo "Cleanup complete"

install-dev: ## Install development dependencies (coverage tool)
	@echo "Installing development dependencies..."
	@pip3 install coverage
	@echo "Development dependencies installed"

lint: ## Run basic Python syntax check
	@echo "Checking Python syntax..."
	@python3 -m py_compile pm_encoder.py
	@python3 -m py_compile tests/test_pm_encoder.py
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
