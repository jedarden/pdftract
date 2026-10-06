#!/usr/bin/env python3
"""Verify the MCP catalog, SDK mapping, and stdio client contract.

The check deliberately talks to a real ``pdftract mcp --stdio`` process. It
compares the wire-level tools/list response with both the JSON catalog and
the documented ten advertised names. It also parses the nine-method SDK
contract and checks the catalog's MCP-to-SDK mapping in both directions, so a
surface change cannot silently drift. It then calls every tool with valid
arguments, missing arguments, and a document-specific failure. Tool failures
must remain successful JSON-RPC responses whose result has ``isError: true``;
turning one into a top-level JSON-RPC error would break MCP clients that expect
to inspect the call result.

Beyond that generic smoke, every documented client configuration — the Claude
Desktop and Cursor JSON snippets and the Continue YAML snippet in
``docs/integrations/mcp-clients.md`` — is smoke-tested against its own stdio
server. Each snippet's ``command`` is resolved through an isolated executable
mapping to the binary under test, and its ``args`` are passed verbatim. This
exercises PATH lookup and the Claude Desktop absolute-path form without
writing to a system installation directory. Every launch must complete the full
connection lifecycle: initialize handshake, ``tools/list`` discovery against
the catalog, a successful invocation, a missing-argument in-band failure, an
unknown-tool ``-32601`` rejection, and a clean exit on stdin EOF. This is what
catches a documentation edit that changes a snippet's arguments without the
server contract following along.

Set ``PDFTRACT_MCP_BIN`` to a prebuilt binary (and optionally include
arguments) to avoid compiling. Otherwise the checker starts the workspace
binary through Cargo. ``PDFTRACT_MCP_TIMEOUT`` controls the per-frame timeout
in seconds.
"""

from __future__ import annotations

import json
import os
import re
import selectors
import shlex
import shutil
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any, NoReturn


ROOT = Path(__file__).resolve().parents[1]
CATALOG = ROOT / "docs/integrations/mcp-tool-catalog.json"
CLIENT_GUIDE = ROOT / "docs/integrations/mcp-clients.md"
SDK_CONTRACT = ROOT / "docs/notes/sdk-contract.md"
DEFAULT_TIMEOUT = 30.0

# Keep this explicit list next to the checker so a documentation-only rename
# cannot silently become the new contract. The catalog must agree with it.
DOCUMENTED_ADVERTISED_NAMES = (
    "extract",
    "extract_text",
    "extract_markdown",
    "search",
    "get_metadata",
    "hash",
    "get_table",
    "get_form_fields",
    "get_attachments",
    "classify",
)

# The stdio argument vector every documented client configuration must carry,
# and the initialize handshake every MCP client performs after spawning.
DEFAULT_STDIO_ARGS = ["mcp", "--stdio"]
ABSOLUTE_CLIENT_COMMAND = "/usr/local/bin/pdftract"
INITIALIZE_PARAMS: dict[str, Any] = {
    "protocolVersion": "2024-11-05",
    "capabilities": {},
    "clientInfo": {"name": "mcp-tool-contract-check", "version": "1"},
}

# Deliberately absent fixture: a schema-valid path whose document cannot be
# opened, exercising the in-band failure clients see for bad documents.
MISSING_DOCUMENT = "tests/fixtures/mcp-contract-missing.pdf"

VALID_ARGUMENTS: dict[str, dict[str, Any]] = {
    "extract": {
        "path": "tests/fixtures/test-minimal.pdf",
        "pages": "1",
        "ocr": False,
        "formats": ["json"],
        "auto_profile": False,
        "password": "",
        "receipts": "off",
    },
    "extract_text": {
        "path": "tests/fixtures/test-minimal.pdf",
        "pages": "1",
        "ocr": False,
        "password": "",
        "receipts": "off",
    },
    "extract_markdown": {
        "path": "tests/fixtures/test-minimal.pdf",
        "pages": "1",
        "ocr": False,
        "anchors": True,
        "password": "",
        "receipts": "off",
    },
    "search": {
        "path": "crates/pdftract-py/tests/fixtures/search_regression.pdf",
        "pattern": "PYTHON_SEARCH_REGRESSION",
        "case_insensitive": True,
        "max_matches": 10,
        "password": "",
    },
    "get_metadata": {"path": "tests/fixtures/test-minimal.pdf", "password": ""},
    "hash": {"path": "tests/fixtures/test-minimal.pdf", "password": ""},
    "get_table": {
        "path": "tests/fixtures/hybrid/hybrid-002-vector-form-over-scan.pdf",
        "page": 0,
        "table_index": 0,
        "password": "",
    },
    "get_form_fields": {
        "path": "tests/fixtures/test-minimal.pdf",
        "password": "",
    },
    "get_attachments": {
        "path": "tests/fixtures/test-minimal.pdf",
        "include_data": True,
    },
    "classify": {"path": "tests/fixtures/test-minimal.pdf"},
}


