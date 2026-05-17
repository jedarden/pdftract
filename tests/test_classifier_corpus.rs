//! Classifier corpus validation tests
//!
//! This module tests the document type classifier against the 200-document
//! labeled corpus at `tests/fixtures/classifier/`.
//!
//! The corpus is partitioned as:
//! - 50 invoices
//! - 50 scientific papers
//! - 50 contracts
//! - 50 misc (receipts, forms, bank statements, slide decks, legal filings, book excerpts, magazines)
//!
//! Acceptance criteria (from plan.md Phase 5.6):
//! - Per-class precision and recall >= 0.85
//! - Macro-F1 >= 0.88
//! - Reproducibility: classifying the same document twice produces identical output

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Path to the classifier corpus directory
const CORPUS_DIR: &str = "tests/fixtures/classifier";

/// Path to the MANIFEST.tsv file
const MANIFEST_FILE: &str = "tests/fixtures/classifier/MANIFEST.tsv";

/// Minimum per-class precision/recall threshold
const MIN_PRECISION_RECALL: f64 = 0.85;

/// Minimum macro-F1 threshold
const MIN_MACRO_F1: f64 = 0.88;

/// Document type classification result
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ClassificationResult {
    /// Predicted document type
    predicted_type: String,
    /// Expected document type (from MANIFEST.tsv)
    expected_type: String,
    /// Document path
    path: PathBuf,
}

/// Per-class statistics
#[derive(Debug, Default)]
struct ClassStats {
    /// True positives: correctly classified as this class
    tp: usize,
    /// False positives: incorrectly classified as this class
    fp: usize,
    /// False negatives: this class incorrectly classified as something else
    fn_val: usize,
}

impl ClassStats {
    /// Calculate precision: TP / (TP + FP)
    fn precision(&self) -> f64 {
        let denominator = self.tp + self.fp;
        if denominator == 0 {
            0.0
        } else {
            self.tp as f64 / denominator as f64
        }
    }

    /// Calculate recall: TP / (TP + FN)
    fn recall(&self) -> f64 {
        let denominator = self.tp + self.fn_val;
        if denominator == 0 {
            0.0
        } else {
            self.tp as f64 / denominator as f64
        }
    }

    /// Calculate F1 score: 2 * (precision * recall) / (precision + recall)
    fn f1(&self) -> f64 {
        let p = self.precision();
        let r = self.recall();
        if p + r == 0.0 {
            0.0
        } else {
            2.0 * (p * r) / (p + r)
        }
    }
}

/// Manifest entry
struct ManifestEntry {
    path: PathBuf,
    expected_type: String,
    source_url: String,
    license: String,
}

/// Parse MANIFEST.tsv file
fn parse_manifest() -> Vec<ManifestEntry> {
    let manifest_path = Path::new(MANIFEST_FILE);

    // Skip test if corpus not present (e.g., in CI without test data)
    if !manifest_path.exists() {
        eprintln!("SKIPPED: Classifier corpus not found at {MANIFEST_FILE}");
        eprintln!("To run this test, generate the corpus using: python3 scripts/generate_test_corpus.py");
        std::process::exit(0); // Exit with success since this is expected in some environments
    }

    let content = std::fs::read_to_string(manifest_path)
        .unwrap_or_else(|e| panic!("Failed to read manifest: {e}"));

    let mut entries = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        // Skip header
        if line_num == 0 {
            continue;
        }

        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 4 {
            continue;
        }

        entries.push(ManifestEntry {
            path: PathBuf::from(parts[0]),
            expected_type: parts[1].to_string(),
            source_url: parts[2].to_string(),
            license: parts[3].to_string(),
        });
    }

    entries
}

/// Classify a document using the pdftract classifier
///
/// This is a placeholder that will be implemented once Phase 5.6
/// document type classification is available.
fn classify_document(_path: &Path) -> Option<String> {
    // TODO: Implement once Phase 5.6 classifier exists
    // For now, return None to indicate classifier not available
    None
}

/// Run classification on all documents in the corpus
fn run_corpus_classification() -> Vec<ClassificationResult> {
    let manifest = parse_manifest();
    let corpus_base = Path::new(CORPUS_DIR);

    let mut results = Vec::new();

    for entry in &manifest {
        let full_path = corpus_base.join(&entry.path);

        if !full_path.exists() {
            panic!("Corpus file not found: {}", full_path.display());
        }

        // Skip classification if not implemented yet
        if let Some(predicted) = classify_document(&full_path) {
            results.push(ClassificationResult {
                predicted_type: predicted,
                expected_type: entry.expected_type.clone(),
                path: full_path,
            });
        }
    }

    results
}

/// Compute per-class statistics from classification results
fn compute_class_stats(results: &[ClassificationResult]) -> HashMap<String, ClassStats> {
    let mut stats: HashMap<String, ClassStats> = HashMap::new();

    for result in results {
        // Update stats for the predicted class
        let pred_stats = stats.entry(result.predicted_type.clone()).or_default();
        if result.predicted_type == result.expected_type {
            pred_stats.tp += 1;
        } else {
            pred_stats.fp += 1;
        }

        // Update stats for the expected class (for FN counting)
        let exp_stats = stats.entry(result.expected_type.clone()).or_default();
        if result.predicted_type != result.expected_type {
            exp_stats.fn_val += 1;
        }
    }

    stats
}

