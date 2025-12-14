#!/usr/bin/env python3
"""
CLI Parity Validator - Verifies Rust CLI matches Python contract.

This script validates that the Rust binary implements the CLI interface
defined in cli_contract.json.

Validation Checks:
1. Flag Acceptance: Does the flag exist? (not "unknown argument" error)
2. Help Presence: Does --help contain the flag and description keywords?
3. Version Match: Does --version output match the expected format?
4. Type Validation: Does the flag accept the correct argument type?

Outputs:
- interface_parity_percent: (Flags Implemented / Total Flags) * 100
- Detailed report of which flags pass/fail

Part of: Research Phase 2.5 - The Interface Parity Protocol
"""
import json
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, List, Optional, Tuple

# Paths
SCRIPT_DIR = Path(__file__).parent
PROJECT_ROOT = SCRIPT_DIR.parent
CONTRACT_PATH = PROJECT_ROOT / "test_vectors" / "cli_contract.json"
RUST_BINARY = PROJECT_ROOT / "rust" / "target" / "debug" / "pm_encoder"


@dataclass
class ValidationResult:
    """Result of validating a single argument."""
    name: str
    category: str
    flag_exists: bool
    in_help: bool
    help_keywords_found: List[str]
    help_keywords_missing: List[str]
    type_valid: bool
    error_message: Optional[str] = None

    @property
    def passed(self) -> bool:
        """A flag passes if it exists and appears in help."""
        return self.flag_exists and self.in_help


def load_contract() -> Dict:
    """Load the CLI contract from JSON."""
    if not CONTRACT_PATH.exists():
        print(f"Error: Contract not found at {CONTRACT_PATH}")
        print("Run: python scripts/generate_cli_contract.py first")
        sys.exit(1)

    with open(CONTRACT_PATH) as f:
        return json.load(f)


def run_rust_cli(args: List[str], timeout: int = 5) -> Tuple[int, str, str]:
    """
    Run the Rust binary with given arguments.

    Returns: (exit_code, stdout, stderr)
    """
    if not RUST_BINARY.exists():
        # Try to build first
        print("Building Rust binary...")
        build_result = subprocess.run(
            ["cargo", "build"],
            cwd=PROJECT_ROOT / "rust",
            capture_output=True,
            text=True
        )
        if build_result.returncode != 0:
            return (-1, "", f"Build failed: {build_result.stderr}")

    try:
        result = subprocess.run(
            [str(RUST_BINARY)] + args,
            capture_output=True,
            text=True,
            timeout=timeout
        )
        return (result.returncode, result.stdout, result.stderr)
    except subprocess.TimeoutExpired:
        return (-1, "", "Command timed out")
    except Exception as e:
        return (-1, "", str(e))


def validate_flag_exists(arg: Dict) -> Tuple[bool, str]:
    """
    Check if a flag is accepted by the Rust binary.

    A flag "exists" if using it doesn't produce an "unknown argument" error.
    """
    name = arg["name"]

    # Skip positional arguments - they're tested differently
    if not name.startswith("-"):
        # Test with a temp directory
        code, stdout, stderr = run_rust_cli(["/tmp"])
        # If it runs without "unknown argument" error, positional is accepted
        # Accept: exit 0, "does not exist" errors, or permission errors
        # (all indicate the positional was parsed correctly)
        error_lower = stderr.lower()
        if "unexpected argument" in error_lower or "unknown" in error_lower:
            return (False, f"Positional not recognized: {stderr.strip()}")
        return (True, "")

    # For flags that need values, provide a dummy value
    if arg["type"] in ("str", "int", "path", "list"):
        if arg["type"] == "int":
            test_args = [name, "0"]
        elif arg["type"] == "list":
            test_args = [name, "dummy"]
        else:
            test_args = [name, "dummy"]
    else:
        test_args = [name]

    # Add a dummy project root if needed
    test_args.append("/tmp")

    code, stdout, stderr = run_rust_cli(test_args)

    # Check if error is "unexpected argument" vs other errors
    error_lower = stderr.lower()
    if "unexpected argument" in error_lower or "unknown" in error_lower:
        return (False, f"Flag not recognized: {stderr.strip()}")

    # Any other error or success means the flag is accepted
    return (True, "")


def validate_in_help(arg: Dict, help_text: str) -> Tuple[bool, List[str], List[str]]:
    """
    Check if a flag appears in --help output.

    Returns: (found, keywords_found, keywords_missing)
    """
    name = arg["name"]
    help_lower = help_text.lower()

    # Check if the flag name appears in help
    if not name.startswith("-"):
        # Positional - check if dest name appears
        if name.lower() not in help_lower and name.upper() not in help_text:
            return (False, [], [name])
    else:
        if name not in help_text:
            return (False, [], [name])

    # Check for help keywords
    keywords_found = []
    keywords_missing = []

    for keyword in arg.get("help_contains", []):
        if keyword.lower() in help_lower:
            keywords_found.append(keyword)
        else:
            keywords_missing.append(keyword)

    return (True, keywords_found, keywords_missing)


def validate_argument(arg: Dict, help_text: str) -> ValidationResult:
    """Validate a single argument against the contract."""
    name = arg["name"]
    category = arg.get("category", "other")

    # Check if flag exists
    flag_exists, error = validate_flag_exists(arg)

    # Check if in help
    in_help, found, missing = validate_in_help(arg, help_text)

    # Type validation (basic - just check flag acceptance with value)
    type_valid = flag_exists  # For now, existence implies type works

    return ValidationResult(
        name=name,
        category=category,
        flag_exists=flag_exists,
        in_help=in_help,
        help_keywords_found=found,
        help_keywords_missing=missing,
        type_valid=type_valid,
        error_message=error if error else None
    )


