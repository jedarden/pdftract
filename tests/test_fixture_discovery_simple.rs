use std::path::{Path, PathBuf};

fn discover_pdf_fixtures<P: AsRef<Path>>(fixtures_path: P) -> Vec<PathBuf> {
    let fixtures_path = fixtures_path.as_ref();
    let mut pdf_files = Vec::new();

    if let Ok(entries) = std::fs::read_dir(fixtures_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pdf_files.extend(discover_pdf_fixtures(&path));
            } else if path.extension().map(|e| e.eq_ignore_ascii_case("pdf")).unwrap_or(false) {
                pdf_files.push(path);
            }
        }
    }

    pdf_files.sort();
    pdf_files
}

#[test]
fn test_fixture_discovery() {
    let fixtures_dir = "tests/fixtures";
    let pdf_files = discover_pdf_fixtures(fixtures_dir);

    // Verify we found some PDF files
    assert!(!pdf_files.is_empty(), "Should discover at least one PDF file");

    // Verify all files exist
    assert!(pdf_files.iter().all(|p| p.exists()), "All discovered files should exist");

    // Verify all files are PDFs
    assert!(pdf_files.iter().all(|p| p.extension().map(|e| e.eq_ignore_ascii_case("pdf")).unwrap_or(false)),
            "All discovered files should be PDFs");

    // Verify paths are sorted
    assert!(pdf_files.windows(2).all(|w| w[0] < w[1]), "Paths should be sorted");

    // Verify at least first 20 files if we have that many
    for path in pdf_files.iter().take(20) {
        assert!(path.exists(), "File should exist: {}", path.display());
    }
}
