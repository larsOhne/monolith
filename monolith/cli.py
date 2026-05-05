"""CLI entry point."""

from __future__ import annotations

import sys


def main() -> None:
    print("Monolith — personal knowledge system")
    print("  monolith-mcp   start the MCP stdio server")
    print("  uvicorn monolith.server.main:app --reload   start the API + UI server")
    sys.exit(0)
