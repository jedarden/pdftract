#!/usr/bin/env python3
"""Check the documented MCP catalog against a real tools/list response.

The JSON catalog is deliberately allowed to describe registered compatibility
tools that are callable but not advertised. The wire-level comparison is
therefore made against the catalog's ``advertised`` subset.

Set PDFTRACT_MCP_BIN to a prebuilt binary (and optionally include arguments)
to avoid compiling. Otherwise the checker starts the workspace binary through
Cargo.
"""

from __future__ import annotations

import json
import os
import shlex
import subprocess
import sys
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
CATALOG = ROOT / "docs/integrations/mcp-tool-catalog.json"


def fail(message: str) -> None:
    print(f"MCP catalog drift: {message}", file=sys.stderr)
    raise SystemExit(1)


def read_frame(stream: Any) -> dict[str, Any]:
    headers: dict[str, str] = {}
    while True:
        line = stream.readline()
        if not line:
            fail("server closed stdout before a response")
        if line in (b"\r\n", b"\n"):
            break
        try:
            key, value = line.decode("ascii").rstrip("\r\n").split(":", 1)
        except ValueError:
            fail(f"invalid response header: {line!r}")
        headers[key.lower()] = value.strip()

    try:
        length = int(headers["content-length"])
    except (KeyError, ValueError):
        fail(f"response did not contain a valid Content-Length header: {headers}")

    body = stream.read(length)
    if len(body) != length:
        fail("server closed stdout in the middle of a response")
    try:
        value = json.loads(body)
    except json.JSONDecodeError as error:
        fail(f"tools/list response was not JSON: {error}")
    if not isinstance(value, dict):
        fail("JSON-RPC response was not an object")
    return value


def send_frame(stream: Any, request_id: int, method: str, params: dict[str, Any]) -> None:
    body = json.dumps(
        {"jsonrpc": "2.0", "id": request_id, "method": method, "params": params},
        separators=(",", ":"),
    ).encode("utf-8")
    stream.write(f"Content-Length: {len(body)}\r\n\r\n".encode("ascii"))
    stream.write(body)
    stream.flush()


def response_result(response: dict[str, Any], method: str) -> dict[str, Any]:
    if "error" in response:
        fail(f"{method} returned an error: {response['error']}")
    result = response.get("result")
    if not isinstance(result, dict):
        fail(f"{method} response did not contain an object result")
    return result


def command() -> list[str]:
    configured = os.environ.get("PDFTRACT_MCP_BIN")
    if configured:
        parts = shlex.split(configured)
        if "mcp" not in parts:
            parts.extend(["mcp", "--stdio"])
        return parts
    return [
        "cargo",
        "run",
        "--quiet",
        "-p",
        "pdftract-cli",
        "--features",
        "mcp",
        "--",
        "mcp",
        "--stdio",
    ]


def main() -> int:
    try:
        catalog = json.loads(CATALOG.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        fail(f"cannot read {CATALOG}: {error}")

    entries = catalog.get("tools") if isinstance(catalog, dict) else None
    if not isinstance(entries, list) or len(entries) != 10:
        fail("authoritative catalog must contain exactly ten tools")

    names = [entry.get("name") for entry in entries if isinstance(entry, dict)]
    if len(names) != 10 or len(set(names)) != 10 or any(not name for name in names):
        fail("authoritative catalog must contain ten unique non-empty names")

    expected = sorted(
        entry["name"] for entry in entries if entry.get("advertised") is True
    )
    if not expected:
        fail("authoritative catalog has no advertised tools")

    process = subprocess.Popen(
        command(),
        cwd=ROOT,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    try:
        assert process.stdin is not None
        assert process.stdout is not None
        send_frame(
            process.stdin,
            1,
            "initialize",
            {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "mcp-catalog-drift-check", "version": "1"},
            },
        )
        response_result(read_frame(process.stdout), "initialize")

        send_frame(process.stdin, 2, "tools/list", {})
        result = response_result(read_frame(process.stdout), "tools/list")
        tools = result.get("tools")
        if not isinstance(tools, list):
            fail("tools/list result did not contain a tools array")
        actual = sorted(
            tool.get("name")
            for tool in tools
            if isinstance(tool, dict) and isinstance(tool.get("name"), str)
        )
        if actual != expected:
            missing = sorted(set(expected) - set(actual))
            extra = sorted(set(actual) - set(expected))
            fail(f"advertised tools differ (missing={missing}, extra={extra})")
        if any(
            not isinstance(tool, dict)
            or not isinstance(tool.get("description"), str)
            or not isinstance(tool.get("inputSchema"), dict)
            for tool in tools
        ):
            fail("tools/list contains an entry missing description or inputSchema")

        print(
            f"MCP catalog OK: {len(entries)} registered, "
            f"{len(expected)} advertised ({', '.join(expected)})"
        )
        return 0
    finally:
        if process.stdin is not None:
            process.stdin.close()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5)


if __name__ == "__main__":
    main()
