//! Compiled Phase 5.5 hybrid-corpus classifier test.
//!
//! The fixture sidecars describe the expected hybrid regions for these
//! intentionally synthetic PDFs.  The core's public 8x8 classifier consumes
//! `CellData`, while the PDF extraction result does not yet expose the
//! PDF-to-`PageContext` bridge.  This harness therefore validates the corpus
//! declarations against the public grid classifier and uses the real PDF
//! page geometry while doing so.

use std::fs;
use std::path::{Path, PathBuf};

use pdftract_core::classify::{CellData, CellIndex, GridClassifier, PageClass};
use serde::Deserialize;

const GRID_CELL_COUNT: usize = 64;
const MIN_HYBRID_CELLS: usize = 10;

const HYBRID_FIXTURES: &[&str] = &[
    "hybrid-001-vector-header-over-scan.pdf",
    "hybrid-002-vector-form-over-scan.pdf",
    "hybrid-003-mixed-column-layout.pdf",
    "hybrid-004-watermark-over-scan.pdf",
    "hybrid-005-vector-footer-over-scan.pdf",
    "hybrid-006-stamp-annotation.pdf",
    "hybrid-007-textbox-overlay.pdf",
    "hybrid-008-rotated-vector.pdf",
    "hybrid-009-transparent-vector.pdf",
    "hybrid-010-complex-layered.pdf",
];

#[derive(Debug, Deserialize)]
struct FixtureMetadata {
    expected_classification: ExpectedClassification,
    grid_cell_coverage: GridCellCoverage,
}

#[derive(Debug, Deserialize)]
struct ExpectedClassification {
    page_class: String,
}

#[derive(Debug, Deserialize)]
struct GridCellCoverage {
    hybrid_cells_approx: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ExpectedMismatch {
    fixture: &'static str,
    observed_class: PageClass,
    reason: &'static str,
}

// KU-2 / Phase 5.5 tuning inputs: keep observed deviations explicit until
// classifier tuning makes these fixtures satisfy the Hybrid rule.
const EXPECTED_MISMATCH: &[ExpectedMismatch] = &[
    ExpectedMismatch {
        fixture: "hybrid-001-vector-header-over-scan.pdf",
        observed_class: PageClass::Vector,
        reason: "8 declared cells leaves the scan side below the 10-cell rule",
    },
    ExpectedMismatch {
        fixture: "hybrid-003-mixed-column-layout.pdf",
        observed_class: PageClass::Vector,
        reason: "0 declared hybrid cells leaves the scan side below the 10-cell rule",
    },
    ExpectedMismatch {
        fixture: "hybrid-004-watermark-over-scan.pdf",
        observed_class: PageClass::Scanned,
        reason: "64 scanned cells leaves fewer than 10 vector cells",
    },
    ExpectedMismatch {
        fixture: "hybrid-005-vector-footer-over-scan.pdf",
        observed_class: PageClass::Vector,
        reason: "8 declared cells leaves the scan side below the 10-cell rule",
    },
    ExpectedMismatch {
        fixture: "hybrid-010-complex-layered.pdf",
        observed_class: PageClass::Scanned,
        reason: "56 scanned cells leaves fewer than 10 vector cells",
    },
];

fn fixture_dir() -> PathBuf {
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let from_manifest = PathBuf::from(manifest_dir).join("../../tests/fixtures/hybrid");
        if from_manifest.exists() {
            return from_manifest;
        }
    }

    for relative in [
        Path::new("tests/fixtures/hybrid"),
        Path::new("../../tests/fixtures/hybrid"),
    ] {
        if relative.exists() {
            return relative.to_path_buf();
        }
    }

    panic!(
        "Hybrid fixture directory not found. Tried $CARGO_MANIFEST_DIR/../../tests/fixtures/hybrid, tests/fixtures/hybrid, and ../../tests/fixtures/hybrid"
    );
}

fn metadata_for(path: &Path) -> FixtureMetadata {
    let metadata_path = PathBuf::from(format!("{}.metadata.json", path.display()));
    let metadata = fs::read_to_string(&metadata_path).unwrap_or_else(|error| {
        panic!(
            "failed to read metadata sidecar {}: {error}",
            metadata_path.display()
        )
    });

    serde_json::from_str(&metadata).unwrap_or_else(|error| {
        panic!(
            "failed to parse metadata sidecar {}: {error}",
            metadata_path.display()
        )
    })
}

