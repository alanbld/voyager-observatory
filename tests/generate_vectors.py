#!/usr/bin/env python3
"""
Test Vector Generator for Rust v0.2.0 Validation

This script generates JSON test vectors that define expected behavior
for the Rust engine to validate against.

Each test vector includes:
- Input configuration (files, settings)
- Expected output (format, content, checksums)
- Metadata (name, description, version)

Usage:
    python3 tests/generate_vectors.py

Output:
    Creates/updates JSON files in test_vectors/
"""

import json
import os
import sys
import tempfile
import hashlib
from pathlib import Path

# Add parent directory to path to import pm_encoder
sys.path.insert(0, str(Path(__file__).parent.parent))
import pm_encoder


def generate_basic_serialization_vector():
    """
    Vector 1: Basic file serialization
    
    Tests:
    - Directory traversal finds the file
    - Plus/Minus format is correct
    - MD5 checksum is calculated correctly
    - Full content is included
    """
    print("Generating basic_serialization.json...")
    
    # Use the sample Rust fixture
    fixture_path = Path("tests/fixtures/rust/sample.rs")
    
    if not fixture_path.exists():
        print(f"ERROR: Fixture not found: {fixture_path}")
        return None
    
    # Read the fixture content
    with open(fixture_path, 'r') as f:
        content = f.read()
    
    # Calculate MD5 (same algorithm pm_encoder uses)
    md5_hash = hashlib.md5(content.encode('utf-8')).hexdigest()
    
    # Run pm_encoder on the fixtures directory
    with tempfile.NamedTemporaryFile(mode='w', delete=False, suffix='.txt') as tmp:
        tmp_path = tmp.name
    
    try:
        # Serialize just the rust fixtures
        import subprocess
        result = subprocess.run(
            ['./pm_encoder.py', 'tests/fixtures/rust', '-o', tmp_path],
            capture_output=True,
            text=True
        )
        
        if result.returncode != 0:
            print(f"ERROR: pm_encoder failed: {result.stderr}")
            return None
        
        # Read the generated output
        with open(tmp_path, 'r') as f:
            python_output = f.read()
    
    finally:
        os.unlink(tmp_path)
    
    # Create the test vector
    vector = {
        "name": "basic_serialization",
        "description": "Single Rust file with complete content - validates Plus/Minus format and MD5 checksum",
        "version": "0.2.0",
        "input": {
            "files": {
                "sample.rs": content
            },
            "config": {
                "sort_by": "name",
                "sort_order": "asc"
            }
        },
        "expected": {
            "format": "plus_minus",
            "files_count": 1,
            "contains": [
                "++++++++++ sample.rs ++++++++++",
                "// Sample Rust file for testing serialization",
                "use std::collections::HashMap;",
                "pub struct Config {",
                f"---------- sample.rs {md5_hash} sample.rs ----------"
            ],
            "checksum": md5_hash,
            "full_output": python_output
        },
        "metadata": {
            "created_by": "generate_vectors.py",
            "python_version": pm_encoder.__version__,
            "purpose": "Rust v0.2.0 core serialization validation"
        }
    }
    
    return vector


def generate_binary_detection_vector():
    """
    Vector 2: Binary file detection
    
    Tests:
    - Files with null bytes are detected as binary
    - Binary files are skipped from output
    - Non-binary files are included
    """
    print("Generating binary_detection.json...")
    
    # Create temporary directory with binary and text files
    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir_path = Path(tmpdir)
        
        # Create a binary file (with null bytes)
        binary_file = tmpdir_path / "image.bin"
        with open(binary_file, 'wb') as f:
            f.write(b'\x00\x01\x02\x03\xff\xfe\xfd\xfc\x00\x00')
        
        # Create a text file
        text_file = tmpdir_path / "readme.txt"
        text_content = "This is a regular text file.\nNo null bytes here!\n"
        with open(text_file, 'w') as f:
            f.write(text_content)
        
        # Calculate MD5 for text file
        text_md5 = hashlib.md5(text_content.encode('utf-8')).hexdigest()
        
        # Run pm_encoder
        with tempfile.NamedTemporaryFile(mode='w', delete=False, suffix='.txt') as tmp:
            tmp_path = tmp.name
        
        try:
            import subprocess
            result = subprocess.run(
                ['./pm_encoder.py', str(tmpdir_path), '-o', tmp_path],
                capture_output=True,
                text=True
            )
            
            if result.returncode != 0:
                print(f"ERROR: pm_encoder failed: {result.stderr}")
                return None
            
            with open(tmp_path, 'r') as f:
                python_output = f.read()
        
        finally:
            os.unlink(tmp_path)
        
        # Create the test vector
        vector = {
            "name": "binary_detection",
            "description": "Mixed binary and text files - validates binary file skipping",
            "version": "0.2.0",
            "input": {
                "files": {
                    "image.bin": "[BINARY: \\x00\\x01\\x02\\x03\\xff\\xfe\\xfd\\xfc\\x00\\x00]",
                    "readme.txt": text_content
                },
                "config": {}
            },
            "expected": {
                "format": "plus_minus",
                "files_included": ["readme.txt"],
                "files_skipped": ["image.bin"],
                "contains": [
                    "++++++++++ readme.txt ++++++++++",
                    "This is a regular text file.",
                    f"---------- readme.txt {text_md5} readme.txt ----------"
                ],
                "not_contains": [
                    "image.bin"
                ],
                "full_output": python_output
            },
            "metadata": {
                "created_by": "generate_vectors.py",
                "python_version": pm_encoder.__version__,
                "purpose": "Rust v0.2.0 binary detection validation"
            }
        }
        
        return vector


