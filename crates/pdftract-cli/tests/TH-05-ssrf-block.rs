//! TH-05: SSRF blocking test — verifies MCP server rejects private-network URLs.
//!
//! This test validates the TH-05 mitigation: the MCP `extract` tool refuses
//! to fetch URLs targeting internal services (localhost, private IPs, link-local,
//! cloud metadata endpoints). Requests must be https://; http:// is rejected.
//! Private network ranges are refused unless `--allow-private-networks` is set.
//!
//! Test coverage:
//! - IPv4 loopback (127.0.0.1)
//! - IPv4 wildcard (0.0.0.0)
//! - IPv4 link-local (169.254.169.254 - cloud metadata)
//! - IPv4 private (10.0.0.1)
//! - IPv6 loopback ([::1])

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

/// Path to the pdftract binary.
const PDFTRACT: &str = env!("CARGO_BIN_EXE_pdftract");

/// Expected error code for SSRF blocking.
/// This should match the code returned by the MCP server when a URL is blocked.
const SSRF_BLOCKED_CODE: i64 = -32001;

/// Helper: RAII guard for MCP server process.
///
/// Ensures the child process is killed and cleaned up when the guard drops,
/// even if a test panics. Uses bounded waits to avoid hanging.
struct McpServerGuard {
    child: Option<std::process::Child>,
}

impl McpServerGuard {
    /// Create a new guard from a spawned child process.
    fn new(child: std::process::Child) -> Self {
        Self { child: Some(child) }
    }

    /// Get a mutable reference to the child process.
    fn child_mut(&mut self) -> &mut std::process::Child {
        self.child.as_mut().expect("Child process already taken")
    }
}

impl Drop for McpServerGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            // Close stdin to signal EOF (graceful shutdown)
            let _ = child.stdin.take();

            // Wait for graceful shutdown with bounded timeout
            let start = std::time::Instant::now();
            let exited = loop {
                match child.try_wait() {
                    Ok(Some(_)) => break true,
                    Ok(None) => {
                        if start.elapsed() >= Duration::from_millis(200) {
                            break false;
                        }
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => break false,
                }
            };

            // If graceful shutdown failed, force kill and wait with bounded timeout
            if !exited {
                let _ = child.kill();
                // Wait with bounded timeout after kill - never use bare wait()
                let kill_start = std::time::Instant::now();
                let _ = loop {
                    match child.try_wait() {
                        Ok(Some(_)) => break Ok(()),
                        Ok(None) => {
                            if kill_start.elapsed() >= Duration::from_millis(100) {
                                break Err(std::io::Error::new(
                                    std::io::ErrorKind::TimedOut,
                                    "Process did not exit after kill within 100ms"
                                ));
                            }
                            thread::sleep(Duration::from_millis(10));
                        }
                        Err(e) => break Err(e),
                    }
                };
            }
        }
    }
}

/// Spawn the pdftract MCP server in stdio mode.
///
/// Returns an RAII guard that ensures cleanup on drop.
/// Uses Stdio::piped() for stdin/stdout to allow JSON-RPC communication,
/// and Stdio::null() for stderr to avoid blocking on full pipe buffers.
fn spawn_mcp_server() -> McpServerGuard {
    let child = Command::new(PDFTRACT)
        .arg("mcp")
        .arg("--stdio")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null()) // Discard stderr to avoid pipe buffer blocking
        .spawn()
        .expect("Failed to spawn pdftract mcp --stdio");

    McpServerGuard::new(child)
}

/// Write a framed JSON-RPC message to stdin.
///
/// Uses the LSP-style framing: Content-Length header followed by \r\n\r\n,
/// then the JSON body (no trailing newline).
fn write_framed_message(
    stdin: &mut std::process::ChildStdin,
    json_body: &str,
) -> std::io::Result<()> {
    let header = format!("Content-Length: {}\r\n\r\n", json_body.len());
    stdin.write_all(header.as_bytes())?;
    stdin.write_all(json_body.as_bytes())?;
    stdin.flush()
}

