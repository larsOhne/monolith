"""Drift router."""

from __future__ import annotations

from fastapi import APIRouter, Depends, HTTPException

from monolith.mcp import tools as T
from monolith.server.deps import get_db
from monolith.store.db import DB

router = APIRouter()


@router.get("/{project_slug}")
def check_drift(project_slug: str, db: DB = Depends(get_db)):
    """Run drift detection for all evidence in *project_slug*."""
    try:
        return T.check_drift(db, project_slug)
    except ValueError as exc:
        raise HTTPException(status_code=404, detail=str(exc)) from exc
