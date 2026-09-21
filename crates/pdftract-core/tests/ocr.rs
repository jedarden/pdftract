//! Focused contract tests for the scanned-corpus WER measurement gate
//! (`scripts/measure-wer.sh`).
//!
//! The gate itself shells out to poppler + tesseract and is verified by
//! `scripts/measure-wer.sh --self-test` plus the live corpus run documented in
//! `tests/fixtures/scanned/README.md`. These tests pin the *fixture contract*
//! that the gate depends on — every manifest row matches a file on disk,
//! ground truths are non-trivial, the script stays executable, and the
//! clean/degraded class split that keeps the degraded 200 DPI fixture visible
//! but non-gating is intact — so a missing, moved, or emptied fixture (or a
//! manifest row that drifts from disk) fails `cargo test` directly instead of
//! only surfacing inside a shell run:
//!
//! ```text
//! cargo test -p pdftract-core --test ocr
//! ```
//!
//! These tests are feature-independent and always run. The feature-gated
//! in-Rust OCR pipeline tests live in `ocr_integration.rs`; that feature's
//! compile state is tracked separately (bead `pdftract-ecad3b80`).

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// The measured script, embedded at compile time: moving or renaming it is a
/// compile error here rather than a runtime surprise in the shell gate.
const WER_SCRIPT: &str = include_str!("../../../scripts/measure-wer.sh");

const CORPUS_SUBPATH: &str = "tests/fixtures/scanned";

#[derive(Debug)]
struct ManifestRow {
    name: String,
    class: String,
    scan: String,
    ground_truth: String,
    reference_ocr: String,
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../")
        .canonicalize()
        .expect("workspace root above crates/pdftract-core must exist")
}

fn corpus_root() -> PathBuf {
    let root = repo_root().join(CORPUS_SUBPATH);
    root.canonicalize()
        .unwrap_or_else(|e| panic!("corpus root {} must exist: {e}", root.display()))
}

fn gate_script_path() -> PathBuf {
    repo_root().join("scripts/measure-wer.sh")
}

/// Parse the `FIXTURE_MANIFEST` heredoc out of the embedded script so the
/// manifest's source of truth is tested, never a copied table.
fn manifest_rows() -> Vec<ManifestRow> {
    let mut rows = Vec::new();
    let mut in_manifest = false;
    for line in WER_SCRIPT.lines() {
        if line.starts_with("read -r -d '' FIXTURE_MANIFEST") {
            in_manifest = true;
            continue;
        }
        if !in_manifest {
            continue;
        }
        if line == "EOF" {
            break;
        }
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split('|').collect();
        assert_eq!(
            fields.len(),
            5,
            "manifest row `{line}` must carry 5 |-separated fields: \
             name|class|scan.pdf|ground-truth.txt|reference-ocr.txt"
        );
        rows.push(ManifestRow {
            name: fields[0].to_string(),
            class: fields[1].to_string(),
            scan: fields[2].to_string(),
            ground_truth: fields[3].to_string(),
            reference_ocr: fields[4].to_string(),
        });
    }
    assert!(
        !rows.is_empty(),
        "FIXTURE_MANIFEST heredoc not found in scripts/measure-wer.sh — \
         the heredoc marker or EOF terminator drifted"
    );
    rows
}

#[test]
fn gate_script_is_present_and_executable() {
    let path = gate_script_path();
    let meta = fs::metadata(&path)
        .unwrap_or_else(|e| panic!("{} must exist: {e}", path.display()));
    assert!(meta.is_file(), "{} must be a regular file", path.display());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert!(
            meta.permissions().mode() & 0o111 != 0,
            "{} must keep its executable bit (chmod +x)",
            path.display()
        );
    }
}

