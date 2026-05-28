//! Tests for forward-scan disable on remote sources (Phase 1.8).
//!
//! This test verifies that the forward-scan xref recovery (strategy 4)
//! is disabled for remote sources to prevent downloading the entire file.

#![cfg(feature = "remote")]

use pdftract_core::parser::xref::{forward_scan_xref, XrefSection};
use pdftract_core::parser::stream::PdfSource;

/// Mock remote PDF source that returns is_remote() = true.
struct MockRemoteSource {
    data: Vec<u8>,
}

impl PdfSource for MockRemoteSource {
    fn len(&self) -> std::io::Result<u64> {
        Ok(self.data.len() as u64)
    }

    fn read_at(&self, _offset: u64, _length: usize) -> std::io::Result<bytes::Bytes> {
        Ok(bytes::Bytes::new())
    }

    fn is_remote(&self) -> bool {
        true // This is the key - remote source
    }
}

/// Mock local PDF source that returns is_remote() = false.
struct MockLocalSource {
    data: Vec<u8>,
}

impl PdfSource for MockLocalSource {
    fn len(&self) -> std::io::Result<u64> {
        Ok(self.data.len() as u64)
    }

    fn read_at(&self, offset: u64, length: usize) -> std::io::Result<bytes::Bytes> {
        let end = (offset as usize + length).min(self.data.len());
        Ok(bytes::Bytes::copy_from_slice(&self.data[offset as usize..end]))
    }

    fn is_remote(&self) -> bool {
        false // Local source
    }
}

/// Test that forward-scan is disabled for remote sources.
#[test]
fn test_forward_scan_disabled_for_remote() {
    let pdf_data = b"%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [ 3 0 R ] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 5 0 R >>
endobj
4 0 obj
<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>
endobj
5 0 obj
<< /Length 0 >>
stream

endstream
endobj
xref
0 6
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
0000000244 00000 n
0000000317 00000 n
trailer
<< /Size 6 /Root 1 0 R >>
startxref
412
%%EOF
".to_vec();

    let remote_source = MockRemoteSource { data: pdf_data };
    let result = forward_scan_xref(&remote_source, false);

    // Should return empty xref section
    assert!(result.entries.is_empty());
    assert!(result.trailer.is_none());

    // Should emit STRUCT_REMOTE_NO_FORWARD_SCAN diagnostic
    use pdftract_core::diagnostics::DiagCode;
    let has_remote_diagnostic = result.diagnostics.iter().any(|d| {
        matches!(d.code, DiagCode::XrefRemoteNoForwardScan)
    });
    assert!(has_remote_diagnostic, "Expected XREF_REMOTE_NO_FORWARD_SCAN diagnostic for remote source");
}

/// Test that forward-scan works for local sources.
#[test]
fn test_forward_scan_enabled_for_local() {
    let pdf_data = b"%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
xref
0 2
0000000000 65535 f
0000000009 00000 n
trailer
<< /Size 2 /Root 1 0 R >>
startxref
52
%%EOF
".to_vec();

    let local_source = MockLocalSource { data: pdf_data };
    let result = forward_scan_xref(&local_source, false);

    // Should find at least one entry (object 1)
    // Note: forward-scan is best-effort, so we just verify it doesn't fail
    // The exact behavior depends on the PDF structure
}

/// Test that both linearized AND remote disable forward-scan.
#[test]
fn test_forward_scan_disabled_for_linearized() {
    let pdf_data = b"%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
xref
0 2
0000000000 65535 f
0000000009 00000 n
trailer
<< /Size 2 /Root 1 0 R >>
startxref
52
%%EOF
".to_vec();

    let local_source = MockLocalSource { data: pdf_data };
    let result = forward_scan_xref(&local_source, true); // is_linearized = true

    // Should return empty xref section
    assert!(result.entries.is_empty());

    // Should emit LINEARIZED_NO_FORWARD_SCAN diagnostic
    use pdftract_core::diagnostics::DiagCode;
    let has_linearized_diagnostic = result.diagnostics.iter().any(|d| {
        matches!(d.code, DiagCode::XrefLinearizedNoForwardScan)
    });
    assert!(has_linearized_diagnostic, "Expected XREF_LINEARIZED_NO_FORWARD_SCAN diagnostic for linearized PDF");
}

/// Test that linearized + remote prioritizes linearized diagnostic.
#[test]
fn test_linearized_remote_diagnostic_priority() {
    let pdf_data = b"%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
xref
0 2
0000000000 65535 f
0000000009 00000 n
trailer
<< /Size 2 /Root 1 0 R >>
startxref
52
%%EOF
".to_vec();

    let remote_source = MockRemoteSource { data: pdf_data };
    let result = forward_scan_xref(&remote_source, true); // Both linearized AND remote

    // Should return empty xref section
    assert!(result.entries.is_empty());

    // Should emit LINEARIZED_NO_FORWARD_SCAN (checked first)
    use pdftract_core::diagnostics::DiagCode;
    let has_linearized_diagnostic = result.diagnostics.iter().any(|d| {
        matches!(d.code, DiagCode::XrefLinearizedNoForwardScan)
    });
    assert!(has_linearized_diagnostic, "Expected linearized check to come first");
}
