#!/usr/bin/env python3
# /// script
# requires-python = "==3.11.*"
# dependencies = ["mcp==1.30.0"]
# ///
"""Exercise every documented stdio launch with the official MCP Python SDK.

Run with ``uv run --locked --python 3.11 --script scripts/check-mcp-sdk-client.py``. The companion
``check-mcp-tool-catalog.py`` remains the detailed wire-level baseline.
"""

from __future__ import annotations

import argparse
import importlib.metadata
import importlib.util
import os
import sys
import tempfile
from dataclasses import replace
from datetime import timedelta
from pathlib import Path
from unittest.mock import patch

import anyio
from mcp import ClientSession, StdioServerParameters
from mcp.client import stdio as sdk_stdio
from mcp.shared.exceptions import McpError


ROOT = Path(__file__).resolve().parents[1]
CHECKER = ROOT / "scripts/check-mcp-tool-catalog.py"
SDK_VERSION = "1.30.0"
UNKNOWN_TOOL = "definitely_not_a_pdftract_tool"


def load_checker():
    spec = importlib.util.spec_from_file_location("check_mcp_tool_catalog", CHECKER)
    assert spec is not None and spec.loader is not None
    checker = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = checker
    spec.loader.exec_module(checker)
    return checker


def describe_error(error: Exception) -> str:
    if isinstance(error, BaseExceptionGroup):
        return "; ".join(describe_error(child) for child in error.exceptions)
    return f"{type(error).__name__}: {error}"


async def check_config(config, checker, catalog, mapping: Path, prefix: list[str], timeout: float):
    """Return the failed lifecycle stage, or None after a clean server exit."""
    stage = "launch"
    process = None
    log_path = mapping / f"sdk-stderr-{config.label.replace(' ', '-')}.log"
    try:
        resolved = checker.resolve_client_command(config, mapping)
        # The absolute path in the guide is represented by an executable in
        # the isolated mapping. PATH-based snippets retain their literal command.
        command = str(resolved) if config.command == checker.ABSOLUTE_CLIENT_COMMAND else config.command
        env = {"PATH": str(mapping / "bin") + os.pathsep + os.environ.get("PATH", "")}
        params = StdioServerParameters(
            command=command,
            args=[*prefix, *config.args],
            env=env,
            cwd=str(ROOT),
        )

        # stdio_client owns shutdown but does not expose the process status.
        # Capture the process it creates so exit code 0 can be asserted after
        # the SDK closes stdin and waits for EOF. This hook is pinned to 1.30.0.
        create_process = sdk_stdio._create_platform_compatible_process

        async def capture_process(*args, **kwargs):
            nonlocal process
            process = await create_process(*args, **kwargs)
            return process

        with open(log_path, "w", encoding="utf-8") as errlog:
            with patch.object(sdk_stdio, "_create_platform_compatible_process", capture_process):
                # The outer deadline bounds launch and SDK context cleanup too.
                with anyio.fail_after(timeout * 7 + 10):
                    async with sdk_stdio.stdio_client(params, errlog=errlog) as (read, write):
                        async with ClientSession(
                            read, write, read_timeout_seconds=timedelta(seconds=timeout)
                        ) as session:
                            stage = "initialize"
                            with anyio.fail_after(timeout):
                                initialized = await session.initialize()
                            if initialized.protocolVersion != "2024-11-05":
                                raise AssertionError(
                                    f"protocol version {initialized.protocolVersion!r}"
                                )

                            stage = "tools/list"
                            with anyio.fail_after(timeout):
                                listed = await session.list_tools()
                            checker.compare_tools_list(
                                listed.model_dump(by_alias=True), catalog
                            )

                            stage = "successful tools/call"
                            with anyio.fail_after(timeout):
                                success = await session.call_tool(
                                    "get_metadata", checker.VALID_ARGUMENTS["get_metadata"]
                                )
                            if success.isError or not success.content:
                                raise AssertionError("get_metadata did not return successful content")

                            stage = "in-band tool failure"
                            with anyio.fail_after(timeout):
                                failed = await session.call_tool(
                                    "get_metadata", {"path": checker.MISSING_DOCUMENT}
                                )
                            if failed.isError is not True or not failed.content:
                                raise AssertionError("missing document did not fail in-band")

                            stage = "unknown-tool rejection"
                            try:
                                with anyio.fail_after(timeout):
                                    await session.call_tool(UNKNOWN_TOOL, {})
                            except McpError as error:
                                if error.error.code != -32601:
                                    raise AssertionError(
                                        f"unknown tool returned code {error.error.code}"
                                    ) from error
                            else:
                                raise AssertionError("unknown tool was not rejected")

                            stage = "shutdown"
        if process is None or process.returncode != 0:
            raise AssertionError(
                f"server exit code {None if process is None else process.returncode}, expected 0"
            )
    except Exception as error:
        details = describe_error(error)
        if process is not None and process.returncode is not None:
            details += f"; server exit code {process.returncode}"
        if log_path.exists():
            tail = log_path.read_text(encoding="utf-8", errors="replace").splitlines()[-8:]
            if tail:
                details += "; server stderr: " + " | ".join(tail)
        return stage, details
    return None


async def run_configs(configs, checker, catalog, mapping: Path, prefix: list[str], timeout: float):
    failures = {}
    for config in configs:
        result = await check_config(config, checker, catalog, mapping, prefix, timeout)
        if result is None:
            print(f"MCP SDK session OK: {config.label} [{config.command} {' '.join(config.args)}]")
        else:
            stage, details = result
            failures[config.label] = stage
            print(f"MCP SDK session FAIL: {config.label} stage={stage}: {details}", file=sys.stderr)
    return failures


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--probe-broken-config",
        action="store_true",
        help="prove a bad Cursor launch fails while all other snippets still run",
    )
    args = parser.parse_args()
    if importlib.metadata.version("mcp") != SDK_VERSION:
        parser.error(f"official MCP SDK {SDK_VERSION} is required")
    checker = load_checker()
    try:
        configs = checker.parse_client_configs()
        _, catalog = checker.load_catalog()
        timeout = float(os.environ.get("PDFTRACT_MCP_TIMEOUT", checker.DEFAULT_TIMEOUT))
        if timeout <= 0:
            raise ValueError("PDFTRACT_MCP_TIMEOUT must be positive")
        underlying = checker.server_argv([])
        binary = checker.server_executable(underlying)
    except (checker.CheckFailure, OSError, ValueError) as error:
        print(f"MCP SDK setup FAIL: {error}", file=sys.stderr)
        return 1

    if args.probe_broken_config:
        configs = [
            replace(config, args=["mcp", "--broken-stdio-flag"])
            if config.label == "Cursor block 1"
            else config
            for config in configs
        ]

    with tempfile.TemporaryDirectory(prefix=".pdftract-mcp-sdk-", dir=ROOT) as temporary:
        mapping = Path(temporary)
        checker.create_launch_mapping(mapping, binary)
        failures = anyio.run(run_configs, configs, checker, catalog, mapping, underlying[1:], timeout)

    if args.probe_broken_config:
        expected = {"Cursor block 1": "initialize"}
        if failures != expected:
            print(f"MCP SDK broken-config probe FAIL: expected {expected}, got {failures}", file=sys.stderr)
            return 1
        print("MCP SDK broken-config probe OK: Cursor failed at initialize; other snippets passed")
        return 0
    if failures:
        print(f"MCP SDK client configuration interop FAIL: {len(failures)} snippet(s)", file=sys.stderr)
        return 1
    print(f"MCP SDK client configuration interop OK: {len(configs)} snippets")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
