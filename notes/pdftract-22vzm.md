# Phase 1.7: PDF Structural Fingerprint — Coordinator Close Summary

## Overview
Phase 1.7 (PDF Structural Fingerprint) is complete. All 4 child beads have been closed, implementing a reproducible 256-bit content hash that identifies the semantic content of a PDF independent of metadata churn, byte ordering, and producer-tool re-saves.

## Child Beads Closed

1. **pdftract-q15sh** — Fingerprint algorithm implementation (Merkle SHA-256 over canonicalized inputs)
   - Location: `crates/pdftract-core/src/fingerprint/mod.rs`
   - See: `notes/pdftract-q15sh.md`

2. **pdftract-154mz** — Per-page input canonicalization (geometry rounding, content stream normalization)
   - Location: `crates/pdftract-core/src/fingerprint/canonicalize.rs`
   - See: `notes/pdftract-154mz.md`

3. **pdftract-3954u** — `pdftract hash` CLI subcommand with exit codes 0/2/3/4/5/6
   - Location: `crates/pdftract-cli/src/main.rs`
   - See: `notes/pdftract-3954u.md`

4. **pdftract-ef6xz** — Fingerprint reproducibility test corpus
   - Location: `tests/fingerprint/fixtures/`
   - See: `notes/pdftract-ef6xz.md`

## Acceptance Criteria Verification

### ✅ All 4 child beads closed
All child beads show Status: closed in `bf show`.

### ✅ Algorithm v1 stable and documented
Documentation exists at `crates/pdftract-core/src/fingerprint/algorithm.md` (5.1 KB).
Algorithm version prefix is `pdftract-v1:` (INV-13 compliant).

### ✅ All 5 Critical tests pass (73 total fingerprint tests)
Test run on 2025-06-03:
```
cargo nextest run -j 4 --package pdftract-core fingerprint
Summary: 73 tests run: 73 passed, 3083 skipped
```

Critical tests verified:
- `test_fixture_acrobat_resave` — Acrobat re-saves produce identical fingerprint ✅
- `test_fixture_pdftk_resave` — pdftk re-saves produce identical fingerprint ✅
- `test_fixture_metadata_only` — CreationDate-only changes preserve fingerprint ✅
- `test_fixture_content_edit_one_glyph` — Content edits change fingerprint ✅
- `test_fixture_linearization_toggle` — Linearization toggle preserves fingerprint (KU-7) ✅

### ✅ INV-3 (byte-stable across runs)
Test: `test_inv3_reproducibility_100_invocations`
Verifies 100 invocations on the same PDF produce identical fingerprint output.
Result: PASS (0.011s runtime)

### ✅ INV-13 (version prefix on every emission)
Tests: `test_inv13_fingerprint_format`, `test_inv13_multiple_outputs_match_format`
Verifies output matches regex `^pdftract-v1:[0-9a-f]{64}$`
Result: PASS

### ✅ CLI command functional
```bash
$ ./target/release/pdftract hash tests/fingerprint/fixtures/acrobat_resave/v1.pdf
pdftract-v1:ab24a95f44ceca5d2aed4b6d056adddd8539f44c6cd6ca506534e830c82ea8a8
```

## Implementation Summary

### Algorithm (Merkle-style SHA-256)
Inputs in deterministic order:
1. Page count (u32, big-endian)
2. Per page (in page_index order):
   a. SHA-256 of decoded, token-normalized content streams
   b. SHA-256 of resolved resource dict (sorted by namespace/key)
   c. Page geometry: MediaBox, CropBox, Rotate (4-decimal fixed-point)
3. Structure tree SHA-256 (if tagged; all-zero hash otherwise)
4. Catalog feature flag byte (encrypted, javascript, XFA, OCG bits)

Deliberately excluded (per ADR-008):
- `/Producer`, `/Creator`, `/CreationDate`, `/ModDate`, `/Author`, `/Title`, `/Subject`, `/Keywords`
- `/ID` array, XMP `/Metadata` stream
- xref byte layout, object number assignment
- Inline whitespace (lexer-normalized before hashing)

### CLI Interface
- Command: `pdftract hash <INPUT>` (file path or URL)
- Output: `pdftract-v1:<64-hex-chars>\n`
- Exit codes: 0 success, 2 corrupt, 3 encrypted-no-password, 4 cannot read, 5 network failure, 6 TLS handshake failure

## Downstream Dependencies

This fingerprint is now available for:
- **Phase 6.8 receipts** — binding identity for receipt verification
- **Phase 6.9 cache** — content-addressed cache key for extraction results
- **Users** — identify PDFs across re-saves via `pdftract hash`

## Related Plan References
- Plan: Phase 1.7 PDF Structural Fingerprint (lines 1182-1219)
- ADR-008 (fingerprint excludes metadata)
- INV-3, INV-13 (byte-stability, version prefix)
- KU-7 (linearization toggle test)

## Git Commits
This coordinator bead did not introduce new code commits; its child beads each produced their own commits.
See child bead verification notes for individual commit details.
