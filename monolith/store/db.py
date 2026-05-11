"""DuckDB cache — vault-first schema, query helpers, and rebuild logic.

The DB is a derived cache populated by scanning the vault. Discard it at any
time; it will be rebuilt from <vault>/.monolith/refs/ and the vault markdown
files on next startup.
"""

from __future__ import annotations

import json
from datetime import datetime
from pathlib import Path

import duckdb

from monolith.models import (
    Reference,
    ReferenceStatus,
    Relation,
    Source,
)


def _schema() -> str:
    return """
    CREATE TABLE IF NOT EXISTS sources (
        id           TEXT PRIMARY KEY,
        path         TEXT NOT NULL UNIQUE,
        url          TEXT,
        sha256       TEXT NOT NULL,
        ingested_at  TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS refs (
        id            TEXT PRIMARY KEY,
        note_path     TEXT NOT NULL,
        source_path   TEXT NOT NULL,
        source_hash   TEXT NOT NULL,
        verbatim_text TEXT,
        anchor        TEXT,
        span_hash     TEXT,
        polygon       TEXT,
        marked_at     TEXT NOT NULL,
        status        TEXT NOT NULL DEFAULT 'valid'
    );

    CREATE TABLE IF NOT EXISTS relations (
        id        TEXT PRIMARY KEY,
        note_path TEXT NOT NULL,
        kind      TEXT NOT NULL,
        source    TEXT NOT NULL,
        target    TEXT NOT NULL,
        metadata  TEXT NOT NULL DEFAULT '{}'
    );
    """


