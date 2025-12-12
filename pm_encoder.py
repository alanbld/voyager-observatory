#!/usr/bin/env python3
"""
Serializes a project directory's contents into a single text file
using the Plus/Minus format, with robust directory pruning,
filtering, and sorting capabilities.
"""

__version__ = "1.1.0"
__author__ = "pm_encoder contributors"
__license__ = "MIT"

import argparse
import hashlib
import json
import os
import re
import sys
from pathlib import Path
from fnmatch import fnmatch
from typing import Optional, Tuple, List, Dict, Any
from collections import defaultdict


# ============================================================================
# LANGUAGE ANALYZER SYSTEM
# ============================================================================

class LanguageAnalyzer:
    """Base class for language-specific code analyzers."""

    SUPPORTED_EXTENSIONS = []
    LANGUAGE_NAME = "Unknown"

    def analyze(self, content: str, file_path: Path) -> Dict[str, Any]:
        """
        Analyze file content and return structured information.

        Returns:
            dict with keys: language, classes, functions, imports, entry_points,
            config_keys, documentation, markers, category, critical_sections
        """
        lines = content.split('\n')
        return self.analyze_lines(lines, file_path)

    def analyze_lines(self, lines: List[str], file_path: Path) -> Dict[str, Any]:
        """
        Analyze pre-split lines (performance optimization).

        Returns:
            dict with keys: language, classes, functions, imports, entry_points,
            config_keys, documentation, markers, category, critical_sections
        """
        return {
            "language": self.LANGUAGE_NAME,
            "classes": [],
            "functions": [],
            "imports": [],
            "entry_points": [],
            "config_keys": [],
            "documentation": [],
            "markers": [],
            "category": "unknown",
            "critical_sections": []  # List of (start_line, end_line) tuples
        }

    def get_truncate_ranges(self, content: str, max_lines: int) -> Tuple[List[Tuple[int, int]], Dict[str, Any]]:
        """
        Determine which line ranges to keep when truncating.

        Returns:
            (ranges, analysis) where ranges is [(start, end), ...] of lines to keep
        """
        lines = content.split('\n')
        total_lines = len(lines)

        if total_lines <= max_lines:
            return [(1, total_lines)], self.analyze_lines(lines, None)

        analysis = self.analyze_lines(lines, None)

        # Default strategy: keep first 40% and last 10%
        keep_first = int(max_lines * 0.4)
        keep_last = int(max_lines * 0.1)

        ranges = [
            (1, keep_first),
            (total_lines - keep_last + 1, total_lines)
        ]

        return ranges, analysis


class PythonAnalyzer(LanguageAnalyzer):
    """Analyzer for Python source files."""

    SUPPORTED_EXTENSIONS = ['.py', '.pyw']
    LANGUAGE_NAME = "Python"

    def analyze_lines(self, lines: List[str], file_path: Path) -> Dict[str, Any]:
        """Analyze pre-split lines (performance optimization)."""
        classes = []
        functions = []
        imports = []
        entry_points = []
        markers = []

        # Regex patterns
        class_pattern = re.compile(r'^\s*class\s+(\w+)')
        func_pattern = re.compile(r'^\s*def\s+(\w+)')
        import_pattern = re.compile(r'^\s*(?:from\s+(\S+)\s+)?import\s+(.+)')
        marker_pattern = re.compile(r'#\s*(TODO|FIXME|XXX|HACK|NOTE):?\s*(.+)', re.IGNORECASE)

        # Check for docstrings by joining lines
        content = '\n'.join(lines)
        has_docstrings = '"""' in content or "'''" in content

        for i, line in enumerate(lines, 1):
            # Classes
            if match := class_pattern.match(line):
                classes.append((match.group(1), i))

            # Functions
            if match := func_pattern.match(line):
                functions.append((match.group(1), i))

            # Imports
            if match := import_pattern.match(line):
                if match.group(1):
                    imports.append(f"from {match.group(1)} import {match.group(2)}")
                else:
                    imports.append(f"import {match.group(2)}")

            # Entry points
            if '__name__' in line and '__main__' in line:
                entry_points.append(('__main__ block', i))

            # Markers
            if match := marker_pattern.search(line):
                markers.append((match.group(1), match.group(2).strip(), i))

        # Categorize
        category = "library"
        if any('__main__' in ep[0] for ep in entry_points):
            category = "application"
        if file_path and ('test' in str(file_path).lower() or str(file_path).startswith('tests/')):
            category = "test"

        return {
            "language": "Python",
            "classes": [c[0] for c in classes],
            "functions": [f[0] for f in functions],
            "imports": list(set([imp.split()[1] for imp in imports[:10]])),  # Unique, first 10
            "entry_points": [ep[0] for ep in entry_points],
            "config_keys": [],
            "documentation": ["docstrings"] if has_docstrings else [],
            "markers": [f"{m[0]} (line {m[2]})" for m in markers[:5]],
            "category": category,
            "critical_sections": [(ep[1], ep[1] + 20) for ep in entry_points]
        }

    def get_truncate_ranges(self, content: str, max_lines: int) -> Tuple[List[Tuple[int, int]], Dict[str, Any]]:
        lines = content.split('\n')
        total_lines = len(lines)

        if total_lines <= max_lines:
            return [(1, total_lines)], self.analyze_lines(lines, None)

        analysis = self.analyze_lines(lines, None)

        # Python-specific strategy: preserve imports, class/function signatures, entry points
        keep_first = int(max_lines * 0.5)  # More for imports and setup
        keep_last = int(max_lines * 0.15)  # For entry points

        ranges = [(1, keep_first)]

        # Add entry point sections
        if analysis["critical_sections"]:
            for start, end in analysis["critical_sections"]:
                if start > keep_first:
                    ranges.append((max(start - 5, keep_first + 1), min(end, total_lines)))

        # Add final section
        if total_lines - keep_last > keep_first:
            ranges.append((total_lines - keep_last + 1, total_lines))

        return ranges, analysis


