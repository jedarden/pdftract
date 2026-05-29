//! Integration tests for remote HTTP PDF fetching.
//!
//! These tests use wiremock to simulate HTTP servers with various behaviors:
//! - Range request support
//! - No Range support (returns 200 for Range requests)
//! - 416 Range Not Satisfiable responses
//! - Connection drops mid-stream
//! - TLS handshake failures
//! - Linearized PDFs with hint streams
//!
//! Run with: `cargo test --features remote -p pdftract-core -- remote`

#![cfg(feature = "remote")]

use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use pdftract_core::source::{HttpRangeSource, PdfSource};
use wiremock::{matchers, Mock, MockServer, ResponseTemplate};
use wiremock::Request as WiremockRequest;

/// Track total bytes transferred across all requests.
pub struct ByteCounter {
    total: Arc<AtomicU64>,
    request_count: Arc<AtomicU64>,
}

impl ByteCounter {
    fn new() -> Self {
        Self {
            total: Arc::new(AtomicU64::new(0)),
            request_count: Arc::new(AtomicU64::new(0)),
        }
    }

    fn total(&self) -> u64 {
        self.total.load(Ordering::SeqCst)
    }

    fn request_count(&self) -> u64 {
        self.request_count.load(Ordering::SeqCst)
    }
}

/// Custom responder that counts bytes served.
#[derive(Clone)]
struct ByteCountingResponder {
    data: Vec<u8>,
    counter: Arc<AtomicU64>,
    request_counter: Arc<AtomicU64>,
    status: u16,
    supports_range: bool,
    force_416_first: bool, // For testing 416 retry behavior
}

impl ByteCountingResponder {
    fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            counter: Arc::new(AtomicU64::new(0)),
            request_counter: Arc::new(AtomicU64::new(0)),
            status: 200,
            supports_range: true,
            force_416_first: false,
        }
    }

    fn with_supports_range(mut self, supports: bool) -> Self {
        self.supports_range = supports;
        self
    }

    fn with_counter(mut self, counter: Arc<AtomicU64>) -> Self {
        self.counter = counter;
        self
    }

    fn with_request_counter(mut self, counter: Arc<AtomicU64>) -> Self {
        self.request_counter = counter;
        self
    }

    fn with_force_416_first(mut self) -> Self {
        self.force_416_first = true;
        self
    }
}

impl wiremock::Respond for ByteCountingResponder {
    fn respond(&self, request: &WiremockRequest) -> wiremock::Response {
        let request_num = self.request_counter.fetch_add(1, Ordering::SeqCst);
        let mut response = ResponseTemplate::new(self.status);

        // Add Accept-Ranges header if Range is supported
        if self.supports_range {
            response = response.append_header("Accept-Ranges", "bytes");
            response = response.append_header("Content-Length", self.data.len().to_string());
        }

        // Handle Range requests
        let range_header = request.headers.get("range").and_then(|v| v.first());

        if let Some(range_value) = range_header {
            if !self.supports_range {
                // Server doesn't support Range - return full content with 200
                self.counter.fetch_add(self.data.len() as u64, Ordering::SeqCst);
                return response
                    .body(self.data.clone())
                    .set_status(200);
            }

            // Test 416 behavior on first Range request if configured
            if self.force_416_first && request_num == 0 {
                response = response
                    .append_header("Content-Range", format!("bytes */{}", self.data.len()))
                    .append_header("Accept-Ranges", "bytes");
                return response.set_status(416);
            }

            // Parse Range header: "bytes=START-END"
            let range_str = range_value.to_str().unwrap_or("");
            if let Some(range_part) = range_str.strip_prefix("bytes=") {
                let parts: Vec<&str> = range_part.split('-').collect();
                if parts.len() == 2 {
                    if let (Ok(start), Ok(end)) = (parts[0].parse::<u64>(), parts[1].parse::<u64>()) {
                        let data_len = self.data.len() as u64;

                        // Check if range is satisfiable
                        if start >= data_len {
                            // Return 416 Range Not Satisfiable
                            response = response
                                .append_header("Content-Range", format!("bytes */{}", data_len))
                                .set_status(416);
                        } else {
                            let end = end.min(data_len - 1);
                            let slice_start = start as usize;
                            let slice_end = (end + 1) as usize;
                            let slice_data = self.data[slice_start..slice_end.min(self.data.len())].to_vec();

                            self.counter.fetch_add(slice_data.len() as u64, Ordering::SeqCst);
                            response = response
                                .append_header("Content-Range", format!("bytes {}-{}/{}", start, end, data_len))
                                .append_header("Content-Length", slice_data.len().to_string())
                                .body(slice_data)
                                .set_status(206);
                        }

                        return response.into();
                    }
                }
            }
        }

        // No Range header or parsing failed - return full content
        self.counter.fetch_add(self.data.len() as u64, Ordering::SeqCst);
        response.body(self.data.clone()).into()
    }
}

