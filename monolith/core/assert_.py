"""assert_() — third pipeline stage.

Creates a Statement (a claim you believe) backed by one or more Evidence IDs.
A Statement with zero evidence is rejected.
"""

from __future__ import annotations

import uuid
from datetime import UTC, datetime

from monolith.models import AssertResult, Project, Statement
from monolith.security import validate_statement_content
from monolith.store.db import DB


def assert_(
    project: Project,
    db: DB,
    *,
    content: str,
    evidence_ids: list[str],
) -> AssertResult:
    """Create a Statement claiming *content*, backed by *evidence_ids*.

    Raises ValueError if *evidence_ids* is empty or any ID is unknown.
    """
    clean_content = validate_statement_content(content)

    if not evidence_ids:
        raise ValueError(
            "A statement must be backed by at least one piece of evidence. "
            "Pin a passage first, then assert a statement."
        )

    # Verify all evidence IDs exist
    missing = [eid for eid in evidence_ids if db.get_evidence(eid) is None]
    if missing:
        raise ValueError(f"Unknown evidence ID(s): {missing}")

    stmt = Statement(
        id=str(uuid.uuid4()),
        project_id=project.id,
        content=clean_content,
        evidence_ids=list(evidence_ids),
        created_at=datetime.now(UTC),
    )
    db.insert_statement(stmt)
    return AssertResult(statement=stmt)