class JavaScriptAnalyzer(LanguageAnalyzer):
    """Analyzer for JavaScript/TypeScript files."""

    SUPPORTED_EXTENSIONS = ['.js', '.jsx', '.ts', '.tsx', '.mjs', '.cjs']
    LANGUAGE_NAME = "JavaScript/TypeScript"

    def analyze_lines(self, lines: List[str], file_path: Path) -> Dict[str, Any]:
        """Analyze pre-split lines (performance optimization)."""
        classes = []
        functions = []
        imports = []
        exports = []

        # Patterns
        class_pattern = re.compile(r'^\s*(?:export\s+)?class\s+(\w+)')
        func_pattern = re.compile(r'^\s*(?:export\s+)?(?:async\s+)?function\s+(\w+)')
        arrow_func_pattern = re.compile(r'^\s*(?:export\s+)?const\s+(\w+)\s*=\s*(?:async\s*)?\([^)]*\)\s*=>')
        import_pattern = re.compile(r'^\s*import\s+.*?from\s+[\'"]([^\'"]+)[\'"]')
        export_pattern = re.compile(r'^\s*export\s+(?:default\s+)?(.+)')

        # Check for JSDoc and export default
        content = '\n'.join(lines)
        has_jsdoc = '/**' in content
        has_export_default = 'export default' in content

        for line in lines:
            if match := class_pattern.match(line):
                classes.append(match.group(1))

            if match := func_pattern.match(line):
                functions.append(match.group(1))
            elif match := arrow_func_pattern.match(line):
                functions.append(match.group(1))

            if match := import_pattern.match(line):
                imports.append(match.group(1))

            if match := export_pattern.match(line):
                exports.append(match.group(1)[:30])  # Truncate long exports

        category = "library"
        if file_path and ('test' in str(file_path).lower() or 'spec' in str(file_path).lower()):
            category = "test"
        elif exports or has_export_default:
            category = "module"

        return {
            "language": "JavaScript/TypeScript",
            "classes": classes,
            "functions": functions[:20],
            "imports": imports[:10],
            "entry_points": exports[:5],
            "config_keys": [],
            "documentation": ["JSDoc"] if has_jsdoc else [],
            "markers": [],
            "category": category,
            "critical_sections": []
        }

    def get_truncate_ranges(self, content: str, max_lines: int) -> Tuple[List[Tuple[int, int]], Dict[str, Any]]:
        lines = content.split('\n')
        total_lines = len(lines)

        if total_lines <= max_lines:
            return [(1, total_lines)], self.analyze_lines(lines, None)

        analysis = self.analyze_lines(lines, None)

        # Keep imports at top and exports at bottom
        keep_first = int(max_lines * 0.45)
        keep_last = int(max_lines * 0.15)

        ranges = [
            (1, keep_first),
            (total_lines - keep_last + 1, total_lines)
        ]

        return ranges, analysis


class ShellAnalyzer(LanguageAnalyzer):
    """Analyzer for shell scripts."""

    SUPPORTED_EXTENSIONS = ['.sh', '.bash', '.zsh', '.fish']
    LANGUAGE_NAME = "Shell"

    def analyze_lines(self, lines: List[str], file_path: Path) -> Dict[str, Any]:
        """Analyze pre-split lines (performance optimization)."""
        functions = []
        sourced = []
        shebang = None

        func_pattern = re.compile(r'^\s*(?:function\s+)?(\w+)\s*\(\s*\)\s*\{?')
        source_pattern = re.compile(r'^\s*(?:\.|source)\s+(.+)')

        for i, line in enumerate(lines):
            if i == 0 and line.startswith('#!'):
                shebang = line[2:].strip()

            if match := func_pattern.match(line):
                functions.append(match.group(1))

            if match := source_pattern.match(line):
                sourced.append(match.group(1).strip())

        return {
            "language": f"Shell ({shebang.split('/')[-1] if shebang else 'bash'})",
            "classes": [],
            "functions": functions,
            "imports": sourced[:10],
            "entry_points": [shebang] if shebang else [],
            "config_keys": [],
            "documentation": [],
            "markers": [],
            "category": "script",
            "critical_sections": []
        }


