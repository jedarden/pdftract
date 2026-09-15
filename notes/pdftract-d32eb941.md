# pdftract-d32eb941 — Reconcile mcp-clients.md tool-catalog and Error Handling text with the live tools/list output

Parent: pdftract-478d6e38 (split-child). Siblings: pdftract-1932a457 (parse-error
resilience, closed), pdftract-ab5ce9b5 (−32602 data.reason envelope, closed).

## What was checked

### 1. Tool catalog vs live `tools/list` output

Live round-trip using the doc's own recipe (`docs/integrations/mcp-clients.md`,
"Tools not appearing" section): a `Content-Length`-framed `tools/list` piped to
`pdftract mcp --stdio` (binary `/build/target-workers/debug/pdftract`, built
2026-09-15 10:52, newer than the last change to
`crates/pdftract-cli/src/mcp/tools/` at 338441d4d, 2026-09-13):

```
headers: Content-Length: 7321
id: 1 | has error: False
count: 10
names: ["classify","extract","extract_markdown","extract_text","get_attachments",
        "get_form_fields","get_metadata","get_table","hash","search"]
tools missing description/inputSchema: none
```

Cross-checked against two more independent sources:

- Harness constant `CATALOGED_TOOLS`
  (`crates/pdftract-cli/tests/mcp-client-lifecycle.rs:46`), asserted exactly by
  `documented_lifecycle_initialize_list_call_exit_on_eof`.
- Server registration order in `ToolRegistry::register_all`
  (`crates/pdftract-cli/src/mcp/tools/registry.rs:73`) and each tool's
  `fn name()` — the same 10 unprefixed strings.

**Result: doc line 58 already matches the live output exactly** — the catalog
`extract, extract_text, extract_markdown, search, get_metadata, hash,
get_table, get_form_fields, get_attachments, classify` is correct and carries
no `pdftract.`/`pdftract_extract` prefix. A grep of the whole doc for prefixed
names found none (the `pdftract.extract`-style hits elsewhere under `docs/`
are Python SDK API calls and were left untouched, per the bead's scope).
No doc change was needed for the tool catalog.

### 2. Error Handling bullets vs the sibling-asserted wire contract

Read the assertions at HEAD in `crates/pdftract-cli/tests/mcp-client-lifecycle.rs`:

- `parse_errors_get_error_responses_and_server_continues` — truncated JSON
  pinned to `-32700` with `id: null` and no `result`; bare string, empty batch
  array, wrong `jsonrpc` version, and a well-formed batch each answered with an
  error envelope whose code sits in the spec-reserved `-32700..=-32000` range;
  valid `tools/list` after proves the server keeps serving.
- `invalid_params_rejected_with_32602_data_reason` — `-32602` with non-empty
  `data.reason` on three shapes (params-less, name-less, non-string name),
  request id echoed; unknown tool → `-32601` with id echoed; server keeps
  serving; exit 0 on EOF.
- `out_of_root_and_invalid_params_rejected_with_32602` — `--root` boundary
  rejections carry `data.code` (`PATH_ESCAPES_ROOT`,
  `ABSOLUTE_PATH_NOT_PERMITTED`), not `reason`.

**Divergence found and fixed** in the doc's Error Handling section:

- The old "**Stdio corruption:** Any non-JSON-RPC on stdout breaks the
  connection; restart subprocess" bullet contradicted the proven resilience
  behavior. Replaced with a **Malformed requests** bullet (error envelope,
  `id: null`, code in the spec-reserved `-32700`–`-32000` range, frame stream
  stays in sync) and a **Resilience** bullet (errors echo the request id, the
  server keeps serving, clean exit on stdin EOF; restart only for a genuinely
  broken pipe or dead subprocess).
- **Parse errors** bullet tightened: `-32700` is pinned for unparseable JSON,
  and the response carries no `result`.
- **Invalid params** bullet: `reason` is non-empty, and `--root` boundary
  rejections instead carry `data.code` — both asserted by the tests above.
- **Unknown tool** bullet was already correct; unchanged.

## Test evidence

```
timeout --kill-after=30s 600s cargo test --test mcp-client-lifecycle --no-fail-fast

running 4 tests
test invalid_params_rejected_with_32602_data_reason ... ok
test documented_lifecycle_initialize_list_call_exit_on_eof ... ok
test out_of_root_and_invalid_params_rejected_with_32602 ... ok
test parse_errors_get_error_responses_and_server_continues ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Acceptance criteria

- **PASS** — no tool-name claim in `docs/integrations/mcp-clients.md`
  contradicts the live `tools/list` output (verified by round-trip, not by the
  doc agreeing with itself); no prefixed names.
- **PASS** — Error Handling bullets match the behavior asserted by the two
  sibling children (pdftract-1932a457, pdftract-ab5ce9b5), all four lifecycle
  tests green at HEAD.
- **PASS** — this note cites the doc diff and the catalog evidence; the commit
  cites `pdftract-d32eb941`.
