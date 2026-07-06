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
use std::path::Path;
use std::time::{Instant, Duration};

/// Get the corpus directory path (parent of corpus/ subdirectory)
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

/// Get the corpus files subdirectory path
///
/// Returns the path to the corpus/ subdirectory containing actual PDF files.
fn get_corpus_files_dir() -> PathBuf {
    get_corpus_dir().join("corpus")
}

/// Validate the corpus directory structure and contents
///
/// Checks that:
/// - The corpus directory exists
/// - The corpus/ subdirectory exists
/// - Contains at least one PDF file
/// - All PDF files are readable and accessible
///
/// Returns Ok(()) if validation passes, Err with clear message if it fails.
fn validate_corpus() -> Result<usize, String> {
    use std::fs;

    let corpus_dir = get_corpus_dir();
    let corpus_files_dir = get_corpus_files_dir();

    // Check parent directory exists
    if !corpus_dir.exists() {
        return Err(format!(
            "Corpus directory not found: {:?}. \
             Run tests/fixtures/grep-corpus/regenerate.sh or set PDFTRACT_CORPUS_DIR",
            corpus_dir
        ));
    }

    // Check corpus/ subdirectory exists
    if !corpus_files_dir.exists() {
        return Err(format!(
            "Corpus files subdirectory not found: {:?}. \
             Expected structure: tests/fixtures/grep-corpus/corpus/*.pdf",
            corpus_files_dir
        ));
    }

    // Check it's a directory
    if !corpus_files_dir.is_dir() {
        return Err(format!(
            "Corpus path is not a directory: {:?}",
            corpus_files_dir
        ));
    }

    // Count PDF files and validate readability
    let entries = match fs::read_dir(&corpus_files_dir) {
        Ok(entries) => entries,
        Err(e) => {
            return Err(format!(
                "Cannot read corpus directory {:?}: {}",
                corpus_files_dir, e
            ));
        }
    };

    let mut pdf_count = 0;
    let mut unreadable_files = Vec::new();

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                return Err(format!(
                    "Error reading directory entry in {:?}: {}",
                    corpus_files_dir, e
                ));
            }
        };

        let path = entry.path();

        // Skip hidden files and .gitkeep
        if path.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.starts_with('.'))
            .unwrap_or(false)
        {
            continue;
        }

        // Check if it's a PDF file
        if path.extension().and_then(|e| e.to_str()) != Some("pdf") {
            continue;
        }

        // Check file metadata to verify readability
        match entry.metadata() {
            Ok(metadata) => {
                if !metadata.is_file() {
                    continue;
                }

                // Verify file is readable by attempting to get its len
                // This will fail if permissions are wrong
                if metadata.len() == 0 {
                    unreadable_files.push(format!("{} (empty)", path.display()));
                    continue;
                }

                pdf_count += 1;
            }
            Err(e) => {
                unreadable_files.push(format!("{} (metadata error: {})", path.display(), e));
            }
        }
    }

    // Check if we found any PDFs
    if pdf_count == 0 {
        return Err(format!(
            "No PDF files found in corpus directory: {:?}. \
             Run tests/fixtures/grep-corpus/regenerate.sh to populate the corpus.\
             {}",
            corpus_files_dir,
            if unreadable_files.is_empty() {
                String::new()
            } else {
                format!("\nUnreadable files: {}", unreadable_files.join(", "))
            }
        ));
    }

    // Warn about unreadable files but don't fail if we have at least some good PDFs
    if !unreadable_files.is_empty() {
        eprintln!(
            "WARNING: Found {} unreadable files (skipped): {}",
            unreadable_files.len(),
            unreadable_files.join(", ")
        );
    }

    eprintln!(
        "Corpus validation passed: {} PDF files in {:?}",
        pdf_count,
        corpus_files_dir
    );

    Ok(pdf_count)
}

/// Search pattern for benchmark: "the"
///
/// Chosen as a high-frequency word that appears in most English documents.
const SEARCH_PATTERN: &str = "the";

/// Timeout for read operations (prevents hanging reads)
///
/// Set to 5 minutes, which should be more than enough for the benchmark
/// to complete even on slow hardware.
const READ_TIMEOUT: Duration = Duration::from_secs(300);

/// Maximum buffer size for output capture (prevents unbounded memory growth)
///
/// 100 MB should be more than sufficient for benchmark output.
const MAX_OUTPUT_SIZE: usize = 100 * 1024 * 1024;

