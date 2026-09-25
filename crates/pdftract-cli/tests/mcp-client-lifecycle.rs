//! End-to-end MCP stdio *client-side* integration test.
//!
//! Mirrors the documented handshake lifecycle from
//! `docs/integrations/mcp-clients.md` ("Connection Lifecycle"):
//!
//! 1. **Spawn:** start `pdftract mcp --stdio` as a subprocess
//! 2. **Handshake:** send `initialize`, receive capabilities
//! 3. **List Tools:** call `tools/list` — the 7 implemented tools
//! 4. **Call Tool:** invoke `tools/call` with a tool name and arguments
//! 5. **Terminate:** close stdin; server exits cleanly on EOF
//!
//! plus the documented "Error Handling" contract:
//!
//! - Parse errors: every malformed frame gets a JSON-RPC error response
//!   (`-32700` with `id: null` for unparseable JSON) and the server keeps
//!   serving — subsequent valid requests succeed
//! - Invalid params: server returns `-32602` with a `data.reason` string
//!   for missing/invalid `tools/call` params (`data.code` for `--root`
//!   boundary rejections)
//! - Unknown tool: a `tools/call` naming a tool outside the `tools/list`
//!   catalog returns `-32601`
//! - Errors do not kill the server; it keeps serving requests
//!
//! Unlike `mcp-stdio.rs` (transport-level framing checks) and
//! `root-path-protection.rs` (in-process `resolve_path` unit checks), this
//! file drives the server the way a real MCP client does: one long-lived
//! subprocess, sequential framed requests, responses correlated by id.
//!
//! # Test hygiene (TH-03)
//!
//! The server is held by an RAII guard whose `Drop` closes stdin (graceful
//! EOF path) and then bounded-waits for exit, killing at the deadline and
//! bounding the post-kill wait — never a bare `child.wait()`. Every response
//! read is bounded by a `recv_timeout`; the reader and stderr-drainer threads
//! exit on EOF once the child is gone, and stderr is drained continuously so
//! the child can never block on a full pipe. Run wrapped in a wall-clock
//! `timeout` so a hang fails the loop instead of freezing it.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// The tools cataloged by the server (see `ToolRegistry::register_all` and
/// the tools catalog). `tools/list` must return exactly these.
const CATALOGED_TOOLS: [&str; 7] = [
    "extract",
    "extract_text",
    "extract_markdown",
    "search",
    "get_metadata",
    "hash",
    "get_table",
];

/// Upper bound on waiting for any single JSON-RPC response.
const READ_TIMEOUT: Duration = Duration::from_secs(15);

/// Upper bound on the graceful-EOF shutdown wait before the guard kills.
const SHUTDOWN_TIMEOUT_MS: u64 = 5_000;

/// Bounded wait for child exit: poll `try_wait` until the deadline, then
/// kill and bound the post-kill wait too. The TH-03-safe replacement for a
/// bare `child.wait()` (same semantics as `pdftract_core::timeout`, which is
/// not exported from the crate).
/// Returns `Ok(code)` when the child exits (code `None` if signal-killed),
/// `Err` only if it defies SIGKILL past the post-kill bound.
fn wait_with_timeout(child: &mut Child, timeout_ms: u64) -> std::io::Result<Option<i32>> {
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status.code());
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let kill_deadline = Instant::now() + Duration::from_millis(1_000);
            loop {
                if let Some(status) = child.try_wait()? {
                    return Ok(status.code());
                }
                if Instant::now() >= kill_deadline {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "child did not exit within the post-kill bound",
                    ));
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

// ---------------------------------------------------------------------------
// Minimal single-page PDF fixture
// ---------------------------------------------------------------------------

/// Build a minimal, structurally valid single-page PDF containing `text`.
///
/// Generated in-place rather than checked in so the fixture cannot rot
/// independently of the generator and no binary blob lands in the tree.
fn minimal_pdf(text: &str) -> Vec<u8> {
    let stream = format!("BT /F1 12 Tf 72 720 Td ({text}) Tj ET");
    let objects: Vec<String> = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>"
            .to_string(),
        format!("<< /Length {} >>\nstream\n{}\nendstream", stream.len(), stream),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
    ];

    let mut out = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", i + 1, body).as_bytes());
    }
    let xref_offset = out.len();
    let count = objects.len() + 1;

    out.extend_from_slice(format!("xref\n0 {count}\n").as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for offset in &offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!("trailer\n<< /Size {count} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n")
            .as_bytes(),
    );
    out
}

