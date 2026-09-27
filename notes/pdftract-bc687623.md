# pdftract-bc687623 — automated MCP client-configuration interoperability checks

## What was done

Extended `scripts/check-mcp-tool-catalog.py` beyond parsing the documented
client snippets and running one generic server smoke. The checker now
launches **one stdio server per documented client configuration** — the two
Claude Desktop JSON blocks, the Cursor JSON block, and the Continue YAML
block from `docs/integrations/mcp-clients.md` — using each snippet's exact
`command`/`args` vector (the binary under test is substituted for the
`pdftract` command name; PATH/absolute-path resolution is client behavior
the doc's absolute-path variant already covers). Each per-client run
asserts the full documented connection lifecycle:

1. **Connection** — `initialize` handshake; result must carry
   `protocolVersion: "2024-11-05"`, a `capabilities.tools` object, and a
   non-empty `serverInfo.name` (mirrors `mcp-client-lifecycle.rs:515`).
2. **Discovery** — `tools/list` compared against the authoritative catalog
   (names, descriptions, required/optional argument drift).
3. **Invocation** — a real `extract_text` call (isError=false), a
   missing-document call (in-band `isError=true`), and an unknown-tool call
   (top-level `-32601`).
4. **Terminate** — clean exit with status 0 within 5 s of stdin EOF.

### Supporting changes

- `server_argv(stdio_args)` generalizes the old `command()` so any client
  snippet's args can be launched against the same binary under test
  (`PDFTRACT_MCP_BIN` tail-replacement, cargo-run fallback).
- `_parse_simple_yaml` parses the Continue YAML subset (nested mappings +
  scalar sequences) without PyYAML — the CI image (`rust:1.83-bookworm`)
  does not have it. Anything outside the subset is a CheckFailure that
  names the offending line, forcing doc/tooling alignment.
- `FrameClient.close(require_clean_exit=...)` asserts the terminate step
  while keeping every wait bounded (TH-03 hygiene).
- `Makefile` `mcp-verify` now exports `PDFTRACT_MCP_BIN` resolved via
  `cargo metadata target_directory` instead of letting the checker spawn
  through `cargo run` five times — found live during this bead: with the
  cargo-run fallback, a concurrent worker's rebuild blew the 30 s frame
  timeout on the second spawn (commit da464f9a's prebuild was being
  ignored by the checker it feeds).
- `docs/integrations/mcp-clients.md` documents the per-client smoke in the
  catalog section and points the Connection Lifecycle section at it.

## Verification (all from this dispatch)

Environment: `PDFTRACT_MCP_BIN=/build/pdftract/debug/pdftract`
(built with `cargo build --locked --package pdftract-cli --bin pdftract
--features mcp`, exit 0).

| Command | Exit | Result |
|---|---|---|
| `python3 -m py_compile scripts/check-mcp-tool-catalog.py` | 0 | compiles |
| `python3 scripts/check-mcp-tool-catalog.py` (with `PDFTRACT_MCP_BIN`) | 0 | generic contract OK: 10 tools; 30 calls; 4/4 client configs pass lifecycle |
| `python3 scripts/check-mcp-tool-catalog.py` (cargo-run fallback) | 1 (expected pre-Makefile-fix) | exposed the per-spawn rebuild timeout; after the Makefile fix the target hands the checker the built binary |
| `make mcp-verify` | 0 | builds, resolves `PDFTRACT_MCP_BIN`, 4/4 client configs pass |
| `scripts/definition-of-done.sh --fast` | 0 | in clean HEAD extraction (see close reason) |

### Negative tests (checker must reject breakage)

All via direct module invocation against the real binary; every case
raised `CheckFailure` and left no orphan processes (`pgrep -af
'pdftract[ ]mcp|pdftract[ ]serve'` empty):

- args `["--stdio"]` (no subcommand) → "server closed stdout before a response"
- args `["serve", "--bind", "127.0.0.1:18099"]` (wrong transport) → read timeout, then bounded terminate
- args `[]` → rejected
- Continue YAML mutated (`- --stdio` → `- stdio`) → "must use args ['mcp', '--stdio']"
- Cursor JSON missing `--stdio` → "must use args ['mcp', '--stdio']"
- Note: `["mcp"]` alone is *accepted* by design — `--stdio` is the default
  transport (`crates/pdftract-cli/src/cli.rs:344`), and the snippet-shape
  validator pins the documented `["mcp", "--stdio"]` form anyway.

## Notes

- `parse_client_configs` replaces the old Continue exact-string comparison
  with structural parsing + the same `_assert_stdio_config` contract, so
  formatting changes no longer false-fail while semantic drift still does.
- Bead description had no acceptance-criteria section or plan references;
  the checker itself is the acceptance vehicle, run at HEAD in a clean
  extraction (see close reason).