class MarkdownAnalyzer(LanguageAnalyzer):
    """Analyzer for Markdown documentation."""

    SUPPORTED_EXTENSIONS = ['.md', '.markdown']
    LANGUAGE_NAME = "Markdown"

    def analyze_lines(self, lines: List[str], file_path: Path) -> Dict[str, Any]:
        """Analyze pre-split lines (performance optimization)."""
        headers = []
        code_blocks = []
        links = []

        header_pattern = re.compile(r'^(#{1,6})\s+(.+)')
        code_block_pattern = re.compile(r'^```(\w+)?')
        link_pattern = re.compile(r'\[([^\]]+)\]\(([^\)]+)\)')

        in_code_block = False
        current_lang = None

        for i, line in enumerate(lines, 1):
            if match := header_pattern.match(line):
                level = len(match.group(1))
                headers.append((level, match.group(2), i))

            if match := code_block_pattern.match(line):
                if not in_code_block:
                    current_lang = match.group(1) or "text"
                    code_blocks.append((current_lang, i))
                in_code_block = not in_code_block

            for match in link_pattern.finditer(line):
                links.append(match.group(2))

        return {
            "language": "Markdown",
            "classes": [],
            "functions": [],
            "imports": links[:10],
            "entry_points": [f"H{h[0]}: {h[1]}" for h in headers[:10]],
            "config_keys": [],
            "documentation": ["headers", "code blocks"],
            "markers": [],
            "category": "documentation",
            "critical_sections": [(h[2], h[2] + 10) for h in headers if h[0] <= 2]  # Keep h1, h2 sections
        }

    def get_truncate_ranges(self, content: str, max_lines: int) -> Tuple[List[Tuple[int, int]], Dict[str, Any]]:
        lines = content.split('\n')
        total_lines = len(lines)

        if total_lines <= max_lines:
            return [(1, total_lines)], self.analyze_lines(lines, None)

        analysis = self.analyze_lines(lines, None)

        # Markdown: keep all headers and first paragraph of each section
        ranges = []
        budget = max_lines

        # Always include critical sections (major headers)
        for start, end in analysis["critical_sections"]:
            if budget > 0:
                section_size = min(end - start + 1, int(max_lines * 0.1))
                ranges.append((start, start + section_size - 1))
                budget -= section_size

        # Fill remaining budget with beginning
        if budget > 0:
            ranges.insert(0, (1, budget))

        return sorted(ranges), analysis


class JSONAnalyzer(LanguageAnalyzer):
    """Analyzer for JSON files."""

    SUPPORTED_EXTENSIONS = ['.json']
    LANGUAGE_NAME = "JSON"

    def analyze_lines(self, lines: List[str], file_path: Path) -> Dict[str, Any]:
        """Analyze pre-split lines (performance optimization)."""
        content = '\n'.join(lines)

        try:
            data = json.loads(content)

            def count_keys(obj, depth=0, max_depth=0):
                try:
                    if isinstance(obj, dict):
                        count = len(obj)
                        max_d = depth
                        for v in obj.values():
                            nested_count, nested_depth = count_keys(v, depth + 1, max_depth)
                            count += nested_count
                            max_d = max(max_d, nested_depth)
                        return count, max_d
                    elif isinstance(obj, list):
                        count = 0
                        max_d = depth
                        for item in obj:
                            nested_count, nested_depth = count_keys(item, depth, max_depth)
                            count += nested_count
                            max_d = max(max_d, nested_depth)
                        return count, max_d
                    return 0, depth
                except RecursionError:
                    # Handle deeply nested JSON structures
                    return 0, depth

            total_keys, max_depth = count_keys(data)
            top_keys = list(data.keys())[:20] if isinstance(data, dict) else []

            return {
                "language": "JSON",
                "classes": [],
                "functions": [],
                "imports": [],
                "entry_points": top_keys,
                "config_keys": top_keys,
                "documentation": [],
                "markers": [],
                "category": "config",
                "critical_sections": [],
                "extra": {
                    "total_keys": total_keys,
                    "max_depth": max_depth,
                    "is_array": isinstance(data, list)
                }
            }
        except (json.JSONDecodeError, RecursionError):
            return super().analyze_lines(lines, file_path)

    def get_truncate_ranges(self, content: str, max_lines: int) -> Tuple[List[Tuple[int, int]], Dict[str, Any]]:
        lines = content.split('\n')
        total_lines = len(lines)

        if total_lines <= max_lines:
            return [(1, total_lines)], self.analyze_lines(lines, None)

        analysis = self.analyze_lines(lines, None)

        # JSON: show structure by keeping top-level and sampling nested
        # Keep first 60% and last 10% to preserve structure
        keep_first = int(max_lines * 0.6)
        keep_last = int(max_lines * 0.1)

        ranges = [
            (1, keep_first),
            (total_lines - keep_last + 1, total_lines)
        ]

        return ranges, analysis


class YAMLAnalyzer(LanguageAnalyzer):
    """Analyzer for YAML files."""

    SUPPORTED_EXTENSIONS = ['.yaml', '.yml']
    LANGUAGE_NAME = "YAML"

    def analyze_lines(self, lines: List[str], file_path: Path) -> Dict[str, Any]:
        """Analyze pre-split lines (performance optimization)."""
        keys = []
        key_pattern = re.compile(r'^(\s*)([a-zA-Z_][\w-]*):')

        for line in lines:
            if match := key_pattern.match(line):
                keys.append(match.group(2))

        return {
            "language": "YAML",
            "classes": [],
            "functions": [],
            "imports": [],
            "entry_points": keys[:15],
            "config_keys": keys[:15],
            "documentation": [],
            "markers": [],
            "category": "config",
            "critical_sections": []
        }