def generate_large_file_skip_vector():
    """
    Vector 3: Large file skipping
    
    Tests:
    - Files over 5MB are skipped
    - Smaller files are included
    - Size threshold is respected
    """
    print("Generating large_file_skip.json...")
    
    # Create temporary directory with large and small files
    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir_path = Path(tmpdir)
        
        # Create a large file (6MB)
        large_file = tmpdir_path / "large_data.txt"
        # Write ~6MB of data
        with open(large_file, 'w') as f:
            # Each line is ~100 chars, need ~60,000 lines for 6MB
            for i in range(60000):
                f.write(f"Line {i:05d}: This is line {i} of the large file with some padding text to reach 100 chars.\n")
        
        # Create a small file
        small_file = tmpdir_path / "small.txt"
        small_content = "This is a small file.\nIt will be included in the output.\n"
        with open(small_file, 'w') as f:
            f.write(small_content)
        
        # Calculate MD5 for small file
        small_md5 = hashlib.md5(small_content.encode('utf-8')).hexdigest()
        
        # Run pm_encoder
        with tempfile.NamedTemporaryFile(mode='w', delete=False, suffix='.txt') as tmp:
            tmp_path = tmp.name
        
        try:
            import subprocess
            result = subprocess.run(
                ['./pm_encoder.py', str(tmpdir_path), '-o', tmp_path],
                capture_output=True,
                text=True
            )
            
            if result.returncode != 0:
                print(f"ERROR: pm_encoder failed: {result.stderr}")
                return None
            
            with open(tmp_path, 'r') as f:
                python_output = f.read()
        
        finally:
            os.unlink(tmp_path)
        
        # Get actual size of large file
        large_size = os.path.getsize(large_file)
        
        # Create the test vector
        vector = {
            "name": "large_file_skip",
            "description": "Mixed large (>5MB) and small files - validates size-based skipping",
            "version": "0.2.0",
            "input": {
                "files": {
                    "large_data.txt": f"[LARGE FILE: {large_size} bytes, ~60000 lines]",
                    "small.txt": small_content
                },
                "config": {
                    "max_file_size": 5242880  # 5MB in bytes
                }
            },
            "expected": {
                "format": "plus_minus",
                "files_included": ["small.txt"],
                "files_skipped": ["large_data.txt"],
                "skip_reason": "file_too_large",
                "contains": [
                    "++++++++++ small.txt ++++++++++",
                    "This is a small file.",
                    f"---------- small.txt {small_md5} small.txt ----------"
                ],
                "not_contains": [
                    "large_data.txt",
                    "Line 00000:"
                ],
                "full_output": python_output
            },
            "metadata": {
                "created_by": "generate_vectors.py",
                "python_version": pm_encoder.__version__,
                "purpose": "Rust v0.2.0 large file handling validation",
                "large_file_size_bytes": large_size
            }
        }
        
        return vector


def save_vector(vector, filename):
    """Save a test vector to test_vectors/ directory"""
    if vector is None:
        print(f"  ❌ Skipped {filename} (generation failed)")
        return False
    
    output_path = Path("test_vectors") / filename
    
    # Ensure directory exists
    output_path.parent.mkdir(parents=True, exist_ok=True)
    
    # Write JSON with nice formatting
    with open(output_path, 'w') as f:
        json.dump(vector, f, indent=2)
    
    print(f"  ✅ Created {output_path}")
    return True


def main():
    """Generate all test vectors for Rust v0.2.0"""
    print("=" * 60)
    print("Test Vector Generator - Rust v0.2.0 (Core Serialization)")
    print("=" * 60)
    print()
    
    # Generate each vector
    vectors = [
        (generate_basic_serialization_vector(), "basic_serialization.json"),
        (generate_binary_detection_vector(), "binary_detection.json"),
        (generate_large_file_skip_vector(), "large_file_skip.json"),
    ]
    
    # Save all vectors
    success_count = 0
    for vector, filename in vectors:
        if save_vector(vector, filename):
            success_count += 1
    
    print()
    print("=" * 60)
    print(f"Generated {success_count}/{len(vectors)} test vectors")
    print("=" * 60)
    
    if success_count == len(vectors):
        print()
        print("✅ All test vectors generated successfully!")
        print()
        print("Next steps:")
        print("  1. Review test_vectors/*.json")
        print("  2. git add test_vectors/")
        print("  3. git commit -m 'feat: Add v0.2.0 test vectors'")
        print("  4. Start implementing Rust v0.2.0 to pass these vectors")
        return 0
    else:
        print()
        print("⚠️  Some test vectors failed to generate")
        return 1


if __name__ == "__main__":
    sys.exit(main())
