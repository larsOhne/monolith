"""Security helpers — all external input is validated here before use.

Mirrors graphify/security.py in spirit: every boundary has an explicit check
that raises ValueError rather than silently proceeding.
"""

from __future__ import annotations

import re
from pathlib import Path
from urllib.parse import urlparse

_ALLOWED_SCHEMES = {"http", "https"}
_MAX_LABEL_LEN = 256
_MAX_FETCH_BYTES = 50 * 1024 * 1024  # 50 MB
_CONTROL_CHARS_RE = re.compile(r"[\x00-\x1f\x7f]")


def validate_url(url: str) -> str:
    """Validate that *url* is http/https. Returns the cleaned URL or raises ValueError."""
    url = url.strip()
    parsed = urlparse(url)
    if parsed.scheme not in _ALLOWED_SCHEMES:
        raise ValueError(f"URL scheme {parsed.scheme!r} is not allowed; use http or https")
    if not parsed.netloc:
        raise ValueError(f"URL has no host: {url!r}")
    return url


def validate_path(path: str | Path, *, must_exist: bool = False) -> Path:
    """Resolve *path* to an absolute Path and optionally check existence."""
    resolved = Path(path).expanduser().resolve()
    if must_exist and not resolved.exists():
        raise ValueError(f"Path does not exist: {resolved}")
    return resolved


def validate_slug(slug: str) -> str:
    """Slugs must be non-empty, lowercase alphanumeric + hyphens."""
    slug = slug.strip().lower()
    if not re.fullmatch(r"[a-z0-9][a-z0-9\-]*", slug):
        raise ValueError(
            f"Invalid slug {slug!r}: must be lowercase alphanumeric with optional hyphens"
        )
    return slug


def sanitize_label(label: str) -> str:
    """Strip control chars, truncate to _MAX_LABEL_LEN, HTML-escape."""
    label = _CONTROL_CHARS_RE.sub("", label)
    label = label[:_MAX_LABEL_LEN]
    return (
        label.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace('"', "&quot;")
    )


def validate_evidence_text(text: str) -> str:
    """Ensure verbatim evidence text is non-empty and not absurdly long."""
    text = text.strip()
    if not text:
        raise ValueError("Evidence text must not be empty")
    if len(text) > 100_000:
        raise ValueError("Evidence text exceeds 100 000 character limit")
    return text


def validate_statement_content(content: str) -> str:
    """Ensure statement content is non-empty."""
    content = content.strip()
    if not content:
        raise ValueError("Statement content must not be empty")
    return content
