"""ingest() — first pipeline stage.

Accepts a local file path or a URL and adds the source to a project:
1. Fetches/copies the content.
2. Writes it into the project's sources git repo (pristine copy).
3. Records the Source in DuckDB.

Returns an IngestResult.
"""

from __future__ import annotations

import tempfile
import uuid
from pathlib import Path

import httpx

from monolith.models import IngestResult, Project, Source
from monolith.security import validate_path, validate_url
from monolith.store.db import DB
from monolith.store import fs


def ingest(
    project: Project,
    db: DB,
    *,
    file_path: str | Path | None = None,
    url: str | None = None,
) -> IngestResult:
    """Ingest a source by local *file_path* or *url* into *project*.

    Exactly one of *file_path* or *url* must be provided.
    """
    if (file_path is None) == (url is None):
        raise ValueError("Provide exactly one of file_path or url")

    if url is not None:
        clean_url = validate_url(url)
        return _ingest_url(project, db, clean_url)
    else:
        clean_path = validate_path(file_path, must_exist=True)
        return _ingest_file(project, db, clean_path, original_url=None)


# ------------------------------------------------------------------
# Internal helpers
# ------------------------------------------------------------------

def _ingest_file(
    project: Project,
    db: DB,
    path: Path,
    *,
    original_url: str | None,
) -> IngestResult:
    # Check for duplicates by content hash
    sources_root = fs.project_sources_dir(project.slug)
    # We need the hash before copying to detect exact duplicates
    from monolith.store.fs import _sha256_file  # noqa: PLC0415
    sha256 = _sha256_file(path)
    existing = db.source_by_sha256(project.id, sha256)
    if existing is not None:
        return IngestResult(source=existing, copied_bytes=0)

    rel_path, sha256, git_sha = fs.copy_file_into_repo(project.slug, path)

    source = Source(
        id=str(uuid.uuid4()),
        project_id=project.id,
        path=str(rel_path),
        url=original_url,
        sha256=sha256,
        git_sha=git_sha,
    )
    db.insert_source(source)
    return IngestResult(source=source, copied_bytes=path.stat().st_size)


def _ingest_url(project: Project, db: DB, url: str) -> IngestResult:
    with httpx.Client(follow_redirects=True, timeout=30) as client:
        response = client.get(url)
        response.raise_for_status()

    content = response.text
    # Derive a filename from the URL
    url_path = Path(url.split("?")[0].rstrip("/"))
    filename = url_path.name or "source.txt"
    if not url_path.suffix:
        ct = response.headers.get("content-type", "")
        if "html" in ct:
            filename += ".html"
        elif "pdf" in ct:
            filename += ".pdf"
        else:
            filename += ".txt"

    sha256_hex = __import__("hashlib").sha256(content.encode()).hexdigest()
    existing = db.source_by_sha256(project.id, sha256_hex)
    if existing is not None:
        return IngestResult(source=existing, copied_bytes=0)

    with tempfile.NamedTemporaryFile(mode="w", suffix=Path(filename).suffix, delete=False, encoding="utf-8") as tmp:
        tmp.write(content)
        tmp_path = Path(tmp.name)

    try:
        tmp_path_named = tmp_path.rename(tmp_path.parent / filename)
        result = _ingest_file(project, db, tmp_path_named, original_url=url)
    finally:
        for p in [tmp_path, tmp_path.parent / filename]:
            try:
                p.unlink()
            except FileNotFoundError:
                pass

    return result
