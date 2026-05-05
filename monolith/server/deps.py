"""Shared FastAPI dependencies."""

from __future__ import annotations

from functools import lru_cache

from monolith.store.db import DB


@lru_cache(maxsize=1)
def get_db() -> DB:
    return DB()
