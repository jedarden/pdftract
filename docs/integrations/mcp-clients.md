# MCP Client Configuration Guide

This guide provides copy-paste-ready configuration snippets for connecting pdftract's MCP server to popular AI clients.

## Quick Start

The canonical invocation for the pdftract MCP server is:

```bash
pdftract mcp --stdio
```

Clients discover the binary by absolute path or via `PATH`. The server communicates over standard input/output using the JSON-RPC 2.0 protocol with LSP-style framing (Content-Length headers).

## Authoritative tool catalog

The [MCP tool catalog](./mcp-tool-catalog.json) is the authoritative list of
the ten tool names registered by pdftract. Each entry records whether the tool
is advertised by `tools/list`, whether it remains callable for compatibility,
and its implementation status. Tool names are unprefixed.

The current `tools/list` response contains the seven advertised names:

`extract`, `extract_text`, `extract_markdown`, `search`, `get_metadata`,
`hash`, and `get_table`.

`classify`, `get_form_fields`, and `get_attachments` are registered
compatibility stubs but are intentionally omitted from `tools/list`. Do not
select them from a client; direct calls return `NOT_YET_IMPLEMENTED`. The
advertised `search` and `get_table` entries can also return that in-band error
until their implementation phases land, so clients should always handle an
`isError: true` tool result.

Every client should discover tools with `tools/list` after connecting and use
the returned names rather than assuming that a client-specific prefix or a
future catalog entry is available. To check the documentation against the
actual wire response, run:

```bash
scripts/check-mcp-tool-catalog.py
```

Set `PDFTRACT_MCP_BIN=/path/to/pdftract` to check a prebuilt binary instead of
starting one through Cargo.

## Claude Desktop

### Configuration File Locations

| OS | Config Path |
|----|-------------|
| macOS | `~/Library/Application Support/Claude/claude_desktop_config.json` |
| Linux | `~/.config/Claude/claude_desktop_config.json` |
| Windows | `%APPDATA%\Claude\claude_desktop_config.json` |

### Configuration Snippet

Add the following to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "pdftract": {
      "command": "pdftract",
      "args": ["mcp", "--stdio"]
    }
  }
}
```

If pdftract is not on your `PATH`, use the absolute path:

```json
{
  "mcpServers": {
    "pdftract": {
      "command": "/usr/local/bin/pdftract",
      "args": ["mcp", "--stdio"]
    }
  }
}
```

### Validation

1. Restart Claude Desktop
2. Open a new conversation
3. Ask: "List available tools"
4. Verify that the seven advertised pdftract tools appear without a prefix: `extract`, `extract_text`, `extract_markdown`, `search`, `get_metadata`, `hash`, `get_table`

**Verified against:** Claude Desktop 1.0.0 (2026-05)

## Cursor

### Configuration File Location

- **macOS/Linux:** `~/.cursor/mcp_config.json`
- **Windows:** `%APPDATA%\Cursor\mcp_config.json`

### Configuration Snippet

```json
{
  "mcp": {
    "servers": {
      "pdftract": {
        "command": "pdftract",
        "args": ["mcp", "--stdio"]
      }
    }
  }
}
```

### Validation

1. Restart Cursor
2. Open the MCP panel (Settings → MCP Servers)
3. Verify `pdftract` appears as connected
4. Ask it to list tools and confirm the seven advertised names from the catalog
5. In chat, invoke a tool: `Extract text from document.pdf`

**Verified against:** Cursor 0.42.0 (2026-05)

## Continue

### Configuration File Location

- **macOS/Linux:** `~/.continue/config.yaml`
- **Windows:** `%USERPROFILE%\.continue\config.yaml`

### Configuration Snippet

```yaml
mcpServers:
  pdftract:
    command: pdftract
    args:
      - mcp
      - --stdio
```

### Validation

1. Restart Continue
2. Open the MCP Servers panel
3. Verify `pdftract` shows as "Connected"
4. Ask it to list tools and confirm the seven advertised names from the catalog
5. Test with: "Use pdftract to extract text from a PDF"

**Verified against:** Continue 2024.11.0

## Authentication and filesystem boundaries

Stdio mode is a local child process and does not require a bearer token. For
HTTP mode, use `--auth-token-file PATH` (recommended) or the
`PDFTRACT_MCP_TOKEN` environment variable. The token is sent by clients as an
`Authorization: Bearer <token>` header. Do not put a real token directly in a
command line; the deprecated `--auth-token VALUE` option is rejected unless
`PDFTRACT_INSECURE_CLI_TOKEN=1` is explicitly set.

Non-loopback HTTP binds such as `0.0.0.0:8080` refuse to start without a
token. Loopback binds (`127.0.0.1`, `127.0.0.0/8`, or `::1`) are exempt. A
typical authenticated HTTP invocation is:

```bash
pdftract mcp --bind 127.0.0.1:8080 --auth-token-file ~/.pdftract-token
```

Use `--root DIR` to constrain local PDF access for either transport:

```bash
pdftract mcp --stdio --root /srv/pdftract/input
```

With a root, every local `path` argument must be relative to that directory.
Absolute paths, `..` traversal, and symlinks that resolve outside the root are
rejected with JSON-RPC `-32602` and a `PATH_ESCAPES_ROOT` or
`ABSOLUTE_PATH_NOT_PERMITTED` data code. The root is canonicalized at startup,
so it must already exist and be a directory. HTTPS URLs are not resolved under
the local root; they still pass the server's remote/SSRF checks. Without
`--root`, local paths run in trust-the-caller mode.

## Custom Integration (SDK Template)

For SDK builders, here's a generic stdio MCP client harness in Python using `mcp-sdk-python`:

```python
import asyncio
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client

