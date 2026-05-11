"""Sources router."""

from __future__ import annotations

from pathlib import Path

from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel

from monolith.core import ingest as ingest_mod
from monolith.store.db import DB
from monolith.server.deps import get_db, get_vault_root
from monolith.store import fs

router = APIRouter()


class AddSourceBody(BaseModel):
    file_path: str | None = None
    url: str | None = None


def _source_dict(source, copied_bytes: int = 0) -> dict:
    return {
        "id": source.id,
        "path": source.path,
        "url": source.url,
        "sha256": source.sha256,
        "ingested_at": source.ingested_at.isoformat(),
        "copied_bytes": copied_bytes,
    }


@router.post("", status_code=201)
def add_source(
    body: AddSourceBody,
    vault_root: Path = Depends(get_vault_root),
    db: DB = Depends(get_db),
):
    # Deduplication: check if a source with same hash already exists
    try:
        result = ingest_mod.ingest(
            vault_root,
            file_path=body.file_path,
            url=body.url,
        )
    except (ValueError, FileNotFoundError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc
    except Exception as exc:
        raise HTTPException(status_code=500, detail=str(exc)) from exc

    # Check for existing source with same hash before inserting
    existing = db.get_source_by_sha256(result.source.sha256)
    if existing is not None:
        return _source_dict(existing)

    db.upsert_source(result.source)
    return _source_dict(result.source, result.copied_bytes)


@router.get("/{source_id}/content")
def get_source_content(
    source_id: str,
    vault_root: Path = Depends(get_vault_root),
    db: DB = Depends(get_db),
):
    source = db.get_source(source_id)
    if source is None:
        raise HTTPException(status_code=404, detail="Source not found")
    try:
        text = fs.read_source_text(vault_root, source.path)
    except Exception as exc:
        raise HTTPException(status_code=500, detail=str(exc)) from exc
    return {"content": text}


@router.get("/{source_id}")
def get_source(source_id: str, db: DB = Depends(get_db)):
    source = db.get_source(source_id)
    if source is None:
        raise HTTPException(status_code=404, detail="Source not found")
    return _source_dict(source)


@router.get("")
def list_sources(db: DB = Depends(get_db)):
    return [_source_dict(s) for s in db.list_sources()]
