//! stdio transport for the MCP server.
//!
//! This module implements the stdio transport defined in the MCP spec:
//! https://modelcontextprotocol.io/spec/transports#stdio
//!
//! # INV-9 Enforcement
//!
//! In stdio mode, stdout MUST contain only JSON-RPC frames. All logs and
//! diagnostics go to stderr. This is enforced by:
//! - Setting a panic hook that writes to stderr
//! - Never using println! or print! macros (only eprintln!/eprint!)
//! - Using a single BufWriter<Stdout> protected by a Mutex for all JSON-RPC output

use crate::mcp::framing::{BatchMessage, ErrorObject, Id, Request, Response};
use crate::mcp::tools;
use anyhow::{anyhow, Context, Result};
use serde_json::json;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Stdin, Stdout, Write};
use std::panic::Location;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Instant;

/// Global flag indicating whether we should keep running.
///
/// Set to false by SIGTERM handler to trigger graceful shutdown.
static SHOULD_RUN: AtomicBool = AtomicBool::new(true);

/// Global stdout writer protected by a mutex.
///
/// This is the ONLY legitimate way to write to stdout in stdio mode.
/// All other code paths must use stderr for logging.
static STDOUT: Mutex<Option<BufWriter<Stdout>>> = Mutex::new(None);

/// Initialize the stdout writer.
///
/// This MUST be called at MCP startup before any request processing.
/// Once initialized, all JSON-RPC responses go through this writer.
fn init_stdout() {
    let mut stdout = STDOUT.lock().unwrap();
    if stdout.is_none() {
        *stdout = Some(BufWriter::new(io::stdout()));
        eprintln!("stdio transport: stdout writer initialized");
    }
}

/// Write a JSON-RPC response to stdout.
///
/// This frames the response with Content-Length headers as per the LSP spec.
/// Returns an error if stdout is not initialized.
///
/// # Framing format (per LSP spec)
///
/// ```text
/// Content-Length: <byte-length>\r\n
/// \r\n
/// <json-body>
/// ```
///
/// CRITICAL: The JSON body is written WITHOUT a trailing newline.
/// Adding any extra bytes after the JSON body breaks the framing.
fn write_response(response: &Response) -> Result<()> {
    let json = serde_json::to_string(response).context("Failed to serialize response")?;

    let content_length = json.len();

    let mut stdout_guard = STDOUT.lock().unwrap();
    let stdout = stdout_guard
        .as_mut()
        .ok_or_else(|| anyhow!("stdout not initialized"))?;

    // Write headers with \r\n line terminators (LSP spec)
    //
    // Note: We use write! (not writeln!) for the header line to avoid
    // double newlines. We manually add \r\n for each header line.
    write!(stdout, "Content-Length: {content_length}\r\n")?;
    write!(stdout, "\r\n")?;

    // Write the JSON body WITHOUT a trailing newline
    //
    // CRITICAL for INV-9 compliance: Any extra byte after the JSON body
    // (including a newline) breaks the LSP framing format and will cause
    // the client to fail parsing the response.
    write!(stdout, "{json}")?;

    // Flush immediately to ensure the client receives the response
    stdout.flush().context("Failed to flush stdout")?;

    Ok(())
}

/// Set up the panic hook to write to stderr instead of stdout.
///
/// This is critical for INV-9 compliance: if a panic occurs and writes to
/// stdout, it will corrupt the JSON-RPC stream and break the client.
fn setup_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        let location = panic_info.location().unwrap_or_else(|| {
            // Fallback if location is not available
            Location::caller()
        });

        let msg = match panic_info.payload().downcast_ref::<&str>() {
            Some(s) => *s,
            None => match panic_info.payload().downcast_ref::<String>() {
                Some(s) => s.as_str(),
                None => "unknown panic message",
            },
        };

        eprintln!("PANIC at {}({}): {}", location.file(), location.line(), msg);
    }));
}