// ---------------------------------------------------------------------------
// LSP Content-Length framing
// ---------------------------------------------------------------------------

/// Read one Content-Length-framed message. `Ok(None)` on EOF.
fn read_frame<R: Read>(reader: &mut BufReader<R>) -> std::io::Result<Option<String>> {
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            return Ok(None); // EOF
        }
        let line = line.trim_end_matches(|c| c == '\r' || c == '\n');
        if line.is_empty() {
            break;
        }
        if let Some(value) = line.strip_prefix("Content-Length:") {
            content_length = Some(value.trim().parse::<usize>().map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, format!("bad length: {e}"))
            })?);
        }
    }
    let n = content_length.ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, "missing Content-Length")
    })?;
    let mut body = vec![0u8; n];
    reader.read_exact(&mut body)?;
    Ok(Some(String::from_utf8_lossy(&body).into_owned()))
}

fn frame_message(body: &str) -> Vec<u8> {
    let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    frame.extend_from_slice(body.as_bytes());
    frame
}

// ---------------------------------------------------------------------------
// RAII-guarded server subprocess
// ---------------------------------------------------------------------------

/// A `pdftract mcp --stdio` subprocess driven as a sequential MCP client.
///
/// Dropping the guard closes stdin (the documented graceful-EOF path) and
/// then bounded-waits for exit, killing at the deadline. It is safe to drop
/// at any point, including on panic unwinding.
struct McpServer {
    child: Child,
    stdin: Option<ChildStdin>,
    responses: Receiver<Value>,
    stderr: Arc<Mutex<Vec<u8>>>,
    next_id: u64,
}

