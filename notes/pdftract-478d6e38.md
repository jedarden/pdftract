# pdftract-478d6e38 — MCP error-handling wire contract: tests + doc reconciliation

**Type:** task (P2, weave-generated)
**Date:** 2026-09-15

## What was found at dispatch

The bead was re-dispatched with `failure-count:1`; the prior attempt resolved
`indeterminate` (worker died mid-run — no close, no fail). Its working-tree
artifacts were **stranded but complete**: the doc reconciliation and both new
tests were already written. Per the established treadmill policy, I re-derived
at HEAD, verified the artifacts against the actual server behavior, closed one
honesty gap (below), and landed rather than re-split.

## Work items

### (1) Parse-error resilience — `parse_errors_get_error_responses_and_server_continues`

`crates/pdftract-cli/tests/mcp-client-lifecycle.rs` (new test + two harness
helpers):

- `send_raw()` writes an arbitrary pre-serialized body with **correct LSP
  framing**, so the transport stays in sync and the failure is contained to
  JSON-RPC parsing (the documented "parse errors" scenario).
- `recv_any()` bounds the next response read without id correlation (parse
  errors answer `id: null`, so the strict id check cannot apply).
- Frames exercised: truncated JSON (pinned exactly: `-32700`, `id: null`, no
  `result` key), bare JSON string, empty batch array, wrong `jsonrpc` version,
  well-formed batch. The latter four assert the error envelope + spec-reserved
  code range (-32700..=-32000) without pinning which (the doc pins no code for
  them; the server answers all `-32700`).
- After the abuse: full `tools/list` round-trip returns the 10-tool catalog, a
  second abuse → recovery cycle proves it is not a one-frame fluke, and stdin
  EOF still exits 0.

Server side verified in `crates/pdftract-cli/src/mcp/stdio.rs`: `read_message`
fails on unparseable JSON / batch / wrong-version (the `Request` deserializer
validates `jsonrpc` == "2.0"), and the main loop converts every read failure to
`Response::error(Id::Null, ErrorObject::parse_error())` and **continues**
(`stdio.rs:474-480`).

### (2) `-32602` + `data.reason` envelope — `invalid_params_rejected_with_32602_data_reason`

- `tools/call` with **no params** → `-32602`, `data.reason = "Missing params"`
  (`stdio.rs:296`), request id still echoed.
- `tools/call` with arguments but **no name** → `-32602`,
  `data.reason = "Missing or invalid 'name' field"` (`stdio.rs:307`), reason
  asserted non-empty.
- `tools/call` with a **non-string name** (`42`) → same `-32602` + `data.reason`
  path.
- **Unknown tool** (added this attempt): `tools/call` naming `no_such_tool` →
  `-32601` with the request id echoed — closing the gap below.
- Server keeps serving (`tools/list` succeeds) and still exits 0 on EOF.

### (3) Doc reconciliation — `docs/integrations/mcp-clients.md`

- Claude Desktop validation step no longer says "tools prefixed with
  `pdftract.`": now lists the real unprefixed catalog (`extract`,
  `extract_text`, `extract_markdown`, `search`, `get_metadata`, `hash`,
  `get_table`, `get_form_fields`, `get_attachments`, `classify`) — matches
  `CATALOGED_TOOLS` in the test and the registry's `fn name()`s
  (`tools/registry.rs:542-1093`).
- Python SDK example calls `extract_text`, not `pdftract_extract`.
- Error Handling bullets made precise: parse errors → `-32700` with `id: null`
  and the server continues; invalid params → `-32602` whose `data` carries a
  `reason` string; unknown tool → `-32601`; plus a pointer to this test file.
- Troubleshooting stdio snippet now uses `Content-Length` framing (a bare JSON
  line is silently consumed as a header).

Remaining `pdftract.` occurrences elsewhere (`docs/user-docs/src/sdk/python.md`)
are the Python SDK's module namespace, a different surface — correct as-is.

## Gap closed this attempt

The stranded edit added a doc bullet claiming the Error Handling behaviors are
"asserted end-to-end by the `mcp-client-lifecycle` integration test", but the
`-32601` unknown-tool bullet had no test. Added the assertion (id 5 of the
invalid-params test) so the doc sentence is exactly true.

## Verification

```
timeout --kill-after=30s 600s cargo test --test mcp-client-lifecycle
  running 4 tests
  test documented_lifecycle_initialize_list_call_exit_on_eof ... ok
  test invalid_params_rejected_with_32602_data_reason ... ok
  test parse_errors_get_error_responses_and_server_continues ... ok
  test out_of_root_and_invalid_params_rejected_with_32602 ... ok
  test result: ok. 4 passed; 0 failed
```

Run twice (second run identical) — no flake. `cargo fmt -p pdftract-cli
--check`: no findings in the touched file. Orphan check after runs
(`pgrep -af 'pdftract[ ]mcp|TH[_]0|TH[-]0'`): empty. `.github/workflows/`:
absent, nothing to remove. `src/` untouched — test + doc only.

## Acceptance criteria

- Tests assert the documented wire contract — **PASS** (parse-error resilience
  and the `-32602`/`data.reason` envelope are asserted over a real stdio
  subprocess; unknown-tool `-32601` added).
- `docs/integrations/mcp-clients.md` updated where it diverges — **PASS**
  (validation text, SDK example, Error Handling bullets, framing snippet).