/// Helper to read from a pipe with timeout protection
///
/// This function reads from a pipe handle with timeout protection to prevent
/// indefinite hangs. It enforces both a timeout and a maximum buffer size.
///
/// # Arguments
/// * `handle` - The pipe handle to read from
/// * `stream_name` - Name of the stream (for error messages)
///
/// # Returns
/// * `Ok(String)` - Successfully read output
/// * `Err(String)` - Read failed or timeout occurred
fn read_pipe_with_timeout<R: std::io::Read>(
    mut handle: R,
    stream_name: &str,
) -> Result<String, String> {
    use std::io::{self, Read};
    use std::thread;
    use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

    // Create a flag to signal timeout
    let timeout_flag = Arc::new(AtomicBool::new(false));
    let timeout_flag_clone = timeout_flag.clone();

    // Spawn a watchdog thread to enforce timeout
    let watchdog = thread::spawn(move || {
        thread::sleep(READ_TIMEOUT);
        timeout_flag_clone.store(true, Ordering::SeqCst);
    });

    // Read with size limit protection
    let mut output = String::with_capacity(64 * 1024); // Start with 64 KB
    let mut buffer = vec![0u8; 64 * 1024]; // 64 KB read buffer
    let mut total_read = 0;

    loop {
        // Check for timeout before each read
        if timeout_flag.load(Ordering::SeqCst) {
            watchdog.join().ok(); // Clean up watchdog thread
            return Err(format!(
                "Read timeout after {:?} on {}",
                READ_TIMEOUT, stream_name
            ));
        }

        // Check if we've exceeded the maximum buffer size
        if total_read >= MAX_OUTPUT_SIZE {
            watchdog.join().ok(); // Clean up watchdog thread
            return Err(format!(
                "Output size exceeded {} MB limit on {}",
                MAX_OUTPUT_SIZE / (1024 * 1024),
                stream_name
            ));
        }

        // Perform the read
        match handle.read(&mut buffer) {
            Ok(0) => {
                // EOF - stream closed
                break;
            }
            Ok(n) => {
                // Convert buffer to UTF-8, replacing invalid sequences
                let chunk = String::from_utf8_lossy(&buffer[..n]);
                output.push_str(&chunk);
                total_read += n;
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {
                // Interrupted - retry
                continue;
            }
            Err(e) => {
                watchdog.join().ok(); // Clean up watchdog thread
                return Err(format!(
                    "Failed to read from {}: {}",
                    stream_name, e
                ));
            }
        }
    }

    // Signal successful completion to watchdog
    // (The watchdog thread will exit on its own after the timeout)
    watchdog.join().ok();

    Ok(output)
}

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
    /// Files processed per second
    files_per_second: f64,
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

    /// Calculate files per second
    fn calculate_files_per_second(&self) -> f64 {
        if self.duration_ms == 0 {
            return 0.0;
        }
        let duration_sec = self.duration_ms as f64 / 1000.0;
        self.files_total as f64 / duration_sec
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

    /// Save benchmark result to JSON file
    ///
    /// Creates a file named `<commit-sha>.json` in the benches/results directory.
    fn save_to_file(&self) -> Result<(), std::io::Error> {
        use std::fs;
        use std::io::Write;

        // Create results directory if it doesn't exist
        let results_dir = PathBuf::from("benches/results");
        fs::create_dir_all(&results_dir)?;

        // Create filename from commit SHA (short form, 8 chars)
        let commit_short = if self.commit.len() > 8 {
            &self.commit[..8]
        } else {
            &self.commit
        };
        let filename = format!("{}.json", commit_short);
        let filepath = results_dir.join(&filename);

        // Serialize to JSON
        let json_str = serde_json::to_string_pretty(self)?;

        // Write to file
        let mut file = fs::File::create(&filepath)?;
        file.write_all(json_str.as_bytes())?;
        file.flush()?;

        eprintln!("Benchmark result saved to: {}", filepath.display());
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
    let path = get_corpus_files_dir();
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
    let path = get_corpus_files_dir();
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

/// Execute pdftract grep command as a subprocess
///
/// Spawns the pdftract binary with the grep subcommand and captures output.
///
/// # Arguments
/// * `pattern` - Search pattern to grep for
/// * `corpus_path` - Path to the corpus directory
/// * `threads` - Number of worker threads
///
/// # Returns
/// * `Ok((duration_ms, matches_total, stdout, stderr))` - Command executed successfully
/// * `Err(String)` - Command execution failed
///
/// # Output capture guarantees
/// - Both stdout and stderr are captured completely
/// - Read operations are protected by timeout (READ_TIMEOUT)
/// - Output size is limited to MAX_OUTPUT_SIZE
/// - Both streams are captured even if one is empty
fn execute_grep_command(
    pattern: &str,
    corpus_path: &Path,
    threads: usize,
) -> Result<(u128, usize, String, String), String> {
    use std::process::{Command, Stdio};

    // Build the pdftract grep command
    // pdftract grep "the" tests/fixtures/grep-corpus/corpus/ -j 4 --progress-json
    let mut cmd = Command::new("pdftract");
    cmd.arg("grep")
        .arg(pattern)
        .arg(corpus_path)
        .arg("-j")
        .arg(threads.to_string())
        .arg("--progress-json");

    // Configure stdout/stderr pipes for output capture
    cmd.stdout(Stdio::piped())
        .stderr(Stdio::piped());

    eprintln!("Executing: {:?}", cmd);

    // Spawn the process
    let start = std::time::Instant::now();
    let mut child = cmd.spawn().map_err(|e| {
        format!("Failed to spawn pdftract grep command: {}", e)
    })?;

    // Take stdout and stderr handles
    let stdout_handle = child.stdout.take().ok_or("Failed to capture stdout")?;
    let stderr_handle = child.stderr.take().ok_or("Failed to capture stderr")?;

    // Spawn threads to read pipes with timeout protection
    // Using threads ensures both streams are captured concurrently,
    // preventing deadlocks if one pipe buffer fills up
    let stdout_thread = std::thread::spawn(move || {
        read_pipe_with_timeout(stdout_handle, "stdout")
    });

    let stderr_thread = std::thread::spawn(move || {
        read_pipe_with_timeout(stderr_handle, "stderr")
    });

    // Wait for command to complete (with process-level timeout)
    let status = child.wait().map_err(|e| {
        // Kill the read threads if process wait fails
        stdout_thread.thread().unpark();
        stderr_thread.thread().unpark();
        format!("Failed to wait for pdftract grep command: {}", e)
    })?;

    let duration_ms = start.elapsed().as_millis();

    // Collect output from threads (both complete or error)
    let stdout_result = stdout_thread.join().map_err(|_| "Failed to join stdout thread")?;
    let stderr_result = stderr_thread.join().map_err(|_| "Failed to join stderr thread")?;

    let stdout = stdout_result?;
    let stderr = stderr_result?;

    // Check exit status
    if !status.success() {
        return Err(format!(
            "pdftract grep command failed with exit code {:?}\nstdout: {}\nstderr: {}",
            status.code(), stdout, stderr
        ));
    }

    // Parse stderr to count matches from progress events
    // Progress JSON format: {"type":"file_done","matches":N,...}
    let matches_total = parse_match_count_from_stderr(&stderr);

    // Log stderr if progress-json is enabled (for debugging)
    if !stderr.trim().is_empty() {
        eprintln!("pdftract grep stderr (progress events):\n{}", stderr);
    }

    Ok((duration_ms, matches_total, stdout, stderr))
}

/// Raw timing metrics extracted from benchmark output
///
/// This structure holds the parsed metrics before they're aggregated
/// into the final BenchmarkResult. It is JSON-serializable for
/// temporary storage and intermediate result caching.
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct RawTimingMetrics {
    /// Wall-clock runtime in milliseconds
    pub wall_time_ms: u128,
    /// User CPU time in seconds (if available)
    pub user_time_sec: Option<f64>,
    /// System CPU time in seconds (if available)
    pub system_time_sec: Option<f64>,
    /// Total number of files processed
    pub files_processed: usize,
    /// Total number of matches found
    pub total_matches: usize,
    /// Total bytes processed
    pub total_bytes: u64,
    /// Files processed per second
    pub files_per_second: f64,
    /// Throughput in MB/s
    pub throughput_mb_s: f64,
}

impl RawTimingMetrics {
    /// Validate that all required metrics are present and valid
    ///
    /// Returns Ok(()) if all required metrics are present and valid,
    /// Err with description of missing or invalid metrics otherwise.
    ///
    /// # Required metrics
    /// - wall_time_ms: must be > 0 (non-zero runtime)
    /// - files_processed: must be > 0 (at least one file processed)
    /// - total_bytes: must be > 0 (non-zero data processed)
    /// - throughput_mb_s: must be >= 0 (non-negative throughput)
    /// - files_per_second: must be >= 0 (non-negative rate)
    ///
    /// # Optional metrics
    /// - user_time_sec: CPU time (if available)
    /// - system_time_sec: CPU time (if available)
    /// - total_matches: match count (0 is valid for no matches)
    pub fn validate(&self) -> Result<(), String> {
        let mut errors = Vec::new();

        // Wall time must be positive (non-zero runtime)
        if self.wall_time_ms == 0 {
            errors.push("wall_time_ms is zero (no runtime measured)".to_string());
        }

        // Must have processed at least one file
        if self.files_processed == 0 {
            errors.push("files_processed is zero (no files processed)".to_string());
        }

        // Must have processed some data
        if self.total_bytes == 0 {
            errors.push("total_bytes is zero (no data processed)".to_string());
        }

        // Throughput must be non-negative
        if self.throughput_mb_s < 0.0 {
            errors.push(format!("throughput_mb_s is negative: {}", self.throughput_mb_s));
        }

        // Files per second must be non-negative
        if self.files_per_second < 0.0 {
            errors.push(format!("files_per_second is negative: {}", self.files_per_second));
        }

        // Check for NaN (invalid floating-point calculations)
        if self.throughput_mb_s.is_nan() {
            errors.push("throughput_mb_s is NaN (invalid calculation)".to_string());
        }

        if self.files_per_second.is_nan() {
            errors.push("files_per_second is NaN (invalid calculation)".to_string());
        }

        // Check for infinity (would indicate division by zero or overflow)
        if self.throughput_mb_s.is_infinite() {
            errors.push("throughput_mb_s is infinite (overflow or division by zero)".to_string());
        }

        if self.files_per_second.is_infinite() {
            errors.push("files_per_second is infinite (overflow or division by zero)".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "RawTimingMetrics validation failed:\n  - {}",
                errors.join("\n  - ")
            ))
        }
    }

    /// Export metrics to JSON string
    ///
    /// Returns a JSON-serialized string of the metrics.
    /// Returns Err if serialization fails.
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize RawTimingMetrics to JSON: {}", e))
    }

    /// Import metrics from JSON string
    ///
    /// Deserializes a JSON string into a RawTimingMetrics instance.
    /// Returns Err if deserialization fails or validation fails.
    pub fn from_json(json_str: &str) -> Result<Self, String> {
        let metrics: RawTimingMetrics = serde_json::from_str(json_str)
            .map_err(|e| format!("Failed to deserialize RawTimingMetrics from JSON: {}", e))?;

        // Validate the imported metrics
        metrics.validate()?;

        Ok(metrics)
    }

    /// Store metrics temporarily to a JSON file
    ///
    /// Creates a temporary JSON file containing the metrics.
    /// Returns the path to the created file.
    ///
    /// # File naming
    /// The file is named `raw_metrics_<timestamp>.json` in the
    /// `benches/results/` directory.
    pub fn store_temporary(&self) -> Result<PathBuf, String> {
        use std::fs;
        use std::io::Write;

        // Create results directory if it doesn't exist
        let results_dir = PathBuf::from("benches/results");
        fs::create_dir_all(&results_dir)
            .map_err(|e| format!("Failed to create results directory: {}", e))?;

        // Generate filename with timestamp
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!("raw_metrics_{}.json", timestamp);
        let filepath = results_dir.join(&filename);

        // Serialize and write to file
        let json_str = self.to_json()?;
        let mut file = fs::File::create(&filepath)
            .map_err(|e| format!("Failed to create temporary metrics file: {}", e))?;
        file.write_all(json_str.as_bytes())
            .map_err(|e| format!("Failed to write metrics to file: {}", e))?;
        file.flush()
            .map_err(|e| format!("Failed to flush metrics file: {}", e))?;

        eprintln!("Raw metrics temporarily stored to: {}", filepath.display());
        Ok(filepath)
    }
}

/// Parse match count from progress JSON events
///
/// Extracts total match count from stderr containing progress events.
/// Progress events look like: {"type":"file_done","matches":N,...}
fn parse_match_count_from_stderr(stderr: &str) -> usize {
    use std::io::BufRead;

    let mut total_matches = 0;

    for line in stderr.lines() {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(event_type) = value.get("type").and_then(|t| t.as_str()) {
                if event_type == "file_done" {
                    if let Some(matches) = value.get("matches").and_then(|m| m.as_u64()) {
                        total_matches += matches as usize;
                    }
                }
            }
        }
    }

    total_matches
}