impl McpServer {
    /// Spawn `pdftract mcp --stdio [extra args]` with bounded, drained IO.
    fn spawn(extra_args: &[&str]) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_pdftract"))
            .arg("mcp")
            .arg("--stdio")
            .args(extra_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to spawn pdftract mcp --stdio");

        let stdout = child.stdout.take().expect("stdout was piped");
        let stderr = child.stderr.take().expect("stderr was piped");

        // Reader thread: framed stdout -> (bounded) response channel.
        // Exits on EOF once the child is gone; detached threads cannot hang
        // the test because every *test-side* receive is recv_timeout-bounded.
        let (tx, rx) = channel::<Value>();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            while let Ok(Some(body)) = read_frame(&mut reader) {
                match serde_json::from_str::<Value>(&body) {
                    Ok(msg) => {
                        if tx.send(msg).is_err() {
                            break; // test side dropped
                        }
                    }
                    Err(e) => {
                        // INV-9 violation: non-JSON-RPC on stdout. Surface it
                        // rather than deadlocking the client.
                        eprintln!("client: non-JSON frame on stdout ({e}): {body}");
                    }
                }
            }
        });

        // Stderr drain thread: keeps the child's stderr pipe empty (a full
        // pipe would block the server) and retains the log tail for
        // diagnostics.
        let stderr_log: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let stderr_sink = Arc::clone(&stderr_log);
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stderr);
            let mut chunk = [0u8; 4096];
            loop {
                match reader.read(&mut chunk) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        let mut log = stderr_sink.lock().unwrap();
                        log.extend_from_slice(&chunk[..n]);
                        let excess = log.len().saturating_sub(256 * 1024);
                        if excess > 0 {
                            log.drain(..excess);
                        }
                    }
                }
            }
        });

        let stdin = child.stdin.take();
        Self {
            child,
            stdin,
            responses: rx,
            stderr: stderr_log,
            next_id: 0,
        }
    }

    /// Send a JSON-RPC request and return its (id-correlated) response.
    fn request(&mut self, method: &str, params: Value) -> Value {
        self.next_id += 1;
        let id = self.next_id;
        let mut message = json!({"jsonrpc": "2.0", "id": id, "method": method});
        if !params.is_null() {
            message["params"] = params;
        }
        let body = serde_json::to_string(&message).expect("serialize request");

        let stdin = self
            .stdin
            .as_mut()
            .unwrap_or_else(|| panic!("server stdin already closed (request {id}: {method})"));
        stdin
            .write_all(&frame_message(&body))
            .unwrap_or_else(|e| panic!("failed writing request {id} ({method}): {e}"));
        stdin.flush().expect("flush request");

        self.recv_response(id)
    }

    /// Bounded wait for the response whose `id` matches `id`.
    fn recv_response(&self, id: u64) -> Value {
        match self.responses.recv_timeout(READ_TIMEOUT) {
            Ok(response) => {
                assert_eq!(
                    response.get("id").and_then(Value::as_u64),
                    Some(id),
                    "protocol desync: response id does not match request {id}: {response}"
                );
                assert_eq!(
                    response.get("jsonrpc").and_then(Value::as_str),
                    Some("2.0"),
                    "response is not JSON-RPC 2.0: {response}"
                );
                response
            }
            Err(RecvTimeoutError::Timeout) => panic!(
                "timed out after {READ_TIMEOUT:?} waiting for response to request {id}; stderr tail:\n{}",
                self.stderr_tail()
            ),
            Err(RecvTimeoutError::Disconnected) => panic!(
                "server closed stdout before responding to request {id}; stderr tail:\n{}",
                self.stderr_tail()
            ),
        }
    }

    /// Send an arbitrary pre-serialized body as one correctly framed message,
    /// without touching the id counter. Used for deliberately malformed
    /// frames: the framing itself is valid, so the server's transport stays
    /// in sync and the failure is contained to JSON-RPC parsing — which is
    /// exactly the documented "parse errors" scenario.
    fn send_raw(&mut self, body: &str) {
        let stdin = self
            .stdin
            .as_mut()
            .unwrap_or_else(|| panic!("server stdin already closed (send_raw: {body})"));
        stdin
            .write_all(&frame_message(body))
            .unwrap_or_else(|e| panic!("failed writing raw frame ({body}): {e}"));
        stdin.flush().expect("flush raw frame");
    }

    /// Bounded wait for the next response *without* id correlation. Parse
    /// errors are answered with `id: null` (an unparseable request carries
    /// no usable id), so the strict id check in `recv_response` cannot apply.
    fn recv_any(&self) -> Value {
        match self.responses.recv_timeout(READ_TIMEOUT) {
            Ok(response) => response,
            Err(RecvTimeoutError::Timeout) => panic!(
                "timed out after {READ_TIMEOUT:?} waiting for a response; stderr tail:\n{}",
                self.stderr_tail()
            ),
            Err(RecvTimeoutError::Disconnected) => panic!(
                "server closed stdout before responding; stderr tail:\n{}",
                self.stderr_tail()
            ),
        }
    }

    fn stderr_tail(&self) -> String {
        let log = self.stderr.lock().unwrap();
        let start = log.len().saturating_sub(2_000);
        String::from_utf8_lossy(&log[start..]).into_owned()
    }

    /// Close stdin, signaling EOF (documented terminate step).
    fn close_stdin(&mut self) {
        self.stdin.take();
    }

    /// Bounded wait for server exit; panics if it outlives the deadline.
    /// Returns the exit code (`None` if killed by a signal).
    fn wait_for_exit(&mut self) -> Option<i32> {
        match wait_with_timeout(&mut self.child, SHUTDOWN_TIMEOUT_MS) {
            Ok(code) => code,
            Err(e) => panic!(
                "server did not exit within {SHUTDOWN_TIMEOUT_MS}ms of stdin EOF: {e}; stderr tail:\n{}",
                self.stderr_tail()
            ),
        }
    }
}

impl Drop for McpServer {
    fn drop(&mut self) {
        // Graceful path first: EOF lets the server drain and exit 0.
        self.stdin.take();
        // wait_with_timeout kills at the deadline and bounds the post-kill
        // wait — the TH-03-safe replacement for a bare child.wait().
        let _ = wait_with_timeout(&mut self.child, SHUTDOWN_TIMEOUT_MS);
    }
}

// ---------------------------------------------------------------------------
// Assertions
// ---------------------------------------------------------------------------

/// Assert the response is a success result and return `result`.
fn assert_success<'a>(response: &'a Value, what: &str) -> &'a Value {
    assert!(
        response.get("error").is_none(),
        "{what} returned an error: {response}"
    );
    let result = response
        .get("result")
        .unwrap_or_else(|| panic!("{what} response has no result: {response}"));
    assert!(
        result.is_object(),
        "{what} result is not an object: {response}"
    );
    result
}

