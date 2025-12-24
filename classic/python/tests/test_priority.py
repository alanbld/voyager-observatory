#!/usr/bin/env python3
"""
Test suite for v1.7.0 Priority Groups feature.

Tests:
- Priority resolution logic
- Backward compatibility with v1.6.0 lenses
- Fallback priority for unmatched files
- Highest priority wins when multiple groups match
- Group configuration retrieval
"""

import unittest
import sys
from pathlib import Path

# Import from parent directory
sys.path.insert(0, str(Path(__file__).parent.parent))
import pm_encoder


class TestPriorityResolution(unittest.TestCase):
    """Test priority resolution logic for files."""

    def setUp(self):
        """Set up test lens configurations."""
        # NOTE: The spec says "Return HIGHEST priority among all matches"
        # So if a file matches both *.py (80) and tests/** (10), it gets 80
        # To override, ensure specific patterns have HIGHER priority
        self.lens_with_groups = {
            "description": "Test lens with priority groups",
            "groups": [
                {"name": "core", "pattern": "src/core/**/*.py", "priority": 100},
                {"name": "python", "pattern": "*.py", "priority": 80},
                {"name": "config", "pattern": "*.json", "priority": 60},
                # Tests directory - use dedicated test file pattern
                {"name": "test_files", "pattern": "tests/**/*.py", "priority": 10},
                {"name": "test_dir", "pattern": "tests/**", "priority": 5},
            ],
            "fallback": {"priority": 50}
        }

        self.lens_without_groups = {
            "description": "Legacy lens without groups",
            "include": ["*.py"],
            "exclude": ["tests/**"],
            "truncate": 500
        }

    def test_priority_resolution_basic(self):
        """Test basic priority resolution for different file types."""
        manager = pm_encoder.LensManager()
        manager.active_lens_config = self.lens_with_groups

        # Python file should match "*.py" pattern with priority 80
        self.assertEqual(manager.get_file_priority(Path("main.py")), 80)

        # JSON file should match "*.json" pattern with priority 60
        self.assertEqual(manager.get_file_priority(Path("config.json")), 60)

        # Test file matches BOTH "*.py" (80) and "tests/**/*.py" (10)
        # Per spec: "Return HIGHEST priority among all matches"
        # So test file gets 80, not 10
        self.assertEqual(manager.get_file_priority(Path("tests/test_main.py")), 80)

    def test_specific_pattern_override(self):
        """Test that specific patterns can override generic ones with HIGHER priority."""
        manager = pm_encoder.LensManager()
        # To make test files lower priority than regular Python, use a config
        # where the test pattern has LOWER priority AND is the only match
        manager.active_lens_config = {
            "groups": [
                # Use directory patterns that don't overlap with extension patterns
                {"pattern": "src/**/*.py", "priority": 100},  # Only src/ Python files
                {"pattern": "tests/**", "priority": 10},       # Everything in tests/
            ],
            "fallback": {"priority": 50}
        }

        # src/ Python files get 100
        self.assertEqual(manager.get_file_priority(Path("src/main.py")), 100)

        # tests/ files get 10 (no overlap with src/**/*.py)
        self.assertEqual(manager.get_file_priority(Path("tests/test_main.py")), 10)

        # Other files get fallback 50
        self.assertEqual(manager.get_file_priority(Path("lib/utils.py")), 50)

    def test_highest_priority_wins(self):
        """Test that when a file matches multiple groups, highest priority wins."""
        manager = pm_encoder.LensManager()
        manager.active_lens_config = self.lens_with_groups

        # File in src/core/ matches both "src/core/**/*.py" (100) and "*.py" (80)
        # Should return 100 (highest)
        priority = manager.get_file_priority(Path("src/core/main.py"))
        self.assertEqual(priority, 100)

        # Also test with different ordering in groups
        lens_reversed = {
            "groups": [
                {"pattern": "*.py", "priority": 80},
                {"pattern": "src/core/**/*.py", "priority": 100},  # Defined later
            ]
        }
        manager.active_lens_config = lens_reversed
        priority = manager.get_file_priority(Path("src/core/main.py"))
        self.assertEqual(priority, 100)  # Still highest wins

    def test_fallback_priority(self):
        """Test that files matching no groups get fallback priority."""
        manager = pm_encoder.LensManager()
        manager.active_lens_config = self.lens_with_groups

        # Unknown file type should get fallback priority (50)
        self.assertEqual(manager.get_file_priority(Path("unknown.xyz")), 50)
        self.assertEqual(manager.get_file_priority(Path("random/file.txt")), 50)

    def test_fallback_default_is_50(self):
        """Test that fallback defaults to 50 if not specified."""
        manager = pm_encoder.LensManager()
        manager.active_lens_config = {
            "groups": [
                {"pattern": "*.py", "priority": 100}
            ]
            # No fallback specified
        }

        # Non-matching file should get default fallback of 50
        self.assertEqual(manager.get_file_priority(Path("unknown.txt")), 50)

    def test_backward_compatibility_no_groups(self):
        """Test that lenses without groups return default priority 50."""
        manager = pm_encoder.LensManager()
        manager.active_lens_config = self.lens_without_groups

        # All files should get priority 50 when no groups defined
        self.assertEqual(manager.get_file_priority(Path("main.py")), 50)
        self.assertEqual(manager.get_file_priority(Path("config.json")), 50)
        self.assertEqual(manager.get_file_priority(Path("random.txt")), 50)

    def test_backward_compatibility_empty_config(self):
        """Test with empty or None config."""
        manager = pm_encoder.LensManager()

        # No active config
        self.assertEqual(manager.get_file_priority(Path("any.py")), 50)

        # Empty config
        manager.active_lens_config = {}
        self.assertEqual(manager.get_file_priority(Path("any.py")), 50)

    def test_priority_with_explicit_config(self):
        """Test get_file_priority with explicit config parameter."""
        manager = pm_encoder.LensManager()

        custom_config = {
            "groups": [
                {"pattern": "*.rs", "priority": 200},
                {"pattern": "*.py", "priority": 150},
            ],
            "fallback": {"priority": 25}
        }

        # Should use explicit config, not active_lens_config
        self.assertEqual(manager.get_file_priority(Path("main.rs"), custom_config), 200)
        self.assertEqual(manager.get_file_priority(Path("main.py"), custom_config), 150)
        self.assertEqual(manager.get_file_priority(Path("other.txt"), custom_config), 25)