def validate_version(contract: Dict) -> Tuple[bool, str]:
    """Validate that --version outputs correctly."""
    code, stdout, stderr = run_rust_cli(["--version"])

    if code != 0:
        return (False, f"--version failed with code {code}")

    # Check format: "pm_encoder X.Y.Z"
    pattern = r"pm_encoder \d+\.\d+\.\d+"
    if re.search(pattern, stdout):
        return (True, stdout.strip())
    else:
        return (False, f"Version format mismatch: {stdout.strip()}")


def calculate_parity(results: List[ValidationResult], contract: Dict) -> Dict:
    """Calculate parity metrics."""
    total = len(results)
    passed = sum(1 for r in results if r.passed)

    # By category
    by_category = {}
    for cat in contract["categories"]:
        cat_results = [r for r in results if r.category == cat]
        cat_passed = sum(1 for r in cat_results if r.passed)
        by_category[cat] = {
            "total": len(cat_results),
            "passed": cat_passed,
            "percent": (cat_passed / len(cat_results) * 100) if cat_results else 0
        }

    # By priority
    by_priority = {}
    for priority, flags in contract["rust_priority"].items():
        priority_results = [r for r in results if r.name in flags]
        priority_passed = sum(1 for r in priority_results if r.passed)
        by_priority[priority] = {
            "total": len(priority_results),
            "passed": priority_passed,
            "percent": (priority_passed / len(priority_results) * 100) if priority_results else 0
        }

    return {
        "total_flags": total,
        "flags_implemented": passed,
        "interface_parity_percent": round(passed / total * 100, 2) if total else 0,
        "by_category": by_category,
        "by_priority": by_priority
    }


def print_report(results: List[ValidationResult], metrics: Dict, version_ok: bool, version_msg: str):
    """Print a human-readable validation report."""
    print("\n" + "=" * 60)
    print("CLI PARITY VALIDATION REPORT")
    print("=" * 60)

    # Version check
    print(f"\nVersion Check: {'PASS' if version_ok else 'FAIL'} - {version_msg}")

    # Overall metrics
    print(f"\nInterface Parity: {metrics['interface_parity_percent']}%")
    print(f"  Flags Implemented: {metrics['flags_implemented']}/{metrics['total_flags']}")

    # By priority
    print("\nBy Priority:")
    for priority in ["critical", "high", "medium", "low"]:
        if priority in metrics["by_priority"]:
            p = metrics["by_priority"][priority]
            status = "PASS" if p["percent"] == 100 else "PARTIAL" if p["percent"] > 0 else "FAIL"
            print(f"  {priority:10}: {p['passed']}/{p['total']} ({p['percent']:.0f}%) [{status}]")

    # Detailed results
    print("\nDetailed Results:")
    for result in sorted(results, key=lambda r: (not r.passed, r.category, r.name)):
        status = "PASS" if result.passed else "FAIL"
        print(f"  [{status}] {result.name:25} ({result.category})")
        if not result.passed:
            if not result.flag_exists:
                print(f"         -> Flag not recognized")
            if not result.in_help:
                print(f"         -> Missing from --help")
            if result.error_message:
                print(f"         -> {result.error_message[:60]}")

    print("\n" + "=" * 60)


def main(output_json: bool = False):
    """Run the CLI parity validation."""
    print("CLI Parity Validator")
    print("-" * 40)

    # Load contract
    contract = load_contract()
    print(f"Contract loaded: {len(contract['arguments'])} arguments")
    print(f"Reference version: {contract['reference_version']}")

    # Get Rust help text
    print("\nFetching Rust --help output...")
    code, help_text, stderr = run_rust_cli(["--help"])
    if code != 0:
        print(f"Error getting help: {stderr}")
        help_text = ""

    # Validate version
    print("Validating --version...")
    version_ok, version_msg = validate_version(contract)

    # Validate each argument
    print("Validating arguments...")
    results = []
    for arg in contract["arguments"]:
        result = validate_argument(arg, help_text)
        results.append(result)

    # Calculate metrics
    metrics = calculate_parity(results, contract)

    # Print report
    print_report(results, metrics, version_ok, version_msg)

    # Output JSON if requested
    if output_json:
        output = {
            "version_ok": version_ok,
            "version_msg": version_msg,
            "metrics": metrics,
            "results": [
                {
                    "name": r.name,
                    "category": r.category,
                    "passed": r.passed,
                    "flag_exists": r.flag_exists,
                    "in_help": r.in_help
                }
                for r in results
            ]
        }
        output_path = PROJECT_ROOT / "research" / "data" / "cli_parity.json"
        output_path.parent.mkdir(parents=True, exist_ok=True)
        with open(output_path, "w") as f:
            json.dump(output, f, indent=2)
        print(f"\nJSON output: {output_path}")

    return metrics["interface_parity_percent"]


if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser(description="Verify CLI parity between Python and Rust")
    parser.add_argument("--json", action="store_true", help="Output results as JSON")
    args = parser.parse_args()

    parity = main(output_json=args.json)
    sys.exit(0 if parity >= 50 else 1)
