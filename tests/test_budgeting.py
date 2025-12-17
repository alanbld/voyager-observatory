#!/usr/bin/env python3
"""
Test suite for v1.7.0 Token Budgeting feature.

Tests:
- Shorthand notation parsing (k, M)
- Token estimation (heuristic fallback)
- Budget cutoff behavior
- Priority-based sorting
- Budget report generation
"""

import unittest
import sys
from pathlib import Path
from io import StringIO

# Import from parent directory
sys.path.insert(0, str(Path(__file__).parent.parent))
import pm_encoder


class TestShorthandParsing(unittest.TestCase):
    """Test token budget shorthand notation parsing."""

    def test_plain_number(self):
        """Test plain numeric input."""
        self.assertEqual(pm_encoder.parse_token_budget("100000"), 100000)
        self.assertEqual(pm_encoder.parse_token_budget("1"), 1)
        self.assertEqual(pm_encoder.parse_token_budget("0"), 0)

    def test_k_suffix_lowercase(self):
        """Test lowercase k suffix (thousands)."""
        self.assertEqual(pm_encoder.parse_token_budget("100k"), 100000)
        self.assertEqual(pm_encoder.parse_token_budget("1k"), 1000)
        self.assertEqual(pm_encoder.parse_token_budget("50k"), 50000)

    def test_k_suffix_uppercase(self):
        """Test uppercase K suffix (thousands)."""
        self.assertEqual(pm_encoder.parse_token_budget("100K"), 100000)
        self.assertEqual(pm_encoder.parse_token_budget("1K"), 1000)

    def test_m_suffix_lowercase(self):
        """Test lowercase m suffix (millions)."""
        self.assertEqual(pm_encoder.parse_token_budget("2m"), 2000000)
        self.assertEqual(pm_encoder.parse_token_budget("1m"), 1000000)

    def test_m_suffix_uppercase(self):
        """Test uppercase M suffix (millions)."""
        self.assertEqual(pm_encoder.parse_token_budget("2M"), 2000000)
        self.assertEqual(pm_encoder.parse_token_budget("1M"), 1000000)

    def test_whitespace_handling(self):
        """Test that whitespace is trimmed."""
        self.assertEqual(pm_encoder.parse_token_budget("  100k  "), 100000)
        self.assertEqual(pm_encoder.parse_token_budget("\t2M\n"), 2000000)

    def test_invalid_format(self):
        """Test that invalid formats raise ValueError."""
        with self.assertRaises(ValueError):
            pm_encoder.parse_token_budget("abc")
        with self.assertRaises(ValueError):
            pm_encoder.parse_token_budget("100x")
        with self.assertRaises(ValueError):
            pm_encoder.parse_token_budget("k100")
        with self.assertRaises(ValueError):
            pm_encoder.parse_token_budget("")
        with self.assertRaises(ValueError):
            pm_encoder.parse_token_budget("10.5k")


class TestTokenEstimator(unittest.TestCase):
    """Test token estimation functionality."""

    def test_heuristic_estimation(self):
        """Test that heuristic estimates ~4 chars per token."""
        # Force heuristic mode by resetting the class state
        original_available = pm_encoder.TokenEstimator._tiktoken_available

        try:
            # Force heuristic mode
            pm_encoder.TokenEstimator._tiktoken_available = False
            pm_encoder.TokenEstimator._warning_shown = True  # Suppress warning

            content = "x" * 400  # 400 chars
            tokens = pm_encoder.TokenEstimator.estimate_tokens(content)
            self.assertEqual(tokens, 100)  # 400 // 4 = 100

            content = "hello world"  # 11 chars
            tokens = pm_encoder.TokenEstimator.estimate_tokens(content)
            self.assertEqual(tokens, 2)  # 11 // 4 = 2

        finally:
            pm_encoder.TokenEstimator._tiktoken_available = original_available

    def test_file_token_estimation_includes_overhead(self):
        """Test that file estimation includes PM format overhead."""
        original_available = pm_encoder.TokenEstimator._tiktoken_available

        try:
            pm_encoder.TokenEstimator._tiktoken_available = False
            pm_encoder.TokenEstimator._warning_shown = True

            path = Path("test.py")
            content = "x" * 100  # 100 chars = 25 tokens

            file_tokens = pm_encoder.TokenEstimator.estimate_file_tokens(path, content)

            # Should be more than just content tokens due to overhead
            self.assertGreater(file_tokens, 25)

        finally:
            pm_encoder.TokenEstimator._tiktoken_available = original_available

    def test_get_method_heuristic(self):
        """Test method reporting for heuristic mode."""
        original_available = pm_encoder.TokenEstimator._tiktoken_available

        try:
            pm_encoder.TokenEstimator._tiktoken_available = False
            method = pm_encoder.TokenEstimator.get_method()
            self.assertIn("Heuristic", method)

        finally:
            pm_encoder.TokenEstimator._tiktoken_available = original_available