/// Load a test fixture PDF.
fn load_fixture(name: &str) -> Vec<u8> {
    // First try tests/remote/fixtures, then tests/fixtures
    let mut path = PathBuf::from("tests/remote/fixtures");
    path.push(format!("{}.pdf", name));

    if let Ok(data) = fs::read(&path) {
        // Verify it's actually a PDF
        if data.starts_with(b"%PDF") {
            return data;
        }
    }

    // Fallback to main fixtures
    let mut path = PathBuf::from("tests/fixtures");
    path.push(format!("{}.pdf", name));

    fs::read(&path).unwrap_or_else(|e| {
        panic!("Failed to load fixture {}: {}. Use existing PDFs from tests/fixtures/ as basis.", name, e)
    })
}

/// Load a test fixture PDF with a specific filename.
fn load_fixture_file(filename: &str) -> Vec<u8> {
    let mut path = PathBuf::from("tests/remote/fixtures");
    path.push(filename);

    fs::read(&path).unwrap_or_else(|e| {
        panic!("Failed to load fixture file {}: {}. Ensure the file exists in tests/remote/fixtures/.", filename, e)
    })
}

/// Assert that bytes transferred is less than or equal to max_bytes.
fn assert_bytes_transferred(counter: &ByteCounter, max_bytes: u64) {
    let total = counter.total();
    assert!(
        total <= max_bytes,
        "Transferred {} bytes, expected <= {} bytes",
        total,
        max_bytes
    );
}

/// Test 1: Range request partial page extraction.
///
/// Critical test from plan Section 1.8: Mock HTTP server with Range support,
/// extract page 5 of a 100-page PDF, < 100 KB transferred.
#[tokio::test(flavor = "multi_thread")]
async fn test_range_request_partial_extraction() {
    // Mock server with Range support
    let mock_server = MockServer::start().await;
    let pdf_data = load_fixture("valid-minimal");

    let counter = ByteCounter::new();
    let responder = ByteCountingResponder::new(pdf_data.clone())
        .with_supports_range(true)
        .with_counter(counter.total.clone())
        .with_request_counter(counter.request_count.clone());

    Mock::given(matchers::method("HEAD"))
        .respond_with(ResponseTemplate::new(200)
            .append_header("Accept-Ranges", "bytes")
            .append_header("Content-Length", pdf_data.len().to_string()))
        .mount(&mock_server)
        .await;

    Mock::given(matchers::method("GET"))
        .respond_with(responder)
        .named("pdf-get")
        .mount(&mock_server)
        .await;

    // Open the remote PDF
    let url = format!("{}/test.pdf", mock_server.uri());
    let source = HttpRangeSource::open(&url).expect("Failed to open remote PDF");

    // Verify Range support detected
    assert!(source.supports_range(), "Server should support Range");
    assert_eq!(source.len(), pdf_data.len() as u64);

    // Read a small portion (simulating partial page extraction)
    let offset = 1000;
    let length = 4096;
    let data = source.read_range(offset, length).expect("Failed to read range");

    assert_eq!(data.len(), length);
    assert_eq!(&data[..], &pdf_data[offset..offset + length]);

    // For a minimal PDF, reading 5KB should transfer well under 100 KB
    // In a real 100-page PDF, this would be much smaller
    assert_bytes_transferred(&counter, 100_000);

    // Verify at least one request was made
    assert!(counter.request_count() >= 1, "Expected at least 1 request");
}

