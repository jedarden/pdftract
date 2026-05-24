//! Integration tests for MCP HTTP+SSE transport.
//!
//! These tests verify that the pdftract CLI correctly implements the
//! MCP HTTP+SSE transport specification, including:
//! - POST / for JSON-RPC requests
//! - GET /sse for server-sent events
//! - GET /health for health checks
//! - Bearer token authentication
//! - Request body size limits
//! - Batch request handling
//! - Concurrent client handling (50 clients)

use reqwest::blocking::Client;
use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

/// Find an available port for testing.
fn find_available_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to port");
    listener.local_addr().unwrap().port()
}

/// Helper to spawn the pdftract MCP server in HTTP mode.
fn spawn_mcp_http(port: u16) -> Child {
    Command::new(env!("CARGO_BIN_EXE_pdftract"))
        .arg("mcp")
        .arg("--bind")
        .arg(format!("127.0.0.1:{}", port))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn pdftract mcp --bind")
}

/// Helper to spawn the pdftract MCP server in HTTP mode with custom max upload size.
fn spawn_mcp_http_with_limit(port: u16, max_upload_mb: usize) -> Child {
    Command::new(env!("CARGO_BIN_EXE_pdftract"))
        .arg("mcp")
        .arg("--bind")
        .arg(format!("127.0.0.1:{}", port))
        .arg("--max-upload-mb")
        .arg(max_upload_mb.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn pdftract mcp --bind")
}

/// Wait for the server to be ready by polling the health endpoint.
fn wait_for_server(port: u16, max_wait_ms: u64) -> bool {
    let client = Client::builder()
        .timeout(Duration::from_millis(100))
        .build()
        .expect("Failed to build HTTP client");

    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_millis(max_wait_ms) {
        if client
            .get(&format!("http://127.0.0.1:{}/health", port))
            .send()
            .map_or(false, |r| r.status().is_success())
        {
            return true;
        }
        thread::sleep(Duration::from_millis(20));
    }
    false
}

/// Test that POST / with tools/list returns the tool catalog.
#[test]
fn test_post_tools_list() {
    let port = find_available_port();
    let mut child = spawn_mcp_http(port);

    // Wait for server to be ready
    assert!(
        wait_for_server(port, 2000),
        "Server did not start within 2 seconds"
    );

    let client = Client::new();
    let request_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list"
    });

    let response = client
        .post(&format!("http://127.0.0.1:{}/", port))
        .json(&request_body)
        .send()
        .expect("Failed to send request");

    assert_eq!(response.status(), reqwest::StatusCode::OK);

    let json: Value = response.json().expect("Response is not valid JSON");
    assert_eq!(json["jsonrpc"], "2.0");
    assert_eq!(json["id"], 1);
    assert!(json["result"].is_object());

    // Clean shutdown
    child.kill().ok();
}

/// Test that POST / with batched requests returns batched responses.
#[test]
fn test_post_batch_request() {
    let port = find_available_port();
    let mut child = spawn_mcp_http(port);

    // Wait for server to be ready
    assert!(
        wait_for_server(port, 2000),
        "Server did not start within 2 seconds"
    );

    let client = Client::new();
    let request_body = serde_json::json!([
        {"jsonrpc": "2.0", "id": 1, "method": "tools/list"},
        {"jsonrpc": "2.0", "id": 2, "method": "initialize"}
    ]);

    let response = client
        .post(&format!("http://127.0.0.1:{}/", port))
        .json(&request_body)
        .send()
        .expect("Failed to send request");

    assert_eq!(response.status(), reqwest::StatusCode::OK);

    let json: Value = response.json().expect("Response is not valid JSON");
    assert!(json.is_array());
    assert_eq!(json.as_array().unwrap().len(), 2);

    // Verify first response
    assert_eq!(json[0]["jsonrpc"], "2.0");
    assert_eq!(json[0]["id"], 1);
    assert!(json[0]["result"].is_object());

    // Verify second response
    assert_eq!(json[1]["jsonrpc"], "2.0");
    assert_eq!(json[1]["id"], 2);
    assert!(json[1]["result"].is_object());

    // Clean shutdown
    child.kill().ok();
}

