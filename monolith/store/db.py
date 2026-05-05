"""DuckDB store — schema, migrations, query helpers."""

from __future__ import annotations

import os
from datetime import datetime
from pathlib import Path

import duckdb
from datetime import datetime

from monolith.models import (
    Evidence,
    EvidenceStatus,
    Project,
    Source,
    Statement,
)

_DEFAULT_DB = Path(os.environ.get("MONOLITH_DB", Path.home() / ".monolith" / "monolith.ddb"))


def _schema() -> str:
    return """
    CREATE TABLE IF NOT EXISTS projects (
        id          TEXT PRIMARY KEY,
        name        TEXT NOT NULL,
        slug        TEXT NOT NULL UNIQUE,
        description TEXT NOT NULL DEFAULT ''
    );

    CREATE TABLE IF NOT EXISTS sources (
        id           TEXT PRIMARY KEY,
        project_id   TEXT NOT NULL REFERENCES projects(id),
        path         TEXT NOT NULL,
        url          TEXT,
        sha256       TEXT NOT NULL,
        git_sha      TEXT NOT NULL,
        ingested_at  TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS evidence (
        id              TEXT PRIMARY KEY,
        source_id       TEXT NOT NULL REFERENCES sources(id),
        verbatim_text   TEXT NOT NULL,
        char_start      INTEGER NOT NULL,
        char_end        INTEGER NOT NULL,
        git_sha_at_pin  TEXT NOT NULL,
        status          TEXT NOT NULL DEFAULT 'valid'
    );

    CREATE TABLE IF NOT EXISTS statements (
        id         TEXT PRIMARY KEY,
        project_id TEXT NOT NULL REFERENCES projects(id),
        content    TEXT NOT NULL,
        created_at TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS statement_evidence (
        statement_id TEXT NOT NULL REFERENCES statements(id),
        evidence_id  TEXT NOT NULL REFERENCES evidence(id),
        PRIMARY KEY (statement_id, evidence_id)
    );
    """


