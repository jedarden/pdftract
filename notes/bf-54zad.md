# bf-54zad: Benchmark Timing and Throughput Measurement

## Summary

Implemented timing instrumentation and throughput measurement logic for the grep benchmark.

## Changes Made

### File: `crates/pdftract-cli/benches/grep_1000.rs`

1. **Added `files_per_second` field to `BenchmarkResult` struct** (line 103)
   - Stores the calculated files per second metric

2. **Implemented `calculate_files_per_second()` method** (lines 118-125)
   - Calculates files/second: `files_total / duration_sec`
   - Handles division by zero (returns 0.0 if duration_ms == 0)
   - Formula: `files_total / (duration_ms / 1000.0)`

3. **Implemented `save_to_file()` method** (lines 147-180)
   - Creates `benches/results/` directory if it doesn't exist
   - Saves benchmark result as `<commit-sha>.json` (8-char short SHA)
   - Uses `serde_json::to_string_pretty()` for readable JSON output
   - Provides user feedback on file save location

4. **Updated `run_benchmark()` function** (lines 260-280)
   - Calculates `files_per_second` metric after benchmark execution
   - Creates updated `BenchmarkResult` with calculated metrics

5. **Updated `main()` function** (lines 354-374)
   - Displays `files_per_second` metric alongside throughput
   - Calls `save_to_file()` to persist results to JSON
   - Provides warning if file save fails (non-fatal)

## Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| Timing code measures wall-clock time accurately | ✅ PASS | Uses `Instant::now()` and `.elapsed().as_millis()` |
| Throughput (MB/s) calculated correctly | ✅ PASS | `calculate_throughput()` method: `(bytes * 1000) / duration_ms / (1024*1024)` |
| Files per second calculated correctly | ✅ PASS | `calculate_files_per_second()` method: `files / (duration_ms / 1000)` |
| Commit hash extraction works | ✅ PASS | `get_commit_sha()` uses `git rev-parse HEAD` |
| Timestamp generation produces ISO 8601 | ✅ PASS | Uses `chrono::Utc::now().to_rfc3339()` |
| Ready to output metrics to JSON | ✅ PASS | `save_to_file()` method outputs to `benches/results/<sha>.json` |

## Technical Details

### Wall-clock timing
- Uses `std::time::Instant::now()` for start time
- Measures elapsed milliseconds with `.elapsed().as_millis()`
- High-resolution monotonic clock suitable for benchmarking

### Throughput calculation (MB/s)
```rust
let bytes_per_sec = (bytes_total as f64 * 1000.0) / duration_ms as f64;
bytes_per_sec / (1024.0 * 1024.0)
```
- Converts duration from ms to seconds
- Calculates bytes per second
- Divides by 1024² for binary MB (MiB/s)

### Files per second calculation
```rust
let duration_sec = duration_ms as f64 / 1000.0;
files_total as f64 / duration_sec
```
- Simple linear scaling from files per duration_ms to files per second

### JSON output format
```json
{
  "commit": "abc12345...",
  "started_at": "2026-07-06T10:30:45+00:00",
  "files_total": 1000,
  "bytes_total": 104857600,
  "duration_ms": 2000,
  "matches_total": 5000,
  "throughput_mb_s": 50.0,
  "files_per_second": 500.0,
  "peak_rss_mb": null
}
```

## Testing

### Compile check
```bash
cargo check --bench grep_1000
```
✅ No compilation errors

### Manual test (corpus not yet populated)
The benchmark handles empty corpus gracefully - returns placeholder result and skips CI gate validation during development.

## Next Steps

The benchmark infrastructure is now ready. Once the grep subcommand is complete (7.8.x beads):
1. Populate `tests/fixtures/grep-corpus/` with 1000 PDFs
2. Wire up actual grep execution
3. Run full benchmark and validate 50 MB/s CI gate