class LanguageAnalyzerRegistry:
    """Registry for managing language analyzers."""

    def __init__(self):
        self.analyzers = {}
        self.default_analyzer = LanguageAnalyzer()

        # Register built-in analyzers
        self._register_builtin()

    def _register_builtin(self):
        """Register all built-in language analyzers."""
        for analyzer_class in [
            PythonAnalyzer,
            JavaScriptAnalyzer,
            ShellAnalyzer,
            MarkdownAnalyzer,
            JSONAnalyzer,
            YAMLAnalyzer
        ]:
            analyzer = analyzer_class()
            for ext in analyzer.SUPPORTED_EXTENSIONS:
                self.analyzers[ext] = analyzer

    def load_plugins(self, plugin_dir: Optional[Path] = None):
        """Load custom language analyzer plugins from directory."""
        if plugin_dir is None:
            plugin_dir = Path.home() / '.pm_encoder' / 'plugins'

        if not plugin_dir.exists():
            return

        # This is a placeholder for the plugin loading system
        # In a full implementation, we would:
        # 1. Scan plugin_dir for .py files
        # 2. Import them safely
        # 3. Look for LanguageAnalyzer subclasses
        # 4. Register them
        pass

    def get_analyzer(self, file_path: Path) -> LanguageAnalyzer:
        """Get the appropriate analyzer for a file."""
        ext = file_path.suffix.lower()
        return self.analyzers.get(ext, self.default_analyzer)

    def get_supported_languages(self) -> List[str]:
        """Return list of supported language names."""
        languages = set()
        for analyzer in self.analyzers.values():
            languages.add(analyzer.LANGUAGE_NAME)
        return sorted(list(languages))


# ============================================================================
# TRUNCATION SYSTEM
# ============================================================================

class TruncationStats:
    """Tracks statistics about truncation operations."""

    def __init__(self):
        self.files_analyzed = 0
        self.files_truncated = 0
        self.original_lines = 0
        self.final_lines = 0
        self.original_size = 0
        self.final_size = 0
        self.by_language = defaultdict(lambda: {"analyzed": 0, "truncated": 0})

    def add_file(self, language: str, original_lines: int, final_lines: int, was_truncated: bool):
        """Record stats for a processed file."""
        self.files_analyzed += 1
        self.original_lines += original_lines
        self.final_lines += final_lines

        self.by_language[language]["analyzed"] += 1

        if was_truncated:
            self.files_truncated += 1
            self.by_language[language]["truncated"] += 1

    def print_report(self):
        """Print a summary report."""
        if self.files_analyzed == 0:
            return

        print("\n" + "="*70, file=sys.stderr)
        print("TRUNCATION REPORT", file=sys.stderr)
        print("="*70, file=sys.stderr)

        print(f"Files analyzed: {self.files_analyzed}", file=sys.stderr)
        print(f"Files truncated: {self.files_truncated} ({self.files_truncated*100//max(self.files_analyzed,1)}%)", file=sys.stderr)
        print(f"Lines: {self.original_lines:,} → {self.final_lines:,} ({self._reduction_pct(self.original_lines, self.final_lines)}% reduction)", file=sys.stderr)

        if self.by_language:
            print(f"\nBy Language:", file=sys.stderr)
            for lang in sorted(self.by_language.keys()):
                stats = self.by_language[lang]
                print(f"  {lang}: {stats['analyzed']} files, {stats['truncated']} truncated", file=sys.stderr)

        # Rough token estimation (1 token ≈ 4 chars)
        orig_tokens = self.original_lines * 40 // 4  # Assume ~40 chars/line
        final_tokens = self.final_lines * 40 // 4
        print(f"\nEstimated tokens: ~{orig_tokens:,} → ~{final_tokens:,} ({self._reduction_pct(orig_tokens, final_tokens)}% reduction)", file=sys.stderr)
        print("="*70, file=sys.stderr)

    def _reduction_pct(self, original, final):
        """Calculate reduction percentage."""
        if original == 0:
            return 0
        return int((original - final) * 100 / original)


