"""Vault scanner — derives References, Relations from vault markdown files.

Scans .monolith/refs/ for reference notes (monolith_type: reference).
Scans all .md files for relation notes (monolith_type: relation).
Also finds notes that wikilink to reference anchors (^ref-*).
"""

from __future__ import annotations

import json
import re
import uuid
from pathlib import Path

from monolith.models import Reference, ReferenceStatus, Relation

_FRONTMATTER_RE = re.compile(r"^---\n(.*?)\n---", re.DOTALL)
_REF_ANCHOR_IN_LINK_RE = re.compile(r"\[\[.*?#\^(ref-[a-f0-9]+)\]\]")
_HEADING_ANCHOR_RE = re.compile(r"##\s+\^(ref-[a-f0-9]+)")


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def scan_references(vault_root: Path) -> list[Reference]:
    """Scan .monolith/refs/ for reference notes."""
    refs_path = vault_root / ".monolith" / "refs"
    if not refs_path.exists():
        return []

    references: list[Reference] = []
    for note_file in sorted(refs_path.rglob("*.md")):
        try:
            text = note_file.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue

        fm = _parse_frontmatter(text)
        if fm.get("monolith_type") != "reference":
            continue

        note_rel = str(note_file.relative_to(vault_root))

        # A single ref file can contain multiple ^ref- blocks separated by ---
        for block in _split_ref_blocks(text):
            ref = _parse_ref_block(block, note_rel, fm)
            if ref is not None:
                references.append(ref)

    return references


def scan_relations(vault_root: Path) -> list[Relation]:
    """Scan all .md files for monolith_type: relation frontmatter."""
    relations: list[Relation] = []
    for note_file in sorted(vault_root.rglob("*.md")):
        if ".monolith" in note_file.parts:
            continue
        try:
            text = note_file.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue

        fm = _parse_frontmatter(text)
        if fm.get("monolith_type") != "relation":
            continue

        note_rel = str(note_file.relative_to(vault_root))
        meta = {
            k: v
            for k, v in fm.items()
            if k not in ("monolith_type", "kind", "source", "target")
        }
        relations.append(
            Relation(
                id=str(uuid.uuid4()),
                note_path=note_rel,
                kind=fm.get("kind", "connects"),
                source=fm.get("source", "").strip("[]"),
                target=fm.get("target", "").strip("[]"),
                metadata=meta,
            )
        )
    return relations


def scan_ref_links(vault_root: Path) -> dict[str, list[str]]:
    """Return {note_path: [ref_ids]} for all notes that wikilink to ^ref- anchors."""
    result: dict[str, list[str]] = {}
    for note_file in sorted(vault_root.rglob("*.md")):
        if ".monolith" in note_file.parts:
            continue
        try:
            text = note_file.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue

        ref_ids = _REF_ANCHOR_IN_LINK_RE.findall(text)
        if ref_ids:
            note_rel = str(note_file.relative_to(vault_root))
            result[note_rel] = ref_ids

    return result


# ---------------------------------------------------------------------------
# Internal helpers
# ---------------------------------------------------------------------------


def _parse_frontmatter(text: str) -> dict[str, str]:
    """Parse simple key: value YAML frontmatter. Returns {} if not found."""
    m = _FRONTMATTER_RE.match(text)
    if not m:
        return {}
    fm: dict[str, str] = {}
    for line in m.group(1).splitlines():
        if ":" in line:
            key, _, val = line.partition(":")
            fm[key.strip()] = val.strip()
    return fm


def _split_ref_blocks(text: str) -> list[str]:
    """Split a reference note into individual ^ref- blocks (separated by ---)."""
    # Remove frontmatter first
    body = _FRONTMATTER_RE.sub("", text, count=1).lstrip()
    # Split on horizontal rules that separate ref blocks
    blocks = re.split(r"\n---\n", body)
    return [b.strip() for b in blocks if b.strip()]


def _parse_ref_block(block: str, note_rel: str, fm: dict[str, str]) -> Reference | None:
    """Parse a single ^ref- block from a reference note."""
    anchor_m = _HEADING_ANCHOR_RE.search(block)
    if not anchor_m:
        return None

    ref_id = anchor_m.group(1)
    verbatim = _extract_blockquote(block)

    # Fields after the blockquote (key: value lines not in a heading/blockquote)
    inline = _parse_inline_fields(block)

    polygon = _parse_polygon(inline.get("polygon", ""))

    return Reference(
        id=ref_id,
        note_path=note_rel,
        source_path=fm.get("source", ""),
        source_hash=fm.get("source_hash", ""),
        verbatim_text=verbatim,
        anchor=inline.get("anchor", "").strip('"') or None,
        span_hash=inline.get("span_hash") or None,
        polygon=polygon,
        marked_at=fm.get("marked", ""),
        status=ReferenceStatus.valid,
    )


def _extract_blockquote(text: str) -> str | None:
    """Extract content of the first blockquote (> lines) in text."""
    lines = text.splitlines()
    quote_lines: list[str] = []
    in_quote = False
    for line in lines:
        if line.startswith("> "):
            in_quote = True
            quote_lines.append(line[2:])
        elif in_quote and line == ">":
            quote_lines.append("")
        elif in_quote:
            break
    return "\n".join(quote_lines) if quote_lines else None


def _parse_inline_fields(text: str) -> dict[str, str]:
    """Parse key: value pairs that are NOT inside blockquotes or headings."""
    result: dict[str, str] = {}
    for line in text.splitlines():
        if line.startswith(">") or line.startswith("#"):
            continue
        if ":" in line and not line.startswith(" "):
            key, _, val = line.partition(":")
            key = key.strip()
            if key and not key.startswith("["):
                result[key] = val.strip()
    return result


def _parse_polygon(val: str) -> list | None:
    """Parse [[x,y],[x,y],...] polygon string."""
    if not val or not val.startswith("[["):
        return None
    try:
        return json.loads(val)
    except (ValueError, TypeError):
        return None