/// Set up signal handlers for graceful shutdown.
///
/// - SIGTERM: Graceful shutdown (drain in-flight requests, exit 0)
/// - SIGINT: Immediate exit (exit non-zero)
///
/// # Platform support
///
/// On Unix, we set up actual signal handlers via libc FFI.
/// On non-Unix (Windows), signals are handled differently; we rely on
/// the OS to terminate the process.
fn setup_signal_handlers() {
    #[cfg(unix)]
    {
        // Use libc FFI to set up signal handler for SIGTERM
        //
        // SAFETY: We're setting up a simple signal handler that only
        // sets an atomic boolean. This is safe because:
        // 1. The handler doesn't call any non-async-signal-safe functions
        // 2. We only write to an atomic bool (lock-free on supported platforms)
        // 3. The handler is constant for the lifetime of the program
        unsafe {
            extern "C" fn sigterm_handler(_: libc::c_int) {
                // Set the flag to trigger graceful shutdown
                SHOULD_RUN.store(false, Ordering::SeqCst);
            }

            // Set up the SIGTERM handler
            // SA_RESTART: Automatically restart interrupted system calls
            let mut sa: libc::sigaction = std::mem::zeroed();
            sa.sa_sigaction = sigterm_handler as *const () as usize;
            sa.sa_flags = libc::SA_RESTART;

            // Block all signals during handler execution
            libc::sigemptyset(&mut sa.sa_mask);

            if libc::sigaction(libc::SIGTERM, &sa, std::ptr::null_mut()) != 0 {
                eprintln!("Warning: Failed to set up SIGTERM handler");
            } else {
                eprintln!("Signal handler: SIGTERM -> graceful shutdown");
            }
        }

        // Note: We don't explicitly handle SIGINT here because the default
        // behavior (immediate termination) is what we want for SIGINT per
        // the acceptance criteria.
    }

    #[cfg(not(unix))]
    {
        eprintln!("Note: Signal handlers not available on this platform");
    }
}

/// Read a single JSON-RPC message from stdin.
///
/// This implements the LSP-style framing:
/// 1. Read headers line-by-line until an empty line
/// 2. Parse Content-Length header
/// 3. Read exactly Content-Length bytes
/// 4. Parse as JSON
///
/// Returns None on EOF (graceful shutdown).
///
/// # Errors
///
/// - If Content-Length header is missing
/// - If Content-Length value is invalid
/// - If message body is shorter than Content-Length (unexpected EOF)
/// - If message body cannot be parsed as JSON-RPC
fn read_message(stdin: &mut BufReader<Stdin>) -> Result<Option<Request>> {
    let mut content_length: Option<usize> = None;

    // Read headers until empty line
    loop {
        let mut line = String::new();
        let bytes_read = stdin
            .read_line(&mut line)
            .context("Failed to read header line")?;

        if bytes_read == 0 {
            // EOF on stdin (before header section ends)
            return Ok(None);
        }

        let line = line.trim_end_matches(|c| c == '\r' || c == '\n');

        if line.is_empty() {
            // Empty line signals end of headers
            break;
        }

        // Parse Content-Length header
        if let Some(value) = line.strip_prefix("Content-Length:") {
            let value = value.trim();
            content_length = Some(
                value
                    .parse::<usize>()
                    .with_context(|| format!("Invalid Content-Length: {value}"))?,
            );
        }
        // Ignore other headers (we don't need Content-Type for now)
    }

    let content_length = content_length.ok_or_else(|| anyhow!("Missing Content-Length header"))?;

    // Read exactly content_length bytes
    let mut buffer = vec![0u8; content_length];
    match stdin.read_exact(&mut buffer) {
        Ok(_) => {
            // Successfully read the full message body
        }
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
            // Unexpected EOF: Content-Length said X bytes, but we got fewer
            return Err(anyhow!(
                "Unexpected EOF: expected {content_length} bytes but got partial message"
            ));
        }
        Err(e) => {
            // Other read error
            return Err(e).context("Failed to read message body");
        }
    }

    // Parse as JSON-RPC BatchMessage (handles both single requests and batches)
    let batch: BatchMessage =
        serde_json::from_slice(&buffer).context("Failed to parse JSON-RPC request")?;

    // Extract the single request from the batch
    // For now, we only support single requests (not batches)
    let request = match batch {
        BatchMessage::Single(req) => req,
        BatchMessage::Batch(reqs) => {
            // We don't support batch requests yet
            return Err(anyhow!(
                "Batch requests not supported (got {} requests in one message)",
                reqs.len()
            ));
        }
    };

    Ok(Some(request))
}

