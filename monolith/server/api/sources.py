"""Sources router."""

from __future__ import annotations

from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel

from monolith.mcp import tools as T
from monolith.server.deps import get_db
from monolith.store.db import DB
from monolith.store import fs

router = APIRouter()


class AddSourceBody(BaseModel):
    project_slug: str
    file_path: str | None = None
    url: str | None = None


@router.post("", status_code=201)
def add_source(body: AddSourceBody, db: DB = Depends(get_db)):
    try:
        return T.add_source(db, body.project_slug, file_path=body.file_path, url=body.url)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc
    except Exception as exc:
        raise HTTPException(status_code=500, detail=str(exc)) from exc


@router.get("/{source_id}")
def get_source(source_id: str, db: DB = Depends(get_db)):
    source = db.get_source(source_id)
    if source is None:
        raise HTTPException(status_code=404, detail="Source not found")
    return {
        "id": source.id,
        "project_id": source.project_id,
        "path": source.path,
        "url": source.url,
        "sha256": source.sha256,
        "git_sha": source.git_sha,
        "ingested_at": source.ingested_at.isoformat(),
    }


@router.get("/{source_id}/content")
def get_source_content(source_id: str, db: DB = Depends(get_db)):
    """Return the current on-disk text of a source file."""
    source = db.get_source(source_id)
    if source is None:
        raise HTTPException(status_code=404, detail="Source not found")
    project = db.get_project(source.project_id)
    if project is None:
        raise HTTPException(status_code=404, detail="Project not found")
    try:
        text = fs.read_current_text(project.slug, source.path)
    except Exception as exc:
        raise HTTPException(status_code=500, detail=str(exc)) from exc
    return {"content": text}


@router.get("")
def list_sources(project_slug: str, db: DB = Depends(get_db)):
    project = db.get_project_by_slug(project_slug)
    if project is None:
        raise HTTPException(status_code=404, detail="Project not found")
    sources = db.list_sources(project.id)
    return [
        {
            "id": s.id,
            "path": s.path,
            "url": s.url,
            "sha256": s.sha256,
            "git_sha": s.git_sha,
            "ingested_at": s.ingested_at.isoformat(),
        }
        for s in sources
    ]
