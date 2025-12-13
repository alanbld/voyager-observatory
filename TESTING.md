# Testing Guide for pm_encoder

This document explains how to run tests, add new tests, and maintain the >98% coverage requirement.

## Quick Start

```bash
# Run all tests
make test

# Run tests with coverage report
make coverage

# Run all quality checks
make quality

# Run CI pipeline locally
make ci
```

## Current Test Coverage

**Target**: >98% code coverage
**Current**: Check with `make coverage`

Coverage report locations:
- Terminal output: `make coverage`
- HTML report: `htmlcov/index.html` (after running `make coverage`)

## Test Structure

```
tests/
├── test_pm_encoder.py        # Core functionality tests (10 tests)
├── test_comprehensive.py      # Comprehensive coverage tests (coming soon)
├── test_documentation.py      # Documentation sync tests (coming soon)
└── fixtures/                  # Test data
    ├── python/
    ├── javascript/
    ├── rust/
    ├── shell/
    ├── markdown/
    ├── yaml/
    ├── json/
    └── edge_cases/
```

## Running Tests

### Basic Test Commands

```bash
# Run all tests (verbose)
python3 -m unittest discover -s tests -p 'test_*.py' -v

# Run all tests (quiet)
python3 -m unittest discover -s tests -p 'test_*.py'

# Run specific test file
python3 -m unittest tests.test_pm_encoder

# Run specific test class
python3 -m unittest tests.test_pm_encoder.TestStructureMode

# Run specific test
python3 -m unittest tests.test_pm_encoder.TestStructureMode.test_structure_mode_trigger
```

### Coverage Commands

```bash
# Run with coverage
python3 -m coverage run -m unittest discover -s tests -p 'test_*.py'

# Show coverage report
python3 -m coverage report -m

# Generate HTML report
python3 -m coverage html

# Check if coverage meets 98% threshold
python3 -m coverage report --fail-under=98
```

## Coverage Requirements

### Minimum Coverage

- **Overall**: >98% of pm_encoder.py must be covered
- **Per-file**: No file should drop below 95%
- **Critical paths**: 100% coverage on:
  - All language analyzers
  - Lens system
  - Structure mode logic
  - CLI argument parsing

### What Doesn't Count

Coverage excludes:
- Lines with `# pragma: no cover`
- `if __name__ == "__main__"` blocks (tested via integration)
- Defensive error handling for impossible states

## Writing Tests

### Test Template

```python
import unittest
from pathlib import Path
from io import StringIO
import pm_encoder

class TestMyFeature(unittest.TestCase):
    """Test description."""

    def setUp(self):
        """Set up test fixtures."""
        self.test_data = "sample"

    def tearDown(self):
        """Clean up after tests."""
        pass

    def test_basic_functionality(self):
        """Test basic feature behavior."""
        result = pm_encoder.my_function(self.test_data)
        self.assertEqual(result, expected_value)

    def test_edge_case(self):
        """Test edge case handling."""
        with self.assertRaises(ValueError):
            pm_encoder.my_function(invalid_input)
```

### Test Categories

1. **Unit Tests**: Test individual functions/classes in isolation
2. **Integration Tests**: Test component interactions
3. **Edge Case Tests**: Test boundary conditions, errors, empty inputs
4. **Regression Tests**: Prevent bugs from recurring

### Testing Checklist

When adding a new feature:
- [ ] Add unit tests for new functions/classes
- [ ] Add integration tests for feature workflows
- [ ] Add edge case tests (empty, None, invalid)
- [ ] Update coverage to maintain >98%
- [ ] All tests pass: `make test`
- [ ] Coverage check passes: `make coverage-check`

## Test Fixtures

### Using Fixtures

Test fixtures are in `tests/fixtures/`:

```python
def test_python_analysis(self):
    """Test Python file analysis."""
    fixture_path = Path(__file__).parent / "fixtures" / "python" / "sample.py"
    content = fixture_path.read_text()

    analyzer = pm_encoder.PythonAnalyzer()
    result = analyzer.analyze(content, fixture_path)

    self.assertIn("main", result["functions"])
```

### Creating Fixtures

To add a new fixture:

1. Create file in appropriate `fixtures/` subdirectory
2. Make it realistic but minimal
3. Cover common patterns for that file type
4. Include edge cases if relevant

Example:
```bash
# Create new Python fixture
echo 'def test(): pass' > tests/fixtures/python/minimal.py
```

## Common Testing Patterns

### Testing File Processing

