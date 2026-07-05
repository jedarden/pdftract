# PDF 2.0 Feature Coverage

**Risk R10:** PDF 2.0 features (PAdES-LTV signatures, AES-256 enhancements, `/Encryption V5`) not covered

**Proof Obligation PB-10:** Support PDF 2.0 features incrementally; ship an explicit compatibility matrix in this document.

## Overview

PDF 2.0 (ISO 32000-2:2017) introduces numerous changes and enhancements over PDF 1.7. This document tracks pdftract's support for PDF 2.0 features, what's implemented, what's deferred, and the compatibility matrix.

## PDF Version Support Matrix

| PDF Version | ISO Specification | Read Support | Write Support | Notes |
|-------------|------------------|--------------|----------------|-------|
| PDF 1.0 | (1993) | ✅ Full | ❌ None | Obsolete; rare in wild |
| PDF 1.1 | (1994) | ✅ Full | ❌ None | Obsolete; rare in wild |
| PDF 1.2 | (1996) | ✅ Full | ❌ None | Obsolete; rare in wild |
| PDF 1.3 | (1999) | ✅ Full | ❌ None | Common; JPEG 2000, EC-MA流 |
| PDF 1.4 | (2001) | ✅ Full | ❌ None | **Target baseline**; transparency, metadata |
| PDF 1.5 | (2003) | ✅ Full | ❌ None | Object streams, cross-reference streams |
| PDF 1.6 | (2004) | ✅ Full | ❌ None | Adobe Extension Level 3 |
| PDF 1.7 | (2008) | ✅ Full | ❌ None | Adobe Extension Level 8; PDF/A-2, PDF/UA-1 |
| PDF 2.0 | (2017) | ⚠️ Partial | ❌ None | See detailed breakdown below |

**Key:** ✅ Supported | ⚠️ Partial | ❌ Unsupported | 🔄 Deferred to v1.1+

## PDF 2.0 Feature Coverage

### ✅ Fully Supported (v1.0.0)

#### Encryption: AES-256 (V=5, R=5)

**Status:** ✅ Implemented (Phase 1.4)

- **Standard:** PDF 2.0 `/Encryption V=5, R=5`
- **Algorithm:** AES-256 in CBC mode
- **Key derivation:** PDF 2.0 Revision 5 algorithm (32-byte encryption key)
- **Test fixture:** `EC-06-aes256-encrypted.pdf`

**Implementation:**
```rust
// Phase 1.4 decryption handler
pub fn decrypt_pdf_2_encryption(
    pdf_data: &[u8],
    password: &str,
) -> Result<Vec<u8>, DecryptError> {
    // PDF 2.0 V=5, R=5 uses AES-256-CBC
    let key = derive_key_r5(password, &pdf_data.encrypt_key_metadata);
    let decrypted = aes_256_cbc_decrypt(&pdf_data.encrypted_streams, &key);
    Ok(decrypted)
}
```

**Limitations:**
- No cryptographic validation (signature verification, certificate chain checks)
- Password-only decryption (no certificate-based encryption)
- See "Crypto Validation" below for non-goal documentation

#### XMP Metadata: PDF 2.0 Schema

**Status:** ✅ Implemented (Phase 1.6)

- **Standard:** XMP specification 2014 + PDF 2.0 properties
- **Properties:** `pdf:Prefix`, `pdf:Keywords`, `pdf:Version`
- **Test fixture:** Tagged PDFs with PDF 2.0 XMP

**Implementation:**
```rust
// Phase 1.6 metadata extraction
pub fn extract_xmp_metadata(pdf: &PdfDocument) -> Result<XmpMetadata, MetadataError> {
    let xmp_stream = pdf.get_xmp_stream()?;
    let xml = std::str::from_utf8(&xmp_stream.data)?;
    let pdf_version = extract_pdf_version_from_xmp(xml)?;
    Ok(XmpMetadata { pdf_version, ... })
}
```