class TestGroupConfiguration(unittest.TestCase):
    """Test group configuration retrieval."""

    def setUp(self):
        """Set up test lens with detailed group configs."""
        self.lens_config = {
            "groups": [
                {"name": "core", "pattern": "src/**/*.py", "priority": 100, "truncate_mode": "structure"},
                {"name": "config", "pattern": "*.json", "priority": 80, "truncate_mode": "smart", "truncate": 200},
                {"name": "tests", "pattern": "tests/**", "priority": 20, "truncate_mode": "simple"},
            ],
            "fallback": {"priority": 50, "truncate_mode": "smart", "truncate": 500}
        }

    def test_get_file_group_config_match(self):
        """Test getting group config for matching files."""
        manager = pm_encoder.LensManager()
        manager.active_lens_config = self.lens_config

        # Python file in src/ should get core group config
        config = manager.get_file_group_config(Path("src/main.py"))
        self.assertEqual(config["priority"], 100)
        self.assertEqual(config["truncate_mode"], "structure")

        # JSON file should get config group
        config = manager.get_file_group_config(Path("settings.json"))
        self.assertEqual(config["priority"], 80)
        self.assertEqual(config["truncate_mode"], "smart")
        self.assertEqual(config["truncate"], 200)

    def test_get_file_group_config_fallback(self):
        """Test getting fallback config for non-matching files."""
        manager = pm_encoder.LensManager()
        manager.active_lens_config = self.lens_config

        # Unknown file type should get fallback config
        config = manager.get_file_group_config(Path("unknown.xyz"))
        self.assertEqual(config["priority"], 50)
        self.assertEqual(config["truncate_mode"], "smart")
        self.assertEqual(config["truncate"], 500)

    def test_get_file_group_config_no_groups(self):
        """Test getting config when no groups defined."""
        manager = pm_encoder.LensManager()
        manager.active_lens_config = {"description": "Legacy lens"}

        # Should return default priority dict
        config = manager.get_file_group_config(Path("any.py"))
        self.assertEqual(config["priority"], 50)


