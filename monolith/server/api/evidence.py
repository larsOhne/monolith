"""Evidence router."""

from __future__ import annotations

from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel

from monolith.mcp import tools as T
from monolith.server.deps import get_db
from monolith.store.db import DB

router = APIRouter()


class PinEvidenceBody(BaseModel):
    source_id: str
    verbatim_text: str


class SearchBody(BaseModel):
    query: str
    limit: int = 20


@router.post("", status_code=201)
def pin_evidence(body: PinEvidenceBody, db: DB = Depends(get_db)):
    try:
        return T.pin_evidence(db, body.source_id, body.verbatim_text)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc


@router.get("/{evidence_id}")
def get_evidence(evidence_id: str, db: DB = Depends(get_db)):
    ev = db.get_evidence(evidence_id)
    if ev is None:
        raise HTTPException(status_code=404, detail="Evidence not found")
    return {
        "id": ev.id,
        "source_id": ev.source_id,
        "verbatim_text": ev.verbatim_text,
        "char_start": ev.char_start,
        "char_end": ev.char_end,
        "git_sha_at_pin": ev.git_sha_at_pin,
        "status": ev.status.value,
    }


@router.get("/search")
def search_evidence(query: str, limit: int = 20, db: DB = Depends(get_db)):
    return T.search_evidence(db, query, limit=limit)
