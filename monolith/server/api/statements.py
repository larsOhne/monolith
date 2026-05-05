"""Statements router."""

from __future__ import annotations

from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel

from monolith.mcp import tools as T
from monolith.server.deps import get_db
from monolith.store.db import DB

router = APIRouter()


class CreateStatementBody(BaseModel):
    project_slug: str
    content: str
    evidence_ids: list[str]


@router.post("", status_code=201)
def create_statement(body: CreateStatementBody, db: DB = Depends(get_db)):
    try:
        return T.create_statement(db, body.project_slug, body.content, body.evidence_ids)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc


@router.get("/{statement_id}")
def get_statement(statement_id: str, db: DB = Depends(get_db)):
    stmt = db.get_statement(statement_id)
    if stmt is None:
        raise HTTPException(status_code=404, detail="Statement not found")
    return {
        "id": stmt.id,
        "project_id": stmt.project_id,
        "content": stmt.content,
        "evidence_ids": stmt.evidence_ids,
        "created_at": stmt.created_at.isoformat(),
    }


@router.get("/{statement_id}/provenance")
def get_provenance(statement_id: str, db: DB = Depends(get_db)):
    try:
        return T.get_provenance_chain(db, statement_id)
    except ValueError as exc:
        raise HTTPException(status_code=404, detail=str(exc)) from exc


@router.get("")
def list_statements(project_slug: str, db: DB = Depends(get_db)):
    project = db.get_project_by_slug(project_slug)
    if project is None:
        raise HTTPException(status_code=404, detail="Project not found")
    stmts = db.list_statements(project.id)
    return [
        {
            "id": s.id,
            "content": s.content,
            "evidence_ids": s.evidence_ids,
            "created_at": s.created_at.isoformat(),
        }
        for s in stmts
    ]