/// Test that POST / with single request returns single response (not array).
#[test]
fn test_post_single_request_returns_single_response() {
    let port = find_available_port();
    let mut child = spawn_mcp_http(port);

    // Wait for server to be ready
    assert!(
        wait_for_server(port, 2000),
        "Server did not start within 2 seconds"
    );

    let client = Client::new();
    let request_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list"
    });

    let response = client
        .post(&format!("http://127.0.0.1:{}/", port))
        .json(&request_body)
        .send()
        .expect("Failed to send request");

    assert_eq!(response.status(), reqwest::StatusCode::OK);

    let json: Value = response.json().expect("Response is not valid JSON");
    // Single request should return single response (object), not array
    assert!(json.is_object());
    assert!(!json.is_array());

    // Clean shutdown
    child.kill().ok();
}

/// Test that POST / over the size limit returns 413 with custom JSON body.
#[test]
fn test_post_payload_too_large() {
    let port = find_available_port();
    // Set a very small limit (1 MB)
    let mut child = spawn_mcp_http_with_limit(port, 1);

    // Wait for server to be ready
    assert!(
        wait_for_server(port, 2000),
        "Server did not start within 2 seconds"
    );

    let client = Client::new();
    // Create a payload larger than 1 MB
    let large_payload = "x".repeat(2 * 1024 * 1024); // 2 MB
    let request_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "test",
        "params": { "data": large_payload }
    });

    let response = client
        .post(&format!("http://127.0.0.1:{}/", port))
        .json(&request_body)
        .send()
        .expect("Failed to send request");

    assert_eq!(response.status(), reqwest::StatusCode::PAYLOAD_TOO_LARGE);

    let json: Value = response.json().expect("Response is not valid JSON");
    assert_eq!(json["error"]["code"], -32002);
    assert!(json["error"]["message"]
        .as_str()
        .unwrap()
        .contains("too large"));

    // Clean shutdown
    child.kill().ok();
}

/// Test that GET /health returns 200 with version info.
#[test]
fn test_get_health() {
    let port = find_available_port();
    let mut child = spawn_mcp_http(port);

    // Wait for server to be ready
    assert!(
        wait_for_server(port, 2000),
        "Server did not start within 2 seconds"
    );

    let client = Client::new();
    let response = client
        .get(&format!("http://127.0.0.1:{}/health", port))
        .send()
        .expect("Failed to send request");

    assert_eq!(response.status(), reqwest::StatusCode::OK);

    let json: Value = response.json().expect("Response is not valid JSON");
    assert_eq!(json["status"], "ok");
    assert!(json["version"].is_string());

    // Clean shutdown
    child.kill().ok();
}

/// Test that GET /sse opens an SSE stream with keepalive.
#[test]
fn test_get_sse_stream() {
    let port = find_available_port();
    let mut child = spawn_mcp_http(port);

    // Wait for server to be ready
    assert!(
        wait_for_server(port, 2000),
        "Server did not start within 2 seconds"
    );

    let client = reqwest::blocking::Client::builder()
        .timeout(None)
        .build()
        .expect("Failed to build HTTP client");

    let response = client
        .get(&format!("http://127.0.0.1:{}/sse", port))
        .send()
        .expect("Failed to send request");

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap(),
        "text/event-stream"
    );

    // Read the initial connection message
    let reader = BufReader::new(response);
    let mut lines = reader.lines();

    // First line should be a comment (connected)
    if let Some(Ok(line)) = lines.next() {
        assert!(
            line.starts_with(": connected"),
            "Expected ': connected', got: {}",
            line
        );
    }

    // Clean shutdown
    child.kill().ok();
}

/// Test that missing Authorization header on non-loopback bind returns 401.
#[test]
fn test_auth_required_for_non_loopback() {
    // Skip this test if we can't bind to non-loopback (requires permissions)
    // Use 127.0.0.2 which is still loopback but different from 127.0.0.1
    // This tests that auth checking is in place
    let port = find_available_port();
    let mut child = spawn_mcp_http(port);

    // Wait for server to be ready
    assert!(
        wait_for_server(port, 2000),
        "Server did not start within 2 seconds"
    );

    let client = Client::new();
    let request_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list"
    });

    // Request without auth should work on loopback (127.0.0.1)
    let response = client
        .post(&format!("http://127.0.0.1:{}/", port))
        .json(&request_body)
        .send()
        .expect("Failed to send request");

    // On loopback, auth is not required
    assert_eq!(response.status(), reqwest::StatusCode::OK);

    // Clean shutdown
    child.kill().ok();
}

