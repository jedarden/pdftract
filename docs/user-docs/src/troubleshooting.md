# Troubleshooting

This guide maps common pdftract failures to their causes and fixes. Each error is associated with a **diagnostic code** that appears in extraction output (see `diagnostics` in the JSON response or CLI stderr).

> **For the authoritative diagnostic code catalog**, see the [Diagnostics Reference](./troubleshooting/diagnostics.md).

## Symptom → Diagnostic Lookup

| Symptom | Likely Diagnostic Code |
|---------|----------------------|
| PDF won't open, "encrypted" error | `ENCRYPTION_UNSUPPORTED` |
| Text extraction incomplete or missing | `XREF_REPAIRED`, `OCR_*_UNSUPPORTED` |
| Process hangs or runs very long | `STREAM_BOMB` |
| "Path outside root" (MCP mode) | `MCP_PATH_TRAVERSAL` |
| Cache errors / corrupted entries | `CACHE_ENTRY_CORRUPT`, `CACHE_INTEGRITY_FAIL` |
| Profile fails to load | `PROFILE_INVALID`, `PROFILE_SECRETS_FORBIDDEN` |
| Remote URL fetch blocked | `URL_PRIVATE_NETWORK` |
| Requested page doesn't exist | `PAGE_OUT_OF_RANGE` |
| Text contains placeholder characters (⍰) | `GLYPH_UNMAPPED` |
| Broken vector graphics not recovered | `BROKENVECTOR_OCR_UNAVAILABLE` |
| JavaScript warning in output | `JAVASCRIPT_PRESENT` |
| Circular reference warnings | `STRUCT_CIRCULAR_REF`, `STRUCT_XOBJECT_CYCLE` |
| Stack overflow warnings | `GSTATE_STACK_OVERFLOW` |

---

## XREF_REPAIRED warning

**What it means**: pdftract found the PDF's cross-reference table was corrupt and ran the forward-scan fallback (Phase 1.3) to recover.

**Cause**: PDF created or transmitted with truncation or corruption. The `startxref` offset points outside the file, or the xref table is malformed.

**Fix**: Usually no action needed; extraction succeeds with the recovered xref. Output may be incomplete on truncated files. If extraction fails, the PDF is unsalvageable.

**Severity**: info (extraction continues)

---

## STREAM_BOMB error

**What it means**: A compressed stream exceeded the decompression size limit (default: 512 MB).

**Cause**: A hostile PDF with a "compression bomb" — a small stream that expands to multi-GB size (e.g., 10 KB → 2 GB). This is a common security exploit pattern.

**Fix**: 
- If the PDF is **trusted**: Increase the limit with `--max-decompress-gb 2` (or higher)
- If the PDF is **untrusted**: Treat as a hostile file; do not process

**Severity**: error (stream aborted; partial extraction returned)

---

## ENCRYPTION_UNSUPPORTED fatal

**What it means**: The PDF is encrypted with an unsupported handler or the wrong password.

**Cause**: 
- PDF encrypted with an unknown handler (e.g., Adobe LiveCycle policy server)
- PDF password-protected but no password (or wrong password) supplied

**Fix**:
```bash
# Supply password via environment variable
export PDFTRACT_PASSWORD="your-password"
pdftract extract document.pdf

# Or via stdin
echo "your-password" | pdftract extract --password-stdin document.pdf
```

If the handler is unsupported (e.g., Adobe LiveCycle), use an Adobe-side decryption tool first, or a dedicated password recovery tool like `pdfcrack` or `john`.

**Severity**: fatal (process exits with code 3)

---

## OCR_JBIG2_UNSUPPORTED / OCR_JPX_UNSUPPORTED / OCR_CCITT_UNSUPPORTED warning

**What it means**: A page contains an image that requires a decoder not available in the current build.

**Cause**:
- `OCR_JBIG2_UNSUPPORTED`: JBIG2-encoded image (rare)
- `OCR_JPX_UNSUPPORTED`: JPEG 2000-encoded image
- `OCR_CCITT_UNSUPPORTED`: CCITT fax-encoded image

**Fix**:
```bash
# Build with full-render feature (enables all decoders via PDFium)
cargo build --release --features full-render

# Or install system libraries:
# - JPX: install libopenjp2
# - CCITT: install libtiff
```

**Severity**: warn (page skipped from OCR; extraction continues)

---

## BROKENVECTOR_OCR_UNAVAILABLE warning

**What it means**: A page contains broken vector graphics that could be recovered via OCR, but the OCR feature is disabled.

**Cause**: Build was compiled without the `ocr` feature.

**Fix**: Rebuild with OCR enabled:
```bash
cargo build --release --features ocr
```

**Severity**: warn (broken vector graphics not recovered; extraction continues)

---

## MCP_PATH_TRAVERSAL / PATH_OUTSIDE_ROOT error

**What it means**: (MCP mode) The requested path escapes the `--root` directory boundary.

**Cause**: A tool call attempted path traversal (e.g., `../../etc/passwd`).

**Fix**:
- Adjust the requested path to stay within `--root`
- Or restart the MCP server without `--root` restriction (not recommended for multi-tenant deployments)

**Severity**: error (request rejected)

---

## URL_PRIVATE_NETWORK error

**What it means**: Remote fetch blocked because the URL targets a private network address.

**Cause**: URL targets localhost, private IP ranges (RFC 1918), or link-local addresses. This is an SSRF (Server-Side Request Forgery) protection.

**Fix**:
```bash
# If you trust the URL, allow private networks:
pdftract extract --allow-private-networks https://internal-server/docs.pdf
```