class DB:
    """Thin wrapper around a DuckDB connection."""

    def __init__(self, path: Path | None = None) -> None:
        db_path = path or Path.home() / ".monolith" / "monolith.db"
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
    # Sources
    # ------------------------------------------------------------------

    def upsert_source(self, source: Source) -> None:
        self._conn.execute(
            """
            INSERT INTO sources (id, path, url, sha256, ingested_at)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT (id) DO UPDATE SET
                path = excluded.path,
                url = excluded.url,
                sha256 = excluded.sha256,
                ingested_at = excluded.ingested_at
            """,
            [source.id, source.path, source.url, source.sha256,
             source.ingested_at.isoformat()],
        )

    def get_source(self, source_id: str) -> Source | None:
        row = self._conn.execute(
            "SELECT id, path, url, sha256, ingested_at FROM sources WHERE id = ?",
            [source_id],
        ).fetchone()
        if row is None:
            return None
        return Source(row[0], row[1], row[2], row[3], datetime.fromisoformat(row[4]))

    def get_source_by_path(self, path: str) -> Source | None:
        row = self._conn.execute(
            "SELECT id, path, url, sha256, ingested_at FROM sources WHERE path = ?",
            [path],
        ).fetchone()
        if row is None:
            return None
        return Source(row[0], row[1], row[2], row[3], datetime.fromisoformat(row[4]))

    def get_source_by_sha256(self, sha256: str) -> Source | None:
        row = self._conn.execute(
            "SELECT id, path, url, sha256, ingested_at FROM sources WHERE sha256 = ?",
            [sha256],
        ).fetchone()
        if row is None:
            return None
        return Source(row[0], row[1], row[2], row[3], datetime.fromisoformat(row[4]))

    def list_sources(self) -> list[Source]:
        rows = self._conn.execute(
            "SELECT id, path, url, sha256, ingested_at FROM sources ORDER BY path"
        ).fetchall()
        return [Source(r[0], r[1], r[2], r[3], datetime.fromisoformat(r[4])) for r in rows]

    # ------------------------------------------------------------------
    # References
    # ------------------------------------------------------------------

    def upsert_reference(self, ref: Reference) -> None:
        self._conn.execute(
            """
            INSERT INTO refs (id, note_path, source_path, source_hash, verbatim_text,
                              anchor, span_hash, polygon, marked_at, status)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT (id) DO UPDATE SET
                note_path     = excluded.note_path,
                source_path   = excluded.source_path,
                source_hash   = excluded.source_hash,
                verbatim_text = excluded.verbatim_text,
                anchor        = excluded.anchor,
                span_hash     = excluded.span_hash,
                polygon       = excluded.polygon,
                marked_at     = excluded.marked_at,
                status        = excluded.status
            """,
            [
                ref.id,
                ref.note_path,
                ref.source_path,
                ref.source_hash,
                ref.verbatim_text,
                ref.anchor,
                ref.span_hash,
                json.dumps(ref.polygon) if ref.polygon else None,
                ref.marked_at,
                ref.status.value,
            ],
        )

    def get_reference(self, ref_id: str) -> Reference | None:
        row = self._conn.execute(
            "SELECT id, note_path, source_path, source_hash, verbatim_text, anchor, "
            "span_hash, polygon, marked_at, status FROM refs WHERE id = ?",
            [ref_id],
        ).fetchone()
        if row is None:
            return None
        return _row_to_reference(row)

    def list_references(self) -> list[Reference]:
        rows = self._conn.execute(
            "SELECT id, note_path, source_path, source_hash, verbatim_text, anchor, "
            "span_hash, polygon, marked_at, status FROM refs ORDER BY marked_at DESC"
        ).fetchall()
        return [_row_to_reference(r) for r in rows]

    def update_reference_status(self, ref_id: str, status: ReferenceStatus) -> None:
        self._conn.execute(
            "UPDATE refs SET status = ? WHERE id = ?",
            [status.value, ref_id],
        )

    def search_references(self, query: str, limit: int = 20) -> list[Reference]:
        rows = self._conn.execute(
            """
            SELECT id, note_path, source_path, source_hash, verbatim_text, anchor,
                   span_hash, polygon, marked_at, status
            FROM refs
            WHERE lower(verbatim_text) LIKE lower(?) OR lower(anchor) LIKE lower(?)
            LIMIT ?
            """,
            [f"%{query}%", f"%{query}%", limit],
        ).fetchall()
        return [_row_to_reference(r) for r in rows]

    # ------------------------------------------------------------------
    # Relations
    # ------------------------------------------------------------------

    def upsert_relation(self, rel: Relation) -> None:
        self._conn.execute(
            """
            INSERT INTO relations (id, note_path, kind, source, target, metadata)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT (id) DO UPDATE SET
                note_path = excluded.note_path,
                kind      = excluded.kind,
                source    = excluded.source,
                target    = excluded.target,
                metadata  = excluded.metadata
            """,
            [rel.id, rel.note_path, rel.kind, rel.source, rel.target,
             json.dumps(rel.metadata)],
        )

    def list_relations(self) -> list[Relation]:
        rows = self._conn.execute(
            "SELECT id, note_path, kind, source, target, metadata FROM relations ORDER BY kind"
        ).fetchall()
        return [
            Relation(r[0], r[1], r[2], r[3], r[4], json.loads(r[5]))
            for r in rows
        ]

    # ------------------------------------------------------------------
    # Rebuild from vault
    # ------------------------------------------------------------------

    def rebuild_from_vault(self, vault_root: Path) -> None:
        """Repopulate the cache by scanning the vault. Safe to call on startup."""
        from monolith.core.vault import scan_references, scan_relations

        # Truncate and repopulate reference and relation caches
        self._conn.execute("DELETE FROM refs")
        self._conn.execute("DELETE FROM relations")

        for ref in scan_references(vault_root):
            self.upsert_reference(ref)

        for rel in scan_relations(vault_root):
            self.upsert_relation(rel)


# ------------------------------------------------------------------
# Row helpers
# ------------------------------------------------------------------


def _row_to_reference(row: tuple) -> Reference:
    id_, note_path, source_path, source_hash, verbatim_text, anchor, span_hash, polygon, marked_at, status = row
    return Reference(
        id=id_,
        note_path=note_path,
        source_path=source_path,
        source_hash=source_hash,
        verbatim_text=verbatim_text,
        anchor=anchor,
        span_hash=span_hash,
        polygon=json.loads(polygon) if polygon else None,
        marked_at=marked_at or "",
        status=ReferenceStatus(status),
    )
