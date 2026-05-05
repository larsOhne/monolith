"""graph.py — fifth pipeline stage.

Delegates to graphify to build a knowledge graph from the project's sources,
then writes graph.json + graph.html into the project graph directory.

graphify's output format (NetworkX node-link JSON) is reused directly by the
MCP serve layer and the UI.
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

from monolith.models import Project
from monolith.store import fs


def synthesize(project: Project, *, mode: str = "default") -> Path:
    """Run graphify over the project's sources dir and return the graph.json path.

    *mode* is passed as --mode to graphify (default | deep).
    Raises RuntimeError if graphify exits with a non-zero status.
    """
    sources_dir = fs.project_sources_dir(project.slug)
    graph_dir = fs.project_graph_dir(project.slug)
    graph_dir.mkdir(parents=True, exist_ok=True)

    cmd = [
        sys.executable, "-m", "graphify",
        str(sources_dir),
        "--no-viz" if False else "",  # always build the HTML viz
        "--out", str(graph_dir),
    ]
    # Strip empty strings from cmd list
    cmd = [c for c in cmd if c]

    if mode == "deep":
        cmd += ["--mode", "deep"]

    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        raise RuntimeError(
            f"graphify failed (exit {result.returncode}):\n{result.stderr}"
        )

    graph_json = graph_dir / "graph.json"
    if not graph_json.exists():
        raise RuntimeError(f"graphify did not produce graph.json in {graph_dir}")

    return graph_json


def load_graph(project: Project) -> dict:
    """Load and return the graph.json for *project* as a plain dict.

    Returns an empty graph dict if the graph has not been built yet.
    """
    graph_json = fs.project_graph_dir(project.slug) / "graph.json"
    if not graph_json.exists():
        return {"nodes": [], "links": []}
    return json.loads(graph_json.read_text(encoding="utf-8"))


def query_graph(project: Project, query: str, *, budget: int = 1500) -> str:
    """Run graphify query on the project's graph and return the result string."""
    graph_json = fs.project_graph_dir(project.slug) / "graph.json"
    if not graph_json.exists():
        return "No graph found for this project. Run synthesize() first."

    cmd = [
        sys.executable, "-m", "graphify",
        "query", query,
        "--graph", str(graph_json),
        "--budget", str(budget),
    ]
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        return f"graphify query error: {result.stderr.strip()}"
    return result.stdout.strip()
