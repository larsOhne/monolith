"""References router (/api/refs).

References are vault notes in .monolith/refs/. This router provides:
  POST /api/refs          — mark a verbatim passage and write a reference note
  GET  /api/refs          — list all references (from DB cache)
  GET  /api/refs/{ref_id} — get a single reference
  GET  /api/refs/search   — full-text search over verbatim text / anchor
"""

from __future__ import annotations

from pathlib import Path

from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel

from monolith.core import mark as mark_mod
from monolith.store.db import DB
from monolith.server.deps import get_db, get_vault_root

router = APIRouter()


class MarkBody(BaseModel):
    source_path: str    # vault-relative path to the source file
    verbatim_text: str
    anchor: str | None = None


def _ref_dict(ref) -> dict:
    return {
        "id": ref.id,
        "note_path": ref.note_path,
        "source_path": ref.source_path,
        "source_hash": ref.source_hash,
        "verbatim_text": ref.verbatim_text,
        "anchor": ref.anchor,
        "span_hash": ref.span_hash,
        "polygon": ref.polygon,
        "marked_at": ref.marked_at,
        "status": ref.status.value,
    }


@router.post("", status_code=201)
def create_reference(
    body: MarkBody,
    vault_root: Path = Depends(get_vault_root),
    db: DB = Depends(get_db),
):
    try:
        result = mark_mod.mark(
            vault_root,
            body.source_path,
            verbatim_text=body.verbatim_text,
            anchor=body.anchor,
        )
    except (FileNotFoundError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc

    ref = result.reference
    db.upsert_reference(ref)
    return _ref_dict(ref)


@router.get("/search")
def search_references(query: str, limit: int = 20, db: DB = Depends(get_db)):
    refs = db.search_references(query, limit=limit)
    return [_ref_dict(r) for r in refs]


@router.get("/{ref_id}")
def get_reference(ref_id: str, db: DB = Depends(get_db)):
    ref = db.get_reference(ref_id)
    if ref is None:
        raise HTTPException(status_code=404, detail="Reference not found")
    return _ref_dict(ref)


@router.get("")
def list_references(db: DB = Depends(get_db)):
    return [_ref_dict(r) for r in db.list_references()]