```python
def test_file_serialization(self):
    """Test file serialization."""
    with tempfile.TemporaryDirectory() as tmpdir:
        test_file = Path(tmpdir) / "test.py"
        test_file.write_text("print('hello')")

        output = StringIO()
        pm_encoder.serialize(
            Path(tmpdir),
            output,
            ignore_patterns=[],
            include_patterns=[],
            sort_by="name",
            sort_order="asc"
        )

        result = output.getvalue()
        self.assertIn("++++++++++ test.py ++++++++++", result)
```

### Testing Language Analyzers

```python
def test_analyzer_detects_functions(self):
    """Test function detection."""
    code = """
    def foo():
        pass

    async def bar():
        pass
    """

    analyzer = pm_encoder.PythonAnalyzer()
    lines = code.split('\n')
    result = analyzer.analyze_lines(lines, Path("test.py"))

    self.assertIn("foo", result["functions"])
    self.assertIn("bar", result["functions"])
```

### Testing Structure Mode

```python
def test_structure_preserves_signatures(self):
    """Test structure mode keeps signatures only."""
    code = """
    def process(x):
        result = x * 2
        return result
    """

    analyzer = pm_encoder.PythonAnalyzer()
    lines = code.split('\n')
    ranges = analyzer.get_structure_ranges(lines)

    # Extract kept lines
    kept = []
    for start, end in ranges:
        kept.extend(lines[start-1:end])

    output = '\n'.join(kept)
    self.assertIn("def process(x):", output)
    self.assertNotIn("result = x * 2", output)
```

## Debugging Failed Tests

### Verbose Output

```bash
# Run with verbose output
python3 -m unittest tests.test_pm_encoder -v

# Show print statements
python3 -m unittest tests.test_pm_encoder 2>&1 | cat
```

### Isolate Failing Test

```bash
# Run only the failing test
python3 -m unittest tests.test_pm_encoder.TestClass.test_method -v

# Add debug prints
def test_failing_case(self):
    result = function_under_test()
    print(f"DEBUG: result = {result}")  # Add this
    self.assertEqual(result, expected)
```

### Coverage Gaps

```bash
# Generate HTML coverage report
make coverage

# Open htmlcov/index.html in browser
# Red lines = not covered
# Green lines = covered
```

## Continuous Integration

Tests run automatically on:
- Every push to GitHub
- Every pull request
- Pre-commit hooks (if configured)

### CI Requirements

All of these must pass:
- [ ] All unit tests pass
- [ ] Coverage ≥98%
- [ ] No syntax errors (`make lint`)
- [ ] Self-serialization works (`make self-serialize`)
- [ ] Documentation synchronized (`make docs`)

### Running CI Locally

Before pushing:
```bash
# Run full CI pipeline
make ci

# If this passes, your PR will likely pass CI
```

## Performance Testing

### Benchmarking

```python
import time

def test_performance_large_file(self):
    """Test performance on large files."""
    large_content = "line\n" * 10000

    start = time.time()
    pm_encoder.truncate_content(
        large_content,
        Path("large.py"),
        max_lines=500,
        mode="smart",
        analyzer_registry=pm_encoder.LanguageAnalyzerRegistry(),
        include_summary=True
    )
    elapsed = time.time() - start

    self.assertLess(elapsed, 1.0)  # Should complete in <1 second
```

## Troubleshooting

### Coverage Not Installing

```bash
pip3 install coverage
# or
make install-dev
```

### Tests Pass But Coverage Fails

```bash
# Check what's not covered
make coverage

# Look at htmlcov/index.html for details
# Red lines need test coverage
```

### Import Errors

```bash
# Ensure you're in the project root
cd /path/to/pm_encoder

# Run tests from project root
python3 -m unittest discover -s tests
```

## Best Practices

1. **Test Names**: Use descriptive names that explain what's being tested
2. **One Assert Per Test**: Each test should verify one specific behavior
3. **Independent Tests**: Tests should not depend on each other
4. **Fast Tests**: Keep tests fast (<5s total runtime)
5. **Deterministic**: Tests should always produce the same result
6. **Readable**: Anyone should understand what the test does

## Contributing Tests

When contributing:

1. Run `make test` locally - all must pass
2. Run `make coverage-check` - must be ≥98%
3. Add tests for new features
4. Add tests for bug fixes (regression tests)
5. Document tricky test logic with comments

## Questions?

- Check existing tests in `tests/test_pm_encoder.py` for examples
- See `CONTRIBUTING.md` for contribution guidelines
- Open an issue with the `testing` label

---

**Remember**: Tests are documentation. Write tests that explain how the code should behave.
