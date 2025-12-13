#!/usr/bin/env python3
"""
Comprehensive Test Suite for pm_encoder
Target: >98% code coverage

This test suite systematically covers:
- All 7 language analyzers with edge cases
- CLI argument parsing and main() function
- Configuration system
- Edge cases and error handling
- Performance regression tests
"""

import unittest
import tempfile
import shutil
import json
import sys
import subprocess
from pathlib import Path
from io import StringIO

# Import from parent directory
sys.path.insert(0, str(Path(__file__).parent.parent))
import pm_encoder


class TestAllLanguageAnalyzers(unittest.TestCase):
    """Comprehensive tests for all 7 language analyzers."""

    def setUp(self):
        """Set up test fixtures."""
        self.fixtures_dir = Path(__file__).parent / "fixtures"

    def test_python_analyzer_comprehensive(self):
        """Test Python analyzer with comprehensive patterns."""
        fixture = self.fixtures_dir / "python" / "sample.py"
        content = fixture.read_text()

        analyzer = pm_encoder.PythonAnalyzer()
        lines = content.split('\n')
        result = analyzer.analyze_lines(lines, fixture)

        # Verify detection
        self.assertEqual(result["language"], "Python")
        self.assertIn("DataProcessor", result["classes"])
        self.assertIn("process", result["functions"])
        self.assertIn("decorated_function", result["functions"])
        # async_handler is an async function that should be detected
        self.assertTrue(len(result["functions"]) >= 4)
        self.assertIn("__main__ block", result["entry_points"])

    def test_javascript_analyzer_comprehensive(self):
        """Test JavaScript analyzer with comprehensive patterns."""
        fixture = self.fixtures_dir / "javascript" / "sample.js"
        content = fixture.read_text()

        analyzer = pm_encoder.JavaScriptAnalyzer()
        lines = content.split('\n')
        result = analyzer.analyze_lines(lines, fixture)

        self.assertEqual(result["language"], "JavaScript/TypeScript")
        self.assertIn("App", result["classes"])
        self.assertIn("useCounter", result["functions"])

    def test_rust_analyzer_comprehensive(self):
        """Test Rust analyzer with comprehensive patterns."""
        fixture = self.fixtures_dir / "rust" / "sample.rs"
        content = fixture.read_text()

        analyzer = pm_encoder.RustAnalyzer()
        lines = content.split('\n')
        result = analyzer.analyze_lines(lines, fixture)

        self.assertEqual(result["language"], "Rust")
        self.assertIn("Config", result["classes"])
        self.assertIn("Processable", result["classes"])  # Trait
        self.assertIn("new", result["functions"])
        self.assertIn("async_handler", result["functions"])
        # Category is "test" because path contains "tests/"
        self.assertEqual(result["category"], "test")

    def test_shell_analyzer_comprehensive(self):
        """Test Shell analyzer with comprehensive patterns."""
        fixture = self.fixtures_dir / "shell" / "sample.sh"
        content = fixture.read_text()

        analyzer = pm_encoder.ShellAnalyzer()
        lines = content.split('\n')
        result = analyzer.analyze_lines(lines, fixture)

        self.assertEqual(result["language"], "Shell (bash)")
        self.assertIn("setup", result["functions"])
        self.assertIn("cleanup", result["functions"])
        self.assertIn("process_data", result["functions"])

    def test_markdown_analyzer_comprehensive(self):
        """Test Markdown analyzer with comprehensive patterns."""
        fixture = self.fixtures_dir / "markdown" / "sample.md"
        content = fixture.read_text()

        analyzer = pm_encoder.MarkdownAnalyzer()
        lines = content.split('\n')
        result = analyzer.analyze_lines(lines, fixture)

        self.assertEqual(result["language"], "Markdown")
        self.assertEqual(result["category"], "documentation")
        # Should detect headers
        self.assertTrue(len(result["entry_points"]) > 0)

    def test_json_analyzer_comprehensive(self):
        """Test JSON analyzer with comprehensive patterns."""
        fixture = self.fixtures_dir / "json" / "sample.json"
        content = fixture.read_text()

        analyzer = pm_encoder.JSONAnalyzer()
        lines = content.split('\n')
        result = analyzer.analyze_lines(lines, fixture)

        self.assertEqual(result["language"], "JSON")
        self.assertEqual(result["category"], "config")
        # Should detect keys
        self.assertTrue(len(result["config_keys"]) > 0)

    def test_yaml_analyzer_comprehensive(self):
        """Test YAML analyzer with comprehensive patterns."""
        fixture = self.fixtures_dir / "yaml" / "sample.yml"
        content = fixture.read_text()

        analyzer = pm_encoder.YAMLAnalyzer()
        lines = content.split('\n')
        result = analyzer.analyze_lines(lines, fixture)

        self.assertEqual(result["language"], "YAML")
        self.assertEqual(result["category"], "config")
        self.assertIn("name", result["config_keys"])

    def test_python_get_truncate_ranges(self):
        """Test Python truncate ranges calculation."""
        code = "import os\n" * 100 + "def main():\n    pass\n" * 50

        analyzer = pm_encoder.PythonAnalyzer()
        ranges, analysis = analyzer.get_truncate_ranges(code, max_lines=50)

        self.assertTrue(len(ranges) > 0)
        self.assertEqual(analysis["language"], "Python")

    def test_javascript_get_truncate_ranges(self):
        """Test JavaScript truncate ranges calculation."""
        code = "import React from 'react';\n" * 100 + "function App() {\n  return null;\n}\n"

        analyzer = pm_encoder.JavaScriptAnalyzer()
        ranges, analysis = analyzer.get_truncate_ranges(code, max_lines=50)

        self.assertTrue(len(ranges) > 0)
        self.assertEqual(analysis["language"], "JavaScript/TypeScript")

    def test_shell_get_truncate_ranges(self):
        """Test Shell truncate ranges calculation."""
        code = "#!/bin/bash\n" + "echo 'line'\n" * 100

        analyzer = pm_encoder.ShellAnalyzer()
        ranges, analysis = analyzer.get_truncate_ranges(code, max_lines=50)

        self.assertTrue(len(ranges) > 0)

    def test_markdown_get_truncate_ranges(self):
        """Test Markdown truncate ranges calculation."""
        code = "# Header\n\nContent line\n" * 100

        analyzer = pm_encoder.MarkdownAnalyzer()
        ranges, analysis = analyzer.get_truncate_ranges(code, max_lines=50)

        self.assertTrue(len(ranges) > 0)

    def test_analyzer_registry_get_analyzer(self):
        """Test analyzer registry returns correct analyzer."""
        registry = pm_encoder.LanguageAnalyzerRegistry()

        # Test each supported extension
        self.assertIsInstance(registry.get_analyzer(Path("test.py")), pm_encoder.PythonAnalyzer)
        self.assertIsInstance(registry.get_analyzer(Path("test.js")), pm_encoder.JavaScriptAnalyzer)
        self.assertIsInstance(registry.get_analyzer(Path("test.rs")), pm_encoder.RustAnalyzer)
        self.assertIsInstance(registry.get_analyzer(Path("test.sh")), pm_encoder.ShellAnalyzer)
        self.assertIsInstance(registry.get_analyzer(Path("test.md")), pm_encoder.MarkdownAnalyzer)
        self.assertIsInstance(registry.get_analyzer(Path("test.json")), pm_encoder.JSONAnalyzer)
        self.assertIsInstance(registry.get_analyzer(Path("test.yml")), pm_encoder.YAMLAnalyzer)

        # Test unknown extension returns default
        self.assertIsInstance(registry.get_analyzer(Path("test.unknown")), pm_encoder.LanguageAnalyzer)

    def test_analyzer_registry_get_supported_languages(self):
        """Test analyzer registry lists supported languages."""
        registry = pm_encoder.LanguageAnalyzerRegistry()
        languages = registry.get_supported_languages()

        self.assertIn("Python", languages)
        self.assertIn("JavaScript/TypeScript", languages)
        self.assertIn("Rust", languages)
        # Shell analyzer returns "Shell" not "Shell (bash)"
        self.assertTrue(any("Shell" in lang for lang in languages))
        self.assertIn("Markdown", languages)
        self.assertIn("JSON", languages)
        self.assertIn("YAML", languages)


