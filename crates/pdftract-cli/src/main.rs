use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;

mod codegen;
mod mcp;
mod password;
use codegen::Language;

// Re-export diagnostics for the --list-diagnostics and --explain-diagnostic commands
pub use pdftract_core::diagnostics::{DiagCode, DiagInfo, DIAGNOSTIC_CATALOG};

#[derive(Parser)]
#[command(name = "pdftract")]
#[command(about = "pdftract CLI - PDF extraction and conformance testing", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all diagnostic codes with their metadata
    ListDiagnostics,
    /// Explain a specific diagnostic code in detail
    ExplainDiagnostic {
        /// Diagnostic code to explain (e.g., STRUCT_MISSING_KEY, STREAM_BOMB)
        code: String,
    },
    /// Compare actual results against expected values with tolerances (for conformance testing)
    Compare {
        /// Path to the actual results JSON
        actual: PathBuf,
        /// Path to the expected results JSON
        expected: PathBuf,
        /// Path to the tolerances JSON (optional)
        #[arg(short, long)]
        tolerances: Option<PathBuf>,
        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },
    /// Run SDK conformance test suite
    Conformance {
        /// Path to the conformance suite JSON
        #[arg(short, long, default_value = "tests/sdk-conformance/cases.json")]
        suite: PathBuf,
        /// SDK name
        #[arg(short, long, default_value = "pdftract")]
        sdk: String,
        /// SDK version
        #[arg(short, long, default_value = "0.1.0")]
        version: String,
        /// Output report path
        #[arg(short, long, default_value = "conformance-report.json")]
        output: PathBuf,
    },
    /// SDK code generation commands
    Sdk {
        #[command(subcommand)]
        sdk_command: SdkCommands,
    },
    /// Extract text and structure from a PDF file
    Extract {
        /// Path to the PDF file (use '-' for stdin)
        input: PathBuf,

        /// Read password from stdin (one line, terminated by newline)
        #[arg(long, conflicts_with = "password")]
        password_stdin: bool,

        /// PDF password (INSECURE: rejected unless PDFTRACT_INSECURE_CLI_PASSWORD=1)
        #[arg(long, conflicts_with = "password_stdin")]
        password: Option<String>,

        /// Output format (json, text, markdown)
        #[arg(short, long, default_value = "json")]
        format: String,
    },
    /// Start the MCP (Model Context Protocol) server
    ///
    /// Per ADR-006: stdio and HTTP transports are mutually exclusive because they have
    /// opposite stdout discipline (stdio: JSON-RPC sink; HTTP: log channel). Exactly one
    /// transport must be selected per invocation.
    Mcp {
        /// Use stdio transport (for Claude Desktop, Claude Code, Continue, Cursor)
        ///
        /// This is the default transport mode if neither --stdio nor --bind is specified.
        #[arg(long, conflicts_with = "bind")]
        stdio: bool,

        /// Bind address for the MCP server (e.g., "127.0.0.1:8080", "[::1]:9000", "0.0.0.0:3000")
        ///
        /// Enables HTTP+SSE transport mode. Mutually exclusive with --stdio.
        #[arg(short, long, value_name = "ADDR", conflicts_with = "stdio")]
        bind: Option<String>,

        /// Path to a file containing the bearer token (RECOMMENDED)
        #[arg(long, conflicts_with = "auth_token")]
        auth_token_file: Option<PathBuf>,

        /// Bearer token for authentication (INSECURE: rejected unless PDFTRACT_INSECURE_CLI_TOKEN=1)
        #[arg(long, conflicts_with = "auth_token_file")]
        auth_token: Option<String>,

        /// Maximum request body size in MB (default: 256)
        #[arg(long, default_value = "256")]
        max_upload_mb: usize,

        /// Root directory for local filesystem access (enforces path-traversal protection)
        ///
        /// When set, all local-path tool arguments are resolved relative to DIR and any
        /// path that escapes DIR is rejected with JSON-RPC error code -32602.
        /// HTTPS URLs are not affected by this flag. Without --root, the server runs in
        /// trust-the-caller mode (no path-check applied).
        #[arg(long, value_name = "DIR")]
        root: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum SdkCommands {
    /// Generate SDK skeleton from templates
    Codegen {
        /// Target language
        #[arg(short, long)]
        lang: Language,
        /// Output directory
        #[arg(short, long)]
        out: PathBuf,
        /// Version string (defaults to current pdftract version)
        #[arg(short, long, default_value = "0.1.0")]
        version: String,
    },
    /// Validate existing SDK against current generator output
    Validate {
        /// Target language
        #[arg(short, long)]
        lang: Language,
        /// Path to existing SDK directory
        #[arg(short, long)]
        sdk_dir: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::ListDiagnostics => {
            cmd_list_diagnostics()?;
        }
        Commands::ExplainDiagnostic { code } => {
            cmd_explain_diagnostic(&code)?;
        }
        Commands::Compare {
            actual,
            expected,
            tolerances,
            format,
        } => {
            cmd_compare(actual, expected, tolerances, &format)?;
        }
        Commands::Conformance {
            suite,
            sdk,
            version,
            output,
        } => {
            cmd_conformance(suite, &sdk, &version, output)?;
        }
        Commands::Sdk { sdk_command } => {
            cmd_sdk(sdk_command)?;
        }
        Commands::Extract {
            input,
            password_stdin,
            password,
            format,
        } => {
            if let Err(e) = cmd_extract(input, password_stdin, password, &format) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Mcp {
            stdio,
            bind,
            auth_token_file,
            auth_token,
            max_upload_mb,
            root,
        } => {
            // Per ADR-006: exactly one transport must be selected.
            // If neither --stdio nor --bind is specified, default to stdio mode.
            let use_stdio = stdio || bind.is_none();

            // Validate and canonicalize the root directory if provided
            let root_path = match root {
                Some(ref root_arg) => {
                    match mcp::canonicalize_root(root_arg) {
                        Ok(canonical) => Some(canonical),
                        Err(e) => {
                            eprintln!("Error: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                None => None,
            };

            // Report root configuration
            if let Some(ref root) = root_path {
                eprintln!("Root directory: {} (path-traversal protection enabled)", root.display());
            } else {
                eprintln!("No root directory (trust-the-caller mode)");
            }

            if use_stdio {
                // stdio mode (default for Claude Desktop, Claude Code, etc.)
                if let Err(e) = mcp::run_stdio(root_path.as_deref()) {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            } else {
                // HTTP mode (--bind was specified)
                let bind_addr = bind.expect("--bind is Some when use_stdio is false");
                if let Err(e) = mcp::run(bind_addr, auth_token_file, auth_token, Some(max_upload_mb), root_path) {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}

fn cmd_extract(
    input: PathBuf,
    password_stdin: bool,
    password: Option<String>,
    format: &str,
) -> Result<()> {
    // Resolve password using the priority order defined in TH-07
    let resolved_password = match password::resolve_password(password_stdin, password) {
        Ok(pwd) => pwd,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(password::EXIT_USAGE_ERROR as i32);
        }
    };

    // Report password status (never the value itself)
    if resolved_password.is_some() {
        eprintln!("Password provided via secure channel");
    }

    // Stub: For now, just report what would be extracted
    // Full extraction implementation is in separate beads
    eprintln!("Extract command invoked");
    eprintln!("  Input: {:?}", input);
    eprintln!("  Format: {}", format);
    eprintln!("  Password: {}", if resolved_password.is_some() { "yes" } else { "no" });

    // TODO: Implement actual PDF extraction
    // This will be done in the extraction implementation beads
    eprintln!("NOTE: Full extraction implementation is pending (see plan for extraction beads)");

    Ok(())
}

fn cmd_list_diagnostics() -> Result<()> {
    println!("pdftract Diagnostic Codes");
    println!();
    println!("This catalog lists all diagnostic codes emitted during PDF parsing and extraction.");
    println!("Each diagnostic includes a severity level, recoverable flag, phase origin, and suggested action.");
    println!();

    // Group by category
    let mut categories: std::collections::HashMap<&str, Vec<&DiagInfo>> = std::collections::HashMap::new();
    for info in DIAGNOSTIC_CATALOG {
        categories.entry(info.category).or_default().push(info);
    }

    // Define category order
    let category_order = vec![
        "STRUCT", "XREF", "STREAM", "ENCRYPTION", "PAGE", "FONT",
        "OCR", "REMOTE", "GSTATE", "LAYOUT", "MCP", "CACHE",
    ];

    for category in category_order {
        if let Some(infos) = categories.get(category) {
            println!("=== {}_* codes ===", category);
            println!();

            for info in infos {
                println!("{} ({})", info.code, info.severity);
                println!("  Phase: {}", info.phase);
                println!("  Recoverable: {}", if info.recoverable { "Yes" } else { "No" });
                println!("  Action: {}", info.suggested_action);
                println!();
            }
        }
    }

    println!("Total: {} diagnostic codes", DIAGNOSTIC_CATALOG.len());
    Ok(())
}

fn cmd_explain_diagnostic(code: &str) -> Result<()> {
    // Normalize the input code (handle case-insensitivity and strip whitespace)
    let code_upper = code.to_uppercase().trim().to_string();

    // Try to find the diagnostic by name in the catalog
    let info = DIAGNOSTIC_CATALOG
        .iter()
        .find(|info| info.code.name() == code_upper)
        .ok_or_else(|| anyhow::anyhow!("Unknown diagnostic code: {}", code))?;

    println!("Diagnostic: {}", info.code);
    println!("Category: {}", info.category);
    println!("Severity: {}", info.severity);
    println!("Recoverable: {}", if info.recoverable { "Yes" } else { "No" });
    println!("Phase Origin: {}", info.phase);
    println!();
    println!("Description:");

    // Get the description from the DiagCode's doc comment
    // We can't access doc comments at runtime, but we can provide useful info
    match info.code {
        DiagCode::StructInvalidName => {
            println!("  Invalid name character or malformed name object");
            println!("  Names containing invalid characters or exceeding the 127-byte limit are truncated.");
        }
        DiagCode::StructInvalidHex => {
            println!("  Invalid hexadecimal character in hex string or name escape");
            println!("  Non-hex characters in <...> strings or #XX escapes are skipped.");
        }
        DiagCode::StructInvalidOctal => {
            println!("  Invalid octal escape sequence in literal string");
            println!("  Invalid \\NNN escapes are passed through literally.");
        }
        DiagCode::StructInvalidStreamHeader => {
            println!("  Invalid stream header");
            println!("  The 'stream' keyword must be followed by CRLF or LF per PDF spec.");
        }
        DiagCode::StructUnexpectedByte => {
            println!("  Unexpected byte during parsing");
            println!("  A byte doesn't match expected token syntax; lexer resynchronizes.");
        }
        DiagCode::StructUnexpectedEof => {
            println!("  Unexpected end of file");
            println!("  The file ends mid-token; parsing continues with partial data.");
        }
        DiagCode::StructUnterminatedString => {
            println!("  Unterminated literal string");
            println!("  A literal string is missing a closing parenthesis.");
        }
        DiagCode::StructMissingKey => {
            println!("  Missing required dictionary key");
            println!("  A required key is absent from a dictionary.");
        }
        DiagCode::StructCircularRef => {
            println!("  Circular reference detected");
            println!("  An indirect reference forms a cycle (A → B → A).");
        }
        DiagCode::StructXobjectCycle => {
            println!("  Form XObject cycle detected");
            println!("  A form XObject invokes itself directly or indirectly.");
        }
        DiagCode::StructDepthExceeded => {
            println!("  Dictionary nesting depth exceeds limit");
            println!("  Structure is too deeply nested; truncated to prevent stack overflow.");
        }
        DiagCode::StructInvalidDictValue => {
            println!("  Invalid dictionary value");
            println!("  A dictionary key is not followed by a value.");
        }
        DiagCode::StructInvalidDictKey => {
            println!("  Invalid dictionary key");
            println!("  A dictionary key is not a name object.");
        }
        DiagCode::StructInvalidIndirectHeader => {
            println!("  Invalid indirect object header");
            println!("  The 'N G obj' header is malformed.");
        }
        DiagCode::StructIntegerOverflow => {
            println!("  Integer overflow during parsing");
            println!("  An integer would overflow i64; value is clamped.");
        }
        DiagCode::StructInvalidObjstm => {
            println!("  Invalid object stream format");
            println!("  An object stream has a malformed header or invalid data.");
        }
        DiagCode::StructInvalidGeometry => {
            println!("  Invalid geometry value");
            println!("  NaN or Inf in MediaBox/CropBox/Rotate; canonicalized to 0.");
        }
        DiagCode::StructInvalidUtf16 => {
            println!("  Invalid UTF-16BE encoding");
            println!("  A UTF-16BE string has odd length or invalid encoding.");
        }
        DiagCode::StructUnresolvedDestination => {
            println!("  Unresolved named destination");
            println!("  An outline references a named destination (not yet resolved).");
        }
        DiagCode::StructNonGotoOutline => {
            println!("  Non-GoTo action in outline");
            println!("  An outline has an action other than GoTo/URI.");
        }
        DiagCode::StructInvalidPdfDocEncoding => {
            println!("  Invalid PDFDocEncoding");
            println!("  A PDFDocEncoding string cannot be decoded to UTF-8.");
        }
        DiagCode::StructHybridConflict => {
            println!("  Hybrid xref conflict");
            println!("  Traditional xref and stream disagree on object state.");
        }
        DiagCode::StructInvalidPrevOffset => {
            println!("  Invalid /Prev offset in xref chain");
            println!("  A trailer's /Prev offset points to invalid data.");
        }
        DiagCode::XrefInvalidHeader => {
            println!("  Invalid xref keyword or header");
            println!("  The xref table doesn't start with the 'xref' keyword.");
        }
        DiagCode::XrefInvalidEntry => {
            println!("  Malformed xref entry");
            println!("  An xref entry doesn't match the 20-byte format.");
        }
        DiagCode::XrefInvalidSubsectionHeader => {
            println!("  Invalid subsection header");
            println!("  An xref subsection header is malformed.");
        }
        DiagCode::XrefObjectZeroNotFree => {
            println!("  Object 0 is not free");
            println!("  Object 0 is marked as in-use, violating PDF spec.");
        }
        DiagCode::XrefTrailerNotFound => {
            println!("  Trailer dictionary not found");
            println!("  The trailer dictionary couldn't be located or parsed.");
        }
        DiagCode::XrefTruncated => {
            println!("  Truncated xref table");
            println!("  The xref table ends unexpectedly.");
        }
        DiagCode::XrefRepaired => {
            println!("  Xref was reconstructed");
            println!("  Forward scan recovered xref entries after primary strategies failed.");
        }
        DiagCode::XrefLinearizedNoForwardScan => {
            println!("  Forward scan disabled for linearized PDF");
            println!("  Forward scan would incorrectly find the partial first-page xref.");
        }
        DiagCode::XrefRemoteNoForwardScan => {
            println!("  Forward scan disabled for remote sources");
            println!("  Forward scan would require fetching the entire file.");
        }
        DiagCode::XrefInvalidStreamFormat => {
            println!("  Invalid xref stream format");
            println!("  An xref stream has a malformed header or invalid /W array.");
        }
        DiagCode::XrefInvalidStreamEntry => {
            println!("  Invalid xref stream entry");
            println!("  An xref stream entry cannot be parsed due to invalid data.");
        }
        DiagCode::StreamDecodeError => {
            println!("  Stream decompression failed");
            println!("  A stream decoder encountered corrupt data mid-decompression.");
        }
        DiagCode::StreamBomb => {
            println!("  Decompression bomb limit exceeded");
            println!("  A stream's decompressed size would exceed the safety limit.");
        }
        DiagCode::StreamUnknownFilter => {
            println!("  Unknown filter name");
            println!("  A stream specifies an unsupported filter.");
        }
        DiagCode::StreamInvalidParams => {
            println!("  Invalid filter parameters");
            println!("  A stream's /DecodeParms dictionary is malformed.");
        }
        DiagCode::EncryptionUnsupported => {
            println!("  Unsupported encryption or no password");
            println!("  PDF is encrypted and no password was supplied or algorithm is unsupported.");
        }
        DiagCode::EncryptionWrongPassword => {
            println!("  Password incorrect");
            println!("  The supplied password doesn't match the PDF's encryption key.");
        }
        DiagCode::PageOutOfRange => {
            println!("  Page number out of range");
            println!("  --pages specifies a page number greater than the document's page count.");
        }
        DiagCode::PageInvalidCount => {
            println!("  Invalid page count");
            println!("  The /Count key in the /Pages tree is invalid.");
        }
        DiagCode::PageInvalidRotate => {
            println!("  Invalid /Rotate value");
            println!("  A page's /Rotate value is not a multiple of 90.");
        }
        DiagCode::FontGlyphUnmapped => {
            println!("  Glyph could not be mapped to Unicode");
            println!("  A glyph has no entry in /ToUnicode CMap, AGL, fingerprint, or shape match.");
        }
        DiagCode::FontNotFound => {
            println!("  Font not found or couldn't be parsed");
            println!("  A referenced font is missing from the PDF or couldn't be parsed.");
        }
        DiagCode::FontInvalidCmap => {
            println!("  Invalid CMap format");
            println!("  A CMap stream is malformed.");
        }
        DiagCode::OcrJbig2Unsupported => {
            println!("  JBIG2 decoder not available");
            println!("  Build with --features full-render to enable JBIG2 decoding.");
        }
        DiagCode::OcrJpxUnsupported => {
            println!("  JPEG2000 decoder not available");
            println!("  Build with --features full-render or install libopenjp2.");
        }
        DiagCode::OcrCcittUnsupported => {
            println!("  CCITT fax decoder not available");
            println!("  Install libtiff system library or build with --features full-render.");
        }
        DiagCode::OcrTesseractFailed => {
            println!("  Tesseract OCR failed");
            println!("  Tesseract crashed or returned an error.");
        }
        DiagCode::OcrBrokenVectorUnavailable => {
            println!("  OCR unavailable on broken-vector page");
            println!("  Build with --features ocr to enable OCR recovery.");
        }
        DiagCode::RemoteFetchInterrupted => {
            println!("  HTTP fetch interrupted or failed");
            println!("  Network error, timeout, or server error occurred.");
        }
        DiagCode::RemoteNoRangeSupport => {
            println!("  Server does not support Range requests");
            println!("  Falls back to downloading the entire file.");
        }
        DiagCode::RemoteTlsFailed => {
            println!("  TLS handshake failed");
            println!("  The TLS handshake failed; check the server's certificate.");
        }
        DiagCode::RemoteDnsFailed => {
            println!("  DNS resolution failed");
            println!("  The hostname could not be resolved.");
        }
        DiagCode::GstateStackOverflow => {
            println!("  Graphics state stack overflow");
            println!("  The graphics state stack exceeded the internal limit.");
        }
        DiagCode::GstateStackUnderflow => {
            println!("  Graphics state stack underflow");
            println!("  More Q operators than q operators in the content stream.");
        }
        DiagCode::GstateBtEtMismatch => {
            println!("  Mismatched BT/ET pair");
            println!("  The content stream has mismatched BT/ET operators.");
        }
        DiagCode::LayoutTaggedPdfDeferred => {
            println!("  Tagged PDF StructTree deferred");
            println!("  StructTree is ignored; XY-cut is used instead (Phase 7.1 pending).");
        }
        DiagCode::LayoutReadingOrderAmbiguous => {
            println!("  Reading order may be incorrect");
            println!("  The reading order algorithm detected ambiguity.");
        }
        DiagCode::LayoutLowReadability => {
            println!("  Low readability score");
            println!("  Page readability is below 0.85; may indicate mojibake.");
        }
        DiagCode::McpToolInvalidParams => {
            println!("  MCP tool call has invalid parameters");
            println!("  An MCP tool call doesn't match the tool's schema.");
        }
        DiagCode::McpPathTraversal => {
            println!("  MCP path traversal attempt");
            println!("  An MCP path escapes the --root directory.");
        }
        DiagCode::CacheEntryCorrupt => {
            println!("  Cache entry is corrupted");
            println!("  A cached entry failed to deserialize and was deleted.");
        }
        DiagCode::CacheWriteFailed => {
            println!("  Cache write failed");
            println!("  Writing to the cache failed (e.g., out of disk space).");
        }
    }

    println!();
    println!("Suggested Action: {}", info.suggested_action);
    println!();
    println!("Phase Origin: {}", info.phase);

    Ok(())
}

fn cmd_compare(actual: PathBuf, expected: PathBuf, tolerances: Option<PathBuf>, format: &str) -> Result<()> {
    let actual_json = fs::read_to_string(&actual)
        .context(format!("Failed to read actual results from {:?}", actual))?;
    let actual_val: serde_json::Value = serde_json::from_str(&actual_json)
        .context("Failed to parse actual results as JSON")?;

    let expected_json = fs::read_to_string(&expected)
        .context(format!("Failed to read expected results from {:?}", expected))?;
    let expected_val: serde_json::Value = serde_json::from_str(&expected_json)
        .context("Failed to parse expected results as JSON")?;

    let tolerances_val = if let Some(tol_path) = tolerances {
        let tol_json = fs::read_to_string(&tol_path)
            .context(format!("Failed to read tolerances from {:?}", tol_path))?;
        Some(serde_json::from_str::<serde_json::Value>(&tol_json)
            .context("Failed to parse tolerances as JSON")?)
    } else {
        None
    };

    let result = compare_values(&actual_val, &expected_val, tolerances_val.as_ref())?;

    match format {
        "json" => {
            let output = serde_json::to_string_pretty(&result)?;
            println!("{}", output);
        }
        _ => {
            print_compare_result(&result);
        }
    }

    Ok(())
}

fn cmd_sdk(command: SdkCommands) -> Result<()> {
    match command {
        SdkCommands::Codegen { lang, out, version } => {
            let template_dir = PathBuf::from("templates/sdk-skeleton");
            let mut generator = codegen::CodeGenerator::new(&template_dir, version)?;
            generator.generate(lang, &out)?;
            println!("\nSDK generated successfully to: {}", out.display());
        }
        SdkCommands::Validate { lang, sdk_dir } => {
            let template_dir = PathBuf::from("templates/sdk-skeleton");
            let mut generator = codegen::CodeGenerator::new(&template_dir, "0.1.0".to_string())?;
            let result = generator.validate(lang, &sdk_dir)?;

            if result.differences.is_empty() {
                println!("SDK is up to date with current generator output.");
            } else {
                println!("Found {} differences:", result.differences.len());
                for diff in &result.differences {
                    match diff.kind {
                        codegen::DifferenceKind::MissingInSdk => {
                            println!("  MISSING: {}", diff.path);
                        }
                        codegen::DifferenceKind::ExtraInSdk => {
                            println!("  EXTRA: {}", diff.path);
                        }
                        codegen::DifferenceKind::ContentDiff => {
                            println!("  MODIFIED: {}", diff.path);
                        }
                    }
                }
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

fn cmd_conformance(suite: PathBuf, sdk: &str, version: &str, output: PathBuf) -> Result<()> {
    println!("Running conformance suite: {:?}", suite);
    println!("SDK: {} v{}", sdk, version);
    println!("Output: {:?}", output);

    let suite_json = fs::read_to_string(&suite)
        .context(format!("Failed to read suite from {:?}", suite))?;
    let suite_val: serde_json::Value = serde_json::from_str(&suite_json)
        .context("Failed to parse suite as JSON")?;

    let cases = suite_val
        .get("cases")
        .and_then(|v| v.as_array())
        .context("Suite missing 'cases' array")?;

    println!("\nFound {} test cases", cases.len());

    // This is a stub - actual implementation would invoke the SDK
    let results: Vec<serde_json::Value> = cases
        .iter()
        .map(|case| {
            serde_json::json!({
                "id": case.get("id").unwrap_or(&serde_json::json!("unknown")),
                "status": "skip",
                "error": "SDK conformance runner not yet implemented - use language-specific runner"
            })
        })
        .collect();

    let report = serde_json::json!({
        "sdk": sdk,
        "sdk_version": version,
        "suite_version": suite_val.get("version").unwrap_or(&serde_json::json!("unknown")),
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "results": results,
        "summary": {
            "total": results.len(),
            "passed": 0,
            "failed": 0,
            "skipped": results.len(),
            "errors": 0
        }
    });

    fs::write(&output, serde_json::to_string_pretty(&report)?)
        .context(format!("Failed to write report to {:?}", output))?;

    println!("\nReport written to {:?}", output);
    Ok(())
}

#[derive(Debug, serde::Serialize)]
enum CompareResult {
    Pass,
    Fail { reason: String },
    Missing,
}

fn compare_values(
    actual: &serde_json::Value,
    expected: &serde_json::Value,
    tolerances: Option<&serde_json::Value>,
) -> Result<std::collections::HashMap<String, CompareResult>> {
    let mut results = std::collections::HashMap::new();

    compare_recursive(actual, expected, tolerances, "", &mut results);

    Ok(results)
}

fn compare_recursive(
    actual: &serde_json::Value,
    expected: &serde_json::Value,
    tolerances: Option<&serde_json::Value>,
    path: &str,
    results: &mut std::collections::HashMap<String, CompareResult>,
) {
    match (actual, expected) {
        // Handle min/max constraints
        (serde_json::Value::Number(act), serde_json::Value::Object(exp)) => {
            if let Some(min) = exp.get("min").and_then(|v| v.as_i64()) {
                if act.as_i64().map_or(true, |v| v < min) {
                    results.insert(
                        path.to_string(),
                        CompareResult::Fail {
                            reason: format!("value {} is less than minimum {}", act, min),
                        },
                    );
                    return;
                }
            }
            if let Some(max) = exp.get("max").and_then(|v| v.as_i64()) {
                if act.as_i64().map_or(true, |v| v > max) {
                    results.insert(
                        path.to_string(),
                        CompareResult::Fail {
                            reason: format!("value {} is greater than maximum {}", act, max),
                        },
                    );
                    return;
                }
            }
            if let Some(val) = exp.get("value") {
                let tol = find_tolerance(tolerances, path);
                let result = compare_with_tolerance(act, val, tol);
                results.insert(path.to_string(), result);
            } else {
                results.insert(path.to_string(), CompareResult::Pass);
            }
        }
        // String constraints
        (serde_json::Value::String(act), serde_json::Value::Object(exp)) => {
            if let Some(min_len) = exp.get("min_length").and_then(|v| v.as_u64()).map(|v| v as usize) {
                if act.len() < min_len {
                    results.insert(
                        path.to_string(),
                        CompareResult::Fail {
                            reason: format!(
                                "string length {} is less than minimum {}",
                                act.len(),
                                min_len
                            ),
                        },
                    );
                    return;
                }
            }
            if let Some(containers) = exp.get("contains").and_then(|v| v.as_array()) {
                for substring in containers {
                    if let Some(s) = substring.as_str() {
                        if !act.contains(s) {
                            results.insert(
                                path.to_string(),
                                CompareResult::Fail {
                                    reason: format!("string does not contain '{}'", s),
                                },
                            );
                            return;
                        }
                    }
                }
            }
            results.insert(path.to_string(), CompareResult::Pass);
        }
        // Array length constraints
        (serde_json::Value::Array(act), serde_json::Value::Object(exp)) => {
            if let Some(min_len) = exp.get("min").and_then(|v| v.as_u64()).map(|v| v as usize) {
                if act.len() < min_len {
                    results.insert(
                        path.to_string(),
                        CompareResult::Fail {
                            reason: format!(
                                "array length {} is less than minimum {}",
                                act.len(),
                                min_len
                            ),
                        },
                    );
                    return;
                }
            }
            if let Some(max_len) = exp.get("max").and_then(|v| v.as_u64()).map(|v| v as usize) {
                if act.len() > max_len {
                    results.insert(
                        path.to_string(),
                        CompareResult::Fail {
                            reason: format!(
                                "array length {} is greater than maximum {}",
                                act.len(),
                                max_len
                            ),
                        },
                    );
                    return;
                }
            }
            results.insert(path.to_string(), CompareResult::Pass);
        }
        // Direct comparison
        (a, e) => {
            if a == e {
                results.insert(path.to_string(), CompareResult::Pass);
            } else {
                results.insert(
                    path.to_string(),
                    CompareResult::Fail {
                        reason: format!("expected {:?}, got {:?}", e, a),
                    },
                );
            }
        }
    }
}

fn compare_with_tolerance(
    actual: &serde_json::Number,
    expected: &serde_json::Value,
    tolerance: Option<&serde_json::Value>,
) -> CompareResult {
    let act_val = actual.as_f64().unwrap();
    let exp_val = match expected {
        serde_json::Value::Number(n) => n.as_f64().unwrap(),
        _ => return CompareResult::Fail { reason: "expected value is not a number".to_string() },
    };

    if let Some(tol) = tolerance {
        if let Some(obj) = tol.as_object() {
            if let Some(abs_tol) = obj.get("abs").and_then(|v| v.as_f64()) {
                let diff = (act_val - exp_val).abs();
                if diff <= abs_tol {
                    return CompareResult::Pass;
                }
            }
            if let Some(rel_tol) = obj.get("rel").and_then(|v| v.as_f64()) {
                let diff = (act_val - exp_val).abs();
                let avg = (act_val + exp_val) / 2.0;
                if avg > 0.0 && diff / avg <= rel_tol {
                    return CompareResult::Pass;
                }
            }
        }
    }

    // Direct comparison
    if (act_val - exp_val).abs() < f64::EPSILON {
        CompareResult::Pass
    } else {
        CompareResult::Fail {
            reason: format!("numeric mismatch: {} vs {}", act_val, exp_val),
        }
    }
}

fn find_tolerance<'a>(
    tolerances: Option<&'a serde_json::Value>,
    path: &str,
) -> Option<&'a serde_json::Value> {
    let tol = tolerances?;
    if let Some(obj) = tol.as_object() {
        // Try exact path match
        if let Some(val) = obj.get(path) {
            return Some(val);
        }
        // Try wildcard patterns
        for (key, val) in obj {
            if key.contains('*') {
                let pattern = key.replace('*', ".*");
                if let Ok(re) = regex::Regex::new(&pattern) {
                    if re.is_match(path) {
                        return Some(val);
                    }
                }
            }
        }
    }
    None
}

fn print_compare_result(results: &std::collections::HashMap<String, CompareResult>) {
    let mut passed = 0;
    let mut failed = 0;

    for (path, result) in results {
        match result {
            CompareResult::Pass => {
                passed += 1;
            }
            CompareResult::Fail { reason } => {
                failed += 1;
                eprintln!("FAIL [{}]: {}", path, reason);
            }
            CompareResult::Missing => {
                failed += 1;
                eprintln!("MISSING [{}]: value not found in actual", path);
            }
        }
    }

    println!("\nComparison complete:");
    println!("  Passed: {}", passed);
    println!("  Failed: {}", failed);

    if failed > 0 {
        std::process::exit(1);
    }
}
