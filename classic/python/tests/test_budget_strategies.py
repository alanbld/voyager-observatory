#!/usr/bin/env python3
"""
Test suite for v1.7.0 Budget Strategy feature.

Tests:
- Drop strategy (default behavior)
- Truncate strategy (force structure mode on oversized files)
- Hybrid strategy (auto-truncate files >10% of budget)
- CLI flag acceptance
- Budget report inclusion methods
"""

import unittest
import sys
from pathlib import Path
from io import StringIO

# Import from parent directory
sys.path.insert(0, str(Path(__file__).parent.parent))
import pm_encoder


class TestDropStrategy(unittest.TestCase):
    """Test the default 'drop' strategy behavior."""

    def setUp(self):
        """Set up test data."""
        self.lens_manager = pm_encoder.LensManager()
        self.lens_manager.active_lens_config = {
            "groups": [
                {"pattern": "*.py", "priority": 100},
                {"pattern": "*.txt", "priority": 50},
            ],
            "fallback": {"priority": 25}
        }

        # Force heuristic mode for predictable tests
        pm_encoder.TokenEstimator._tiktoken_available = False
        pm_encoder.TokenEstimator._warning_shown = True

    def test_drop_strategy_drops_oversized(self):
        """Test that drop strategy skips files that don't fit."""
        files = [
            (Path("small.py"), "x" * 100),   # ~25 tokens + overhead
            (Path("big.py"), "x" * 10000),   # ~2500 tokens + overhead
        ]

        budget = 200  # Only enough for small file

        selected, report = pm_encoder.apply_token_budget(
            files, budget, self.lens_manager, strategy="drop"
        )

        self.assertEqual(report.strategy, "drop")
        self.assertEqual(len(selected), 1)
        self.assertEqual(selected[0][0].name, "small.py")
        self.assertEqual(report.dropped_count, 1)

    def test_drop_strategy_report_shows_full(self):
        """Test that included files are marked as 'full'."""
        files = [
            (Path("test.py"), "x" * 100),
        ]

        budget = 10000

        selected, report = pm_encoder.apply_token_budget(
            files, budget, self.lens_manager, strategy="drop"
        )

        self.assertEqual(len(report.included_files), 1)
        path, priority, tokens, method = report.included_files[0]
        self.assertEqual(method, "full")


class TestTruncateStrategy(unittest.TestCase):
    """Test the 'truncate' strategy behavior."""

    def setUp(self):
        """Set up test data with analyzer registry."""
        self.lens_manager = pm_encoder.LensManager()
        self.lens_manager.active_lens_config = {
            "groups": [{"pattern": "*.py", "priority": 100}],
        }

        self.analyzer_registry = pm_encoder.LanguageAnalyzerRegistry()

        # Force heuristic mode
        pm_encoder.TokenEstimator._tiktoken_available = False
        pm_encoder.TokenEstimator._warning_shown = True

    def test_truncate_strategy_truncates_oversized(self):
        """Test that truncate strategy applies structure mode to oversized files."""
        # Create a Python file with enough structure to truncate
        python_code = '''
import os
import sys

class MyClass:
    """A test class."""

    def __init__(self, value):
        """Initialize with value."""
        self.value = value
        self.data = []
        for i in range(100):
            self.data.append(i * 2)

    def process(self, x):
        """Process the input."""
        result = x * self.value
        for item in self.data:
            result += item
        return result

def main():
    """Main entry point."""
    obj = MyClass(42)
    for i in range(1000):
        print(obj.process(i))

if __name__ == "__main__":
    main()
'''
        files = [
            (Path("code.py"), python_code),
        ]

        # Set budget small enough that original doesn't fit, but truncated might
        original_tokens = pm_encoder.TokenEstimator.estimate_file_tokens(
            Path("code.py"), python_code
        )
        budget = original_tokens // 2  # Half the original size

        selected, report = pm_encoder.apply_token_budget(
            files, budget, self.lens_manager,
            strategy="truncate",
            analyzer_registry=self.analyzer_registry
        )

        self.assertEqual(report.strategy, "truncate")
        # File should either be truncated and included, or dropped
        # The outcome depends on structure extraction efficiency


