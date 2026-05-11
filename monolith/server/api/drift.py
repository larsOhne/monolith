"""Drift router."""

from __future__ import annotations

from pathlib import Path

from fastapi import APIRouter, Depends

from monolith.core import drift as drift_mod
from monolith.server.deps import get_vault_root

router = APIRouter()


@router.get("")
def check_drift(vault_root: Path = Depends(get_vault_root)):
    """Run drift detection for all references in the vault."""
    report = drift_mod.check_drift(vault_root)
    return {
        "has_issues": report.has_issues,
        "entries": [
            {
                "reference_id": e.reference_id,
                "source_path": e.source_path,
                "status": e.status.value,
                "diff_snippet": e.diff_snippet,
            }
            for e in report.entries
        ],
    }