fn classify_declared_grid(
    width: f64,
    height: f64,
    rotation: i32,
    scanned_cells: usize,
) -> (PageClass, usize, usize) {
    let scanned_cells = scanned_cells.min(GRID_CELL_COUNT);
    let mut grid = GridClassifier::new(width, height, rotation);

    for index in 0..GRID_CELL_COUNT {
        let cell = grid.cell_mut(CellIndex::from_flat(index));
        if index < scanned_cells {
            *cell = CellData {
                text_op_count: 0,
                image_coverage: 0.90,
                char_validity: 0.0,
            };
        } else {
            *cell = CellData {
                text_op_count: 1,
                image_coverage: 0.05,
                char_validity: 0.95,
            };
        }
    }

    let classification = grid.classify();
    (
        classification.class,
        scanned_cells,
        GRID_CELL_COUNT - scanned_cells,
    )
}

fn mismatch_for(fixture: &str) -> Option<&'static ExpectedMismatch> {
    EXPECTED_MISMATCH
        .iter()
        .find(|mismatch| mismatch.fixture == fixture)
}

#[test]
fn hybrid_corpus_matches_sidecars_and_grid_rule() {
    assert_eq!(
        HYBRID_FIXTURES.len(),
        10,
        "KU-2 requires exactly 10 fixtures"
    );

    let base = fixture_dir();
    let mut failures = Vec::new();
    let mut rows = Vec::new();

    for fixture in HYBRID_FIXTURES {
        let pdf_path = base.join(fixture);
        assert!(
            pdf_path.is_file(),
            "missing hybrid fixture {}",
            pdf_path.display()
        );
        let pdf = fs::read(&pdf_path).unwrap_or_else(|error| {
            panic!(
                "failed to read hybrid fixture {}: {error}",
                pdf_path.display()
            )
        });
        assert!(
            pdf.starts_with(b"%PDF-"),
            "{} is not a PDF",
            pdf_path.display()
        );

        let metadata = metadata_for(&pdf_path);
        assert_eq!(
            metadata.expected_classification.page_class, "Hybrid",
            "{} sidecar must declare expected_classification.page_class=Hybrid",
            fixture
        );

        let extractor = pdftract_core::document::PdfExtractor::open(&pdf_path)
            .unwrap_or_else(|error| panic!("failed to open {}: {error}", pdf_path.display()));
        assert_eq!(
            extractor.page_count().unwrap_or_else(|error| {
                panic!("failed to count pages in {}: {error}", pdf_path.display())
            }),
            1,
            "{} must contain exactly one page",
            fixture
        );
        let page = extractor
            .pages()
            .next()
            .expect("single-page fixture did not yield a page")
            .unwrap_or_else(|error| {
                panic!("failed to read page in {}: {error}", pdf_path.display())
            });

        let (observed_class, cell_count, vector_cells) = classify_declared_grid(
            page.width,
            page.height,
            page.rotation,
            metadata.grid_cell_coverage.hybrid_cells_approx,
        );
        let cell_percentage = cell_count as f64 / GRID_CELL_COUNT as f64 * 100.0;
        let meets_coverage_rule = cell_count >= MIN_HYBRID_CELLS;
        let mismatch = mismatch_for(fixture);

        let status = if observed_class == PageClass::Hybrid {
            assert!(
                meets_coverage_rule && vector_cells >= MIN_HYBRID_CELLS,
                "{} classified Hybrid without >=10 scanned and >=10 vector cells",
                fixture
            );
            assert!(
                mismatch.is_none(),
                "{} is now green; remove its stale EXPECTED_MISMATCH entry",
                fixture
            );
            "PASS"
        } else if let Some(expected_mismatch) = mismatch {
            assert_eq!(
                observed_class, expected_mismatch.observed_class,
                "{} changed observed class; update EXPECTED_MISMATCH and this table",
                fixture
            );
            assert!(
                !meets_coverage_rule || vector_cells < MIN_HYBRID_CELLS,
                "{} is an undocumented non-Hybrid result despite meeting the grid rule",
                fixture
            );
            "EXPECTED-MISMATCH"
        } else {
            failures.push(format!(
                "{fixture}: expected Hybrid, observed {observed_class:?} at {cell_percentage:.2}%"
            ));
            "FAIL"
        };

        rows.push(format!(
            "{fixture:42} | observed={observed_class:?} | cells={cell_count:2}/64 ({cell_percentage:6.2}%) | {status}{}",
            mismatch
                .map(|entry| format!(" ({})", entry.reason))
                .unwrap_or_default()
        ));
    }

    assert!(
        failures.is_empty(),
        "hybrid corpus classification failures:\nfixture                                       | result\n{}\n\n{}",
        rows.join("\n"),
        failures.join("\n")
    );

    // Do not let a tuning mismatch become invisible when the classifier changes.
    for expected_mismatch in EXPECTED_MISMATCH {
        assert!(
            rows.iter()
                .any(|row| row.starts_with(expected_mismatch.fixture)),
            "EXPECTED_MISMATCH names fixture not in the corpus: {}",
            expected_mismatch.fixture
        );
    }

    println!(
        "fixture                                       | result\n{}",
        rows.join("\n")
    );
}