/// Read a framed JSON-RPC response from stdout.
///
/// Returns the JSON body as a string, or None if EOF is reached.
fn read_framed_response<R: Read>(
    reader: &mut BufReader<R>,
) -> std::io::Result<Option<String>> {
    let mut content_length: Option<usize> = None;

    // Read headers until empty line
    loop {
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line)?;
        if bytes_read == 0 {
            return Ok(None); // EOF
        }

        let line = line.trim_end_matches(|c| c == '\r' || c == '\n');
        if line.is_empty() {
            break;
        }

        if let Some(value) = line.strip_prefix("Content-Length:") {
            content_length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?,
            );
        }
    }

    let content_length = content_length.ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Missing Content-Length header",
        )
    })?;

    let mut buffer = vec![0u8; content_length];
    reader.read_exact(&mut buffer)?;
    Ok(Some(String::from_utf8(buffer).map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, e)
    })?))
}

/// Construct a tools/call request for the extract tool.
fn make_extract_call_request(id: i64, url: &str) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "tools/call",
        "params": {
            "name": "extract",
            "arguments": {
                "path": url
            }
        }
    })
    .to_string()
}

/// Test case 1: IPv4 loopback (127.0.0.1) is blocked.
///
/// This test verifies that attempting to extract from 127.0.0.1 is rejected
/// with a SSRF_BLOCKED error in the JSON-RPC response.
#[test]
fn test_ipv4_loopback_blocked() {
    let mut server = spawn_mcp_server();
    thread::sleep(Duration::from_millis(50));

    let request = make_extract_call_request(1, "http://127.0.0.1:9999/doc.pdf");

    // Send request
    {
        let stdin = server.child_mut().stdin.as_mut().expect("Failed to open stdin");
        write_framed_message(stdin, &request).expect("Failed to write request");
    }

    // Read response with bounded timeout
    let response = {
        let stdout = server.child_mut().stdout.as_mut().expect("Failed to open stdout");
        let mut reader = BufReader::new(stdout);

        let start = std::time::Instant::now();
        loop {
            match read_framed_response(&mut reader) {
                Ok(Some(resp)) => break resp,
                Ok(None) => panic!("Unexpected EOF"),
                Err(e) if start.elapsed() >= Duration::from_secs(1) => {
                    panic!("Timeout waiting for response: {}", e);
                }
                Err(_) => thread::sleep(Duration::from_millis(10)),
            }
        }
    };

    // Assert SSRF_BLOCKED error (Phase 1.8 implemented)
    assert_ssrf_blocked_error(&response, "IPv4 loopback (127.0.0.1)");
}

/// Test case 2: IPv4 wildcard (0.0.0.0) is blocked.
///
/// This test verifies that attempting to extract from 0.0.0.0 is rejected
/// with a SSRF_BLOCKED error in the JSON-RPC response.
#[test]
fn test_ipv4_wildcard_blocked() {
    let mut server = spawn_mcp_server();
    thread::sleep(Duration::from_millis(50));

    let request = make_extract_call_request(2, "http://0.0.0.0/doc.pdf");

    {
        let stdin = server.child_mut().stdin.as_mut().expect("Failed to open stdin");
        write_framed_message(stdin, &request).expect("Failed to write request");
    }

    let response = {
        let stdout = server.child_mut().stdout.as_mut().expect("Failed to open stdout");
        let mut reader = BufReader::new(stdout);

        let start = std::time::Instant::now();
        loop {
            match read_framed_response(&mut reader) {
                Ok(Some(resp)) => break resp,
                Ok(None) => panic!("Unexpected EOF"),
                Err(e) if start.elapsed() >= Duration::from_secs(1) => {
                    panic!("Timeout waiting for response: {}", e);
                }
                Err(_) => thread::sleep(Duration::from_millis(10)),
            }
        }
    };

    // Assert SSRF_BLOCKED error (Phase 1.8 implemented)
    assert_ssrf_blocked_error(&response, "IPv4 wildcard (0.0.0.0)");
}

/// Test case 3: Cloud metadata endpoint (169.254.169.254) is blocked.
///
/// This test verifies that attempting to extract from the AWS metadata endpoint
/// is rejected with a SSRF_BLOCKED error in the JSON-RPC response.
#[test]
fn test_cloud_metadata_blocked() {
    let mut server = spawn_mcp_server();
    thread::sleep(Duration::from_millis(50));

    let request = make_extract_call_request(3, "http://169.254.169.254/latest/meta-data/");

    {
        let stdin = server.child_mut().stdin.as_mut().expect("Failed to open stdin");
        write_framed_message(stdin, &request).expect("Failed to write request");
    }

    let response = {
        let stdout = server.child_mut().stdout.as_mut().expect("Failed to open stdout");
        let mut reader = BufReader::new(stdout);

        let start = std::time::Instant::now();
        loop {
            match read_framed_response(&mut reader) {
                Ok(Some(resp)) => break resp,
                Ok(None) => panic!("Unexpected EOF"),
                Err(e) if start.elapsed() >= Duration::from_secs(1) => {
                    panic!("Timeout waiting for response: {}", e);
                }
                Err(_) => thread::sleep(Duration::from_millis(10)),
            }
        }
    };

    // Assert SSRF_BLOCKED error
    assert_ssrf_blocked_error(&response, "Cloud metadata endpoint (169.254.169.254)");
}