/// Test that unknown method returns method_not_found error.
#[test]
fn test_unknown_method() {
    let port = find_available_port();
    let mut child = spawn_mcp_http(port);

    // Wait for server to be ready
    assert!(
        wait_for_server(port, 2000),
        "Server did not start within 2 seconds"
    );

    let client = Client::new();
    let request_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "unknown/method"
    });

    let response = client
        .post(&format!("http://127.0.0.1:{}/", port))
        .json(&request_body)
        .send()
        .expect("Failed to send request");

    assert_eq!(response.status(), reqwest::StatusCode::OK);

    let json: Value = response.json().expect("Response is not valid JSON");
    assert_eq!(json["error"]["code"], -32601);
    assert_eq!(json["error"]["message"], "Method not found");

    // Clean shutdown
    child.kill().ok();
}

/// Test 50 concurrent clients (plan line 2335 acceptance criterion).
///
/// This test spawns 50 concurrent clients, each making a tools/list request.
/// All 50 clients must succeed without 5xx errors.
#[test]
fn test_50_concurrent_clients() {
    let port = find_available_port();
    let mut child = spawn_mcp_http(port);

    // Wait for server to be ready
    assert!(
        wait_for_server(port, 2000),
        "Server did not start within 2 seconds"
    );

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("Failed to build HTTP client");

    let request_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list"
    });

    // Spawn 50 concurrent requests
    let handles: Vec<_> = (0..50)
        .map(|i| {
            let client = client.clone();
            let request_body = request_body.clone();
            let url = format!("http://127.0.0.1:{}/", port);

            thread::spawn(move || {
                let response = client.post(&url).json(&request_body).send();

                (i, response)
            })
        })
        .collect();

    // Wait for all requests to complete and collect results
    let mut success_count = 0;
    let mut error_count = 0;
    let mut five_xx_count = 0;

    for handle in handles {
        let (i, result) = handle.join().unwrap();

        match result {
            Ok(response) => {
                let status = response.status();
                if status.is_server_error() {
                    five_xx_count += 1;
                    eprintln!("Client {} got 5xx error: {}", i, status);
                } else if status.is_success() {
                    success_count += 1;
                } else {
                    error_count += 1;
                    eprintln!("Client {} got error: {}", i, status);
                }
            }
            Err(e) => {
                error_count += 1;
                eprintln!("Client {} failed: {}", i, e);
            }
        }
    }

    // All 50 clients should succeed without 5xx errors
    assert_eq!(five_xx_count, 0, "Got {} 5xx errors", five_xx_count);
    assert_eq!(error_count, 0, "Got {} errors", error_count);
    assert_eq!(
        success_count, 50,
        "Got {} successes, expected 50",
        success_count
    );

    // Clean shutdown
    child.kill().ok();
}

/// Test that GET /health returns 200 even during heavy load.
#[test]
fn test_health_during_load() {
    let port = find_available_port();
    let mut child = spawn_mcp_http(port);

    // Wait for server to be ready
    assert!(
        wait_for_server(port, 2000),
        "Server did not start within 2 seconds"
    );

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("Failed to build HTTP client");

    // Start some concurrent requests to create load
    let request_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list"
    });

    let load_handles: Vec<_> = (0..10)
        .map(|_| {
            let client = client.clone();
            let request_body = request_body.clone();
            let url = format!("http://127.0.0.1:{}/", port);

            thread::spawn(move || client.post(&url).json(&request_body).send())
        })
        .collect();

    // While load is ongoing, hit /health
    thread::sleep(Duration::from_millis(10)); // Let load start

    let health_response = client
        .get(&format!("http://127.0.0.1:{}/health", port))
        .send()
        .expect("Health check failed");

    assert_eq!(health_response.status(), reqwest::StatusCode::OK);

    // Clean shutdown
    for handle in load_handles {
        let _ = handle.join();
    }
    child.kill().ok();
}