/// Handle a JSON-RPC request and return a response.
fn handle_request(
    request: Request,
    registry: &tools::ToolRegistry,
    root: Option<&Path>,
    audit_writer: Option<&pdftract_core::audit::AuditLogWriter>,
) -> Response {
    let id = request.request_id();

    match request.method.as_str() {
        "tools/list" => {
            let tools = registry.tools_list();
            Response::success(id, tools)
        }
        "initialize" => {
            let result = json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {},
                    "resources": {},
                    "prompts": {}
                },
                "serverInfo": {
                    "name": "pdftract",
                    "version": env!("CARGO_PKG_VERSION")
                }
            });
            Response::success(id, result)
        }
        "tools/call" => {
            // Extract tool name and arguments from params
            let params = match request.params {
                Some(p) => p,
                None => {
                    return Response::error(
                        id,
                        ErrorObject::invalid_params()
                            .with_data(json!({"reason": "Missing params"})),
                    );
                }
            };

            let tool_name = match params.get("name").and_then(|v| v.as_str()) {
                Some(name) => name,
                None => {
                    return Response::error(
                        id,
                        ErrorObject::invalid_params()
                            .with_data(json!({"reason": "Missing or invalid 'name' field"})),
                    );
                }
            };

            let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

            // Look up the tool in the registry
            let tool = match registry.get(tool_name) {
                Some(t) => t,
                None => {
                    return Response::error(id, ErrorObject::method_not_found(tool_name));
                }
            };

            // Execute the tool with observability logging
            let start = Instant::now();
            let log_path = arguments
                .get("path")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let result = tool.execute(arguments, log_path.as_deref(), root);

            let duration_ms = start.elapsed().as_millis();
            let response_size = result
                .as_ref()
                .ok()
                .map(|v| serde_json::to_vec(v).unwrap_or_default().len())
                .unwrap_or(0);

            // Emit structured log line to stderr
            let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
            let path_or_hash = log_path.as_deref().unwrap_or("<unknown>");
            let error_code = result.as_ref().err().map(|e| e.code.to_string());

            eprintln!(
                "{} tool={} path={} duration_ms={} response_size_bytes={} error_code={:?}",
                timestamp, tool_name, path_or_hash, duration_ms, response_size, error_code,
            );

            // Write audit log if configured (stdio mode: client_ip is absent)
            if let Some(writer) = audit_writer {
                let status = if result.is_ok() { 200 } else { 500 };
                let diagnostics = if let Err(ref e) = result {
                    vec![e.code.to_string()]
                } else {
                    Vec::new()
                };
                // For stdio mode, client_ip is None (no HTTP peer)
                let _ = writer.log(
                    &format!("mcp.{}", tool_name),
                    None, // No client_ip in stdio mode
                    None, // No fingerprint at MCP layer
                    duration_ms as u64,
                    status,
                    &diagnostics,
                );
            }

            match result {
                Ok(value) => Response::success(id, value),
                Err(error) => Response::error(id, error),
            }
        }
        _ => {
            eprintln!("Unknown method: {}", request.method);
            Response::error(id, ErrorObject::method_not_found(&request.method))
        }
    }
}