def truncate_content(
    content: str,
    file_path: Path,
    max_lines: int,
    mode: str,
    analyzer_registry: LanguageAnalyzerRegistry,
    include_summary: bool
) -> Tuple[str, bool, Dict[str, Any]]:
    """
    Truncate file content intelligently based on language.

    Returns:
        (truncated_content, was_truncated, analysis)
    """
    lines = content.split('\n')
    total_lines = len(lines)

    if total_lines <= max_lines:
        return content, False, {}

    analyzer = analyzer_registry.get_analyzer(file_path)

    if mode == 'simple':
        # Simple mode: just keep first N lines
        truncated = '\n'.join(lines[:max_lines])
        analysis = {"language": "Unknown", "category": "unknown"}

        if include_summary:
            marker = f"\n\n{'='*70}\nTRUNCATED at line {max_lines}/{total_lines} ({(total_lines-max_lines)*100//total_lines}% reduced)\nTo get full content: --include \"{file_path.as_posix()}\" --truncate 0\n{'='*70}\n"
            truncated += marker

        return truncated, True, analysis

    else:  # smart mode
        ranges, analysis = analyzer.get_truncate_ranges(content, max_lines)

        # Extract lines from ranges
        kept_lines = []
        last_end = 0

        for start, end in ranges:
            # Add truncation marker if there's a gap
            if start > last_end + 1 and last_end > 0:
                gap_size = start - last_end - 1
                kept_lines.append(f"\n... [{gap_size} lines omitted] ...\n")

            # Add the lines from this range (convert to 0-indexed)
            kept_lines.extend(lines[start-1:end])
            last_end = end

        truncated = '\n'.join(kept_lines)

        if include_summary:
            # Create detailed truncation marker
            marker_lines = [
                "",
                "=" * 70,
                f"TRUNCATED at line {max_lines}/{total_lines} ({(total_lines-max_lines)*100//total_lines}% reduction)",
                f"Language: {analysis.get('language', 'Unknown')}",
                f"Category: {analysis.get('category', 'unknown')}"
            ]

            if analysis.get('classes'):
                classes_str = ', '.join(analysis['classes'][:10])
                if len(analysis['classes']) > 10:
                    classes_str += f", ... (+{len(analysis['classes'])-10} more)"
                marker_lines.append(f"Classes ({len(analysis['classes'])}): {classes_str}")

            if analysis.get('functions'):
                funcs_str = ', '.join(analysis['functions'][:10])
                if len(analysis['functions']) > 10:
                    funcs_str += f", ... (+{len(analysis['functions'])-10} more)"
                marker_lines.append(f"Functions ({len(analysis['functions'])}): {funcs_str}")

            if analysis.get('imports'):
                imports_str = ', '.join(analysis['imports'][:8])
                if len(analysis['imports']) > 8:
                    imports_str += ", ..."
                marker_lines.append(f"Key imports: {imports_str}")

            if analysis.get('entry_points'):
                marker_lines.append(f"Entry points: {', '.join(str(ep) for ep in analysis['entry_points'][:5])}")

            if analysis.get('markers'):
                marker_lines.append(f"Markers: {', '.join(analysis['markers'][:5])}")

            marker_lines.append("")
            marker_lines.append(f"To get full content: --include \"{file_path.as_posix()}\" --truncate 0")
            marker_lines.append("=" * 70)

            truncated += '\n' + '\n'.join(marker_lines)

        return truncated, True, analysis


def load_config(config_path: Optional[Path]) -> Tuple[List[str], List[str]]:
    """Loads ignore and include patterns from a JSON config file."""
    # Default patterns to ignore common build artifacts and vcs folders
    ignore_patterns = [".git", "target", ".venv", "__pycache__", "*.pyc", "*.swp"]
    include_patterns = []

    if config_path and config_path.is_file():
        try:
            with config_path.open("r", encoding="utf-8") as f:
                data = json.load(f)
                ignore_patterns.extend(data.get("ignore_patterns", []))
                include_patterns.extend(data.get("include_patterns", []))
        except (json.JSONDecodeError, IOError) as e:
            print(f"Warning: Could not read or parse {config_path}: {e}", file=sys.stderr)

    return ignore_patterns, include_patterns

def is_binary(file_path: Path) -> bool:
    """
    Checks if a file is likely binary by reading a chunk and looking for null bytes.
    This is a common and effective heuristic.
    """
    try:
        with file_path.open('rb') as f:
            chunk = f.read(1024)  # Read the first 1KB
        return b'\x00' in chunk
    except IOError:
        return True # If we can't read it, treat it as problematic

def read_file_content(file_path: Path) -> Optional[str]:
    """
    Reads a file's content, skipping binary files and large files.
    Tries UTF-8 then latin-1 encoding for text files.
    """
    try:
        # 1. Check for large files first
        if file_path.stat().st_size > 5 * 1024 * 1024: # 5 MB limit
            print(f"[SKIP] {file_path.as_posix()} (file too large)", file=sys.stderr)
            return None

        # 2. Check for binary files using the null-byte heuristic
        if is_binary(file_path):
            print(f"[SKIP] {file_path.as_posix()} (likely binary)", file=sys.stderr)
            return None

        # 3. If it seems like a text file, read it
        return file_path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        try:
            # Fallback for other text encodings that are not UTF-8
            return file_path.read_text(encoding="latin-1")
        except (IOError, UnicodeDecodeError) as e:
            print(f"Error: Could not read file {file_path}: {e}. Skipping.", file=sys.stderr)
            return None
    except IOError as e:
        print(f"Error: Could not read file {file_path}: {e}. Skipping.", file=sys.stderr)
        return None

