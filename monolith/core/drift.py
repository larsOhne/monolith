"""drift() — vault-first drift detection.

Reads Reference notes from .monolith/refs/, checks each against its source,
classifies as valid / drifted / broken.

- valid   — verbatim text still present in source unchanged
- drifted — anchor phrase found but exact verbatim text differs (source edited around it)
- broken  — anchor not found at all
"""

from __future__ import annotations

from pathlib import Path

from monolith.models import DriftEntry, DriftReport, Reference, ReferenceStatus
from monolith.core.vault import scan_references
from monolith.store import fs


def check_drift(vault_root: Path) -> DriftReport:
    """Scan all reference notes in vault and classify for drift."""
    references = scan_references(vault_root)
    report = DriftReport()

    for ref in references:
        source_abs = vault_root / ref.source_path
        if not source_abs.exists():
            report.entries.append(
                DriftEntry(
                    reference_id=ref.id,
                    source_path=ref.source_path,
                    status=ReferenceStatus.broken,
                    diff_snippet="Source file no longer exists in vault.",
                )
            )
            continue

        current_hash = "sha256:" + fs.sha256_file(source_abs)
        if current_hash == ref.source_hash:
            # Source file unchanged — reference is still valid
            continue

        try:
            current_text = source_abs.read_text(encoding="utf-8", errors="replace")
        except OSError as exc:
            report.entries.append(
                DriftEntry(
                    reference_id=ref.id,
                    source_path=ref.source_path,
                    status=ReferenceStatus.broken,
                    diff_snippet=f"Could not read source: {exc}",
                )
            )
            continue

        new_status = _classify(ref, current_text)
        if new_status != ReferenceStatus.valid:
            snippet = _context_snippet(ref, current_text)
            report.entries.append(
                DriftEntry(
                    reference_id=ref.id,
                    source_path=ref.source_path,
                    status=new_status,
                    diff_snippet=snippet,
                )
            )

    return report


def _classify(ref: Reference, current_text: str) -> ReferenceStatus:
    if ref.verbatim_text and ref.verbatim_text in current_text:
        return ReferenceStatus.valid
    if ref.anchor and ref.anchor in current_text:
        return ReferenceStatus.drifted
    return ReferenceStatus.broken


def _context_snippet(ref: Reference, current_text: str) -> str:
    """Return a few lines of context around where the anchor was expected."""
    anchor = ref.anchor or (ref.verbatim_text or "")[:80]
    if not anchor:
        return "(no anchor; cannot locate passage)"

    idx = current_text.find(anchor)
    if idx == -1:
        stored_preview = (ref.verbatim_text or anchor)[:200]
        return f"Was:\n{stored_preview!r}\n\n(not found in current source)"

    # Context around where anchor IS found (to show what changed nearby)
    line_start = max(0, current_text.rfind("\n", 0, idx) + 1)
    end_search = idx + len(anchor) + 200
    line_end = current_text.find("\n", idx + len(anchor), end_search)
    if line_end == -1:
        line_end = min(len(current_text), end_search)

    context = current_text[line_start:line_end]
    stored = (ref.verbatim_text or "")[:200]
    return f"Was:\n{stored!r}\n\nCurrent context:\n{context!r}"
