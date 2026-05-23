# bf-5dnh1: Memory ceiling for fuzz and proptests

## Summary

Implemented local memory ceiling enforcement for property tests to coordinate with the CI memory-ceiling gate (bf-1g1fd).

## Work Completed

### 1. Created `scripts/run-proptest-with-limits.sh`

A bash script that runs proptest modules under cgroup MemoryMax, providing local development parity with CI enforcement.

**Features:**
- Cgroup v2 MemoryMax support (preferred) with fallback to cgroup v1
- Configurable memory limit (default: 2048 MB)
- Per-module or full test suite execution
- Proptest seed configuration for reproducibility
- Clean failure mode (allocation errors, not host OOM)

**Usage:**
```bash
# Run all proptests with 2 GB limit
scripts/run-proptest-with-limits.sh

# Run single module
scripts/run-proptest-with-limits.sh lexer

# Custom memory limit
MEMORY_MAX_MB=4096 scripts/run-proptest-with-limits.sh
```

### 2. Created `scripts/README.md`

Documentation for the scripts directory, including:
- Memory ceiling enforcement explanation
- Fuzz and proptest script usage
- Environment variable configuration
- Rationale for memory ceilings

### 3. Verified existing protections

**Fuzz tests** (already in place from bf-1g1fd):
- CI: `.ci/argo-workflows/pdftract-nightly-fuzz.yaml` has cgroup MemoryMax (1536 MB) + libfuzzer RSS limits (1024 MB)
- Local: `scripts/run-fuzz-with-limits.sh` provides equivalent enforcement

**Proptests** (now complete):
- CI: `.ci/argo-workflows/pdftract-ci.yaml` has cgroup MemoryMax (6 GB glibc, 4 GB musl)
- Local: `scripts/run-proptest-with-limits.sh` provides equivalent enforcement

**Input size caps** (already in place):
- Lexer/object parser: `0..10_000` bytes
- Xref/stream parsers: `0..100_000` bytes
- Nested structures: depth-limited (e.g., 500 for parser depth checks)

## Acceptance Criteria

| Criterion | Status | Notes |
|-----------|--------|-------|
| Fuzz tests run under memory ceiling | PASS | Already in place from bf-1g1fd |
| Proptests run under memory ceiling | PASS | New script provides local enforcement |
| Per-process memory limit enforced | PASS | Cgroup MemoryMax wrapper |
| Proptest input sizes capped | PASS | Existing bounds in proptest code |
| Coordinated with bf-1g1fd | PASS | References bf-1g1fd in commit and docs |

## Retrospective

**What worked:**
- Leveraged existing cgroup patterns from `run-fuzz-with-limits.sh`
- Clean separation between CI (k8s cgroup) and local (system cgroup) enforcement
- Documentation in `scripts/README.md` provides clear rationale

**What didn't:**
- Initial attempt to use `setrlimit RLIMIT_AS` per the bead description was not practical for Rust's proptest harness (no easy hook point)
- Cgroup wrapper is more portable and aligns with existing bf-1g1fd implementation

**Surprise:**
- The fuzz job already had comprehensive memory limits from bf-1g1fd; this bead primarily needed to add local proptest parity

**Reusable pattern:**
- For future test harnesses that need memory limits: use cgroup v2 MemoryMax with cgroup v1 fallback, disable OOM killer for clean failures
- Document both CI and local invocation in a central README

## Commit

`test(bf-5dnh1): add memory ceiling enforcement for proptests`

- `scripts/run-proptest-with-limits.sh`: Cgroup wrapper for proptest execution
- `scripts/README.md`: Documentation for memory ceiling enforcement