/// Assert the response is a spec-shaped JSON-RPC *error* envelope and return
/// the `error` object: JSON-RPC 2.0, an `error` object carrying a numeric
/// `code` and a non-empty `message`, and no `result` key alongside the error.
/// `Some(code)` pins the exact code; `None` only requires it to sit in the
/// spec-reserved error range (-32700..=-32000). The `id` semantics differ per
/// error class (parse errors answer `id: null`; request validation echoes the
/// request id), so callers assert it.
fn assert_error_envelope<'a>(response: &'a Value, what: &str, code: Option<i64>) -> &'a Value {
    assert_eq!(
        response.get("jsonrpc").and_then(Value::as_str),
        Some("2.0"),
        "{what} response is not JSON-RPC 2.0: {response}"
    );
    let error = response
        .get("error")
        .unwrap_or_else(|| panic!("{what} did not return an error object: {response}"));
    let actual = error
        .get("code")
        .and_then(Value::as_i64)
        .unwrap_or_else(|| panic!("{what} error must carry a numeric code: {response}"));
    match code {
        Some(expected) => assert_eq!(
            actual, expected,
            "{what} must answer with code {expected}: {response}"
        ),
        None => assert!(
            (-32700..=-32000).contains(&actual),
            "{what} code must sit in the spec-reserved error range: {response}"
        ),
    }
    assert!(
        error
            .get("message")
            .and_then(Value::as_str)
            .is_some_and(|m| !m.is_empty()),
        "{what} error must carry a non-empty message: {response}"
    );
    assert!(
        response.get("result").is_none(),
        "{what} must not carry a result alongside an error: {response}"
    );
    error
}