class TestCLIComprehensive(unittest.TestCase):
    """Comprehensive CLI and main() function tests."""

    def setUp(self):
        """Set up test directory."""
        self.test_dir = tempfile.mkdtemp()
        self.test_path = Path(self.test_dir)

    def tearDown(self):
        """Clean up test directory."""
        shutil.rmtree(self.test_dir)

    def test_main_with_truncate_simple(self):
        """Test main() with simple truncation mode."""
        # Create test file
        test_file = self.test_path / "test.py"
        test_file.write_text("line\n" * 100)

        output_file = self.test_path / "output.txt"

        # Run via subprocess
        result = subprocess.run(
            ["./pm_encoder.py", str(self.test_path), "--truncate", "10",
             "--truncate-mode", "simple", "-o", str(output_file)],
            capture_output=True,
            text=True,
            cwd=Path(__file__).parent.parent
        )

        self.assertEqual(result.returncode, 0)
        self.assertTrue(output_file.exists())

    def test_main_with_truncate_smart(self):
        """Test main() with smart truncation mode."""
        test_file = self.test_path / "test.py"
        test_file.write_text("import os\n" + "def main():\n    pass\n" * 50)

        output_file = self.test_path / "output.txt"

        result = subprocess.run(
            ["./pm_encoder.py", str(self.test_path), "--truncate", "20",
             "--truncate-mode", "smart", "-o", str(output_file)],
            capture_output=True,
            text=True,
            cwd=Path(__file__).parent.parent
        )

        self.assertEqual(result.returncode, 0)
        self.assertTrue(output_file.exists())

    def test_main_with_lens_architecture(self):
        """Test main() with architecture lens."""
        test_file = self.test_path / "test.py"
        test_file.write_text("def test(): pass")

        output_file = self.test_path / "output.txt"

        result = subprocess.run(
            ["./pm_encoder.py", str(self.test_path), "--lens", "architecture",
             "-o", str(output_file)],
            capture_output=True,
            text=True,
            cwd=Path(__file__).parent.parent
        )

        self.assertEqual(result.returncode, 0)
        # Check for meta file
        content = output_file.read_text()
        self.assertIn(".pm_encoder_meta", content)

    def test_main_with_sorting_mtime_desc(self):
        """Test main() with mtime descending sort."""
        # Create files with different mtimes
        import time
        file1 = self.test_path / "file1.txt"
        file1.write_text("first")
        time.sleep(0.01)
        file2 = self.test_path / "file2.txt"
        file2.write_text("second")

        output_file = self.test_path / "output.txt"

        result = subprocess.run(
            ["./pm_encoder.py", str(self.test_path), "--sort-by", "mtime",
             "--sort-order", "desc", "-o", str(output_file)],
            capture_output=True,
            text=True,
            cwd=Path(__file__).parent.parent
        )

        self.assertEqual(result.returncode, 0)

    def test_main_version_flag(self):
        """Test --version flag."""
        result = subprocess.run(
            ["./pm_encoder.py", "--version"],
            capture_output=True,
            text=True,
            cwd=Path(__file__).parent.parent
        )

        self.assertEqual(result.returncode, 0)
        self.assertIn("1.2.2", result.stdout)

    def test_main_create_plugin(self):
        """Test --create-plugin flag."""
        result = subprocess.run(
            ["./pm_encoder.py", "--create-plugin", "kotlin"],
            capture_output=True,
            text=True,
            cwd=Path(__file__).parent.parent
        )

        self.assertEqual(result.returncode, 0)
        self.assertIn("kotlin", result.stdout.lower())

    def test_main_plugin_prompt(self):
        """Test --plugin-prompt flag."""
        result = subprocess.run(
            ["./pm_encoder.py", "--plugin-prompt", "swift"],
            capture_output=True,
            text=True,
            cwd=Path(__file__).parent.parent
        )

        self.assertEqual(result.returncode, 0)
        self.assertIn("swift", result.stdout.lower())


