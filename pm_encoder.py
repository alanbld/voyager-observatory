#!/usr/bin/env python3
"""
Serializes a project directory's contents into a single text file
using the Plus/Minus format, with robust directory pruning and
filtering capabilities from both a config file and CLI arguments.
"""

import argparse
import hashlib
import json
import sys
from pathlib import Path
from fnmatch import fnmatch

def load_config(config_path: Path | None) -> tuple[list[str], list[str]]:
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

def read_file_content(file_path: Path) -> str | None:
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

def write_pm_format(output_stream, relative_path: Path, content: str):
    """Writes a single file's data in the Plus/Minus format."""
    path_str = relative_path.as_posix()
    
    checksum = hashlib.md5(content.encode('utf-8')).hexdigest()
    
    output_stream.write(f"++++++++++ {path_str} ++++++++++\n")
    output_stream.write(content)
    if not content.endswith('\n'):
        output_stream.write('\n')
    output_stream.write(f"---------- {path_str} {checksum} {path_str} ----------\n")

def serialize(
    project_root: Path,
    output_stream,
    ignore_patterns: list,
    include_patterns: list,
):
    """Walks the project root and serializes files based on filter logic."""
    
    def walk(current_dir: Path):
        # Sort items for deterministic output
        sorted_items = sorted(list(current_dir.iterdir()), key=lambda p: (p.is_file(), p.name.lower()))
        
        for item in sorted_items:
            relative_path = item.relative_to(project_root)
            
            # Improved ignore logic: check if any part of the path matches an ignore pattern.
            if any(fnmatch(part, pattern) for part in relative_path.parts for pattern in ignore_patterns):
                if item.is_dir():
                    print(f"[SKIP DIR] {relative_path.as_posix()} (matches ignore pattern)", file=sys.stderr)
                continue

            if item.is_dir():
                walk(item) # Recurse into subdirectory
            elif item.is_file():
                # If include_patterns is specified, path must match one of them
                if include_patterns:
                    if not any(fnmatch(relative_path.as_posix(), pattern) for pattern in include_patterns):
                        continue # Skip if not in the explicit include list
                
                content = read_file_content(item)
                if content is not None:
                    print(f"[KEEP] {relative_path.as_posix()}", file=sys.stderr)
                    write_pm_format(output_stream, relative_path, content)

    walk(project_root)

def main():
    """Main entry point for the script."""
    parser = argparse.ArgumentParser(
        description="Serialize project files into the Plus/Minus format.",
        formatter_class=argparse.RawTextHelpFormatter
    )
    parser.add_argument("project_root", type=Path, help="The root directory of the project to serialize.")
    parser.add_argument("-o", "--output", type=argparse.FileType('w', encoding='utf-8'), default=sys.stdout,
                        help="Output file path. Defaults to standard output.")
    parser.add_argument("-c", "--config", type=Path, default=Path(".pm_encoder_config.json"),
                        help="Path to a JSON configuration file for ignore/include patterns.\nDefaults to ./.pm_encoder_config.json")
    parser.add_argument("--include", nargs='*', default=[],
                        help="One or more glob patterns for files to include. Overrides config includes.")
    parser.add_argument("--exclude", nargs='*', default=[],
                        help="One or more glob patterns for files/dirs to exclude. Adds to config excludes.")

    args = parser.parse_args()

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
    
    print(f"\nSerializing '{args.project_root}'...", file=sys.stderr)
    
    try:
        serialize(
            args.project_root,
            args.output,
            ignore_patterns,
            include_patterns,
        )
        print(f"\nSuccessfully serialized project.", file=sys.stderr)
    finally:
        if args.output is not sys.stdout:
            args.output.close()

if __name__ == "__main__":
    main()