class DB:
    """Thin wrapper around a DuckDB connection."""

    def __init__(self, path: Path | None = None) -> None:
        db_path = path or _DEFAULT_DB
        db_path.parent.mkdir(parents=True, exist_ok=True)
        self._conn = duckdb.connect(str(db_path))
        self._conn.execute(_schema())

    def close(self) -> None:
        self._conn.close()

    def __enter__(self) -> "DB":
        return self

    def __exit__(self, *_: object) -> None:
        self.close()

    # ------------------------------------------------------------------
    # Projects
    # ------------------------------------------------------------------

    def upsert_project(self, project: Project) -> None:
        self._conn.execute(
            """
            INSERT INTO projects (id, name, slug, description)
            VALUES (?, ?, ?, ?)
            ON CONFLICT (id) DO UPDATE SET
                name = excluded.name,
                slug = excluded.slug,
                description = excluded.description
            """,
            [project.id, project.name, project.slug, project.description],
        )

    def get_project(self, project_id: str) -> Project | None:
        row = self._conn.execute(
            "SELECT id, name, slug, description FROM projects WHERE id = ?",
            [project_id],
        ).fetchone()
        if row is None:
            return None
        return Project(*row)

    def get_project_by_slug(self, slug: str) -> Project | None:
        row = self._conn.execute(
            "SELECT id, name, slug, description FROM projects WHERE slug = ?",
            [slug],
        ).fetchone()
        if row is None:
            return None
        return Project(*row)

    def list_projects(self) -> list[Project]:
        rows = self._conn.execute(
            "SELECT id, name, slug, description FROM projects ORDER BY name"
        ).fetchall()
        return [Project(*r) for r in rows]

    # ------------------------------------------------------------------
    # Sources
    # ------------------------------------------------------------------

    def insert_source(self, source: Source) -> None:
        self._conn.execute(
            """
            INSERT INTO sources (id, project_id, path, url, sha256, git_sha, ingested_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            """,
            [
                source.id,
                source.project_id,
                source.path,
                source.url,
                source.sha256,
                source.git_sha,
                source.ingested_at,
            ],
        )

    def get_source(self, source_id: str) -> Source | None:
        row = self._conn.execute(
            "SELECT id, project_id, path, url, sha256, git_sha, ingested_at FROM sources WHERE id = ?",
            [source_id],
        ).fetchone()
        if row is None:
            return None
        id_, project_id, path, url, sha256, git_sha, ingested_at = row
        return Source(id_, project_id, path, url, sha256, git_sha, datetime.fromisoformat(ingested_at))

    def list_sources(self, project_id: str) -> list[Source]:
        rows = self._conn.execute(
            "SELECT id, project_id, path, url, sha256, git_sha, ingested_at FROM sources WHERE project_id = ? ORDER BY path",
            [project_id],
        ).fetchall()
        return [Source(r[0], r[1], r[2], r[3], r[4], r[5], datetime.fromisoformat(r[6])) for r in rows]

    def source_by_sha256(self, project_id: str, sha256: str) -> Source | None:
        row = self._conn.execute(
            "SELECT id, project_id, path, url, sha256, git_sha, ingested_at FROM sources WHERE project_id = ? AND sha256 = ?",
            [project_id, sha256],
        ).fetchone()
        if row is None:
            return None
        id_, project_id, path, url, sha256, git_sha, ingested_at = row
        return Source(id_, project_id, path, url, sha256, git_sha, datetime.fromisoformat(ingested_at))

    # ------------------------------------------------------------------
    # Evidence
    # ------------------------------------------------------------------

    def insert_evidence(self, ev: Evidence) -> None:
        self._conn.execute(
            """
            INSERT INTO evidence (id, source_id, verbatim_text, char_start, char_end, git_sha_at_pin, status)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            """,
            [ev.id, ev.source_id, ev.verbatim_text, ev.char_start, ev.char_end, ev.git_sha_at_pin, ev.status.value],
        )

    def get_evidence(self, evidence_id: str) -> Evidence | None:
        row = self._conn.execute(
            "SELECT id, source_id, verbatim_text, char_start, char_end, git_sha_at_pin, status FROM evidence WHERE id = ?",
            [evidence_id],
        ).fetchone()
        if row is None:
            return None
        id_, source_id, verbatim_text, char_start, char_end, git_sha_at_pin, status = row
        return Evidence(id_, source_id, verbatim_text, char_start, char_end, git_sha_at_pin, EvidenceStatus(status))

    def list_evidence_for_source(self, source_id: str) -> list[Evidence]:
        rows = self._conn.execute(
            "SELECT id, source_id, verbatim_text, char_start, char_end, git_sha_at_pin, status FROM evidence WHERE source_id = ?",
            [source_id],
        ).fetchall()
        return [
            Evidence(r[0], r[1], r[2], r[3], r[4], r[5], EvidenceStatus(r[6]))
            for r in rows
        ]

    def list_all_evidence(self) -> list[Evidence]:
        rows = self._conn.execute(
            "SELECT id, source_id, verbatim_text, char_start, char_end, git_sha_at_pin, status FROM evidence"
        ).fetchall()
        return [
            Evidence(r[0], r[1], r[2], r[3], r[4], r[5], EvidenceStatus(r[6]))
            for r in rows
        ]

    def update_evidence_status(self, evidence_id: str, status: EvidenceStatus) -> None:
        self._conn.execute(
            "UPDATE evidence SET status = ? WHERE id = ?",
            [status.value, evidence_id],
        )

    def search_evidence(self, query: str, limit: int = 20) -> list[Evidence]:
        rows = self._conn.execute(
            """
            SELECT id, source_id, verbatim_text, char_start, char_end, git_sha_at_pin, status
            FROM evidence
            WHERE lower(verbatim_text) LIKE lower(?)
            LIMIT ?
            """,
            [f"%{query}%", limit],
        ).fetchall()
        return [
            Evidence(r[0], r[1], r[2], r[3], r[4], r[5], EvidenceStatus(r[6]))
            for r in rows
        ]

    # ------------------------------------------------------------------
    # Statements
    # ------------------------------------------------------------------

    def insert_statement(self, stmt: Statement) -> None:
        self._conn.execute(
            "INSERT INTO statements (id, project_id, content, created_at) VALUES (?, ?, ?, ?)",
            [stmt.id, stmt.project_id, stmt.content, stmt.created_at.isoformat()],
        )
        for ev_id in stmt.evidence_ids:
            self._conn.execute(
                "INSERT INTO statement_evidence (statement_id, evidence_id) VALUES (?, ?)",
                [stmt.id, ev_id],
            )

    def get_statement(self, statement_id: str) -> Statement | None:
        row = self._conn.execute(
            "SELECT id, project_id, content, created_at FROM statements WHERE id = ?",
            [statement_id],
        ).fetchone()
        if row is None:
            return None
        ev_rows = self._conn.execute(
            "SELECT evidence_id FROM statement_evidence WHERE statement_id = ? ORDER BY rowid",
            [statement_id],
        ).fetchall()
        return Statement(row[0], row[1], row[2], [r[0] for r in ev_rows], datetime.fromisoformat(row[3]))

    def list_statements(self, project_id: str) -> list[Statement]:
        rows = self._conn.execute(
            "SELECT id, project_id, content, created_at FROM statements WHERE project_id = ? ORDER BY created_at DESC",
            [project_id],
        ).fetchall()
        result = []
        for row in rows:
            ev_rows = self._conn.execute(
                "SELECT evidence_id FROM statement_evidence WHERE statement_id = ? ORDER BY rowid",
                [row[0]],
            ).fetchall()
            result.append(Statement(row[0], row[1], row[2], [r[0] for r in ev_rows], datetime.fromisoformat(row[3])))
        return result

    # ------------------------------------------------------------------
    # Provenance
    # ------------------------------------------------------------------

    def get_provenance_chain(self, statement_id: str) -> dict:
        """Return the full statement → evidence → source chain as a plain dict."""
        stmt = self.get_statement(statement_id)
        if stmt is None:
            return {}
        chain: dict = {
            "statement": {"id": stmt.id, "content": stmt.content, "created_at": stmt.created_at.isoformat()},
            "evidence": [],
        }
        for ev_id in stmt.evidence_ids:
            ev = self.get_evidence(ev_id)
            if ev is None:
                continue
            src = self.get_source(ev.source_id)
            chain["evidence"].append({
                "id": ev.id,
                "verbatim_text": ev.verbatim_text,
                "char_start": ev.char_start,
                "char_end": ev.char_end,
                "git_sha_at_pin": ev.git_sha_at_pin,
                "status": ev.status.value,
                "source": {
                    "id": src.id if src else None,
                    "path": src.path if src else None,
                    "url": src.url if src else None,
                    "sha256": src.sha256 if src else None,
                    "git_sha": src.git_sha if src else None,
                },
            })
        return chain