class TestEdgeCasesComprehensive(unittest.TestCase):
    """Comprehensive edge case testing."""

    def setUp(self):
        """Set up test directory."""
        self.test_dir = tempfile.mkdtemp()
        self.test_path = Path(self.test_dir)

    def tearDown(self):
        """Clean up test directory."""
        shutil.rmtree(self.test_dir)

    def test_empty_directory(self):
        """Test serialization of empty directory."""
        output = StringIO()

        pm_encoder.serialize(
            self.test_path,
            output,
            ignore_patterns=[],
            include_patterns=[],
            sort_by="name",
            sort_order="asc"
        )

        result = output.getvalue()
        self.assertEqual(len(result), 0)  # No files to serialize

    def test_binary_file_skipped(self):
        """Test that binary files are skipped."""
        # Create a binary file
        binary_file = self.test_path / "test.bin"
        binary_file.write_bytes(b'\x00\x01\x02\x03\xFF\xFE')

        output = StringIO()
        pm_encoder.serialize(
            self.test_path,
            output,
            ignore_patterns=[],
            include_patterns=[],
            sort_by="name",
            sort_order="asc"
        )

        result = output.getvalue()
        # Binary file should be skipped
        self.assertNotIn("test.bin", result)

    def test_large_file_skipped(self):
        """Test that files >5MB are skipped."""
        large_file = self.test_path / "large.txt"
        # Create a file just over 5MB
        large_file.write_text("x" * (5 * 1024 * 1024 + 1000))

        output = StringIO()
        pm_encoder.serialize(
            self.test_path,
            output,
            ignore_patterns=[],
            include_patterns=[],
            sort_by="name",
            sort_order="asc"
        )

        result = output.getvalue()
        # Large file should be skipped
        self.assertNotIn("large.txt", result)

    def test_unicode_content(self):
        """Test handling of unicode content."""
        unicode_file = self.test_path / "unicode.txt"
        unicode_file.write_text("Hello 世界 🌍 Здравствуй мир")

        output = StringIO()
        pm_encoder.serialize(
            self.test_path,
            output,
            ignore_patterns=[],
            include_patterns=[],
            sort_by="name",
            sort_order="asc"
        )

        result = output.getvalue()
        self.assertIn("Hello 世界 🌍", result)

    def test_deeply_nested_json(self):
        """Test deeply nested JSON doesn't cause RecursionError."""
        # Create deeply nested JSON
        nested = {"level": 1}
        current = nested
        for i in range(2, 100):
            current["child"] = {"level": i}
            current = current["child"]

        json_file = self.test_path / "deep.json"
        json_file.write_text(json.dumps(nested))

        analyzer = pm_encoder.JSONAnalyzer()
        lines = json_file.read_text().split('\n')

        # Should not raise RecursionError
        try:
            result = analyzer.analyze_lines(lines, json_file)
            self.assertEqual(result["language"], "JSON")
        except RecursionError:
            self.fail("RecursionError should be caught and handled")

    def test_truncate_content_structure_mode(self):
        """Test truncate_content with structure mode."""
        code = """import os

class Test:
    def method(self):
        impl = "details"
        return impl
"""
        registry = pm_encoder.LanguageAnalyzerRegistry()

        truncated, was_truncated, analysis = pm_encoder.truncate_content(
            code,
            Path("test.py"),
            max_lines=1000,
            mode="structure",
            analyzer_registry=registry,
            include_summary=True
        )

        self.assertTrue(was_truncated)
        self.assertIn("import os", truncated)
        self.assertIn("class Test:", truncated)
        self.assertIn("def method(self):", truncated)
        self.assertNotIn('impl = "details"', truncated)


class TestConfigurationSystem(unittest.TestCase):
    """Test configuration loading and precedence."""

    def setUp(self):
        """Set up test directory."""
        self.test_dir = tempfile.mkdtemp()
        self.test_path = Path(self.test_dir)

    def tearDown(self):
        """Clean up test directory."""
        shutil.rmtree(self.test_dir)

    def test_load_config_no_file(self):
        """Test load_config when no config file exists."""
        ignore, include, lenses = pm_encoder.load_config(Path("nonexistent.json"))

        # Should return defaults
        self.assertIn(".git", ignore)
        self.assertEqual(include, [])
        self.assertEqual(lenses, {})

    def test_load_config_with_file(self):
        """Test load_config with valid config file."""
        config_file = self.test_path / "config.json"
        config_data = {
            "ignore_patterns": ["custom_ignore/**"],
            "include_patterns": ["*.custom"],
            "lenses": {
                "test_lens": {
                    "description": "Test lens",
                    "truncate": 100
                }
            }
        }
        config_file.write_text(json.dumps(config_data))

        ignore, include, lenses = pm_encoder.load_config(config_file)

        self.assertIn("custom_ignore/**", ignore)
        self.assertIn("*.custom", include)
        self.assertIn("test_lens", lenses)

    def test_load_config_malformed_json(self):
        """Test load_config with malformed JSON."""
        config_file = self.test_path / "bad_config.json"
        config_file.write_text("{bad json}")

        # Should handle gracefully
        ignore, include, lenses = pm_encoder.load_config(config_file)

        # Should return defaults
        self.assertIn(".git", ignore)

    def test_lens_manager_custom_lens(self):
        """Test LensManager with custom lens."""
        custom_lenses = {
            "my_lens": {
                "description": "My custom lens",
                "include": ["*.py"],
                "truncate": 200
            }
        }

        lens_manager = pm_encoder.LensManager(custom_lenses)
        base_config = {
            "ignore_patterns": [".git"],
            "include_patterns": [],
            "sort_by": "name",
            "sort_order": "asc"
        }

        result = lens_manager.apply_lens("my_lens", base_config)

        self.assertEqual(result["include_patterns"], ["*.py"])
        self.assertEqual(result["truncate"], 200)

    def test_lens_manager_invalid_lens(self):
        """Test LensManager with invalid lens name."""
        lens_manager = pm_encoder.LensManager()
        base_config = {}

        with self.assertRaises(ValueError):
            lens_manager.apply_lens("nonexistent_lens", base_config)


