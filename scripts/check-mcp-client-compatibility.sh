#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

echo "=== Building the release MCP stdio server ==="
binary="$(
    cargo build --locked --release --package pdftract-cli --bin pdftract --features mcp --message-format=json |
        python3 -c '
import json
import sys

paths = []
for line in sys.stdin:
    message = json.loads(line)
    target = message.get("target", {})
    if (
        message.get("reason") == "compiler-artifact"
        and target.get("name") == "pdftract"
        and "bin" in target.get("kind", [])
        and message.get("executable")
    ):
        paths.append(message["executable"])
if len(paths) != 1:
    raise SystemExit(f"expected one built pdftract executable, found {len(paths)}")
print(paths[0])
'
)"
test -x "$binary"

echo "=== Checking documented launch vectors and wire-level tool catalog ==="
PDFTRACT_MCP_BIN="$binary" python3 scripts/check-mcp-tool-catalog.py

echo "=== Checking documented configurations with the pinned MCP Python SDK ==="
PDFTRACT_MCP_BIN="$binary" UV_CACHE_DIR="${UV_CACHE_DIR:-$PWD/.mcp-sdk-cache}" \
    UV_PYTHON_INSTALL_DIR="${UV_PYTHON_INSTALL_DIR:-$PWD/.mcp-sdk-python}" \
    uv run --locked --python 3.11 --script scripts/check-mcp-sdk-client.py
