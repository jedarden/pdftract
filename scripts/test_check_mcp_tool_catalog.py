#!/usr/bin/env python3
"""Focused launch-vector checks for the documented MCP client snippets."""

from __future__ import annotations

import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[1]
CHECKER = ROOT / "scripts/check-mcp-tool-catalog.py"
SPEC = importlib.util.spec_from_file_location("check_mcp_tool_catalog", CHECKER)
assert SPEC is not None and SPEC.loader is not None
checker = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = checker
SPEC.loader.exec_module(checker)


class ClientLaunchTests(unittest.TestCase):
    def test_all_documented_commands_launch_with_verbatim_args(self) -> None:
        configs = checker.parse_client_configs()
        self.assertEqual(
            [(config.label, config.command) for config in configs],
            [
                ("Claude Desktop block 1", "pdftract"),
                ("Claude Desktop block 2", "/usr/local/bin/pdftract"),
                ("Cursor block 1", "pdftract"),
                ("Continue block 1", "pdftract"),
            ],
        )
        with tempfile.TemporaryDirectory(dir=ROOT) as temporary:
            mapping = Path(temporary)
            checker.create_launch_mapping(mapping, Path(sys.executable))
            for config in configs:
                with self.subTest(config=config.label):
                    resolved = checker.resolve_client_command(config, mapping)
                    env = os.environ.copy()
                    env["PATH"] = str(mapping / "bin") + os.pathsep + env.get("PATH", "")
                    completed = subprocess.run(
                        [
                            config.command,
                            "-c",
                            "import json, sys; print(json.dumps(sys.argv[1:]))",
                            *config.args,
                        ],
                        executable=(
                            resolved
                            if config.command == checker.ABSOLUTE_CLIENT_COMMAND
                            else None
                        ),
                        env=env,
                        check=True,
                        capture_output=True,
                        text=True,
                    )
                    self.assertEqual(json.loads(completed.stdout), config.args)

    def test_wrong_command_and_argument_fail_with_named_snippet(self) -> None:
        guide = checker.CLIENT_GUIDE.read_text(encoding="utf-8")
        cases = (
            (
                '"command": "pdftract"',
                '"command": "missing-pdftract"',
                "Claude Desktop configuration block 1 command",
            ),
            (
                '"command": "/usr/local/bin/pdftract"',
                '"command": "/opt/bin/pdftract"',
                "Claude Desktop configuration block 2 command",
            ),
            (
                '"args": ["mcp", "--stdio"]',
                '"args": ["mcp", "--http"]',
                "Claude Desktop configuration block 1 args",
            ),
            (
                "      - --stdio",
                "      - --http",
                "Continue configuration block 1 args",
            ),
        )
        with tempfile.TemporaryDirectory(dir=ROOT) as temporary:
            guide_path = Path(temporary) / "mcp-clients.md"
            for old, new, expected in cases:
                with self.subTest(expected=expected):
                    self.assertIn(old, guide)
                    guide_path.write_text(guide.replace(old, new, 1), encoding="utf-8")
                    with patch.object(checker, "CLIENT_GUIDE", guide_path):
                        with self.assertRaisesRegex(checker.CheckFailure, expected):
                            checker.parse_client_configs()

    def test_missing_executable_fails_with_named_source_and_snippet(self) -> None:
        with patch.dict(os.environ, {"PDFTRACT_MCP_BIN": "/missing/pdftract"}):
            with self.assertRaisesRegex(
                checker.CheckFailure, "PDFTRACT_MCP_BIN executable.*not found"
            ):
                checker.server_executable(checker.server_argv([]))

        config = checker.parse_client_configs()[2]
        with tempfile.TemporaryDirectory(dir=ROOT) as temporary:
            with self.assertRaisesRegex(
                checker.CheckFailure, "Cursor block 1 command.*no executable"
            ):
                checker.resolve_client_command(config, Path(temporary))


if __name__ == "__main__":
    unittest.main()