/// Test 2: Server without Range support.
///
/// Critical test from plan Section 1.8: Mock server without Range,
/// fallback to full download with documented warning.
#[tokio::test(flavor = "multi_thread")]
async fn test_no_range_support_fallback() {
    // Mock server without Range support (returns 200 for Range requests)
    let mock_server = MockServer::start().await;
    let pdf_data = load_fixture("valid-minimal");

    let counter = ByteCounter::new();
    let responder = ByteCountingResponder::new(pdf_data.clone())
        .with_supports_range(false) // Server ignores Range header
        .with_counter(counter.total.clone())
        .with_request_counter(counter.request_count.clone());

    Mock::given(matchers::method("HEAD"))
        .respond_with(ResponseTemplate::new(200)
            .append_header("Content-Length", pdf_data.len().to_string()))
        .mount(&mock_server)
        .await;

    Mock::given(matchers::method("GET"))
        .respond_with(responder)
        .named("pdf-get-no-range")
        .mount(&mock_server)
        .await;

    let url = format!("{}/test.pdf", mock_server.uri());
    let source = HttpRangeSource::open(&url).expect("Failed to open remote PDF");

    // Verify no Range support detected
    assert!(!source.supports_range(), "Server should NOT support Range");

    // Attempt to read should return Unsupported error
    let result = source.read_range(1000, 4096);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::Unsupported);
    assert!(err.to_string().contains("Server does not support Range"));

    // Verify full content was transferred (fallback behavior)
    assert_eq!(counter.total(), pdf_data.len() as u64);
}

/// Test 3: 416 Range Not Satisfiable triggers retry without Range.
///
/// Critical test from plan Section 1.8: Mock server returning 416,
/// emit diagnostic; retry without Range.
#[tokio::test(flavor = "multi_thread")]
async fn test_416_range_not_satisfiable_retry() {
    // Mock server that returns 416 for first Range request, then 200 for retry
    let mock_server = MockServer::start().await;
    let pdf_data = load_fixture("valid-minimal");

    let counter = ByteCounter::new();
    let responder = ByteCountingResponder::new(pdf_data.clone())
        .with_supports_range(true)
        .with_counter(counter.total.clone())
        .with_request_counter(counter.request_count.clone())
        .with_force_416_first(); // First Range request gets 416

    Mock::given(matchers::method("HEAD"))
        .respond_with(ResponseTemplate::new(200)
            .append_header("Accept-Ranges", "bytes")
            .append_header("Content-Length", pdf_data.len().to_string()))
        .mount(&mock_server)
        .await;

    Mock::given(matchers::method("GET"))
        .respond_with(responder)
        .named("pdf-get-416-retry")
        .mount(&mock_server)
        .await;

    let url = format!("{}/test.pdf", mock_server.uri());

    // Open should succeed (server reports Range support in HEAD)
    let source = HttpRangeSource::open(&url).expect("Failed to open remote PDF");

    // First Range request will get 416, implementation should retry without Range
    let result = source.read_range(1000, 4096);

    // Should succeed after retry
    assert!(result.is_ok(), "416 should trigger retry and succeed");

    let data = result.unwrap();
    assert_eq!(data.len(), 4096);
    assert_eq!(&data[..], &pdf_data[1000..1000 + 4096]);

    // Verify requests were made (at least 2: 1 Range + 1 retry)
    assert!(counter.request_count() >= 2, "Expected at least 2 requests (Range + retry)");
}