def write_pm_format(output_stream, relative_path: Path, content: str, was_truncated: bool = False, original_lines: int = 0):
    """Writes a single file's data in the Plus/Minus format."""
    path_str = relative_path.as_posix()

    checksum = hashlib.md5(content.encode('utf-8')).hexdigest()

    # Header with optional truncation info
    if was_truncated:
        output_stream.write(f"++++++++++ {path_str} [TRUNCATED: {original_lines} lines] ++++++++++\n")
    else:
        output_stream.write(f"++++++++++ {path_str} ++++++++++\n")

    output_stream.write(content)
    if not content.endswith('\n'):
        output_stream.write('\n')

    # Footer with optional truncation marker
    if was_truncated:
        output_stream.write(f"---------- {path_str} [TRUNCATED:{original_lines}→{len(content.split(chr(10)))}] {checksum} {path_str} ----------\n")
    else:
        output_stream.write(f"---------- {path_str} {checksum} {path_str} ----------\n")

def serialize(
    project_root: Path,
    output_stream,
    ignore_patterns: list,
    include_patterns: list,
    sort_by: str,
    sort_order: str,
    truncate_lines: int = 0,
    truncate_mode: str = 'simple',
    truncate_summary: bool = True,
    truncate_exclude: list = None,
    show_stats: bool = False,
    language_plugins_dir: Optional[Path] = None,
):
    """Collects, sorts, and serializes files based on specified criteria."""
    files_to_process = []

    # Initialize language analyzer registry
    analyzer_registry = LanguageAnalyzerRegistry()
    if language_plugins_dir:
        analyzer_registry.load_plugins(language_plugins_dir)

    # Initialize truncation stats
    stats = TruncationStats() if (truncate_lines > 0 or show_stats) else None

    if truncate_exclude is None:
        truncate_exclude = []

    # Step 1: Collect all valid file paths recursively
    def collect_files(current_dir: Path):
        try:
            # Sort items locally for deterministic traversal, preventing filesystem order dependency
            sorted_items = sorted(list(current_dir.iterdir()), key=lambda p: p.name.lower())
        except OSError as e:
            print(f"Warning: Could not read directory {current_dir}: {e}", file=sys.stderr)
            return

        for item in sorted_items:
            relative_path = item.relative_to(project_root)

            if any(fnmatch(part, pattern) for part in relative_path.parts for pattern in ignore_patterns):
                if item.is_dir():
                    print(f"[SKIP DIR] {relative_path.as_posix()} (matches ignore pattern)", file=sys.stderr)
                continue

            if item.is_dir():
                collect_files(item)
            elif item.is_file():
                if include_patterns and not any(fnmatch(relative_path.as_posix(), pattern) for pattern in include_patterns):
                    continue
                files_to_process.append(item)

    collect_files(project_root)

    # Step 2: Sort the collected list of files globally
    reverse_order = sort_order == 'desc'
    sort_key_func = None

    if sort_by == 'name':
        sort_key_func = lambda p: p.relative_to(project_root).as_posix()
    elif sort_by == 'mtime':
        sort_key_func = lambda p: p.stat().st_mtime
    elif sort_by == 'ctime':
        sort_key_func = lambda p: p.stat().st_ctime

    print(f"\nSorting {len(files_to_process)} files by {sort_by} ({sort_order})...", file=sys.stderr)
    files_to_process.sort(key=sort_key_func, reverse=reverse_order)

    # Step 3: Process and write the sorted files
    for file_path in files_to_process:
        relative_path = file_path.relative_to(project_root)
        content = read_file_content(file_path)

        if content is not None:
            original_lines = len(content.split('\n'))
            was_truncated = False
            analysis = {}

            # Apply truncation if enabled
            if truncate_lines > 0:
                # Check if file should be excluded from truncation
                should_truncate = not any(
                    fnmatch(relative_path.as_posix(), pattern)
                    for pattern in truncate_exclude
                )

                if should_truncate:
                    content, was_truncated, analysis = truncate_content(
                        content,
                        relative_path,
                        truncate_lines,
                        truncate_mode,
                        analyzer_registry,
                        truncate_summary
                    )

            # Record stats
            if stats:
                final_lines = len(content.split('\n'))
                language = analysis.get('language', 'Unknown') if analysis else 'Unknown'
                stats.add_file(language, original_lines, final_lines, was_truncated)

            # Print status
            if was_truncated:
                print(f"[TRUNCATED] {relative_path.as_posix()} ({original_lines} → {len(content.split(chr(10)))} lines)", file=sys.stderr)
            else:
                print(f"[KEEP] {relative_path.as_posix()}", file=sys.stderr)

            # Write to output
            write_pm_format(output_stream, relative_path, content, was_truncated, original_lines)

    # Print stats if requested
    if stats and show_stats:
        stats.print_report()

