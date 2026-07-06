//! Shared CLI definitions for pdftract.
//!
//! This module contains the clap derive structs that define the CLI interface.
//! These are used by both main.rs (for the actual CLI) and lib.rs (for documentation).

use clap::{ArgAction, Parser, Subcommand};
use std::path::PathBuf;

// Language type is re-exported from codegen module (declared in main.rs/lib.rs)
pub use crate::codegen::Language;

// Import inspect and verify_receipt modules for use in Commands enum
pub use crate::inspect::InspectArgs;
pub use crate::verify_receipt::VerifyReceiptCommand;

// Import grep module for use in Commands enum (feature-gated)
#[cfg(feature = "grep")]
pub use crate::grep::GrepArgs;

#[derive(Parser)]
#[command(name = "pdftract")]
#[command(about = "pdftract CLI - PDF extraction and conformance testing", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
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
        #[arg(short = 'k', long, default_value = "pdftract")]
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
    Grep(GrepArgs),
    /// Inspect a PDF file in a local web browser with debugging overlays
    Inspect(InspectArgs),
    /// Verify a receipt against a PDF file
    VerifyReceipt(VerifyReceiptCommand),
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
pub enum SdkCommands {
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
pub enum CacheCommands {
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
pub enum ProfilesCommands {
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