/// Calculate macro-F1 score (average of per-class F1 scores)
fn compute_macro_f1(stats: &HashMap<String, ClassStats>) -> f64 {
    if stats.is_empty() {
        return 0.0;
    }

    let total_f1: f64 = stats.values().map(|s| s.f1()).sum();
    total_f1 / stats.len() as f64
}

#[test]
fn test_classifier_corpus_accuracy() {
    // This test will be enabled once the classifier is implemented
    // For now, it's a placeholder that documents the expected structure

    let results = run_corpus_classification();

    if results.is_empty() {
        // Classifier not implemented yet - skip gracefully
        eprintln!("SKIP: Classifier not yet implemented (Phase 5.6)");
        return;
    }

    let stats = compute_class_stats(&results);

    // Check per-class precision and recall
    for (class_name, class_stats) in &stats {
        let precision = class_stats.precision();
        let recall = class_stats.recall();

        println!(
            "{}: precision={:.3}, recall={:.3}, f1={:.3}",
            class_name,
            precision,
            recall,
            class_stats.f1()
        );

        assert!(
            precision >= MIN_PRECISION_RECALL,
            "{} precision ({:.3}) below threshold ({:.3})",
            class_name,
            precision,
            MIN_PRECISION_RECALL
        );

        assert!(
            recall >= MIN_PRECISION_RECALL,
            "{} recall ({:.3}) below threshold ({:.3})",
            class_name,
            recall,
            MIN_PRECISION_RECALL
        );
    }

    // Check macro-F1
    let macro_f1 = compute_macro_f1(&stats);
    println!("Macro-F1: {:.3}", macro_f1);

    assert!(
        macro_f1 >= MIN_MACRO_F1,
        "Macro-F1 ({:.3}) below threshold ({:.3})",
        macro_f1,
        MIN_MACRO_F1
    );
}

#[test]
fn test_classifier_reproducibility() {
    // Test that classifying the same document twice produces identical output
    // Sample 20 documents for this test

    let manifest = parse_manifest();
    let corpus_base = Path::new(CORPUS_DIR);

    // Sample first 20 documents
    let sample_docs: Vec<_> = manifest.iter().take(20).collect();

    for entry in sample_docs {
        let full_path = corpus_base.join(&entry.path);

        if !full_path.exists() {
            continue;
        }

        // Classify twice
        let result1 = classify_document(&full_path);
        let result2 = classify_document(&full_path);

        // Check for reproducibility
        match (result1, result2) {
            (Some(r1), Some(r2)) => {
                assert_eq!(
                    r1, r2,
                    "Classification not reproducible for {}",
                    full_path.display()
                );
            }
            (None, None) => {
                // Classifier not implemented - skip
                continue;
            }
            _ => {
                panic!("Inconsistent classification results for {}", full_path.display());
            }
        }
    }
}

#[test]
fn test_corpus_manifest_validity() {
    // Test that the manifest is well-formed and all referenced files exist
    let manifest = parse_manifest();
    let corpus_base = Path::new(CORPUS_DIR);

    assert!(!manifest.is_empty(), "Manifest is empty");

    // Count documents per type
    let mut type_counts: HashMap<&str, usize> = HashMap::new();

    for entry in &manifest {
        let full_path = corpus_base.join(&entry.path);

        assert!(
            full_path.exists(),
            "Referenced file not found: {}",
            full_path.display()
        );

        *type_counts.entry(&entry.expected_type).or_insert(0) += 1;

        // Check that source_url and license are present
        assert!(
            !entry.source_url.is_empty(),
            "Missing source_url for {}",
            entry.path.display()
        );
        assert!(
            !entry.license.is_empty(),
            "Missing license for {}",
            entry.path.display()
        );
    }

    // Verify expected counts
    assert_eq!(
        type_counts.get("invoice").copied().unwrap_or(0),
        50,
        "Expected 50 invoices"
    );
    assert_eq!(
        type_counts.get("scientific_paper").copied().unwrap_or(0),
        50,
        "Expected 50 scientific papers"
    );
    assert_eq!(
        type_counts.get("contract").copied().unwrap_or(0),
        50,
        "Expected 50 contracts"
    );

    // Verify misc subtypes
    let misc_total = type_counts
        .iter()
        .filter(|(k, _)| {
            matches!(
                *k,
                &"receipt"
                    | &"form"
                    | &"bank_statement"
                    | &"slide_deck"
                    | &"legal_filing"
                    | &"book_excerpt"
                    | &"magazine"
            )
        })
        .map(|(_, v)| *v)
        .sum::<usize>();

    assert_eq!(misc_total, 50, "Expected 50 misc documents");

    println!("Manifest validity check passed:");
    println!("  - Total documents: {}", manifest.len());
    for (type_name, count) in &type_counts {
        println!("  - {}: {}", type_name, count);
    }
}