/// Extract `error.data.reason` as a non-empty string — the documented
/// invalid-params contract: "a `-32602` error whose `data` carries a `reason`
/// string explaining the rejection" (docs/integrations/mcp-clients.md,
/// "Error Handling"). Applies to the *general* invalid-params envelope; the
/// `--root` boundary rejections carry `data.code` instead.
fn assert_data_reason<'a>(what: &str, response: &'a Value) -> &'a str {
    let reason = response
        .get("error")
        .and_then(|error| error.get("data"))
        .and_then(|data| data.get("reason"))
        .and_then(Value::as_str)
        .unwrap_or_else(|| {
            panic!("{what} must carry a string data.reason (documented contract): {response}")
        });
    assert!(
        !reason.is_empty(),
        "{what} data.reason must be a non-empty string: {response}"
    );
    reason
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// The full documented lifecycle: spawn → initialize → tools/list (10
/// cataloged tools) → tools/call on a fixture → clean exit on stdin EOF.
#[test]
fn documented_lifecycle_initialize_list_call_exit_on_eof() {
    let dir = tempfile::tempdir().expect("tempdir");
    let pdf_path = dir.path().join("doc.pdf");
    std::fs::write(&pdf_path, minimal_pdf("Hello pdftract MCP")).expect("write fixture");

    // Step 1: Spawn. Absolute fixture path (no --root): trust-the-caller
    // mode, the default Claude Desktop configuration from the doc.
    let mut server = McpServer::spawn(&[]);

    // Step 2: Handshake.
    let init = server.request(
        "initialize",
        json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "pdftract-conformance-test", "version": "0.1.0"}
        }),
    );
    let init_result = assert_success(&init, "initialize");
    assert_eq!(
        init_result["protocolVersion"], "2024-11-05",
        "server must advertise the documented protocol version"
    );
    assert!(
        init_result["capabilities"]["tools"].is_object(),
        "server must advertise tool capability"
    );
    assert!(
        init_result["capabilities"]["resources"].is_null(),
        "server must not advertise unsupported resources capability"
    );
    assert!(
        init_result["capabilities"]["prompts"].is_null(),
        "server must not advertise unsupported prompts capability"
    );
    assert_eq!(init_result["serverInfo"]["name"], "pdftract");
    assert_eq!(
        init_result["serverInfo"]["version"],
        env!("CARGO_PKG_VERSION")
    );

    // Step 3: List tools — exactly the 7 implemented tools, each with the
    // fields clients need (name, description, inputSchema).
    let list = server.request("tools/list", json!({}));
    let list_result = assert_success(&list, "tools/list");
    let tools = list_result["tools"].as_array().unwrap_or_else(|| {
        panic!(
            "tools/list result.tools missing: {response}",
            response = list
        )
    });
    let mut names: Vec<&str> = tools
        .iter()
        .map(|t| {
            t["name"]
                .as_str()
                .unwrap_or_else(|| panic!("tool without string name: {t}"))
        })
        .collect();
    names.sort_unstable();
    let mut expected: Vec<&str> = CATALOGED_TOOLS.to_vec();
    expected.sort_unstable();
    assert_eq!(
        names, expected,
        "tools/list must return exactly the 7 implemented tools"
    );
    for tool in tools {
        assert!(
            tool["description"].is_string(),
            "tool missing description: {tool}"
        );
        assert!(
            tool["inputSchema"].is_object(),
            "tool missing inputSchema: {tool}"
        );
    }

    // Step 4: Call an extraction tool on the fixture.
    let call = server.request(
        "tools/call",
        json!({
            "name": "extract_text",
            "arguments": {"path": pdf_path.to_str().expect("utf-8 fixture path")}
        }),
    );
    assert_eq!(
        call["id"], 3,
        "tools/call response must carry the request id"
    );
    // Tool execution always returns an MCP CallToolResult. A failed
    // extraction is an in-band result, not a JSON-RPC error envelope.
    let call_result = assert_success(&call, "tools/call");
    assert!(
        call_result["content"].as_array().is_some_and(|content| {
            content
                .first()
                .is_some_and(|block| block["type"] == "text" && block["text"].is_string())
        }),
        "tools/call result must carry text content: {call}"
    );
    if call_result["isError"] == true {
        assert!(
            call_result["content"][0]["text"]
                .as_str()
                .is_some_and(|text| !text.is_empty()),
            "failed tools/call must explain the failure in content: {call}"
        );
    } else {
        assert!(
            call_result["structuredContent"]["text"].is_string(),
            "extract_text success result must preserve structured text: {call}"
        );
    }

    // Step 5: Terminate — server exits cleanly on stdin EOF.
    server.close_stdin();
    let code = server.wait_for_exit();
    assert_eq!(
        code,
        Some(0),
        "server must exit 0 on stdin EOF (documented terminate step)"
    );
}

