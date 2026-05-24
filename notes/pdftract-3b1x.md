# pdftract-3b1x: SDK invocation note final-pass

**Bead:** pdftract-3b1x
**Title:** Note: docs/notes/sdk-invocation.md final-pass alignment with subprocess contract
**Date:** 2026-05-24

## Summary

Updated `docs/notes/sdk-invocation.md` to v1.0 final-pass, documenting the subprocess invocation contract that every language SDK follows.

## Changes Made

### Added Subprocess Contract Section (lines 14-248)

A comprehensive new section at the top of the document (before language examples) covering:

1. **argv layout** - Canonical form an SDK should construct, with rules for multi-value flags, PDF path positioning, and special `-` stdin path
2. **stdin discipline** - Two purposes: password ingress via `--password-stdin` and PDF bytes from stdin (`-` path). Documented TH-07 restriction on `--password VALUE`
3. **stdout discipline** - Extraction output is the ONLY thing on stdout in `--json`/`--text` mode. INV-9 reference for MCP stdio mode
4. **stderr discipline** - Log levels (error/warn/info/debug/trace), what's logged vs never logged (passwords, tokens, PDF bytes)
5. **Exit code taxonomy** - Full table with codes 0, 64-78, including TH-03 (exit 78 for config errors) and TH-07 (exit 64 for password policy violations)
6. **Environment variable pass-through** - All recognized env vars: `PDFTRACT_PASSWORD`, `PDFTRACT_MCP_TOKEN`, `PDFTRACT_INSECURE_CLI_PASSWORD`, `PDFTRACT_INSECURE_CLI_TOKEN`, `RUST_LOG`, `NO_COLOR`, `XDG_CONFIG_HOME`, `PDFTRACT_CONFIG_DIR`
7. **`--progress-json` event schema** - ndjson format with event types: `open`, `page_started`, `page_completed`, `ocr_started`, `ocr_completed`, `profile_matched`, `password_received`, `completed`, `error`
8. **`--capture-diagnostics` archive layout** - zip/tar format, contained files (`manifest.json`, `runtime_config.json`, `stderr.log`, `pdf_fingerprint.txt`, `pdf_source_sanitized.pdf`, `version.txt`), secret scrubbing rules

### Updated Language Examples with TH-07 Compliance

All language examples now demonstrate TH-07-compliant password handling:

- **Python** (lines 270-408): Added `extract_pdf_password_stdin()` and `extract_pdf_from_bytes()` functions. Updated HTTP example to send password as form field.
- **Node.js** (lines 470-595): Added `extractPdfPasswordStdin()` function using stdin. Updated HTTP example with password form field.
- **Go** (lines 643-747): Updated subprocess example to pass password via `PDFTRACT_PASSWORD` env var. Updated HTTP example with password form field.
- **Ruby** (lines 820-950): Added `extract_pdf_password_stdin()` method. Updated HTTP example with password form field.
- **Java** (lines 988-1190): Updated subprocess example to pass password via `PDFTRACT_PASSWORD` env var. Updated HTTP example with password form field.
- **Rust** (lines 1238-1440): Updated subprocess example to pass password via env var. Updated HTTP example with password form field.

### Added Progress JSON Parsing Examples (lines 1442-1675)

Three complete examples (Python, Node.js, Rust) showing how to parse `--progress-json` events from stderr while extraction is running. Each example demonstrates:
- Line-by-line stderr parsing
- JSON parse fallback for human log lines
- Event type handling (open, page_started, page_completed, ocr_started/finished, profile_matched, password_received, completed, error)
- TH-07 note that `password_received` event never includes the password value

## Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| Secrets-handling (TH-07) corrections | PASS | All examples updated to use env/stdin, not `--password VALUE` |
| argv/stdin/stdout/stderr discipline sections | PASS | Comprehensive "Subprocess Contract" section added |
| Exit code taxonomy with TH-NN references | PASS | Full table with TH-03 (exit 78) and TH-07 (exit 64) references |
| --progress-json event schema | PASS | All event types documented with JSON examples |
| --capture-diagnostics archive layout | PASS | File layout, JSON schemas, and scrubbing rules documented |
| Rust, Python, Node examples verified | PASS | All three languages have complete subprocess and HTTP examples |

## File Statistics

- **Before:** 1100 lines
- **After:** 1837 lines (+737 lines, ~67% growth)
- **Location:** `/home/coding/pdftract/docs/notes/sdk-invocation.md`

## Verification Notes

1. **Documentation compiles** - All Rust code in examples is syntactically correct
2. **TH-07 compliance** - Every password-handling example uses env var or stdin, never `--password VALUE` flag
3. **TH-03 reference** - Exit code 78 for config errors (MCP bind without auth-token) is documented
4. **Progress JSON examples** - Real-world parsing code in Python, Node.js, and Rust
5. **Secret scrubbing** - `--capture-diagnostics` section explicitly states what gets redacted (passwords, tokens, full text)

## Related Plan References

- Plan line 833: per-threat tests
- Plan line 874: TH-03 exit 78 (MCP bind without auth-token)
- Plan line 878: TH-07 password CLI policy
- Plan line 907: `--password-stdin` documentation
- Plan lines 911-913: password redaction in progress-json
- Plan line 921: token in SecretString

## Commits

- `docs(pdftract-3b1x): finalize sdk-invocation.md with subprocess contract and TH-07 compliance`

## Next Steps

None. This documentation task is complete and unblocks downstream SDK implementations.
