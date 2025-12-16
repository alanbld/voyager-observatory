"""
RepoSource Abstraction - The Critical Architectural Decision

This module defines the interface that decouples pm_coach from filesystem access.
All repository data flows through RepoSource, enabling future streaming implementations
without refactoring the differential testing logic.

Design Rationale (Multi-AI Consensus):
- AI Studio: "Design for streaming injection"
- ChatGPT: "Streaming-aware contract without implementing streaming"
- Claude: "RepoSource protocol is the elegant solution"

Usage:
    source = ClonedRepoSource("https://github.com/user/repo")
    for file in source.walk():
        content = source.get_content(file.path)
    source.cleanup()

Future Sources (v1.7.0+):
    - GitHubAPISource: Stream from GitHub API without cloning
    - TarballSource: Process release archives
    - InMemorySource: For testing
"""

from typing import Protocol, Iterator, Optional, runtime_checkable
from dataclasses import dataclass, field
from pathlib import Path
from datetime import datetime


@dataclass
class FileDescriptor:
    """Metadata about a file without loading its content.

    This enables lazy loading - we can filter files by metadata
    before paying the cost of reading content.
    """
    path: str                    # Relative path from repo root
    size: int                    # Size in bytes
    mtime: float                 # Modification time (Unix timestamp)
    is_binary: bool = False      # Binary file detection
    mode: str = "file"           # file, symlink, submodule

    @property
    def extension(self) -> str:
        """Get file extension (lowercase, without dot)."""
        return Path(self.path).suffix.lstrip('.').lower()

    @property
    def name(self) -> str:
        """Get filename without path."""
        return Path(self.path).name

    def __repr__(self) -> str:
        return f"FileDescriptor({self.path!r}, {self.size}B)"


@dataclass
class RepoMetadata:
    """High-level information about a repository.

    Used for reporting and filtering before processing.
    """
    name: str                              # Repository name (e.g., "pm_encoder")
    url: Optional[str] = None              # Clone URL or API endpoint
    default_branch: str = "main"           # Default branch name
    size_bytes: int = 0                    # Total size in bytes
    file_count: int = 0                    # Number of files
    primary_language: Optional[str] = None # Detected primary language

    # Extended metadata (populated during walk)
    languages: dict = field(default_factory=dict)  # {".py": 42, ".rs": 15, ...}
    clone_time_ms: Optional[int] = None            # Time to clone (if applicable)

    def __repr__(self) -> str:
        return f"RepoMetadata({self.name!r}, {self.file_count} files, {self.size_bytes}B)"


@runtime_checkable
class RepoSource(Protocol):
    """Abstract interface for repository data access.

    This is the critical abstraction that enables pm_coach to work with:
    - Cloned repositories (v1.6.0)
    - GitHub API streams (v1.7.0+)
    - Tarballs, zip files, in-memory fixtures

    All pm_coach logic consumes RepoSource, never raw filesystem.
    """

    def get_metadata(self) -> RepoMetadata:
        """Return high-level info about the repo.

        This is called first to decide whether to process the repo
        (e.g., skip if too large, wrong language, etc.)
        """
        ...

    def walk(self) -> Iterator[FileDescriptor]:
        """Yield file metadata without reading content.

        Enables filtering by metadata before paying I/O cost.
        Files are yielded in deterministic order (sorted by path).
        """
        ...

    def get_content(self, path: str) -> str:
        """Lazy load content for a specific file.

        Args:
            path: Relative path as returned by walk()

        Returns:
            File content as string (UTF-8 with latin-1 fallback)

        Raises:
            FileNotFoundError: If path doesn't exist
            UnicodeDecodeError: If file can't be decoded (binary)
        """
        ...

    def cleanup(self) -> None:
        """Clean up temporary resources.

        For ClonedRepoSource: Delete temp directory
        For GitHubAPISource: Close connections
        For InMemorySource: No-op

        Should be safe to call multiple times.
        """
        ...

    # Streaming extensions (v1.6.0+)

    def walk_with_content(self) -> Iterator[tuple]:
        """Stream files with content - true zero-copy streaming.

        Yields:
            (FileDescriptor, content: str) tuples

        This is the streaming-first method for v1.7.0+ sources that can
        provide content without separate I/O (e.g., GitHub API, tarballs).

        Default implementation falls back to walk() + get_content().
        """
        ...

    def supports_streaming(self) -> bool:
        """Return True if this source supports true streaming.

        True streaming means walk_with_content() doesn't require
        two separate I/O operations per file.

        - ClonedRepoSource: False (must read file after stat)
        - GitHubAPISource: True (content in API response)
        - TarballSource: True (content during extraction)
        """
        ...


@dataclass
class FileWithContent:
    """File descriptor bundled with content - for streaming sources."""
    descriptor: FileDescriptor
    content: str

    @property
    def path(self) -> str:
        return self.descriptor.path

    @property
    def size(self) -> int:
        return self.descriptor.size


# Type alias for clarity
RepoSourceType = RepoSource
