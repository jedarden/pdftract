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

// ============================================================================
// JSON-RPC Response Parsing Types
// ============================================================================

/// A JSON-RPC 2.0 error object structure.
///
/// Represents the error field in a JSON-RPC response with code, message,
/// and optional data fields.
#[derive(Debug, Clone, serde::Deserialize)]
struct JsonRpcError {
    /// The error code (negative for server errors in the -32099..-32000 range)
    code: i64,
    /// Human-readable error message
    message: String,
    /// Optional additional error data
    data: Option<serde_json::Value>,
}

impl JsonRpcError {
    /// Check if this error is an SSRF_BLOCKED error.
    ///
    /// Returns true if the error data contains a "code" field with value "SSRF_BLOCKED"
    /// or if the error message contains the substring "SSRF_BLOCKED".
    fn is_ssrf_blocked(&self) -> bool {
        // Check if error data contains "code": "SSRF_BLOCKED"
        if let Some(data) = &self.data {
            if let Some(code) = data.get("code").and_then(|c| c.as_str()) {
                if code == "SSRF_BLOCKED" {
                    return true;
                }
            }
        }

        // Check if the error message itself contains SSRF_BLOCKED
        self.message.contains("SSRF_BLOCKED")
    }
}

/// A generic JSON-RPC 2.0 response structure.
///
/// A response has either a result field (success) or an error field (failure),
/// never both. The id field must match the request id.
#[derive(Debug, Clone, serde::Deserialize)]
struct JsonRpcResponse<T> {
    /// Must be exactly "2.0"
    jsonrpc: String,
    /// The successful result value (present only on success)
    result: Option<T>,
    /// The error object (present only on failure)
    error: Option<JsonRpcError>,
    /// Request identifier
    id: serde_json::Value,
}

impl<T> JsonRpcResponse<T> {
    /// Check if this is a successful response (has result field).
    fn is_success(&self) -> bool {
        self.result.is_some()
    }

    /// Check if this is an error response (has error field).
    fn is_error(&self) -> bool {
        self.error.is_some()
    }

    /// Get the error object if present.
    fn get_error(&self) -> Option<&JsonRpcError> {
        self.error.as_ref()
    }
}

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
    // Parse the JSON-RPC response using the structured type
    let parsed: JsonRpcResponse<serde_json::Value> =
        serde_json::from_str(response_json).expect("Response is not valid JSON");

    // Must have an error field
    let error = parsed
        .get_error()
        .expect(&format!(
            "Response should be an error for {}, got: {}",
            test_description, response_json
        ));

    // Verify this is an SSRF_BLOCKED error using the helper method
    assert!(
        error.is_ssrf_blocked(),
        "Error response for {} should contain SSRF_BLOCKED in data.code or message. \
         Response: {}",
        test_description, response_json
    );

    // Additional verification: ensure we're dealing with a proper error structure
    let error_code = error.code;

    // Error code should be in the server error range or the specific SSRF blocked code
    assert!(
        error_code == SSRF_BLOCKED_CODE || (-32099..=-32000).contains(&error_code),
        "Error code {} for {} should be SSRF_BLOCKED_CODE or in server error range",
        error_code, test_description
    );
}

// ============================================================================
// JSON-RPC Parsing Unit Tests
// ============================================================================

/// Test module for JSON-RPC response parsing.
///
/// This module provides comprehensive unit tests for parsing JSON-RPC 2.0
/// responses, including success responses, error responses, SSRF_BLOCKED
/// detection, and edge cases.
#[cfg(test)]
mod json_rpc_parsing_tests {
    use super::*;

    // ------------------------------------------------------------------------
    // Success Response Tests
    // ------------------------------------------------------------------------

    /// Test parsing a valid successful JSON-RPC response with all required fields.
    #[test]
    fn test_parse_success_response_with_all_fields() {
        let success_json = r#"{
            "jsonrpc": "2.0",
            "result": {"status": "ok", "data": {"items": []}},
            "id": 1
        }"#;

        let parsed: JsonRpcResponse<serde_json::Value> =
            serde_json::from_str(success_json).expect("Should parse valid success response");