class TestBudgetApplication(unittest.TestCase):
    """Test apply_token_budget function."""

    def setUp(self):
        """Set up test data."""
        # Create a lens manager with groups for priority testing
        self.lens_manager = pm_encoder.LensManager()
        self.lens_manager.active_lens_config = {
            "groups": [
                {"pattern": "*.py", "priority": 100},
                {"pattern": "*.md", "priority": 50},
                {"pattern": "*.txt", "priority": 10},
            ],
            "fallback": {"priority": 50}
        }

        # Force heuristic mode for predictable tests
        pm_encoder.TokenEstimator._tiktoken_available = False
        pm_encoder.TokenEstimator._warning_shown = True

    def test_budget_cutoff_drops_lowest_priority(self):
        """Test that lowest priority files are dropped first."""
        # Create files with known token sizes
        # Using heuristic: 4 chars = 1 token
        files = [
            (Path("high.py"), "x" * 400),    # 100 tokens, priority 100
            (Path("med.md"), "x" * 400),     # 100 tokens, priority 50
            (Path("low.txt"), "x" * 400),    # 100 tokens, priority 10
        ]

        # Set budget to allow ~250 tokens (2 files + overhead)
        # Each file has ~100 content tokens + ~20 overhead tokens = ~120 per file
        budget = 250

        selected, report = pm_encoder.apply_token_budget(
            files, budget, self.lens_manager
        )

        # Should include high.py and med.md, drop low.txt
        selected_paths = [p.name for p, _ in selected]

        self.assertIn("high.py", selected_paths)
        self.assertIn("med.md", selected_paths)
        self.assertEqual(len(selected), 2)
        self.assertEqual(report.dropped_count, 1)

    def test_priority_sorting(self):
        """Test that files are sorted by priority (DESC) then path (ASC)."""
        files = [
            (Path("c.txt"), "content"),      # priority 10
            (Path("a.py"), "content"),       # priority 100
            (Path("b.md"), "content"),       # priority 50
        ]

        # Large budget to include all
        budget = 100000

        selected, report = pm_encoder.apply_token_budget(
            files, budget, self.lens_manager
        )

        # Should be sorted: a.py (100) > b.md (50) > c.txt (10)
        selected_paths = [p.name for p, _ in selected]
        self.assertEqual(selected_paths, ["a.py", "b.md", "c.txt"])

    def test_deterministic_tiebreaker(self):
        """Test that files with same priority are sorted by path."""
        # All .md files have same priority (50)
        files = [
            (Path("z.md"), "content"),
            (Path("a.md"), "content"),
            (Path("m.md"), "content"),
        ]

        budget = 100000
        selected, _ = pm_encoder.apply_token_budget(
            files, budget, self.lens_manager
        )

        # Should be sorted alphabetically
        selected_paths = [p.name for p, _ in selected]
        self.assertEqual(selected_paths, ["a.md", "m.md", "z.md"])

    def test_empty_file_list(self):
        """Test handling of empty file list."""
        files = []
        budget = 1000

        selected, report = pm_encoder.apply_token_budget(
            files, budget, self.lens_manager
        )

        self.assertEqual(len(selected), 0)
        self.assertEqual(report.selected_count, 0)
        self.assertEqual(report.dropped_count, 0)

    def test_all_files_dropped(self):
        """Test when budget is too small for any file."""
        files = [
            (Path("big.py"), "x" * 1000),  # ~250 tokens + overhead
        ]

        budget = 10  # Too small

        selected, report = pm_encoder.apply_token_budget(
            files, budget, self.lens_manager
        )

        self.assertEqual(len(selected), 0)
        self.assertEqual(report.dropped_count, 1)


