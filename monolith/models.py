"""Core domain models for Monolith.

Vault-first design: Sources, References, Relations, and Statements all live as
markdown files in the user's vault. DuckDB is a derived cache rebuilt on startup
by scanning the vault.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import UTC, datetime
from enum import Enum


class ReferenceStatus(str, Enum):
    valid = "valid"
    drifted = "drifted"
    broken = "broken"


# Backward-compat alias used by old code paths
EvidenceStatus = ReferenceStatus


@dataclass
class Project:
    id: str
    name: str
    slug: str
    description: str = ""


@dataclass
class Source:
    id: str
    path: str           # vault-relative path, e.g. "sources/ipcc-report.md"
    url: str | None     # original URL if ingested from the web
    sha256: str         # content hash at ingest time
    ingested_at: datetime = field(default_factory=lambda: datetime.now(UTC))


@dataclass
class Reference:
    """A pinned passage in a source document.

    Lives as a markdown note in <vault>/.monolith/refs/.
    """

    id: str                         # ^ref-<shortHash> anchor, e.g. "ref-a3f2b9c1"
    note_path: str                  # vault-relative path to the reference note
    source_path: str                # vault-relative path to source
    source_hash: str                # "sha256:<hex>" of source at mark time
    verbatim_text: str | None       # copied verbatim text (markdown sources)
    anchor: str | None              # short search phrase for drift detection
    span_hash: str | None           # "sha256:<hex>" of verbatim_text
    polygon: list | None            # [[x,y],...] for image references
    marked_at: str                  # ISO date string
    status: ReferenceStatus = ReferenceStatus.valid


@dataclass
class Relation:
    """A typed link between two vault notes. Lives as a markdown note."""

    id: str
    note_path: str      # vault-relative path to the relation note
    kind: str           # supports | disputes | extends | connects | …
    source: str         # wikilink target (note path or title)
    target: str         # wikilink target (note path or title)
    metadata: dict = field(default_factory=dict)


@dataclass
class Statement:
    """A claim note in the vault. Lives as any vault markdown file."""

    id: str
    note_path: str                              # vault-relative path
    content: str                                # markdown body
    reference_ids: list[str] = field(default_factory=list)  # linked ^ref- anchors
    created_at: datetime = field(default_factory=lambda: datetime.now(UTC))


# ---------------------------------------------------------------------------
# Lightweight result types
# ---------------------------------------------------------------------------

@dataclass
class IngestResult:
    source: Source
    copied_bytes: int


@dataclass
class MarkResult:
    reference: Reference


@dataclass
class DriftEntry:
    reference_id: str
    source_path: str
    status: ReferenceStatus     # drifted or broken
    diff_snippet: str           # context around the change


@dataclass
class DriftReport:
    entries: list[DriftEntry] = field(default_factory=list)

    @property
    def has_issues(self) -> bool:
        return bool(self.entries)
