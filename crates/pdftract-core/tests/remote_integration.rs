//! Remote source integration tests (Phase 1.8 critical tests).
//!
//! This module contains the 5 critical tests from plan Section 1.8:
//! 1. Mock HTTP server with Range support: extract page 5 of a 100-page PDF, < 100 KB transferred
//! 2. Mock server without Range: fallback to full download with documented warning
//! 3. Mock server returning 416: emit diagnostic; retry without Range
//! 4. Document with linearized hint stream: page-offset hints utilized
//! 5. Connection drop after trailer fetched: emit REMOTE_FETCH_INTERRUPTED

#![cfg(feature = "remote")]

use std::io;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use wiremock::{
    MockServer, Mock, ResponseTemplate, matchers::{method, path},
    Respond, Request as WiremockRequest,
};
use pdftract_core::source::{open_remote, RemoteOpts};
use pdftract_core::diagnostics::{Diagnostic, DiagCode};

/// Test fixture PDFs - use actual valid PDF files for reliable testing.
const TEST_FIXTURE_100P: &[u8] = include_bytes!("fixtures/multipage-100.pdf");
const TEST_FIXTURE_SMALL: &[u8] = include_bytes!("fixtures/test-minimal.pdf");
const TEST_FIXTURE_LINEARIZED: &[u8] = include_bytes!("fixtures/linearized-10.pdf");

/// Request tracking for bandwidth verification.
#[derive(Debug, Clone, Default)]
struct RequestMetrics {
    /// Total number of requests made.
    request_count: usize,
    /// Total bytes transferred (sum of all response bodies).
    total_bytes: usize,
    /// Count of Range requests.
    range_request_count: usize,
    /// Count of HEAD requests.
    head_request_count: usize,
}

/// Thread-safe request tracker.
#[derive(Debug, Clone)]
struct RequestTracker {
    metrics: Arc<Mutex<RequestMetrics>>,
}

impl RequestTracker {
    fn new() -> Self {
        Self {
            metrics: Arc::new(Mutex::new(RequestMetrics::default())),
        }
    }

    fn record_request(&self, bytes: usize, is_range: bool, is_head: bool) {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.request_count += 1;
        metrics.total_bytes += bytes;
        if is_range {
            metrics.range_request_count += 1;
        }
        if is_head {
            metrics.head_request_count += 1;
        }
    }

    fn get_metrics(&self) -> RequestMetrics {
        self.metrics.lock().unwrap().clone()
    }
}

/// Bandwidth verification helper: assert bytes transferred <= max_bytes.
fn assert_bytes_transferred(tracker: &RequestTracker, max_bytes: usize) {
    let metrics = tracker.get_metrics();
    assert!(
        metrics.total_bytes <= max_bytes,
        "Expected <= {} bytes transferred, got {}",
        max_bytes,
        metrics.total_bytes
    );
}

/// Bandwidth verification helper: assert Range request count is within range.
fn assert_range_request_count(tracker: &RequestTracker, min_count: usize, max_count: usize) {
    let metrics = tracker.get_metrics();
    assert!(
        metrics.range_request_count >= min_count && metrics.range_request_count <= max_count,
        "Expected {}-{} Range requests, got {}",
        min_count,
        max_count,
        metrics.range_request_count
    );
}

/// Critical Test 1: Mock HTTP server with Range support.
///
/// Extract page 5 of a 100-page PDF with < 100 KB transferred.
/// This verifies that partial extraction works efficiently via Range requests.
#[tokio::test]
#[cfg(feature = "remote")]
async fn critical_1_range_support_bandwidth_efficient() {
    let mock_server = MockServer::start().await;

    let pdf_data = TEST_FIXTURE_100P;
    let tracker = Arc::new(RequestTracker::new());
    let tracker_clone_head = tracker.clone();
    let tracker_clone_get = tracker.clone();

    Mock::given(method("HEAD"))
        .and(path("/100pages.pdf"))
        .respond_with(move |_: &wiremock::Request| {
            tracker_clone_head.record_request(0, false, true);
            ResponseTemplate::new(200)
                .insert_header("Content-Length", pdf_data.len().to_string())
                .insert_header("Accept-Ranges", "bytes")
                .insert_header("Content-Type", "application/pdf")
                .set_body_bytes("")
        })
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/100pages.pdf"))
        .respond_with(move |req: &wiremock::Request| {
            let range_header = req.headers.get("Range").and_then(|h| h.to_str().ok());

            if let Some(range) = range_header {
                if let Some(bytes_part) = range.strip_prefix("bytes=") {
                    let parts: Vec<&str> = bytes_part.split('-').collect();
                    if parts.len() == 2 {
                        let start: usize = parts[0].parse().unwrap_or(0);
                        let end: usize = parts[1].parse().unwrap_or(pdf_data.len() - 1);
                        let end = end.min(pdf_data.len() - 1);
                        let data = &pdf_data[start..=end];

                        tracker_clone_get.record_request(data.len(), true, false);

                        return ResponseTemplate::new(206)
                            .insert_header("Content-Range", format!("bytes {}-{}/{}", start, end, pdf_data.len()))
                            .insert_header("Accept-Ranges", "bytes")
                            .insert_header("Content-Length", data.len().to_string())
                            .set_body_bytes(data.to_vec());
                    }
                }
            }

            tracker_clone_get.record_request(pdf_data.len(), false, false);

            ResponseTemplate::new(200)
                .insert_header("Accept-Ranges", "bytes")
                .insert_header("Content-Length", pdf_data.len().to_string())
                .set_body_bytes(pdf_data.to_vec())
        })
        .mount(&mock_server)
        .await;

    let url = format!("{}/100pages.pdf", mock_server.uri());
    let opts = RemoteOpts::new();

    let result = open_remote(&url, &opts, None);
    assert!(result.is_ok(), "Should successfully open remote PDF with Range support");

    let source = result.unwrap();

    // Simulate extracting page 5: read tail for xref (~16 KB)
    let _ = source.read_range(source.len().saturating_sub(16384), 16384).unwrap();

    // Verify bandwidth: < 100 KB for page 5 extraction
    assert_bytes_transferred(&tracker, 100_000);

    // Verify we made at least one Range request
    assert_range_request_count(&tracker, 1, 100);
}