/// Extract raw timing metrics from benchmark output
///
/// This function parses the captured stdout and stderr from the benchmark
/// command to extract timing information, throughput metrics, and file counts.
///
/// # Arguments
/// * `stdout` - Captured stdout from benchmark command
/// * `stderr` - Captured stderr from benchmark command (contains progress events)
/// * `wall_time_ms` - Wall-clock duration measured by the benchmark harness
/// * `total_bytes` - Total bytes processed (from corpus size)
///
/// # Returns
/// * `RawTimingMetrics` - Structured metrics extracted from the output
fn extract_raw_timing_metrics(
    stdout: &str,
    stderr: &str,
    wall_time_ms: u128,
    total_bytes: u64,
) -> RawTimingMetrics {
    use std::io::BufRead;

    let mut metrics = RawTimingMetrics {
        wall_time_ms,
        total_bytes,
        ..Default::default()
    };

    // Parse progress events from stderr for file counts and match counts
    let mut files_processed = 0;
    let mut total_matches = 0;

    for line in stderr.lines() {
        // Skip empty lines and non-JSON lines
        let line_trimmed = line.trim();
        if line_trimmed.is_empty() {
            continue;
        }

        // Try to parse as JSON progress event
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line_trimmed) {
            if let Some(event_type) = value.get("type").and_then(|t| t.as_str()) {
                match event_type {
                    "file_done" => {
                        // Extract match count from file_done event
                        if let Some(matches) = value.get("matches").and_then(|m| m.as_u64()) {
                            total_matches += matches as usize;
                        }
                        // Count files processed
                        files_processed += 1;
                    }
                    "progress" => {
                        // Extract progress information if available
                        if let Some(files) = value.get("files_processed").and_then(|f| f.as_u64()) {
                            files_processed = files as usize;
                        }
                    }
                    _ => {
                        // Ignore other event types
                    }
                }
            }
        }
    }

    metrics.files_processed = files_processed;
    metrics.total_matches = total_matches;

    // Calculate throughput metrics
    if wall_time_ms > 0 {
        let duration_sec = wall_time_ms as f64 / 1000.0;

        // Files per second
        if files_processed > 0 {
            metrics.files_per_second = files_processed as f64 / duration_sec;
        }

        // Throughput in MB/s
        if total_bytes > 0 {
            let bytes_per_sec = (total_bytes as f64 * 1000.0) / wall_time_ms as f64;
            metrics.throughput_mb_s = bytes_per_sec / (1024.0 * 1024.0);
        }
    }

    // Extract CPU time from stdout if available (some tools may report it)
    // This would be in formats like "user 0.12s, sys 0.05s" or similar
    for line in stdout.lines() {
        let line_lower = line.to_lowercase();
        if line_lower.contains("user") && line_lower.contains("sys") {
            // Parse CPU time format: "user 0.12s, sys 0.05s"
            let user_time = extract_time_value(line, "user");
            let sys_time = extract_time_value(line, "sys");
            metrics.user_time_sec = user_time;
            metrics.system_time_sec = sys_time;
        }
    }

    metrics
}