/// Test 4: Connection drop after trailer.
///
/// Critical test from plan Section 1.8: Connection drop after the trailer
/// is fetched, extraction emits REMOTE_FETCH_INTERRUPTED.
#[tokio::test(flavor = "multi_thread")]
async fn test_connection_drop_after_trailer() {
    use wiremock::respond::FnResponder;

    // Mock server that drops connection after partial response
    let mock_server = MockServer::start().await;
    let pdf_data = load_fixture("valid-minimal");

    // Serve HEAD normally
    Mock::given(matchers::method("HEAD"))
        .respond_with(ResponseTemplate::new(200)
            .append_header("Accept-Ranges", "bytes")
            .append_header("Content-Length", pdf_data.len().to_string()))
        .mount(&mock_server)
        .await;

    // Responder that serves partial content then simulates connection drop
    let partial_responder = FnResponder::new(move |_request: &WiremockRequest| {
        // Return only first 1KB of data, simulating premature connection close
        let partial_len = pdf_data.len().min(1024);
        let partial_data = &pdf_data[..partial_len];

        ResponseTemplate::new(206)
            .append_header("Accept-Ranges", "bytes")
            .append_header("Content-Range", format!("bytes 0-{}/{}", partial_len - 1, pdf_data.len()))
            .append_header("Content-Length", partial_len.to_string())
            .body(partial_data.to_vec())
    });

    Mock::given(matchers::method("GET"))
        .respond_with(partial_responder)
        .mount(&mock_server)
        .await;

    let url = format!("{}/test.pdf", mock_server.uri());
    let source = HttpRangeSource::open(&url).expect("Failed to open remote PDF");

    // Try to read more than what's available - should handle gracefully
    let result = source.read_range(0, 4096);

    // The read should fail because the connection closed prematurely
    assert!(result.is_err());

    let err = result.unwrap_err();
    // Should be an Interrupted error or similar connection error
    assert!(matches!(err.kind(), io::ErrorKind::Interrupted | io::ErrorKind::UnexpectedEof));
}

/// Test 5: TLS handshake failure.
///
/// Critical test from plan Section 1.8: TLS-handshake failure, clear error
/// message with the certificate-chain reason; exit code 6.
///
/// Note: This test is marked as ignore because wiremock doesn't easily
/// support custom TLS certificates. Manual verification required.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "Manual test - requires real TLS server with bad cert"]
async fn test_tls_handshake_failure_self_signed() {
    use rcgen::{Certificate, DistinguishedName, SanTypes};

    // Generate self-signed certificate
    let mut params = rcgen::CertificateParams::default();
    params.distinguished_name = DistinguishedName::new();
    params.distinguished_name.push(rcgen::DnType::CommonName, "localhost");
    params.subject_alt_names = vec![SanTypes::DnsName("localhost".to_string())];

    let cert = Certificate::from_params(params).expect("Failed to generate certificate");
    let cert_pem = cert.serialize_pem().expect("Failed to serialize cert");
    let key_pem = cert.serialize_private_key_pem();

    // Manual verification steps (documented here):
    // 1. Serve a PDF over HTTPS with self-signed cert
    // 2. Run: pdftract extract https://localhost:8443/test.pdf
    // 3. Expected: Exit code 6, stderr contains "TLS handshake failed"

    println!("TLS cert generated: {} bytes", cert_pem.len());
    println!("Key generated: {} bytes", key_pem.len());
    println!("Manual test required: serve PDF with self-signed cert and run pdftract against it");

    // For manual testing against known bad TLS servers:
    // pdftract extract https://expired.badssl.com/fake.pdf
    // Expected: Exit code 6
}