def validate_argument_vectors(catalog: dict[str, dict[str, Any]]) -> None:
    """Require each smoke vector to exercise its complete advertised schema."""
    for name in DOCUMENTED_ADVERTISED_NAMES:
        arguments = VALID_ARGUMENTS.get(name)
        if arguments is None:
            raise CheckFailure(f"missing valid argument vector for {name}")
        entry = catalog[name]
        expected = set(entry.get("required_arguments", [])) | set(
            entry.get("optional_arguments", [])
        )
        actual = set(arguments)
        if actual != expected:
            raise CheckFailure(
                f"valid argument vector for {name} does not cover its schema "
                f"(missing={sorted(expected - actual)}, "
                f"extra={sorted(actual - expected)})"
            )


def parse_sdk_surface() -> tuple[dict[str, str | None], int]:
    """Read the SDK method table's MCP column without a Markdown dependency."""
    try:
        markdown = SDK_CONTRACT.read_text(encoding="utf-8")
    except OSError as error:
        raise CheckFailure(f"cannot read SDK contract {SDK_CONTRACT}: {error}") from error

    in_method_surface = False
    methods: dict[str, str | None] = {}
    for line in markdown.splitlines():
        if line == "## Method surface":
            in_method_surface = True
            continue
        if in_method_surface and line.startswith("#"):
            break
        if not in_method_surface or not line.startswith("|"):
            continue

        columns = [column.strip() for column in line.strip().strip("|").split("|")]
        if len(columns) != 3 or columns[0] == "Method" or set(columns[0]) <= {"-"}:
            continue
        method_match = re.match(r"`([^`]+)`", columns[0])
        if method_match is None:
            raise CheckFailure(f"SDK method table has an invalid method row: {line}")
        method = method_match.group(1).split("(", 1)[0]
        if method in methods:
            raise CheckFailure(f"SDK method table repeats {method}")
        mcp_tool = None if columns[2] == "(n/a)" else columns[2].strip("`")
        if mcp_tool is not None and not re.fullmatch(r"[a-z][a-z0-9_]*", mcp_tool):
            raise CheckFailure(
                f"SDK method table has an invalid MCP tool for {method}: {mcp_tool!r}"
            )
        if mcp_tool is not None and mcp_tool in methods.values():
            raise CheckFailure(f"SDK method table repeats MCP tool {mcp_tool}")
        methods[method] = mcp_tool

    if len(methods) != 9:
        raise CheckFailure(
            f"SDK contract must define exactly nine methods; found {len(methods)}"
        )
    return methods, sum(mcp_tool is not None for mcp_tool in methods.values())


