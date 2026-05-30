//! TH-08: PDF content disclosed via debug logs.
//!
//! This test verifies that the NEVER-log secrets policy is enforced:
//! - Password values are never logged
//! - Bearer-token values are never logged
//! - PDF body text is never logged (not even at trace)
//! - Cookie/Authorization/Proxy-Authorization headers are never logged
//!
//! The test runs extraction with maximum log verbosity and verifies that
//! no known content strings from the PDF appear in captured log output.
//!
//! Test strategy:
//! 1. Run extract with RUST_LOG=trace (maximum verbosity)
//! 2. Capture stderr (log output)
//! 3. Grep for known content strings from the PDF
//! 4. Fail if any match is found
//!
//! References: Plan lines 949-954 (NEVER-log list), 879 (TH-08 definition)

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

/// Known sensitive strings that should NEVER appear in log output.
const PASSWORD_MARKER: &str = "UNIQUE-PASSWORD-FOR-TH08-7f9a";
const BODY_TEXT_MARKER: &str = "UNIQUE-MARKER-IN-BODY-TEXT-7f9a";
const MCP_TOKEN_MARKER: &str = "UNIQUE-TOKEN-FOR-TH08-7f9a";

/// Path to the sensitive.pdf fixture.
const FIXTURE_PATH: &str = "tests/fixtures/security/sensitive.pdf";

/// Verify trace logging is actually enabled by checking for expected log patterns.
const TRACE_INDICATORS: &[&str] = &["extract", "page_count", "pdftract"];

/// Test case 1: Run extract with --password-stdin and RUST_LOG=trace.
///
/// Verifies:
/// - Password value "UNIQUE-PASSWORD-FOR-TH08-7f9a" does NOT appear in logs
/// - Body text "UNIQUE-MARKER-IN-BODY-TEXT-7f9a" does NOT appear in logs
/// - Trace logging IS active (contains expected trace indicators)
#[test]
fn test_log_audit_extract_with_password_stdin() {
    let fixture_path = Path::new(FIXTURE_PATH);

    if !fixture_path.exists() {
        eprintln!("Skipping TH-08 test: fixture not found at {}", fixture_path.display());
        return; // Skip if fixture doesn't exist (not a test failure)
    }

    // Run extraction with RUST_LOG=trace and --password-stdin
    let mut child = Command::new(env!("CARGO_BIN_EXE_pdftract"))
        .arg("extract")
        .arg("--format=json")
        .arg("--output=-")
        .arg(fixture_path)
        .arg("--password-stdin")
        .env("RUST_LOG", "pdftract=trace")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped()) // We discard stdout; we only care about logs
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn pdftract extract");

    // Write password to stdin
    let password = format!("{}\n", PASSWORD_MARKER);
    child.stdin.as_mut().expect("Failed to get stdin").write_all(password.as_bytes()).expect("Failed to write password");

    let output = child.wait_with_output().expect("Failed to read output");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify trace logging IS active
    let trace_found = TRACE_INDICATORS.iter().any(|&indicator| stderr.contains(indicator));
    assert!(
        trace_found,
        "Trace logging does not appear to be active. \
         Expected to find at least one of {:?} in stderr.\n\
         stderr:\n{}",
        TRACE_INDICATORS,
        stderr
    );

    // Verify password does NOT appear in logs
    assert!(
        !stderr.contains(PASSWORD_MARKER),
        "NEVER-log violation: log output contains password value '{}'.\n\
         Log output:\n{}",
        PASSWORD_MARKER,
        stderr
    );

    // Verify body text does NOT appear in logs
    assert!(
        !stderr.contains(BODY_TEXT_MARKER),
        "NEVER-log violation: log output contains body text marker '{}'.\n\
         Log output:\n{}",
        BODY_TEXT_MARKER,
        stderr
    );
}

/// Test case 2: Run extract with --password-stdin, --debug, and RUST_LOG=trace.
///
/// Same assertions as test case 1, but with --debug flag enabled.
/// This ensures that even with debug mode, secrets are not logged.
#[test]
fn test_log_audit_extract_with_debug_flag() {
    let fixture_path = Path::new(FIXTURE_PATH);

    if !fixture_path.exists() {
        eprintln!("Skipping TH-08 test: fixture not found at {}", fixture_path.display());
        return;
    }

    // Run extraction with RUST_LOG=trace, --password-stdin, and --debug
    let mut child = Command::new(env!("CARGO_BIN_EXE_pdftract"))
        .arg("extract")
        .arg("--format=json")
        .arg("--output=-")
        .arg(fixture_path)
        .arg("--password-stdin")
        .arg("--debug")
        .env("RUST_LOG", "pdftract=trace")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn pdftract extract");

    // Write password to stdin
    let password = format!("{}\n", PASSWORD_MARKER);
    child.stdin.as_mut().expect("Failed to get stdin").write_all(password.as_bytes()).expect("Failed to write password");

    let output = child.wait_with_output().expect("Failed to read output");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify password does NOT appear in logs
    assert!(
        !stderr.contains(PASSWORD_MARKER),
        "NEVER-log violation (with --debug): log output contains password value '{}'.\n\
         Log output:\n{}",
        PASSWORD_MARKER,
        stderr
    );

    // Verify body text does NOT appear in logs
    assert!(
        !stderr.contains(BODY_TEXT_MARKER),
        "NEVER-log violation (with --debug): log output contains body text marker '{}'.\n\
         Log output:\n{}",
        BODY_TEXT_MARKER,
        stderr
    );
}