def create_plugin_template(language_name: str):
    """Generate a plugin template file."""
    template = f'''"""
pm_encoder Language Plugin: {language_name}
Analyzer for {language_name} files

Usage:
    1. Save this file to ~/.pm_encoder/plugins/{language_name.lower()}_analyzer.py
    2. Update SUPPORTED_EXTENSIONS with your language's file extensions
    3. Implement the analyze() method with language-specific parsing
    4. Test: ./pm_encoder.py . --truncate 500 --language-plugins ~/.pm_encoder/plugins/
"""

import re
from pathlib import Path
from typing import Dict, List, Tuple, Any


class LanguageAnalyzer:
    """Language analyzer for {language_name}."""

    SUPPORTED_EXTENSIONS = ['.{language_name.lower()}']  # UPDATE THIS
    LANGUAGE_NAME = "{language_name}"

    def analyze(self, content: str, file_path: Path) -> Dict[str, Any]:
        """
        Analyze {language_name} file content and return structured information.

        Returns:
            dict with keys: language, classes, functions, imports, entry_points,
            config_keys, documentation, markers, category, critical_sections
        """
        lines = content.split('\\n')

        classes = []
        functions = []
        imports = []
        entry_points = []
        markers = []

        # TODO: Add language-specific regex patterns
        # Example patterns (customize for your language):
        # class_pattern = re.compile(r'^\\s*class\\s+(\\w+)')
        # func_pattern = re.compile(r'^\\s*function\\s+(\\w+)')
        # import_pattern = re.compile(r'^\\s*import\\s+(.+)')

        for i, line in enumerate(lines, 1):
            # TODO: Parse classes
            # if match := class_pattern.match(line):
            #     classes.append(match.group(1))

            # TODO: Parse functions
            # if match := func_pattern.match(line):
            #     functions.append(match.group(1))

            # TODO: Parse imports
            # if match := import_pattern.match(line):
            #     imports.append(match.group(1))

            pass  # Remove this when you add parsing logic

        # TODO: Categorize the file
        category = "unknown"  # Options: application, library, test, config, documentation, script

        return {{
            "language": self.LANGUAGE_NAME,
            "classes": classes,
            "functions": functions,
            "imports": imports[:10],
            "entry_points": entry_points,
            "config_keys": [],
            "documentation": [],
            "markers": markers,
            "category": category,
            "critical_sections": []  # List of (start_line, end_line) tuples
        }}

    def get_truncate_ranges(self, content: str, max_lines: int) -> Tuple[List[Tuple[int, int]], Dict[str, Any]]:
        """
        Determine which line ranges to keep when truncating.

        Returns:
            (ranges, analysis) where ranges is [(start, end), ...] of lines to keep
        """
        lines = content.split('\\n')
        total_lines = len(lines)

        if total_lines <= max_lines:
            return [(1, total_lines)], self.analyze(content, None)

        analysis = self.analyze(content, None)

        # Default strategy: keep first 40% and last 10%
        # TODO: Customize based on language structure
        keep_first = int(max_lines * 0.4)
        keep_last = int(max_lines * 0.1)

        ranges = [
            (1, keep_first),
            (total_lines - keep_last + 1, total_lines)
        ]

        return ranges, analysis
'''

    print(template)
    print(f"\n# Plugin template generated for {language_name}", file=sys.stderr)
    print(f"# Save to: ~/.pm_encoder/plugins/{language_name.lower()}_analyzer.py", file=sys.stderr)


def create_plugin_prompt(language_name: str):
    """Generate an AI prompt for creating a plugin."""
    prompt = f'''# AI Prompt: Create pm_encoder Language Plugin for {language_name}

I need a language analyzer plugin for pm_encoder to support {language_name} files.

## Requirements

Create a Python class that analyzes {language_name} source files and extracts:

1. **Classes/Types**: Detect class, struct, interface, or type definitions
2. **Functions/Methods**: Identify function/method declarations
3. **Imports/Dependencies**: Find import/require/use statements
4. **Entry Points**: Locate main functions, exports, or program entry points
5. **File Category**: Classify as application|library|test|config|documentation|script
6. **Critical Sections**: Identify important code ranges to preserve during truncation

## Plugin Interface

```python
class LanguageAnalyzer:
    SUPPORTED_EXTENSIONS = ['.ext']  # File extensions for {language_name}
    LANGUAGE_NAME = "{language_name}"

    def analyze(self, content: str, file_path: Path) -> Dict[str, Any]:
        # Return dict with: language, classes, functions, imports,
        # entry_points, category, critical_sections
        pass

    def get_truncate_ranges(self, content: str, max_lines: int) -> Tuple[List[Tuple[int, int]], Dict[str, Any]]:
        # Return line ranges to keep when truncating
        # Strategy: preserve imports, class/function signatures, entry points
        pass
```

## Constraints

- Use regex patterns only (no external dependencies, no AST parsing)
- Compatible with Python 3.6+
- Fast heuristics (aim for <100ms per file)
- Good enough > perfect (80/20 rule applies)

## Example {language_name} File

Please analyze this typical {language_name} file structure:

```{language_name.lower()}
# TODO: Add example {language_name} code here
# Include: imports, class definitions, functions, entry points
```

## Deliverable

Provide a complete Python plugin file following the template generated by:
`./pm_encoder.py --create-plugin {language_name.lower()}`

The plugin should intelligently truncate {language_name} files while preserving:
- Import/dependency declarations
- Class and function signatures
- Entry point code
- Critical business logic sections
'''

    print(prompt)