def validate_sdk_mapping(catalog: dict[str, dict[str, Any]]) -> None:
    """Require catalog mappings and the SDK contract's MCP column to agree."""
    sdk_methods, shared_count = parse_sdk_surface()
    catalog_mapping: dict[str, str] = {}

    for name in DOCUMENTED_ADVERTISED_NAMES:
        entry = catalog[name]
        if "sdk_method" not in entry or "mapping_status" not in entry:
            raise CheckFailure(f"catalog entry {name} lacks SDK mapping fields")
        sdk_method = entry["sdk_method"]
        status = entry["mapping_status"]
        if sdk_method is None:
            if status != "mcp_only" or not isinstance(entry.get("mapping_note"), str):
                raise CheckFailure(
                    f"MCP-only catalog entry {name} must have mapping_status "
                    "mcp_only and a mapping_note"
                )
            if not entry["mapping_note"].strip():
                raise CheckFailure(f"MCP-only catalog entry {name} has an empty mapping_note")
            continue

        if not isinstance(sdk_method, str) or sdk_method not in sdk_methods:
            raise CheckFailure(
                f"catalog entry {name} references unknown SDK method {sdk_method!r}"
            )
        if status != "shared":
            raise CheckFailure(f"shared catalog entry {name} must have mapping_status shared")
        catalog_mapping[name] = sdk_method

    contract_mapping = {
        mcp_tool: method
        for method, mcp_tool in sdk_methods.items()
        if mcp_tool is not None
    }
    if catalog_mapping != contract_mapping:
        raise CheckFailure(
            "MCP-to-SDK mapping drift: "
            f"catalog={catalog_mapping!r}, contract={contract_mapping!r}"
        )
    if shared_count != len(catalog_mapping):
        raise CheckFailure(
            f"shared MCP/SDK count drift: contract={shared_count}, "
            f"catalog={len(catalog_mapping)}"
        )


class CheckFailure(RuntimeError):
    """A contract assertion failed."""


def fail(message: str) -> NoReturn:
    print(f"MCP catalog/client contract failure: {message}", file=sys.stderr)
    raise SystemExit(1)


def _client_section(markdown: str, client: str) -> str:
    heading = f"## {client}"
    start = markdown.find(heading)
    if start < 0:
        raise CheckFailure(f"{client} section is missing from {CLIENT_GUIDE}")
    end = markdown.find("\n## ", start + len(heading))
    return markdown[start:] if end < 0 else markdown[start:end]


def _fenced_blocks(section: str, language: str) -> list[str]:
    pattern = rf"```{re.escape(language)}\s*\n(.*?)```"
    return re.findall(pattern, section, flags=re.DOTALL)


def _documented_server(config: Any, client: str, block_number: int) -> dict[str, Any]:
    """Pull the pdftract server entry out of a parsed client configuration."""
    if not isinstance(config, dict):
        raise CheckFailure(
            f"{client} configuration block {block_number} is not an object"
        )
    if client == "Cursor":
        mcp = config.get("mcp")
        servers = mcp.get("servers") if isinstance(mcp, dict) else None
    else:
        servers = config.get("mcpServers")
    if not isinstance(servers, dict) or not isinstance(servers.get("pdftract"), dict):
        raise CheckFailure(
            f"{client} configuration block {block_number} lacks mcp server pdftract"
        )
    return servers["pdftract"]


def _assert_stdio_config(config: Any, client: str, block_number: int) -> None:
    server = _documented_server(config, client, block_number)
    command = server.get("command")
    expected_command = (
        ABSOLUTE_CLIENT_COMMAND
        if client == "Claude Desktop" and block_number == 2
        else "pdftract"
    )
    if command != expected_command:
        raise CheckFailure(
            f"{client} configuration block {block_number} command {command!r} "
            f"must be {expected_command!r}"
        )
    if server.get("args") != DEFAULT_STDIO_ARGS:
        raise CheckFailure(
            f"{client} configuration block {block_number} "
            f"args {server.get('args')!r} must be {DEFAULT_STDIO_ARGS!r}"
        )


