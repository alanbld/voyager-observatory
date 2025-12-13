#!/usr/bin/env bash
#
# pm_encoder Development Environment Bootstrap
# Uses 'uv' for fast, reliable dependency management
#
set -e

VENV_DIR=".venv"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  pm_encoder Development Bootstrap"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo

# Check if uv is installed
if ! command -v uv &> /dev/null; then
    echo "❌ 'uv' is not installed."
    echo
    echo "Install uv with:"
    echo "  curl -LsSf https://astral.sh/uv/install.sh | sh"
    echo
    echo "Or on macOS with Homebrew:"
    echo "  brew install uv"
    echo
    exit 1
fi

echo "✓ uv found: $(uv --version)"
echo

# Create virtual environment
echo "Creating virtual environment..."
if [ -d "$VENV_DIR" ]; then
    echo "  → Existing $VENV_DIR found, recreating..."
    rm -rf "$VENV_DIR"
fi

uv venv "$VENV_DIR"
echo "✓ Virtual environment created at $VENV_DIR"
echo

# Install dev dependencies
echo "Installing development dependencies..."
uv pip install --python "$VENV_DIR/bin/python" pytest pytest-cov coverage
echo "✓ Dependencies installed"
echo

# Verify installation
echo "Verifying installation..."
"$VENV_DIR/bin/python" --version
"$VENV_DIR/bin/pytest" --version | head -1
"$VENV_DIR/bin/coverage" --version | head -1
echo

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Environment ready!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo
echo "Usage:"
echo "  source .venv/bin/activate  # Activate manually"
echo "  make test                  # Run tests (auto-detects venv)"
echo "  make coverage              # Run with coverage"
echo "  make quality               # Full quality checks"
echo