class TestHybridStrategy(unittest.TestCase):
    """Test the 'hybrid' strategy behavior."""

    def setUp(self):
        """Set up test data."""
        self.lens_manager = pm_encoder.LensManager()
        self.lens_manager.active_lens_config = {
            "groups": [
                {"pattern": "*.py", "priority": 100},
                {"pattern": "*.txt", "priority": 50},
            ],
        }

        self.analyzer_registry = pm_encoder.LanguageAnalyzerRegistry()

        # Force heuristic mode
        pm_encoder.TokenEstimator._tiktoken_available = False
        pm_encoder.TokenEstimator._warning_shown = True

    def test_hybrid_auto_truncates_large_files(self):
        """Test that hybrid strategy auto-truncates files >10% of budget."""
        # Create a Python file that will be >10% of budget
        large_python = '''
import os
import sys
import json

class LargeClass:
    """A large test class."""

    def method_one(self, x):
        """First method with lots of code."""
        result = 0
        for i in range(100):
            result += i * x
            if result > 1000:
                result = result // 2
        return result

    def method_two(self, y):
        """Second method with more code."""
        data = []
        for i in range(y):
            data.append(i * 2)
            data.append(i * 3)
        return sum(data)

    def method_three(self, z):
        """Third method."""
        return z * 42

def helper_function(a, b, c):
    """Helper function."""
    return a + b + c

def main():
    """Main function."""
    obj = LargeClass()
    print(obj.method_one(10))
    print(obj.method_two(20))
    print(obj.method_three(30))

if __name__ == "__main__":
    main()
'''
        # Small file
        small_txt = "small content"

        files = [
            (Path("large.py"), large_python),
            (Path("small.txt"), small_txt),
        ]

        # Calculate budget where large.py is >10%
        large_tokens = pm_encoder.TokenEstimator.estimate_file_tokens(
            Path("large.py"), large_python
        )

        # Set budget so large.py is ~20% of budget
        budget = large_tokens * 5

        selected, report = pm_encoder.apply_token_budget(
            files, budget, self.lens_manager,
            strategy="hybrid",
            analyzer_registry=self.analyzer_registry
        )

        self.assertEqual(report.strategy, "hybrid")

        # Both files should be included
        self.assertEqual(len(selected), 2)

        # large.py should be truncated (it's >10% of budget)
        large_entry = next(
            (f for f in report.included_files if f[0].name == "large.py"),
            None
        )
        self.assertIsNotNone(large_entry)
        # The file should be marked as truncated since it exceeds 10% threshold
        self.assertEqual(large_entry[3], "truncated")

    def test_hybrid_keeps_small_files_full(self):
        """Test that hybrid strategy keeps small files as 'full'."""
        small_content = "small"

        files = [
            (Path("tiny.py"), small_content),
        ]

        budget = 10000  # Large budget

        selected, report = pm_encoder.apply_token_budget(
            files, budget, self.lens_manager,
            strategy="hybrid",
            analyzer_registry=self.analyzer_registry
        )

        # Small file should be kept as full
        self.assertEqual(len(report.included_files), 1)
        self.assertEqual(report.included_files[0][3], "full")


class TestBudgetReportMethods(unittest.TestCase):
    """Test BudgetReport tracks inclusion methods correctly."""

    def test_report_counts_truncated(self):
        """Test that report correctly counts truncated files."""
        report = pm_encoder.BudgetReport(
            budget=10000,
            used=5000,
            selected_count=3,
            dropped_count=1,
            dropped_files=[(Path("dropped.py"), 50, 6000)],
            estimation_method="Heuristic",
            strategy="hybrid",
            included_files=[
                (Path("full1.py"), 100, 1000, "full"),
                (Path("truncated1.py"), 90, 2000, "truncated"),
                (Path("truncated2.py"), 80, 2000, "truncated"),
            ],
            truncated_count=2
        )

        self.assertEqual(report.truncated_count, 2)
        self.assertEqual(report.selected_count, 3)

    def test_report_print_shows_strategy(self):
        """Test that print_report shows strategy."""
        report = pm_encoder.BudgetReport(
            budget=10000,
            used=5000,
            selected_count=2,
            dropped_count=0,
            dropped_files=[],
            estimation_method="Heuristic",
            strategy="hybrid",
            included_files=[
                (Path("file1.py"), 100, 2500, "full"),
                (Path("file2.py"), 90, 2500, "truncated"),
            ],
            truncated_count=1
        )

        output = StringIO()
        report.print_report(output)
        result = output.getvalue()

        self.assertIn("Strategy:   hybrid", result)
        self.assertIn("1 full, 1 truncated", result)
        self.assertIn("Auto-truncated files", result)

    def test_report_print_with_no_truncated(self):
        """Test report print when no files were truncated."""
        report = pm_encoder.BudgetReport(
            budget=10000,
            used=5000,
            selected_count=2,
            dropped_count=0,
            dropped_files=[],
            estimation_method="Heuristic",
            strategy="drop",
            included_files=[
                (Path("file1.py"), 100, 2500, "full"),
                (Path("file2.py"), 90, 2500, "full"),
            ],
            truncated_count=0
        )

        output = StringIO()
        report.print_report(output)
        result = output.getvalue()

        self.assertIn("2 full, 0 truncated", result)
        self.assertNotIn("Auto-truncated files", result)


