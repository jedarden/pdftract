// Debug script to test fingerprint computation with timeouts
use std::path::Path;
use std::time::Instant;

fn main() {
    let fixtures = vec![
        "tests/fingerprint/fixtures/byte_identical/v1.pdf",
        "tests/fingerprint/fixtures/acrobat_resave/v1.pdf",
        "tests/fingerprint/fixtures/pdftk_resave/v1.pdf",
        "tests/fingerprint/fixtures/qpdf_resave/v1.pdf",
        "tests/fingerprint/fixtures/linearization_toggle/v1.pdf",
        "tests/fingerprint/fixtures/metadata_only/v1.pdf",
        "tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf",
        "tests/fingerprint/fixtures/content_edit_one_paragraph/v1.pdf",
    ];

    for path in fixtures {
        println!("\n=== Testing {} ===", path);
        let path_obj = Path::new(path);
        if !path_obj.exists() {
            println!("  File not found!");
            continue;
        }

        let start = Instant::now();
        match pdftract_core::document::compute_pdf_fingerprint(path_obj) {
            Ok(fp) => {
                let elapsed = start.elapsed();
                println!("  ✓ Fingerprint: {} (took {:?}", fp, elapsed);
            }
            Err(e) => {
                let elapsed = start.elapsed();
                println!("  ✗ Error after {:?}: {}", elapsed, e);
            }
        }

        // Safety: if any test takes > 5 seconds, abort
        if start.elapsed().as_secs() > 5 {
            println!("  WARNING: Test taking too long, aborting");
            break;
        }
    }
}
