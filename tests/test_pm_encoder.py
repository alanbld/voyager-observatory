#!/usr/bin/env python3
"""
Comprehensive test suite for pm_encoder v1.2.1

Tests critical functionality including:
- Structure mode triggering logic
- Lens precedence rules
- Language-specific structure extraction
- Meta file injection
- Ignore patterns
"""

import unittest
import tempfile
import shutil
import json
import sys
from pathlib import Path
from io import StringIO

# Import from parent directory
sys.path.insert(0, str(Path(__file__).parent.parent))
import pm_encoder


class TestStructureMode(unittest.TestCase):
    """Test structure mode truncation logic."""

    def setUp(self):
        """Create temporary test directory."""
        self.test_dir = tempfile.mkdtemp()
        self.test_path = Path(self.test_dir)

    def tearDown(self):
        """Clean up temporary directory."""
        shutil.rmtree(self.test_dir)

    def test_structure_mode_trigger(self):
        """Test that structure mode works even when truncate=0 (the bug fix)."""
        # Create a Python file with class and function
        py_file = self.test_path / "test.py"
        py_file.write_text("""import os
import sys

class MyClass:
    def __init__(self):
        self.x = 1
        self.y = 2
        self.z = 3

    def method_one(self):
        # Implementation details
        result = self.x + self.y
        return result * self.z

def standalone_function():
    print("Hello")
    print("World")
    return 42
""")

        # Serialize with structure mode but truncate=0
        output = StringIO()
        pm_encoder.serialize(
            self.test_path,
            output,
            ignore_patterns=[".git"],
            include_patterns=[],
            sort_by="name",
            sort_order="asc",
            truncate_lines=0,  # BUG FIX: structure mode should still work!
            truncate_mode="structure",
            truncate_summary=True,
            truncate_exclude=[],
            show_stats=False,
            language_plugins_dir=None,
            lens_manager=None
        )

        result = output.getvalue()

        # Verify structure mode was applied
        self.assertIn("import os", result)
        self.assertIn("import sys", result)
        self.assertIn("class MyClass:", result)
        self.assertIn("def __init__(self):", result)
        self.assertIn("def method_one(self):", result)
        self.assertIn("def standalone_function():", result)

        # Verify implementation details were REMOVED
        self.assertNotIn("self.x = 1", result)
        self.assertNotIn("result = self.x + self.y", result)
        self.assertNotIn('print("Hello")', result)

        # Verify structure mode marker is present
        self.assertIn("STRUCTURE MODE", result)

    def test_python_structure(self):
        """Test Python structure extraction preserves signatures, removes bodies."""
        py_file = self.test_path / "module.py"
        py_file.write_text("""from typing import List

@decorator
def decorated_function(arg1: str, arg2: int) -> bool:
    # Complex implementation
    x = arg1.upper()
    y = arg2 * 2
    return len(x) > y

class DataProcessor:
    def process(self, data: List[str]):
        for item in data:
            print(item)
        return True
""")

        analyzer = pm_encoder.PythonAnalyzer()
        lines = py_file.read_text().split('\n')
        structure_ranges = analyzer.get_structure_ranges(lines)

        # Extract structure lines
        kept_lines = []
        for start, end in structure_ranges:
            kept_lines.extend(lines[start-1:end])
        structure_output = '\n'.join(kept_lines)

        # Should include
        self.assertIn("from typing import List", structure_output)
        self.assertIn("@decorator", structure_output)
        self.assertIn("def decorated_function(arg1: str, arg2: int) -> bool:", structure_output)
        self.assertIn("class DataProcessor:", structure_output)
        self.assertIn("def process(self, data: List[str]):", structure_output)

        # Should NOT include (implementation details)
        self.assertNotIn("x = arg1.upper()", structure_output)
        self.assertNotIn("y = arg2 * 2", structure_output)
        self.assertNotIn("for item in data:", structure_output)
        self.assertNotIn("print(item)", structure_output)

    def test_js_structure(self):
        """Test JavaScript/TypeScript structure extraction."""
        js_file = self.test_path / "app.js"
        js_file.write_text("""import React from 'react';
import { useState } from 'react';

export class Component {
    constructor(props) {
        this.state = { count: 0 };
        this.handleClick = this.handleClick.bind(this);
    }

    handleClick() {
        this.setState({ count: this.state.count + 1 });
    }
}

export const useCustomHook = (initial) => {
    const [value, setValue] = useState(initial);
    const increment = () => setValue(value + 1);
    return [value, increment];
};

function helperFunction(x, y) {
    const sum = x + y;
    const product = x * y;
    return { sum, product };
}
""")

        analyzer = pm_encoder.JavaScriptAnalyzer()
        lines = js_file.read_text().split('\n')
        structure_ranges = analyzer.get_structure_ranges(lines)

        kept_lines = []
        for start, end in structure_ranges:
            kept_lines.extend(lines[start-1:end])
        structure_output = '\n'.join(kept_lines)

        # Should include
        self.assertIn("import React from 'react'", structure_output)
        self.assertIn("export class Component {", structure_output)
        self.assertIn("export const useCustomHook = (initial) =>", structure_output)

        # Should NOT include detailed implementation
        self.assertNotIn("this.state = { count: 0 }", structure_output)
        self.assertNotIn("const sum = x + y", structure_output)

    def test_json_fallback(self):
        """Test that JSON files are NOT truncated in structure mode (fallback to smart)."""
        json_file = self.test_path / "data.json"
        json_content = json.dumps({
            "key1": "value1",
            "key2": "value2",
            "nested": {"a": 1, "b": 2}
        }, indent=2)
        json_file.write_text(json_content)

        analyzer = pm_encoder.JSONAnalyzer()
        lines = json_content.split('\n')
        structure_ranges = analyzer.get_structure_ranges(lines)

        # JSON should not support structure mode (should return empty list)
        self.assertEqual(structure_ranges, [])

        # Test that truncate_content falls back to smart mode
        truncated, was_truncated, analysis = pm_encoder.truncate_content(
            json_content,
            Path("data.json"),
            max_lines=1000,
            mode='structure',
            analyzer_registry=pm_encoder.LanguageAnalyzerRegistry(),
            include_summary=True
        )

        # Should have JSON content (fallback worked)
        self.assertIn("key1", truncated)
        self.assertIn("value1", truncated)