class TestPerformanceRegression(unittest.TestCase):
    """Performance regression tests."""

    def setUp(self):
        """Set up test directory."""
        self.test_dir = tempfile.mkdtemp()
        self.test_path = Path(self.test_dir)

    def tearDown(self):
        """Clean up test directory."""
        shutil.rmtree(self.test_dir)

    def test_large_number_of_files_performance(self):
        """Test performance with many files."""
        import time

        # Create 100 small files (representative of real workload)
        for i in range(100):
            file_path = self.test_path / f"file_{i}.py"
            file_path.write_text(f"# File {i}\ndef test_{i}(): pass\n")

        start_time = time.time()

        output = StringIO()
        pm_encoder.serialize(
            self.test_path,
            output,
            ignore_patterns=[],
            include_patterns=[],
            sort_by="name",
            sort_order="asc"
        )

        elapsed = time.time() - start_time

        # Should complete in reasonable time (< 2 seconds for 100 files)
        self.assertLess(elapsed, 2.0)


class TestTruncationWithSummary(unittest.TestCase):
    """Test truncation with summary markers."""

    def test_simple_truncation_with_summary(self):
        """Test simple truncation mode with include_summary=True."""
        content = "\n".join([f"line {i}" for i in range(100)])
        analyzer_registry = pm_encoder.LanguageAnalyzerRegistry()

        result, was_truncated, analysis = pm_encoder.truncate_content(
            content,
            Path("test.txt"),
            max_lines=10,
            mode="simple",
            analyzer_registry=analyzer_registry,
            include_summary=True
        )

        self.assertTrue(was_truncated)
        self.assertIn("TRUNCATED", result)
        self.assertIn("10/100", result)

    def test_smart_truncation_with_summary(self):
        """Test smart truncation mode with include_summary=True."""
        # Create Python content with classes and functions
        content = '''import os
import sys

class TestClass:
    def method1(self):
        pass

    def method2(self):
        pass

def function1():
    """Function 1."""
    pass

def function2():
    """Function 2."""
    pass

if __name__ == "__main__":
    function1()
'''

        analyzer_registry = pm_encoder.LanguageAnalyzerRegistry()
        result, was_truncated, analysis = pm_encoder.truncate_content(
            content,
            Path("test.py"),
            max_lines=10,
            mode="smart",
            analyzer_registry=analyzer_registry,
            include_summary=True
        )

        self.assertTrue(was_truncated)
        self.assertIn("TRUNCATED", result)
        self.assertIn("Language:", result)
        self.assertIn("Python", result)

    def test_structure_mode_without_summary(self):
        """Test structure mode with include_summary=False."""
        content = '''def test_function():
    """Test function."""
    result = 1 + 2
    return result
'''

        analyzer_registry = pm_encoder.LanguageAnalyzerRegistry()
        result, was_truncated, analysis = pm_encoder.truncate_content(
            content,
            Path("test.py"),
            max_lines=50,
            mode="structure",
            analyzer_registry=analyzer_registry,
            include_summary=False
        )

        # Structure mode should extract signatures
        self.assertIn("def test_function():", result)


class TestDirectFunctionCalls(unittest.TestCase):
    """Test functions directly for coverage."""

    def test_create_plugin_template_direct(self):
        """Test create_plugin_template() directly."""
        import sys
        from io import StringIO

        # Capture stdout
        old_stdout = sys.stdout
        old_stderr = sys.stderr
        sys.stdout = StringIO()
        sys.stderr = StringIO()

        try:
            pm_encoder.create_plugin_template("TestLang")
            output = sys.stdout.getvalue()
            stderr = sys.stderr.getvalue()

            # Verify template was generated
            self.assertIn("pm_encoder Language Plugin: TestLang", output)
            self.assertIn("class LanguageAnalyzer:", output)
            self.assertIn("def analyze(self, content: str, file_path: Path)", output)
            self.assertIn("Plugin template generated", stderr)
        finally:
            sys.stdout = old_stdout
            sys.stderr = old_stderr

    def test_create_plugin_prompt_direct(self):
        """Test create_plugin_prompt() directly."""
        import sys
        from io import StringIO

        # Capture stdout
        old_stdout = sys.stdout
        sys.stdout = StringIO()

        try:
            pm_encoder.create_plugin_prompt("Kotlin")
            output = sys.stdout.getvalue()

            # Verify prompt was generated
            self.assertIn("AI Prompt: Create pm_encoder Language Plugin for Kotlin", output)
            self.assertIn("Requirements", output)
            self.assertIn("Plugin Interface", output)
        finally:
            sys.stdout = old_stdout

    def test_truncation_stats_print_report(self):
        """Test TruncationStats.print_report()."""
        import sys
        from io import StringIO

        stats = pm_encoder.TruncationStats()
        stats.add_file("Python", 100, 50, True)
        stats.add_file("Python", 200, 100, True)
        stats.add_file("JavaScript", 150, 75, True)

        # Capture stderr
        old_stderr = sys.stderr
        sys.stderr = StringIO()

        try:
            stats.print_report()
            output = sys.stderr.getvalue()

            # Verify report was generated
            self.assertIn("TRUNCATION REPORT", output)
            self.assertIn("Files analyzed: 3", output)
            self.assertIn("Python", output)
            self.assertIn("JavaScript", output)
        finally:
            sys.stderr = old_stderr

    def test_lens_manager_print_manifest(self):
        """Test LensManager.print_manifest()."""
        import sys
        from io import StringIO

        lens_manager = pm_encoder.LensManager()
        base_config = {
            "ignore_patterns": [],
            "include_patterns": [],
            "sort_by": "name",
            "sort_order": "asc",
            "truncate": 0,
            "truncate_mode": "smart",
            "truncate_exclude": []
        }

        # Apply architecture lens
        lens_manager.apply_lens("architecture", base_config)

        # Capture stderr
        old_stderr = sys.stderr
        sys.stderr = StringIO()

        try:
            lens_manager.print_manifest()
            output = sys.stderr.getvalue()

            # Verify manifest was printed
            self.assertIn("[LENS: architecture]", output)
            self.assertIn("Description:", output)
        finally:
            sys.stderr = old_stderr

    def test_analyzer_registry_load_plugins(self):
        """Test LanguageAnalyzerRegistry.load_plugins()."""
        registry = pm_encoder.LanguageAnalyzerRegistry()

        # Call load_plugins with non-existent directory (should handle gracefully)
        registry.load_plugins(Path("/nonexistent/path"))

        # Call load_plugins with default (should handle gracefully)
        registry.load_plugins()

    def test_json_analyzer_get_truncate_ranges(self):
        """Test JSONAnalyzer.get_truncate_ranges()."""
        content = '''{\n''' + '''\n'''.join([f'  "key{i}": "value{i}",' for i in range(100)]) + '''\n  "last": "value"\n}'''

        analyzer = pm_encoder.JSONAnalyzer()
        ranges, analysis = analyzer.get_truncate_ranges(content, 20)

        # Should return ranges
        self.assertTrue(len(ranges) > 0)
        self.assertEqual(analysis["language"], "JSON")

    def test_shell_analyzer_get_structure_ranges(self):
        """Test ShellAnalyzer.get_structure_ranges()."""
        lines = [
            "#!/bin/bash",
            "source /etc/profile",
            ". ~/.bashrc",
            "function setup() {",
            "    echo 'setup'",
            "}",
            "process_data() {",
            "    local input=$1",
            "}"
        ]

        analyzer = pm_encoder.ShellAnalyzer()
        ranges = analyzer.get_structure_ranges(lines)

        # Should keep shebang and function declarations
        self.assertTrue(len(ranges) > 0)