### ⚠️ Partially Supported (v1.0.0)

#### Linearization (Fast Web View)

**Status:** ⚠️ Basic support; fingerprint stability verified (Phase 1.7, KU-7)

- **Standard:** PDF 1.4+ feature for incremental download
- **pdftract behavior:** Reads linearized PDFs correctly; fingerprint computed from **primary content stream** (ignores hint tables)
- **Test:** Phase 1.7 critical test verifies fingerprint stability after `qpdf --linearize`

**Limitations:**
- Does not use hint tables for optimized reading
- Reads entire file into memory (no progressive loading)
- Fingerprint includes hint stream data (but is stable across re-linearization)

#### Object Streams (Compressed Objects)

**Status:** ⚠️ Read-only; supports decompression but not compression

- **Standard:** PDF 1.5+; compresses indirect objects in a single stream
- **pdftract behavior:** Decompresses object streams during parsing (Phase 1.2)
- **Implementation:** Uses `flate2` crate for decompression

**Limitations:**
- Cannot write object streams (no PDF generation support)
- Does not validate object stream integrity (assumes well-formed)

#### Cross-Reference Streams

**Status:** ⚠️ Read-only; supports hybrid xref tables

- **Standard:** PDF 1.5+; compressed xref tables
- **pdftract behavior:** Falls back to forward scan if xref stream is corrupt (Phase 1.3, EC-07)

**Limitations:**
- No write support for xref streams
- Hybrid xref (table + stream) may not be fully optimized

### ❌ Unsupported but Documented (v1.0.0)

#### PAdES-LTV Signatures

**Status:** ❌ Unsupported; non-goal per Phase 7.3

- **Standard:** PDF 2.0 Part 3 (PAdES-LTV — Long-Term Validation)
- **Function:** Document signatures with certificate validation and timestamp tokens
- **pdftract behavior:** Extracts signature dictionary as raw metadata; **does not validate**

**Non-Goal Rationale:**
- Crypto validation is out of scope for v1.0.0 (documented in Phase 7.3)
- Signature validation requires:
  - X.509 certificate chain verification
  - CRL/OCSP revocation checking
  - Timestamp token validation (RFC 3161)
  - Complex trust anchor management

**Workaround:**
- Use dedicated PDF validation tools (Adobe Acrobat Reader, `pdfsig` from Poppler)
- Extract signature metadata via pdftract's `/V` dictionary parsing

**Future (v1.1+):**
- May add `--validate-signatures` flag using `openssl` or `rustls` crates
- Would require explicit trust store configuration

#### `/Encryption V5` Cryptographic Enhancements

**Status:** ⚠️ Basic AES-256 decryption; no public-key encryption

- **Standard:** PDF 2.0 `/Encryption V5` adds:
  - Public-key encryption (recipient list)
  - Custom encryption methods
  - Encrypt metadata independently

- **pdftract support:**
  - ✅ Password-based AES-256 (V=5, R=5)
  - ❌ Public-key encryption (certificate-based)
  - ❌ Custom encryption methods
  - ❌ Separate metadata encryption

**Limitations:**
- Only password-based decryption is implemented
- Certificate-based encryption (recipient list) not supported
- Errors with clear diagnostic: `EncryptionError("Certificate-based encryption not supported; see docs/notes/pdf-2-coverage.md")`

#### Adobe Extension Levels

**Status:** ❌ Not parsed; treats all PDF 1.7+ files uniformly

- **Standard:** Adobe Extension Levels (E1-E8) add features beyond ISO 32000-1:2008
- **pdftract behavior:** Ignores `/Extensions` dictionary; reads any PDF 1.7+ file