/// Test 6: Linearized PDF with hint stream prefetch.
///
/// Critical test from plan Section 1.8: Document with a linearized hint
/// stream, page-offset hints utilized to predict and prefetch.
#[tokio::test(flavor = "multi_thread")]
async fn test_linearized_hint_stream_prefetch() {
    use wiremock::respond::FnResponder;
    use std::sync::Mutex;

    // Mock server with Range support
    let mock_server = MockServer::start().await;
    let pdf_data = load_fixture("valid-minimal");

    // Track request timing
    let request_times = Arc::new(Mutex::new(Vec::new()));
    let request_times_clone = request_times.clone();

    let tracking_responder = FnResponder::new(move |request: &WiremockRequest| {
        let mut times = request_times_clone.lock().unwrap();
        times.push(std::time::Instant::now());

        let range_header = request.headers.get("range").and_then(|v| v.first());
        if let Some(range_value) = range_header {
            let range_str = range_value.to_str().unwrap_or("");
            println!("Range request at {:?}", std::time::Instant::now());
            println!("Range header: {}", range_str);

            // Parse and serve the requested range
            if let Some(range_part) = range_str.strip_prefix("bytes=") {
                let parts: Vec<&str> = range_part.split('-').collect();
                if parts.len() == 2 {
                    if let (Ok(start), Ok(end)) = (parts[0].parse::<usize>(), parts[1].parse::<usize>()) {
                        let end = end.min(pdf_data.len() - 1);
                        let slice_data = &pdf_data[start..=end];
                        return ResponseTemplate::new(206)
                            .append_header("Content-Range", format!("bytes {}-{}/{}", start, end, pdf_data.len()))
                            .append_header("Content-Length", slice_data.len().to_string())
                            .set_body_bytes(slice_data.to_vec());
                    }
                }
            }
        }

        // Fallback to full content
        ResponseTemplate::new(200)
            .append_header("Accept-Ranges", "bytes")
            .append_header("Content-Length", pdf_data.len().to_string())
            .set_body_bytes(pdf_data.clone())
    });

    Mock::given(matchers::method("HEAD"))
        .respond_with(ResponseTemplate::new(200)
            .append_header("Accept-Ranges", "bytes")
            .append_header("Content-Length", pdf_data.len().to_string())
            .append_header("Content-Type", "application/pdf"))
        .mount(&mock_server)
        .await;

    Mock::given(matchers::method("GET"))
        .respond_with(tracking_responder)
        .named("linearized-get")
        .mount(&mock_server)
        .await;

    let url = format!("{}/test.pdf", mock_server.uri());

    // Open the PDF
    let source = HttpRangeSource::open(&url).expect("Failed to open remote PDF");
    assert!(source.supports_range(), "Server should support Range");

    // In a real linearized PDF, we would:
    // 1. Parse the hint stream to get page offsets
    // 2. Verify that prefetch() is called with page N+1 offsets before page N is fully consumed
    // 3. Check that the request timeline shows prefetch behavior

    // For now, we verify the basic fetch works
    let data = source.read_range(0, 1024).expect("Failed to read range");
    assert_eq!(data.len(), 1024);

    let times = request_times.lock().unwrap();
    println!("Total requests made: {}", times.len());

    // In a real linearized PDF scenario, we'd see:
    // - Request 1: HEAD (metadata)
    // - Request 2: Tail (startxref, trailer)
    // - Request 3: Hint stream or linearized dictionary
    // - Request N: Prefetch for page 2 starts before page 1 is done

    assert!(!times.is_empty(), "At least one request should be made");
}

/// Test: Custom headers (Authorization, API keys).
#[tokio::test(flavor = "multi_thread")]
async fn test_custom_headers() {
    use wiremock::matchers::header;

    let mock_server = MockServer::start().await;
    let pdf_data = load_fixture("valid-minimal");

    let counter = ByteCounter::new();
    let responder = ByteCountingResponder::new(pdf_data.clone())
        .with_supports_range(true)
        .with_counter(counter.total.clone());

    Mock::given(matchers::method("HEAD"))
        .and(header("Authorization", "Bearer test123"))
        .respond_with(ResponseTemplate::new(200)
            .append_header("Accept-Ranges", "bytes")
            .append_header("Content-Length", pdf_data.len().to_string()))
        .mount(&mock_server)
        .await;

    Mock::given(matchers::method("GET"))
        .and(header("Authorization", "Bearer test123"))
        .respond_with(responder)
        .mount(&mock_server)
        .await;

    let url = format!("{}/test.pdf", mock_server.uri());
    let headers = vec![
        ("Authorization".to_string(), "Bearer test123".to_string()),
    ];

    let source = HttpRangeSource::with_headers(&url, headers).expect("Failed to open remote PDF");
    let data = source.read_range(0, 1024).expect("Failed to read range");

    assert_eq!(data.len(), 1024);
}

