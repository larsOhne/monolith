"""MCP tool implementations for Monolith.

Each function here is a pure Python callable that the MCP server wires up.
They share a DB instance opened once at server startup.
"""

from __future__ import annotations

import uuid
from pathlib import Path

from monolith.core import assert_, drift, graph, ingest, pin
from monolith.security import validate_slug
from monolith.store.db import DB


# ------------------------------------------------------------------
# Tool: list_projects
# ------------------------------------------------------------------

def list_projects(db: DB) -> list[dict]:
    """Return all projects with a brief health summary."""
    projects = db.list_projects()
    result = []
    for p in projects:
        sources = db.list_sources(p.id)
        all_ev = []
        for s in sources:
            all_ev.extend(db.list_evidence_for_source(s.id))
        broken = sum(1 for e in all_ev if e.status.value in ("drifted", "broken"))
        result.append({
            "id": p.id,
            "name": p.name,
            "slug": p.slug,
            "description": p.description,
            "source_count": len(sources),
            "evidence_count": len(all_ev),
            "evidence_with_issues": broken,
        })
    return result


# ------------------------------------------------------------------
# Tool: add_source
# ------------------------------------------------------------------

def add_source(
    db: DB,
    project_slug: str,
    *,
    file_path: str | None = None,
    url: str | None = None,
) -> dict:
    """Ingest a source into *project_slug*. Returns the Source record as a dict."""
    slug = validate_slug(project_slug)
    project = db.get_project_by_slug(slug)
    if project is None:
        raise ValueError(f"Project {slug!r} not found. Create it first.")

    result = ingest.ingest(
        project,
        db,
        file_path=Path(file_path) if file_path else None,
        url=url,
    )
    s = result.source
    return {
        "id": s.id,
        "project_id": s.project_id,
        "path": s.path,
        "url": s.url,
        "sha256": s.sha256,
        "git_sha": s.git_sha,
        "ingested_at": s.ingested_at.isoformat(),
        "copied_bytes": result.copied_bytes,
    }


# ------------------------------------------------------------------
# Tool: pin_evidence
# ------------------------------------------------------------------

def pin_evidence(
    db: DB,
    source_id: str,
    verbatim_text: str,
) -> dict:
    """Pin a verbatim passage in *source_id*. Returns the Evidence record as a dict."""
    source = db.get_source(source_id)
    if source is None:
        raise ValueError(f"Source {source_id!r} not found")
    project = db.get_project(source.project_id)
    if project is None:
        raise ValueError(f"Project for source {source_id!r} not found")

    result = pin.pin(project, source, db, verbatim_text=verbatim_text)
    ev = result.evidence
    return {
        "id": ev.id,
        "source_id": ev.source_id,
        "verbatim_text": ev.verbatim_text,
        "char_start": ev.char_start,
        "char_end": ev.char_end,
        "git_sha_at_pin": ev.git_sha_at_pin,
        "status": ev.status.value,
    }


# ------------------------------------------------------------------
# Tool: create_statement
# ------------------------------------------------------------------

def create_statement(
    db: DB,
    project_slug: str,
    content: str,
    evidence_ids: list[str],
) -> dict:
    """Assert a claim backed by *evidence_ids*. Returns the Statement as a dict."""
    slug = validate_slug(project_slug)
    project = db.get_project_by_slug(slug)
    if project is None:
        raise ValueError(f"Project {slug!r} not found")

    result = assert_.assert_(project, db, content=content, evidence_ids=evidence_ids)
    stmt = result.statement
    return {
        "id": stmt.id,
        "project_id": stmt.project_id,
        "content": stmt.content,
        "evidence_ids": stmt.evidence_ids,
        "created_at": stmt.created_at.isoformat(),
    }


# ------------------------------------------------------------------
# Tool: check_drift
# ------------------------------------------------------------------

def check_drift(db: DB, project_slug: str) -> dict:
    """Run drift detection for *project_slug*. Returns a DriftReport as a dict."""
    slug = validate_slug(project_slug)
    project = db.get_project_by_slug(slug)
    if project is None:
        raise ValueError(f"Project {slug!r} not found")

    report = drift.check_drift(project, db)
    return {
        "has_issues": report.has_issues,
        "entries": [
            {
                "evidence_id": e.evidence_id,
                "source_id": e.source_id,
                "old_git_sha": e.old_git_sha,
                "new_git_sha": e.new_git_sha,
                "status": e.status.value,
                "diff_snippet": e.diff_snippet,
            }
            for e in report.entries
        ],
    }


# ------------------------------------------------------------------
# Tool: search_evidence
# ------------------------------------------------------------------

def search_evidence(db: DB, query: str, limit: int = 20) -> list[dict]:
    """Full-text search over evidence verbatim text. Returns matching Evidence dicts."""
    results = db.search_evidence(query, limit=limit)
    return [
        {
            "id": ev.id,
            "source_id": ev.source_id,
            "verbatim_text": ev.verbatim_text,
            "char_start": ev.char_start,
            "char_end": ev.char_end,
            "git_sha_at_pin": ev.git_sha_at_pin,
            "status": ev.status.value,
        }
        for ev in results
    ]


# ------------------------------------------------------------------
# Tool: get_provenance_chain
# ------------------------------------------------------------------

def get_provenance_chain(db: DB, statement_id: str) -> dict:
    """Return the full statement → evidence → source provenance chain."""
    chain = db.get_provenance_chain(statement_id)
    if not chain:
        raise ValueError(f"Statement {statement_id!r} not found")
    return chain


# ------------------------------------------------------------------
# Tool: query_graph
# ------------------------------------------------------------------

def query_graph_tool(db: DB, project_slug: str, query: str, budget: int = 1500) -> str:
    """Query the graphify knowledge graph for *project_slug*."""
    slug = validate_slug(project_slug)
    project = db.get_project_by_slug(slug)
    if project is None:
        raise ValueError(f"Project {slug!r} not found")
    return graph.query_graph(project, query, budget=budget)


# ------------------------------------------------------------------
# Tool: create_project
# ------------------------------------------------------------------

def create_project(db: DB, name: str, slug: str, description: str = "") -> dict:
    """Create a new project. Returns the Project record as a dict."""
    from monolith.models import Project  # noqa: PLC0415
    from monolith.store import fs  # noqa: PLC0415

    clean_slug = validate_slug(slug)
    existing = db.get_project_by_slug(clean_slug)
    if existing is not None:
        raise ValueError(f"Project with slug {clean_slug!r} already exists")

    project = Project(id=str(uuid.uuid4()), name=name, slug=clean_slug, description=description)
    db.upsert_project(project)
    # Ensure the sources git repo is initialised
    fs.ensure_project_repo(clean_slug)
    return {"id": project.id, "name": project.name, "slug": project.slug, "description": project.description}
