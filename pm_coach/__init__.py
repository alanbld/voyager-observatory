"""
pm_coach - Differential Testing Framework for pm_encoder

A tool that compares Python and Rust pm_encoder output on real-world
repositories to discover edge cases and auto-generate test vectors.

Architecture: Source-Agnostic design allows future streaming without refactoring.

v0.2.0: Added streaming protocol extensions (walk_with_content, supports_streaming)
"""

__version__ = "0.2.0"

from .source import RepoSource, FileDescriptor, RepoMetadata, FileWithContent

__all__ = ["RepoSource", "FileDescriptor", "RepoMetadata", "FileWithContent"]