**Severity**: error (request rejected with HTTP 400 in serve mode)

---

## CACHE_ENTRY_CORRUPT warning

**What it means**: A cache entry failed integrity verification.

**Cause**: Cache file corruption (disk error, concurrent write, etc.).

**Fix**: None needed — the entry is automatically deleted and extraction re-runs. If this recurs frequently, check your disk filesystem.

**Severity**: warn (entry deleted; extraction re-runs)

---

## CACHE_INTEGRITY_FAIL diagnostic

**What it means**: A cache entry's HMAC verification failed, indicating potential cache poisoning.

**Cause**: Malicious co-tenant wrote a forged cache entry (multi-user cache scenarios), or disk corruption.

**Fix**: The entry is treated as a cache miss and extraction re-runs. In multi-user environments, ensure per-user cache directories or verify cache permissions.

**Severity**: warn (entry rejected; extraction re-runs)

---

## PROFILE_INVALID / PROFILE_SECRETS_FORBIDDEN error

**What it means**: Profile YAML failed validation.

**Cause**:
- `PROFILE_INVALID`: YAML syntax error or schema violation
- `PROFILE_SECRETS_FORBIDDEN`: Profile contains secret-keyword keys (`password:`, `token:`, `secret:`, `api_key:`)

**Fix**:
```bash
# For schema errors, check the YAML syntax:
pdftract profile show --profile-path your-profile.yaml

# For secrets errors, remove secret keys from the profile.
# Secrets should be passed via environment variables, not profiles.
```

**Severity**: error (profile rejected)

---

## PAGE_OUT_OF_RANGE warning

**What it means**: The `--pages` argument exceeds the document's actual page count.

**Cause**: Page range specified (e.g., `--pages 1-100`) on a document with fewer pages (e.g., 10 pages).

**Fix**: Adjust the `--pages` argument to the actual page count:
```bash
# First, get the page count:
pdftract inspect document.json | jq '.page_count'

# Then extract with a valid range:
pdftract extract --pages 1-10 document.pdf
```

**Severity**: warn (pages clamped to available range)

---

## GLYPH_UNMAPPED warning

**What it means**: A glyph could not be resolved by any of the four encoding levels.

**Cause**: Font encoding corruption, missing font embedding, or non-standard encoding.

**Fix**: Output contains the Unicode replacement character (⍰). No direct fix; consider re-saving the PDF through a normalizing tool (e.g., Adobe Acrobat, qpdf).

**Severity**: warn (character replaced with U+FFFD; extraction continues)

---

## JAVASCRIPT_PRESENT info

**What it means**: PDF contains embedded JavaScript (in `/AA`, `/OpenAction`, or `/JS` entries).

**Cause**: PDF includes JavaScript actions (common in forms, interactive documents).

**Fix**: None needed for extraction — pdftract NEVER executes embedded JavaScript. JavaScript actions are surfaced in `metadata.javascript_actions[]` for downstream review.

**Severity**: info (JavaScript is not executed)

---

## STRUCT_CIRCULAR_REF / STRUCT_XOBJECT_CYCLE / GSTATE_STACK_OVERFLOW warning

**What it means**: PDF contains circular references or malformed content streams.

**Cause**:
- `STRUCT_CIRCULAR_REF`: Indirect object reference cycle
- `STRUCT_XOBJECT_CYCLE`: XObject (image/form) reference cycle
- `GSTATE_STACK_OVERFLOW`: Graphics state stack exceeds depth limit

**Fix**: Usually no action needed — pdftract breaks cycles at the second visit (or depth 20 for XObjects). If output is incomplete, investigate the source PDF for a producer bug.

**Severity**: warn (cycle broken; extraction continues)

---

## REMOTE_FETCH_INTERRUPTED error

**What it means**: Remote fetch was interrupted (network timeout, connection reset, etc.).

**Cause**: Network connectivity issues, server timeout, or premature connection close.

**Fix**: Retry the request; check network connectivity:
```bash
# Retry with increased timeout:
pdftract extract --timeout-seconds 120 https://example.com/document.pdf
```

**Severity**: error (request aborted)

---

## REMOTE_NO_RANGE_SUPPORT warning

**What it means**: Remote server does not support HTTP Range requests.

**Cause**: Server lacks `Accept-Ranges` header or returns 206 Unsupported.

**Fix**: None needed — pdftract falls back to whole-file download. For large files, consider hosting on a Range-supporting server.

**Severity**: warn (fallback to whole-file download)

---

## TAGGED_PDF_STRUCT_TREE_DEFERRED info

**What it means**: Tagged PDF structure tree extraction is deferred in this version.

**Cause**: Phase 7.1 (full structure tree extraction) is not yet implemented.

**Fix**: None needed — this is a temporary fallback. Structure tree extraction will be added in v1.0.0.

**Severity**: info (structure tree not extracted)

---

## Getting Help

If you encounter a diagnostic code not listed here, or the suggested fix doesn't resolve your issue:

1. **Check the [Diagnostics Reference](./troubleshooting/diagnostics.md)** for the full catalog
2. **Search existing issues** on [GitHub](https://github.com/jedarden/pdftract/issues)
3. **Open a new issue** with:
   - The diagnostic code(s)
   - A minimal reproducible example (PDF or command)
   - The `--debug` output if safe to share

## Related Documentation

- [Diagnostics Reference](./troubleshooting/diagnostics.md) — Full diagnostic code catalog
- [FAQ](./faq.md) — Common questions and answers
- [Advanced: OCR Configuration](./advanced/ocr.md) — OCR troubleshooting details
