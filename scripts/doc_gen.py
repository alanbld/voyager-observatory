#!/usr/bin/env python3
"""
Documentation Generator for pm_encoder

Synchronizes auto-generated content in documentation files.
Looks for markers like <!-- BEGIN_GEN:VERSION --> and replaces content
between BEGIN and END markers.
"""

import re
import sys
from pathlib import Path

# Add parent directory to path to import pm_encoder
sys.path.insert(0, str(Path(__file__).parent.parent))
import pm_encoder


def get_version():
    """Get current version from pm_encoder."""
    return pm_encoder.__version__


def get_help_text():
    """Get --help output from pm_encoder."""
    import subprocess
    result = subprocess.run(
        ["./pm_encoder.py", "--help"],
        capture_output=True,
        text=True,
        cwd=Path(__file__).parent.parent
    )
    return result.stdout


def get_lens_table():
    """Generate markdown table of built-in lenses."""
    lens_manager = pm_encoder.LensManager()
    lenses = lens_manager.BUILT_IN_LENSES

    lines = [
        "| Lens | Description | Mode | Sort |",
        "|------|-------------|------|------|"
    ]

    for name, config in sorted(lenses.items()):
        desc = config.get("description", "")
        mode = config.get("truncate_mode", "smart")
        sort = config.get("sort_by", "name")
        lines.append(f"| **{name}** | {desc} | {mode} | {sort} |")

    return '\n'.join(lines)


def get_language_support():
    """Generate markdown table of supported languages."""
    # This is a simplified version - could be enhanced to auto-detect from analyzers
    return """| Language | Extensions | Smart Mode | Structure Mode |
|----------|-----------|------------|----------------|
| Python | .py, .pyw | ✅ | ✅ |
| JavaScript/TypeScript | .js, .jsx, .ts, .tsx | ✅ | ✅ |
| Rust | .rs | ✅ | ✅ |
| Shell | .sh, .bash, .zsh | ✅ | ✅ |
| Markdown | .md | ✅ | ❌ |
| JSON | .json | ✅ | ❌ |
| YAML | .yaml, .yml | ✅ | ❌ |"""


def process_file(file_path: Path, dry_run=False):
    """Process a file and replace auto-generated sections."""
    if not file_path.exists():
        print(f"Warning: {file_path} does not exist, skipping")
        return False

    content = file_path.read_text()
    original_content = content

    # Define generators
    generators = {
        "VERSION": get_version,
        "LENS_TABLE": get_lens_table,
        "LANGUAGE_SUPPORT": get_language_support,
        # "HELP": get_help_text,  # Commented out as it's large
    }

    # Process each marker type
    modified = False
    for marker_type, generator in generators.items():
        begin_marker = f"<!-- BEGIN_GEN:{marker_type} -->"
        end_marker = f"<!-- END_GEN:{marker_type} -->"

        # Pattern to match content between markers
        pattern = re.compile(
            f"{re.escape(begin_marker)}(.*?){re.escape(end_marker)}",
            re.DOTALL
        )

        matches = list(pattern.finditer(content))
        if matches:
            for match in reversed(matches):  # Process in reverse to preserve positions
                generated_content = generator()
                new_section = f"{begin_marker}\n{generated_content}\n{end_marker}"

                content = content[:match.start()] + new_section + content[match.end():]
                modified = True

    if modified:
        if dry_run:
            print(f"Would update: {file_path}")
        else:
            file_path.write_text(content)
            print(f"Updated: {file_path}")
        return True
    else:
        print(f"No markers found in: {file_path}")
        return False


def main():
    """Main entry point."""
    import argparse

    parser = argparse.ArgumentParser(
        description="Synchronize auto-generated documentation content"
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Show what would be updated without making changes"
    )
    parser.add_argument(
        "files",
        nargs="*",
        help="Files to process (default: README.md, TUTORIAL.md)"
    )

    args = parser.parse_args()

    # Determine files to process
    if args.files:
        files = [Path(f) for f in args.files]
    else:
        project_root = Path(__file__).parent.parent
        files = [
            project_root / "README.md",
            project_root / "TUTORIAL.md",
        ]

    print("pm_encoder Documentation Generator")
    print("=" * 50)

    updated_count = 0
    for file_path in files:
        if process_file(file_path, dry_run=args.dry_run):
            updated_count += 1

    print("=" * 50)
    if args.dry_run:
        print(f"Would update {updated_count} file(s)")
    else:
        print(f"Updated {updated_count} file(s)")

    return 0


if __name__ == "__main__":
    sys.exit(main())