class TestMainFunctionDirect(unittest.TestCase):
    """Test main() function directly for coverage."""

    def setUp(self):
        """Set up test directory."""
        self.test_dir = tempfile.mkdtemp()
        self.test_path = Path(self.test_dir)
        self.output_file = self.test_path / "output.txt"

        # Create a test Python file
        test_file = self.test_path / "test.py"
        test_file.write_text("def foo():\n    pass\n")

        # Save original sys.argv
        self.original_argv = sys.argv

    def tearDown(self):
        """Clean up test directory."""
        # Restore sys.argv
        sys.argv = self.original_argv
        shutil.rmtree(self.test_dir)

    def test_main_basic_serialization(self):
        """Test main() basic serialization."""
        sys.argv = ["pm_encoder.py", str(self.test_path), "-o", str(self.output_file)]

        try:
            pm_encoder.main()
            self.assertTrue(self.output_file.exists())
        except SystemExit:
            pass

    def test_main_with_lens_and_manifest(self):
        """Test main() with lens to trigger print_manifest()."""
        sys.argv = ["pm_encoder.py", str(self.test_path), "--lens", "architecture",
                    "-o", str(self.output_file)]

        try:
            pm_encoder.main()
            self.assertTrue(self.output_file.exists())
        except SystemExit:
            pass

    def test_main_with_truncation_enabled(self):
        """Test main() with truncation to trigger stats."""
        sys.argv = ["pm_encoder.py", str(self.test_path), "--truncate", "5",
                    "--truncate-stats", "-o", str(self.output_file)]

        try:
            pm_encoder.main()
            self.assertTrue(self.output_file.exists())
        except SystemExit:
            pass

    def test_main_with_include_override(self):
        """Test main() with --include to trigger override message."""
        sys.argv = ["pm_encoder.py", str(self.test_path), "--include", "*.py",
                    "-o", str(self.output_file)]

        try:
            pm_encoder.main()
            self.assertTrue(self.output_file.exists())
        except SystemExit:
            pass

    def test_main_with_exclude_addition(self):
        """Test main() with --exclude to trigger exclusion."""
        sys.argv = ["pm_encoder.py", str(self.test_path), "--exclude", "*.pyc",
                    "-o", str(self.output_file)]

        try:
            pm_encoder.main()
            self.assertTrue(self.output_file.exists())
        except SystemExit:
            pass

    def test_main_with_custom_config(self):
        """Test main() with custom config file."""
        config_file = self.test_path / "custom_config.json"
        config_file.write_text('{"ignore_patterns": ["*.pyc"]}')

        sys.argv = ["pm_encoder.py", str(self.test_path), "-c", str(config_file),
                    "-o", str(self.output_file)]

        try:
            pm_encoder.main()
            self.assertTrue(self.output_file.exists())
        except SystemExit:
            pass

    def test_main_with_sort_options(self):
        """Test main() with sort options."""
        sys.argv = ["pm_encoder.py", str(self.test_path), "--sort-by", "mtime",
                    "--sort-order", "desc", "-o", str(self.output_file)]

        try:
            pm_encoder.main()
            self.assertTrue(self.output_file.exists())
        except SystemExit:
            pass

    def test_main_with_structure_mode(self):
        """Test main() with structure truncation mode."""
        sys.argv = ["pm_encoder.py", str(self.test_path), "--truncate", "10",
                    "--truncate-mode", "structure", "-o", str(self.output_file)]

        try:
            pm_encoder.main()
            self.assertTrue(self.output_file.exists())
        except SystemExit:
            pass

    def test_main_with_truncate_exclude_pattern(self):
        """Test main() with truncate-exclude pattern."""
        sys.argv = ["pm_encoder.py", str(self.test_path), "--truncate", "5",
                    "--truncate-exclude", "*.py", "-o", str(self.output_file)]

        try:
            pm_encoder.main()
            self.assertTrue(self.output_file.exists())
        except SystemExit:
            pass


