# pdftract-ab5ce9b5 — general -32602 invalid-params envelope carries `data.reason`

Bead: `pdftract-ab5ce9b5` (split-child of `pdftract-478d6e38`; blocker `pdftract-1932a457` closed)
Scope: `docs/integrations/mcp-clients.md` "Error Handling" bullet 2, general invalid-params
envelope only (boundary `data.code` rejections belong to `pdftract-6696g`).

## What was found at HEAD

`invalid_params_rejected_with_32602_data_reason`
(`crates/pdftract-cli/tests/mcp-client-lifecycle.rs`) already covered the envelope's shape:

- params-less `tools/call` → `-32602` pinned, `data.reason` asserted **string**, id echo
  asserted explicitly (`Some(2)`) plus per-request via `recv_response` correlation
- name-less `tools/call` → `-32602` pinned, `data.reason` asserted string **and non-empty**
- non-string name (`{"name": 42}`) → `-32602` pinned, `data.reason` asserted **string** only
- unknown tool → `-32601` pinned with explicit id echo (`Some(5)`)
- keep-serving (`tools/list` after rejections) and exit 0 on stdin EOF both asserted

Gap vs. acceptance criterion 1 ("a **non-empty** string `data.reason`" for *each* of the
three shapes): non-emptiness was pinned only on the name-less arm — `""` would have
passed the other two arms. The id echo on the name-less / non-string-name arms is
enforced by `recv_response`'s strict per-request id assertion, so no duplicate was added.

## What was landed

`crates/pdftract-cli/tests/mcp-client-lifecycle.rs`, one commit:

- New `assert_data_reason(what, response)` helper: extracts `error.data.reason` as a
  string (panics with the full response if absent/non-string) and asserts it is
  non-empty — the documented contract wording ("a `reason` string explaining the
  rejection").
- Applied to all three shapes, replacing the name-less arm's inline extraction; the
  three arms now assert the identical envelope contract.
- Test doc comment sharpened to "non-empty `data.reason` string".

Server side (unchanged, verified as the assertion target):
`crates/pdftract-cli/src/mcp/stdio.rs:295-307` produces `"Missing params"` and
"Missing or invalid 'name' field"` — both non-empty, id echoed via `Response::error(id, ..)`;
unknown names go through `ErrorObject::method_not_found` → `-32601`
(`crates/pdftract-cli/src/mcp/framing/mod.rs:291`).

## Verification

```
timeout --kill-after=30s 600s cargo test -p pdftract-cli --test mcp-client-lifecycle

running 4 tests
test documented_lifecycle_initialize_list_call_exit_on_eof ... ok
test invalid_params_rejected_with_32602_data_reason ... ok
test out_of_root_and_invalid_params_rejected_with_32602 ... ok
test parse_errors_get_error_responses_and_server_continues ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Exit code 0 (timeout did not fire; wall 25s incl. warm incremental build).
rustfmt-clean (`rustfmt --emit stdout --edition 2021 | diff` — content identical).
TH-03 orphans: `pgrep -af 'pdftract[ ]mcp|TH[_]0|TH[-]0'` → no matches.

## Acceptance criteria

- PASS — three invalid-params shapes each assert `-32602` + non-empty string
  `data.reason` + echoed request id (explicit on params-less; via `recv_response`
  strict correlation on all).
- PASS — unknown tool → `-32601`; keep-serving after all rejections; exit 0 on
  stdin EOF (pre-existing assertions re-derived at HEAD, all green).
- PASS — suite green under the mandated timeout; no orphaned processes; this note;
  commit cites `pdftract-ab5ce9b5`.

Wire-contract only: no assertion depends on extraction success (the parser defect is
tracked separately).