/// The documented error contract over the wire: with `--root` set, a
/// traversal path and an absolute path are rejected in-band with `isError`
/// and `structuredContent.code == -32602` (with the boundary code in the
/// nested data), a missing-params `tools/call` is rejected with `-32602` +
/// `data.reason`, paths inside root are NOT rejected by the boundary, and the
/// server keeps serving after rejections.
#[test]
fn out_of_root_and_invalid_params_rejected_with_32602() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("root");
    std::fs::create_dir(&root).expect("create root");
    std::fs::write(root.join("inside.pdf"), minimal_pdf("inside root")).expect("write inside");
    // The escape target exists as a sibling of root, so canonicalize()
    // succeeds and the escape *check* (not resolution failure) rejects.
    std::fs::write(dir.path().join("outside.pdf"), minimal_pdf("outside root"))
        .expect("write outside");

    let mut server = McpServer::spawn(&["--root", root.to_str().expect("utf-8 root")]);

    // Relative traversal out of root → in-band -32602, PATH_ESCAPES_ROOT.
    let traversal = server.request(
        "tools/call",
        json!({
            "name": "extract_text",
            "arguments": {"path": "../outside.pdf"}
        }),
    );
    let error = assert_success(&traversal, "traversal tools/call");
    assert_eq!(error["isError"], true);
    assert_eq!(
        error["structuredContent"]["code"], -32602,
        "traversal rejection code: {traversal}"
    );
    assert_eq!(
        error["structuredContent"]["data"]["code"], "PATH_ESCAPES_ROOT",
        "traversal rejection must carry the boundary code: {traversal}"
    );

    // Absolute path under --root → in-band -32602,
    // ABSOLUTE_PATH_NOT_PERMITTED.
    let absolute = server.request(
        "tools/call",
        json!({
            "name": "extract_text",
            "arguments": {"path": "/etc/passwd"}
        }),
    );
    let error = assert_success(&absolute, "absolute-path tools/call");
    assert_eq!(error["isError"], true);
    assert_eq!(
        error["structuredContent"]["code"], -32602,
        "absolute-path rejection code: {absolute}"
    );
    assert_eq!(
        error["structuredContent"]["data"]["code"], "ABSOLUTE_PATH_NOT_PERMITTED",
        "absolute-path rejection must carry the boundary code: {absolute}"
    );

    // tools/call without a tool name → -32602 with `data.reason`
    // (the documented "Invalid params" error contract).
    let unnamed = server.request("tools/call", json!({"arguments": {"path": "inside.pdf"}}));
    let error = unnamed
        .get("error")
        .unwrap_or_else(|| panic!("missing tool name must be rejected, got: {unnamed}"));
    assert_eq!(
        error["code"], -32602,
        "missing-name rejection code: {unnamed}"
    );
    assert!(
        error["data"]["reason"].is_string(),
        "missing-name rejection must carry data.reason (documented contract): {unnamed}"
    );

    // A path inside root passes the security boundary — the response must
    // not be a -32602 boundary rejection. (Extraction itself may still fail
    // while the parser defect tracked separately is open; that is not a
    // path-boundary verdict.)
    let inside = server.request(
        "tools/call",
        json!({
            "name": "extract_text",
            "arguments": {"path": "inside.pdf"}
        }),
    );
    if inside.get("error").is_some() {
        let error = inside.get("error").unwrap();
        assert_ne!(
            error["code"], -32602,
            "path inside root must not be rejected by the boundary check: {inside}"
        );
    } else {
        assert_success(&inside, "tools/call(inside)");
        assert_ne!(
            inside["result"]["structuredContent"]["code"], -32602,
            "path inside root must not be rejected by the boundary check: {inside}"
        );
    }

    // Errors don't kill the server: it still answers after rejections.
    let ping = server.request("tools/list", json!({}));
    assert!(
        ping.get("result").is_some(),
        "server must keep serving after error responses: {ping}"
    );
}

