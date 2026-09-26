# mcp

Run pdftract as an MCP (Model Context Protocol) server. For copy-paste client
configuration, the complete tool catalog, authentication rules, root
restrictions, and wire-level validation command, see the
[MCP client configuration guide](../../../integrations/mcp-clients.md).

The default transport is stdio:

```bash
pdftract mcp --stdio
```

Use `--root DIR` to limit local PDF paths to a canonical directory. For HTTP
mode, use `--auth-token-file PATH` for a bearer token; non-loopback binds
require authentication.
