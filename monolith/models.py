"""Core domain models for Monolith."""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import UTC, datetime
from enum import Enum


class EvidenceStatus(str, Enum):
    valid = "valid"
    drifted = "drifted"
    broken = "broken"


@dataclass
class Project:
    id: str
    name: str
    slug: str
    description: str = ""


@dataclass
class Source:
    id: str
    project_id: str
    path: str  # path inside the project sources git repo
    url: str | None  # original URL if ingested from the web
    sha256: str  # content hash at ingest time
    git_sha: str  # commit SHA in the project sources repo
    ingested_at: datetime = field(default_factory=lambda: datetime.now(UTC))


@dataclass
class Evidence:
    id: str
    source_id: str
    verbatim_text: str
    char_start: int
    char_end: int
    git_sha_at_pin: str  # commit SHA in sources repo when this was pinned
    status: EvidenceStatus = EvidenceStatus.valid


@dataclass
class Statement:
    id: str
    project_id: str
    content: str
    evidence_ids: list[str] = field(default_factory=list)
    created_at: datetime = field(default_factory=lambda: datetime.now(UTC))


# ---------------------------------------------------------------------------
# Lightweight result types passed between pipeline stages
# ---------------------------------------------------------------------------

@dataclass
class IngestResult:
    source: Source
    copied_bytes: int


@dataclass
class PinResult:
    evidence: Evidence


@dataclass
class AssertResult:
    statement: Statement


@dataclass
class DriftEntry:
    evidence_id: str
    source_id: str
    old_git_sha: str
    new_git_sha: str
    status: EvidenceStatus  # drifted or broken
    diff_snippet: str  # unified diff excerpt around the pinned passage


@dataclass
class DriftReport:
    entries: list[DriftEntry] = field(default_factory=list)

    @property
    def has_issues(self) -> bool:
        return bool(self.entries)
