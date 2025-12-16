"""
ClonedRepoSource - Shallow Git Clone Implementation

The v1.6.0 reference implementation of RepoSource that clones repos
to a temporary directory for analysis.

Optimizations:
- Shallow clone (--depth 1): Only latest commit
- Blob filter (--filter=blob:none): Fetch blobs on-demand
- Single branch: Only default branch

Usage:
    source = ClonedRepoSource("https://github.com/user/repo")
    try:
        meta = source.get_metadata()
        for file in source.walk():
            content = source.get_content(file.path)
    finally:
        source.cleanup()

Or with context manager:
    with ClonedRepoSource("https://github.com/user/repo") as source:
        ...
"""

import os
import shutil
import subprocess
import tempfile
import time
from pathlib import Path
from typing import Iterator, Optional

from ..source import RepoSource, FileDescriptor, RepoMetadata


# Default patterns to ignore during walk
DEFAULT_IGNORE_PATTERNS = {
    ".git",
    ".svn",
    ".hg",
    "__pycache__",
    "node_modules",
    ".venv",
    "venv",
    ".tox",
    ".pytest_cache",
    ".mypy_cache",
    "target",  # Rust
    "build",   # Generic
    "dist",    # Python/JS
}

# Binary file extensions to skip
BINARY_EXTENSIONS = {
    "png", "jpg", "jpeg", "gif", "bmp", "ico", "webp",
    "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx",
    "zip", "tar", "gz", "bz2", "xz", "7z", "rar",
    "exe", "dll", "so", "dylib", "a", "o", "pyc", "pyo",
    "woff", "woff2", "ttf", "eot", "otf",
    "mp3", "mp4", "avi", "mov", "wav", "flac",
    "sqlite", "db", "sqlite3",
}

# Max file size to read (5MB)
MAX_FILE_SIZE = 5 * 1024 * 1024


