"""Filesystem and git operations for source management.

Each project gets its own git repository under:
    ~/.monolith/projects/<slug>/sources/

Sources are committed pristine. Every Evidence record stores the git SHA
that was HEAD when the passage was pinned, enabling exact drift detection.
"""

from __future__ import annotations

import hashlib
import os
import shutil
from pathlib import Path

import git

_MONOLITH_ROOT = Path(os.environ.get("MONOLITH_ROOT", Path.home() / ".monolith"))


def monolith_root() -> Path:
    return _MONOLITH_ROOT


def project_sources_dir(slug: str) -> Path:
    return _MONOLITH_ROOT / "projects" / slug / "sources"


def project_graph_dir(slug: str) -> Path:
    return _MONOLITH_ROOT / "projects" / slug / "graph"


def ensure_project_repo(slug: str) -> git.Repo:
    """Return the git.Repo for the project's sources dir, initialising if needed."""
    sources = project_sources_dir(slug)
    sources.mkdir(parents=True, exist_ok=True)
    if not (sources / ".git").exists():
        repo = git.Repo.init(str(sources))
        # Initial empty commit so there is always a HEAD
        repo.index.commit("init: initialise sources repository")
    else:
        repo = git.Repo(str(sources))
    return repo


def copy_file_into_repo(slug: str, source_path: Path) -> tuple[Path, str, str]:
    """Copy *source_path* into the project sources repo and commit it.

    Returns (relative_path_inside_repo, sha256_hex, git_sha).
    """
    repo = ensure_project_repo(slug)
    sources_root = project_sources_dir(slug)

    dest = sources_root / source_path.name
    # Avoid collisions by appending a counter if the name is already taken
    counter = 1
    stem = source_path.stem
    suffix = source_path.suffix
    while dest.exists() and _sha256_file(dest) != _sha256_file(source_path):
        dest = sources_root / f"{stem}_{counter}{suffix}"
        counter += 1

    shutil.copy2(source_path, dest)

    sha256 = _sha256_file(dest)
    rel_path = dest.relative_to(sources_root)

    repo.index.add([str(rel_path)])
    commit = repo.index.commit(f"add: {rel_path}")
    return rel_path, sha256, commit.hexsha


def write_text_into_repo(slug: str, filename: str, content: str) -> tuple[Path, str, str]:
    """Write *content* as *filename* into the project sources repo and commit it.

    Returns (relative_path_inside_repo, sha256_hex, git_sha).
    """
    repo = ensure_project_repo(slug)
    sources_root = project_sources_dir(slug)

    dest = sources_root / filename
    dest.write_text(content, encoding="utf-8")

    sha256 = _sha256_str(content)
    rel_path = dest.relative_to(sources_root)

    repo.index.add([str(rel_path)])
    commit = repo.index.commit(f"add: {rel_path}")
    return rel_path, sha256, commit.hexsha


def read_blob_at_sha(slug: str, rel_path: str | Path, git_sha: str) -> str:
    """Return the text content of *rel_path* at *git_sha* in the project sources repo."""
    repo = git.Repo(str(project_sources_dir(slug)))
    commit = repo.commit(git_sha)
    blob = commit.tree / str(rel_path)
    return blob.data_stream.read().decode("utf-8", errors="replace")


def read_current_text(slug: str, rel_path: str | Path) -> str:
    """Return the current on-disk text for *rel_path* inside the project sources dir."""
    full = project_sources_dir(slug) / rel_path
    return full.read_text(encoding="utf-8", errors="replace")


def current_head_sha(slug: str) -> str:
    repo = git.Repo(str(project_sources_dir(slug)))
    return repo.head.commit.hexsha


def get_diff_between(slug: str, old_sha: str, new_sha: str, rel_path: str) -> str:
    """Return a unified diff string for *rel_path* between *old_sha* and *new_sha*."""
    repo = git.Repo(str(project_sources_dir(slug)))
    try:
        diff = repo.git.diff(old_sha, new_sha, "--", rel_path)
    except git.GitCommandError:
        diff = ""
    return diff


# ------------------------------------------------------------------
# Internal helpers
# ------------------------------------------------------------------

def _sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def _sha256_str(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()