**Impact:**
- Extension-specific features (e.g., Adobe's private encryption variants) may fail
- Most common extensions (transparency, layers) are standard in PDF 2.0 anyway

### 🔄 Deferred to v1.1+

#### 3D Artwork (PRC / U3D)

**Status:** 🔄 Deferred

- **Standard:** PDF 2.0 `/3D` annotations with PRC or U3D embedded models
- **Use case:** CAD documents, 3D product specifications
- **Priority:** Low (niche use case)
- **Planned:** v1.2+ with `--features 3d` gate

#### Rich Media (Flash, Video, Audio)

**Status:** 🔄 Deferred

- **Standard:** PDF 2.0 `/RichMedia` annotations
- **Use case:** Interactive presentations, multimedia documents
- **Priority:** Low (Flash is obsolete; HTML5 preferred)
- **Planned:** v1.2+ with `--features richmedia` gate

#### Web Capture (XFDF)

**Status:** 🔄 Deferred

- **Standard:** PDF 2.0 `/XFDF` forms for web form capture
- **Use case:** AcroForm submission to web servers
- **Priority:** Medium (relevant to Phase 7.4 forms work)
- **Planned:** v1.1+ as part of forms enhancements

## Compatibility Matrix by Feature

| PDF 2.0 Feature | ISO Reference | Read Support | Write Support | Notes |
|-----------------|---------------|--------------|----------------|-------|
| **Encryption** |||||
| AES-256 password (V=5, R=5) | ISO 32000-2:2017 §7.6 | ✅ Phase 1.4 | ❌ | Phase 7.3 non-goal: no crypto validation |
| Public-key encryption | ISO 32000-2:2017 §7.6.4 | ❌ | ❌ | Deferred to v1.1+ |
| Custom encryption methods | ISO 32000-2:2017 §7.6.5 | ❌ | ❌ | Deferred to v1.2+ |
| **Metadata** |||||
| XMP 2014 schema | ISO 32000-2:2017 §14.3 | ✅ Phase 1.6 | ❌ | Full XMP parsing |
| PDF 2.0 properties | ISO 32000-2:2017 §14.3.2 | ✅ Phase 1.6 | ❌ | `pdf:Prefix`, `pdf:Keywords` |
| **Structure** |||||
| Tagged PDF (PDF/UA-1) | ISO 32000-2:2017 §14.8 | ✅ Phase 7.1 | ❌ | StructTree extraction |
| Reading order | ISO 32000-2:2017 §14.8.4 | ✅ Phase 4.5 | ❌ | XY-cut + reading order |
| **Fonts** |||||
| OpenType fonts | ISO 32000-2:2017 §9.8 | ✅ Phase 2.2 | ❌ | Subset + ToUnicode |
| Variable fonts | ISO 32000-2:2017 §9.9 | ⚠️ Partial | ❌ | Reads as static fonts |
| CJK composite fonts | ISO 32000-2:2017 §9.10 | ✅ Phase 2.3 | ❌ | Type0 + CIDFonts |
| **Graphics** |||||
| JPEG 2000 (JPX) | ISO 32000-2:2017 §8.9.5 | ⚠️ Phase 5.2 | ❌ | OCR support via `full-render` |
| Artifact marked content | ISO 32000-2:2017 §14.7 | ✅ Phase 7.1 | ❌ | Suppress from output |
| Optional content (OCG) | ISO 32000-2:2017 §8.11 | ✅ Phase 1.4 | ❌ | BaseState = OFF handling |
| **Forms** |||||
| AcroForm 2.0 | ISO 32000-2:2017 §12.7 | ✅ Phase 7.4 | ❌ | Field extraction |
| XFA forms | ISO 32000-2:2017 §12.7.8 | ⚠️ Phase 7.4 | ❌ | Placeholder detection |
| **Other** |||||
| Linearization | PDF 1.4+ | ⚠️ Phase 1.7 | ❌ | Read-only, stable fingerprint |
| Object streams | PDF 1.5+ | ✅ Phase 1.2 | ❌ | Decompression only |
| Xref streams | PDF 1.5+ | ✅ Phase 1.3 | ❌ | Hybrid table support |
| PAdES-LTV signatures | ISO 32000-2:2017 §12.8 | ❌ | ❌ | Non-goal; metadata only |
| 3D artwork | ISO 32000-2:2017 §13.6 | ❌ | ❌ | Deferred to v1.2+ |
| Rich media | ISO 32000-2:2017 §13.7 | ❌ | ❌ | Deferred to v1.2+ |

## Non-Goals (Explicitly Out of Scope)

Per Phase 7.3 documentation, the following are **non-goals for v1.0.0**:

1. **Cryptographic validation:** No signature verification, certificate chain checking, or revocation checking
2. **PDF generation:** No write support for any PDF version (read-only tool)
3. **Rendering:** No visual rendering of PDF pages (text extraction only)
4. **JavaScript execution:** No `/JS` action evaluation or form field scripting
5. **Embedded file extraction:** No `/EmbeddedFiles` extraction (attachments)

## Version Detection

pdftract detects PDF 2.0 files via:

1. **Header check:** `%PDF-2.0` or `%PDF-2.N` (N = subversion)
2. **Version key:** `/Version` entry in `/Catalog` dictionary
3. **XMP metadata:** `pdf:Version` property in XMP packet

**Implementation:**
```rust
// Phase 1.1: PDF version detection
pub fn detect_pdf_version(pdf_data: &[u8]) -> PdfVersion {
    // Check header
    if let Some(version) = parse_header_version(pdf_data) {
        return version;
    }
    // Check /Version key
    if let Some(version) = parse_catalog_version(pdf_data) {
        return version;
    }
    // Check XMP
    if let Some(version) = parse_xmp_version(pdf_data) {
        return version;
    }
    PdfVersion::V1_4  // Default baseline
}
```

## Testing

### PDF 2.0 Test Fixtures

- **`EC-06-aes256-encrypted.pdf`** — AES-256 encryption (V=5, R=5)
- **Tagged PDFs with XMP** — PDF 2.0 metadata extraction
- **Linearized PDFs** — Fingerprint stability test (KU-7)

### Regression Tests

```bash
# Run PDF 2.0-specific tests
cargo nextest run pdf_2_encryption
cargo nextest run pdf_2_metadata
cargo nextest run linearization_fingerprint

# Verify PDF 2.0 extraction works
cargo run -- extract tests/fixtures/encrypted/EC-06-aes256-encrypted.pdf --password user256 --json out.json
```

## Future Enhancements (v1.1+)

### v1.1 Planned Additions

1. **Certificate-based encryption** (V=5, R=5 public-key):
   - Parse `/Recipients` list
   - Use `rustls` or `openssl` for decryption
   - Feature gate: `--features encryption-cert`

2. **PAdES-LTV signature metadata extraction**:
   - Extract `/V` and `/Sig` dictionaries
   - Return certificate metadata (subject, issuer, validity)
   - No validation (metadata only)
   - Feature gate: `--features signatures`

3. **PDF 2.0 properties in schema**:
   - Add `pdf_version` to metadata schema
   - Expose `pdf:Prefix`, `pdf:Keywords` in output JSON

### v1.2+ Considerations

1. **Variable font support** (partial):
   - Read `fvar` table for named instances
   - Select default instance for glyph mapping
   - Feature gate: `--features variable-fonts`

2. **3D artwork extraction**:
   - Parse PRC/U3D streams
   - Return mesh data as JSON
   - Feature gate: `--features 3d`

## References

- Plan Risk R10 (line ~564)
- Plan Proof Obligation PB-10 (line ~583)
- ISO 32000-2:2017 — PDF 2.0 specification
- `tests/fixtures/encrypted/EC-06-aes256-encrypted.pdf` — PDF 2.0 encryption test fixture
- Phase 1.4 implementation: PDF 2.0 AES-256 decryption
- Phase 1.6 implementation: XMP metadata extraction
- Phase 7.1 implementation: Tagged PDF / PDF/UA-1 structure tree
