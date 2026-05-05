"""pin() — second pipeline stage.

Pins a verbatim passage inside a source as Evidence.
The passage is looked up character-by-character in the current content of the
source file; the git SHA that is HEAD at pin time is stored so drift() can
later detect if that passage moved or disappeared.
"""

from __future__ import annotations

import uuid

from monolith.models import Evidence, EvidenceStatus, PinResult, Project, Source
from monolith.security import validate_evidence_text
from monolith.store.db import DB
from monolith.store import fs


def pin(
    project: Project,
    source: Source,
    db: DB,
    *,
    verbatim_text: str,
) -> PinResult:
    """Pin *verbatim_text* inside *source*.

    The text must appear verbatim (exact substring match) in the current
    on-disk content of the source file.

    Returns a PinResult containing the new Evidence record.
    """
    text = validate_evidence_text(verbatim_text)

    current_text = fs.read_current_text(project.slug, source.path)
    idx = current_text.find(text)
    if idx == -1:
        raise ValueError(
            f"The passage was not found verbatim in source {source.path!r}. "
            "Check for whitespace or encoding differences."
        )

    head_sha = fs.current_head_sha(project.slug)

    ev = Evidence(
        id=str(uuid.uuid4()),
        source_id=source.id,
        verbatim_text=text,
        char_start=idx,
        char_end=idx + len(text),
        git_sha_at_pin=head_sha,
        status=EvidenceStatus.valid,
    )
    db.insert_evidence(ev)
    return PinResult(evidence=ev)
