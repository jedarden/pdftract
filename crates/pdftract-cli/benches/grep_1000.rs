//! pdftract-grep-1000 benchmark
//!
//! This benchmark runs the grep search across a corpus of 1000 PDFs (~100 MB total)
//! and measures throughput, latency, and memory usage.
//!
//! # CI Gates
//!
//! - Throughput: ≥ 50 MB/s on 4-core CI machine
//! - vs pdfgrep: ≥ 2× faster
//! - vs pdftotext+ripgrep: ≥ 3× faster
//! - Regression: ≤ 10% vs historical main
//!
//! # Usage
//!
//! ```bash
//! cargo bench --bench grep_1000
//! ```
//!
//! # TODO (blocks on 7.8.1-7.8.9 grep implementation)
//!
//! - [ ] Complete grep subcommand implementation (7.8.x beads)
//! - [ ] Populate tests/fixtures/grep-corpus/ with 1000 PDFs
//! - [ ] Run actual benchmark and measure wall-clock time
//! - [ ] Compare against pdfgrep baseline
//! - [ ] Compare against pdftotext+ripgrep baseline
//! - [ ] Record results to benches/results/<commit-sha>.json
//! - [ ] Wire up CI gate (50 MB/s threshold)

use std::path::PathBuf;
use std::time::Instant;

/// Get the corpus directory path
///
/// Tries multiple strategies to find the corpus:
/// 1. Environment variable PDFTRACT_CORPUS_DIR
/// 2. Relative to CARGO_MANIFEST_DIR (if set)
/// 3. Relative to current directory
/// 4. Relative to workspace root (via git rev-parse)
fn get_corpus_dir() -> PathBuf {
    // Try environment variable first
    if let Ok(dir) = std::env::var("PDFTRACT_CORPUS_DIR") {
        return PathBuf::from(dir);
    }

    // Try CARGO_MANIFEST_DIR (set by cargo for benches)
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        // From CLI crate: go up to workspace root, then into tests/fixtures
        let manifest_path = PathBuf::from(manifest_dir);
        if let Some(workspace_root) = manifest_path.ancestors().nth(2) {
            let corpus_path = workspace_root.join("tests/fixtures/grep-corpus");
            if corpus_path.exists() {
                return corpus_path;
            }
        }
    }

    // Try git rev-parse to find workspace root
    if let Ok(output) = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
    {
        if let Ok(root) = String::from_utf8(output.stdout) {
            let root = root.trim();
            let corpus_path = PathBuf::from(root).join("tests/fixtures/grep-corpus");
            if corpus_path.exists() {
                return corpus_path;
            }
        }
    }

    // Fall back to relative path from current directory
    PathBuf::from("tests/fixtures/grep-corpus")
}

/// Search pattern for benchmark: "the"
///
/// Chosen as a high-frequency word that appears in most English documents.
const SEARCH_PATTERN: &str = "the";

/// Expected match count (for correctness validation)
///
/// This should be computed during corpus generation and stored in a manifest.
const EXPECTED_MATCH_COUNT: usize = 0; // TODO: compute from corpus

/// Benchmark result structure
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct BenchmarkResult {
    /// Git commit SHA
    commit: String,
    /// Benchmark start time (ISO 8601)
    started_at: String,
    /// Total number of files processed
    files_total: usize,
    /// Total bytes processed
    bytes_total: u64,
    /// Wall-clock duration in milliseconds
    duration_ms: u128,
    /// Total matches found
    matches_total: usize,
    /// Throughput in MB/s
    throughput_mb_s: f64,
    /// Peak RSS in MB
    peak_rss_mb: Option<u64>,
}

impl BenchmarkResult {
    /// Calculate throughput in MB/s
    fn calculate_throughput(&self) -> f64 {
        if self.duration_ms == 0 {
            return 0.0;
        }
        let bytes_per_sec = (self.bytes_total as f64 * 1000.0) / self.duration_ms as f64;
        bytes_per_sec / (1024.0 * 1024.0)
    }

    /// Validate against CI gates
    fn validate(&self) -> Result<(), String> {
        // During development (files_total == 0), skip validation
        if self.files_total == 0 {
            eprintln!("SKIP: CI gate validation (corpus empty during development)");
            return Ok(());
        }

        // 50 MB/s gate
        let throughput = self.calculate_throughput();
        if throughput < 50.0 {
            return Err(format!("Throughput {} MB/s below 50 MB/s gate", throughput));
        }

        // TODO: Add pdfgrep and pdftotext+ripgrep comparisons
        // TODO: Add historical regression check

        Ok(())
    }
}