class TestLenses(unittest.TestCase):
    """Test Context Lenses functionality."""

    def setUp(self):
        """Create temporary test directory."""
        self.test_dir = tempfile.mkdtemp()
        self.test_path = Path(self.test_dir)

    def tearDown(self):
        """Clean up temporary directory."""
        shutil.rmtree(self.test_dir)

    def test_meta_injection(self):
        """Test that .pm_encoder_meta file is injected when using a lens."""
        # Create a simple Python file
        py_file = self.test_path / "main.py"
        py_file.write_text("print('hello')")

        # Create lens manager and apply architecture lens
        lens_manager = pm_encoder.LensManager()
        base_config = {
            "ignore_patterns": [".git"],
            "include_patterns": [],
            "sort_by": "name",
            "sort_order": "asc",
            "truncate": 0,
            "truncate_mode": "simple",
            "truncate_exclude": []
        }
        lens_config = lens_manager.apply_lens("architecture", base_config)

        # Serialize with lens
        output = StringIO()
        pm_encoder.serialize(
            self.test_path,
            output,
            ignore_patterns=lens_config["ignore_patterns"],
            include_patterns=lens_config["include_patterns"],
            sort_by=lens_config["sort_by"],
            sort_order=lens_config["sort_order"],
            truncate_lines=lens_config.get("truncate", 0),
            truncate_mode=lens_config.get("truncate_mode", "simple"),
            truncate_summary=True,
            truncate_exclude=lens_config.get("truncate_exclude", []),
            show_stats=False,
            language_plugins_dir=None,
            lens_manager=lens_manager
        )

        result = output.getvalue()

        # Verify .pm_encoder_meta is present
        self.assertIn("++++++++++ .pm_encoder_meta ++++++++++", result)
        self.assertIn('Context generated with lens: "architecture"', result)
        self.assertIn("Focus: High-level structure, interfaces, configuration", result)
        self.assertIn("pm_encoder version:", result)

    def test_lens_precedence(self):
        """Test that lens configuration precedence works correctly."""
        # Create custom lens in config
        custom_lenses = {
            "custom": {
                "description": "Custom test lens",
                "include": ["*.py"],
                "exclude": ["tests/**"],
                "truncate": 100,
                "truncate_mode": "smart",
                "sort_by": "mtime"
            }
        }

        lens_manager = pm_encoder.LensManager(custom_lenses)

        base_config = {
            "ignore_patterns": [".git", "node_modules"],
            "include_patterns": ["*.js"],  # Should be overridden by lens
            "sort_by": "name",  # Should be overridden by lens
            "sort_order": "asc",
            "truncate": 0,  # Should be overridden by lens
            "truncate_mode": "simple",  # Should be overridden by lens
            "truncate_exclude": []
        }

        lens_config = lens_manager.apply_lens("custom", base_config)

        # Verify lens settings override base config
        self.assertEqual(lens_config["include_patterns"], ["*.py"])
        self.assertIn("tests/**", lens_config["ignore_patterns"])
        self.assertEqual(lens_config["truncate"], 100)
        self.assertEqual(lens_config["truncate_mode"], "smart")
        self.assertEqual(lens_config["sort_by"], "mtime")

        # Verify base config values are preserved where lens doesn't specify
        self.assertIn(".git", lens_config["ignore_patterns"])
        self.assertIn("node_modules", lens_config["ignore_patterns"])


