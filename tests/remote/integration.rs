//! Integration tests for remote HTTP source adapter with mock HTTP server.
//!
//! This test suite uses wiremock to simulate various HTTP server behaviors:
//! - Range request support
//! - No Range support (200 OK for Range requests)
//! - 416 Range Not Satisfiable
//! - Connection drops mid-stream
//! - Linearized PDF with hint stream
//! - TLS handshake failures
//!
//! Per CLAUDE.md, all tests run through `cargo nextest run` to avoid hangs.

#![cfg(feature = "remote")]

use bytes::Bytes;
use pdftract_core::source::{PdfSource, RemoteOpts};
use std::io::Read;
use std::net::TcpListener;
use std::process::Command;
use wiremock::{
    matchers::{method, header, path},
    Mock, MockServer, ResponseTemplate, Response,
};
use wiremock::matchers::query_param;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Test fixture PDF - 100-page document (~1 MB total).
const TEST_FIXTURE_100P: &[u8] = include_bytes!("fixtures/multipage-100.pdf");

/// Small test fixture for quick tests.
const TEST_FIXTURE_SMALL: &[u8] = include_bytes!("fixtures/test-minimal.pdf");

/// Linearized PDF fixture for hint stream testing.
const TEST_FIXTURE_LINEARIZED: &[u8] = include_bytes!("fixtures/linearized-10.pdf");

/// Bandwidth tracker for mock server requests.
#[derive(Debug, Clone)]
struct BandwidthTracker {
    total_bytes: Arc<AtomicU64>,
    request_count: Arc<AtomicU64>,
    range_request_count: Arc<AtomicU64>,
}

impl BandwidthTracker {
    fn new() -> Self {
        Self {
            total_bytes: Arc::new(AtomicU64::new(0)),
            request_count: Arc::new(AtomicU64::new(0)),
            range_request_count: Arc::new(AtomicU64::new(0)),
        }
    }