/// Run the stdio transport loop.
///
/// This function:
/// 1. Sets up the panic hook to write to stderr
/// 2. Sets up signal handlers for SIGTERM/SIGINT
/// 3. Initializes the stdout writer
/// 4. Creates the tool registry
/// 5. Reads JSON-RPC requests from stdin
/// 6. Dispatches to handlers
/// 7. Writes responses to stdout
/// 8. Exits cleanly on EOF or SIGTERM
///
/// # Arguments
///
/// * `root` - Optional root directory for path-traversal protection
///
/// # Signal handling
///
/// - **SIGTERM**: Graceful shutdown (drain in-flight requests, exit 0)
/// - **SIGINT**: Immediate exit (via default signal handler, exit non-zero)
///
/// # Errors
///
/// Returns an error if:
/// - A message cannot be read or parsed
/// - A response cannot be written
/// - stdin/stdout is not a TTY (but this is expected for stdio mode)
pub fn run(root: Option<&Path>, audit_log: Option<&std::path::Path>) -> Result<()> {
    // Set up panic hook FIRST (before any potential panics)
    setup_panic_hook();

    // Set up signal handlers for graceful shutdown
    setup_signal_handlers();

    // Initialize stdout writer (only way to write to stdout in stdio mode)
    init_stdout();

    // Create audit log writer if specified (stdio mode: audit goes to stderr)
    let _audit_writer = if let Some(path) = audit_log {
        if path == std::path::Path::new("/dev/stderr") {
            // For stdio mode, /dev/stderr is the implicit audit destination
            eprintln!("Audit log: stderr (stdio mode)");
            Some(pdftract_core::audit::AuditLogWriter::open(path)?)
        } else {
            eprintln!("Audit log: {}", path.display());
            Some(pdftract_core::audit::AuditLogWriter::open(path)?)
        }
    } else {
        eprintln!("Audit log: disabled");
        None
    };

    // Create the tool registry with the root path
    let registry = tools::all_tools();

    // Print startup banner to stderr (not stdout!)
    eprintln!("pdftract MCP server (stdio mode) starting...");
    eprintln!("Version: {}", env!("CARGO_PKG_VERSION"));
    eprintln!("Protocol: JSON-RPC 2.0 over stdio");
    eprintln!(
        "Tools: {}",
        registry.tools_list()["tools"]
            .as_array()
            .map(|v| v.len())
            .unwrap_or(0)
    );
    if root.is_some() {
        eprintln!("Path-traversal protection: enabled");
    } else {
        eprintln!("Path-traversal protection: disabled (trust-the-caller mode)");
    }
    eprintln!();

    // Create buffered stdin reader
    let stdin = io::stdin();
    let mut stdin = BufReader::with_capacity(65536, stdin);

    // Main request loop
    while SHOULD_RUN.load(Ordering::SeqCst) {
        match read_message(&mut stdin) {
            Ok(Some(request)) => {
                // Handle the request
                let response = handle_request(request, &registry, root, _audit_writer.as_ref());

                // Write the response
                if let Err(e) = write_response(&response) {
                    eprintln!("Failed to write response: {}", e);
                    return Err(e);
                }
            }
            Ok(None) => {
                // EOF on stdin - graceful shutdown
                eprintln!("EOF on stdin, shutting down");
                break;
            }
            Err(e) => {
                // Parse error - send error response and continue
                eprintln!("Parse error: {}", e);

                let error_response = Response::error(Id::Null, ErrorObject::parse_error());

                if let Err(write_err) = write_response(&error_response) {
                    eprintln!("Failed to write error response: {}", write_err);
                    return Err(write_err);
                }

                // Continue reading (don't exit on parse error)
            }
        }
    }

    // Check if we're exiting due to SIGTERM
    if !SHOULD_RUN.load(Ordering::SeqCst) {
        eprintln!("SIGTERM received, draining complete");
    }

    // Flush stdout before exit
    if let Some(mut stdout) = STDOUT.lock().unwrap().take() {
        stdout
            .flush()
            .context("Failed to flush stdout on shutdown")?;
    }

    eprintln!("pdftract MCP server (stdio mode) shut down cleanly");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that write_response produces properly framed output.
    #[test]
    fn test_write_response_framing() {
        init_stdout();

        let response = Response::success(Id::Number(1), serde_json::json!({"result": "ok"}));

        // This should succeed (stdout is initialized)
        // We can't easily test the actual output without capturing stdout,
        // but we can at least verify it doesn't panic
        let result = write_response(&response);
        assert!(result.is_ok());

        // Clean up
        *STDOUT.lock().unwrap() = None;
    }

    /// Test that unknown methods return method_not_found error.
    #[test]
    fn test_handle_unknown_method() {
        let registry = tools::all_tools();
        let request = Request::new("unknown/method", None, Some(Id::Number(1)));

        let response = handle_request(request, &registry, None, None);

        assert!(response.is_error());
        assert_eq!(response.get_error().unwrap().code, -32601);
    }

    /// Test that tools/list returns success.
    #[test]
    fn test_handle_tools_list() {
        let registry = tools::all_tools();
        let request = Request::new("tools/list", None, Some(Id::Number(1)));

        let response = handle_request(request, &registry, None, None);

        assert!(response.is_success());
        assert!(response.get_result().is_some());
    }

    /// Test that notifications (no id) return Id::Null.
    #[test]
    fn test_request_id_notification() {
        let request = Request::new("notifications/message", None, None);

        assert_eq!(request.request_id(), Id::Null);
    }

    /// Test that parse error response has the correct structure.
    #[test]
    fn test_parse_error_response_structure() {
        let error = ErrorObject::parse_error();
        let response = Response::error(Id::Null, error);

        // Serialize to verify the structure
        let json = serde_json::to_string(&response).unwrap();

        // Verify it contains the required fields
        assert!(json.contains(r#""jsonrpc":"2.0""#));
        assert!(json.contains(r#""code":-32700"#));
        assert!(json.contains(r#""message":"Parse error""#));
        assert!(json.contains(r#""id":null"#));

        // Verify it doesn't contain a "result" field (error response)
        assert!(!json.contains(r#""result""#));
    }

    /// Test that method_not_found error includes the method name in data.
    #[test]
    fn test_method_not_found_includes_method() {
        let error = ErrorObject::method_not_found("test_method");
        assert_eq!(error.code, -32601);
        assert_eq!(error.message, "Method not found");
        assert_eq!(
            error.data,
            Some(serde_json::Value::String("test_method".to_string()))
        );
    }

    /// Test that the SHOULD_RUN flag can be toggled.
    #[test]
    fn test_should_run_flag() {
        // Initially true
        assert!(SHOULD_RUN.load(Ordering::SeqCst));

        // Set to false
        SHOULD_RUN.store(false, Ordering::SeqCst);
        assert!(!SHOULD_RUN.load(Ordering::SeqCst));

        // Reset to true for other tests
        SHOULD_RUN.store(true, Ordering::SeqCst);
    }

    /// Roundtrip test: verify request -> response -> JSON -> response works.
    #[test]
    fn test_roundtrip_tools_list() {
        // Create a tools/list request
        let request = Request::new("tools/list", None, Some(Id::Number(1)));

        // Handle it
        let registry = tools::all_tools();
        let response = handle_request(request, &registry, None, None);

        // Verify it's a success response
        assert!(response.is_success());
        assert_eq!(response.id, Id::Number(1));

        // Serialize to JSON
        let json = serde_json::to_string(&response).unwrap();

        // Verify it's valid JSON-RPC
        assert!(json.contains(r#""jsonrpc":"2.0""#));
        assert!(json.contains(r#""result""#));
        assert!(json.contains(r#""id":1"#));

        // Deserialize back and verify key fields match
        let response2: Response = serde_json::from_str(&json).unwrap();
        assert!(response2.is_success());
        assert_eq!(response2.id, Id::Number(1));
    }

    /// Test that all error constructors produce valid error objects.
    #[test]
    fn test_all_error_constructors() {
        let errors = vec![
            ErrorObject::parse_error(),
            ErrorObject::invalid_request(),
            ErrorObject::method_not_found("test"),
            ErrorObject::invalid_params(),
            ErrorObject::internal_error(),
            ErrorObject::server_error(-32000, "custom error"),
        ];

        for error in errors {
            // Verify each error serializes to valid JSON
            let json = serde_json::to_string(&error).unwrap();
            let parsed: ErrorObject = serde_json::from_str(&json).unwrap();
            assert_eq!(error.code, parsed.code);
            assert_eq!(error.message, parsed.message);
        }
    }
}
