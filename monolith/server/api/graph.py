"""Graph router."""

from __future__ import annotations

from fastapi import APIRouter, Depends, HTTPException

from monolith.core import graph as graph_core
from monolith.mcp import tools as T
from monolith.server.deps import get_db
from monolith.store.db import DB

router = APIRouter()


@router.post("/{project_slug}/build", status_code=202)
def build_graph(project_slug: str, mode: str = "default", db: DB = Depends(get_db)):
    """Trigger graphify synthesis for the project's sources. Returns graph.json path."""
    project = db.get_project_by_slug(project_slug)
    if project is None:
        raise HTTPException(status_code=404, detail="Project not found")
    try:
        graph_path = graph_core.synthesize(project, mode=mode)
    except RuntimeError as exc:
        raise HTTPException(status_code=500, detail=str(exc)) from exc
    return {"graph_json": str(graph_path)}


@router.get("/{project_slug}")
def get_graph(project_slug: str, db: DB = Depends(get_db)):
    """Return the graph.json content for a project."""
    project = db.get_project_by_slug(project_slug)
    if project is None:
        raise HTTPException(status_code=404, detail="Project not found")
    return graph_core.load_graph(project)


@router.get("/{project_slug}/query")
def query_graph(project_slug: str, q: str, budget: int = 1500, db: DB = Depends(get_db)):
    try:
        result = T.query_graph_tool(db, project_slug, q, budget=budget)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc
    return {"result": result}