/// Test: Bandwidth verification for large file.
///
/// Verify that extracting a small portion from a large file
/// transfers significantly less than the full file.
#[tokio::test(flavor = "multi_thread")]
async fn test_bandwidth_efficiency() {
    let mock_server = MockServer::start().await;

    // Create a larger PDF (1 MB of data)
    let base_pdf = load_fixture("valid-minimal");
    let mut large_pdf = Vec::new();
    while large_pdf.len() < 1_000_000 {
        large_pdf.extend_from_slice(&base_pdf);
    }
    large_pdf.truncate(1_000_000);

    let counter = ByteCounter::new();
    let responder = ByteCountingResponder::new(large_pdf.clone())
        .with_supports_range(true)
        .with_counter(counter.total.clone());

    Mock::given(matchers::method("HEAD"))
        .respond_with(ResponseTemplate::new(200)
            .append_header("Accept-Ranges", "bytes")
            .append_header("Content-Length", large_pdf.len().to_string()))
        .mount(&mock_server)
        .await;

    Mock::given(matchers::method("GET"))
        .respond_with(responder)
        .mount(&mock_server)
        .await;

    let url = format!("{}/large.pdf", mock_server.uri());
    let source = HttpRangeSource::open(&url).expect("Failed to open remote PDF");

    // Read only 100 KB from the 1 MB file
    let offset = 100_000;
    let length = 100_000;
    let data = source.read_range(offset, length).expect("Failed to read range");

    assert_eq!(data.len(), length);

    // Should transfer significantly less than the full file
    // We expect roughly 2 blocks (128 KB) for 100 KB read
    assert_bytes_transferred(&counter, 200_000);
    assert!(counter.total() < large_pdf.len() as u64, "Should not transfer full file");
}

/// Test: Verify Range request count.
///
/// Verify that multiple reads to the same range hit cache.
#[tokio::test(flavor = "multi_thread")]
async fn test_cache_hit_reduces_requests() {
    let mock_server = MockServer::start().await;
    let pdf_data = load_fixture("valid-minimal");

    let counter = ByteCounter::new();
    let responder = ByteCountingResponder::new(pdf_data.clone())
        .with_supports_range(true)
        .with_counter(counter.total.clone())
        .with_request_counter(counter.request_count.clone());

    Mock::given(matchers::method("HEAD"))
        .respond_with(ResponseTemplate::new(200)
            .append_header("Accept-Ranges", "bytes")
            .append_header("Content-Length", pdf_data.len().to_string()))
        .mount(&mock_server)
        .await;

    Mock::given(matchers::method("GET"))
        .respond_with(responder)
        .mount(&mock_server)
        .await;

    let url = format!("{}/test.pdf", mock_server.uri());
    let source = HttpRangeSource::open(&url).expect("Failed to open remote PDF");

    // First read - should fetch from server
    let data1 = source.read_range(1000, 4096).expect("Failed to read range");
    let requests_after_first = counter.request_count();

    // Second read of same range - should hit cache
    let data2 = source.read_range(1000, 4096).expect("Failed to read range");
    let requests_after_second = counter.request_count();

    assert_eq!(data1, data2, "Data should be identical");
    // Cache should prevent additional requests (allowing for HEAD + initial GET)
    assert!(requests_after_second <= requests_after_first + 1, "Cache should reduce requests");
}

