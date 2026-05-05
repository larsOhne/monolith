"""FastAPI application — REST API + static UI serving."""

from __future__ import annotations

from pathlib import Path

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from fastapi.staticfiles import StaticFiles

from monolith.server.api import projects, sources, evidence, statements, graph, drift

app = FastAPI(title="Monolith", version="0.1.0")

app.add_middleware(
    CORSMiddleware,
    allow_origins=["http://localhost:5173"],  # Vite dev server
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

app.include_router(projects.router, prefix="/api/projects", tags=["projects"])
app.include_router(sources.router, prefix="/api/sources", tags=["sources"])
app.include_router(evidence.router, prefix="/api/evidence", tags=["evidence"])
app.include_router(statements.router, prefix="/api/statements", tags=["statements"])
app.include_router(graph.router, prefix="/api/graph", tags=["graph"])
app.include_router(drift.router, prefix="/api/drift", tags=["drift"])

# Serve built UI from ui/dist if it exists
_ui_dist = Path(__file__).parent.parent.parent / "ui" / "dist"
if _ui_dist.exists():
    app.mount("/", StaticFiles(directory=str(_ui_dist), html=True), name="ui")