def main():
    """Main entry point for the script."""
    parser = argparse.ArgumentParser(
        description="Serialize project files into the Plus/Minus format with intelligent truncation.",
        formatter_class=argparse.RawTextHelpFormatter,
        epilog="""
Examples:
  # Basic serialization
  ./pm_encoder.py . -o output.txt

  # With smart truncation (500 lines per file)
  ./pm_encoder.py . --truncate 500 --truncate-mode smart -o output.txt

  # Truncate with stats
  ./pm_encoder.py . --truncate 300 --truncate-stats

  # Exclude certain files from truncation
  ./pm_encoder.py . --truncate 500 --truncate-exclude "*.md" "LICENSE"

  # Create a plugin template
  ./pm_encoder.py --create-plugin rust

  # Generate AI prompt for plugin creation
  ./pm_encoder.py --plugin-prompt kotlin
        """
    )

    # Special commands (don't require project_root)
    parser.add_argument("--create-plugin", type=str, metavar="LANGUAGE",
                        help="Generate a plugin template for LANGUAGE and exit")
    parser.add_argument("--plugin-prompt", type=str, metavar="LANGUAGE",
                        help="Generate an AI prompt to create a plugin for LANGUAGE and exit")

    parser.add_argument("--version", action="version", version=f"pm_encoder {__version__}")
    parser.add_argument("project_root", type=Path, nargs='?', help="The root directory of the project to serialize.")
    parser.add_argument("-o", "--output", type=argparse.FileType('w', encoding='utf-8'), default=sys.stdout,
                        help="Output file path. Defaults to standard output.")
    parser.add_argument("-c", "--config", type=Path, default=Path(".pm_encoder_config.json"),
                        help="Path to a JSON configuration file for ignore/include patterns.\nDefaults to ./.pm_encoder_config.json")
    parser.add_argument("--include", nargs='*', default=[],
                        help="One or more glob patterns for files to include. Overrides config includes.")
    parser.add_argument("--exclude", nargs='*', default=[],
                        help="One or more glob patterns for files/dirs to exclude. Adds to config excludes.")
    parser.add_argument("--sort-by", choices=["name", "mtime", "ctime"], default="name",
                        help="Sort files by 'name' (default), 'mtime' (modification time), or 'ctime' (creation time).")
    parser.add_argument("--sort-order", choices=["asc", "desc"], default="asc",
                        help="Sort order: 'asc' (ascending, default) or 'desc' (descending).")

    # Truncation options
    parser.add_argument("--truncate", type=int, metavar="N", default=0,
                        help="Truncate files exceeding N lines (default: 0 = no truncation)")
    parser.add_argument("--truncate-mode", choices=["simple", "smart"], default="simple",
                        help="Truncation strategy: 'simple' (keep first N lines) or 'smart' (language-aware)")
    parser.add_argument("--truncate-summary", action="store_true", default=True,
                        help="Include analysis summary in truncation marker (default: True)")
    parser.add_argument("--no-truncate-summary", dest="truncate_summary", action="store_false",
                        help="Disable truncation summary")
    parser.add_argument("--truncate-exclude", nargs='*', default=[],
                        help="Never truncate files matching these patterns")
    parser.add_argument("--truncate-stats", action="store_true",
                        help="Show detailed truncation statistics report")
    parser.add_argument("--language-plugins", type=Path, metavar="DIR",
                        help="Custom language analyzer plugins directory")

    args = parser.parse_args()

    # Handle special commands that don't need project_root
    if args.create_plugin:
        create_plugin_template(args.create_plugin)
        return

    if args.plugin_prompt:
        create_plugin_prompt(args.plugin_prompt)
        return

    # Validate project_root is provided for normal operations
    if not args.project_root:
        parser.error("project_root is required (unless using --create-plugin or --plugin-prompt)")

    if not args.project_root.is_dir():
        print(f"Error: Project root '{args.project_root}' is not a valid directory.", file=sys.stderr)
        sys.exit(1)

    ignore_patterns, include_patterns = load_config(args.config)

    # Handle command-line overrides
    if args.include:
        print(f"Overriding include patterns with CLI arguments: {args.include}", file=sys.stderr)
        include_patterns = args.include

    if args.exclude:
        print(f"Adding CLI exclude patterns: {args.exclude}", file=sys.stderr)
        ignore_patterns.extend(args.exclude)

    # Show truncation info
    if args.truncate > 0:
        print(f"\nTruncation enabled: {args.truncate} lines per file ({args.truncate_mode} mode)", file=sys.stderr)
        if args.truncate_exclude:
            print(f"Truncation exclusions: {args.truncate_exclude}", file=sys.stderr)

    print(f"\nSerializing '{args.project_root}'...", file=sys.stderr)

    try:
        serialize(
            args.project_root,
            args.output,
            ignore_patterns,
            include_patterns,
            args.sort_by,
            args.sort_order,
            truncate_lines=args.truncate,
            truncate_mode=args.truncate_mode,
            truncate_summary=args.truncate_summary,
            truncate_exclude=args.truncate_exclude,
            show_stats=args.truncate_stats or args.truncate > 0,
            language_plugins_dir=args.language_plugins,
        )
        print(f"\nSuccessfully serialized project.", file=sys.stderr)
    finally:
        if args.output is not sys.stdout:
            args.output.close()

if __name__ == "__main__":
    main()