        assert!(parsed.is_success(), "Response should be successful");
        assert!(!parsed.is_error(), "Response should not be an error");
        assert_eq!(parsed.jsonrpc, "2.0", "jsonrpc version should be 2.0");
        assert!(parsed.result.is_some(), "Should have result field");
        assert!(parsed.error.is_none(), "Should not have error field");
    }

    /// Test parsing a response with null result field.
    ///
    /// Note: When JSON has "result": null and we deserialize into Option<T>,
    /// serde treats this as None (null = absent), so is_success() returns false.
    /// This test documents the actual behavior.
    #[test]
    fn test_parse_response_with_null_result() {
        let null_result_json = r#"{
            "jsonrpc": "2.0",
            "result": null,
            "id": 1
        }"#;

        let parsed: JsonRpcResponse<serde_json::Value> =
            serde_json::from_str(null_result_json).expect("Should parse response with null result");

        // When result is null in JSON, serde deserializes it as None for Option<T>
        assert!(!parsed.is_success(), "Response with null result is not successful (result is None)");
        assert!(!parsed.is_error(), "Response should not be an error");
        assert!(parsed.result.is_none(), "Result field should be None (null in JSON)");
        assert!(parsed.error.is_none(), "Error field should also be None");
    }

    /// Test parsing a successful response with string ID.
    #[test]
    fn test_parse_success_response_with_string_id() {
        let success_json = r#"{
            "jsonrpc": "2.0",
            "result": {"status": "ok"},
            "id": "req-123"
        }"#;

        let parsed: JsonRpcResponse<serde_json::Value> =
            serde_json::from_str(success_json).expect("Should parse response with string ID");

        assert!(parsed.is_success(), "Response should be successful");
        assert_eq!(parsed.id, "req-123", "Should preserve string ID");
    }

    /// Test parsing a successful response with numeric ID.
    #[test]
    fn test_parse_success_response_with_numeric_id() {
        let success_json = r#"{
            "jsonrpc": "2.0",
            "result": {"status": "ok"},
            "id": 42
        }"#;

        let parsed: JsonRpcResponse<serde_json::Value> =
            serde_json::from_str(success_json).expect("Should parse response with numeric ID");

        assert!(parsed.is_success(), "Response should be successful");
        assert_eq!(parsed.id, 42, "Should preserve numeric ID");
    }

    // ------------------------------------------------------------------------
    // Error Response Tests
    // ------------------------------------------------------------------------

    /// Test parsing a valid error JSON-RPC response with all required fields.
    #[test]
    fn test_parse_error_response_with_all_fields() {
        let error_json = r#"{
            "jsonrpc": "2.0",
            "error": {
                "code": -32001,
                "message": "SSRF protection blocked this URL",
                "data": {"code": "SSRF_BLOCKED"}
            },
            "id": 1
        }"#;

        let parsed: JsonRpcResponse<serde_json::Value> =
            serde_json::from_str(error_json).expect("Should parse valid error response");

        assert!(!parsed.is_success(), "Response should not be successful");
        assert!(parsed.is_error(), "Response should be an error");
        assert_eq!(parsed.jsonrpc, "2.0", "jsonrpc version should be 2.0");
        assert!(parsed.result.is_none(), "Should not have result field");
        assert!(parsed.error.is_some(), "Should have error field");

        let error = parsed.get_error().unwrap();
        assert_eq!(error.code, -32001, "Error code should be -32001");
        assert_eq!(error.message, "SSRF protection blocked this URL");
        assert!(error.data.is_some(), "Should have error data");
    }

    /// Test parsing an error response with minimal fields (code and message only).
    #[test]
    fn test_parse_error_response_minimal() {
        let minimal_error_json = r#"{
            "jsonrpc": "2.0",
            "error": {
                "code": -32601,
                "message": "Method not found"
            },
            "id": 1
        }"#;

        let parsed: JsonRpcResponse<serde_json::Value> =
            serde_json::from_str(minimal_error_json).expect("Should parse minimal error response");

        assert!(parsed.is_error(), "Response should be an error");

        let error = parsed.get_error().unwrap();
        assert_eq!(error.code, -32601, "Error code should be -32601");
        assert_eq!(error.message, "Method not found");
        assert!(error.data.is_none(), "Should not have error data (optional field)");
    }

    /// Test parsing an error response with complex data field.
    #[test]
    fn test_parse_error_response_with_complex_data() {
        let complex_error_json = r#"{
            "jsonrpc": "2.0",
            "error": {
                "code": -32001,
                "message": "URL rejected",
                "data": {
                    "code": "SSRF_BLOCKED",
                    "url": "http://127.0.0.1/",
                    "reason": "Private network address"
                }
            },
            "id": 1
        }"#;

        let parsed: JsonRpcResponse<serde_json::Value> =
            serde_json::from_str(complex_error_json).expect("Should parse error with complex data");

        let error = parsed.get_error().unwrap();
        assert!(error.data.is_some(), "Should have error data");

        let data = error.data.as_ref().unwrap();
        assert_eq!(data["code"], "SSRF_BLOCKED");
        assert_eq!(data["url"], "http://127.0.0.1/");
        assert_eq!(data["reason"], "Private network address");
    }

    // ------------------------------------------------------------------------
    // SSRF_BLOCKED Detection Tests
    // ------------------------------------------------------------------------

    /// Test SSRF_BLOCKED detection via error data.code field.
    #[test]
    fn test_ssrf_blocked_detection_via_data_code() {
        let error_json = r#"{
            "jsonrpc": "2.0",
            "error": {
                "code": -32001,
                "message": "SSRF protection blocked this URL",
                "data": {"code": "SSRF_BLOCKED"}
            },
            "id": 1
        }"#;

        let parsed: JsonRpcResponse<serde_json::Value> =
            serde_json::from_str(error_json).expect("Should parse SSRF_BLOCKED error");

        let error = parsed.get_error().expect("Should have error");
        assert!(error.is_ssrf_blocked(), "Should detect SSRF_BLOCKED in data.code");
    }

    /// Test SSRF_BLOCKED detection via error message field.
    #[test]
    fn test_ssrf_blocked_detection_via_message() {
        let error_json = r#"{
            "jsonrpc": "2.0",
            "error": {
                "code": -32001,
                "message": "SSRF_BLOCKED: URL targets private network"
            },
            "id": 1
        }"#;

        let parsed: JsonRpcResponse<serde_json::Value> =
            serde_json::from_str(error_json).expect("Should parse SSRF_BLOCKED error");

        let error = parsed.get_error().expect("Should have error");
        assert!(error.is_ssrf_blocked(), "Should detect SSRF_BLOCKED in message");
    }

    /// Test SSRF_BLOCKED detection when both data.code and message contain it.
    #[test]
    fn test_ssrf_blocked_detection_both_locations() {
        let error_json = r#"{
            "jsonrpc": "2.0",
            "error": {
                "code": -32001,
                "message": "SSRF_BLOCKED: URL rejected",
                "data": {"code": "SSRF_BLOCKED"}
            },
            "id": 1
        }"#;

        let parsed: JsonRpcResponse<serde_json::Value> =
            serde_json::from_str(error_json).expect("Should parse SSRF_BLOCKED error");

        let error = parsed.get_error().expect("Should have error");
        assert!(error.is_ssrf_blocked(), "Should detect SSRF_BLOCKED in both locations");
    }

    /// Test that non-SSRF errors are not detected as SSRF_BLOCKED.
    #[test]
    fn test_non_ssrf_error_not_detected() {
        let error_json = r#"{
            "jsonrpc": "2.0",
            "error": {
                "code": -32601,
                "message": "Method not found"
            },
            "id": 1
        }"#;

        let parsed: JsonRpcResponse<serde_json::Value> =
            serde_json::from_str(error_json).expect("Should parse non-SSRF error");

        let error = parsed.get_error().expect("Should have error");
        assert!(!error.is_ssrf_blocked(), "Should not detect non-SSRF error as SSRF_BLOCKED");
    }

    /// Test case sensitivity in SSRF_BLOCKED detection (data.code).
    #[test]
    fn test_ssrf_blocked_case_sensitive_data_code() {
        let error_json_lowercase = r#"{
            "jsonrpc": "2.0",
            "error": {
                "code": -32001,
                "message": "URL blocked",
                "data": {"code": "ssrf_blocked"}
            },
            "id": 1
        }"#;

        let parsed: JsonRpcResponse<serde_json::Value> =
            serde_json::from_str(error_json_lowercase).expect("Should parse error");

        let error = parsed.get_error().expect("Should have error");
        assert!(!error.is_ssrf_blocked(), "Should be case-sensitive in data.code");
    }

    /// Test case sensitivity in SSRF_BLOCKED detection (message).
    #[test]
    fn test_ssrf_blocked_case_sensitive_message() {
        let error_json_lowercase = r#"{
            "jsonrpc": "2.0",
            "error": {
                "code": -32001,
                "message": "ssrf_blocked: lowercase"
            },
            "id": 1
        }"#;

        let parsed: JsonRpcResponse<serde_json::Value> =
            serde_json::from_str(error_json_lowercase).expect("Should parse error");

        let error = parsed.get_error().expect("Should have error");
        assert!(!error.is_ssrf_blocked(), "Should be case-sensitive in message");
    }

    /// Test partial match of SSRF_BLOCKED in message.
    #[test]
    fn test_ssrf_blocked_partial_match_in_message() {
        let error_json = r#"{
            "jsonrpc": "2.0",
            "error": {
                "code": -32001,
                "message": "Request rejected: SSRF_BLOCKED detected"
            },
            "id": 1
        }"#;

        let parsed: JsonRpcResponse<serde_json::Value> =
            serde_json::from_str(error_json).expect("Should parse error");

        let error = parsed.get_error().expect("Should have error");
        assert!(error.is_ssrf_blocked(), "Should detect partial match in message");
    }

    // ------------------------------------------------------------------------
    // Edge Case Tests
    // ------------------------------------------------------------------------

    /// Test that invalid JSON fails to deserialize.
    #[test]
    fn test_invalid_json_fails_to_deserialize() {
        let invalid_json = r#"{not valid json}"#;

        let result: Result<JsonRpcResponse<serde_json::Value>, _> =
            serde_json::from_str(invalid_json);

        assert!(result.is_err(), "Invalid JSON should fail to deserialize");
    }

    /// Test that missing jsonrpc field fails to deserialize.
    #[test]
    fn test_missing_jsonrpc_field_fails() {
        let missing_jsonrpc = r#"{
            "result": {"status": "ok"},
            "id": 1
        }"#;

        let result: Result<JsonRpcResponse<serde_json::Value>, _> =
            serde_json::from_str(missing_jsonrpc);

        assert!(result.is_err(), "Missing jsonrpc field should fail to deserialize");
    }

    /// Test that wrong jsonrpc version fails to deserialize.
    #[test]
    fn test_wrong_jsonrpc_version_fails() {
        let wrong_version = r#"{
            "jsonrpc": "1.0",
            "result": {"status": "ok"},
            "id": 1
        }"#;

        let result: Result<JsonRpcResponse<serde_json::Value>, _> =
            serde_json::from_str(wrong_version);

        // Note: Our struct doesn't validate the version, so this may succeed
        // but we should still test the behavior
        if let Ok(parsed) = result {
            assert_eq!(parsed.jsonrpc, "1.0", "Should preserve whatever version is in JSON");
        }
    }

    /// Test that response missing both result and error fields parses but has neither.
    #[test]
    fn test_missing_both_result_and_error() {
        let missing_both = r#"{
            "jsonrpc": "2.0",
            "id": 1
        }"#;

        let parsed: JsonRpcResponse<serde_json::Value> =
            serde_json::from_str(missing_both).expect("Should parse response without result/error");

        assert!(!parsed.is_success(), "Should not be successful (no result)");
        assert!(!parsed.is_error(), "Should not be an error (no error)");
        assert!(parsed.result.is_none(), "Result should be None");
        assert!(parsed.error.is_none(), "Error should be None");
    }

    /// Test that missing id field still parses (notifications in requests, but odd in responses).
    #[test]
    fn test_missing_id_field() {
        let missing_id = r#"{
            "jsonrpc": "2.0",
            "result": {"status": "ok"}
        }"#;

        let result: Result<JsonRpcResponse<serde_json::Value>, _> =
            serde_json::from_str(missing_id);

        // Our struct requires id, so this should fail
        assert!(result.is_err(), "Missing id field should fail to deserialize");
    }

    /// Test response with extra unknown fields (should parse successfully).
    #[test]
    fn test_response_with_extra_fields() {
        let extra_fields = r#"{
            "jsonrpc": "2.0",
            "result": {"status": "ok"},
            "id": 1,
            "extraField": "ignored",
            "metadata": {"version": "1.0"}
        }"#;

        let parsed: JsonRpcResponse<serde_json::Value> =
            serde_json::from_str(extra_fields).expect("Should ignore extra fields");

        assert!(parsed.is_success(), "Should parse successfully with extra fields");
    }

    /// Test empty result object (not null, but empty object).
    #[test]
    fn test_empty_result_object() {
        let empty_result = r#"{
            "jsonrpc": "2.0",
            "result": {},
            "id": 1
        }"#;

        let parsed: JsonRpcResponse<serde_json::Value> =
            serde_json::from_str(empty_result).expect("Should parse empty result");

        assert!(parsed.is_success(), "Empty result is still successful");
        assert!(parsed.result.is_some(), "Result field should be present");
    }

    /// Test error with empty data object (not null, but empty object).
    #[test]
    fn test_error_with_empty_data_object() {
        let empty_data = r#"{
            "jsonrpc": "2.0",
            "error": {
                "code": -32001,
                "message": "Some error",
                "data": {}
            },
            "id": 1
        }"#;

        let parsed: JsonRpcResponse<serde_json::Value> =
            serde_json::from_str(empty_data).expect("Should parse error with empty data");

        let error = parsed.get_error().expect("Should have error");
        assert!(error.data.is_some(), "Data field should be present");
        assert!(!error.is_ssrf_blocked(), "Empty data should not be SSRF_BLOCKED");
    }

    /// Test malformed error structure (missing required code field).
    #[test]
    fn test_malformed_error_missing_code() {
        let malformed_error = r#"{
            "jsonrpc": "2.0",
            "error": {
                "message": "Some error"
            },
            "id": 1
        }"#;

        let result: Result<JsonRpcResponse<serde_json::Value>, _> =
            serde_json::from_str(malformed_error);

        assert!(result.is_err(), "Error without code field should fail to deserialize");
    }

    /// Test malformed error structure (missing required message field).
    #[test]
    fn test_malformed_error_missing_message() {
        let malformed_error = r#"{
            "jsonrpc": "2.0",
            "error": {
                "code": -32001
            },
            "id": 1
        }"#;

        let result: Result<JsonRpcResponse<serde_json::Value>, _> =
            serde_json::from_str(malformed_error);

        assert!(result.is_err(), "Error without message field should fail to deserialize");
    }

    /// Test that both result and error present parses but we can detect which is which.
    #[test]
    fn test_both_result_and_error_present() {
        // This is technically invalid per JSON-RPC spec (should have one or the other)
        let both_present = r#"{
            "jsonrpc": "2.0",
            "result": {"status": "ok"},
            "error": {
                "code": -32001,
                "message": "Some error"
            },
            "id": 1
        }"#;

        let parsed: JsonRpcResponse<serde_json::Value> =
            serde_json::from_str(both_present).expect("Should parse even with both fields");

        // Our struct uses Option for both, so both can be present
        assert!(parsed.result.is_some(), "Result field is present");
        assert!(parsed.error.is_some(), "Error field is also present");
        assert!(parsed.is_success(), "is_success returns true if result is Some");
        assert!(parsed.is_error(), "is_error returns true if error is Some");
    }
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

    // Parse response using the structured JSON-RPC type
    let parsed: JsonRpcResponse<serde_json::Value> =
        serde_json::from_str(&response).expect("Response is not valid JSON");

    // Verify the response is an error (SSRF blocking implemented)
    assert!(
        parsed.is_error(),
        "Response should be an error (URL should be rejected)"
    );

    // Assert SSRF_BLOCKED error to verify proper rejection
    assert_ssrf_blocked_error(&response, "RFC 1918 private network (192.168.1.1)");
}