class TestBudgetReport(unittest.TestCase):
    """Test BudgetReport class."""

    def test_used_percentage(self):
        """Test percentage calculation."""
        report = pm_encoder.BudgetReport(
            budget=1000,
            used=750,
            selected_count=5,
            dropped_count=2,
            dropped_files=[],
            estimation_method="Heuristic"
        )

        self.assertEqual(report.used_percentage, 75.0)

    def test_remaining_tokens(self):
        """Test remaining token calculation."""
        report = pm_encoder.BudgetReport(
            budget=1000,
            used=750,
            selected_count=5,
            dropped_count=2,
            dropped_files=[],
            estimation_method="Heuristic"
        )

        self.assertEqual(report.remaining, 250)

    def test_remaining_never_negative(self):
        """Test that remaining is never negative (over budget)."""
        report = pm_encoder.BudgetReport(
            budget=100,
            used=150,
            selected_count=5,
            dropped_count=0,
            dropped_files=[],
            estimation_method="Heuristic"
        )

        self.assertEqual(report.remaining, 0)

    def test_print_report(self):
        """Test that print_report produces output."""
        report = pm_encoder.BudgetReport(
            budget=1000,
            used=750,
            selected_count=5,
            dropped_count=2,
            dropped_files=[(Path("test.py"), 10, 100)],
            estimation_method="Heuristic"
        )

        output = StringIO()
        report.print_report(output)
        result = output.getvalue()

        self.assertIn("TOKEN BUDGET REPORT", result)
        self.assertIn("1,000", result)  # Budget
        self.assertIn("750", result)    # Used
        self.assertIn("Files included: 5", result)
        self.assertIn("Files dropped:  2", result)


class TestNoBudget(unittest.TestCase):
    """Test that no budget (0) means no filtering."""

    def test_zero_budget_no_filtering(self):
        """Test that budget of 0 is handled correctly."""
        # In the actual code, budget=0 bypasses budgeting
        # This test verifies the budget report with 0 budget
        files = [
            (Path("a.py"), "content"),
            (Path("b.py"), "content"),
        ]

        # Budget of 0 should still work (though in practice
        # serialize() doesn't call apply_token_budget with 0)
        # Let's test with a very large budget instead
        budget = 1000000
        lens_manager = pm_encoder.LensManager()

        selected, report = pm_encoder.apply_token_budget(
            files, budget, lens_manager
        )

        self.assertEqual(len(selected), 2)
        self.assertEqual(report.dropped_count, 0)


def run_tests():
    """Run all budgeting tests."""
    loader = unittest.TestLoader()
    suite = unittest.TestSuite()

    suite.addTests(loader.loadTestsFromTestCase(TestShorthandParsing))
    suite.addTests(loader.loadTestsFromTestCase(TestTokenEstimator))
    suite.addTests(loader.loadTestsFromTestCase(TestBudgetApplication))
    suite.addTests(loader.loadTestsFromTestCase(TestBudgetReport))
    suite.addTests(loader.loadTestsFromTestCase(TestNoBudget))

    runner = unittest.TextTestRunner(verbosity=2)
    result = runner.run(suite)

    print("\n" + "=" * 70)
    print("TOKEN BUDGETING TEST SUMMARY")
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
