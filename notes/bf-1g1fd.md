# Memory Ceiling Gate Implementation (bf-1g1fd)

## Summary

Implemented a Tier-1 memory ceiling gate that enforces RSS budgets for PDF extraction, analogous to cargo-bloat for binary size. The gate samples peak RSS while extracting perf + malformed corpora and fails the build if any document exceeds its budget.

## Changes Made

### 1. Expanded xtask memory-ceiling command

**File:** `xtask/src/main.rs`

- Added support for three memory budget categories:
  - Buffered 100-page vector PDF: 512 MB
  - Streaming/NDJSON mode (any page count): 256 MB
  - Adversarial fixtures: 1 GB hard ceiling
- Added streaming mode testing with `--format ndjson`
- Generates JSON report (`memory-report.json`) with:
  - Per-document results (peak RSS, duration, budget, pass/fail)
  - Summary statistics
  - Commit SHA for historical tracking
- Added `MemoryTestResult`, `MemoryReport`, `MemoryBudgetJson`, `MemorySummary` structs

**File:** `xtask/Cargo.toml`

- Added `serde_json` dependency for JSON output
- Added `humantime` dependency for timestamp formatting

### 2. Updated CI memory-ceiling template

**File:** `.ci/argo-workflows/pdftract-ci.yaml`

- Added cgroup MemoryMax enforcement (1.5 GB cap) for clean failure mode
- Supports both cgroup v2 (preferred) and cgroup v1
- Falls back gracefully when cgroup unavailable
- Uses xtask-generated `memory-report.json` for artifact upload
- Shows summary from report in CI logs

### 3. Updated fuzz workflow with cgroup enforcement

**File:** `.ci/argo-workflows/pdftract-nightly-fuzz.yaml`

- Added cgroup MemoryMax enforcement (1.5 GB cap) to fuzz-target template
- Layered memory enforcement:
  - Cgroup MemoryMax: 1536 MB (hard ceiling on entire fuzz run)
  - Libfuzzer `-rss_limit_mb=1024` (per-execution RSS cap)
  - Libfuzzer `-malloc_limit_mb=1024` (total malloc cap)
- Supports both cgroup v2 (preferred) and cgroup v1
- Falls back to libfuzzer limits when cgroup unavailable

## Acceptance Criteria

### PASS

- [x] Harness samples peak RSS while extracting perf + malformed corpora
- [x] Build fails if any document exceeds its memory budget
- [x] Test suite runs under cgroup MemoryMax cap (1.5 GB)
- [x] Fuzz suite runs under cgroup MemoryMax cap (1.5 GB)
- [x] Libfuzzer `-rss_limit_mb=1024` and `-malloc_limit_mb=1024` set
- [x] Memory targets are now Tier-1 gates

### WARN (environmental issues)

None - all infrastructure (cgroups, libfuzzer limits) is standard CI environment

### FAIL

None

## Implementation Notes

### Cgroup Support

The implementation supports both cgroup v2 (preferred) and cgroup v1:
- Cgroup v2: Uses `/sys/fs/cgroup/` with `memory.max` controller
- Cgroup v1: Uses `/sys/fs/cgroup/memory/` with `memory.limit_in_bytes`
- Falls back to libfuzzer limits when cgroup unavailable

### Memory Budgets

Per plan.md line 72-80:

| Category | Budget | Measurement |
|----------|--------|-------------|
| Peak RSS, 100-page vector PDF (buffered mode) | < 512 MB | `tests/fixtures/perf/` |
| Peak RSS, streaming/NDJSON mode (any page count) | < 256 MB | `tests/fixtures/perf/` with `--format ndjson` |
| Peak RSS, adversarial fixtures | < 1 GB | `tests/fixtures/malformed/` |

### RSS Sampling

The xtask `measure_extraction` function:
- Spawns pdftract as a child process
- Samples `/proc/[pid]/status` every 10 ms for `VmRSS` field
- Tracks peak RSS across the extraction run
- Works on Linux; falls back to time-only measurement on other platforms

### JSON Report Format

The `memory-report.json` artifact includes:
```json
{
  "timestamp": "2026-05-23T12:34:56Z",
  "commit_sha": "abc123...",
  "budgets": {
    "buffered_100_page_mb": 512,
    "streaming_any_mb": 256,
    "adversarial_hard_cap_mb": 1024
  },
  "results": [
    {
      "file_name": "example.pdf",
      "category": "buffered",
      "peak_rss_mb": 123,
      "duration_ms": 456,
      "budget_mb": 512,
      "passed": true,
      "error_message": null
    }
  ],
  "summary": {
    "total_tests": 10,
    "passed": 10,
    "failed": 0,
    "all_passed": true
  }
}
```

## Testing

To test locally:
```bash
# Run memory ceiling tests
cargo run --release --bin xtask -- memory-ceiling

# Run fuzz tests with memory limits
bash scripts/run-fuzz-with-limits.sh [target]
```

## References

- Plan section: Phase 0.4 Quality Targets - Memory targets (lines 72-80)
- Bead: bf-1g1fd
- CI template: `.ci/argo-workflows/pdftract-ci.yaml` (memory-ceiling template)
- Fuzz workflow: `.ci/argo-workflows/pdftract-nightly-fuzz.yaml` (fuzz-target template)