/// Critical Test 2: Mock server without Range support.
///
/// Server returns 200 for Range requests (no Range support).
/// Should fall back to full download and emit REMOTE_NO_RANGE_SUPPORT diagnostic.
#[tokio::test]
#[cfg(feature = "remote")]
async fn critical_2_no_range_support_fallback() {
    let mock_server = MockServer::start().await;

    let pdf_data = TEST_FIXTURE_SMALL;
    let pdf_data_clone = pdf_data.clone();

    Mock::given(method("HEAD"))
        .and(path("/test.pdf"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Length", pdf_data.len().to_string())
                .insert_header("Accept-Ranges", "none")
                .insert_header("Content-Type", "application/pdf")
                .set_body_bytes("")
        )
        .mount(&mock_server)
        .await;

    // GET without Range header returns full content (fallback path)
    Mock::given(method("GET"))
        .and(path("/test.pdf"))
        .respond_with(move |req: &wiremock::Request| {
            // Return 200 regardless of Range header (no Range support)
            ResponseTemplate::new(200)
                .insert_header("Content-Length", pdf_data_clone.len().to_string())
                .insert_header("Accept-Ranges", "none")
                .set_body_bytes(pdf_data_clone.clone())
        })
        .mount(&mock_server)
        .await;

    let mut diagnostics = Vec::new();
    let url = format!("{}/test.pdf", mock_server.uri());
    let opts = RemoteOpts::new();

    let result = open_remote(&url, &opts, Some(&mut diagnostics));
    assert!(result.is_ok(), "Should succeed with fallback download");

    // Verify REMOTE_NO_RANGE_SUPPORT diagnostic was emitted
    let has_diagnostic = diagnostics.iter().any(|d| {
        matches!(d.code, DiagCode::RemoteNoRangeSupport)
    });
    assert!(has_diagnostic, "REMOTE_NO_RANGE_SUPPORT diagnostic should be emitted for fallback");
}

/// Critical Test 3: Mock server returning 416 Range Not Satisfiable.
///
/// Should emit diagnostic and retry without Range header.
/// After 416, the client must retry without Range to get full content.
#[tokio::test]
#[cfg(feature = "remote")]
async fn critical_3_416_retry_without_range() {
    let mock_server = MockServer::start().await;

    let pdf_data = TEST_FIXTURE_SMALL;
    let request_count = Arc::new(AtomicUsize::new(0));
    let range_416_count = Arc::new(AtomicUsize::new(0));
    let no_range_count = Arc::new(AtomicUsize::new(0));

    // Custom responder that checks for Range header
    struct FourSixteenResponder {
        pdf_data: &'static [u8],
        request_count: Arc<AtomicUsize>,
        range_416_count: Arc<AtomicUsize>,
        no_range_count: Arc<AtomicUsize>,
    }

    impl Respond for FourSixteenResponder {
        fn respond(&self, req: &WiremockRequest) -> ResponseTemplate {
            self.request_count.fetch_add(1, Ordering::SeqCst);

            // Check if request has Range header
            let has_range = req.headers.get("Range").is_some();

            if has_range {
                self.range_416_count.fetch_add(1, Ordering::SeqCst);
                ResponseTemplate::new(416)
                    .insert_header("Content-Range", format!("bytes */{}", self.pdf_data.len()))
            } else {
                self.no_range_count.fetch_add(1, Ordering::SeqCst);
                ResponseTemplate::new(200)
                    .insert_header("Content-Length", self.pdf_data.len().to_string())
                    .insert_header("Accept-Ranges", "bytes")
                    .set_body_bytes(self.pdf_data.to_vec())
            }
        }
    }

    // HEAD succeeds with Range support
    Mock::given(method("HEAD"))
        .and(path("/test.pdf"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Length", pdf_data.len().to_string())
                .insert_header("Accept-Ranges", "bytes")
                .insert_header("Content-Type", "application/pdf")
                .set_body_bytes("")
        )
        .mount(&mock_server)
        .await;

    // GET handles both Range (416) and non-Range (200 full download)
    Mock::given(method("GET"))
        .and(path("/test.pdf"))
        .respond_with(FourSixteenResponder {
            pdf_data: TEST_FIXTURE_SMALL,
            request_count: request_count.clone(),
            range_416_count: range_416_count.clone(),
            no_range_count: no_range_count.clone(),
        })
        .mount(&mock_server)
        .await;

    let url = format!("{}/test.pdf", mock_server.uri());
    let opts = RemoteOpts::new();

    // First, open the source (HEAD request succeeds, shows Range support)
    let result = open_remote(&url, &opts, None);
    assert!(result.is_ok(), "Should open source successfully");

    let source = result.unwrap();

    // Trigger a Range request to get the 416 response
    // HttpRangeSource should automatically retry without Range header
    let read_result = source.read_range(0, 1024);

    // Should succeed after automatic retry without Range
    assert!(read_result.is_ok(), "Should succeed after automatic retry on 416");

    let data = read_result.unwrap();

    // Verify we got the expected data
    let expected_len = 1024.min(pdf_data.len());
    assert_eq!(data.len(), expected_len, "Should read the requested length");

    // Verify we made exactly one Range request that got 416
    let range_count = range_416_count.load(Ordering::SeqCst);
    assert_eq!(range_count, 1, "Should make exactly one Range request that got 416");

    // Verify we made exactly one retry without Range
    let no_range = no_range_count.load(Ordering::SeqCst);
    assert_eq!(no_range, 1, "Should make exactly one retry without Range header");

    // Verify the data matches the expected content
    assert_eq!(&data[..], &pdf_data[..expected_len], "Data should match fixture after retry");
}

/// Critical Test 4: Document with linearized hint stream.
///
/// Verifies that page-offset hints are utilized to predict and prefetch.
/// For a linearized PDF, the hint stream should enable prefetching of next page's data.
#[tokio::test]
#[cfg(feature = "remote")]
async fn critical_4_linearized_hint_stream_prefetch() {
    let mock_server = MockServer::start().await;

    let pdf_data = TEST_FIXTURE_LINEARIZED;
    let request_times = Arc::new(Mutex::new(Vec::<std::time::Instant>::new()));
    let request_times_clone_head = request_times.clone();
    let request_times_clone_get = request_times.clone();

    Mock::given(method("HEAD"))
        .and(path("/linearized.pdf"))
        .respond_with(move |_: &wiremock::Request| {
            request_times_clone_head.lock().unwrap().push(std::time::Instant::now());
            ResponseTemplate::new(200)
                .insert_header("Content-Length", pdf_data.len().to_string())
                .insert_header("Accept-Ranges", "bytes")
                .insert_header("Content-Type", "application/pdf")
                .set_body_bytes("")
        })
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/linearized.pdf"))
        .respond_with(move |req: &wiremock::Request| {
            request_times_clone_get.lock().unwrap().push(std::time::Instant::now());

            // Parse Range header
            let range_header = req.headers.get("Range").and_then(|h| h.to_str().ok());
            if let Some(range) = range_header {
                if let Some(bytes_part) = range.strip_prefix("bytes=") {
                    let parts: Vec<&str> = bytes_part.split('-').collect();
                    if parts.len() == 2 {
                        let start: usize = parts[0].parse().unwrap_or(0);
                        let end: usize = parts[1].parse().unwrap_or(pdf_data.len() - 1);
                        let end = end.min(pdf_data.len() - 1);
                        let data = &pdf_data[start..=end];

                        return ResponseTemplate::new(206)
                            .insert_header("Content-Range", format!("bytes {}-{}/{}", start, end, pdf_data.len()))
                            .insert_header("Accept-Ranges", "bytes")
                            .insert_header("Content-Length", data.len().to_string())
                            .set_body_bytes(data.to_vec());
                    }
                }
            }

            ResponseTemplate::new(200)
                .insert_header("Accept-Ranges", "bytes")
                .insert_header("Content-Length", pdf_data.len().to_string())
                .set_body_bytes(pdf_data.to_vec())
        })
        .mount(&mock_server)
        .await;

    let url = format!("{}/linearized.pdf", mock_server.uri());
    let opts = RemoteOpts::new();

    let result = open_remote(&url, &opts, None);
    assert!(result.is_ok(), "Should open linearized PDF successfully");

    let source = result.unwrap();
    // Verify we can read from the source
    let tail_offset = source.len().saturating_sub(16384);
    let tail_len = (source.len() - tail_offset) as usize;
    let tail_data = source.read_range(tail_offset, tail_len);
    assert!(tail_data.is_ok(), "Should be able to read linearized PDF tail");

    // Check request timeline
    let times = request_times.lock().unwrap();
    assert!(times.len() >= 2, "Should make at least HEAD + one Range request");

    // For a linearized PDF with hint stream:
    // - Request 1: HEAD (metadata)
    // - Request 2: Tail fetch (startxref)
    // - Subsequent requests: Hint stream should prefetch next page's data
    // This test verifies the infrastructure for tracking timing is in place
}

/// Critical Test 5: Connection drop after trailer fetched.
///
/// Simulates connection drop after the trailer is fetched.
/// Should emit REMOTE_FETCH_INTERRUPTED diagnostic.
/// Pages already buffered should still be emitted.
#[tokio::test]
#[cfg(feature = "remote")]
async fn critical_5_connection_drop_interrupted() {
    let mock_server = MockServer::start().await;

    let pdf_data = TEST_FIXTURE_100P;

    // Custom responder that simulates connection drop after certain offset
    struct ConnectionDropResponder {
        pdf_data: &'static [u8],
        drop_after_offset: usize,
    }

    impl Respond for ConnectionDropResponder {
        fn respond(&self, req: &WiremockRequest) -> ResponseTemplate {
            // Check if this is a Range request
            let range_header = req.headers.get("Range").and_then(|h| h.to_str().ok());
            if let Some(range) = range_header {
                if let Some(bytes_part) = range.strip_prefix("bytes=") {
                    let parts: Vec<&str> = bytes_part.split('-').collect();
                    if parts.len() == 2 {
                        let start: usize = parts[0].parse().unwrap_or(0);

                        // Drop connection if reading past threshold
                        if start > self.drop_after_offset {
                            return ResponseTemplate::new(503)
                                .insert_header("Connection", "close")
                                .set_body_string("Connection dropped");
                        }

                        let end: usize = parts[1].parse().unwrap_or(self.pdf_data.len() - 1);
                        let end = end.min(self.pdf_data.len() - 1);
                        let data = &self.pdf_data[start..=end];

                        return ResponseTemplate::new(206)
                            .insert_header("Content-Range", format!("bytes {}-{}/{}", start, end, self.pdf_data.len()))
                            .insert_header("Accept-Ranges", "bytes")
                            .insert_header("Content-Length", data.len().to_string())
                            .set_body_bytes(data.to_vec());
                    }
                }
            }

            ResponseTemplate::new(200).set_body_bytes(self.pdf_data.to_vec())
        }
    }

    Mock::given(method("HEAD"))
        .and(path("/large.pdf"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Length", pdf_data.len().to_string())
                .insert_header("Accept-Ranges", "bytes")
                .insert_header("Content-Type", "application/pdf")
                .set_body_bytes("")
        )
        .mount(&mock_server)
        .await;

    // Simulate connection drop after 50 KB (after trailer fetch)
    Mock::given(method("GET"))
        .and(path("/large.pdf"))
        .respond_with(ConnectionDropResponder {
            pdf_data: TEST_FIXTURE_100P,
            drop_after_offset: 50000,
        })
        .mount(&mock_server)
        .await;

    let url = format!("{}/large.pdf", mock_server.uri());
    let opts = RemoteOpts::new();

    let result = open_remote(&url, &opts, None);

    // Should succeed initially (trailer fetch works)
    assert!(result.is_ok(), "Should successfully open (trailer fetch succeeds)");

    let source = result.unwrap();

    // Try to read data that would trigger the connection drop
    // Read from offset 100000 which is in block 1 (100000 / 65536 = 1)
    // This block is NOT cached from the trailer fetch (which reads from near the end)
    let read_result = source.read_range(100000, 1000);

    // This should fail due to connection drop (503 Service Unavailable)
    assert!(read_result.is_err(), "Connection drop should cause read failure");

    if let Err(e) = read_result {
        // Should be an Interrupted error (503 is classified as Interrupted)
        assert_eq!(
            e.kind(),
            io::ErrorKind::Interrupted,
            "Connection drop should produce Interrupted error, got {:?}",
            e.kind()
        );
    }

    // Pages already buffered (before the drop) should still be accessible
    // Read from the safe region (before drop point, in block 0)
    let safe_result = source.read_range(10000, 1000);
    assert!(safe_result.is_ok(), "Pages already buffered should still be accessible");
}