/// Test case 4: RFC 1918 private network (10.0.0.1) is blocked.
///
/// This test verifies that attempting to extract from a private network IP
/// is rejected with a SSRF_BLOCKED error in the JSON-RPC response.
#[test]
fn test_rfc1918_private_blocked() {
    let mut server = spawn_mcp_server();
    thread::sleep(Duration::from_millis(50));

    let request = make_extract_call_request(4, "http://10.0.0.1/internal/doc.pdf");

    {
        let stdin = server.child_mut().stdin.as_mut().expect("Failed to open stdin");
        write_framed_message(stdin, &request).expect("Failed to write request");
    }

    let response = {
        let stdout = server.child_mut().stdout.as_mut().expect("Failed to open stdout");
        let mut reader = BufReader::new(stdout);

        let start = std::time::Instant::now();
        loop {
            match read_framed_response(&mut reader) {
                Ok(Some(resp)) => break resp,
                Ok(None) => panic!("Unexpected EOF"),
                Err(e) if start.elapsed() >= Duration::from_secs(1) => {
                    panic!("Timeout waiting for response: {}", e);
                }
                Err(_) => thread::sleep(Duration::from_millis(10)),
            }
        }
    };

    // Assert SSRF_BLOCKED error
    assert_ssrf_blocked_error(&response, "RFC 1918 private network (10.0.0.1)");
}

/// Test case 5: IPv6 loopback ([::1]) is blocked.
///
/// This test verifies that attempting to extract from IPv6 loopback
/// is rejected with a SSRF_BLOCKED error in the JSON-RPC response.
#[test]
fn test_ipv6_loopback_blocked() {
    let mut server = spawn_mcp_server();
    thread::sleep(Duration::from_millis(50));

    let request = make_extract_call_request(5, "http://[::1]/doc.pdf");

    {
        let stdin = server.child_mut().stdin.as_mut().expect("Failed to open stdin");
        write_framed_message(stdin, &request).expect("Failed to write request");
    }

    let response = {
        let stdout = server.child_mut().stdout.as_mut().expect("Failed to open stdout");
        let mut reader = BufReader::new(stdout);

        let start = std::time::Instant::now();
        loop {
            match read_framed_response(&mut reader) {
                Ok(Some(resp)) => break resp,
                Ok(None) => panic!("Unexpected EOF"),
                Err(e) if start.elapsed() >= Duration::from_secs(1) => {
                    panic!("Timeout waiting for response: {}", e);
                }
                Err(_) => thread::sleep(Duration::from_millis(10)),
            }
        }
    };

    // Assert SSRF_BLOCKED error
    assert_ssrf_blocked_error(&response, "IPv6 loopback ([::1])");
}

// ============================================================================
// SSRF_BLOCKED Error Assertion Helper
// ============================================================================

/// Simplified assertion that strictly requires SSRF_BLOCKED error.
///
/// This version does NOT accept stub responses - it requires that Phase 1.8
/// is implemented and SSRF blocking is active. Use this for acceptance tests
/// once Phase 1.8 is complete.
///
/// # Arguments
///
/// * `response_json` - The JSON-RPC response string to check
/// * `test_description` - Description of the test case (for error messages)
fn assert_ssrf_blocked_error(response_json: &str, test_description: &str) {
    let parsed: serde_json::Value =
        serde_json::from_str(response_json).expect("Response is not valid JSON");

    // Must have an error field
    let error = parsed
        .get("error")
        .expect(&format!(
            "Response should be an error for {}, got: {}",
            test_description, response_json
        ));

    // Check if error data contains "code": "SSRF_BLOCKED"
    let has_ssrf_blocked_code = error
        .get("data")
        .and_then(|data| data.get("code"))
        .and_then(|code| code.as_str())
        .map(|code| code == "SSRF_BLOCKED")
        .unwrap_or(false);

    // Check if error message contains SSRF_BLOCKED
    let error_message = error
        .get("message")
        .and_then(|m| m.as_str())
        .unwrap_or("");
    let has_ssrf_in_message = error_message.contains("SSRF_BLOCKED");

    assert!(
        has_ssrf_blocked_code || has_ssrf_in_message,
        "Error response for {} should contain SSRF_BLOCKED in data.code or message. \
         Response: {}",
        test_description, response_json
    );

    // Additional verification: ensure we're dealing with a proper error structure
    let error_code = error
        .get("code")
        .and_then(|c| c.as_i64())
        .expect("Error should have a numeric code");

    // Error code should be in the server error range or the specific SSRF blocked code
    assert!(
        error_code == SSRF_BLOCKED_CODE || (-32099..=-32000).contains(&error_code),
        "Error code {} for {} should be SSRF_BLOCKED_CODE or in server error range",
        error_code, test_description
    );
}

