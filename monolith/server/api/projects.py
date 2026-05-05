"""Projects router."""

from __future__ import annotations

from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel

from monolith.mcp import tools as T
from monolith.server.deps import get_db
from monolith.store.db import DB

router = APIRouter()


class CreateProjectBody(BaseModel):
    name: str
    slug: str
    description: str = ""


@router.get("")
def list_projects(db: DB = Depends(get_db)):
    return T.list_projects(db)


@router.post("", status_code=201)
def create_project(body: CreateProjectBody, db: DB = Depends(get_db)):
    try:
        return T.create_project(db, body.name, body.slug, body.description)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc


@router.get("/{slug}")
def get_project(slug: str, db: DB = Depends(get_db)):
    project = db.get_project_by_slug(slug)
    if project is None:
        raise HTTPException(status_code=404, detail="Project not found")
    return {"id": project.id, "name": project.name, "slug": project.slug, "description": project.description}