class TestEdgeCasesForCoverage(unittest.TestCase):
    """Edge case tests to reach >98% coverage."""

    def test_truncation_summary_with_many_classes(self):
        """Test truncation summary with >10 classes to trigger truncation."""
        # Create content with 15 classes
        classes = '\n\n'.join([f'class Class{i}:\n    pass' for i in range(15)])
        content = f'''import os\n\n{classes}\n'''

        analyzer_registry = pm_encoder.LanguageAnalyzerRegistry()
        result, _, _ = pm_encoder.truncate_content(
            content,
            Path("test.py"),
            max_lines=5,
            mode="smart",
            analyzer_registry=analyzer_registry,
            include_summary=True
        )

        # Should show "... (+N more)" for classes
        self.assertIn("+", result)
        self.assertIn("more", result)

    def test_truncation_summary_with_many_functions(self):
        """Test truncation summary with >10 functions."""
        # Create content with 15 functions
        funcs = '\n\n'.join([f'def func{i}():\n    pass' for i in range(15)])
        content = f'''import os\n\n{funcs}\n'''

        analyzer_registry = pm_encoder.LanguageAnalyzerRegistry()
        result, _, _ = pm_encoder.truncate_content(
            content,
            Path("test.py"),
            max_lines=5,
            mode="smart",
            analyzer_registry=analyzer_registry,
            include_summary=True
        )

        # Should show "... (+N more)" for functions
        self.assertIn("+", result)
        self.assertIn("more", result)

    def test_truncation_summary_with_many_imports(self):
        """Test truncation summary with >8 imports."""
        # Create content with 10 imports
        imports = '\n'.join([f'import module{i}' for i in range(10)])
        content = f'''{imports}\n\ndef test():\n    pass'''

        analyzer_registry = pm_encoder.LanguageAnalyzerRegistry()
        result, _, _ = pm_encoder.truncate_content(
            content,
            Path("test.py"),
            max_lines=5,
            mode="smart",
            analyzer_registry=analyzer_registry,
            include_summary=True
        )

        # Should show "..." for imports
        self.assertIn("...", result)

    def test_lens_manager_print_manifest_with_truncate_disabled(self):
        """Test print_manifest with truncate=0."""
        import sys
        from io import StringIO

        lens_manager = pm_encoder.LensManager({
            "test": {
                "description": "Test lens",
                "truncate": 0,
                "include": []
            }
        })

        base_config = {
            "ignore_patterns": [],
            "include_patterns": [],
            "sort_by": "name",
            "sort_order": "asc",
            "truncate": 0,
            "truncate_mode": "smart",
            "truncate_exclude": []
        }

        lens_manager.apply_lens("test", base_config)

        old_stderr = sys.stderr
        sys.stderr = StringIO()
        try:
            lens_manager.print_manifest()
            output = sys.stderr.getvalue()
            self.assertIn("Disabled (full files)", output)
        finally:
            sys.stderr = old_stderr

    def test_lens_manager_print_manifest_with_exclusions(self):
        """Test print_manifest with exclude patterns."""
        import sys
        from io import StringIO

        lens_manager = pm_encoder.LensManager({
            "test": {
                "description": "Test lens",
                "exclude": ["*.pyc", "*.pyo", "*.log", "*.tmp", "*.cache", "*.bak"],
                "include": []
            }
        })

        base_config = {
            "ignore_patterns": [],
            "include_patterns": [],
            "sort_by": "name",
            "sort_order": "asc",
            "truncate": 0,
            "truncate_mode": "smart",
            "truncate_exclude": []
        }

        lens_manager.apply_lens("test", base_config)

        old_stderr = sys.stderr
        sys.stderr = StringIO()
        try:
            lens_manager.print_manifest()
            output = sys.stderr.getvalue()
            self.assertIn("Excluding:", output)
            self.assertIn("+", output)  # Should show (+N more)
        finally:
            sys.stderr = old_stderr

    def test_lens_manager_get_meta_content(self):
        """Test LensManager.get_meta_content()."""
        lens_manager = pm_encoder.LensManager()
        base_config = {
            "ignore_patterns": [],
            "include_patterns": [],
            "sort_by": "name",
            "sort_order": "asc",
            "truncate": 500,
            "truncate_mode": "smart",
            "truncate_exclude": []
        }

        lens_manager.apply_lens("architecture", base_config)
        meta_content = lens_manager.get_meta_content()

        # Should generate meta content
        self.assertIn("Context generated with lens", meta_content)
        self.assertIn("architecture", meta_content)

    def test_truncation_stats_empty(self):
        """Test TruncationStats.print_report() with no files."""
        import sys
        from io import StringIO

        stats = pm_encoder.TruncationStats()

        old_stderr = sys.stderr
        sys.stderr = StringIO()
        try:
            stats.print_report()
            output = sys.stderr.getvalue()
            # Should not print anything for empty stats
            self.assertEqual(output, "")
        finally:
            sys.stderr = old_stderr

    def test_truncation_stats_reduction_pct_zero_original(self):
        """Test _reduction_pct with original=0."""
        stats = pm_encoder.TruncationStats()
        result = stats._reduction_pct(0, 0)
        self.assertEqual(result, 0)

    def test_json_analyzer_short_content(self):
        """Test JSONAnalyzer.get_truncate_ranges with short content."""
        content = '{"key": "value"}'
        analyzer = pm_encoder.JSONAnalyzer()
        ranges, analysis = analyzer.get_truncate_ranges(content, max_lines=100)

        # Should return all content without truncation
        lines = content.split('\n')
        self.assertEqual(ranges, [(1, len(lines))])

    def test_truncation_summary_with_many_entry_points(self):
        """Test truncation summary with >5 entry points."""
        # Create content with many entry points (using if __name__ patterns)
        content = '''import os

def main1():
    pass

def main2():
    pass

def main3():
    pass

def main4():
    pass

def main5():
    pass

def main6():
    pass

if __name__ == "__main__":
    main1()
'''
        analyzer_registry = pm_encoder.LanguageAnalyzerRegistry()
        result, _, _ = pm_encoder.truncate_content(
            content,
            Path("test.py"),
            max_lines=5,
            mode="smart",
            analyzer_registry=analyzer_registry,
            include_summary=True
        )

        # Should include entry points in summary
        self.assertIn("Entry points:", result)

    def test_lens_manager_print_manifest_with_structure_mode(self):
        """Test print_manifest with structure truncation mode."""
        import sys
        from io import StringIO

        lens_manager = pm_encoder.LensManager({
            "test": {
                "description": "Test lens",
                "truncate_mode": "structure",
                "truncate": 100,
                "include": []
            }
        })

        base_config = {
            "ignore_patterns": [],
            "include_patterns": [],
            "sort_by": "name",
            "sort_order": "asc",
            "truncate": 0,
            "truncate_mode": "smart",
            "truncate_exclude": []
        }

        lens_manager.apply_lens("test", base_config)

        old_stderr = sys.stderr
        sys.stderr = StringIO()
        try:
            lens_manager.print_manifest()
            output = sys.stderr.getvalue()
            self.assertIn("signatures only", output)
        finally:
            sys.stderr = old_stderr

    def test_lens_manager_print_manifest_with_limited_truncate(self):
        """Test print_manifest with specific truncate value."""
        import sys
        from io import StringIO

        lens_manager = pm_encoder.LensManager({
            "test": {
                "description": "Test lens",
                "truncate": 500,
                "include": []
            }
        })

        base_config = {
            "ignore_patterns": [],
            "include_patterns": [],
            "sort_by": "name",
            "sort_order": "asc",
            "truncate": 0,
            "truncate_mode": "smart",
            "truncate_exclude": []
        }

        lens_manager.apply_lens("test", base_config)

        old_stderr = sys.stderr
        sys.stderr = StringIO()
        try:
            lens_manager.print_manifest()
            output = sys.stderr.getvalue()
            self.assertIn("500 lines per file", output)
        finally:
            sys.stderr = old_stderr

    def test_lens_manager_print_manifest_with_includes(self):
        """Test print_manifest with include patterns (>5)."""
        import sys
        from io import StringIO

        lens_manager = pm_encoder.LensManager({
            "test": {
                "description": "Test lens",
                "include": ["*.py", "*.js", "*.ts", "*.rs", "*.go", "*.java", "*.cpp"]
            }
        })

        base_config = {
            "ignore_patterns": [],
            "include_patterns": [],
            "sort_by": "name",
            "sort_order": "asc",
            "truncate": 0,
            "truncate_mode": "smart",
            "truncate_exclude": []
        }

        lens_manager.apply_lens("test", base_config)

        old_stderr = sys.stderr
        sys.stderr = StringIO()
        try:
            lens_manager.print_manifest()
            output = sys.stderr.getvalue()
            self.assertIn("Including:", output)
            self.assertIn("+", output)  # Should show (+N more)
        finally:
            sys.stderr = old_stderr

    def test_lens_manager_no_active_lens_manifest(self):
        """Test print_manifest with no active lens."""
        import sys
        from io import StringIO

        lens_manager = pm_encoder.LensManager()

        old_stderr = sys.stderr
        sys.stderr = StringIO()
        try:
            lens_manager.print_manifest()
            output = sys.stderr.getvalue()
            # Should not print anything
            self.assertEqual(output, "")
        finally:
            sys.stderr = old_stderr


