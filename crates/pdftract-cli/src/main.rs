use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ArgAction};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

mod cache_cmd;
mod classify;
mod codegen;
mod doctor;
mod grep;
mod hash;
mod header;
mod inspect;
mod mcp;
mod migrate;
mod middleware;
mod output;
mod pages;
mod panic_hook;
mod password;
mod profiles_cmd;
mod serve;
mod url;
mod validate;
mod verify_receipt;
use codegen::Language;
use output::OutputConfig;
use pdftract_core::atomic_file_writer::AtomicFileWriter;
use pdftract_core::cache;
use pdftract_core::extract::{extract_pdf, result_to_json};
use pdftract_core::markdown::{block_to_markdown, page_to_markdown, page_to_markdown_with_links, page_to_markdown_with_links_and_footnotes, MarkdownOptions};
use pdftract_core::options::{ExtractionOptions, ReceiptsMode};
use pdftract_core::text::{serialize_document_text, TextOptions};

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

        /// Custom HTTP headers for remote sources (repeatable; format: HEADER:VALUE)
        #[arg(long, value_name = "HEADER:VALUE", action = ArgAction::Append)]
        header: Vec<String>,

        /// Page range to extract (1-based, comma-separated: 1-5,7,12-)
        #[arg(long, value_name = "RANGE")]
        pages: Option<String>,

        /// Output JSON to PATH (use '-' for stdout)
        #[arg(long, value_name = "PATH")]
        json: Vec<PathBuf>,

        /// Output Markdown to PATH (use '-' for stdout)
        #[arg(long, value_name = "PATH")]
        md: Vec<PathBuf>,

        /// Output plain text to PATH (use '-' for stdout)
        #[arg(long, value_name = "PATH")]
        text: Vec<PathBuf>,

        /// Output NDJSON to stdout (mutually exclusive with other formats)
        #[arg(long, conflicts_with_all = ["json", "md", "text", "format"])]
        ndjson: bool,

        /// Output formats (comma-separated: json,markdown,text,ndjson)
        #[arg(long, value_delimiter = ',', value_name = "FORMATS")]
        format: Vec<String>,

        /// Base path for auto-named outputs (used with --format)
        #[arg(short, long, value_name = "BASE")]
        output: Option<PathBuf>,

        /// Receipt mode: off (default), lite, or svg
        #[arg(long, value_name = "MODE", default_value = "off", value_parser = ["off", "lite", "svg"])]
        receipts: String,

        /// Enable OCR for scanned pages (requires 'ocr' feature)
        #[arg(long)]
        ocr: bool,

        /// OCR language codes (comma-separated, e.g., 'eng,fra,deu')
        #[arg(long, value_delimiter = ',')]
        ocr_language: Vec<String>,

        /// Enable cache at this directory (creates if absent)
        #[arg(long, value_name = "DIR")]
        cache_dir: Option<PathBuf>,

        /// Set cache size limit (default 1 GiB; accepts KiB, MiB, GiB suffixes)
        #[arg(long, value_name = "SIZE", default_value = "1 GiB")]
        cache_size: String,

        /// Disable cache for this extraction (even if --cache-dir is set)
        #[arg(long)]
        no_cache: bool,

        /// Emit HTML comment anchors before each block in Markdown output
        #[arg(long)]
        md_anchors: bool,

        /// Suppress page-break horizontal rules between pages
        #[arg(long)]
        md_no_page_breaks: bool,

        /// Auto-detect document type and apply appropriate profile
        #[arg(long)]
        auto: bool,

        /// Force-apply a specific profile (by name or YAML file path)
        #[arg(long, value_name = "NAME|PATH")]
        profile: Option<String>,

        /// Include header blocks in output
        #[arg(long)]
        include_headers: bool,

        /// Include footer blocks in output
        #[arg(long)]
        include_footers: bool,

        /// Include both header and footer blocks in output
        #[arg(long)]
        include_headers_footers: bool,

        /// Include invisible text spans in output (rendering_mode == 3)
        #[arg(long)]
        include_invisible_text: bool,

        /// Include hidden-layer text spans in output (OCG-controlled)
        #[arg(long)]
        include_hidden_layers: bool,

        /// Include watermark blocks in output (no-op until Phase 7)
        #[arg(long)]
        include_watermarks: bool,
    },
    /// Classify document type (runs metadata + signal extraction, not full text extraction)
    Classify {
        /// Path to the PDF file
        input: PathBuf,

        /// Read password from stdin (one line, terminated by newline)
        #[arg(long, conflicts_with = "password")]
        password_stdin: bool,

        /// PDF password (INSECURE: rejected unless PDFTRACT_INSECURE_CLI_PASSWORD=1)
        #[arg(long, conflicts_with = "password_stdin")]
        password: Option<String>,

        /// Directory containing custom profile YAML files
        #[arg(long, value_name = "DIR")]
        profiles: Option<PathBuf>,

        /// Pretty-print JSON output
        #[arg(long)]
        pretty: bool,

        /// Number of top reasons to include (default: all)
        #[arg(long, default_value = "0")]
        top_k: usize,

        /// Exit with code 1 if document type is unknown
        #[arg(long)]
        exit_on_unknown: bool,
    },
    /// Search for text patterns in PDF files with bounding-box results
    #[cfg(feature = "grep")]
    Grep(grep::GrepArgs),
    /// Inspect a PDF file in a local web browser with debugging overlays
    Inspect(inspect::InspectArgs),
    /// Verify a receipt against a PDF file
    VerifyReceipt(verify_receipt::VerifyReceiptCommand),
    /// Compute the PDF structural fingerprint (hash)
    Hash {
        /// Path to the PDF file or URL
        input: String,

        /// PDF password (INSECURE: rejected unless PDFTRACT_INSECURE_CLI_PASSWORD=1)
        #[arg(long)]
        password: Option<String>,

        /// Custom HTTP headers for remote sources (repeatable; format: HEADER:VALUE)
        #[arg(long, value_name = "HEADER:VALUE", action = ArgAction::Append)]
        header: Vec<String>,
    },
    /// Manage the extraction cache
    Cache {
        #[command(subcommand)]
        cache_command: CacheCommands,
    },
    /// Manage document type profiles
    Profiles {
        #[command(subcommand)]
        profiles_command: ProfilesCommands,
    },
    /// Start the HTTP server for extraction
    ///
    /// ## Security Model
    ///
    /// **pdftract serve has no built-in authentication.** Deploy behind a reverse proxy
    /// (nginx, Traefik, Caddy) for production use. The server accepts PDFs via multipart
    /// upload only; no endpoint accepts file paths from server filesystem.
    ///
    /// ## Concurrency
    ///
    /// The server uses a two-level concurrency architecture:
    ///
    /// - **tokio**: Per-request concurrency via the async executor. Each HTTP request
    ///   is handled asynchronously on tokio's multi-threaded runtime.
    /// - **rayon**: Per-document parallelism within each extraction. PDF pages are
    ///   processed in parallel using rayon's work-stealing thread pool.
    ///
    /// The bridge between async (tokio) and sync (rayon) is `tokio::task::spawn_blocking`.
    /// Each POST handler wraps the synchronous extraction call in `spawn_blocking`, which
    /// runs the work on tokio's blocking thread pool (separate from the async reactor).
    ///
    /// This design ensures:
    /// - The async reactor is never blocked by extraction work
    /// - Multiple PDFs can be extracted concurrently (one per request)
    /// - Within each PDF, pages are processed in parallel (rayon)
    /// - Thread pools are sized appropriately (tokio: 512 blocking threads; rayon: num_cpus)
    ///
    /// ## Endpoints
    ///
    /// - `POST /extract` - Extract PDF and return JSON with metadata
    /// - `POST /extract/text` - Extract PDF and return plain text
    /// - `POST /extract/stream` - Extract PDF and return streaming NDJSON
    /// - `GET /health` - Health check (responds within 100ms even during concurrent extractions)
    ///
    /// ## Cache
    ///
    /// Cache is optional. When enabled, extracted results are stored on disk and reused
    /// for identical PDFs. Cache status is reported via the `X-Pdftract-Cache` response header.
    Serve {
        /// Bind address (e.g., "127.0.0.1:8080", "[::1]:9000", "0.0.0.0:3000")
        #[arg(short, long, default_value = "127.0.0.1:8080")]
        bind: String,

        /// Enable cache at this directory
        #[arg(long, value_name = "DIR")]
        cache_dir: Option<PathBuf>,

        /// Set cache size limit (default 1 GiB; accepts KiB, MiB, GiB suffixes)
        #[arg(long, value_name = "SIZE", default_value = "1 GiB")]
        cache_size: String,

        /// Disable cache
        #[arg(long)]
        no_cache: bool,

        /// Maximum request body size in MB (default: 256, max: 4096)
        #[arg(long, default_value = "256")]
        max_upload_mb: usize,

        /// Maximum decompression size in GB (default: 1, overrides per-request max_decompress_gb)
        #[arg(long, value_name = "GB", default_value = "1")]
        max_decompress_gb: usize,

        /// Write per-request audit log to FILE (NDJSON; use "-" for stdout, "/dev/stderr" for stderr)
        ///
        /// Rotation: pdftract does NOT rotate logs; configure logrotate on the audit-log file.
        /// When FILE is "-", rotation is the responsibility of the supervisor (e.g., journald).
        #[arg(long, value_name = "FILE")]
        audit_log: Option<PathBuf>,

        /// Trust X-Forwarded-For header for client IP detection (DANGER: enables IP spoofing if not behind a trusted proxy)
        #[arg(long)]
        trust_forwarded_for: bool,

        /// Directory containing custom profile YAML files (repeatable)
        #[arg(long, value_name = "DIR")]
        profile_dir: Option<PathBuf>,

        /// Enable hot-reload for profiles (re-read directory on every request)
        #[arg(long)]
        profile_hot_reload: bool,
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

        /// Write per-request audit log to FILE (NDJSON; use "-" for stdout, "/dev/stderr" for stderr)
        ///
        /// Rotation: pdftract does NOT rotate logs; configure logrotate on the audit-log file.
        /// When FILE is "-", rotation is the responsibility of the supervisor (e.g., journald).
        #[arg(long, value_name = "FILE")]
        audit_log: Option<PathBuf>,
    },
    /// Validate a JSON file against the pdftract schema
    Validate {
        /// Path to the JSON file to validate (use '-' for stdin)
        file: String,

        /// Path to a custom schema file (default: bundled v1.0 schema)
        #[arg(short, long, value_name = "PATH")]
        schema: Option<String>,

        /// Quiet mode - suppress error output (only exit code matters)
        #[arg(short, long)]
        quiet: bool,
    },
    /// Migrate JSON output between schema versions
    MigrateSchema {
        /// Source schema version (e.g., "1.0", "1.1")
        #[arg(long)]
        from: String,

        /// Target schema version (e.g., "1.0", "1.1")
        #[arg(long)]
        to: String,

        /// Input JSON file (use '-' for stdin)
        #[arg(default_value = "-")]
        input: String,

        /// Output JSON file (use '-' for stdout)
        #[arg(short, long, default_value = "-")]
        output: String,

        /// Pretty-print output JSON
        #[arg(short, long)]
        pretty: bool,
    },
    /// Check environment health and dependencies
    ///
    /// Exit code policy: exits 0 if no checks FAIL (WARN does not affect exit code);
    /// exits 1 if any check FAILs; exits 2 on argument parse errors.
    Doctor {
        /// Print compiled features and exit
        #[arg(long)]
        features: bool,

        /// Output results as JSON
        #[arg(long)]
        json: bool,

        /// Disable colored output
        #[arg(long)]
        no_color: bool,

        /// Explicit form of the default policy (exit 1 if any check FAILs).
        ///
        /// This flag is the default behavior and is provided for CI script
        /// readability. WARN does not affect exit code regardless of this flag.
        #[arg(long)]
        exit_on_fail: bool,

        /// Verify the profile search path includes DIR
        #[arg(long, value_name = "DIR")]
        profile_dir: Option<PathBuf>,

        /// Verify DIR is writable and has sufficient space
        #[arg(long, value_name = "DIR")]
        cache_dir: Option<PathBuf>,

        /// Requested OCR languages (default: eng)
        #[arg(long, value_delimiter = ',')]
        lang: Vec<String>,
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

#[derive(Subcommand)]
enum CacheCommands {
    /// Show cache statistics
    Stats {
        /// Path to the cache directory
        dir: PathBuf,
        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },
    /// Clear all cache entries (preserves index.json and sentinel)
    Clear {
        /// Path to the cache directory
        dir: PathBuf,
        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },
    /// Purge old cache entries
    Purge {
        /// Path to the cache directory
        dir: PathBuf,
        /// Delete entries older than this duration (e.g., "30d", "7d", "1h")
        #[arg(long, value_name = "DURATION")]
        older_than: Option<String>,
        /// Delete entries matching this version constraint (e.g., "<1.0.0")
        #[arg(long, value_name = "CONSTRAINT")]
        version: Option<String>,
    },
}

#[derive(Subcommand)]
enum ProfilesCommands {
    /// List all available profiles
    List,
    /// Show a profile's YAML content
    Show {
        /// Profile name or path to YAML file
        name_or_path: String,
    },
    /// Export a built-in profile to stdout
    Export {
        /// Name of the built-in profile to export
        name: String,
    },
    /// Install a profile to the user config directory
    Install {
        /// Path to the profile YAML file to install
        path: PathBuf,
    },
    /// Validate a profile file
    Validate {
        /// Path to the profile YAML file to validate
        path: PathBuf,
    },
}

fn main() -> Result<()> {
    // Install panic hook for SecretString redaction in backtraces
    // This ensures credentials never leak in crash dumps
    panic_hook::install_panic_hook();

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
            header,
            pages,
            json,
            md,
            text,
            ndjson,
            format,
            receipts,
            ocr,
            ocr_language,
            cache_dir,
            cache_size,
            no_cache,
            md_anchors,
            md_no_page_breaks,
            auto,
            profile,
            output,
            include_headers,
            include_footers,
            include_headers_footers,
            include_invisible_text,
            include_hidden_layers,
            include_watermarks,
        } => {
            if let Err(e) = cmd_extract(
                input,
                password_stdin,
                password,
                header,
                pages,
                json.into_iter().collect(),
                md.into_iter().collect(),
                text.into_iter().collect(),
                ndjson,
                format,
                output,
                &receipts,
                ocr,
                ocr_language,
                cache_dir,
                &cache_size,
                no_cache,
                md_anchors,
                md_no_page_breaks,
                auto,
                profile,
                include_headers,
                include_footers,
                include_headers_footers,
                include_invisible_text,
                include_hidden_layers,
                include_watermarks,
            ) {
                let error_msg = e.to_string();
                eprintln!("Error: {}", error_msg);

                // Exit code 3 for encryption errors (per spec)
                if error_msg.contains("decryption failed") ||
                   error_msg.contains("PDF decryption failed") ||
                   error_msg.contains("Unsupported encryption") ||
                   error_msg.contains("Wrong password") {
                    std::process::exit(3);
                }
                std::process::exit(1);
            }
        }
        Commands::Classify {
            input,
            password_stdin,
            password,
            profiles,
            pretty,
            top_k,
            exit_on_unknown,
        } => {
            if let Err(e) = cmd_classify(
                input,
                password_stdin,
                password,
                profiles,
                pretty,
                top_k,
                exit_on_unknown,
            ) {
                let error_msg = e.to_string();
                eprintln!("Error: {}", error_msg);

                // Exit code 3 for encryption errors (per spec)
                if error_msg.contains("decryption failed") ||
                   error_msg.contains("PDF decryption failed") ||
                   error_msg.contains("Unsupported encryption") ||
                   error_msg.contains("Wrong password") {
                    std::process::exit(3);
                }
                std::process::exit(1);
            }
        }
        #[cfg(feature = "grep")]
        Commands::Grep(args) => {
            if let Err(e) = grep::run_grep(args) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Inspect(args) => {
            if let Err(e) = cmd_inspect(args) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Cache { cache_command } => {
            if let Err(e) = cmd_cache(cache_command) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Profiles { profiles_command } => {
            if let Err(e) = cmd_profiles(profiles_command) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Serve {
            bind,
            cache_dir,
            cache_size,
            no_cache,
            max_upload_mb,
            max_decompress_gb,
            audit_log,
            trust_forwarded_for,
            profile_dir,
            profile_hot_reload,
        } => {
            if let Err(e) = cmd_serve(
                bind,
                cache_dir,
                &cache_size,
                no_cache,
                max_upload_mb,
                max_decompress_gb,
                audit_log,
                trust_forwarded_for,
            ) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::VerifyReceipt(cmd) => {
            if let Err(e) = verify_receipt::run_verify_receipt(cmd) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Hash {
            input,
            password,
            header,
        } => {
            // Parse and validate custom HTTP headers
            let headers = if !header.is_empty() {
                match header::parse_headers(&header) {
                    Ok(h) => {
                        // Check if input is a URL (https:// or http://)
                        if input.starts_with("http://") || input.starts_with("https://") {
                            // Convert HashMap to Vec for HashArgs
                            h.into_iter().collect()
                        } else {
                            // Local file: headers don't apply
                            Vec::new()
                        }
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(2);
                    }
                }
            } else {
                Vec::new()
            };

            let args = hash::HashArgs {
                input,
                password,
                headers,
            };

            if let Err(e) = hash::run_hash(args) {
                let exit_code = hash::map_error_to_exit_code(&e);
                eprintln!("Error: {}", e);
                std::process::exit(exit_code);
            }
        }
        Commands::Mcp {
            stdio,
            bind,
            auth_token_file,
            auth_token,
            max_upload_mb,
            root,
            audit_log,
        } => {
            // Per ADR-006: exactly one transport must be selected.
            // If neither --stdio nor --bind is specified, default to stdio mode.
            let use_stdio = stdio || bind.is_none();

            // Validate and canonicalize the root directory if provided
            let root_path = match root {
                Some(ref root_arg) => match mcp::canonicalize_root(root_arg) {
                    Ok(canonical) => Some(canonical),
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                },
                None => None,
            };

            // Report root configuration
            if let Some(ref root) = root_path {
                eprintln!(
                    "Root directory: {} (path-traversal protection enabled)",
                    root.display()
                );
            } else {
                eprintln!("No root directory (trust-the-caller mode)");
            }

            if use_stdio {
                // stdio mode (default for Claude Desktop, Claude Code, etc.)
                if let Err(e) = mcp::run_stdio(root_path.as_deref(), audit_log.as_deref()) {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            } else {
                // HTTP mode (--bind was specified)
                let bind_addr = bind.expect("--bind is Some when use_stdio is false");
                if let Err(e) = mcp::run(
                    bind_addr,
                    auth_token_file,
                    auth_token,
                    Some(max_upload_mb),
                    root_path,
                    audit_log,
                ) {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Validate {
            file,
            schema,
            quiet,
        } => {
            if let Err(e) = validate::run_validate(validate::ValidateArgs {
                file,
                schema_path: schema,
                quiet,
            }) {
                // Validation failed - exit 1 (error already printed by run_validate unless quiet)
                if !quiet {
                    eprintln!("Error: {}", e);
                }
                std::process::exit(1);
            }
        }
        Commands::MigrateSchema {
            from,
            to,
            input,
            output,
            pretty,
        } => {
            if let Err(e) = migrate::run_migration(&from, &to, &input, &output, pretty) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Doctor {
            features,
            json,
            no_color,
            exit_on_fail,
            profile_dir,
            cache_dir,
            lang,
        } => {
            if let Err(e) = doctor::run(doctor::DoctorOptions {
                features,
                json,
                no_color,
                exit_on_fail,
                profile_dir,
                cache_dir,
                lang,
            }) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

fn cmd_extract(
    input: PathBuf,
    password_stdin: bool,
    password: Option<String>,
    header: Vec<String>,
    pages: Option<String>,
    json: Vec<PathBuf>,
    md: Vec<PathBuf>,
    text: Vec<PathBuf>,
    ndjson: bool,
    format: Vec<String>,
    output: Option<PathBuf>,
    receipts: &str,
    ocr: bool,
    ocr_language: Vec<String>,
    cache_dir: Option<PathBuf>,
    cache_size: &str,
    no_cache: bool,
    md_anchors: bool,
    md_no_page_breaks: bool,
    auto: bool,
    profile: Option<String>,
    include_headers: bool,
    include_footers: bool,
    include_headers_footers: bool,
    include_invisible_text: bool,
    include_hidden_layers: bool,
    include_watermarks: bool,
) -> Result<()> {
    // Validate receipts mode
    let receipts_mode = match ReceiptsMode::from_str(receipts) {
        Ok(mode) => mode,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(2);
        }
    };

    // Validate output configuration
    let output_config = OutputConfig {
        json,
        md,
        text,
        ndjson,
        format_list: format.clone(),
        output_base: output.clone(),
    };

    let output_specs = match output_config.build_specs() {
        Ok(specs) => specs,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(2);
        }
    };

    // Report what outputs will be produced
    if output_specs.len() > 1 {
        eprintln!("Producing {} outputs:", output_specs.len());
        for spec in &output_specs {
            let dest_name = match &spec.dest {
                output::Destination::Stdout => "stdout".to_string(),
                output::Destination::File(p) => p.display().to_string(),
            };
            eprintln!("  {} -> {}", spec.format.name(), dest_name);
        }
    }

    // Check if SVG mode is requested but feature is not available
    if receipts_mode == ReceiptsMode::SvgClip {
        #[cfg(not(feature = "receipts"))]
        {
            eprintln!("Error: --receipts=svg requires the 'receipts' feature to be enabled");
            eprintln!("Build pdftract with: --features receipts");
            std::process::exit(2);
        }
    }

    // Check if OCR is requested but feature is not available
    if ocr {
        #[cfg(not(feature = "ocr"))]
        {
            eprintln!("Error: --ocr requires the 'ocr' feature to be enabled");
            eprintln!("Build pdftract with: --features ocr");
            std::process::exit(2);
        }
    }

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

    // Check if input is a URL
    let input_str = input.to_string_lossy().to_string();
    let is_url = input_str.starts_with("http://") || input_str.starts_with("https://");

    // Parse and validate custom HTTP headers
    let custom_headers = if !header.is_empty() {
        match header::parse_headers(&header) {
            Ok(h) => {
                if is_url {
                    eprintln!("Custom HTTP headers: {}", h.len());
                    h
                } else {
                    // Local file: headers don't apply, but we don't error
                    std::collections::HashMap::new()
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(2);
            }
        }
    } else {
        std::collections::HashMap::new()
    };

    // Parse URL credentials if present
    let (url_for_source, parsed_url) = if is_url {
        match url::parse_url(&input_str) {
            Ok(parsed) => {
                if parsed.has_credentials {
                    eprintln!("Warning: URL contains credentials that are visible in shell history.");
                    eprintln!("Consider using --header 'Authorization: Bearer TOKEN' instead.");
                }
                (parsed.url.clone(), Some(parsed))
            }
            Err(e) => {
                eprintln!("Error parsing URL: {}", e);
                std::process::exit(2);
            }
        }
    } else {
        (input_str.clone(), None)
    };

    // Build extraction options
    let mut options = ExtractionOptions::with_receipts(receipts_mode);

    // Configure password
    options.password = resolved_password;

    // Configure page range
    options.pages = pages;

    // Configure output filtering options
    options.output.include_headers = include_headers || include_headers_footers;
    options.output.include_footers = include_footers || include_headers_footers;
    options.output.include_invisible = include_invisible_text;
    options.output.include_hidden_layers = include_hidden_layers;
    options.output.include_watermarks = include_watermarks;

    // Handle --auto flag: run classifier first
    #[cfg(feature = "profiles")]
    if auto {
        eprintln!("Auto-detecting document type...");

        use pdftract_core::profiles::{
            classify_and_select_profile, extract_signals_from_results, load_extraction_profiles,
            apply_extraction_tuning, apply_profile_to_metadata,
        };

        // Load all extraction profiles
        let profiles = load_extraction_profiles(&[]).unwrap_or_default();

        if !profiles.is_empty() {
            // Perform a lightweight extraction for classification
            let classify_options = ExtractionOptions::default();
            if let Ok(classify_result) = extract_pdf(&input, &classify_options) {
                let has_signature_field = !classify_result.signatures.is_empty();
                let has_form_field = !classify_result.form_fields.is_empty();

                let page_data: Vec<(Vec<_>, Vec<_>)> = classify_result
                    .pages
                    .iter()
                    .map(|p| (p.blocks.clone(), p.spans.clone()))
                    .collect();

                let selected_profile = classify_and_select_profile(
                    &profiles.iter().map(|p| p.profile.clone()).collect::<Vec<_>>(),
                    &page_data,
                    has_signature_field,
                    has_form_field,
                );

                if let Some((profile, match_result)) = selected_profile {
                    eprintln!(
                        "Document type: {} (confidence: {:.2})",
                        profile.name, match_result.confidence
                    );

                    // Apply profile extraction tuning
                    if let Some(ref tuning) = profile.extraction {
                        apply_extraction_tuning(tuning, &mut options);
                    }

                    // Store the selected profile for later field extraction
                    // We'll extract fields after the main extraction
                    // For now, just log the match reasons
                    for reason in match_result.reasons.iter().take(5) {
                        eprintln!("  - {}", reason);
                    }
                } else {
                    eprintln!("Document type: unknown (confidence: below threshold)");
                    eprintln!("Proceeding with default extraction options.");
                }
            } else {
                eprintln!(
                    "Warning: Classification failed. Proceeding with default extraction options."
                );
            }
        } else {
            eprintln!(
                "Warning: No profiles available. Proceeding with default extraction options."
            );
        }
    }

    // Handle --profile flag: load and apply specific profile
    #[cfg(feature = "profiles")]
    if let Some(ref profile_name_or_path) = profile {
        use pdftract_core::profiles::{
            load_extraction_profiles, apply_extraction_tuning,
        };

        eprintln!("Applying profile: {}", profile_name_or_path);

        let profiles = load_extraction_profiles(&[]).unwrap_or_default();

        // Find the profile by name or load from path
        let profile = if std::path::PathBuf::from(profile_name_or_path).exists() {
            // Load from file path
            use pdftract_core::profiles::load_profile_file;
            match load_profile_file(&std::path::PathBuf::from(profile_name_or_path)) {
                Ok(p) => Some(p),
                Err(e) => {
                    eprintln!("Error loading profile: {}", e);
                    std::process::exit(1);
                }
            }
        } else {
            // Find by name
            profiles.iter()
                .find(|p| p.profile.name == *profile_name_or_path)
                .map(|p| p.profile.clone())
        };

        if let Some(p) = profile {
            eprintln!("Loaded profile: {}", p.name);
            if let Some(ref tuning) = p.extraction {
                apply_extraction_tuning(tuning, &mut options);
            }
        } else {
            eprintln!("Error: Profile '{}' not found", profile_name_or_path);
            std::process::exit(1);
        }
    }

    #[cfg(not(feature = "profiles"))]
    if auto {
        eprintln!("Warning: --auto flag requires the 'profiles' feature to be enabled.");
        eprintln!("Build pdftract with: --features profiles");
        eprintln!("Proceeding with default extraction options.");
    }

    #[cfg(not(feature = "profiles"))]
    if profile.is_some() {
        eprintln!("Warning: --profile flag requires the 'profiles' feature to be enabled.");
        eprintln!("Build pdftract with: --features profiles");
        eprintln!("Proceeding with default extraction options.");
    }

    // Set markdown anchors option
    options.markdown_anchors = md_anchors;
    if md_anchors {
        eprintln!("Markdown anchors enabled");
    }

    // Set markdown page breaks option
    options.markdown_no_page_breaks = md_no_page_breaks;
    if md_no_page_breaks {
        eprintln!("Markdown page breaks disabled (--md-no-page-breaks)");
    }

    // Set OCR language if specified
    if !ocr_language.is_empty() {
        options.ocr_language = ocr_language;
        eprintln!("OCR languages: {}", options.ocr_language.join("+"));
    } else if ocr {
        // OCR enabled but no language specified, use default (eng)
        eprintln!("OCR enabled with default language: eng");
    }

    // Create cache directory if specified
    let cache_dir_ref = if let Some(ref dir) = cache_dir {
        if !no_cache {
            if !dir.exists() {
                fs::create_dir_all(dir).context(format!(
                    "Failed to create cache directory: {}",
                    dir.display()
                ))?;
            }
            // Initialize cache index if it doesn't exist
            if cache::layout::index_path(dir).exists() {
                Some(dir.as_path())
            } else {
                // Create initial index
                let _ = cache::layout::save_index(dir, &cache::layout::CacheIndex::default());
                Some(dir.as_path())
            }
        } else {
            None
        }
    } else {
        None
    };

    // Parse cache size
    let cache_size_bytes = if cache_dir_ref.is_some() {
        Some(parse_size(cache_size)?)
    } else {
        None
    };

    // Perform extraction (with different paths for URLs vs local files)
    let (mut result, cache_status, cache_age) = if is_url {
        // Remote extraction path
        #[cfg(not(feature = "remote"))]
        {
            eprintln!("Error: Remote sources require the 'remote' feature to be enabled");
            eprintln!("Build pdftract with: --features remote");
            std::process::exit(2);
        }

        #[cfg(feature = "remote")]
        {
            use pdftract_core::source::{HttpRangeSource, open_source};

            // Combine custom headers with URL credentials
            let mut headers_vec: Vec<(String, String)> = custom_headers
                .into_iter()
                .map(|(k, v)| (k, v))
                .collect();

            // If URL has credentials, ureq will automatically add Authorization header
            // We just pass the URL with credentials to HttpRangeSource
            let extraction_url = if let Some(ref parsed) = parsed_url {
                // If credentials were present, use the original URL (with credentials stripped)
                // ureq will handle the basic auth from the URL
                parsed.url.clone()
            } else {
                url_for_source.clone()
            };

            // Add custom headers to the URL
            // Note: ureq automatically handles basic auth when credentials are in the URL
            let source = HttpRangeSource::with_headers(&extraction_url, headers_vec)
                .context("Failed to open remote PDF source")?;

            use pdftract_core::extract::{ExtractionSource, extract_pdf_from_source};
            let extraction_source = ExtractionSource::Remote(Box::new(source));

            let result = extract_pdf_from_source(extraction_source, &options)
                .context("Failed to extract PDF from remote source")?;

            (result, "skipped".to_string(), None) // Cache not applicable for remote
        }
    } else {
        // Local file extraction path (with cache)
        cache::extract_with_cache(&input, &options, cache_dir_ref, no_cache, cache_size_bytes)
            .context("Failed to extract PDF")?
    };

    // Set cache status metadata
    result.metadata.cache_status = Some(cache_status);
    result.metadata.cache_age_seconds = cache_age;

    // Extract profile fields if --auto or --profile was used
    #[cfg(feature = "profiles")]
    {
        use pdftract_core::profiles::{
            load_extraction_profiles, apply_profile_to_metadata,
        };

        let profile_to_apply = if auto {
            // Re-run classification to get the selected profile
            let profiles = load_extraction_profiles(&[]).unwrap_or_default();
            let page_data: Vec<(Vec<_>, Vec<_>)> = result
                .pages
                .iter()
                .map(|p| (p.blocks.clone(), p.spans.clone()))
                .collect();
            let has_signature_field = !result.signatures.is_empty();
            let has_form_field = !result.form_fields.is_empty();

            use pdftract_core::profiles::classify_and_select_profile;
            classify_and_select_profile(
                &profiles.iter().map(|p| p.profile.clone()).collect::<Vec<_>>(),
                &page_data,
                has_signature_field,
                has_form_field,
            ).map(|(p, _)| p)
        } else if profile.is_some() {
            // Load the specified profile
            let profile_name_or_path = profile.as_ref().unwrap();
            let profiles = load_extraction_profiles(&[]).unwrap_or_default();

            if std::path::PathBuf::from(profile_name_or_path).exists() {
                use pdftract_core::profiles::load_profile_file;
                load_profile_file(&std::path::PathBuf::from(profile_name_or_path)).ok()
            } else {
                profiles.iter()
                    .find(|p| p.profile.name == *profile_name_or_path)
                    .map(|p| p.profile.clone())
            }
        } else {
            None
        };

        // Apply profile to metadata
        if let Some(p) = profile_to_apply {
            let (name, version, fields) = apply_profile_to_metadata(&p, &result.pages);
            // Update the result's metadata with profile information
            result.metadata.profile_name = Some(name);
            result.metadata.profile_version = Some(version);
            result.metadata.profile_fields = fields;
        }
    }

    // Write each output to its destination
    for spec in &output_specs {
        match spec.dest {
            output::Destination::Stdout => {
                // Write to stdout
                write_output(&result, &options, spec.format, &mut std::io::stdout())?;
            }
            output::Destination::File(ref path) => {
                // Create atomic file writer for file output
                let mut writer = AtomicFileWriter::create(path)
                    .context(format!("Failed to create output file writer: {}", path.display()))?;
                write_output(&result, &options, spec.format, &mut writer)?;
                writer.commit().context(format!("Failed to commit output file: {}", path.display()))?;
            }
        }
    }

    Ok(())
}

/// Write output in the specified format to the given writer.
fn write_output<W: std::io::Write>(
    result: &pdftract_core::ExtractionResult,
    options: &ExtractionOptions,
    format: output::Format,
    writer: &mut W,
) -> Result<()> {
    use std::io::Write;

    match format {
        output::Format::Json => {
            let json_output = result_to_json(result);
            let json_str = serde_json::to_string_pretty(&json_output)?;
            writeln!(writer, "{}", json_str)?;
        }
        output::Format::Text => {
            // Plain text output: block-level serialization with form feeds between pages
            // Phase 4.6: serialize blocks in reading order, join with \n\n, pages with \f
            let text_options = TextOptions {
                include_headers_footers: options.output.include_headers || options.output.include_footers,
                include_invisible_text: options.output.include_invisible,
                include_watermarks: options.output.include_watermarks,
            };

            // Build pages array for document-level serialization
            let pages: Vec<(&[pdftract_core::schema::BlockJson], &[pdftract_core::schema::SpanJson])> = result.pages
                .iter()
                .map(|p| (&p.blocks[..], &p.spans[..]))
                .collect();

            let text = serialize_document_text(&pages, &text_options);
            write!(writer, "{}", text)?;
        }
        output::Format::Markdown => {
            // Markdown output: simple conversion with optional anchors
            let include_anchors = options.markdown_anchors;
            // Use the --md-no-page-breaks flag to control page break emission
            let include_page_breaks = !options.markdown_no_page_breaks; // Add --- between pages

            for (page_idx, page) in result.pages.iter().enumerate() {
                let is_last_page = page_idx == result.pages.len() - 1;
                let include_break = include_page_breaks && !is_last_page;

                // Filter links to only those belonging to this page
                let page_links: Vec<_> = result.links.iter()
                    .filter(|link| link.page_index == page_idx)
                    .cloned()
                    .collect();

                // Use markdown module with inline link support (Phase 6.5.5b)
                let md_options = MarkdownOptions {
                    include_headers_footers: options.output.include_headers || options.output.include_footers,
                    include_watermarks: options.output.include_watermarks,
                    include_page_breaks: include_break,
                };
                // Use page_to_markdown_with_links_and_footnotes for footnote support
                // (Phase 7 footnote detection not yet implemented, so pass None for footnotes)
                let md = page_to_markdown_with_links_and_footnotes(
                    &page.blocks,
                    &page.spans,
                    &page.tables,
                    &page_links,
                    page.index,
                    include_anchors,
                    &md_options,
                    None, // No footnotes data until Phase 7 is implemented
                );
                write!(writer, "{}", md)?;
            }

            // Emit signatures footer if any signatures exist
            if !result.signatures.is_empty() {
                writeln!(writer, "\n## Signatures\n")?;
                for sig in &result.signatures {
                    writeln!(writer, "- **{}**: {}", sig.field_name, sig.signer_name)?;
                    if let Some(date) = &sig.signing_date {
                        writeln!(writer, "  - Date: {}", date)?;
                    }
                    if let Some(reason) = &sig.reason {
                        writeln!(writer, "  - Reason: {}", reason)?;
                    }
                    if let Some(location) = &sig.location {
                        writeln!(writer, "  - Location: {}", location)?;
                    }
                    if let Some(sub_filter) = &sig.sub_filter {
                        writeln!(writer, "  - Format: {}", sub_filter)?;
                    }
                    writeln!(writer, "  - Validation Status: {}", sig.validation_status)?;
                }
            }
        }
        output::Format::Ndjson => {
            // NDJSON output: emit one line per block with spans
            for page in &result.pages {
                for (block_idx, block) in page.blocks.iter().enumerate() {
                    let ndjson_record = serde_json::json!({
                        "page": page.index,
                        "block_index": block_idx,
                        "kind": block.kind,
                        "bbox": block.bbox,
                        "spans": block.spans.iter().filter_map(|&span_idx| {
                            page.spans.get(span_idx).map(|span| {
                                serde_json::json!({
                                    "text": span.text,
                                    "font": span.font,
                                    "size": span.size,
                                    "bbox": span.bbox,
                                })
                            })
                        }).collect::<Vec<_>>(),
                    });
                    writeln!(writer, "{}", ndjson_record)?;
                }
            }
        }
    }

    Ok(())
}

fn cmd_classify(
    input: PathBuf,
    password_stdin: bool,
    password: Option<String>,
    profiles_dir: Option<PathBuf>,
    pretty: bool,
    top_k: usize,
    exit_on_unknown: bool,
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

    // Run classification
    let args = classify::ClassifyArgs {
        input,
        profiles_dir,
        pretty,
        top_k,
        exit_on_unknown,
    };

    let output = classify::run_classify(args)?;

    // Print JSON output
    let json_str = classify::format_json(&output, pretty);
    println!("{}", json_str);

    Ok(())
}

fn cmd_list_diagnostics() -> Result<()> {
    println!("pdftract Diagnostic Codes");
    println!();
    println!("This catalog lists all diagnostic codes emitted during PDF parsing and extraction.");
    println!("Each diagnostic includes a severity level, recoverable flag, phase origin, and suggested action.");
    println!();

    // Group by category
    let mut categories: std::collections::HashMap<&str, Vec<&DiagInfo>> =
        std::collections::HashMap::new();
    for info in DIAGNOSTIC_CATALOG {
        categories.entry(info.category).or_default().push(info);
    }

    // Define category order
    let category_order = vec![
        "STRUCT",
        "XREF",
        "STREAM",
        "ENCRYPTION",
        "PAGE",
        "FONT",
        "OCR",
        "REMOTE",
        "GSTATE",
        "LAYOUT",
        "MCP",
        "CACHE",
    ];

    for category in category_order {
        if let Some(infos) = categories.get(category) {
            println!("=== {}_* codes ===", category);
            println!();

            for info in infos {
                println!("{} ({})", info.code, info.severity);
                println!("  Phase: {}", info.phase);
                println!(
                    "  Recoverable: {}",
                    if info.recoverable { "Yes" } else { "No" }
                );
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
    println!(
        "Recoverable: {}",
        if info.recoverable { "Yes" } else { "No" }
    );
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
            println!(
                "  PDF is encrypted and no password was supplied or algorithm is unsupported."
            );
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
            println!(
                "  A glyph has no entry in /ToUnicode CMap, AGL, fingerprint, or shape match."
            );
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
        DiagCode::CmArgCount => {
            println!("  Invalid argument count for cm operator");
            println!("  The cm operator requires exactly 6 numeric arguments.");
        }
        DiagCode::CmDegenerate => {
            println!("  Degenerate matrix");
            println!("  The cm operator received a degenerate matrix (det=0 or NaN); clamped to identity.");
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
        DiagCode::StructInvalidType => {
            println!("  Invalid object type");
            println!("  An object is not the expected type (e.g., expecting a stream but finding a dictionary).");
        }
        DiagCode::StructIncompleteCoverage => {
            println!("  StructTree coverage below threshold");
            println!("  StructTree coverage is below 80% with /Suspects true, triggering XY-cut fallback.");
        }
        DiagCode::FontParseFailed => {
            println!("  Font parsing failed");
            println!("  A font file could not be parsed.");
        }
        DiagCode::FontUnsupported => {
            println!("  Unsupported font type");
            println!("  A font uses an unsupported format or encoding.");
        }
        DiagCode::FontCidtogidmapTruncated => {
            println!("  CIDToGIDMap truncated");
            println!("  A CIDToGIDMap stream is incomplete.");
        }
        _ => {
            println!("  (See diagnostic code)");
        }
    }

    println!();
    println!("Suggested Action: {}", info.suggested_action);
    println!();
    println!("Phase Origin: {}", info.phase);

    Ok(())
}

fn cmd_compare(
    actual: PathBuf,
    expected: PathBuf,
    tolerances: Option<PathBuf>,
    format: &str,
) -> Result<()> {
    let actual_json = fs::read_to_string(&actual)
        .context(format!("Failed to read actual results from {:?}", actual))?;
    let actual_val: serde_json::Value =
        serde_json::from_str(&actual_json).context("Failed to parse actual results as JSON")?;

    let expected_json = fs::read_to_string(&expected).context(format!(
        "Failed to read expected results from {:?}",
        expected
    ))?;
    let expected_val: serde_json::Value =
        serde_json::from_str(&expected_json).context("Failed to parse expected results as JSON")?;

    let tolerances_val = if let Some(tol_path) = tolerances {
        let tol_json = fs::read_to_string(&tol_path)
            .context(format!("Failed to read tolerances from {:?}", tol_path))?;
        Some(
            serde_json::from_str::<serde_json::Value>(&tol_json)
                .context("Failed to parse tolerances as JSON")?,
        )
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

    let suite_json =
        fs::read_to_string(&suite).context(format!("Failed to read suite from {:?}", suite))?;
    let suite_val: serde_json::Value =
        serde_json::from_str(&suite_json).context("Failed to parse suite as JSON")?;

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

fn cmd_cache(command: CacheCommands) -> Result<()> {
    match command {
        CacheCommands::Stats { dir, json } => {
            let stats = cache_cmd::compute_stats(&dir)?;
            if json {
                cache_cmd::display_stats_json(&stats)?;
            } else {
                cache_cmd::display_stats(&stats);
            }
        }
        CacheCommands::Clear { dir, yes } => {
            cache_cmd::clear_cache(&dir, yes)?;
        }
        CacheCommands::Purge {
            dir,
            older_than,
            version,
        } => {
            if older_than.is_none() && version.is_none() {
                eprintln!("Error: --older-than or --version is required for purge");
                eprintln!("Usage: pdftract cache purge DIR --older-than 30d");
                eprintln!("       pdftract cache purge DIR --version '<1.0.0'");
                std::process::exit(2);
            }
            if let Some(duration) = older_than {
                cache_cmd::purge_cache_older_than(&dir, &duration)?;
            }
            if let Some(constraint) = version {
                cache_cmd::purge_cache_version(&dir, &constraint)?;
            }
        }
    }
    Ok(())
}

fn cmd_profiles(command: ProfilesCommands) -> Result<()> {
    use profiles_cmd::{ProfilesArgs, ProfilesCommand};

    // Convert ProfilesCommands to profiles_cmd::ProfilesCommand
    let profiles_command = match command {
        ProfilesCommands::List => ProfilesCommand::List,
        ProfilesCommands::Show { name_or_path } => ProfilesCommand::Show { name_or_path },
        ProfilesCommands::Export { name } => ProfilesCommand::Export { name },
        ProfilesCommands::Install { path } => ProfilesCommand::Install { path },
        ProfilesCommands::Validate { path } => ProfilesCommand::Validate { path },
    };

    let args = ProfilesArgs {
        command: profiles_command,
    };

    profiles_cmd::run_profiles(args)
}

fn cmd_serve(
    bind: String,
    cache_dir: Option<PathBuf>,
    cache_size: &str,
    no_cache: bool,
    max_upload_mb: usize,
    max_decompress_gb: usize,
    audit_log: Option<PathBuf>,
    trust_forwarded_for: bool,
) -> Result<()> {
    // Warn if binding to 0.0.0.0 (no auth, exposed to all interfaces)
    if bind.starts_with("0.0.0.0") || bind.starts_with("[::]") {
        eprintln!("*** WARNING: Binding to {} exposes pdftract serve on ALL interfaces.", bind);
        eprintln!("*** pdftract serve has NO BUILT-IN AUTHENTICATION.");
        eprintln!("*** Deploy behind a reverse proxy (nginx, Traefik, Caddy) for production use.");
        eprintln!();
    }

    // Validate hard cap for max_upload_mb (4 GiB)
    const MAX_UPLOAD_MB_HARD_CAP: usize = 4096;
    if max_upload_mb > MAX_UPLOAD_MB_HARD_CAP {
        anyhow::bail!(
            "--max-upload-mb value {} exceeds hard cap of {} MB (4 GiB). \
             This limit prevents integer overflow when computing the byte limit.",
            max_upload_mb,
            MAX_UPLOAD_MB_HARD_CAP
        );
    }

    // Parse cache size
    let cache_size_bytes = parse_size(cache_size)?;

    // Create cache directory if specified
    if let Some(ref dir) = cache_dir {
        if !dir.exists() {
            fs::create_dir_all(dir).context(format!(
                "Failed to create cache directory: {}",
                dir.display()
            ))?;
        }
    }

    // Run the HTTP server
    tokio::runtime::Runtime::new()
        .context("Failed to create tokio runtime")?
        .block_on(serve::run(
            bind,
            cache_dir,
            cache_size_bytes,
            no_cache,
            max_upload_mb,
            max_decompress_gb,
            audit_log,
            trust_forwarded_for,
        ))
}

/// Wrapper for the inspect subcommand.
///
/// Creates a tokio runtime and runs the async inspect::run function.
fn cmd_inspect(args: inspect::InspectArgs) -> Result<()> {
    tokio::runtime::Runtime::new()
        .context("Failed to create tokio runtime")?
        .block_on(inspect::run(args))
}

/// Parse a size string like "1 GiB", "500 MiB", "2 GiB" into bytes.
fn parse_size(size_str: &str) -> Result<u64> {
    let s = size_str.trim().to_lowercase();
    let multiplier = if s.ends_with("gib") || s.ends_with("gb") || s.ends_with("g") {
        1024 * 1024 * 1024
    } else if s.ends_with("mib") || s.ends_with("mb") || s.ends_with("m") {
        1024 * 1024
    } else if s.ends_with("kib") || s.ends_with("kb") || s.ends_with("k") {
        1024
    } else {
        1 // bytes
    };

    let num_str = s
        .trim_end_matches("gib")
        .trim_end_matches("gb")
        .trim_end_matches("g")
        .trim_end_matches("mib")
        .trim_end_matches("mb")
        .trim_end_matches("m")
        .trim_end_matches("kib")
        .trim_end_matches("kb")
        .trim_end_matches("k")
        .trim()
        .replace('_', "");

    let num: f64 = num_str
        .parse()
        .context(format!("Invalid size value: {}", size_str))?;

    Ok((num * multiplier as f64) as u64)
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
            if let Some(min_len) = exp
                .get("min_length")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize)
            {
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
        _ => {
            return CompareResult::Fail {
                reason: "expected value is not a number".to_string(),
            }
        }
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