class TestBuiltInArchitectureLens(unittest.TestCase):
    """Test the built-in architecture lens has proper groups."""

    def test_architecture_lens_has_groups(self):
        """Test that architecture lens includes groups."""
        arch_lens = pm_encoder.LensManager.BUILT_IN_LENSES["architecture"]

        self.assertIn("groups", arch_lens)
        self.assertIsInstance(arch_lens["groups"], list)
        self.assertGreater(len(arch_lens["groups"]), 0)

    def test_architecture_lens_has_fallback(self):
        """Test that architecture lens has fallback config."""
        arch_lens = pm_encoder.LensManager.BUILT_IN_LENSES["architecture"]

        self.assertIn("fallback", arch_lens)
        self.assertIn("priority", arch_lens["fallback"])
        self.assertEqual(arch_lens["fallback"]["priority"], 50)

    def test_architecture_lens_priority_values(self):
        """Test architecture lens priority values are reasonable."""
        manager = pm_encoder.LensManager()
        base_config = {
            "ignore_patterns": [],
            "include_patterns": [],
            "sort_by": "name",
            "sort_order": "asc"
        }
        manager.apply_lens("architecture", base_config)

        # Python files should have high priority
        py_priority = manager.get_file_priority(Path("main.py"))
        self.assertGreaterEqual(py_priority, 80)

        # Rust files should have high priority
        rs_priority = manager.get_file_priority(Path("lib.rs"))
        self.assertGreaterEqual(rs_priority, 80)

        # Config files should have medium-high priority
        toml_priority = manager.get_file_priority(Path("Cargo.toml"))
        self.assertGreaterEqual(toml_priority, 70)

    def test_debug_lens_no_groups(self):
        """Test that debug lens (legacy) has no groups - backward compat."""
        debug_lens = pm_encoder.LensManager.BUILT_IN_LENSES["debug"]

        # Debug lens should not have groups (backward compatible)
        self.assertNotIn("groups", debug_lens)


class TestArbitraryPriorityValues(unittest.TestCase):
    """Test that arbitrary integer priority values work (v1.7.0 spec)."""

    def test_negative_priority(self):
        """Test negative priority values work."""
        manager = pm_encoder.LensManager()
        manager.active_lens_config = {
            "groups": [
                {"pattern": "*.log", "priority": -100},  # Deprioritize logs
                {"pattern": "*.py", "priority": 100},
            ],
            "fallback": {"priority": 0}
        }

        self.assertEqual(manager.get_file_priority(Path("debug.log")), -100)
        self.assertEqual(manager.get_file_priority(Path("main.py")), 100)
        self.assertEqual(manager.get_file_priority(Path("other.txt")), 0)

    def test_large_priority_values(self):
        """Test large priority values work (beyond 0-100 standard range)."""
        manager = pm_encoder.LensManager()
        manager.active_lens_config = {
            "groups": [
                {"pattern": "*.critical", "priority": 9999},
                {"pattern": "*.ignore", "priority": -9999},
            ]
        }

        self.assertEqual(manager.get_file_priority(Path("important.critical")), 9999)
        self.assertEqual(manager.get_file_priority(Path("skip.ignore")), -9999)


class TestPatternMatching(unittest.TestCase):
    """Test glob pattern matching in priority resolution."""

    def test_simple_extension_pattern(self):
        """Test simple extension patterns like *.py."""
        manager = pm_encoder.LensManager()
        manager.active_lens_config = {
            "groups": [{"pattern": "*.py", "priority": 100}]
        }

        self.assertEqual(manager.get_file_priority(Path("main.py")), 100)
        self.assertEqual(manager.get_file_priority(Path("dir/main.py")), 100)
        self.assertEqual(manager.get_file_priority(Path("main.txt")), 50)  # fallback

    def test_directory_pattern(self):
        """Test directory patterns like tests/**."""
        manager = pm_encoder.LensManager()
        manager.active_lens_config = {
            "groups": [{"pattern": "tests/**", "priority": 20}],
            "fallback": {"priority": 50}
        }

        self.assertEqual(manager.get_file_priority(Path("tests/test_main.py")), 20)
        self.assertEqual(manager.get_file_priority(Path("tests/unit/test_foo.py")), 20)
        self.assertEqual(manager.get_file_priority(Path("src/main.py")), 50)

    def test_specific_filename_pattern(self):
        """Test specific filename patterns."""
        manager = pm_encoder.LensManager()
        manager.active_lens_config = {
            "groups": [
                {"pattern": "Makefile", "priority": 80},
                {"pattern": "Dockerfile", "priority": 75},
                {"pattern": "README.md", "priority": 70},
            ]
        }

        self.assertEqual(manager.get_file_priority(Path("Makefile")), 80)
        self.assertEqual(manager.get_file_priority(Path("Dockerfile")), 75)
        self.assertEqual(manager.get_file_priority(Path("README.md")), 70)


def run_tests():
    """Run all priority tests."""
    loader = unittest.TestLoader()
    suite = unittest.TestSuite()

    suite.addTests(loader.loadTestsFromTestCase(TestPriorityResolution))
    suite.addTests(loader.loadTestsFromTestCase(TestGroupConfiguration))
    suite.addTests(loader.loadTestsFromTestCase(TestBuiltInArchitectureLens))
    suite.addTests(loader.loadTestsFromTestCase(TestArbitraryPriorityValues))
    suite.addTests(loader.loadTestsFromTestCase(TestPatternMatching))

    runner = unittest.TextTestRunner(verbosity=2)
    result = runner.run(suite)

    print("\n" + "=" * 70)
    print("PRIORITY GROUPS TEST SUMMARY")
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
