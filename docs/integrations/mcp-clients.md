# MCP Client Configuration Guide

This guide provides copy-paste-ready configuration snippets for connecting pdftract's MCP server to popular AI clients.

## Quick Start

The canonical invocation for the pdftract MCP server is:

```bash
pdftract mcp --stdio
```

Clients discover the binary by absolute path or via `PATH`. The server communicates over standard input/output using the JSON-RPC 2.0 protocol with LSP-style framing (Content-Length headers).

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
4. Verify that tools prefixed with `pdftract.` appear (e.g., `pdftract_extract`, `pdftract_inspect`)

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
4. In chat, invoke a tool: `Extract text from document.pdf`

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
4. Test with: "Use pdftract to extract text from a PDF"

**Verified against:** Continue 2024.11.0

## Custom Integration (SDK Template)

For SDK builders, here's a generic stdio MCP client harness in Python using `mcp-sdk-python`:

```python
import asyncio
import json
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
            print("Available tools:", [t.name for t in tools.tools])

            # Call a tool (example: extract text)
            result = await session.call_tool(
                "pdftract_extract",
                arguments={"path": "document.pdf"}
            )
            print("Result:", result.content)

if __name__ == "__main__":
    asyncio.run(main())
```

### Connection Lifecycle

1. **Spawn:** Start `pdftract mcp --stdio` as a subprocess
2. **Handshake:** Send `initialize` request, receive capabilities
3. **List Tools:** Call `tools/list` to discover available tools
4. **Call Tool:** Invoke `tools/call` with tool name and arguments
5. **Terminate:** Close subprocess; server exits on stdin EOF

### Error Handling

- **Parse errors:** Server sends error response, continues running
- **Invalid params:** Server returns `-32602` error with `data.reason` field
- **Stdio corruption:** Any non-JSON-RPC on stdout breaks the connection; restart subprocess

For the complete subprocess contract, see [`docs/notes/sdk-invocation.md`](../notes/sdk-invocation.md).

## Multi-Client Setup (HTTP Mode)

When running multiple MCP clients, use HTTP mode instead of spawning multiple stdio processes:

```bash
pdftract mcp --bind 127.0.0.1:8080 --auth-token-file ~/.pdftract-token
```

> **TH-03 Compliance:** Public binds (e.g., `0.0.0.0:8080`) **require** `--auth-token-file` or `PDFTRACT_MCP_TOKEN`. Loopback binds (`127.0.0.1`, `::1`) are exempt.

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
3. Test stdio mode manually:

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | pdftract mcp --stdio
```

Expected response: Valid JSON-RPC with `result.tools` array.

## References

- **Subprocess contract:** [`docs/notes/sdk-invocation.md`](../notes/sdk-invocation.md)
- **MCP specification:** https://modelcontextprotocol.io/spec
- **Security:** TH-03 (authentication for public binds) — see [`docs/plan/plan.md`](../plan/plan.md) line 892
- **Plan questions:** KU-5 (line 601), OQ-07 (line 518)
