#!/usr/bin/env python3
"""
Track daily metrics for The Twins research project.

Collects:
- Python test coverage (from coverage.xml)
- Rust test coverage (from tarpaulin output)
- Test vector parity (from test vector JSON files)
- Interface parity (CLI contract compliance)
- Lines of code (both engines)
- Velocity (rolling 7-day average)

Appends to: research/data/daily_snapshots.csv
"""

import json
import csv
import subprocess
from pathlib import Path
from datetime import date, datetime, timedelta
from typing import Dict, List, Tuple
import xml.etree.ElementTree as ET


def get_project_root() -> Path:
    """Get the project root directory."""
    return Path(__file__).parent.parent


def get_python_coverage() -> float:
    """
    Parse coverage.xml to get Python test coverage percentage.

    Returns:
        Coverage percentage (0-100)
    """
    coverage_file = get_project_root() / "coverage.xml"

    if not coverage_file.exists():
        print("⚠️  coverage.xml not found. Run 'make coverage' first.")
        return 0.0

    try:
        tree = ET.parse(coverage_file)
        root = tree.getroot()

        # Coverage.py XML format: <coverage line-rate="0.95" ...>
        line_rate = root.attrib.get('line-rate', '0')
        coverage_pct = float(line_rate) * 100

        return round(coverage_pct, 1)
    except Exception as e:
        print(f"❌ Error parsing coverage.xml: {e}")
        return 0.0


def get_rust_coverage() -> float:
    """
    Get Rust test coverage from cargo tarpaulin.

    Returns:
        Coverage percentage (0-100)

    Note: Requires 'cargo tarpaulin' to be installed.
          Install with: cargo install cargo-tarpaulin
    """
    rust_dir = get_project_root() / "rust"

    try:
        # Check if tarpaulin is installed
        result = subprocess.run(
            ["cargo", "tarpaulin", "--version"],
            capture_output=True,
            text=True,
            cwd=rust_dir
        )

        if result.returncode != 0:
            print("⚠️  cargo-tarpaulin not installed. Install with: cargo install cargo-tarpaulin")
            return 0.0

        # Run tarpaulin
        result = subprocess.run(
            ["cargo", "tarpaulin", "--out", "Xml", "--output-dir", "."],
            capture_output=True,
            text=True,
            cwd=rust_dir,
            timeout=120
        )

        if result.returncode != 0:
            print(f"⚠️  Tarpaulin failed: {result.stderr}")
            return 0.0

        # Parse cobertura.xml (tarpaulin output)
        cobertura_file = rust_dir / "cobertura.xml"
        if not cobertura_file.exists():
            return 0.0

        tree = ET.parse(cobertura_file)
        root = tree.getroot()

        # Cobertura format: <coverage line-rate="0.92" ...>
        line_rate = root.attrib.get('line-rate', '0')
        coverage_pct = float(line_rate) * 100

        return round(coverage_pct, 1)

    except subprocess.TimeoutExpired:
        print("⚠️  Tarpaulin timed out")
        return 0.0
    except Exception as e:
        print(f"⚠️  Error getting Rust coverage: {e}")
        return 0.0


def get_test_vector_parity() -> Tuple[int, int, float]:
    """
    Count test vectors and calculate parity percentage.

    Returns:
        (total_vectors, passing_vectors, parity_percentage)
    """
    vectors_dir = get_project_root() / "test_vectors" / "rust_parity"

    if not vectors_dir.exists():
        return 0, 0, 0.0

    total = 0
    passing = 0

    # Count all JSON files (except SCHEMA.md, README.md)
    for vector_file in vectors_dir.glob("*.json"):
        try:
            with open(vector_file) as f:
                vector = json.load(f)
                total += 1

                if vector.get("rust_status") == "passing":
                    passing += 1
        except Exception as e:
            print(f"⚠️  Error reading {vector_file}: {e}")

    parity_pct = (passing / total * 100) if total > 0 else 0.0
    return total, passing, round(parity_pct, 1)


def count_lines_of_code(file_path: Path) -> int:
    """
    Count lines of code in a file (excluding blanks and comments).

    Args:
        file_path: Path to source file

    Returns:
        Number of lines (excluding blanks and simple comments)
    """
    try:
        with open(file_path) as f:
            lines = f.readlines()

        # Count non-blank, non-comment lines
        code_lines = 0
        for line in lines:
            stripped = line.strip()
            if stripped and not stripped.startswith('#') and not stripped.startswith('//'):
                code_lines += 1

        return code_lines
    except Exception:
        return 0


def get_python_loc() -> int:
    """Get total lines of Python code."""
    root = get_project_root()
    total = 0

    # Count main file
    total += count_lines_of_code(root / "pm_encoder.py")

    # Count test files
    tests_dir = root / "tests"
    if tests_dir.exists():
        for test_file in tests_dir.glob("test_*.py"):
            total += count_lines_of_code(test_file)

    return total


def get_rust_loc() -> int:
    """Get total lines of Rust code."""
    root = get_project_root()
    rust_dir = root / "rust" / "src"
    total = 0

    if rust_dir.exists():
        # Count all .rs files
        for rs_file in rust_dir.rglob("*.rs"):
            total += count_lines_of_code(rs_file)

        # Count test files
        tests_dir = root / "rust" / "tests"
        if tests_dir.exists():
            for test_file in tests_dir.rglob("*.rs"):
                total += count_lines_of_code(test_file)

    return total