class ClonedRepoSource(RepoSource):
    """RepoSource implementation using shallow git clone.

    Clones the repository to a temporary directory and provides
    file iteration and content access.
    """

    def __init__(
        self,
        repo_url: str,
        branch: Optional[str] = None,
        shallow: bool = True,
        ignore_patterns: Optional[set] = None,
    ):
        """Initialize and clone the repository.

        Args:
            repo_url: Git clone URL (https:// or git@)
            branch: Specific branch to clone (default: repo's default)
            shallow: Use shallow clone (--depth 1)
            ignore_patterns: Directory patterns to skip during walk
        """
        self.repo_url = repo_url
        self.branch = branch
        self.shallow = shallow
        self.ignore_patterns = ignore_patterns or DEFAULT_IGNORE_PATTERNS

        # Extract repo name from URL
        self.repo_name = self._extract_repo_name(repo_url)

        # State
        self._temp_dir: Optional[str] = None
        self._repo_path: Optional[Path] = None
        self._metadata: Optional[RepoMetadata] = None
        self._clone_time_ms: Optional[int] = None
        self._files_cache: Optional[list] = None

        # Clone immediately
        self._clone()

    def _extract_repo_name(self, url: str) -> str:
        """Extract repository name from URL."""
        # Handle both HTTPS and SSH URLs
        name = url.rstrip("/").split("/")[-1]
        if name.endswith(".git"):
            name = name[:-4]
        return name

    def _clone(self) -> None:
        """Execute git clone to temporary directory."""
        self._temp_dir = tempfile.mkdtemp(prefix="pm_coach_")
        self._repo_path = Path(self._temp_dir) / self.repo_name

        # Build clone command
        cmd = ["git", "clone"]

        if self.shallow:
            cmd.extend(["--depth", "1"])
            # Blob filter for even faster clones (Git 2.19+)
            cmd.extend(["--filter=blob:none"])

        if self.branch:
            cmd.extend(["--branch", self.branch])

        cmd.extend(["--single-branch", self.repo_url, str(self._repo_path)])

        # Execute clone
        start = time.time()
        try:
            subprocess.run(
                cmd,
                capture_output=True,
                check=True,
                timeout=300,  # 5 minute timeout
            )
        except subprocess.CalledProcessError as e:
            self.cleanup()
            raise RuntimeError(f"Git clone failed: {e.stderr.decode()}")
        except subprocess.TimeoutExpired:
            self.cleanup()
            raise RuntimeError(f"Git clone timed out for {self.repo_url}")

        self._clone_time_ms = int((time.time() - start) * 1000)

    def get_metadata(self) -> RepoMetadata:
        """Return repository metadata."""
        if self._metadata is not None:
            return self._metadata

        # Walk to compute stats
        files = list(self.walk())
        total_size = sum(f.size for f in files)

        # Count languages by extension
        languages: dict = {}
        for f in files:
            ext = f.extension
            if ext:
                languages[f".{ext}"] = languages.get(f".{ext}", 0) + 1

        # Determine primary language
        primary = max(languages.items(), key=lambda x: x[1])[0] if languages else None

        self._metadata = RepoMetadata(
            name=self.repo_name,
            url=self.repo_url,
            default_branch=self.branch or "main",
            size_bytes=total_size,
            file_count=len(files),
            primary_language=primary,
            languages=languages,
            clone_time_ms=self._clone_time_ms,
        )

        return self._metadata

    def walk(self) -> Iterator[FileDescriptor]:
        """Yield file descriptors for all files in repo."""
        if self._repo_path is None:
            raise RuntimeError("Repository not cloned")

        # Cache files for repeated walks
        if self._files_cache is not None:
            yield from self._files_cache
            return

        files = []
        for root, dirs, filenames in os.walk(self._repo_path):
            # Filter ignored directories in-place
            dirs[:] = [d for d in dirs if d not in self.ignore_patterns]

            for filename in sorted(filenames):
                filepath = Path(root) / filename
                rel_path = str(filepath.relative_to(self._repo_path))

                # Skip hidden files in root
                if filename.startswith(".") and "/" not in rel_path:
                    continue

                try:
                    stat = filepath.stat()
                except OSError:
                    continue

                # Determine if binary
                ext = filepath.suffix.lstrip(".").lower()
                is_binary = ext in BINARY_EXTENSIONS

                # Check if symlink
                mode = "symlink" if filepath.is_symlink() else "file"

                fd = FileDescriptor(
                    path=rel_path,
                    size=stat.st_size,
                    mtime=stat.st_mtime,
                    is_binary=is_binary,
                    mode=mode,
                )
                files.append(fd)

        # Sort by path for deterministic order
        files.sort(key=lambda f: f.path)
        self._files_cache = files
        yield from files

    def get_content(self, path: str) -> str:
        """Read file content with encoding fallback."""
        if self._repo_path is None:
            raise RuntimeError("Repository not cloned")

        filepath = self._repo_path / path

        if not filepath.exists():
            raise FileNotFoundError(f"File not found: {path}")

        if filepath.stat().st_size > MAX_FILE_SIZE:
            raise ValueError(f"File too large: {path} ({filepath.stat().st_size} bytes)")

        # Try UTF-8 first, fallback to latin-1
        try:
            return filepath.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            return filepath.read_text(encoding="latin-1")

    def cleanup(self) -> None:
        """Remove temporary directory."""
        if self._temp_dir and os.path.exists(self._temp_dir):
            shutil.rmtree(self._temp_dir, ignore_errors=True)
            self._temp_dir = None
            self._repo_path = None

    def walk_with_content(self) -> Iterator[tuple]:
        """Yield (FileDescriptor, content) tuples.

        For ClonedRepoSource, this is NOT true streaming - we still
        do two I/O operations per file (stat + read). But it provides
        the unified interface that true streaming sources will implement.
        """
        for fd in self.walk():
            if fd.is_binary or fd.size > MAX_FILE_SIZE:
                continue
            try:
                content = self.get_content(fd.path)
                yield (fd, content)
            except (UnicodeDecodeError, ValueError):
                continue

    def supports_streaming(self) -> bool:
        """ClonedRepoSource does NOT support true streaming.

        We must read the file system twice: once for metadata (stat),
        once for content. True streaming sources (GitHub API, tarballs)
        can provide both in a single operation.
        """
        return False

    def __enter__(self) -> "ClonedRepoSource":
        """Context manager entry."""
        return self

    def __exit__(self, exc_type, exc_val, exc_tb) -> None:
        """Context manager exit - cleanup."""
        self.cleanup()

    def __repr__(self) -> str:
        status = "cloned" if self._repo_path else "cleaned up"
        return f"ClonedRepoSource({self.repo_name!r}, {status})"