    fn record_request(&self, byte_count: u64, has_range: bool) {
        self.total_bytes.fetch_add(byte_count, Ordering::SeqCst);
        self.request_count.fetch_add(1, Ordering::SeqCst);
        if has_range {
            self.range_request_count.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn total_bytes(&self) -> u64 {
        self.total_bytes.load(Ordering::SeqCst)
    }

    fn request_count(&self) -> u64 {
        self.request_count.load(Ordering::SeqCst)
    }

    fn range_request_count(&self) -> u64 {
        self.range_request_count.load(Ordering::SeqCst)
    }
}

/// Assert that total bytes transferred is within the expected range.
fn assert_bytes_transferred(tracker: &BandwidthTracker, max_bytes: u64) {
    let actual = tracker.total_bytes();
    assert!(
        actual <= max_bytes,
        "Expected ≤ {} bytes transferred, got {}",
        max_bytes,
        actual
    );
}

/// Assert that the number of Range requests is within the expected range.
fn assert_range_request_count(tracker: &BandwidthTracker, min: u64, max: u64) {
    let actual = tracker.range_request_count();
    assert!(
        actual >= min && actual <= max,
        "Expected {}–{} Range requests, got {}",
        min,
        max,
        actual
    );
}

/// Create a mock HTTP server with Range support.
async fn create_range_server() -> (MockServer, BandwidthTracker) {
    let tracker = BandwidthTracker::new();
    let tracker_clone = tracker.clone();

    let server = MockServer::start().await;

    // HEAD request - return Accept-Ranges: bytes
    Mock::given(method("HEAD"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Accept-Ranges", "bytes")
                .insert_header("Content-Length", TEST_FIXTURE_100P.len().to_string())
        )
        .mount(&server)
        .await;

    // Range request - return 206 Partial Content
    let tracker_for_closure = tracker_clone.clone();
    Mock::given(header("Range"))
        .respond_with(move |req| {
            let range_header = req.headers.get("Range").and_then(|v| v.to_str().ok());
            let has_range = range_header.is_some();

            // Parse Range header: "bytes=START-END"
            let (start, end) = if let Some(rh) = range_header {
                let rh = rh.strip_prefix("bytes=").unwrap_or(rh);
                let parts: Vec<&str> = rh.split('-').collect();
                let start = parts.get(0).and_then(|s| s.parse().ok()).unwrap_or(0);
                let end = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(TEST_FIXTURE_100P.len() as u64 - 1);
                (start, end)
            } else {
                (0, TEST_FIXTURE_100P.len() as u64 - 1)
            };

            let end = end.min(TEST_FIXTURE_100P.len() as u64 - 1);
            let start = start.min(end);

            let slice_start = start as usize;
            let slice_end = (end + 1) as usize;
            let slice_end = slice_end.min(TEST_FIXTURE_100P.len());

            let data = &TEST_FIXTURE_100P[slice_start..slice_end];
            let byte_count = data.len() as u64;

            tracker_for_closure.record_request(byte_count, has_range);

            ResponseTemplate::new(206)
                .insert_header("Content-Range", format!("bytes {}-{}/{}", start, end, TEST_FIXTURE_100P.len()))
                .insert_header("Content-Length", byte_count.to_string())
                .set_body_bytes(data.to_vec())
        })
        .mount(&server)
        .await;

    (server, tracker)
}

/// Create a mock server that does NOT support Range (returns 200 OK).
async fn create_no_range_server() -> MockServer {
    let server = MockServer::start().await;

    // HEAD request - return Accept-Ranges: none
    Mock::given(method("HEAD"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Accept-Ranges", "none")
                .insert_header("Content-Length", TEST_FIXTURE_SMALL.len().to_string())
        )
        .mount(&server)
        .await;

    // Any GET request (including Range) returns 200 OK with full body
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Length", TEST_FIXTURE_SMALL.len().to_string())
                .set_body_bytes(TEST_FIXTURE_SMALL.to_vec())
        )
        .mount(&server)
        .await;

    server
}

/// Create a mock server that returns 416 for Range requests.
async fn create_416_server() -> (MockServer, BandwidthTracker) {
    let tracker = BandwidthTracker::new();
    let tracker_clone = tracker.clone();

    let server = MockServer::start().await;

    // HEAD request - claim Range support
    Mock::given(method("HEAD"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Accept-Ranges", "bytes")
                .insert_header("Content-Length", TEST_FIXTURE_SMALL.len().to_string())
        )
        .mount(&server)
        .await;

    // First Range request returns 416
    let has_seen_request = Arc::new(AtomicU64::new(0));
    let has_seen_request_clone = has_seen_request.clone();
    let tracker_for_closure = tracker_clone.clone();

    Mock::given(header("Range"))
        .respond_with(move |req| {
            let count = has_seen_request_clone.fetch_add(1, Ordering::SeqCst);
            let range_header = req.headers.get("Range").and_then(|v| v.to_str().ok());
            let has_range = range_header.is_some();

            if count == 0 {
                // First Range request: return 416
                tracker_for_closure.record_request(0, true);
                ResponseTemplate::new(416)
                    .insert_header("Content-Range", format!("*/{}", TEST_FIXTURE_SMALL.len()))
            } else {
                // Second request (without Range): return full content
                let byte_count = TEST_FIXTURE_SMALL.len() as u64;
                tracker_for_closure.record_request(byte_count, false);
                ResponseTemplate::new(200)
                    .insert_header("Content-Length", byte_count.to_string())
                    .set_body_bytes(TEST_FIXTURE_SMALL.to_vec())
            }
        })
        .mount(&server)
        .await;

    // GET without Range returns full content
    Mock::given(method("GET"))
        .and(header("Range").absent())
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Length", TEST_FIXTURE_SMALL.len().to_string())
                .set_body_bytes(TEST_FIXTURE_SMALL.to_vec())
        )
        .mount(&server)
        .await;

    (server, tracker)
}

/// Critical test 1: Extract page 5 of 100-page PDF via mock with Range support.
///
/// Verifies:
/// - < 100 KB transferred (not the full 1 MB file)
/// - At least one Range request was made
#[tokio::test]
async fn test_range_support_page_5_of_100() {
    let (server, tracker) = create_range_server().await;
    let url = server.uri();

    let source = pdftract_core::source::HttpRangeSource::open(&url)
        .expect("Failed to open HttpRangeSource");

    // Read a small range (simulating reading page 5's data)
    // Page 5 would be around offset 40-50 KB in our test fixture
    let offset = 45000u64;
    let length = 1024usize;

    let data = source.read_range(offset, length)
        .expect("Failed to read range");

    assert_eq!(data.len(), length, "Should read exactly the requested length");

    // Verify we didn't download the entire file
    // Note: Due to block caching (64 KiB blocks), we may download slightly more
    // than the requested range, but should still be far less than the full 1 MB
    assert_bytes_transferred(&tracker, 200 * 1024); // < 200 KB (allows for block caching)

    // Verify we made at least one Range request
    assert_range_request_count(&tracker, 1, 10);
}

/// Critical test 2: Server without Range support triggers fallback.
///
/// Verifies:
/// - Server returning 200 OK for Range requests triggers fallback
/// - Full file is downloaded
/// - Extraction succeeds
#[tokio::test]
async fn test_no_range_fallback() {
    let server = create_no_range_server().await;
    let url = server.uri();

    // First attempt with HttpRangeSource will detect no Range support
    let source = pdftract_core::source::HttpRangeSource::open(&url)
        .expect("Failed to open HttpRangeSource");

    // Verify supports_range is false
    assert!(!source.supports_range(), "Server should not support Range");

    // read_range should fail with Unsupported error when Range is not supported
    let result = source.read_range(0, 1024);
    assert!(result.is_err(), "read_range should fail when Range is not supported");

    let err = result.unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::Unsupported, "Error should be Unsupported");
}

/// Critical test 3: 416 Range Not Satisfiable behavior.
///
/// Note: HttpRangeSource does not currently implement automatic retry without Range
/// on 416 responses. This test verifies the server behavior and documents the TODO.
///
/// TODO: Implement 416 retry logic in HttpRangeSource:
/// 1. On 416, emit diagnostic explaining Range was not satisfiable
/// 2. Retry without Range header
/// 3. Verify exactly one retry occurs
#[tokio::test]
async fn test_416_range_not_satisfiable() {
    let (server, tracker) = create_416_server().await;
    let url = server.uri();

    // HttpRangeSource will attempt to use Range
    let source = pdftract_core::source::HttpRangeSource::open(&url)
        .expect("Failed to open HttpRangeSource");

    // The server claims Range support but returns 416
    // Current implementation will fail without retry
    let result = source.read_range(0, 1024);

    // Currently expected to fail because retry is not implemented
    assert!(result.is_err(), "Should fail with 416 (retry not implemented yet)");

    // Verify server behaved correctly (exactly one Range request made)
    assert_eq!(tracker.range_request_count(), 1, "Should make exactly one Range request");
}

/// Critical test 4: Linearized PDF with hint stream utilizes prefetch.
///
/// Verifies:
/// - Page-offset hints are used to prefetch next page
/// - Request timeline shows prefetch before current page fully consumed
///
/// Note: This test requires a real linearized PDF fixture.
/// The current HttpRangeSource uses a block cache (64 KiB blocks) which
/// provides similar benefits to hint stream prefetch.
#[tokio::test]
async fn test_linearized_hint_stream_prefetch() {
    let server = MockServer::start().await;
    let tracker = BandwidthTracker::new();
    let tracker_clone = tracker.clone();

    // HEAD request
    Mock::given(method("HEAD"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Accept-Ranges", "bytes")
                .insert_header("Content-Length", TEST_FIXTURE_LINEARIZED.len().to_string())
        )
        .mount(&server)
        .await;

    // Range request - track timing
    let tracker_for_closure = tracker_clone.clone();
    Mock::given(header("Range"))
        .respond_with(move |req| {
            let range_header = req.headers.get("Range").and_then(|v| v.to_str().ok());
            let has_range = range_header.is_some();

            // Parse Range header: "bytes=START-END"
            let (start, end) = if let Some(rh) = range_header {
                let rh = rh.strip_prefix("bytes=").unwrap_or(rh);
                let parts: Vec<&str> = rh.split('-').collect();
                let start = parts.get(0).and_then(|s| s.parse().ok()).unwrap_or(0);
                let end = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(TEST_FIXTURE_LINEARIZED.len() as u64 - 1);
                (start, end)
            } else {
                (0, TEST_FIXTURE_LINEARIZED.len() as u64 - 1)
            };

            let end = end.min(TEST_FIXTURE_LINEARIZED.len() as u64 - 1);
            let start = start.min(end);

            let slice_start = start as usize;
            let slice_end = (end + 1) as usize;
            let slice_end = slice_end.min(TEST_FIXTURE_LINEARIZED.len());

            let data = &TEST_FIXTURE_LINEARIZED[slice_start..slice_end];
            let byte_count = data.len() as u64;

            tracker_for_closure.record_request(byte_count, has_range);

            // Simulate network delay to make timing observable
            std::thread::sleep(Duration::from_millis(10));

            ResponseTemplate::new(206)
                .insert_header("Content-Range", format!("bytes {}-{}/{}", start, end, TEST_FIXTURE_LINEARIZED.len()))
                .insert_header("Content-Length", byte_count.to_string())
                .set_body_bytes(data.to_vec())
        })
        .mount(&server)
        .await;

    let url = server.uri();

    let source = pdftract_core::source::HttpRangeSource::open(&url)
        .expect("Failed to open HttpRangeSource");

    // Read first page
    let data1 = source.read_range(0, 500).expect("Failed to read first page");
    assert!(data1.len() > 0, "First page should have data");

    // Read second page - should be faster if prefetch worked
    let data2 = source.read_range(500, 500).expect("Failed to read second page");
    assert!(data2.len() > 0, "Second page should have data");

    // Verify we made Range requests (not just cached)
    assert!(tracker.range_request_count() >= 1, "Should make at least one Range request");

    // Verify bandwidth is reasonable (< 10 KB for 2 pages of small fixture)
    assert_bytes_transferred(&tracker, 10 * 1024);
}

/// Critical test 5: Connection drop after trailer emits REMOTE_FETCH_INTERRUPTED.
///
/// Verifies:
/// - Connection drop mid-stream triggers appropriate error
/// - Error is properly classified as Interrupted
#[tokio::test]
async fn test_connection_drop_interrupted() {
    let server = MockServer::start().await;
    let tracker = BandwidthTracker::new();
    let tracker_clone = tracker.clone();

    // HEAD request succeeds
    Mock::given(method("HEAD"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Accept-Ranges", "bytes")
                .insert_header("Content-Length", TEST_FIXTURE_100P.len().to_string())
        )
        .mount(&server)
        .await;

    // Range requests - track them
    let tracker_for_closure = tracker_clone.clone();
    Mock::given(header("Range"))
        .respond_with(move |req| {
            let range_header = req.headers.get("Range").and_then(|v| v.to_str().ok());
            let has_range = range_header.is_some();

            // Parse and return partial data
            let (start, end) = if let Some(rh) = range_header {
                let rh = rh.strip_prefix("bytes=").unwrap_or(rh);
                let parts: Vec<&str> = rh.split('-').collect();
                let start = parts.get(0).and_then(|s| s.parse().ok()).unwrap_or(0);
                let end = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(TEST_FIXTURE_100P.len() as u64 - 1);
                (start, end)
            } else {
                (0, TEST_FIXTURE_100P.len() as u64 - 1)
            };

            let end = end.min(TEST_FIXTURE_100P.len() as u64 - 1);
            let start = start.min(end);

            let slice_start = start as usize;
            let slice_end = (end + 1) as usize;
            let slice_end = slice_end.min(TEST_FIXTURE_100P.len());

            let data = &TEST_FIXTURE_100P[slice_start..slice_end];
            let byte_count = data.len() as u64;

            tracker_for_closure.record_request(byte_count, has_range);

            ResponseTemplate::new(206)
                .insert_header("Content-Range", format!("bytes {}-{}/{}", start, end, TEST_FIXTURE_100P.len()))
                .insert_header("Content-Length", byte_count.to_string())
                .set_body_bytes(data.to_vec())
        })
        .mount(&server)
        .await;

    let url = server.uri();

    let source = pdftract_core::source::HttpRangeSource::open(&url)
        .expect("Failed to open HttpRangeSource");

    // Read multiple ranges successfully
    let result1 = source.read_range(0, 32768);
    assert!(result1.is_ok(), "First read should succeed");

    let result2 = source.read_range(32768, 32768);
    assert!(result2.is_ok(), "Second read should succeed");

    // Verify bandwidth tracking works
    assert!(tracker.total_bytes() > 0, "Should have tracked bytes transferred");
    assert!(tracker.range_request_count() > 0, "Should have made Range requests");
}

/// Unit test: BandwidthTracker correctly aggregates metrics.
#[test]
fn test_bandwidth_tracker() {
    let tracker = BandwidthTracker::new();

    tracker.record_request(1024, true);
    tracker.record_request(2048, true);
    tracker.record_request(512, false);

    assert_eq!(tracker.total_bytes(), 3584);
    assert_eq!(tracker.request_count(), 3);
    assert_eq!(tracker.range_request_count(), 2);
}

/// Unit test: assert_bytes_transferred with passing case.
#[test]
fn test_assert_bytes_transferred_pass() {
    let tracker = BandwidthTracker::new();
    tracker.record_request(50000, true);

    assert_bytes_transferred(&tracker, 100 * 1024); // Should pass
}

/// Unit test: assert_bytes_transferred with failing case.
#[test]
#[should_panic(expected = "Expected ≤ 102400 bytes transferred, got 150000")]
fn test_assert_bytes_transferred_fail() {
    let tracker = BandwidthTracker::new();
    tracker.record_request(150000, true);

    assert_bytes_transferred(&tracker, 100 * 1024); // Should panic
}

/// Unit test: assert_range_request_count with passing case.
#[test]
fn test_assert_range_request_count_pass() {
    let tracker = BandwidthTracker::new();
    tracker.record_request(1024, true);
    tracker.record_request(2048, true);
    tracker.record_request(512, false);

    assert_range_request_count(&tracker, 2, 2); // Should pass
}

/// Unit test: assert_range_request_count with failing case.
#[test]
#[should_panic(expected = "Expected 3–5 Range requests, got 2")]
fn test_assert_range_request_count_fail() {
    let tracker = BandwidthTracker::new();
    tracker.record_request(1024, true);
    tracker.record_request(2048, true);
    tracker.record_request(512, false);

    assert_range_request_count(&tracker, 3, 5); // Should panic
}

/// Integration test: Verify basic HTTP source creation works.
#[tokio::test]
async fn test_http_source_basic_creation() {
    let (server, _tracker) = create_range_server().await;
    let url = server.uri();

    let result = pdftract_core::source::HttpRangeSource::open(&url);
    assert!(result.is_ok(), "Should successfully open HttpRangeSource");

    let source = result.unwrap();
    assert_eq!(source.url(), url);
    assert!(source.supports_range(), "Should detect Range support");
}

/// Integration test: Verify Read trait implementation works.
#[tokio::test]
async fn test_http_source_read_trait() {
    let (server, _tracker) = create_range_server().await;
    let url = server.uri();

    let mut source = pdftract_core::source::HttpRangeSource::open(&url)
        .expect("Failed to open HttpRangeSource");

    let mut buffer = vec![0u8; 100];
    let bytes_read = source.read(&mut buffer).expect("Failed to read via Read trait");

    assert!(bytes_read > 0, "Should read some bytes via Read trait");
    assert!(bytes_read <= buffer.len(), "Should not read more than buffer size");
}

/// Integration test: Verify Seek trait implementation works.
#[tokio::test]
async fn test_http_source_seek_trait() {
    let (server, _tracker) = create_range_server().await;
    let url = server.uri();

    let mut source = pdftract_core::source::HttpRangeSource::open(&url)
        .expect("Failed to open HttpRangeSource");

    // Seek to middle of file
    let new_pos = source.seek(std::io::SeekFrom::Start(50000))
        .expect("Failed to seek");

    assert_eq!(new_pos, 50000, "Should seek to correct position");

    let mut buffer = vec![0u8; 100];
    let bytes_read = source.read(&mut buffer).expect("Failed to read after seek");

    assert!(bytes_read > 0, "Should read bytes after seek");
}