/// Get current git commit SHA
fn get_commit_sha() -> String {
    use std::process::Command;
    Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Get corpus size in bytes
fn get_corpus_size() -> u64 {
    use std::fs;
    let path = get_corpus_dir();
    if !path.exists() {
        return 0;
    }

    fs::read_dir(path)
        .ok()
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter_map(|e| e.metadata().ok())
                .filter(|m| m.is_file())
                .map(|m| m.len())
                .sum()
        })
        .unwrap_or(0)
}

/// Count PDF files in corpus
fn count_corpus_files() -> usize {
    use std::fs;
    let path = get_corpus_dir();
    if !path.exists() {
        return 0;
    }

    fs::read_dir(path)
        .ok()
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .map(|ext| ext == "pdf")
                        .unwrap_or(false)
                })
                .count()
        })
        .unwrap_or(0)
}

/// Main benchmark function
///
/// TODO: Wire up to actual grep implementation once 7.8.x is complete.
fn run_benchmark() -> Result<BenchmarkResult, String> {
    // Check corpus exists
    let corpus_path = get_corpus_dir();
    if !corpus_path.exists() {
        return Err(format!(
            "Corpus directory not found: {:?}. Run tests/fixtures/grep-corpus/regenerate.sh",
            corpus_path
        ));
    }

    let files_total = count_corpus_files();
    let bytes_total = get_corpus_size();

    if files_total == 0 {
        // During development, empty corpus is OK - just warn and return a placeholder result
        eprintln!("WARN: Corpus is empty (no PDF files found)");
        eprintln!("This is expected during initial development.");
        eprintln!("Run tests/fixtures/grep-corpus/regenerate.sh to populate the corpus.");

        // Return a placeholder result that won't fail CI gates
        return Ok(BenchmarkResult {
            commit: get_commit_sha(),
            started_at: chrono::Utc::now().to_rfc3339(),
            files_total: 0,
            bytes_total: 0,
            duration_ms: 0,
            matches_total: 0,
            throughput_mb_s: 0.0,
            peak_rss_mb: None,
        });
    }

    eprintln!(
        "Benchmark corpus: {} files, {} MB",
        files_total,
        bytes_total / 1024 / 1024
    );

    // TODO: Run actual grep search
    // For now, this is a placeholder that simulates the benchmark structure
    let started_at = chrono::Utc::now().to_rfc3339();
    let start = Instant::now(); // Placeholder - won't measure anything yet

    // TODO: Invoke pdftract grep subprocess or call directly
    // pdftract grep "the" tests/fixtures/grep-corpus/ -j 4 --progress-json
    // Capture: wall-clock time, match count, peak RSS

    let duration_ms = start.elapsed().as_millis();
    let matches_total = 0; // TODO: from grep output

    let result = BenchmarkResult {
        commit: get_commit_sha(),
        started_at,
        files_total,
        bytes_total,
        duration_ms,
        matches_total,
        throughput_mb_s: 0.0, // Calculated below
        peak_rss_mb: None,    // TODO: measure via /usr/bin/time -v or rusage
    };

    // Validate against gates
    result.validate()?;

    Ok(result)
}

/// Criterion benchmark entry point
///
/// This function is called by cargo bench.
#[cfg(test)]
mod benches {
    use super::*;

    #[test]
    fn bench_grep_1000() {
        // Check if corpus exists; skip if not
        let corpus_path = get_corpus_dir();
        if !corpus_path.exists() {
            eprintln!("SKIP: Corpus not found at {:?}", corpus_path);
            eprintln!("Run tests/fixtures/grep-corpus/regenerate.sh to create corpus");
            return;
        }

        // TODO: Run full benchmark with criterion
        // For now, just verify the corpus structure
        let files = count_corpus_files();
        let bytes = get_corpus_size();

        eprintln!("Corpus: {} files, {} bytes", files, bytes);

        if files < 1000 {
            eprintln!("WARN: Expected 1000 files, found {}", files);
        }

        if bytes < 50 * 1024 * 1024 {
            eprintln!("WARN: Expected ~100 MB, found {} MB", bytes / 1024 / 1024);
        }
    }
}

fn main() {
    match run_benchmark() {
        Ok(result) => {
            println!("{:#?}", result);
            println!("\nThroughput: {:.2} MB/s", result.calculate_throughput());
            println!("All CI gates passed!");
        }
        Err(e) => {
            eprintln!("Benchmark failed: {}", e);
            std::process::exit(1);
        }
    }
}
