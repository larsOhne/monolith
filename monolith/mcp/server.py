"""MCP stdio server entry point for Monolith.

Exposes all Monolith operations as MCP tools so any MCP-compatible AI
assistant (Claude Code, Cursor, Copilot, etc.) can drive the system.

Run:
    monolith-mcp
or:
    python -m monolith.mcp.server
"""

from __future__ import annotations

import sys
from pathlib import Path

from mcp.server import Server
from mcp.server.stdio import stdio_server
from mcp.types import TextContent, Tool
import mcp.types as types

from monolith.mcp import tools as T
from monolith.store.db import DB

_TOOLS: list[Tool] = [
    Tool(
        name="create_project",
        description="Create a new Monolith project.",
        inputSchema={
            "type": "object",
            "properties": {
                "name": {"type": "string", "description": "Human-readable project name"},
                "slug": {"type": "string", "description": "URL-safe identifier (lowercase, hyphens)"},
                "description": {"type": "string", "description": "Optional project description"},
            },
            "required": ["name", "slug"],
        },
    ),
    Tool(
        name="list_projects",
        description="List all projects with evidence health summary.",
        inputSchema={"type": "object", "properties": {}},
    ),
    Tool(
        name="add_source",
        description="Ingest a source document into a project by file path or URL.",
        inputSchema={
            "type": "object",
            "properties": {
                "project_slug": {"type": "string"},
                "file_path": {"type": "string", "description": "Absolute path to a local file"},
                "url": {"type": "string", "description": "URL to fetch and ingest"},
            },
            "required": ["project_slug"],
        },
    ),
    Tool(
        name="pin_evidence",
        description="Pin a verbatim passage from a source as evidence.",
        inputSchema={
            "type": "object",
            "properties": {
                "source_id": {"type": "string"},
                "verbatim_text": {"type": "string", "description": "Exact text to pin"},
            },
            "required": ["source_id", "verbatim_text"],
        },
    ),
    Tool(
        name="create_statement",
        description="Assert a claim backed by one or more evidence IDs.",
        inputSchema={
            "type": "object",
            "properties": {
                "project_slug": {"type": "string"},
                "content": {"type": "string", "description": "The claim being asserted"},
                "evidence_ids": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "IDs of evidence that supports this claim",
                },
            },
            "required": ["project_slug", "content", "evidence_ids"],
        },
    ),
    Tool(
        name="check_drift",
        description="Detect which evidence passages have changed or disappeared since being pinned.",
        inputSchema={
            "type": "object",
            "properties": {
                "project_slug": {"type": "string"},
            },
            "required": ["project_slug"],
        },
    ),
    Tool(
        name="search_evidence",
        description="Full-text search over all pinned evidence passages.",
        inputSchema={
            "type": "object",
            "properties": {
                "query": {"type": "string"},
                "limit": {"type": "integer", "default": 20},
            },
            "required": ["query"],
        },
    ),
    Tool(
        name="get_provenance_chain",
        description="Return the full statement → evidence → source provenance chain.",
        inputSchema={
            "type": "object",
            "properties": {
                "statement_id": {"type": "string"},
            },
            "required": ["statement_id"],
        },
    ),
    Tool(
        name="query_graph",
        description="Query the graphify knowledge graph for a project.",
        inputSchema={
            "type": "object",
            "properties": {
                "project_slug": {"type": "string"},
                "query": {"type": "string"},
                "budget": {"type": "integer", "default": 1500},
            },
            "required": ["project_slug", "query"],
        },
    ),
]


def _dispatch(db: DB, name: str, args: dict) -> str:
    import json  # noqa: PLC0415

    match name:
        case "create_project":
            result = T.create_project(db, **args)
        case "list_projects":
            result = T.list_projects(db)
        case "add_source":
            result = T.add_source(db, **args)
        case "pin_evidence":
            result = T.pin_evidence(db, **args)
        case "create_statement":
            result = T.create_statement(db, **args)
        case "check_drift":
            result = T.check_drift(db, **args)
        case "search_evidence":
            result = T.search_evidence(db, **args)
        case "get_provenance_chain":
            result = T.get_provenance_chain(db, **args)
        case "query_graph":
            result = T.query_graph_tool(db, **args)
        case _:
            raise ValueError(f"Unknown tool: {name!r}")

    if isinstance(result, str):
        return result
    return json.dumps(result, indent=2, default=str)


async def _run() -> None:
    db = DB()
    server = Server("monolith")

    @server.list_tools()
    async def list_tools() -> list[Tool]:
        return _TOOLS

    @server.call_tool()
    async def call_tool(name: str, arguments: dict) -> list[types.ContentBlock]:
        try:
            output = _dispatch(db, name, arguments)
        except Exception as exc:  # noqa: BLE001
            output = f"Error: {exc}"
        return [TextContent(type="text", text=output)]

    async with stdio_server() as (read_stream, write_stream):
        await server.run(read_stream, write_stream, server.create_initialization_options())


def main() -> None:
    import asyncio  # noqa: PLC0415
    asyncio.run(_run())


if __name__ == "__main__":
    main()