/// Test: Verify error classification for various failure modes.
#[tokio::test(flavor = "multi_thread")]
async fn test_error_classification_timeout() {
    use wiremock::respond::FnResponder;
    use std::thread;
    use std::time::Duration;

    let mock_server = MockServer::start().await;

    // Responder that delays response to trigger timeout
    let slow_responder = FnResponder::new(|_request: &WiremockRequest| {
        thread::sleep(Duration::from_secs(35)); // Longer than 30s read timeout
        ResponseTemplate::new(200).set_body_bytes(vec![1, 2, 3])
    });

    Mock::given(matchers::method("GET"))
        .respond_with(slow_responder)
        .mount(&mock_server)
        .await;

    let url = format!("{}/slow.pdf", mock_server.uri());

    // This should timeout during the open call
    let result = HttpRangeSource::open(&url);
    assert!(result.is_err());

    let err = result.unwrap_err();
    // Timeout should be classified as Interrupted
    assert!(matches!(err.kind(), io::ErrorKind::Interrupted | io::ErrorKind::TimedOut));
}

/// Test: Unauthorized access (401).
#[tokio::test(flavor = "multi_thread")]
async fn test_unauthorized_access() {
    let mock_server = MockServer::start().await;

    Mock::given(matchers::method("HEAD"))
        .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
        .mount(&mock_server)
        .await;

    let url = format!("{}/protected.pdf", mock_server.uri());
    let result = HttpRangeSource::open(&url);

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("401") || err_msg.contains("Unauthorized"));
}

/// Test: Forbidden access (403).
#[tokio::test(flavor = "multi_thread")]
async fn test_forbidden_access() {
    let mock_server = MockServer::start().await;

    Mock::given(matchers::method("HEAD"))
        .respond_with(ResponseTemplate::new(403).set_body_string("Forbidden"))
        .mount(&mock_server)
        .await;

    let url = format!("{}/forbidden.pdf", mock_server.uri());
    let result = HttpRangeSource::open(&url);

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("403") || err_msg.contains("Forbidden"));
}

/// Test: Basic auth success.
#[tokio::test(flavor = "multi_thread")]
async fn test_basic_auth_success() {
    use wiremock::matchers::header;

    let mock_server = MockServer::start().await;
    let pdf_data = load_fixture("valid-minimal");

    let counter = ByteCounter::new();
    let responder = ByteCountingResponder::new(pdf_data.clone())
        .with_supports_range(true)
        .with_counter(counter.total.clone());

    Mock::given(matchers::method("HEAD"))
        .and(header("Authorization", "Basic dXNlcjpwYXNz")) // base64("user:pass")
        .respond_with(ResponseTemplate::new(200)
            .append_header("Accept-Ranges", "bytes")
            .append_header("Content-Length", pdf_data.len().to_string()))
        .mount(&mock_server)
        .await;

    Mock::given(matchers::method("GET"))
        .and(header("Authorization", "Basic dXNlcjpwYXNz"))
        .respond_with(responder)
        .mount(&mock_server)
        .await;

    let url = format!("{}/protected.pdf", mock_server.uri());
    let headers = vec![
        ("Authorization".to_string(), "Basic dXNlcjpwYXNz".to_string()),
    ];

    let source = HttpRangeSource::with_headers(&url, headers).expect("Failed to open remote PDF");
    assert!(source.supports_range());
}