class TestIgnorePatterns(unittest.TestCase):
    """Test ignore patterns functionality."""

    def setUp(self):
        """Create temporary test directory."""
        self.test_dir = tempfile.mkdtemp()
        self.test_path = Path(self.test_dir)

    def tearDown(self):
        """Clean up temporary directory."""
        shutil.rmtree(self.test_dir)

    def test_ignore_patterns(self):
        """Test that .git and other ignore patterns are respected."""
        # Create directory structure
        (self.test_path / ".git").mkdir()
        (self.test_path / ".git" / "config").write_text("git config")
        (self.test_path / "__pycache__").mkdir()
        (self.test_path / "__pycache__" / "cache.pyc").write_text("cache")
        (self.test_path / "src").mkdir()
        (self.test_path / "src" / "main.py").write_text("print('main')")
        (self.test_path / "test.log").write_text("log data")

        output = StringIO()
        pm_encoder.serialize(
            self.test_path,
            output,
            ignore_patterns=[".git", "__pycache__", "*.log"],
            include_patterns=[],
            sort_by="name",
            sort_order="asc",
            truncate_lines=0,
            truncate_mode="simple",
            truncate_summary=True,
            truncate_exclude=[],
            show_stats=False,
            language_plugins_dir=None,
            lens_manager=None
        )

        result = output.getvalue()

        # Should include
        self.assertIn("src/main.py", result)
        self.assertIn("print('main')", result)

        # Should NOT include (ignored)
        self.assertNotIn(".git", result)
        self.assertNotIn("git config", result)
        self.assertNotIn("__pycache__", result)
        self.assertNotIn("cache.pyc", result)
        self.assertNotIn("test.log", result)
        self.assertNotIn("log data", result)


class TestBuiltInLenses(unittest.TestCase):
    """Test built-in lenses are properly defined."""

    def test_all_lenses_exist(self):
        """Test that all 4 built-in lenses exist."""
        lens_manager = pm_encoder.LensManager()
        lenses = lens_manager.BUILT_IN_LENSES

        self.assertIn("architecture", lenses)
        self.assertIn("debug", lenses)
        self.assertIn("security", lenses)
        self.assertIn("onboarding", lenses)

    def test_architecture_lens_has_safety_limit(self):
        """Test that architecture lens has the safety limit (v1.2.1 fix)."""
        lens_manager = pm_encoder.LensManager()
        arch_lens = lens_manager.BUILT_IN_LENSES["architecture"]

        self.assertEqual(arch_lens["truncate"], 2000)
        self.assertEqual(arch_lens["truncate_mode"], "structure")


def run_tests():
    """Run all tests and print results."""
    # Create test suite
    loader = unittest.TestLoader()
    suite = unittest.TestSuite()

    # Add all test classes
    suite.addTests(loader.loadTestsFromTestCase(TestStructureMode))
    suite.addTests(loader.loadTestsFromTestCase(TestLenses))
    suite.addTests(loader.loadTestsFromTestCase(TestIgnorePatterns))
    suite.addTests(loader.loadTestsFromTestCase(TestBuiltInLenses))

    # Run tests with verbose output
    runner = unittest.TextTestRunner(verbosity=2)
    result = runner.run(suite)

    # Print summary
    print("\n" + "="*70)
    print("TEST SUMMARY")
    print("="*70)
    print(f"Tests run: {result.testsRun}")
    print(f"Successes: {result.testsRun - len(result.failures) - len(result.errors)}")
    print(f"Failures: {len(result.failures)}")
    print(f"Errors: {len(result.errors)}")
    print("="*70)

    return result.wasSuccessful()


if __name__ == "__main__":
    success = run_tests()
    sys.exit(0 if success else 1)
