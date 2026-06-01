# Verification Note: pdftract-46tdo (troubleshooting.md)

## Acceptance Criteria Summary

### PASS
- [x] `docs/user-docs/src/troubleshooting.md` exists
- [x] Covers at least 15 diagnostic codes most likely to be user-visible (covers 20+ codes)
- [x] Top-level TOC for navigation (symptom → diagnostic lookup table)
- [x] Cross-links to Diagnostic Code Catalog (troubleshooting/diagnostics.md reference)
- [x] mdBook renders cleanly (verified with `mdbook build`)

## Diagnostic Codes Covered

The troubleshooting.md covers the following user-visible diagnostic codes:

1. XREF_REPAIRED - Cross-reference table corruption
2. STREAM_BOMB - Decompression bomb
3. ENCRYPTION_UNSUPPORTED - PDF encryption errors
4. OCR_JBIG2_UNSUPPORTED - JBIG2 decoder missing
5. OCR_JPX_UNSUPPORTED - JPEG2000 decoder missing
6. OCR_CCITT_UNSUPPORTED - CCITT fax decoder missing
7. BROKENVECTOR_OCR_UNAVAILABLE - OCR unavailable for broken vectors
8. MCP_PATH_TRAVERSAL - Path traversal in MCP mode
9. PATH_OUTSIDE_ROOT - Path escapes root directory
10. URL_PRIVATE_NETWORK - SSRF protection for private networks
11. CACHE_ENTRY_CORRUPT - Cache integrity failure
12. CACHE_INTEGRITY_FAIL - Cache HMAC verification failed
13. PROFILE_INVALID - Profile YAML validation failed
14. PROFILE_SECRETS_FORBIDDEN - Forbidden secret keys in profile
15. PAGE_OUT_OF_RANGE - Page number exceeds document count
16. GLYPH_UNMAPPED - Font glyph encoding failure
17. JAVASCRIPT_PRESENT - JavaScript in PDF (never executed)
18. STRUCT_CIRCULAR_REF - Circular reference detected
19. STRUCT_XOBJECT_CYCLE - XObject reference cycle
20. GSTATE_STACK_OVERFLOW - Graphics state stack overflow
21. REMOTE_FETCH_INTERRUPTED - HTTP fetch interrupted
22. REMOTE_NO_RANGE_SUPPORT - Server lacks Range support
23. TAGGED_PDF_STRUCT_TREE_DEFERRED - Structure tree extraction not implemented

## Files Modified

- `docs/user-docs/src/troubleshooting.md` - Already comprehensive; verified content
- `docs/user-docs/src/troubleshooting/diagnostics.md` - Created diagnostics reference page

## Build Verification

```bash
cd /home/coding/pdftract/docs/user-docs
mdbook build
# INFO: Book building has started
# INFO: Running the html backend
# INFO: HTML book written to `/home/coding/pdftract/docs/user-docs/build/user-docs`
# Build successful
```

## Documentation Structure

- Top-level symptom → diagnostic lookup table for quick navigation
- One section per diagnostic code with:
  - "What it means" description
  - Root cause explanation
  - Actionable fix (commands, config changes, or escalation)
  - Severity level indicator
- Cross-links to Diagnostic Code Catalog
- "Getting Help" section with escalation paths
- "Related Documentation" section for further reading

## User-Targeted Language

The page uses jargon-light language aimed at end users:
- Explains technical terms (xref, SSRF, etc.) in plain language
- Provides concrete command examples for fixes
- Indicates severity levels clearly (info/warn/error/fatal)
- Separates "no action needed" cases from actionable fixes

## References

- Plan section: DOC epic + Edge Case Catalog
- Phase 6.1.5: Diagnostic Code Catalog (`docs/integrations/diagnostics-codes.md`)
- Coordinator: pdftract-53no (parent)