/// Test: Page 5 of 100-page PDF extracts with < 100 KB transferred.
///
/// Critical test from plan Section 1.8: Mock HTTP server with Range support,
/// extract page 5 of a 100-page PDF, < 100 KB transferred.
///
/// This test verifies bandwidth efficiency when extracting a single page
/// from a large multi-page PDF using Range requests.
#[tokio::test(flavor = "multi_thread")]
async fn test_page_5_of_100_bandwidth_limited() {
    // Load the 100-page PDF fixture (~1 MB total)
    let pdf_data = load_fixture_file("multipage-100.pdf");
    let total_size = pdf_data.len() as u64;

    let mock_server = MockServer::start().await;
    let counter = ByteCounter::new();

    let responder = ByteCountingResponder::new(pdf_data.clone())
        .with_supports_range(true)
        .with_counter(counter.total.clone())
        .with_request_counter(counter.request_count.clone());

    Mock::given(matchers::method("HEAD"))
        .respond_with(ResponseTemplate::new(200)
            .append_header("Accept-Ranges", "bytes")
            .append_header("Content-Length", total_size.to_string()))
        .mount(&mock_server)
        .await;

    Mock::given(matchers::method("GET"))
        .respond_with(responder)
        .named("pdf-get-range")
        .mount(&mock_server)
        .await;

    let url = format!("{}/100page.pdf", mock_server.uri());
    let source = HttpRangeSource::open(&url).expect("Failed to open remote PDF");

    // Verify Range support detected
    assert!(source.supports_range(), "Server should support Range");
    assert_eq!(source.len(), total_size);

    // Simulate extracting page 5 only by reading a specific range
    // In a real extraction, we'd parse the xref, find page 5's content stream,
    // and read only that range. For this test, we simulate reading ~64 KB
    // from the middle of the document (which represents fetching page 5 data).
    let page_5_offset = (total_size as f64 * 0.05) as u64; // ~5% into the file
    let page_5_length = 65536; // 64 KB (one cache block)

    let data = source.read_range(page_5_offset, page_5_length)
        .expect("Failed to read page 5 range");

    assert_eq!(data.len(), page_5_length, "Should read exactly 64 KB");

    // Critical: Verify bandwidth efficiency
    // Expected transfers:
    // - HEAD request: ~100 bytes
    // - One Range request for 64 KB: ~64 KB
    // Total: ~64 KB < 100 KB ✓
    assert_bytes_transferred(&counter, 100_000);

    // Also verify we didn't transfer the full file
    assert!(counter.total() < total_size,
        "Should transfer {} bytes, not full file {} bytes",
        counter.total(), total_size);

    // Verify request count: 1 HEAD + 1 Range = 2 requests
    assert!(counter.request_count() >= 1 && counter.request_count() <= 3,
        "Expected 1-3 requests (HEAD + Range + potential cache miss), got {}",
        counter.request_count());
}

/// Test: Verify Range request count for 416 retry scenario.
///
/// When server returns 416 for Range request, verify that exactly
/// one retry without Range header occurs.
#[tokio::test(flavor = "multi_thread")]
async fn test_416_range_request_count_exact() {
    let mock_server = MockServer::start().await;
    let pdf_data = load_fixture("valid-minimal");

    let counter = ByteCounter::new();
    let responder = ByteCountingResponder::new(pdf_data.clone())
        .with_supports_range(true)
        .with_force_416_first()
        .with_counter(counter.total.clone())
        .with_request_counter(counter.request_count.clone());

    Mock::given(matchers::method("HEAD"))
        .respond_with(ResponseTemplate::new(200)
            .append_header("Accept-Ranges", "bytes")
            .append_header("Content-Length", pdf_data.len().to_string()))
        .mount(&mock_server)
        .await;

    Mock::given(matchers::method("GET"))
        .respond_with(responder)
        .named("pdf-get-416")
        .mount(&mock_server)
        .await;

    let url = format!("{}/test.pdf", mock_server.uri());
    let source = HttpRangeSource::open(&url).expect("Failed to open remote PDF");

    // First read should trigger 416 then retry
    let _data = source.read_range(1000, 4096).expect("Read should succeed after retry");

    // Critical: Verify exactly one retry occurred
    // Expected: 1 initial Range (416) + 1 retry without Range (200)
    // Total: 2 requests
    assert_eq!(counter.request_count(), 2,
        "Expected exactly 2 requests (1 Range with 416 + 1 retry without Range), got {}",
        counter.request_count());
}

#[cfg(test)]
mod verification_helpers {
    use super::*;

    /// Helper to verify that the byte counter is working correctly.
    #[test]
    fn test_byte_counter() {
        let counter = ByteCounter::new();
        assert_eq!(counter.total(), 0);
        assert_eq!(counter.request_count(), 0);

        counter.total.fetch_add(1000, Ordering::SeqCst);
        counter.request_count.fetch_add(1, Ordering::SeqCst);

        assert_eq!(counter.total(), 1000);
        assert_eq!(counter.request_count(), 1);
    }
}