def _parse_simple_yaml(block: str, source: str) -> dict[str, Any]:
    """Parse the YAML subset the client guide uses: nested mappings and
    sequences of scalars. Anything outside that subset is a documentation
    contract failure rather than a parsing guess (CI has no PyYAML, and the
    documented snippets are the contract being checked)."""
    lines = [line for line in block.splitlines() if line.strip()]
    root: dict[str, Any] = {}
    stack: list[tuple[int, Any]] = [(-1, root)]
    for index, raw_line in enumerate(lines):
        indent = len(raw_line) - len(raw_line.lstrip(" "))
        line = raw_line.strip()
        while len(stack) > 1 and indent <= stack[-1][0]:
            stack.pop()
        container = stack[-1][1]

        if line.startswith("- "):
            if not isinstance(container, list):
                raise CheckFailure(
                    f"{source} YAML line {raw_line!r} is a sequence item "
                    "inside a mapping"
                )
            container.append(line[2:].strip())
            continue

        match = re.fullmatch(r"([^:\s]+):\s*(.*)", line)
        if not match:
            raise CheckFailure(
                f"{source} YAML line {raw_line!r} is not a supported mapping entry"
            )
        key, value = match.group(1), match.group(2).strip()
        if value:
            if not isinstance(container, dict):
                raise CheckFailure(
                    f"{source} YAML line {raw_line!r} puts a mapping key "
                    "inside a sequence"
                )
            container[key] = value
            continue

        # Empty value: the nested container's type is decided by the next line.
        next_indent = -1
        next_line = ""
        if index + 1 < len(lines):
            next_raw = lines[index + 1]
            next_indent = len(next_raw) - len(next_raw.lstrip(" "))
            next_line = next_raw.strip()
        child: Any = [] if next_indent > indent and next_line.startswith("- ") else {}
        if not isinstance(container, dict):
            raise CheckFailure(
                f"{source} YAML line {raw_line!r} puts a mapping key inside a sequence"
            )
        container[key] = child
        stack.append((indent, child))
    return root


@dataclass(frozen=True)
class ClientConfig:
    """One documented client snippet, reduced to its launch vector."""

    client: str
    label: str
    command: str
    args: list[str]


def parse_client_configs() -> list[ClientConfig]:
    """Parse and validate every client snippet users are told to paste."""
    try:
        markdown = CLIENT_GUIDE.read_text(encoding="utf-8")
    except OSError as error:
        raise CheckFailure(f"cannot read {CLIENT_GUIDE}: {error}") from error

    configs: list[ClientConfig] = []
    for client in ("Claude Desktop", "Cursor"):
        blocks = _fenced_blocks(_client_section(markdown, client), "json")
        expected_count = 2 if client == "Claude Desktop" else 1
        if len(blocks) != expected_count:
            raise CheckFailure(
                f"{client} must have {expected_count} JSON configuration "
                f"block(s); found {len(blocks)}"
            )
        for number, block in enumerate(blocks, start=1):
            try:
                config = json.loads(block)
            except json.JSONDecodeError as error:
                raise CheckFailure(
                    f"{client} configuration block {number} is invalid JSON: {error}"
                ) from error
            _assert_stdio_config(config, client, number)
            server = _documented_server(config, client, number)
            configs.append(
                ClientConfig(
                    client=client,
                    label=f"{client} block {number}",
                    command=server["command"],
                    args=list(server["args"]),
                )
            )

    continue_blocks = _fenced_blocks(_client_section(markdown, "Continue"), "yaml")
    if len(continue_blocks) != 1:
        raise CheckFailure(
            "Continue configuration must contain exactly one YAML block"
        )
    parsed = _parse_simple_yaml(continue_blocks[0], "Continue")
    _assert_stdio_config(parsed, "Continue", 1)
    server = _documented_server(parsed, "Continue", 1)
    configs.append(
        ClientConfig(
            client="Continue",
            label="Continue block 1",
            command=server["command"],
            args=list(server["args"]),
        )
    )
    return configs


def server_argv(stdio_args: list[str]) -> list[str]:
    """Underlying binary and optional prefix, followed by the given stdio args."""
    configured = os.environ.get("PDFTRACT_MCP_BIN")
    if configured:
        parts = shlex.split(configured)
        if not parts:
            raise CheckFailure("PDFTRACT_MCP_BIN contains no executable")
        # Replace a trailing generic stdio tail rather than stacking onto it.
        if len(parts) >= 2 and parts[-2:] == DEFAULT_STDIO_ARGS:
            parts = parts[:-2]
        return [*parts, *stdio_args]
    return [
        "cargo",
        "run",
        "--quiet",
        "-p",
        "pdftract-cli",
        "--features",
        "mcp",
        "--",
        *stdio_args,
    ]


def server_executable(argv: list[str]) -> Path:
    """Resolve the underlying binary before creating the isolated mapping."""
    found = shutil.which(argv[0])
    if found is None:
        source = "PDFTRACT_MCP_BIN" if os.environ.get("PDFTRACT_MCP_BIN") else "Cargo"
        raise CheckFailure(
            f"{source} executable {argv[0]!r} was not found or is not executable"
        )
    return Path(found).resolve()