class TestAdditionalCoverage(unittest.TestCase):
    """Additional tests for edge cases and coverage."""

    def test_base_language_analyzer_analyze(self):
        """Test base LanguageAnalyzer.analyze() method."""
        analyzer = pm_encoder.LanguageAnalyzer()
        content = "some content\nmore content"
        result = analyzer.analyze(content, Path("test.txt"))

        # Base analyzer should return default structure
        self.assertEqual(result["language"], "Unknown")
        self.assertEqual(result["classes"], [])
        self.assertEqual(result["functions"], [])

    def test_base_language_analyzer_get_truncate_ranges_no_truncation(self):
        """Test get_truncate_ranges when content is short enough."""
        analyzer = pm_encoder.LanguageAnalyzer()
        content = "line1\nline2\nline3"
        ranges, analysis = analyzer.get_truncate_ranges(content, max_lines=10)

        # Should return all lines when content is short
        self.assertEqual(ranges, [(1, 3)])

    def test_unknown_file_extension(self):
        """Test handling of unknown file extension."""
        test_dir = tempfile.mkdtemp()
        try:
            test_path = Path(test_dir)
            unknown_file = test_path / "test.xyz"
            unknown_file.write_text("content")

            output = StringIO()
            pm_encoder.serialize(
                test_path,
                output,
                ignore_patterns=[],
                include_patterns=[],
                sort_by="name",
                sort_order="asc"
            )

            # Should handle unknown file type gracefully
            result = output.getvalue()
            self.assertIn("test.xyz", result)
        finally:
            shutil.rmtree(test_dir)

    def test_json_analyzer_recursion_error(self):
        """Test JSONAnalyzer handling of deeply nested JSON."""
        # Create extremely deeply nested JSON to trigger RecursionError path
        content = "{" * 5000 + "}" * 5000

        analyzer = pm_encoder.JSONAnalyzer()
        lines = content.split('\n')
        result = analyzer.analyze_lines(lines, Path("test.json"))

        # Should fall back to base analyzer on RecursionError
        self.assertIsNotNone(result)

    def test_truncation_summary_with_all_metadata(self):
        """Test truncation summary with classes, functions, imports, etc."""
        content = '''import os
import sys
import json

class Class1:
    pass

class Class2:
    pass

def func1():
    pass

def func2():
    pass

if __name__ == "__main__":
    func1()
'''
        analyzer_registry = pm_encoder.LanguageAnalyzerRegistry()
        result, was_truncated, analysis = pm_encoder.truncate_content(
            content,
            Path("test.py"),
            max_lines=5,
            mode="smart",
            analyzer_registry=analyzer_registry,
            include_summary=True
        )

        # Should include classes, functions, imports in summary
        self.assertIn("Classes", result)
        self.assertIn("Functions", result)
        self.assertIn("Key imports", result)

    def test_main_with_plugin_template_via_main(self):
        """Test --create-plugin through main()."""
        import sys
        from io import StringIO

        original_argv = sys.argv
        old_stdout = sys.stdout
        sys.stdout = StringIO()

        try:
            sys.argv = ["pm_encoder.py", "--create-plugin", "TestLang"]
            pm_encoder.main()
            output = sys.stdout.getvalue()
            self.assertIn("pm_encoder Language Plugin: TestLang", output)
        except SystemExit:
            pass
        finally:
            sys.argv = original_argv
            sys.stdout = old_stdout

    def test_main_with_plugin_prompt_via_main(self):
        """Test --plugin-prompt through main()."""
        import sys
        from io import StringIO

        original_argv = sys.argv
        old_stdout = sys.stdout
        sys.stdout = StringIO()

        try:
            sys.argv = ["pm_encoder.py", "--plugin-prompt", "Kotlin"]
            pm_encoder.main()
            output = sys.stdout.getvalue()
            self.assertIn("AI Prompt", output)
        except SystemExit:
            pass
        finally:
            sys.argv = original_argv
            sys.stdout = old_stdout


