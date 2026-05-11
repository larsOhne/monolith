"""ingest() — vault-first first pipeline stage.

Accepts a local file path or a URL and copies the source into the vault:
1. Fetches/copies the content.
2. Writes it into <vault>/sources/ (hash-deduplicated).
3. Returns an IngestResult (DB cache updated by the caller/router).
"""

from __future__ import annotations

import tempfile
import uuid
from pathlib import Path

import httpx

from monolith.models import IngestResult, Source
from monolith.security import validate_path, validate_url
from monolith.store import fs


def ingest(
    vault_root: Path,
    *,
    file_path: str | Path | None = None,
    url: str | None = None,
) -> IngestResult:
    """Ingest a source by local *file_path* or *url* into the vault.

    Exactly one of *file_path* or *url* must be provided.
    Returns an IngestResult with the new or existing Source.
    """
    if (file_path is None) == (url is None):
        raise ValueError("Provide exactly one of file_path or url")

    if url is not None:
        clean_url = validate_url(url)
        return _ingest_url(vault_root, clean_url)
    else:
        clean_path = validate_path(file_path, must_exist=True)
        return _ingest_file(vault_root, clean_path, original_url=None)


def _ingest_file(
    vault_root: Path,
    path: Path,
    *,
    original_url: str | None,
) -> IngestResult:
    rel_path, sha256 = fs.copy_source_into_vault(vault_root, path)
    source = Source(
        id=str(uuid.uuid4()),
        path=rel_path,
        url=original_url,
        sha256=sha256,
    )
    return IngestResult(source=source, copied_bytes=path.stat().st_size)


def _ingest_url(vault_root: Path, url: str) -> IngestResult:
    with httpx.Client(follow_redirects=True, timeout=30) as client:
        response = client.get(url)
        response.raise_for_status()

    content = response.text
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

    with tempfile.NamedTemporaryFile(
        mode="w",
        suffix=Path(filename).suffix,
        delete=False,
        encoding="utf-8",
    ) as tmp:
        tmp.write(content)
        tmp_path = Path(tmp.name)

    named_path = tmp_path.parent / filename
    try:
        tmp_path.rename(named_path)
        result = _ingest_file(vault_root, named_path, original_url=url)
    finally:
        for p in [tmp_path, named_path]:
            try:
                p.unlink()
            except FileNotFoundError:
                pass

    return result