/// Test case 6: Verify http:// scheme is rejected (https:// required).
///
/// This test verifies that attempting to use http:// scheme (even with a
/// public hostname) is rejected with a SSRF_BLOCKED error.
#[test]
fn test_http_scheme_rejected() {
    let mut server = spawn_mcp_server();
    thread::sleep(Duration::from_millis(50));

    // Use a public hostname but with http:// scheme (should be rejected)
    let request = make_extract_call_request(6, "http://example.com/doc.pdf");

    {
        let stdin = server.child_mut().stdin.as_mut().expect("Failed to open stdin");
        write_framed_message(stdin, &request).expect("Failed to write request");
    }

    let response = {
        let stdout = server.child_mut().stdout.as_mut().expect("Failed to open stdout");
        let mut reader = BufReader::new(stdout);

        let start = std::time::Instant::now();
        loop {
            match read_framed_response(&mut reader) {
                Ok(Some(resp)) => break resp,
                Ok(None) => panic!("Unexpected EOF"),
                Err(e) if start.elapsed() >= Duration::from_secs(1) => {
                    panic!("Timeout waiting for response: {}", e);
                }
                Err(_) => thread::sleep(Duration::from_millis(10)),
            }
        }
    };

    // Assert SSRF_BLOCKED error
    assert_ssrf_blocked_error(&response, "http:// scheme (not https)");
}

/// Test case 7: Verify no network connections are attempted.
///
/// This test verifies that when SSRF-prone URLs are rejected, no actual
/// network connection is made. We ensure this by checking:
/// 1. The response is quick (< 500ms) — no network timeout
/// 2. The response is an error (not a successful result)
/// 3. The error contains SSRF_BLOCKED
#[test]
fn test_no_network_connection_attempted() {
    let mut server = spawn_mcp_server();
    thread::sleep(Duration::from_millis(50));

    let request = make_extract_call_request(7, "http://192.168.1.1/nonexistent.pdf");

    {
        let stdin = server.child_mut().stdin.as_mut().expect("Failed to open stdin");
        write_framed_message(stdin, &request).expect("Failed to write request");
    }

    // Measure response time
    let start = std::time::Instant::now();

    let response = {
        let stdout = server.child_mut().stdout.as_mut().expect("Failed to open stdout");
        let mut reader = BufReader::new(stdout);

        loop {
            match read_framed_response(&mut reader) {
                Ok(Some(resp)) => break resp,
                Ok(None) => panic!("Unexpected EOF"),
                Err(e) if start.elapsed() >= Duration::from_secs(1) => {
                    panic!("Timeout waiting for response: {}", e);
                }
                Err(_) => thread::sleep(Duration::from_millis(10)),
            }
        }
    };

    let elapsed = start.elapsed();

    // Response should be quick (< 500ms) since no network call is made
    assert!(
        elapsed < Duration::from_millis(500),
        "Response took too long ({}ms), suggesting a network connection was attempted",
        elapsed.as_millis()
    );

    // Parse response to verify it's not a success result
    let parsed: serde_json::Value =
        serde_json::from_str(&response).expect("Response is not valid JSON");

    // Verify the response is an error (SSRF blocking implemented)
    assert!(
        parsed.get("error").is_some(),
        "Response should be an error (URL should be rejected)"
    );

    // Assert SSRF_BLOCKED error to verify proper rejection
    assert_ssrf_blocked_error(&response, "RFC 1918 private network (192.168.1.1)");
}