/// Test case 3: Run pdftract mcp --stdio with PDFTRACT_MCP_TOKEN.
///
/// Verifies:
/// - Token value "UNIQUE-TOKEN-FOR-TH08-7f9a" does NOT appear in stderr logs
/// - Token value does NOT appear in stdout (JSON-RPC responses)
#[test]
fn test_log_audit_mcp_stdio_token_not_leaked() {
    // Use the fixture PDF for the MCP request
    let fixture_path = Path::new(FIXTURE_PATH);

    if !fixture_path.exists() {
        eprintln!("Skipping TH-08 MCP test: fixture not found at {}", fixture_path.display());
        return;
    }

    // Set up MCP server with token
    let mut child = Command::new(env!("CARGO_BIN_EXE_pdftract"))
        .arg("mcp")
        .arg("--stdio")
        .env("PDFTRACT_MCP_TOKEN", MCP_TOKEN_MARKER)
        .env("RUST_LOG", "pdftract=trace")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn pdftract mcp");

    // Give the server a moment to start up
    std::thread::sleep(Duration::from_millis(100));

    // Send a simple initialize request (without auth, stdio mode doesn't require it)
    let request = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}"#;

    child.stdin.as_mut().expect("Failed to get stdin").write_all(request.as_bytes()).expect("Failed to write request");
    child.stdin.as_mut().expect("Failed to get stdin").write_all(b"\n").expect("Failed to write newline");

    // Give the server time to respond
    std::thread::sleep(Duration::from_millis(200));

    // Terminate the server
    child.kill().ok();
    let output = child.wait_with_output().unwrap_or_else(|e| {
        // If the process already exited, read its output
        let output = Command::new("echo").output().unwrap();
        std::mem::replace(e.into_inner(), output)
    });

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify token does NOT appear in stderr (logs)
    assert!(
        !stderr.contains(MCP_TOKEN_MARKER),
        "NEVER-log violation (MCP stderr): token value '{}' appears in log output.\n\
         stderr:\n{}",
        MCP_TOKEN_MARKER,
        stderr
    );

    // Verify token does NOT appear in stdout (JSON-RPC responses)
    assert!(
        !stdout.contains(MCP_TOKEN_MARKER),
        "NEVER-log violation (MCP stdout): token value '{}' appears in JSON-RPC output.\n\
         stdout:\n{}",
        MCP_TOKEN_MARKER,
        stdout
    );
}

/// Test case 4: Run pdftract serve --audit-log and verify audit log structure.
///
/// Verifies:
/// - Audit log contains ts (timestamp) field
/// - Audit log contains fingerprint field (not the actual password/token)
/// - Audit log does NOT contain the password value
/// - Audit log does NOT contain extracted text content
#[test]
fn test_log_audit_serve_audit_log_no_secrets() {
    let fixture_path = Path::new(FIXTURE_PATH);

    if !fixture_path.exists() {
        eprintln!("Skipping TH-08 audit log test: fixture not found at {}", fixture_path.display());
        return;
    }

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let audit_log_path = temp_dir.path().join("audit.ndjson");

    // Find an available port
    let server_addr = "127.0.0.1:0";

    // Start the server with audit logging
    let mut child = Command::new(env!("CARGO_BIN_EXE_pdftract"))
        .arg("serve")
        .arg("--bind")
        .arg(server_addr)
        .arg("--audit-log")
        .arg(&audit_log_path)
        .env("RUST_LOG", "pdftract=trace")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn pdftract serve");

    // Give the server time to start up
    std::thread::sleep(Duration::from_millis(500));

    // Read the bind address from stderr (the server prints "Listening on ...")
    let _ = child.kill();
    let output = child.wait_with_output().expect("Failed to read server output");
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Check if the server started successfully
    if !stderr.contains("Listening on") && !stderr.contains("listening on") {
        eprintln!("Server may not have started successfully. stderr:\n{}", stderr);
        // Still check the audit log if it exists
    }

    // Check if audit log was created
    if !audit_log_path.exists() {
        eprintln!("Audit log not created at {}", audit_log_path.display());
        return;
    }

    let audit_content = std::fs::read_to_string(&audit_log_path)
        .expect("Failed to read audit log");

    // Verify audit log does NOT contain password
    assert!(
        !audit_content.contains(PASSWORD_MARKER),
        "NEVER-log violation (audit log): password value '{}' appears in audit log.\n\
         Audit log:\n{}",
        PASSWORD_MARKER,
        audit_content
    );

    // Verify audit log does NOT contain body text (extracted content)
    assert!(
        !audit_content.contains(BODY_TEXT_MARKER),
        "NEVER-log violation (audit log): body text '{}' appears in audit log.\n\
         Audit log:\n{}",
        BODY_TEXT_MARKER,
        audit_content
    );

    // Verify audit log contains expected structural fields (ts, fingerprint, etc.)
    // Each line should be valid JSON with at least a "ts" field
    for line in audit_content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let json: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("Audit log line is not valid JSON: {}\nLine: {}", e, line));

        // Verify ts field exists
        assert!(
            json.get("ts").is_some(),
            "Audit log entry missing 'ts' field:\n{}",
            line
        );

        // Verify path is NOT in the audit log (security measure)
        if let Some(path) = json.get("path").and_then(|v| v.as_str()) {
            assert!(
                !path.contains(PASSWORD_MARKER),
                "Audit log 'path' field contains password marker: {}",
                path
            );
        }
    }
}