#[test]
fn fixture_manifest_rows_are_wellformed() {
    let rows = manifest_rows();
    assert!(
        rows.len() >= 6,
        "gate corpus shrank: {} manifested fixtures, expected at least the \
         5 clean + 1 degraded rows",
        rows.len()
    );
    let mut names = HashSet::new();
    for row in &rows {
        assert!(
            !row.name.trim().is_empty() && !row.name.contains(' '),
            "fixture name `{}` must be a non-empty token",
            row.name
        );
        assert!(
            names.insert(row.name.as_str()),
            "duplicate manifest entry for fixture `{}`",
            row.name
        );
        assert!(
            row.scan.ends_with(".pdf"),
            "fixture {}: scan column must be a .pdf path, got `{}`",
            row.name,
            row.scan
        );
        assert!(
            row.ground_truth.ends_with(".txt") && row.reference_ocr.ends_with(".txt"),
            "fixture {}: ground-truth and reference-OCR columns must be .txt paths",
            row.name
        );
    }
}

#[test]
fn clean_and_degraded_classes_are_separated() {
    let rows = manifest_rows();
    for row in &rows {
        assert!(
            row.class == "clean" || row.class == "degraded",
            "fixture {}: class `{}` must be `clean` or `degraded`",
            row.name,
            row.class
        );
    }
    let clean = rows.iter().filter(|r| r.class == "clean").count();
    let degraded = rows.iter().filter(|r| r.class == "degraded").count();
    assert!(
        clean >= 5,
        "Phase 5 gate needs at least 5 clean 300 DPI fixtures, found {clean}"
    );
    assert!(
        degraded >= 1,
        "the degraded 200 DPI fixture must stay manifested as class `degraded` \
         so its expected-high WER is reported without gating"
    );
}

#[test]
fn clean_fixtures_span_at_least_four_document_types() {
    let dirs: HashSet<String> = manifest_rows()
        .iter()
        .filter(|r| r.class == "clean")
        .map(|r| {
            Path::new(&r.scan)
                .parent()
                .unwrap_or(Path::new("."))
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert!(
        dirs.len() >= 4,
        "clean fixtures must span at least 4 document-type directories \
         (receipt, invoice, letter, form, multi-page), found {}: {dirs:?}",
        dirs.len()
    );
}

#[test]
fn gate_thresholds_match_phase5_contract() {
    assert!(
        WER_SCRIPT.contains("THRESHOLD_PCT=\"3\""),
        "measure-wer.sh must default the clean gate to the Phase 5 threshold (3%)"
    );
    assert!(
        WER_SCRIPT.contains("DEGRADED_TARGET_PCT="),
        "measure-wer.sh must carry a separate informational soft target so the \
         degraded fixture is reported independently of the clean gate"
    );
}

#[test]
fn manifest_scans_exist_and_are_pdfs() {
    let root = corpus_root();
    for row in manifest_rows() {
        let path = root.join(&row.scan);
        let bytes = fs::read(&path).unwrap_or_else(|e| {
            panic!(
                "fixture {}: scan {} unreadable: {e}",
                row.name,
                path.display()
            )
        });
        assert!(
            bytes.starts_with(b"%PDF"),
            "fixture {}: {} does not start with the %PDF magic",
            row.name,
            path.display()
        );
    }
}

#[test]
fn ground_truths_exist_and_are_nontrivial() {
    let root = corpus_root();
    for row in manifest_rows() {
        let path = root.join(&row.ground_truth);
        let text = fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!(
                "fixture {}: ground truth {} unreadable: {e}",
                row.name,
                path.display()
            )
        });
        let words = text.split_whitespace().count();
        assert!(
            words >= 10,
            "fixture {}: ground truth {} carries only {words} words — \
             a trivial transcript cannot measure a WER gate",
            row.name,
            path.display()
        );
    }
}

#[test]
fn reference_ocr_outputs_exist() {
    let root = corpus_root();
    for row in manifest_rows() {
        let path = root.join(&row.reference_ocr);
        let text = fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!(
                "fixture {}: reference OCR {} unreadable (needed by \
                 `measure-wer.sh --recorded`): {e}",
                row.name,
                path.display()
            )
        });
        assert!(
            !text.trim().is_empty(),
            "fixture {}: reference OCR {} is empty — regenerate it per \
             tests/fixtures/scanned/GEN_MANIFEST.md",
            row.name,
            path.display()
        );
    }
}