class TestCLIAdditional(unittest.TestCase):
    """Additional CLI tests for coverage."""

    def setUp(self):
        """Set up test directory."""
        self.test_dir = tempfile.mkdtemp()
        self.test_path = Path(self.test_dir)

        # Create a test Python file
        test_file = self.test_path / "test.py"
        test_file.write_text("def foo():\n    pass\n")

    def tearDown(self):
        """Clean up test directory."""
        shutil.rmtree(self.test_dir)

    def test_main_with_truncate_stats(self):
        """Test main() with --truncate-stats flag."""
        output_file = self.test_path / "output.txt"
        result = subprocess.run(
            ["./pm_encoder.py", str(self.test_path), "--truncate", "5",
             "--truncate-stats", "-o", str(output_file)],
            capture_output=True,
            text=True
        )
        self.assertEqual(result.returncode, 0)

    def test_main_with_truncate_exclude(self):
        """Test main() with --truncate-exclude flag."""
        output_file = self.test_path / "output.txt"
        result = subprocess.run(
            ["./pm_encoder.py", str(self.test_path), "--truncate", "5",
             "--truncate-exclude", "*.py", "-o", str(output_file)],
            capture_output=True,
            text=True
        )
        self.assertEqual(result.returncode, 0)

    def test_main_with_no_truncate_summary(self):
        """Test main() with --no-truncate-summary flag."""
        output_file = self.test_path / "output.txt"
        result = subprocess.run(
            ["./pm_encoder.py", str(self.test_path), "--truncate", "5",
             "--no-truncate-summary", "-o", str(output_file)],
            capture_output=True,
            text=True
        )
        self.assertEqual(result.returncode, 0)

    def test_main_with_exclude_flag(self):
        """Test main() with --exclude flag."""
        output_file = self.test_path / "output.txt"
        result = subprocess.run(
            ["./pm_encoder.py", str(self.test_path), "--exclude", "*.pyc",
             "-o", str(output_file)],
            capture_output=True,
            text=True
        )
        self.assertEqual(result.returncode, 0)
        self.assertIn("Adding CLI exclude patterns", result.stderr)

    def test_main_with_include_flag(self):
        """Test main() with --include flag."""
        output_file = self.test_path / "output.txt"
        result = subprocess.run(
            ["./pm_encoder.py", str(self.test_path), "--include", "*.py",
             "-o", str(output_file)],
            capture_output=True,
            text=True
        )
        self.assertEqual(result.returncode, 0)
        self.assertIn("Overriding include patterns", result.stderr)

    def test_main_missing_project_root(self):
        """Test main() with missing project_root argument."""
        result = subprocess.run(
            ["./pm_encoder.py"],
            capture_output=True,
            text=True
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("project_root is required", result.stderr)

    def test_main_invalid_project_root(self):
        """Test main() with invalid project_root directory."""
        result = subprocess.run(
            ["./pm_encoder.py", "/nonexistent/directory/path"],
            capture_output=True,
            text=True
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("not a valid directory", result.stderr)

    def test_main_with_invalid_lens(self):
        """Test main() with invalid lens name."""
        output_file = self.test_path / "output.txt"
        result = subprocess.run(
            ["./pm_encoder.py", str(self.test_path), "--lens", "nonexistent",
             "-o", str(output_file)],
            capture_output=True,
            text=True
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Error:", result.stderr)


def run_tests():
    """Run all comprehensive tests."""
    loader = unittest.TestLoader()
    suite = unittest.TestSuite()

    # Add all test classes
    suite.addTests(loader.loadTestsFromTestCase(TestAllLanguageAnalyzers))
    suite.addTests(loader.loadTestsFromTestCase(TestCLIComprehensive))
    suite.addTests(loader.loadTestsFromTestCase(TestEdgeCasesComprehensive))
    suite.addTests(loader.loadTestsFromTestCase(TestConfigurationSystem))
    suite.addTests(loader.loadTestsFromTestCase(TestPerformanceRegression))
    suite.addTests(loader.loadTestsFromTestCase(TestTruncationWithSummary))
    suite.addTests(loader.loadTestsFromTestCase(TestDirectFunctionCalls))
    suite.addTests(loader.loadTestsFromTestCase(TestMainFunctionDirect))
    suite.addTests(loader.loadTestsFromTestCase(TestEdgeCasesForCoverage))
    suite.addTests(loader.loadTestsFromTestCase(TestAdditionalCoverage))
    suite.addTests(loader.loadTestsFromTestCase(TestCLIAdditional))

    # Run tests
    runner = unittest.TextTestRunner(verbosity=2)
    result = runner.run(suite)

    return result.wasSuccessful()


if __name__ == "__main__":
    success = run_tests()
    sys.exit(0 if success else 1)
