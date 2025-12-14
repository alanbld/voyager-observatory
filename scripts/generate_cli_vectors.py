#!/usr/bin/env python3
"""
Generate CLI test vectors for Python/Rust parity testing.

These vectors test CLI interface behavior: --help, --version, error handling.
Unlike serialization vectors, CLI vectors test binary execution, not library output.
"""
import json
import subprocess
import sys
from pathlib import Path

# Output directory
VECTORS_DIR = Path(__file__).parent.parent / "test_vectors" / "rust_parity"


def run_python_cli(args: list) -> dict:
    """Run Python CLI and capture output."""
    cmd = ["python3", str(Path(__file__).parent.parent / "pm_encoder.py")] + args
    result = subprocess.run(cmd, capture_output=True, text=True)
    return {
        "stdout": result.stdout,
        "stderr": result.stderr,
        "exit_code": result.returncode
    }


def generate_help_vector() -> dict:
    """Generate cli_01_help.json - tests --help output."""
    result = run_python_cli(["--help"])

    # Extract key flags that must be present in both implementations
    required_flags = [
        "--help",
        "--version",
        "-o", "--output",
        "-c", "--config",
        "--include",
        "--exclude",
        "--sort-by",
        "--sort-order",
        "--truncate",
        "--truncate-mode",
        "--lens",
        "project_root"
    ]

    # Key descriptions that convey semantic meaning
    required_descriptions = [
        "Plus/Minus format",
        "output file",
        "configuration file",
        "glob patterns",
        "sort",
        "truncate",
        "lens"
    ]

    return {
        "name": "cli_01_help",
        "description": "Verify --help outputs usage information with all flags",
        "category": "cli",
        "input": {
            "cli_args": ["--help"]
        },
        "expected": {
            "exit_code": 0,
            "stdout_contains": required_flags,
            "stdout_contains_any": required_descriptions,
            "stderr": "",
            "reference_output": result["stdout"]
        },
        "validation_mode": "semantic",
        "notes": "Help text format may differ (argparse vs clap), but must contain same flags",
        "python_validated": True,
        "rust_status": "pending"
    }


def generate_version_vector() -> dict:
    """Generate cli_02_version.json - tests --version output."""
    result = run_python_cli(["--version"])

    return {
        "name": "cli_02_version",
        "description": "Verify --version outputs version in correct format",
        "category": "cli",
        "input": {
            "cli_args": ["--version"]
        },
        "expected": {
            "exit_code": 0,
            "stdout_regex": r"pm_encoder \d+\.\d+\.\d+",
            "stdout_contains": ["pm_encoder"],
            "stderr": "",
            "reference_output": result["stdout"].strip()
        },
        "validation_mode": "regex",
        "notes": "Version format must be 'pm_encoder X.Y.Z'",
        "python_validated": True,
        "rust_status": "pending"
    }


def generate_invalid_arg_vector() -> dict:
    """Generate cli_03_invalid_arg.json - tests error handling for unknown flags."""
    result = run_python_cli(["--this-flag-does-not-exist"])

    return {
        "name": "cli_03_invalid_arg",
        "description": "Verify unknown flags produce error with non-zero exit code",
        "category": "cli",
        "input": {
            "cli_args": ["--this-flag-does-not-exist"]
        },
        "expected": {
            "exit_code_nonzero": True,
            "stderr_contains": ["error", "unrecognized"],
            "stderr_contains_any": ["invalid", "unknown", "unrecognized"],
            "reference_stderr": result["stderr"]
        },
        "validation_mode": "error",
        "notes": "Error message wording may differ, but must indicate invalid argument",
        "python_validated": True,
        "rust_status": "pending"
    }


def generate_missing_dir_vector() -> dict:
    """Generate cli_04_missing_dir.json - tests error for non-existent directory."""
    result = run_python_cli(["/nonexistent/path/that/does/not/exist"])

    return {
        "name": "cli_04_missing_dir",
        "description": "Verify non-existent directory produces appropriate error",
        "category": "cli",
        "input": {
            "cli_args": ["/nonexistent/path/that/does/not/exist"]
        },
        "expected": {
            "exit_code_nonzero": True,
            "stderr_contains_any": ["error", "not exist", "not found", "no such", "invalid"],
            "reference_stderr": result["stderr"]
        },
        "validation_mode": "error",
        "notes": "Must fail gracefully for non-existent paths",
        "python_validated": True,
        "rust_status": "pending"
    }


def main():
    """Generate all CLI test vectors."""
    print("Generating CLI test vectors...")

    vectors = [
        ("cli_01_help.json", generate_help_vector()),
        ("cli_02_version.json", generate_version_vector()),
        ("cli_03_invalid_arg.json", generate_invalid_arg_vector()),
        ("cli_04_missing_dir.json", generate_missing_dir_vector()),
    ]

    for filename, vector in vectors:
        path = VECTORS_DIR / filename
        with open(path, "w") as f:
            json.dump(vector, f, indent=2)
        print(f"  Created: {path}")

    print(f"\nGenerated {len(vectors)} CLI test vectors.")
    print("\nNext steps:")
    print("  1. Add Clap to Rust Cargo.toml")
    print("  2. Implement CLI parsing in main.rs")
    print("  3. Run: cargo test --test test_vectors")


if __name__ == "__main__":
    main()