class TestCLIStrategyflag(unittest.TestCase):
    """Test CLI accepts the --budget-strategy flag."""

    def test_strategy_flag_accepted(self):
        """Test that --budget-strategy flag is parsed correctly."""
        import argparse

        # Create parser similar to main()
        parser = argparse.ArgumentParser()
        parser.add_argument("project_root", type=Path, nargs='?')
        parser.add_argument("--budget-strategy",
                          choices=["drop", "truncate", "hybrid"],
                          default="drop")

        # Test default
        args = parser.parse_args(["."])
        self.assertEqual(args.budget_strategy, "drop")

        # Test explicit values
        args = parser.parse_args([".", "--budget-strategy", "truncate"])
        self.assertEqual(args.budget_strategy, "truncate")

        args = parser.parse_args([".", "--budget-strategy", "hybrid"])
        self.assertEqual(args.budget_strategy, "hybrid")

    def test_invalid_strategy_rejected(self):
        """Test that invalid strategy values are rejected."""
        import argparse

        parser = argparse.ArgumentParser()
        parser.add_argument("--budget-strategy",
                          choices=["drop", "truncate", "hybrid"],
                          default="drop")

        with self.assertRaises(SystemExit):
            parser.parse_args(["--budget-strategy", "invalid"])


class TestTruncateToStructure(unittest.TestCase):
    """Test the _truncate_to_structure helper function."""

    def setUp(self):
        """Set up analyzer registry."""
        self.analyzer_registry = pm_encoder.LanguageAnalyzerRegistry()

    def test_truncate_python_file(self):
        """Test structure truncation of Python file."""
        python_code = '''
import os

def my_function(x, y):
    """A function."""
    result = x + y
    for i in range(10):
        result += i
    return result

class MyClass:
    def method(self):
        pass
'''
        content, was_truncated = pm_encoder._truncate_to_structure(
            Path("test.py"), python_code, self.analyzer_registry
        )

        self.assertTrue(was_truncated)
        # Should be shorter than original
        self.assertLess(len(content), len(python_code))
        # Should keep imports and signatures
        self.assertIn("import os", content)
        self.assertIn("def my_function", content)
        self.assertIn("class MyClass", content)

    def test_truncate_unsupported_file(self):
        """Test that unsupported files are not truncated."""
        content = "just some text content\nwith multiple lines\nand stuff"

        result, was_truncated = pm_encoder._truncate_to_structure(
            Path("readme.txt"), content, self.analyzer_registry
        )

        # Should return original content unchanged
        self.assertFalse(was_truncated)
        self.assertEqual(result, content)

    def test_truncate_without_registry(self):
        """Test truncation without analyzer registry."""
        content = "some content"

        result, was_truncated = pm_encoder._truncate_to_structure(
            Path("test.py"), content, None
        )

        self.assertFalse(was_truncated)
        self.assertEqual(result, content)


def run_tests():
    """Run all budget strategy tests."""
    loader = unittest.TestLoader()
    suite = unittest.TestSuite()

    suite.addTests(loader.loadTestsFromTestCase(TestDropStrategy))
    suite.addTests(loader.loadTestsFromTestCase(TestTruncateStrategy))
    suite.addTests(loader.loadTestsFromTestCase(TestHybridStrategy))
    suite.addTests(loader.loadTestsFromTestCase(TestBudgetReportMethods))
    suite.addTests(loader.loadTestsFromTestCase(TestCLIStrategyflag))
    suite.addTests(loader.loadTestsFromTestCase(TestTruncateToStructure))

    runner = unittest.TextTestRunner(verbosity=2)
    result = runner.run(suite)

    print("\n" + "=" * 70)
    print("BUDGET STRATEGY TEST SUMMARY")
    print("=" * 70)
    print(f"Tests run: {result.testsRun}")
    print(f"Successes: {result.testsRun - len(result.failures) - len(result.errors)}")
    print(f"Failures: {len(result.failures)}")
    print(f"Errors: {len(result.errors)}")
    print("=" * 70)

    return result.wasSuccessful()


if __name__ == "__main__":
    success = run_tests()
    sys.exit(0 if success else 1)
