"""Entry point for the Monolith FastAPI server.

Supports ``--port 0`` for OS-assigned port assignment.  When the env var
``MONOLITH_READY_FD`` is set the actual bound port is written to that file
descriptor as ``port=<N>\\n`` so the parent process (the desktop app) can
connect without polling.
"""

from __future__ import annotations

import argparse
import logging
import os
import signal
import socket

import uvicorn

logger = logging.getLogger(__name__)

_shutdown = False


def _handle_sigterm(signum: int, frame: object) -> None:  # noqa: ARG001
    global _shutdown  # noqa: PLW0603
    _shutdown = True
    raise SystemExit(0)


def _write_ready(fd: int, port: int) -> None:
    """Write port to the ready file descriptor and close it."""
    try:
        os.write(fd, f"port={port}\n".encode())
        os.close(fd)
    except OSError as exc:
        logger.warning("Could not write to MONOLITH_READY_FD: %s", exc)


def _find_free_port() -> int:
    """Bind to port 0 and return the OS-assigned port number."""
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def main() -> None:
    parser = argparse.ArgumentParser(description="Monolith API server")
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=8000)
    parser.add_argument("--reload", action="store_true")
    args = parser.parse_args()

    signal.signal(signal.SIGTERM, _handle_sigterm)

    port = args.port
    if port == 0:
        port = _find_free_port()

    ready_fd = int(os.environ.get("MONOLITH_READY_FD", "0"))

    # Write port *before* uvicorn starts so the parent can connect as soon as
    # the server is ready (uvicorn's startup hook fires after bind).
    # We use a uvicorn lifespan event to write it at the right moment.
    if ready_fd:
        import asyncio
        from contextlib import asynccontextmanager

        from fastapi import FastAPI

        @asynccontextmanager
        async def lifespan(application: FastAPI):  # noqa: ARG001
            _write_ready(ready_fd, port)
            yield

        # Patch the app's lifespan *only* when running as a worker
        from monolith.server.main import app
        app.router.lifespan_context = lifespan  # type: ignore[assignment]

    uvicorn.run(
        "monolith.server.main:app",
        host=args.host,
        port=port,
        reload=args.reload,
        log_level="info",
    )


if __name__ == "__main__":
    main()