async def main():
    # Server parameters
    server_params = StdioServerParameters(
        command="pdftract",
        args=["mcp", "--stdio"]
    )

    async with stdio_client(server_params) as (read, write):
        async with ClientSession(read, write) as session:
            # Initialize connection
            await session.initialize()

            # List available tools
            tools = await session.list_tools()
            names = sorted(t.name for t in tools.tools)
            expected = sorted([
                "extract", "extract_text", "extract_markdown", "search",
                "get_metadata", "hash", "get_table",
            ])
            assert names == expected, (names, expected)
            print("Available tools:", names)

            # Call a tool (example: extract text)
            result = await session.call_tool(
                "extract_text",
                arguments={"path": "document.pdf"}
            )
            print("Result:", result.content)

if __name__ == "__main__":
    asyncio.run(main())
```

### Connection Lifecycle

1. **Spawn:** Start `pdftract mcp --stdio` as a subprocess (optionally add `--root DIR`)
2. **Handshake:** Send `initialize` request, receive capabilities
3. **List Tools:** Call `tools/list`; compare the returned names with the advertised entries in [`mcp-tool-catalog.json`](./mcp-tool-catalog.json)
4. **Call Tool:** Invoke `tools/call` with tool name and arguments
5. **Terminate:** Close subprocess; server exits on stdin EOF

### Error Handling

- **Parse errors:** Unparseable JSON gets a JSON-RPC error response pinned to `-32700`, with `id: null` and no `result`, and the server continues running — subsequent valid requests are served normally
- **Malformed requests:** JSON that parses but is not a valid single request — a bare string, an empty batch array, a wrong `jsonrpc` version, or a batch (batches are unsupported) — is likewise answered with an error envelope carrying `id: null`, whose `code` sits in the spec-reserved server-error range (`-32700` through `-32000`); the frame stream stays in sync and the connection remains usable
- **Invalid params:** Server returns a `-32602` error whose `data` carries a non-empty `reason` string explaining the rejection (e.g. `tools/call` without a `name` field). `--root` boundary rejections instead carry a `code` string in `data` (`PATH_ESCAPES_ROOT`, `ABSOLUTE_PATH_NOT_PERMITTED`)
- **Unknown tool:** Calling a tool that is not in the `tools/list` catalog returns `-32601` (method not found)
- **Advertised stubs:** `search` and `get_table` are listed but may return an in-band `NOT_YET_IMPLEMENTED` result; the three compatibility stubs omitted from `tools/list` must not be selected by discovery-based clients
- **Resilience:** None of the above kill the server — every error response still echoes the request `id`, the server keeps serving valid requests afterwards, and it still exits cleanly on stdin EOF. Only a genuinely broken pipe or a dead subprocess requires restarting.

These behaviors are asserted end-to-end by the `mcp-client-lifecycle` integration test (`crates/pdftract-cli/tests/mcp-client-lifecycle.rs`).

For the complete subprocess contract, see [`docs/notes/sdk-invocation.md`](../notes/sdk-invocation.md).

## Multi-Client Setup (HTTP Mode)

When running multiple MCP clients, use HTTP mode instead of spawning multiple stdio processes:

```bash
pdftract mcp --bind 127.0.0.1:8080 --auth-token-file ~/.pdftract-token
```

> **TH-03 Compliance:** Public binds (e.g., `0.0.0.0:8080`) **require** `--auth-token-file` or `PDFTRACT_MCP_TOKEN`. Loopback binds (`127.0.0.1`, `::1`) are exempt. Configure the client to send the token as an `Authorization: Bearer` header.

Configure clients to use HTTP endpoint (client-specific syntax varies; consult client documentation).

## Troubleshooting

### Binary not on PATH

**Symptom:** Client shows "Failed to start MCP server" or similar.

**Solution:** Use absolute path to the pdftract binary:

```json
{
  "command": "/absolute/path/to/pdftract",
  "args": ["mcp", "--stdio"]
}
```

### Permission denied (Linux/macOS)

**Symptom:** "Permission denied" when starting the server.

**Solution:** Ensure the binary is executable:

```bash
chmod +x /path/to/pdftract
```

On macOS, if Gatekeeper blocks the binary:

```bash
xattr -d com.apple.quarantine /path/to/pdftract
```

### Connection hangs / timeout

**Symptom:** Client waits indefinitely after starting MCP server.

**Cause:** Stdio pipe buffering issue.

**Solution:** Ensure the server process is producing output. Check stderr for logs:

```bash
pdftract mcp --stdio 2>mcp.log
```

If the log shows "stdio transport: stdout writer initialized", the server is running. The issue may be client-side framing.

### Tools not appearing

**Symptom:** Server connects but `tools/list` returns empty or tools don't appear in client UI.

**Diagnosis:**

1. Check stderr logs for initialization errors
2. Verify `pdftract --version` matches expected version
3. Test stdio mode manually (the server expects LSP-style `Content-Length` framing — a bare JSON line is silently consumed as a header and produces no response):

```bash
body='{"jsonrpc":"2.0","id":1,"method":"tools/list"}'
printf 'Content-Length: %d\r\n\r\n%s' "${#body}" "$body" | pdftract mcp --stdio
```

Expected response: a `Content-Length`-framed, valid JSON-RPC body with a
`result.tools` array containing exactly the advertised names in the
authoritative catalog. A quick automated check is
`scripts/check-mcp-tool-catalog.py`.

## References

- **Subprocess contract:** [`docs/notes/sdk-invocation.md`](../notes/sdk-invocation.md)
- **MCP specification:** https://modelcontextprotocol.io/spec
- **Security:** TH-03 (authentication for public binds) — see [`docs/plan/plan.md`](../plan/plan.md) line 892
- **Plan questions:** KU-5 (line 601), OQ-07 (line 518)
