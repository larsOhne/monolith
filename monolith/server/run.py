"""Entry point for the Monolith FastAPI server.

Supports ``--port 0`` for OS-assigned port. When ``MONOLITH_READY_FD`` is set
the actual bound port is written to that fd as ``port=<N>\\n`` so the Rust
desktop app can connect without polling.

On startup the server scans the vault and rebuilds the DuckDB cache.
"""

from __future__ import annotations

import argparse
import logging
import os
import signal
import socket
from contextlib import asynccontextmanager
from pathlib import Path

import uvicorn

logger = logging.getLogger(__name__)


def _write_ready(fd: int, port: int) -> None:
    try:
        os.write(fd, f"port={port}\n".encode())
        os.close(fd)
    except OSError as exc:
        logger.warning("Could not write to MONOLITH_READY_FD: %s", exc)


def _find_free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def _build_lifespan(ready_fd: int, port: int):
    @asynccontextmanager
    async def lifespan(application):  # noqa: ARG001
        # Rebuild the vault cache synchronously before accepting requests
        from monolith.server.deps import get_vault_root, get_db
        from monolith.store import fs

        vault_root = get_vault_root()
        fs.ensure_vault_structure(vault_root)
        db = get_db()
        logger.info("Rebuilding cache from vault: %s", vault_root)
        db.rebuild_from_vault(vault_root)
        logger.info("Vault cache ready.")

        if ready_fd:
            _write_ready(ready_fd, port)

        yield  # server runs here

    return lifespan


def main() -> None:
    parser = argparse.ArgumentParser(description="Monolith API server")
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=8000)
    parser.add_argument("--reload", action="store_true")
    args = parser.parse_args()

    signal.signal(signal.SIGTERM, lambda *_: (_ for _ in ()).throw(SystemExit(0)))

    port = args.port
    if port == 0:
        port = _find_free_port()

    ready_fd = int(os.environ.get("MONOLITH_READY_FD", "0"))

    from monolith.server.main import app
    app.router.lifespan_context = _build_lifespan(ready_fd, port)

    uvicorn.run(
        "monolith.server.main:app",
        host=args.host,
        port=port,
        reload=args.reload,
        log_level="info",
    )


if __name__ == "__main__":
    main()