def _map_executable(source: Path, destination: Path) -> None:
    """Create a local executable alias, including on systems without symlinks."""
    destination.parent.mkdir(parents=True, exist_ok=True)
    try:
        destination.symlink_to(source)
    except OSError:
        try:
            os.link(source, destination)
        except OSError:
            shutil.copy2(source, destination)


def create_launch_mapping(directory: Path, executable: Path) -> None:
    """Install only the two command forms documented in the client guide."""
    suffix = ".exe" if os.name == "nt" else ""
    _map_executable(executable, directory / "bin" / f"pdftract{suffix}")
    absolute_relative = ABSOLUTE_CLIENT_COMMAND.lstrip("/") + suffix
    _map_executable(executable, directory / absolute_relative)


def resolve_client_command(config: ClientConfig, directory: Path) -> Path:
    """Resolve a snippet command inside the mapping, never on the host PATH."""
    if config.command == "pdftract":
        found = shutil.which(config.command, path=str(directory / "bin"))
        resolved = Path(found) if found is not None else None
    elif config.command == ABSOLUTE_CLIENT_COMMAND:
        suffix = ".exe" if os.name == "nt" else ""
        resolved = directory / (config.command.lstrip("/") + suffix)
    else:
        resolved = None
    if resolved is None or not resolved.is_file() or not os.access(resolved, os.X_OK):
        raise CheckFailure(
            f"{config.label} command {config.command!r} has no executable "
            "in the isolated client mapping"
        )
    return resolved


@dataclass
class FrameClient:
    process: subprocess.Popen[bytes]
    timeout: float

    def __post_init__(self) -> None:
        assert self.process.stdin is not None
        assert self.process.stdout is not None
        self.stdin = self.process.stdin
        self.stdout = self.process.stdout
        self.selector = selectors.DefaultSelector()
        self.selector.register(self.stdout, selectors.EVENT_READ)
        self.next_id = 1

    def _read_exact(self, size: int) -> bytes:
        data = bytearray()
        deadline = time.monotonic() + self.timeout
        while len(data) < size:
            remaining = deadline - time.monotonic()
            if remaining <= 0 or not self.selector.select(remaining):
                raise CheckFailure(
                    f"timed out reading response body ({len(data)}/{size} bytes)"
                )
            chunk = os.read(self.stdout.fileno(), size - len(data))
            if not chunk:
                raise CheckFailure("server closed stdout in the middle of a response")
            data.extend(chunk)
        return bytes(data)

    def _read_line(self) -> bytes:
        data = bytearray()
        deadline = time.monotonic() + self.timeout
        while True:
            remaining = deadline - time.monotonic()
            if remaining <= 0 or not self.selector.select(remaining):
                raise CheckFailure(f"timed out reading response header: {data!r}")
            chunk = os.read(self.stdout.fileno(), 1)
            if not chunk:
                raise CheckFailure("server closed stdout before a response")
            data.extend(chunk)
            if chunk == b"\n":
                return bytes(data)

    def receive(self) -> dict[str, Any]:
        headers: dict[str, str] = {}
        while True:
            line = self._read_line()
            if line in (b"\r\n", b"\n"):
                break
            try:
                key, value = line.decode("ascii").rstrip("\r\n").split(":", 1)
            except (UnicodeDecodeError, ValueError) as error:
                raise CheckFailure(f"invalid response header {line!r}: {error}") from error
            headers[key.lower()] = value.strip()

        try:
            length = int(headers["content-length"])
        except (KeyError, ValueError) as error:
            raise CheckFailure(f"response lacks valid Content-Length: {headers}") from error

        try:
            response = json.loads(self._read_exact(length))
        except json.JSONDecodeError as error:
            raise CheckFailure(f"response body is not JSON: {error}") from error
        if not isinstance(response, dict):
            raise CheckFailure("JSON-RPC response is not an object")
        return response

    def send(self, method: str, params: dict[str, Any] | None = None) -> dict[str, Any]:
        request_id = self.next_id
        self.next_id += 1
        request: dict[str, Any] = {
            "jsonrpc": "2.0",
            "id": request_id,
            "method": method,
        }
        if params is not None:
            request["params"] = params
        body = json.dumps(request, separators=(",", ":")).encode("utf-8")
        self.stdin.write(f"Content-Length: {len(body)}\r\n\r\n".encode("ascii"))
        self.stdin.write(body)
        self.stdin.flush()

        response = self.receive()
        if response.get("id") != request_id:
            raise CheckFailure(
                f"{method} response id {response.get('id')!r} != {request_id}"
            )
        return response

    def notify(self, method: str) -> None:
        body = json.dumps(
            {"jsonrpc": "2.0", "method": method}, separators=(",", ":")
        ).encode("utf-8")
        self.stdin.write(f"Content-Length: {len(body)}\r\n\r\n".encode("ascii"))
        self.stdin.write(body)
        self.stdin.flush()

    def close(self, require_clean_exit: bool = False) -> None:
        """Close stdin (the documented terminate step) and reap the process.

        Every wait is bounded so a wedged server can never hang the checker.
        With ``require_clean_exit`` the documented exit-on-EOF behavior is
        asserted: the server must have exited on its own within the bound and
        with status 0.
        """
        self.selector.close()
        try:
            self.stdin.close()
        except OSError:
            pass
        exited = False
        try:
            self.process.wait(timeout=5)
            exited = True
        except subprocess.TimeoutExpired:
            pass
        if not exited:
            self.process.terminate()
            try:
                self.process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait(timeout=5)
        if require_clean_exit:
            if not exited:
                raise CheckFailure(
                    "server did not exit within 5s of stdin EOF "
                    "(documented terminate step broken)"
                )
            if self.process.returncode != 0:
                raise CheckFailure(
                    f"server exited with code {self.process.returncode} on "
                    "stdin EOF (expected 0)"
                )


