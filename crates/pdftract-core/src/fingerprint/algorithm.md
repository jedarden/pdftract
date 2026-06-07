# PDF Structural Fingerprint Algorithm v1

## Overview

The PDF structural fingerprint is a reproducible 256-bit content hash that identifies the **semantic** content of a PDF independent of metadata churn, byte ordering, and producer-tool re-saves.

## Algorithm Version

**Version:** `pdftract-v1`

**Version Prefix:** All fingerprints emitted by this implementation are prefixed with `pdftract-v1:` to ensure algorithm changes cannot silently produce mismatches against historical fingerprints (INV-13).

## Merkle-Style Hash Inputs

The fingerprint is computed as SHA-256 over the following inputs in **deterministic order**:

### 1. Page Count (4 bytes)

- Format: `u32` in big-endian byte order
- Represents: Number of pages in the document

### 2. Per-Page Contributions

For each page in **page_index order** (0 to n-1):

#### 2a. Content Streams (32 bytes per page)

- Hash: SHA-256 of concatenated, **decoded** content streams
- Normalization: Content streams are tokenized and re-emitted with single 0x20 separators between tokens
- Order: Streams are concatenated in the order they appear in the page's `/Contents` array
- Comments: Dropped (not included in hash)

#### 2b. Resource Dictionary (32 bytes per page)

- Hash: SHA-256 of the resolved resource dictionary
- Namespaces: `/Font`, `/XObject`, `/ExtGState`, `/ColorSpace`, `/Pattern`, `/Shading`, `/Properties`
- Ordering: Keys within each namespace are sorted lexicographically
- Encoding: JSON-equivalent canonical serialization

#### 2c. Page Geometry (36 bytes per page)

- **MediaBox**: 4 coordinates × 8 bytes each = 32 bytes
- **CropBox** (if present): 4 coordinates × 8 bytes each = 32 bytes
- **Rotate**: 4 bytes in big-endian i32

All geometry values are **canonicalized** to 4-decimal-place fixed-point integers:
- Formula: `(x * 10000).round_ties_even() as i64` (banker's rounding)
- Encoding: 8-byte big-endian i64 per coordinate
- NaN/Inf: Canonicalized to 0 with diagnostic emitted

### 3. Structure Tree (32 bytes)

- If the document is tagged PDF (`/StructTreeRoot` present):
  - SHA-256 of the structure tree serialized as canonical JSON
  - Keys: `/S`, `/Lang`, `/Alt`, `/ActualText`
  - Recursive walk of `/K` array
- If not tagged:
  - All-zero hash: `[0u8; 32]`

### 4. Catalog Feature Flags (1 byte)

Single byte encoding the following boolean flags:

| Bit | Flag | Description |
|-----|------|-------------|
| 0 | `is_encrypted` | Document has `/Encrypt` dictionary |
| 1 | `contains_javascript` | Document contains JavaScript actions |
| 2 | `contains_xfa` | Document has XFA forms |
| 3 | `ocg_present` | Document has Optional Content Groups |

Encoding: `is_encrypted | (contains_javascript << 1) | (contains_xfa << 2) | (ocg_present << 3)`

## Deliberately Excluded Inputs

Per ADR-008, the following are **explicitly excluded** from the fingerprint:

### Metadata (not content)
- `/Producer`
- `/Creator`
- `/CreationDate`
- `/ModDate`
- `/Author`
- `/Title`
- `/Subject`
- `/Keywords`

### Identifier that varies per save
- `/ID` array (changes even for byte-identical content)

### XMP metadata
- `/Metadata` stream (orthogonal to semantic content)

### Byte layout
- xref byte layout
- Object number assignment
- Inline whitespace in content streams (lexer-normalized before hashing)

## Output Format

**Format:** `pdftract-v1:` + lowercase hex SHA-256

**Example:** `pdftract-v1:a7f3c8d9e4b2a1f6c5d4e3b2a1098765432109abcdefabcdefabcdefabcdefabcd`

**Length:** 13 characters (prefix) + 64 characters (hex) = 77 characters total

**Regex:** `^pdftract-v1:[0-9a-f]{64}$` (INV-13)

## Invariants

### INV-3: Byte-Stable Across Runs

100 calls on the same PDF produce **identical** fingerprint output.

**Test:** `test_inv3_reproducibility_100_invocations`

### INV-8: No Panics

No input, including invalid data, causes a panic. NaN/Inf values are canonicalized to 0 with diagnostics emitted.

### INV-13: Version Prefix

Every fingerprint output matches the regex `^pdftract-v1:[0-9a-f]{64}$`.

**Test:** `test_inv13_fingerprint_format`

## Critical Tests

Per Phase 1.7 acceptance criteria:

1. **Acrobat + pdftk same:** Re-saved by Acrobat and pdftk → identical fingerprint
2. **CreationDate-only same:** Only `/CreationDate` changed → identical fingerprint
3. **Glyph-removed differ:** One glyph removed → different fingerprint
4. **10-invocation identical:** Same file, 10 runs → identical each time
5. **Linearized vs non-linearized same:** Linearized and non-linearized versions → identical fingerprint (KU-7)

## Performance

**Target:** < 100 ms for 100-page PDF

**Test:** `test_performance_100_page_pdf`

## Implementation Location

- **Core algorithm:** `crates/pdftract-core/src/fingerprint/mod.rs`
- **Canonicalization:** `crates/pdftract-core/src/fingerprint/canonicalize.rs`
- **CLI command:** `pdftract hash FILE.pdf`
- **Tests:** `crates/pdftract-core/tests/fingerprint_reproducibility.rs`

## References

- Plan section: Phase 1.7 PDF Structural Fingerprint (lines 1182-1219)
- ADR-008 (fingerprint excludes metadata)
- INV-3, INV-13
- KU-7 (linearization toggle test)
