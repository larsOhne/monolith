"""Shared FastAPI dependencies."""

from __future__ import annotations

from functools import lru_cache
from pathlib import Path

from monolith.store import fs
from monolith.store.db import DB


@lru_cache(maxsize=1)
def get_vault_root() -> Path:
    """Return the vault root from MONOLITH_VAULT env var."""
    return fs.get_vault_root()


@lru_cache(maxsize=1)
def get_db() -> DB:
    vault_root = get_vault_root()
    return DB(path=fs.db_path(vault_root))
