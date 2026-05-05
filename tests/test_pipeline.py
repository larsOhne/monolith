"""Tests for the core pipeline: ingest → pin → assert_ → drift."""

from __future__ import annotations

import os
import tempfile
from pathlib import Path

import pytest

# Point store to a temp location so tests don't touch ~/.monolith
@pytest.fixture(autouse=True)
def isolate_monolith(tmp_path, monkeypatch):
    monkeypatch.setenv("MONOLITH_ROOT", str(tmp_path / "monolith"))
    monkeypatch.setenv("MONOLITH_DB", str(tmp_path / "monolith" / "test.ddb"))
    # Reload fs module so it picks up the new env var
    import importlib
    import monolith.store.fs as fs_mod
    importlib.reload(fs_mod)
    yield


def _make_db(tmp_path):
    from monolith.store.db import DB
    return DB(tmp_path / "monolith" / "test.ddb")


def _make_project(db, slug="test-proj"):
    from monolith.mcp.tools import create_project
    return create_project(db, "Test Project", slug, "desc")


# ------------------------------------------------------------------
# DB layer
# ------------------------------------------------------------------

def test_project_crud(tmp_path):
    from monolith.store.db import DB
    from monolith.models import Project
    db = DB(tmp_path / "test.ddb")
    p = Project(id="p1", name="My Proj", slug="my-proj")
    db.upsert_project(p)
    assert db.get_project("p1").name == "My Proj"
    assert db.get_project_by_slug("my-proj").id == "p1"
    assert len(db.list_projects()) == 1


# ------------------------------------------------------------------
# Security helpers
# ------------------------------------------------------------------

def test_validate_url_ok():
    from monolith.security import validate_url
    assert validate_url("https://example.com/paper.pdf") == "https://example.com/paper.pdf"


def test_validate_url_bad_scheme():
    from monolith.security import validate_url
    with pytest.raises(ValueError, match="not allowed"):
        validate_url("file:///etc/passwd")


def test_validate_slug_bad():
    from monolith.security import validate_slug
    with pytest.raises(ValueError):
        validate_slug("Has Spaces!")


# ------------------------------------------------------------------
# Ingest
# ------------------------------------------------------------------

def test_ingest_file(tmp_path):
    from monolith.store.db import DB
    from monolith.models import Project
    from monolith.core.ingest import ingest
    from monolith.store import fs
    import importlib
    importlib.reload(fs)

    db = DB(tmp_path / "m" / "test.ddb")
    p = Project(id="p1", name="T", slug="t")
    db.upsert_project(p)

    source_file = tmp_path / "paper.txt"
    source_file.write_text("The sky is blue and the grass is green.")

    result = ingest(p, db, file_path=source_file)
    assert result.copied_bytes > 0
    assert result.source.project_id == "p1"

    # Duplicate ingest returns same source
    result2 = ingest(p, db, file_path=source_file)
    assert result2.source.id == result.source.id
    assert result2.copied_bytes == 0


# ------------------------------------------------------------------
# Pin
# ------------------------------------------------------------------

def test_pin_passage(tmp_path):
    from monolith.store.db import DB
    from monolith.models import Project
    from monolith.core.ingest import ingest
    from monolith.core.pin import pin
    from monolith.store import fs
    import importlib
    importlib.reload(fs)

    db = DB(tmp_path / "m" / "test.ddb")
    p = Project(id="p1", name="T", slug="t")
    db.upsert_project(p)

    f = tmp_path / "doc.txt"
    f.write_text("Attention is all you need.")
    result = ingest(p, db, file_path=f)

    pin_result = pin(p, result.source, db, verbatim_text="Attention is all you need.")
    ev = pin_result.evidence
    assert ev.char_start == 0
    assert ev.verbatim_text == "Attention is all you need."


def test_pin_missing_text(tmp_path):
    from monolith.store.db import DB
    from monolith.models import Project
    from monolith.core.ingest import ingest
    from monolith.core.pin import pin
    from monolith.store import fs
    import importlib
    importlib.reload(fs)

    db = DB(tmp_path / "m" / "test.ddb")
    p = Project(id="p1", name="T", slug="t")
    db.upsert_project(p)

    f = tmp_path / "doc.txt"
    f.write_text("Hello world.")
    result = ingest(p, db, file_path=f)

    with pytest.raises(ValueError, match="not found verbatim"):
        pin(p, result.source, db, verbatim_text="This text is not in the file at all.")


# ------------------------------------------------------------------
# Assert
# ------------------------------------------------------------------

def test_assert_requires_evidence(tmp_path):
    from monolith.store.db import DB
    from monolith.models import Project
    from monolith.core.assert_ import assert_
    from monolith.store import fs
    import importlib
    importlib.reload(fs)

    db = DB(tmp_path / "m" / "test.ddb")
    p = Project(id="p1", name="T", slug="t")
    db.upsert_project(p)

    with pytest.raises(ValueError, match="at least one"):
        assert_(p, db, content="Some claim", evidence_ids=[])


def test_assert_creates_statement(tmp_path):
    from monolith.store.db import DB
    from monolith.models import Project
    from monolith.core.ingest import ingest
    from monolith.core.pin import pin
    from monolith.core.assert_ import assert_
    from monolith.store import fs
    import importlib
    importlib.reload(fs)

    db = DB(tmp_path / "m" / "test.ddb")
    p = Project(id="p1", name="T", slug="t")
    db.upsert_project(p)

    f = tmp_path / "doc.txt"
    f.write_text("The transformer architecture uses self-attention.")
    src = ingest(p, db, file_path=f).source
    ev = pin(p, src, db, verbatim_text="uses self-attention").evidence

    result = assert_(p, db, content="Self-attention is central to transformers.", evidence_ids=[ev.id])
    stmt = result.statement
    assert stmt.content == "Self-attention is central to transformers."
    assert ev.id in stmt.evidence_ids

    # Provenance chain
    chain = db.get_provenance_chain(stmt.id)
    assert chain["statement"]["id"] == stmt.id
    assert len(chain["evidence"]) == 1
