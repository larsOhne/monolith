"""drift() — fourth pipeline stage.

For every Evidence record in a project, compare the source content at
*git_sha_at_pin* against the current HEAD of the sources repo. Classify
each piece of evidence as:

- valid   — the verbatim passage is present unchanged at the same position
- drifted — the passage still exists in the file but at a different position
            (the source was edited around it)
- broken  — the passage no longer appears in the file at all

The DB is updated in place and a DriftReport is returned.
"""

from __future__ import annotations

from monolith.models import DriftEntry, DriftReport, EvidenceStatus, Project
from monolith.store.db import DB
from monolith.store import fs


def check_drift(project: Project, db: DB) -> DriftReport:
    """Scan all evidence in *project* and update statuses. Returns a DriftReport."""
    sources = db.list_sources(project.id)
    report = DriftReport()

    for source in sources:
        evidence_list = db.list_evidence_for_source(source.id)
        if not evidence_list:
            continue

        head_sha = fs.current_head_sha(project.slug)
        current_text = fs.read_current_text(project.slug, source.path)

        for ev in evidence_list:
            if ev.git_sha_at_pin == head_sha:
                # Source has not changed since pin — still valid
                continue

            new_status = _classify(ev.verbatim_text, current_text)

            if new_status != ev.status:
                db.update_evidence_status(ev.id, new_status)

            if new_status != EvidenceStatus.valid:
                diff = fs.get_diff_between(project.slug, ev.git_sha_at_pin, head_sha, source.path)
                report.entries.append(
                    DriftEntry(
                        evidence_id=ev.id,
                        source_id=source.id,
                        old_git_sha=ev.git_sha_at_pin,
                        new_git_sha=head_sha,
                        status=new_status,
                        diff_snippet=_truncate_diff(diff),
                    )
                )

    return report


# ------------------------------------------------------------------
# Internal helpers
# ------------------------------------------------------------------

def _classify(verbatim: str, current_text: str) -> EvidenceStatus:
    """Determine evidence status by searching *verbatim* in *current_text*."""
    if verbatim in current_text:
        return EvidenceStatus.valid
    # Try a relaxed match: strip leading/trailing whitespace from each line
    stripped_verbatim = _normalize(verbatim)
    stripped_current = _normalize(current_text)
    if stripped_verbatim in stripped_current:
        return EvidenceStatus.drifted
    return EvidenceStatus.broken


def _normalize(text: str) -> str:
    return "\n".join(line.strip() for line in text.splitlines())


def _truncate_diff(diff: str, max_lines: int = 30) -> str:
    lines = diff.splitlines()
    if len(lines) <= max_lines:
        return diff
    return "\n".join(lines[:max_lines]) + f"\n… ({len(lines) - max_lines} more lines)"