def start_client(
    timeout: float,
    argv: list[str] | None = None,
    *,
    executable: Path | None = None,
    env: dict[str, str] | None = None,
) -> FrameClient:
    process = subprocess.Popen(
        argv if argv is not None else server_argv(DEFAULT_STDIO_ARGS),
        executable=executable,
        env=env,
        cwd=ROOT,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        bufsize=0,
        start_new_session=True,
    )
    return FrameClient(process, timeout)


def result_of(response: dict[str, Any], operation: str) -> dict[str, Any]:
    if "error" in response:
        raise CheckFailure(f"{operation} returned top-level error: {response['error']}")
    result = response.get("result")
    if not isinstance(result, dict):
        raise CheckFailure(f"{operation} did not return an object result")
    return result


def load_catalog() -> tuple[list[dict[str, Any]], dict[str, dict[str, Any]]]:
    try:
        catalog = json.loads(CATALOG.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        fail(f"cannot read {CATALOG}: {error}")
    if not isinstance(catalog, dict):
        fail("authoritative catalog is not an object")

    entries = catalog.get("tools")
    if not isinstance(entries, list) or len(entries) != 10:
        fail("authoritative catalog must contain exactly ten tools")
    if any(not isinstance(entry, dict) for entry in entries):
        fail("authoritative catalog contains a non-object tool entry")

    names = [entry["name"] for entry in entries if isinstance(entry.get("name"), str)]
    if len(names) != 10 or len(set(names)) != 10:
        fail("authoritative catalog must contain ten unique string names")
    if set(names) != set(DOCUMENTED_ADVERTISED_NAMES):
        fail(
            "catalog names do not match the documented ten advertised names "
            f"({', '.join(DOCUMENTED_ADVERTISED_NAMES)})"
        )
    if any(entry.get("advertised") is not True for entry in entries):
        fail("all ten documented tools must be advertised")
    if any(entry.get("callable") is not True for entry in entries):
        fail("all ten advertised tools must be callable")

    by_name = {entry["name"]: entry for entry in entries}
    try:
        validate_sdk_mapping(by_name)
        validate_argument_vectors(by_name)
    except CheckFailure as error:
        fail(str(error))
    missing_fixtures = sorted(
        str(ROOT / args["path"])
        for args in VALID_ARGUMENTS.values()
        if not (ROOT / args["path"]).is_file()
    )
    if missing_fixtures:
        fail(f"smoke fixture is missing: {missing_fixtures[0]}")
    return entries, by_name


def compare_tools_list(
    result: dict[str, Any], catalog: dict[str, dict[str, Any]]
) -> None:
    tools = result.get("tools")
    if not isinstance(tools, list):
        raise CheckFailure("tools/list result did not contain a tools array")
    actual: dict[str, dict[str, Any]] = {}
    for tool in tools:
        if not isinstance(tool, dict) or not isinstance(tool.get("name"), str):
            raise CheckFailure("tools/list contains an invalid tool entry")
        name = tool["name"]
        if name in actual:
            raise CheckFailure(f"tools/list contains duplicate tool {name!r}")
        actual[name] = tool

    expected_names = set(DOCUMENTED_ADVERTISED_NAMES)
    actual_names = set(actual)
    if actual_names != expected_names:
        raise CheckFailure(
            "advertised tools differ "
            f"(missing={sorted(expected_names - actual_names)}, "
            f"extra={sorted(actual_names - expected_names)})"
        )

    for name in DOCUMENTED_ADVERTISED_NAMES:
        tool = actual[name]
        entry = catalog[name]
        if tool.get("description") != entry.get("description"):
            raise CheckFailure(f"description drift for {name}")
        schema = tool.get("inputSchema")
        if not isinstance(schema, dict):
            raise CheckFailure(f"{name} is missing an inputSchema object")
        properties = schema.get("properties")
        required = schema.get("required")
        if not isinstance(properties, dict) or not isinstance(required, list):
            raise CheckFailure(f"{name} inputSchema lacks properties or required")
        expected_required = sorted(entry.get("required_arguments", []))
        expected_optional = sorted(entry.get("optional_arguments", []))
        if sorted(required) != expected_required:
            raise CheckFailure(
                f"required argument drift for {name}: "
                f"expected={expected_required}, actual={sorted(required)}"
            )
        expected_arguments = set(expected_required + expected_optional)
        if set(properties) != expected_arguments:
            raise CheckFailure(
                f"argument drift for {name}: "
                f"expected={sorted(expected_arguments)}, actual={sorted(properties)}"
            )


def assert_call_result(response: dict[str, Any], name: str, want_error: bool) -> None:
    result = result_of(response, f"tools/call {name}")
    if not isinstance(result.get("isError"), bool):
        raise CheckFailure(f"tools/call {name} result lacks boolean isError")
    if result["isError"] is not want_error:
        raise CheckFailure(
            f"tools/call {name} expected isError={want_error}, "
            f"got {result['isError']}"
        )
    content = result.get("content")
    if not isinstance(content, list) or not content:
        raise CheckFailure(f"tools/call {name} result lacks content")


def assert_initialize_result(result: dict[str, Any]) -> None:
    """Assert the handshake fields clients inspect to show a connection."""
    if result.get("protocolVersion") != "2024-11-05":
        raise CheckFailure(
            "initialize advertised protocolVersion "
            f"{result.get('protocolVersion')!r}, expected '2024-11-05'"
        )
    capabilities = result.get("capabilities")
    if not isinstance(capabilities, dict) or not isinstance(
        capabilities.get("tools"), dict
    ):
        raise CheckFailure("initialize did not advertise a tools capability object")
    server_info = result.get("serverInfo")
    if (
        not isinstance(server_info, dict)
        or not isinstance(server_info.get("name"), str)
        or not server_info["name"]
    ):
        raise CheckFailure("initialize did not return serverInfo with a name")


def assert_unknown_tool(response: dict[str, Any]) -> None:
    """Unknown tools must be JSON-RPC method-not-found errors."""
    error = response.get("error")
    if not isinstance(error, dict) or error.get("code") != -32601:
        raise CheckFailure(
            f"unknown tool must be rejected with top-level -32601, got {error!r}"
        )


def run_tool_conformance(
    client: FrameClient,
) -> tuple[int, int, int]:
    """Exercise every advertised tool's success and in-band failure paths."""
    valid_calls = 0
    invalid_calls = 0
    document_failures = 0
    for name in DOCUMENTED_ADVERTISED_NAMES:
        arguments = VALID_ARGUMENTS[name]
        valid = client.send("tools/call", {"name": name, "arguments": arguments})
        assert_call_result(valid, name, want_error=False)
        valid_calls += 1

        invalid = client.send("tools/call", {"name": name, "arguments": {}})
        assert_call_result(invalid, name, want_error=True)
        invalid_calls += 1

        document_args = dict(arguments)
        document_args["path"] = MISSING_DOCUMENT
        document_failure = client.send(
            "tools/call", {"name": name, "arguments": document_args}
        )
        # A document error is a successful JSON-RPC response with an MCP error
        # result. This catches accidental Response::error changes.
        assert_call_result(document_failure, name, want_error=True)
        document_failures += 1

    unknown = client.send(
        "tools/call",
        {"name": "definitely_not_a_pdftract_tool", "arguments": {}},
    )
    assert_unknown_tool(unknown)
    return valid_calls, invalid_calls, document_failures


def smoke_client_config(
    config: ClientConfig,
    catalog_by_name: dict[str, dict[str, Any]],
    timeout: float,
    mapping: Path,
    server_prefix: list[str],
) -> str:
    """Smoke-test one documented client configuration end to end.

    Launches the snippet's exact argument vector, then walks the documented
    connection lifecycle: initialize, tools/list discovery against the
    catalog, per-tool invocation behavior (success, invalid arguments,
    in-band document failure, unknown-tool rejection), and a clean exit on
    stdin EOF.
    """
    executable = resolve_client_command(config, mapping)
    argv = [config.command, *server_prefix, *config.args]
    env = os.environ.copy()
    env["PATH"] = str(mapping / "bin") + os.pathsep + env.get("PATH", "")
    try:
        client = start_client(
            timeout,
            argv,
            executable=executable if config.command == ABSOLUTE_CLIENT_COMMAND else None,
            env=env,
        )
    except OSError as error:
        raise CheckFailure(
            f"{config.label} could not launch command {config.command!r} "
            f"with args {config.args!r}: {error}"
        ) from error
    try:
        assert_initialize_result(
            result_of(client.send("initialize", INITIALIZE_PARAMS), "initialize")
        )
        client.notify("notifications/initialized")
        compare_tools_list(
            result_of(client.send("tools/list", {}), "tools/list"), catalog_by_name
        )
        run_tool_conformance(client)
    except BaseException:
        client.close()
        raise
    client.close(require_clean_exit=True)
    return shlex.join([config.command, *config.args])


def run_smoke(client: FrameClient, catalog_by_name: dict[str, dict[str, Any]]) -> None:
    result_of(client.send("initialize", INITIALIZE_PARAMS), "initialize")
    client.notify("notifications/initialized")
    catalog_result = result_of(client.send("tools/list", {}), "tools/list")
    compare_tools_list(catalog_result, catalog_by_name)

    valid_calls, invalid_calls, document_failures = run_tool_conformance(client)

    print(
        "MCP catalog/client contract OK: "
        f"{len(catalog_by_name)} advertised tools; "
        f"{valid_calls} valid, {invalid_calls} invalid, "
        f"{document_failures} in-band document-failure calls"
    )


def main() -> int:
    try:
        configs = parse_client_configs()
        _, catalog_by_name = load_catalog()
        timeout = float(os.environ.get("PDFTRACT_MCP_TIMEOUT", DEFAULT_TIMEOUT))
        if timeout <= 0:
            fail("PDFTRACT_MCP_TIMEOUT must be positive")

        underlying = server_argv([])
        server_binary = server_executable(underlying)

        client = start_client(timeout)
        try:
            run_smoke(client, catalog_by_name)
        finally:
            client.close()

        with tempfile.TemporaryDirectory(
            prefix=".pdftract-mcp-launch-", dir=ROOT
        ) as temporary:
            mapping = Path(temporary)
            create_launch_mapping(mapping, server_binary)
            for config in configs:
                vector = smoke_client_config(
                    config, catalog_by_name, timeout, mapping, underlying[1:]
                )
                print(f"MCP client config smoke OK: {config.label} [{vector}]")
        print(
            f"MCP client configuration interop OK: {len(configs)} documented "
            f"configurations ({', '.join(dict.fromkeys(c.client for c in configs))})"
        )
    except (CheckFailure, OSError, ValueError) as error:
        fail(str(error))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