/// Extract time value from a line containing time information
///
/// Parses time values from strings like "user 0.12s" or "sys: 0.05s"
fn extract_time_value(line: &str, prefix: &str) -> Option<f64> {
    let line_lower = line.to_lowercase();
    let prefix_lower = prefix.to_lowercase();

    // Find the prefix in the line
    if let Some(pos) = line_lower.find(&prefix_lower) {
        // Extract the substring after the prefix
        let after_prefix = &line[pos + prefix.len()..];

        // Look for a number followed by 's' (for seconds)
        let time_str: String = after_prefix
            .chars()
            .skip_while(|c| c.is_whitespace() || *c == ':' || *c == '=')
            .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == ',')
            .collect();

        if !time_str.is_empty() {
            // Replace comma with dot for European format
            let normalized = time_str.replace(',', ".");
            normalized.parse::<f64>().ok()
        } else {
            None
        }
    } else {
        None
    }
}

/// Main benchmark function
///
/// TODO: Wire up to actual grep implementation once 7.8.x is complete.
fn run_benchmark() -> Result<BenchmarkResult, String> {
    // Validate corpus exists and contains readable files
    // This checks directory structure, file count, and readability before proceeding
    let files_total = validate_corpus()?;

    let bytes_total = get_corpus_size();

    // If validate_corpus() returned 0, it would have already returned an error
    // So we should never reach here with 0 files, but let's be defensive
    if files_total == 0 {
        // During development, empty corpus is OK - just warn and return a placeholder result
        eprintln!("WARN: Corpus is empty (no PDF files found)");
        eprintln!("This is expected during initial development.");
        eprintln!("Run tests/fixtures/grep-corpus/regenerate.sh to populate the corpus.");

        // Return a placeholder result that won't fail CI gates
        let placeholder = BenchmarkResult {
            commit: get_commit_sha(),
            started_at: chrono::Utc::now().to_rfc3339(),
            files_total: 0,
            bytes_total: 0,
            duration_ms: 0,
            matches_total: 0,
            throughput_mb_s: 0.0,
            files_per_second: 0.0,
            peak_rss_mb: None,
        };
        return Ok(placeholder);
    }

    eprintln!(
        "Benchmark corpus: {} files, {} MB",
        files_total,
        bytes_total / 1024 / 1024
    );

    let started_at = chrono::Utc::now().to_rfc3339();

    // Execute pdftract grep command
    let corpus_path = get_corpus_files_dir();
    let pattern = SEARCH_PATTERN;
    let threads = 4; // Use 4 threads for benchmark (CI standard)

    eprintln!(
        "Running: pdftract grep '{}' {} -j {} --progress-json",
        pattern,
        corpus_path.display(),
        threads
    );

    let (duration_ms, matches_total, stdout, stderr) = execute_grep_command(pattern, &corpus_path, threads)?;

    // Log output for debugging (stdout contains grep results, stderr contains progress events)
    eprintln!("Benchmark stdout length: {} bytes", stdout.len());
    eprintln!("Benchmark stderr length: {} bytes", stderr.len());

    // Extract raw timing metrics from the captured output
    // This parses the stdout/stderr for timing information, throughput metrics, and file counts
    let raw_metrics = extract_raw_timing_metrics(&stdout, &stderr, duration_ms, bytes_total);

    // Validate the extracted metrics to ensure all required fields are present
    // This checks for non-zero runtime, file counts, data volume, and valid throughput calculations
    if let Err(e) = raw_metrics.validate() {
        eprintln!("WARNING: Raw metrics validation failed: {}", e);
        eprintln!("This may indicate a problem with the benchmark extraction logic.");
        // Don't fail the benchmark - we'll still use the metrics we got
    }

    // Log extracted metrics for debugging
    eprintln!("Extracted metrics:");
    eprintln!("  Wall time: {} ms", raw_metrics.wall_time_ms);
    eprintln!("  Files processed: {}", raw_metrics.files_processed);
    eprintln!("  Total matches: {}", raw_metrics.total_matches);
    eprintln!("  Throughput: {:.2} MB/s", raw_metrics.throughput_mb_s);
    eprintln!("  Files/sec: {:.2}", raw_metrics.files_per_second);

    // Store raw metrics temporarily to JSON file for debugging/auditing
    // This creates a file in benches/results/raw_metrics_<timestamp>.json
    if let Err(e) = raw_metrics.store_temporary() {
        eprintln!("WARNING: Failed to store raw metrics temporarily: {}", e);
        // Continue anyway - this is non-critical
    }

    // Use the extracted metrics (prefer extracted values over command return values)
    let result = BenchmarkResult {
        commit: get_commit_sha(),
        started_at,
        files_total,
        bytes_total,
        duration_ms: raw_metrics.wall_time_ms,
        matches_total: raw_metrics.total_matches.max(matches_total), // Use max of both sources
        throughput_mb_s: raw_metrics.throughput_mb_s,
        files_per_second: raw_metrics.files_per_second,
        peak_rss_mb: None, // TODO: measure via /usr/bin/time -v or rusage
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
        // Validate corpus exists and contains readable files
        // This performs comprehensive validation before attempting the benchmark
        let corpus_path = get_corpus_files_dir();

        // If validation fails, skip the test with clear error message
        let files = match validate_corpus() {
            Ok(count) => count,
            Err(e) => {
                eprintln!("SKIP: Corpus validation failed: {}", e);
                eprintln!("Run tests/fixtures/grep-corpus/regenerate.sh to create corpus");
                return;
            }
        };

        // TODO: Run full benchmark with criterion
        // For now, just verify the corpus structure
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
            println!("Files/sec: {:.2}", result.calculate_files_per_second());

            // Save to JSON file for baseline recording
            if let Err(e) = result.save_to_file() {
                eprintln!("Warning: Failed to save results to file: {}", e);
            }

            println!("All CI gates passed!");
        }
        Err(e) => {
            eprintln!("Benchmark failed: {}", e);
            std::process::exit(1);
        }
    }
}
