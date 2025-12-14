#!/usr/bin/env python3
"""
CLI Contract Generator - Introspects Python argparse to generate the CLI contract.

This script extracts all CLI arguments from pm_encoder.py's argparse configuration
and generates a formal contract (JSON) that defines the expected CLI interface.

The contract is used by verify_cli_parity.py to validate that the Rust implementation
matches the Python reference.

Part of: Research Phase 2.5 - The Interface Parity Protocol
"""
import argparse
import json
import sys
from pathlib import Path
from typing import Any, Dict, List

# Add parent directory to path to import pm_encoder
sys.path.insert(0, str(Path(__file__).parent.parent))


def get_argparse_parser():
    """
    Import pm_encoder and extract its argument parser.

    We need to recreate the parser because pm_encoder.py parses args on import.
    """
    # Import the module to get version
    import pm_encoder
    version = pm_encoder.__version__

    # Recreate the parser (extracted from pm_encoder.py main())
    parser = argparse.ArgumentParser(
        description="Serialize project files into the Plus/Minus format with intelligent truncation.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )

    # Special commands
    parser.add_argument("--create-plugin", type=str, metavar="LANGUAGE",
                        help="Generate a plugin template for LANGUAGE and exit")
    parser.add_argument("--plugin-prompt", type=str, metavar="LANGUAGE",
                        help="Generate an AI prompt to create a plugin for LANGUAGE and exit")
    parser.add_argument("--init-prompt", action="store_true",
                        help="Generate instruction file and CONTEXT.txt for AI CLI integration and exit")
    parser.add_argument("--init-lens", type=str, metavar="LENS", default="architecture",
                        help="Lens to use with --init-prompt (default: architecture)")
    parser.add_argument("--target", type=str, choices=["claude", "gemini"], default="claude",
                        help="Target AI for --init-prompt: 'claude' (CLAUDE.md) or 'gemini' (GEMINI_INSTRUCTIONS.txt) (default: claude)")

    parser.add_argument("--version", action="version", version=f"pm_encoder {version}")
    parser.add_argument("project_root", type=Path, nargs='?', help="The root directory of the project to serialize.")
    parser.add_argument("-o", "--output", type=str, default="-",
                        help="Output file path. Defaults to standard output.")
    parser.add_argument("-c", "--config", type=str, default=".pm_encoder_config.json",
                        help="Path to a JSON configuration file for ignore/include patterns.")
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
    parser.add_argument("--truncate-mode", choices=["simple", "smart", "structure"], default="simple",
                        help="Truncation strategy: 'simple' (keep first N lines), 'smart' (language-aware), or 'structure' (signatures only)")
    parser.add_argument("--truncate-summary", action="store_true", default=True,
                        help="Include analysis summary in truncation marker (default: True)")
    parser.add_argument("--no-truncate-summary", dest="truncate_summary", action="store_false",
                        help="Disable truncation summary")
    parser.add_argument("--truncate-exclude", nargs='*', default=[],
                        help="Never truncate files matching these patterns")
    parser.add_argument("--truncate-stats", action="store_true",
                        help="Show detailed truncation statistics report")
    parser.add_argument("--language-plugins", type=str, metavar="DIR",
                        help="Custom language analyzer plugins directory")

    # Context Lenses
    parser.add_argument("--lens", type=str, metavar="NAME",
                        help="Apply a context lens (architecture|debug|security|onboarding|custom)")

    return parser, version


def extract_arg_type(action) -> str:
    """Extract the argument type as a string."""
    if action.type is not None:
        if action.type == int:
            return "int"
        elif action.type == str:
            return "str"
        elif action.type == Path or str(action.type) == "<class 'pathlib.Path'>":
            return "path"
        else:
            return str(action.type)
    elif isinstance(action, argparse._StoreTrueAction):
        return "bool"
    elif isinstance(action, argparse._StoreFalseAction):
        return "bool"
    elif isinstance(action, argparse._VersionAction):
        return "version"
    elif action.nargs in ('*', '+', '?'):
        return "list" if action.nargs in ('*', '+') else "optional"
    else:
        return "str"


def extract_contract(parser, version: str) -> Dict[str, Any]:
    """Extract the CLI contract from an argparse parser."""
    contract = {
        "schema_version": "1.0",
        "tool_name": "pm_encoder",
        "reference_version": version,
        "description": "CLI contract for pm_encoder - defines expected interface",
        "generated_by": "generate_cli_contract.py",
        "arguments": [],
        "categories": {
            "core": ["project_root", "--output", "--config"],
            "filtering": ["--include", "--exclude"],
            "sorting": ["--sort-by", "--sort-order"],
            "truncation": ["--truncate", "--truncate-mode", "--truncate-summary",
                          "--no-truncate-summary", "--truncate-exclude", "--truncate-stats"],
            "lenses": ["--lens"],
            "plugins": ["--language-plugins", "--create-plugin", "--plugin-prompt"],
            "init": ["--init-prompt", "--init-lens", "--target"],
            "meta": ["--help", "--version"]
        },
        "rust_priority": {
            "critical": ["--help", "--version", "project_root", "--output"],
            "high": ["--config", "--include", "--exclude", "--sort-by", "--sort-order"],
            "medium": ["--truncate", "--truncate-mode", "--lens"],
            "low": ["--truncate-summary", "--truncate-exclude", "--truncate-stats",
                   "--language-plugins", "--create-plugin", "--plugin-prompt",
                   "--init-prompt", "--init-lens", "--target"]
        }
    }

    # Add implicit --help
    contract["arguments"].append({
        "name": "--help",
        "short": "-h",
        "type": "bool",
        "required": False,
        "default": None,
        "choices": None,
        "help_contains": ["help", "message"],
        "category": "meta",
        "rust_status": "implemented"
    })

    for action in parser._actions:
        if isinstance(action, argparse._HelpAction):
            continue  # Already added --help above

        # Get the primary flag name
        if action.option_strings:
            name = action.option_strings[-1]  # Use long form
            short = action.option_strings[0] if len(action.option_strings) > 1 else None
        else:
            name = action.dest
            short = None

        arg_entry = {
            "name": name,
            "short": short if short and short != name else None,
            "type": extract_arg_type(action),
            "required": action.required if hasattr(action, 'required') else False,
            "default": str(action.default) if action.default is not None else None,
            "choices": list(action.choices) if action.choices else None,
            "help_contains": extract_help_keywords(action.help) if action.help else [],
            "metavar": action.metavar,
            "nargs": str(action.nargs) if action.nargs else None,
            "rust_status": "pending"
        }

        # Determine category
        for cat, args in contract["categories"].items():
            if name in args or (short and short in args):
                arg_entry["category"] = cat
                break
        else:
            arg_entry["category"] = "other"

        contract["arguments"].append(arg_entry)

    return contract


def extract_help_keywords(help_text: str) -> List[str]:
    """Extract key words from help text for semantic validation."""
    if not help_text:
        return []

    # Extract meaningful words (skip common words)
    skip_words = {'the', 'a', 'an', 'to', 'for', 'of', 'in', 'on', 'at', 'by', 'or', 'and'}
    words = help_text.lower().split()
    keywords = [w.strip('.,()[]') for w in words if len(w) > 2 and w not in skip_words]

    # Return first few meaningful keywords
    return keywords[:5]


def main():
    """Generate the CLI contract."""
    print("Generating CLI Contract...")

    parser, version = get_argparse_parser()
    contract = extract_contract(parser, version)

    # Write contract to test_vectors
    output_path = Path(__file__).parent.parent / "test_vectors" / "cli_contract.json"
    with open(output_path, "w") as f:
        json.dump(contract, f, indent=2)

    print(f"  Contract generated: {output_path}")
    print(f"  Reference version: {version}")
    print(f"  Total arguments: {len(contract['arguments'])}")
    print(f"  Categories: {len(contract['categories'])}")

    # Print summary by category
    print("\nArguments by category:")
    for cat, args in contract["categories"].items():
        print(f"  {cat}: {len(args)} args")

    # Print priority summary
    print("\nRust implementation priority:")
    for priority, args in contract["rust_priority"].items():
        print(f"  {priority}: {len(args)} args")

    return contract


if __name__ == "__main__":
    main()
