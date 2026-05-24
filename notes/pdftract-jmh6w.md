# pdftract-jmh6w: rayon+tokio concurrency bridge

## Summary

Implemented Phase 6.4.3: rayon+tokio concurrency bridge with documentation and testing.

## Changes Made

### 1. Documentation (serve.rs)
- Added comprehensive "Concurrency model" section to module rustdoc explaining:
  - Two-level concurrency: tokio (per-request) + rayon (per-document)
  - spawn_blocking bridge between async and sync
  - Thread pool sizing (tokio: 512, rayon: num_cpus)
- Added "Error codes" section documenting all error response codes

### 2. CLI Help (main.rs)
- Added long_about to Serve command documenting:
  - Concurrency architecture
  - Endpoints (/extract, /extract/text, /extract/stream, /health)
  - Cache behavior

### 3. Error Handling (serve.rs)
- Added `InternalPanic` variant to `AxumError` enum
- Updated `IntoResponse` to return specific error codes:
  - BAD_REQUEST (400)
  - EXTRACTION_ERROR (422)
  - INTERNAL_ERROR (500)
  - INTERNAL_PANIC (500)
- Improved JoinError handling in all three POST handlers:
  - `extract_handler`
  - `extract_text_handler`
  - `extract_stream_handler`
- Distinguishes between cancellation (INTERNAL_ERROR) and panic (INTERNAL_PANIC)

### 4. Testing (serve.rs)
- `test_error_into_response`: Verifies error status codes
- `test_cache_status_conversions`: Tests CacheStatus enum conversions
- `test_concurrent_requests_parallel`: Critical integration test
  - Starts server on random port
  - Verifies /health responds in < 100ms
  - Launches 8 concurrent extraction requests
  - Verifies all requests complete
  - Verifies wallclock time < serialized estimate (proves parallelism)
  - Verifies /health still responds quickly during load

### 5. Dependencies (Cargo.toml)
- Added `multipart` feature to reqwest dev-dependency for integration testing

## Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| 8 concurrent requests complete in parallel | PASS | Integration test runs 8 concurrent requests and verifies parallelism |
| /health responds in < 100ms during 8 concurrent extractions | PASS | Test verifies /health response time < 100ms under load |
| Rayon par_iter inside spawn_blocking works | PASS | Already implemented; unchanged |
| Module rustdoc documents concurrency model | PASS | Added "Concurrency model" section to serve.rs rustdoc |
| CLI --help documents concurrency model | PASS | Added long_about to Serve command |

## Git Commit

```
66b3eff feat(pdftract-jmh6w): implement rayon+tokio concurrency bridge
```

## Test Results

```
running 3 tests
test serve::tests::test_cache_status_conversions ... ok
test serve::tests::test_error_into_response ... ok
test serve::tests::test_concurrent_requests_parallel ... ok

test result: ok. 3 passed; 0 failed; 0 ignored
```

## Notes

- The spawn_blocking pattern was already implemented; this bead added documentation, improved error handling, and testing
- Integration test uses existing test fixture `hello.pdf` from pdftract-libpdftract
- Test focuses on concurrency proof (all requests complete in parallel) rather than extraction success
- The pre-existing clippy error in pdftract-core build.rs is unrelated to this change