/// Documented "Error Handling" contract, parse-error half: every malformed
/// frame gets a JSON-RPC error response and the server keeps serving — never
/// a hang, never a silent drop, never a success result.
///
/// Each malformed frame is sent with correct LSP framing, so the transport
/// stays in sync and the failure is contained to JSON-RPC parsing. Frames
/// exercised:
///
/// - truncated JSON (the classic parse error) → the spec-mandated `-32700`
///   with `id: null` and no `result` key — pinned exactly
/// - valid JSON that is not a Request object (a bare string)
/// - an empty batch array (invalid per JSON-RPC 2.0)
/// - a wrong `jsonrpc` version
/// - a well-formed batch (parses, but this server does not support batches)
///
/// The latter four fulfill the same documented promise — "server sends error
/// response, continues running" — but the doc pins no code for them, so the
/// test asserts the error envelope and that the code sits in the
/// spec-reserved range (-32700..=-32000) without pinning which. (This server
/// currently answers all read failures with `-32700`; the spec itself would
/// say `-32600` for syntactically-valid-but-invalid Request objects, so the
/// exact code here is an implementation choice, not contract.)
#[test]
fn parse_errors_get_error_responses_and_server_continues() {
    let mut server = McpServer::spawn(&[]);

    // Sanity: the server answers normally before any abuse.
    let init = server.request(
        "initialize",
        json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "pdftract-conformance-test", "version": "0.1.0"}
        }),
    );
    assert_success(&init, "initialize");

    // (label, raw body, pinned code if the contract fixes one)
    let malformed: [(&str, &str, Option<i64>); 5] = [
        (
            "truncated JSON",
            r#"{"jsonrpc":"2.0","id":99,"method":"tools/lis"#,
            Some(-32700),
        ),
        ("bare JSON string", r#""just a string""#, None),
        ("empty batch array", r#"[]"#, None),
        (
            "wrong jsonrpc version",
            r#"{"jsonrpc":"1.0","id":98,"method":"tools/list"}"#,
            None,
        ),
        (
            "unsupported batch",
            r#"[{"jsonrpc":"2.0","id":97,"method":"tools/list"}]"#,
            None,
        ),
    ];

    for (what, frame, pinned) in malformed {
        server.send_raw(frame);
        let response = server.recv_any();
        assert_error_envelope(&response, what, pinned);
        assert!(
            response.get("id").map(Value::is_null).unwrap_or(false),
            "{what} must be answered with id null (no usable request id): {response}"
        );
    }

    // Second half of the documented promise: the server is still serving.
    // A full valid round-trip proves the frame stream did not desync and no
    // per-connection state was corrupted by the malformed frames.
    let list = server.request("tools/list", json!({}));
    let result = assert_success(&list, "tools/list after parse errors");
    let tools = result["tools"]
        .as_array()
        .unwrap_or_else(|| panic!("tools/list result.tools missing after parse errors: {list}"));
    assert_eq!(
        tools.len(),
        CATALOGED_TOOLS.len(),
        "tools/list must return the full catalog after parse errors: {list}"
    );

    // One more abuse → recovery cycle, so resilience is not a one-frame
    // fluke, then the documented terminate step still works.
    server.send_raw(r#"{"jsonrpc":"2.0","id":96,"method":"tools/list""#);
    let response = server.recv_any();
    assert_error_envelope(&response, "second truncated JSON", Some(-32700));
    let again = server.request("tools/list", json!({}));
    assert_success(&again, "tools/list after second parse error");

    server.close_stdin();
    let code = server.wait_for_exit();
    assert_eq!(
        code,
        Some(0),
        "server must still exit 0 on stdin EOF after parse errors"
    );
}

/// Documented "Error Handling" contract, invalid-params half: `tools/call`
/// with unusable params is rejected with `-32602` carrying a non-empty
/// `data.reason` string, the error response still echoes the request id, and
/// the server keeps serving. This is the *general* envelope contract — the
/// `--root` boundary-specific `data.code` rejections are covered by
/// `out_of_root_and_invalid_params_rejected_with_32602` above.
#[test]
fn invalid_params_rejected_with_32602_data_reason() {
    let mut server = McpServer::spawn(&[]);

    // Handshake, so the sequence mirrors a real client session.
    let init = server.request(
        "initialize",
        json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "pdftract-conformance-test", "version": "0.1.0"}
        }),
    );
    assert_success(&init, "initialize");

    // tools/call with no params object at all (`request` omits null params).
    let bare = server.request("tools/call", Value::Null);
    assert_error_envelope(&bare, "params-less tools/call", Some(-32602));
    assert_data_reason("params-less tools/call", &bare);
    assert_eq!(
        bare.get("id").and_then(Value::as_u64),
        Some(2), // second request of the session (initialize was id 1)
        "invalid-params rejection must still echo the request id: {bare}"
    );

    // tools/call with arguments but no tool name.
    let unnamed = server.request(
        "tools/call",
        json!({"arguments": {"path": "somewhere.pdf"}}),
    );
    assert_error_envelope(&unnamed, "name-less tools/call", Some(-32602));
    assert_data_reason("name-less tools/call", &unnamed);

    // tools/call with a non-string name (wrong type, not just missing).
    let mistyped = server.request("tools/call", json!({"name": 42, "arguments": {}}));
    assert_error_envelope(&mistyped, "non-string tool name", Some(-32602));
    assert_data_reason("non-string tool name", &mistyped);

    // A name that parses but is not in the tools/list catalog is rejected
    // with -32601 (method not found) — the documented "Unknown tool" bullet —
    // still echoing the request id.
    let unknown = server.request(
        "tools/call",
        json!({"name": "no_such_tool", "arguments": {}}),
    );
    assert_error_envelope(&unknown, "unknown tool", Some(-32601));
    assert_eq!(
        unknown.get("id").and_then(Value::as_u64),
        Some(5), // fifth request of the session (initialize was id 1)
        "unknown-tool rejection must still echo the request id: {unknown}"
    );

    // The server keeps serving after the rejections.
    let list = server.request("tools/list", json!({}));
    assert_success(&list, "tools/list after invalid params");

    // And the documented terminate step still works.
    server.close_stdin();
    let code = server.wait_for_exit();
    assert_eq!(
        code,
        Some(0),
        "server must still exit 0 on stdin EOF after invalid params"
    );
}