def get_interface_parity() -> float:
    """
    Get CLI interface parity percentage by running verify_cli_parity.py.

    Returns:
        Interface parity percentage (0-100)
    """
    root = get_project_root()
    validator_script = root / "scripts" / "verify_cli_parity.py"
    contract_file = root / "test_vectors" / "cli_contract.json"

    if not validator_script.exists():
        print("⚠️  verify_cli_parity.py not found.")
        return 0.0

    if not contract_file.exists():
        print("⚠️  cli_contract.json not found. Run generate_cli_contract.py first.")
        return 0.0

    try:
        # Run the validator and capture output
        result = subprocess.run(
            ["python3", str(validator_script), "--json"],
            capture_output=True,
            text=True,
            cwd=root,
            timeout=60
        )

        # Parse JSON output from research/data/cli_parity.json
        parity_file = root / "research" / "data" / "cli_parity.json"
        if parity_file.exists():
            with open(parity_file) as f:
                data = json.load(f)
                return data.get("metrics", {}).get("interface_parity_percent", 0.0)

        # Fallback: parse from stdout
        for line in result.stdout.split('\n'):
            if "Interface Parity:" in line:
                # Extract percentage from "Interface Parity: 50.0%"
                pct = line.split(':')[1].strip().rstrip('%')
                return float(pct)

        return 0.0

    except subprocess.TimeoutExpired:
        print("⚠️  Interface parity check timed out")
        return 0.0
    except Exception as e:
        print(f"⚠️  Error checking interface parity: {e}")
        return 0.0


def calculate_velocity(csv_file: Path, days: int = 7) -> float:
    """
    Calculate rolling average of vectors passing per day.

    Args:
        csv_file: Path to daily_snapshots.csv
        days: Number of days for rolling average

    Returns:
        Average vectors passing per day
    """
    if not csv_file.exists():
        return 0.0

    try:
        with open(csv_file) as f:
            reader = csv.DictReader(f)
            rows = list(reader)

        if len(rows) < 2:
            return 0.0

        # Get last N days
        recent_rows = rows[-days:] if len(rows) >= days else rows

        # Calculate change in passing vectors
        first_passing = int(recent_rows[0]['vectors_passing'])
        last_passing = int(recent_rows[-1]['vectors_passing'])
        days_elapsed = len(recent_rows)

        velocity = (last_passing - first_passing) / days_elapsed
        return round(velocity, 2)

    except Exception as e:
        print(f"⚠️  Error calculating velocity: {e}")
        return 0.0


def append_snapshot() -> None:
    """Collect metrics and append to daily_snapshots.csv."""
    root = get_project_root()
    data_dir = root / "research" / "data"
    data_dir.mkdir(parents=True, exist_ok=True)

    csv_file = data_dir / "daily_snapshots.csv"

    # Collect metrics
    print("📊 Collecting metrics...")

    today = date.today().isoformat()
    python_cov = get_python_coverage()
    rust_cov = get_rust_coverage()
    vectors_total, vectors_passing, parity_pct = get_test_vector_parity()
    interface_parity = get_interface_parity()
    python_loc = get_python_loc()
    rust_loc = get_rust_loc()
    velocity = calculate_velocity(csv_file)

    # Prepare row
    row = {
        'date': today,
        'python_coverage': python_cov,
        'rust_coverage': rust_cov,
        'parity_pct': parity_pct,
        'interface_parity': interface_parity,
        'python_loc': python_loc,
        'rust_loc': rust_loc,
        'vectors_total': vectors_total,
        'vectors_passing': vectors_passing,
        'velocity': velocity
    }

    # Check if we've already recorded today
    existing_dates = set()
    if csv_file.exists():
        with open(csv_file) as f:
            reader = csv.DictReader(f)
            existing_dates = {row['date'] for row in reader}

    if today in existing_dates:
        print(f"⚠️  Snapshot for {today} already exists. Skipping.")
        return

    # Write header if new file
    write_header = not csv_file.exists()

    # Append row
    with open(csv_file, 'a', newline='') as f:
        fieldnames = ['date', 'python_coverage', 'rust_coverage', 'parity_pct',
                     'interface_parity', 'python_loc', 'rust_loc',
                     'vectors_total', 'vectors_passing', 'velocity']
        writer = csv.DictWriter(f, fieldnames=fieldnames)

        if write_header:
            writer.writeheader()

        writer.writerow(row)

    # Display results
    print("\n✅ Snapshot recorded:")
    print(f"   Date: {today}")
    print(f"   Python Coverage: {python_cov}%")
    print(f"   Rust Coverage: {rust_cov}%")
    print(f"   Logic Parity: {parity_pct}% ({vectors_passing}/{vectors_total} vectors)")
    print(f"   Interface Parity: {interface_parity}% (CLI contract)")
    print(f"   Python LOC: {python_loc}")
    print(f"   Rust LOC: {rust_loc}")
    print(f"   Velocity: {velocity} vectors/day (7-day avg)")
    print(f"\n📁 Saved to: {csv_file}")


def main():
    """Main entry point."""
    print("🔬 The Twins Research - Daily Metrics Tracker\n")
    append_snapshot()


if __name__ == "__main__":
    main()
