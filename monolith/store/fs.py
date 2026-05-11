"""Filesystem operations for vault-first Monolith.

The vault root is a regular folder (an Obsidian vault or any directory).
Monolith stores its derived data in <vault>/.monolith/:

    .monolith/refs/         reference notes (auto-created by mark())
    .monolith/graph/        graph build artifacts
    .monolith/monolith.db   DuckDB cache
    .monolith/config.yaml   project metadata

Sources live in <vault>/sources/.
"""

from __future__ import annotations

import hashlib
import os
import shutil
from pathlib import Path

_DEFAULT_VAULT = Path(os.environ.get("MONOLITH_VAULT", Path.home() / "monolith-vault"))


def get_vault_root() -> Path:
    return Path(os.environ.get("MONOLITH_VAULT", str(_DEFAULT_VAULT)))


def monolith_dir(vault_root: Path) -> Path:
    return vault_root / ".monolith"


def refs_dir(vault_root: Path) -> Path:
    return monolith_dir(vault_root) / "refs"


def sources_dir(vault_root: Path) -> Path:
    return vault_root / "sources"


def graph_dir(vault_root: Path) -> Path:
    return monolith_dir(vault_root) / "graph"


def db_path(vault_root: Path) -> Path:
    return monolith_dir(vault_root) / "monolith.db"


def config_path(vault_root: Path) -> Path:
    return monolith_dir(vault_root) / "config.yaml"


def ensure_vault_structure(vault_root: Path) -> None:
    """Create all necessary directories inside the vault."""
    for d in [
        monolith_dir(vault_root),
        refs_dir(vault_root),
        sources_dir(vault_root),
        graph_dir(vault_root),
    ]:
        d.mkdir(parents=True, exist_ok=True)


def copy_source_into_vault(vault_root: Path, source_path: Path) -> tuple[str, str]:
    """Copy source_path into <vault>/sources/ and return (vault_rel_path, sha256)."""
    ensure_vault_structure(vault_root)
    dest_dir = sources_dir(vault_root)
    dest = dest_dir / source_path.name
    # Avoid name collisions while preserving deduplication by content hash
    counter = 1
    stem = source_path.stem
    suffix = source_path.suffix
    while dest.exists() and sha256_file(dest) != sha256_file(source_path):
        dest = dest_dir / f"{stem}_{counter}{suffix}"
        counter += 1
    if not dest.exists():
        shutil.copy2(source_path, dest)
    rel = str(dest.relative_to(vault_root))
    return rel, sha256_file(dest)


def read_source_text(vault_root: Path, vault_rel_path: str) -> str:
    """Read current text of a source file."""
    return (vault_root / vault_rel_path).read_text(encoding="utf-8", errors="replace")


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def sha256_str(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


# ---------------------------------------------------------------------------
# Legacy helpers kept for compatibility with existing git-based drift code.
# These will be removed once the vault-first drift is fully wired.
# ---------------------------------------------------------------------------

def _legacy_monolith_root() -> Path:
    return Path(os.environ.get("MONOLITH_ROOT", Path.home() / ".monolith"))


def project_sources_dir(slug: str) -> Path:
    return _legacy_monolith_root() / "projects" / slug / "sources"


def project_graph_dir(slug: str) -> Path:
    return _legacy_monolith_root() / "projects" / slug / "graph"


def _sha256_file(path: Path) -> str:
    return sha256_file(path)


def _sha256_str(text: str) -> str:
    return sha256_str(text)
