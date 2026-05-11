"""mark() — create a reference note pinning a passage in a source.

Vault-first replacement for pin(). Instead of inserting into DuckDB,
writes a markdown note to <vault>/.monolith/refs/ and returns a Reference.
The DB cache is updated separately by the caller if needed.
"""

from __future__ import annotations

import hashlib
import re
from datetime import date
from pathlib import Path

from monolith.models import MarkResult, Reference, ReferenceStatus
from monolith.store import fs
from monolith.security import validate_evidence_text


def mark(
    vault_root: Path,
    source_vault_rel: str,
    *,
    verbatim_text: str,
    anchor: str | None = None,
) -> MarkResult:
    """Pin verbatim_text from source_vault_rel and write a reference note.

    *source_vault_rel* is a path relative to the vault root, e.g. "sources/foo.md".
    The reference note is written to .monolith/refs/<source-stem>.md.

    Returns a MarkResult with the new Reference.
    """
    text = validate_evidence_text(verbatim_text)

    source_abs = vault_root / source_vault_rel
    if not source_abs.exists():
        raise FileNotFoundError(f"Source not found in vault: {source_vault_rel!r}")

    current_text = source_abs.read_text(encoding="utf-8", errors="replace")
    if text not in current_text:
        raise ValueError(
            f"Passage not found verbatim in {source_vault_rel!r}. "
            "Check for whitespace or encoding differences."
        )

    source_hash = "sha256:" + fs.sha256_file(source_abs)
    span_hash = "sha256:" + fs.sha256_str(text)
    ref_short = hashlib.sha256(text.encode()).hexdigest()[:8]
    ref_id = f"ref-{ref_short}"

    if anchor is None:
        anchor = text.splitlines()[0][:80].strip()

    source_stem = Path(source_vault_rel).stem
    refs_path = vault_root / ".monolith" / "refs"
    refs_path.mkdir(parents=True, exist_ok=True)
    note_file = refs_path / f"{source_stem}.md"
    note_rel = str(note_file.relative_to(vault_root))

    if note_file.exists():
        # Append a new ref block to the existing per-source ref file
        existing = note_file.read_text(encoding="utf-8")
        note_file.write_text(
            existing.rstrip() + "\n\n---\n\n" + _ref_block(ref_id, text, anchor, span_hash),
            encoding="utf-8",
        )
    else:
        note_file.write_text(
            _full_note(source_vault_rel, source_hash, ref_id, text, anchor, span_hash),
            encoding="utf-8",
        )

    ref = Reference(
        id=ref_id,
        note_path=note_rel,
        source_path=source_vault_rel,
        source_hash=source_hash,
        verbatim_text=text,
        anchor=anchor,
        span_hash=span_hash,
        polygon=None,
        marked_at=date.today().isoformat(),
        status=ReferenceStatus.valid,
    )
    return MarkResult(reference=ref)


def _full_note(
    source_vault_rel: str,
    source_hash: str,
    ref_id: str,
    verbatim_text: str,
    anchor: str,
    span_hash: str,
) -> str:
    today = date.today().isoformat()
    frontmatter = (
        f"---\n"
        f"monolith_type: reference\n"
        f"source: {source_vault_rel}\n"
        f"source_hash: {source_hash}\n"
        f"marked: {today}\n"
        f"---\n\n"
    )
    return frontmatter + _ref_block(ref_id, verbatim_text, anchor, span_hash)


def _ref_block(ref_id: str, verbatim_text: str, anchor: str, span_hash: str) -> str:
    quoted = "\n".join(f"> {line}" for line in verbatim_text.splitlines())
    return (
        f"## ^{ref_id}\n\n"
        f"{quoted}\n\n"
        f'anchor: "{anchor}"\n'
        f"span_hash: {span_hash}\n"
    )
