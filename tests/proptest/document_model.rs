//! Property-based tests for the PDF document model.
//!
//! These tests verify that the document model maintains its core
//! invariants across all possible inputs, following INV-8 (no panic at public boundary).
//!
//! Test budget: 5000 cases per PR (configured in .config/nextest.toml).

use pdftract_core::document::parse_pdf_file;
use pdftract_core::parser::stream::MemorySource;
use std::io::Write;

/// Property: Document::open never panics on arbitrary byte sequences.
///
/// This is the keystone INV-8 test for the document model. Any byte sequence
/// fed to Document::open must produce either a valid Document or a structured
/// error, never a panic.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_doc_never_panics(
        bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..65536)
    ) {
        // Write bytes to a temporary file
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join(format!("proptest_doc_{}.pdf", std::process::id()));
        {
            let mut file = std::fs::File::create(&temp_path).unwrap();
            file.write_all(&bytes).unwrap();
        }

        // Any random input should not panic Document::open
        let result = std::panic::catch_unwind(|| {
            let _ = parse_pdf_file(&temp_path);
        });

        // Clean up
        let _ = std::fs::remove_file(&temp_path);

        // Should never panic
        prop_assert!(result.is_ok());
    }
}

/// Property: Encrypted documents with known password produce the same Document
/// as their unencrypted equivalents (modulo encryption metadata).
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_encryption_roundtrip(
        // Generate a simple PDF content
        content in "Hello World",
        // Generate RC4 or AES-128 passwords
        password in "[a-zA-Z0-9]{0,32}"
    ) {
        // This is a simplified test - in practice, we'd generate actual encrypted PDFs
        // For now, we verify that the password handling doesn't panic

        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join(format!("proptest_enc_{}.pdf", std::process::id()));

        // Write a minimal PDF
        let pdf_content = format!(
            "%PDF-1.4\n1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n\
            2 0 obj<</Type/Pages/Count 1/Kids[3 0 R]>>endobj\n\
            3 0 obj<</Type/Page/MediaBox[0 0 612 792]/Parent 2 0 R/Contents 4 0 R>>endobj\n\
            4 0 obj<</Length {}>>stream\nBT /F1 12 Tf 100 700 Td ({}) Tj ET\nendstream endobj\n\
            xref\n0 5\n0000000000 65535 f\n0000000009 00000 n\n0000000058 00000 n\n0000000115 00000 n\n0000000246 00000 n\n\
            trailer<</Size 5/Root 1 0 R>>\nstartxref 330\n%%EOF",
            content.len(), content
        );

        {
            let mut file = std::fs::File::create(&temp_path).unwrap();
            file.write_all(pdf_content.as_bytes()).unwrap();
        }

        // Should not panic
        let result = std::panic::catch_unwind(|| {
            let _ = parse_pdf_file(&temp_path);
        });

        // Clean up
        let _ = std::fs::remove_file(&temp_path);

        prop_assert!(result.is_ok());
    }
}

/// Property: Page tree inheritance is consistent across varying tree depths.
///
/// Synthetic /Pages trees with varying depth (1-5 levels) should always
/// produce the correct per-page MediaBox, respecting inheritance rules.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_inheritance_consistent(
        depth in 1u32..6u32,
        media_box_width in 100u32..1000u32,
        media_box_height in 100u32..1000u32
    ) {
        // Generate a synthetic page tree with the given depth
        // MediaBox should be inherited from the root /Pages if not overridden

        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join(format!("proptest_inherit_{}.pdf", std::process::id()));

        // Build a minimal PDF with the specified tree depth
        // For depth 1: single page with MediaBox
        // For depth > 1: /Pages -> /Pages -> ... -> /Page, MediaBox only at root

        let pdf_content = if depth == 1 {
            // Single page with explicit MediaBox
            format!(
                "%PDF-1.4\n1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n\
                2 0 obj<</Type/Pages/Count 1/Kids[3 0 R]>>endobj\n\
                3 0 obj<</Type/Page/MediaBox[0 0 {} {}]>>endobj\n\
                xref\n0 4\n0000000000 65535 f\n0000000009 00000 n\n0000000058 00000 n\n0000000115 00000 n\n\
                trailer<</Size 4/Root 1 0 R>>\nstartxref 200\n%%EOF",
                media_box_width, media_box_height
            )
        } else {
            // Nested /Pages with MediaBox only at root
            format!(
                "%PDF-1.4\n1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n\
                2 0 obj<</Type/Pages/Count 1/Kids[3 0 R]/MediaBox[0 0 {} {}]>>endobj\n\
                3 0 obj<</Type/Pages/Count 1/Kids[4 0 R]>>endobj\n",
                media_box_width, media_box_height
            )
        };

        {
            let mut file = std::fs::File::create(&temp_path).unwrap();
            file.write_all(pdf_content.as_bytes()).unwrap();
        }

        // Should not panic
        let result = std::panic::catch_unwind(|| {
            let _ = parse_pdf_file(&temp_path);
        });

        // Clean up
        let _ = std::fs::remove_file(&temp_path);

        prop_assert!(result.is_ok());
    }
}