/// Umbrella acceptance pin (pdftract-bcf935ec, parent pdftract-257d92c3):
/// `extract_text` over the stdio wire returns REAL CONTENT for both checked-in
/// `valid-minimal.pdf` fixtures — not the `-32002` extraction-failed relay the
/// MCP dogfood pilot recorded while xref/page-tree resolution was broken
/// (docs/notes/mcp-dogfood-pilot.md). The repo-root W3C dummy carries "Test";
/// the core-suite minimal — the "Document contains no pages" repro, fixed by
/// the xref-drift recovery of pdftract-4683109d plus the free-list/mixed-chain
/// recovery of pdftract-4196ae99 — carries "Hello World". Every other call in
/// this file tolerates an extraction error arm (the wire contract is the
/// concern there); this pin is where extraction success itself is asserted,
/// matching `realworld_extraction_baseline.rs`.
#[test]
fn extract_text_returns_content_for_checked_in_valid_minimal_fixtures() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fixtures = [
        (
            "repo-root valid-minimal (W3C dummy)",
            repo_root.join("tests/fixtures/valid-minimal.pdf"),
            "Test",
        ),
        (
            "core valid-minimal (page-tree repro)",
            repo_root.join("crates/pdftract-core/tests/fixtures/valid-minimal.pdf"),
            "Hello World",
        ),
    ];

    let mut server = McpServer::spawn(&[]);
    let init = server.request(
        "initialize",
        json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "pdftract-conformance-test", "version": "0.1.0"}
        }),
    );
    assert_success(&init, "initialize");

    for (what, path, expected) in fixtures {
        let call = server.request(
            "tools/call",
            json!({
                "name": "extract_text",
                "arguments": {"path": path.to_str().expect("utf-8 fixture path")}
            }),
        );
        // Content must come back: no JSON-RPC error arm at all, so -32002
        // ("Extraction failed") is excluded by construction.
        let call_result = assert_success(&call, &format!("extract_text on {what}"));
        let text = call_result["structuredContent"]["text"]
            .as_str()
            .unwrap_or_else(|| panic!("extract_text on {what} must carry a string 'text': {call}"));
        assert!(
            !text.trim().is_empty(),
            "extract_text on {what} returned an empty text layer: {call}"
        );
        assert!(
            text.contains(expected),
            "extract_text on {what} must contain {expected:?}, got {} chars: {text:?}",
            text.len()
        );
    }

    // Documented terminate step still holds on the success path.
    server.close_stdin();
    let code = server.wait_for_exit();
    assert_eq!(code, Some(0), "server must exit 0 on stdin EOF");
}

/// A notification must not consume the response slot for the next request.
/// This is the pipelining failure that occurs when notifications/initialized
/// is answered with a JSON-RPC error carrying id:null.
#[test]
fn initialized_notification_is_silent_when_requests_are_pipelined() {
    let mut server = McpServer::spawn(&[]);

    server.send_raw(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);
    server.send_raw(r#"{"jsonrpc":"2.0","id":42,"method":"tools/list"}"#);

    let response = server.recv_response(42);
    assert!(
        response["result"]["tools"].is_array(),
        "unexpected response: {response}"
    );

    server.close_stdin();
    assert_eq!(server.wait_for_exit(), Some(0));
}

/// Once a known tool has been selected, execution failures belong in the MCP
/// result envelope rather than the JSON-RPC error slot.
#[test]
fn tool_failures_are_in_band_call_results() {
    let mut server = McpServer::spawn(&[]);
    let init = server.request("initialize", json!({"capabilities": {}}));
    assert_success(&init, "initialize");

    let call = server.request(
        "tools/call",
        json!({
            "name": "hash",
            "arguments": {"path": "/path/that/does/not/exist.pdf"}
        }),
    );
    let result = assert_success(&call, "failed tools/call");
    assert_eq!(
        result["isError"], true,
        "tool failure must be in-band: {call}"
    );
    assert!(result["content"].as_array().is_some_and(|content| {
        content.first().is_some_and(|block| {
            block["type"] == "text" && block["text"].as_str().is_some_and(|text| !text.is_empty())
        })
    }));
    assert_eq!(result["structuredContent"]["data"]["code"], "PATH_INVALID");

    server.close_stdin();
    assert_eq!(server.wait_for_exit(), Some(0));
}
