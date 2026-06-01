//! Generate CLI Reference documentation using clap-markdown.
//!
//! This binary generates the canonical CLI Reference documentation for pdftract,
//! which is checked into the repository at docs/user-docs/src/cli-reference.md.
//!
//! Usage: cargo run --manifest-path=xtask/Cargo.toml --bin gen_cli_reference

use std::fs;
use std::path::PathBuf;

const AUTOGEN_END_MARKER: &str = "<!-- AUTOGEN END -->";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Find the workspace root
    let workspace_root = find_workspace_root();

    // Generate the CLI reference markdown
    let generated_markdown = generate_cli_reference();

    // Write to docs/user-docs/src/cli-reference.md
    let cli_ref_path = workspace_root.join("docs/user-docs/src/cli-reference.md");

    // Create the directory if it doesn't exist
    if let Some(parent) = cli_ref_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Read existing file to preserve hand-curated content
    let hand_curated_content = if cli_ref_path.exists() {
        let existing = fs::read_to_string(&cli_ref_path)?;
        if let Some(idx) = existing.find(AUTOGEN_END_MARKER) {
            Some(existing[idx + AUTOGEN_END_MARKER.len()..].to_string())
        } else {
            None
        }
    } else {
        None
    };

    // Build the final output
    let mut final_output = String::new();

    // Add autogen notice at the top
    final_output.push_str("> This page is auto-generated from the clap command tree.\n");
    final_output.push_str("> Run `cargo run --manifest-path=xtask/Cargo.toml --bin gen_cli_reference` to regenerate.\n\n");
    final_output.push_str(generated_markdown.trim_end());
    final_output.push_str("\n\n");
    final_output.push_str(AUTOGEN_END_MARKER);
    final_output.push_str("\n\n");

    // Add hand-curated content if it exists
    if let Some(curated) = hand_curated_content {
        final_output.push_str(curated.trim_start());
        println!("Preserved hand-curated content after AUTOGEN END marker.");
    } else {
        // Add a default hand-curated section header
        final_output.push_str("## Hand-Curated Content\n\n");
        final_output.push_str("> **Note:** Any content added after this marker will be preserved\n");
        final_output.push_str("> when the CLI reference is regenerated. This section is for\n");
        final_output.push_str("> additional context that doesn't fit in the auto-generated sections.\n\n");
        final_output.push_str("### Common Patterns\n\n");
        final_output.push_str("#### Basic Extraction\n\n");
        final_output.push_str("```bash\npdftract extract document.pdf\n```\n\n");
        final_output.push_str("#### JSON Output\n\n");
        final_output.push_str("```bash\npdftract extract --json output.json document.pdf\n```\n\n");
        final_output.push_str("#### Markdown with Anchors\n\n");
        final_output.push_str("```bash\npdftract extract --md-anchors --md output.md document.pdf\n```\n\n");
        final_output.push_str("### Exit Codes\n\n");
        final_output.push_str("- `0`: Success\n");
        final_output.push_str("- `1`: General error (extraction failed, file not found, etc.)\n");
        final_output.push_str("- `2`: Usage error (invalid arguments, conflicting flags)\n");
        final_output.push_str("- `3`: Decryption error (wrong or missing password)\n");
    }

    fs::write(&cli_ref_path, final_output)?;

    println!("Generated CLI reference at: {}", cli_ref_path.display());

    Ok(())
}

/// Find the workspace root by searching for Cargo.toml
fn find_workspace_root() -> PathBuf {
    let mut current = std::env::current_dir().unwrap();

    // If we're in the xtask directory, go to parent
    if current.ends_with("xtask") {
        current = current.parent().unwrap().to_path_buf();
    }

    // Search upward for Cargo.toml with workspace members
    loop {
        let cargo_toml = current.join("Cargo.toml");
        if cargo_toml.exists() {
            let content = fs::read_to_string(&cargo_toml).unwrap_or_default();
            if content.contains("[workspace]") {
                return current;
            }
        }

        // Move to parent directory
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => panic!("Could not find workspace root"),
        }
    }
}

/// Generate CLI reference markdown using clap-markdown.
///
/// This function creates a minimal clap Command that matches the pdftract CLI
/// structure and generates comprehensive markdown documentation.
fn generate_cli_reference() -> String {
    use clap::{Command, Arg, ArgAction, ValueHint};

    let mut cmd = Command::new("pdftract")
        .about("pdftract CLI - PDF extraction and conformance testing")
        .long_about(
            "pdftract is a command-line tool for extracting text and structure from PDF files.\n\
             It supports JSON, Markdown, plain text, and NDJSON output formats, with\n\
             advanced features like OCR, document classification, and conformance testing."
        )
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::new("help")
                .short('h')
                .long("help")
                .action(ArgAction::Help)
                .global(true)
                .help("Print help information")
        )
        .arg(
            Arg::new("version")
                .short('V')
                .long("version")
                .action(ArgAction::Version)
                .global(true)
                .help("Print version information")
        );

    // extract subcommand
    cmd = cmd.subcommand(
        Command::new("extract")
            .about("Extract text and structure from a PDF file")
            .long_about(
                "Extract content from PDF files in multiple formats.\n\
                 Supports local files, remote URLs, and stdin input."
            )
            .arg(
                Arg::new("input")
                    .help("Path to the PDF file (use '-' for stdin)")
                    .value_hint(ValueHint::FilePath)
                    .required(true)
            )
            .arg(
                Arg::new("password_stdin")
                    .long("password-stdin")
                    .help("Read password from stdin (one line, terminated by newline)")
                    .conflicts_with("password")
            )
            .arg(
                Arg::new("password")
                    .long("password")
                    .value_name("PASSWORD")
                    .help("PDF password (INSECURE: rejected unless PDFTRACT_INSECURE_CLI_PASSWORD=1)")
                    .conflicts_with("password_stdin")
            )
            .arg(
                Arg::new("header")
                    .long("header")
                    .value_name("HEADER:VALUE")
                    .action(ArgAction::Append)
                    .help("Custom HTTP headers for remote sources (repeatable; format: HEADER:VALUE)")
            )
            .arg(
                Arg::new("pages")
                    .long("pages")
                    .value_name("RANGE")
                    .help("Page range to extract (1-based, comma-separated: 1-5,7,12-)")
            )
            .arg(
                Arg::new("json")
                    .long("json")
                    .value_name("PATH")
                    .action(ArgAction::Append)
                    .help("Output JSON to PATH (use '-' for stdout)")
            )
            .arg(
                Arg::new("md")
                    .long("md")
                    .value_name("PATH")
                    .action(ArgAction::Append)
                    .help("Output Markdown to PATH (use '-' for stdout)")
            )
            .arg(
                Arg::new("text")
                    .long("text")
                    .value_name("PATH")
                    .action(ArgAction::Append)
                    .help("Output plain text to PATH (use '-' for stdout)")
            )
            .arg(
                Arg::new("ndjson")
                    .long("ndjson")
                    .action(ArgAction::SetTrue)
                    .help("Output NDJSON to stdout (mutually exclusive with other formats)")
                    .conflicts_with_all(["json", "md", "text", "format"])
            )
            .arg(
                Arg::new("format")
                    .long("format")
                    .value_name("FORMATS")
                    .value_delimiter(',')
                    .action(ArgAction::Append)
                    .help("Output formats (comma-separated: json,markdown,text,ndjson)")
            )
            .arg(
                Arg::new("output")
                    .short('o')
                    .long("output")
                    .value_name("BASE")
                    .help("Base path for auto-named outputs (used with --format)")
            )
            .arg(
                Arg::new("receipts")
                    .long("receipts")
                    .value_name("MODE")
                    .default_value("off")
                    .value_parser(["off", "lite", "svg"])
                    .help("Receipt mode: off (default), lite, or svg")
            )
            .arg(
                Arg::new("ocr")
                    .long("ocr")
                    .action(ArgAction::SetTrue)
                    .help("Enable OCR for scanned pages (requires 'ocr' feature)")
            )
            .arg(
                Arg::new("ocr_language")
                    .long("ocr-language")
                    .value_name("LANGS")
                    .value_delimiter(',')
                    .action(ArgAction::Append)
                    .help("OCR language codes (comma-separated, e.g., 'eng,fra,deu')")
            )
            .arg(
                Arg::new("cache_dir")
                    .long("cache-dir")
                    .value_name("DIR")
                    .value_hint(ValueHint::DirPath)
                    .help("Enable cache at this directory (creates if absent)")
            )
            .arg(
                Arg::new("cache_size")
                    .long("cache-size")
                    .value_name("SIZE")
                    .default_value("1 GiB")
                    .help("Set cache size limit (default 1 GiB; accepts KiB, MiB, GiB suffixes)")
            )
            .arg(
                Arg::new("no_cache")
                    .long("no-cache")
                    .action(ArgAction::SetTrue)
                    .help("Disable cache for this extraction (even if --cache-dir is set)")
            )
            .arg(
                Arg::new("md_anchors")
                    .long("md-anchors")
                    .action(ArgAction::SetTrue)
                    .help("Emit HTML comment anchors before each block in Markdown output")
            )
            .arg(
                Arg::new("auto")
                    .long("auto")
                    .action(ArgAction::SetTrue)
                    .help("Auto-detect document type and apply appropriate profile")
            )
            .arg(
                Arg::new("profile")
                    .long("profile")
                    .value_name("NAME|PATH")
                    .help("Force-apply a specific profile (by name or YAML file path)")
            )
            .arg(
                Arg::new("include_headers")
                    .long("include-headers")
                    .action(ArgAction::SetTrue)
                    .help("Include header blocks in output")
            )
            .arg(
                Arg::new("include_footers")
                    .long("include-footers")
                    .action(ArgAction::SetTrue)
                    .help("Include footer blocks in output")
            )
            .arg(
                Arg::new("include_headers_footers")
                    .long("include-headers-footers")
                    .action(ArgAction::SetTrue)
                    .help("Include both header and footer blocks in output")
            )
            .arg(
                Arg::new("include_invisible_text")
                    .long("include-invisible-text")
                    .action(ArgAction::SetTrue)
                    .help("Include invisible text spans in output (rendering_mode == 3)")
            )
            .arg(
                Arg::new("include_hidden_layers")
                    .long("include-hidden-layers")
                    .action(ArgAction::SetTrue)
                    .help("Include hidden-layer text spans in output (OCG-controlled)")
            )
            .arg(
                Arg::new("include_watermarks")
                    .long("include-watermarks")
                    .action(ArgAction::SetTrue)
                    .help("Include watermark blocks in output (no-op until Phase 7)")
            )
    );

    // classify subcommand
    cmd = cmd.subcommand(
        Command::new("classify")
            .about("Classify document type")
            .long_about(
                "Runs metadata + signal extraction to classify document type.\n\
                 Not full text extraction - suitable for quick categorization."
            )
            .arg(
                Arg::new("input")
                    .help("Path to the PDF file")
                    .value_hint(ValueHint::FilePath)
                    .required(true)
            )
            .arg(
                Arg::new("password_stdin")
                    .long("password-stdin")
                    .help("Read password from stdin (one line, terminated by newline)")
                    .conflicts_with("password")
            )
            .arg(
                Arg::new("password")
                    .long("password")
                    .value_name("PASSWORD")
                    .help("PDF password (INSECURE: rejected unless PDFTRACT_INSECURE_CLI_PASSWORD=1)")
                    .conflicts_with("password_stdin")
            )
            .arg(
                Arg::new("profiles")
                    .long("profiles")
                    .value_name("DIR")
                    .value_hint(ValueHint::DirPath)
                    .help("Directory containing custom profile YAML files")
            )
            .arg(
                Arg::new("pretty")
                    .long("pretty")
                    .action(ArgAction::SetTrue)
                    .help("Pretty-print JSON output")
            )
            .arg(
                Arg::new("top_k")
                    .long("top-k")
                    .value_name("N")
                    .default_value("0")
                    .help("Number of top reasons to include (default: all)")
            )
            .arg(
                Arg::new("exit_on_unknown")
                    .long("exit-on-unknown")
                    .action(ArgAction::SetTrue)
                    .help("Exit with code 1 if document type is unknown")
            )
    );

    // grep subcommand
    cmd = cmd.subcommand(
        Command::new("grep")
            .about("Search for text patterns in PDF files")
            .long_about(
                "Search for text patterns with bounding-box results.\n\
                 Requires the 'grep' feature flag."
            )
            .arg(
                Arg::new("pattern")
                    .help("Regular expression pattern to search for")
                    .required(true)
            )
            .arg(
                Arg::new("paths")
                    .help("PDF files or directories to search")
                    .value_hint(ValueHint::FilePath)
                    .action(ArgAction::Append)
                    .required(true)
            )
            .arg(
                Arg::new("context")
                    .short('C')
                    .long("context")
                    .value_name("LINES")
                    .default_value("0")
                    .help("Number of context lines to show")
            )
            .arg(
                Arg::new("ignore_case")
                    .short('i')
                    .long("ignore-case")
                    .action(ArgAction::SetTrue)
                    .help("Case-insensitive search")
            )
            .arg(
                Arg::new("json")
                    .long("json")
                    .action(ArgAction::SetTrue)
                    .help("Output results as JSON")
            )
    );

    // inspect subcommand
    cmd = cmd.subcommand(
        Command::new("inspect")
            .about("Inspect a PDF file in a local web browser")
            .long_about(
                "Launch a local web server with debugging overlays for PDF inspection.\n\
                 Provides visual feedback on extraction accuracy and layout analysis.\n\
                 Requires the 'inspect' feature flag."
            )
            .arg(
                Arg::new("input")
                    .help("Path to the PDF file")
                    .value_hint(ValueHint::FilePath)
                    .required(true)
            )
            .arg(
                Arg::new("bind")
                    .short('b')
                    .long("bind")
                    .value_name("ADDR")
                    .default_value("127.0.0.1:0")
                    .help("Bind address for the inspector server (use 0.0.0.0:0 for accessibility from other devices)")
            )
            .arg(
                Arg::new("password")
                    .long("password")
                    .value_name("PASSWORD")
                    .help("PDF password (INSECURE: rejected unless PDFTRACT_INSECURE_CLI_PASSWORD=1)")
            )
            .arg(
                Arg::new("ocr")
                    .long("ocr")
                    .action(ArgAction::SetTrue)
                    .help("Enable OCR for scanned pages (requires 'ocr' feature)")
            )
            .arg(
                Arg::new("no_browser")
                    .long("no-browser")
                    .action(ArgAction::SetTrue)
                    .help("Don't automatically open browser")
            )
    );

    // serve subcommand
    cmd = cmd.subcommand(
        Command::new("serve")
            .about("Start the HTTP server for extraction")
            .long_about(
                "Start an HTTP server for PDF extraction via REST API.\n\n\
                 **Security Model:** pdftract serve has no built-in authentication. \
                 Deploy behind a reverse proxy (nginx, Traefik, Caddy) for production use.\n\n\
                 **Endpoints:**\n\
                 - POST /extract - Extract PDF and return JSON with metadata\n\
                 - POST /extract/text - Extract PDF and return plain text\n\
                 - POST /extract/stream - Extract PDF and return streaming NDJSON\n\
                 - GET /health - Health check\n\n\
                 Requires the 'serve' feature flag."
            )
            .arg(
                Arg::new("bind")
                    .short('b')
                    .long("bind")
                    .value_name("ADDR")
                    .default_value("127.0.0.1:8080")
                    .help("Bind address (e.g., \"127.0.0.1:8080\", \"[::1]:9000\", \"0.0.0.0:3000\")")
            )
            .arg(
                Arg::new("cache_dir")
                    .long("cache-dir")
                    .value_name("DIR")
                    .value_hint(ValueHint::DirPath)
                    .help("Enable cache at this directory")
            )
            .arg(
                Arg::new("cache_size")
                    .long("cache-size")
                    .value_name("SIZE")
                    .default_value("1 GiB")
                    .help("Set cache size limit (default 1 GiB; accepts KiB, MiB, GiB suffixes)")
            )
            .arg(
                Arg::new("no_cache")
                    .long("no-cache")
                    .action(ArgAction::SetTrue)
                    .help("Disable cache")
            )
            .arg(
                Arg::new("max_upload_mb")
                    .long("max-upload-mb")
                    .value_name("MB")
                    .default_value("256")
                    .help("Maximum request body size in MB (default: 256, max: 4096)")
            )
            .arg(
                Arg::new("max_decompress_gb")
                    .long("max-decompress-gb")
                    .value_name("GB")
                    .default_value("1")
                    .help("Maximum decompression size in GB (default: 1)")
            )
            .arg(
                Arg::new("audit_log")
                    .long("audit-log")
                    .value_name("FILE")
                    .value_hint(ValueHint::FilePath)
                    .help("Write per-request audit log to FILE (NDJSON; use \"-\" for stdout)")
            )
            .arg(
                Arg::new("trust_forwarded_for")
                    .long("trust-forwarded-for")
                    .action(ArgAction::SetTrue)
                    .help("Trust X-Forwarded-For header for client IP detection (DANGER: enables IP spoofing if not behind a trusted proxy)")
            )
            .arg(
                Arg::new("profile_dir")
                    .long("profile-dir")
                    .value_name("DIR")
                    .value_hint(ValueHint::DirPath)
                    .help("Directory containing custom profile YAML files (repeatable)")
            )
            .arg(
                Arg::new("profile_hot_reload")
                    .long("profile-hot-reload")
                    .action(ArgAction::SetTrue)
                    .help("Enable hot-reload for profiles (re-read directory on every request)")
            )
    );

    // mcp subcommand
    cmd = cmd.subcommand(
        Command::new("mcp")
            .about("Start the MCP (Model Context Protocol) server")
            .long_about(
                "Start an MCP server for AI assistant integration.\n\n\
                 Per ADR-006: stdio and HTTP transports are mutually exclusive.\n\
                 Exactly one transport must be selected per invocation.\n\n\
                 Requires the 'mcp' feature flag."
            )
            .arg(
                Arg::new("stdio")
                    .long("stdio")
                    .action(ArgAction::SetTrue)
                    .help("Use stdio transport (for Claude Desktop, Claude Code, Continue, Cursor)")
                    .conflicts_with("bind")
            )
            .arg(
                Arg::new("bind")
                    .short('b')
                    .long("bind")
                    .value_name("ADDR")
                    .help("Bind address for the MCP server (enables HTTP+SSE transport)")
                    .conflicts_with("stdio")
            )
            .arg(
                Arg::new("auth_token_file")
                    .long("auth-token-file")
                    .value_name("PATH")
                    .value_hint(ValueHint::FilePath)
                    .help("Path to a file containing the bearer token (RECOMMENDED)")
                    .conflicts_with("auth_token")
            )
            .arg(
                Arg::new("auth_token")
                    .long("auth-token")
                    .value_name("TOKEN")
                    .help("Bearer token for authentication (INSECURE: rejected unless PDFTRACT_INSECURE_CLI_TOKEN=1)")
                    .conflicts_with("auth_token_file")
            )
            .arg(
                Arg::new("max_upload_mb")
                    .long("max-upload-mb")
                    .value_name("MB")
                    .default_value("256")
                    .help("Maximum request body size in MB (default: 256)")
            )
            .arg(
                Arg::new("root")
                    .long("root")
                    .value_name("DIR")
                    .value_hint(ValueHint::DirPath)
                    .help("Root directory for local filesystem access (enforces path-traversal protection)")
            )
            .arg(
                Arg::new("audit_log")
                    .long("audit-log")
                    .value_name("FILE")
                    .value_hint(ValueHint::FilePath)
                    .help("Write per-request audit log to FILE (NDJSON; use \"-\" for stdout)")
            )
    );

    // cache subcommand
    let mut cache_cmd = Command::new("cache")
        .about("Manage the extraction cache")
        .long_about(
            "Manage the content-addressed extraction cache.\n\
             Cache entries are stored by PDF hash and version constraint.\n\
             Requires the 'cache' feature flag."
        );

    cache_cmd = cache_cmd.subcommand(
        Command::new("stats")
            .about("Show cache statistics")
            .arg(
                Arg::new("dir")
                    .value_name("DIR")
                    .value_hint(ValueHint::DirPath)
                    .required(true)
                    .help("Path to the cache directory")
            )
            .arg(
                Arg::new("json")
                    .long("json")
                    .action(ArgAction::SetTrue)
                    .help("Output in JSON format")
            )
    );

    cache_cmd = cache_cmd.subcommand(
        Command::new("clear")
            .about("Clear all cache entries")
            .long_about("Clear all cache entries (preserves index.json and sentinel)")
            .arg(
                Arg::new("dir")
                    .value_name("DIR")
                    .value_hint(ValueHint::DirPath)
                    .required(true)
                    .help("Path to the cache directory")
            )
            .arg(
                Arg::new("yes")
                    .short('y')
                    .long("yes")
                    .action(ArgAction::SetTrue)
                    .help("Skip confirmation prompt")
            )
    );

    cache_cmd = cache_cmd.subcommand(
        Command::new("purge")
            .about("Purge old cache entries")
            .arg(
                Arg::new("dir")
                    .value_name("DIR")
                    .value_hint(ValueHint::DirPath)
                    .required(true)
                    .help("Path to the cache directory")
            )
            .arg(
                Arg::new("older_than")
                    .long("older-than")
                    .value_name("DURATION")
                    .help("Delete entries older than this duration (e.g., \"30d\", \"7d\", \"1h\")")
            )
            .arg(
                Arg::new("version")
                    .long("version")
                    .value_name("CONSTRAINT")
                    .help("Delete entries matching this version constraint (e.g., \"<1.0.0\")")
            )
    );

    cmd = cmd.subcommand(cache_cmd);

    // profiles subcommand
    let mut profiles_cmd = Command::new("profiles")
        .about("Manage document type profiles")
        .long_about(
            "Manage document type profiles for classification and extraction tuning.\n\
             Requires the 'profiles' feature flag."
        );

    profiles_cmd = profiles_cmd.subcommand(
        Command::new("list")
            .about("List all available profiles")
    );

    profiles_cmd = profiles_cmd.subcommand(
        Command::new("show")
            .about("Show a profile's YAML content")
            .arg(
                Arg::new("name_or_path")
                    .value_name("NAME|PATH")
                    .required(true)
                    .help("Profile name or path to YAML file")
            )
    );

    profiles_cmd = profiles_cmd.subcommand(
        Command::new("export")
            .about("Export a built-in profile to stdout")
            .arg(
                Arg::new("name")
                    .value_name("NAME")
                    .required(true)
                    .help("Name of the built-in profile to export")
            )
    );

    profiles_cmd = profiles_cmd.subcommand(
        Command::new("install")
            .about("Install a profile to the user config directory")
            .arg(
                Arg::new("path")
                    .value_name("PATH")
                    .value_hint(ValueHint::FilePath)
                    .required(true)
                    .help("Path to the profile YAML file to install")
            )
    );

    profiles_cmd = profiles_cmd.subcommand(
        Command::new("validate")
            .about("Validate a profile file")
            .arg(
                Arg::new("path")
                    .value_name("PATH")
                    .value_hint(ValueHint::FilePath)
                    .required(true)
                    .help("Path to the profile YAML file to validate")
            )
    );

    cmd = cmd.subcommand(profiles_cmd);

    // doctor subcommand
    cmd = cmd.subcommand(
        Command::new("doctor")
            .about("Check environment health and dependencies")
            .long_about(
                "Run environment health checks for pdftract dependencies and configuration.\n\n\
                 Exit code policy:\n\
                 - Exits 0 if no checks FAIL (WARN does not affect exit code)\n\
                 - Exits 1 if any check FAILs\n\
                 - Exits 2 on argument parse errors"
            )
            .arg(
                Arg::new("features")
                    .long("features")
                    .action(ArgAction::SetTrue)
                    .help("Print compiled features and exit")
            )
            .arg(
                Arg::new("json")
                    .long("json")
                    .action(ArgAction::SetTrue)
                    .help("Output results as JSON")
            )
            .arg(
                Arg::new("no_color")
                    .long("no-color")
                    .action(ArgAction::SetTrue)
                    .help("Disable colored output")
            )
            .arg(
                Arg::new("exit_on_fail")
                    .long("exit-on-fail")
                    .action(ArgAction::SetTrue)
                    .help("Explicit form of the default policy (exit 1 if any check FAILs)")
            )
            .arg(
                Arg::new("profile_dir")
                    .long("profile-dir")
                    .value_name("DIR")
                    .value_hint(ValueHint::DirPath)
                    .help("Verify the profile search path includes DIR")
            )
            .arg(
                Arg::new("cache_dir")
                    .long("cache-dir")
                    .value_name("DIR")
                    .value_hint(ValueHint::DirPath)
                    .help("Verify DIR is writable and has sufficient space")
            )
            .arg(
                Arg::new("lang")
                    .long("lang")
                    .value_name("LANGS")
                    .value_delimiter(',')
                    .action(ArgAction::Append)
                    .help("Requested OCR languages (default: eng)")
            )
    );

    // hash subcommand
    cmd = cmd.subcommand(
        Command::new("hash")
            .about("Compute the PDF structural fingerprint")
            .long_about(
                "Compute a structural hash/fingerprint of a PDF file.\n\
                 This hash is based on the PDF's structure (xref, trailers, object\n\
                 locations) rather than content, making it useful for identifying\n\
                 identical documents with different metadata."
            )
            .arg(
                Arg::new("input")
                    .value_name("PATH|URL")
                    .required(true)
                    .help("Path to the PDF file or URL")
            )
            .arg(
                Arg::new("password")
                    .long("password")
                    .value_name("PASSWORD")
                    .help("PDF password (INSECURE: rejected unless PDFTRACT_INSECURE_CLI_PASSWORD=1)")
            )
            .arg(
                Arg::new("header")
                    .long("header")
                    .value_name("HEADER:VALUE")
                    .action(ArgAction::Append)
                    .help("Custom HTTP headers for remote sources (repeatable; format: HEADER:VALUE)")
            )
    );

    // verify-receipt subcommand
    cmd = cmd.subcommand(
        Command::new("verify-receipt")
            .about("Verify a receipt against a PDF file")
            .long_about(
                "Verify a visual citation receipt against the original PDF.\n\
                 Checks that quoted text appears at the expected locations.\n\
                 Requires the 'receipts' feature flag."
            )
            .arg(
                Arg::new("receipt")
                    .value_name("PATH")
                    .value_hint(ValueHint::FilePath)
                    .required(true)
                    .help("Path to the receipt JSON file")
            )
            .arg(
                Arg::new("pdf")
                    .long("pdf")
                    .value_name("PATH")
                    .value_hint(ValueHint::FilePath)
                    .required(true)
                    .help("Path to the original PDF file")
            )
            .arg(
                Arg::new("tolerance")
                    .long("tolerance")
                    .value_name("PIXELS")
                    .default_value("10")
                    .help("Tolerance for bounding box matching in pixels")
            )
            .arg(
                Arg::new("json")
                    .long("json")
                    .action(ArgAction::SetTrue)
                    .help("Output results as JSON")
            )
    );

    // conformance subcommand
    cmd = cmd.subcommand(
        Command::new("conformance")
            .about("Run SDK conformance test suite")
            .arg(
                Arg::new("suite")
                    .short('s')
                    .long("suite")
                    .value_name("PATH")
                    .value_hint(ValueHint::FilePath)
                    .default_value("tests/sdk-conformance/cases.json")
                    .help("Path to the conformance suite JSON")
            )
            .arg(
                Arg::new("sdk")
                    .short('k')
                    .long("sdk")
                    .value_name("NAME")
                    .default_value("pdftract")
                    .help("SDK name")
            )
            .arg(
                Arg::new("version")
                    .short('v')
                    .long("version")
                    .value_name("VERSION")
                    .default_value("0.1.0")
                    .help("SDK version")
            )
            .arg(
                Arg::new("output")
                    .short('o')
                    .long("output")
                    .value_name("PATH")
                    .value_hint(ValueHint::FilePath)
                    .default_value("conformance-report.json")
                    .help("Output report path")
            )
    );

    // compare subcommand
    cmd = cmd.subcommand(
        Command::new("compare")
            .about("Compare actual results against expected values")
            .long_about(
                "Compare actual extraction results against expected values with tolerances.\n\
                 Used for conformance testing and validation."
            )
            .arg(
                Arg::new("actual")
                    .value_name("PATH")
                    .value_hint(ValueHint::FilePath)
                    .required(true)
                    .help("Path to the actual results JSON")
            )
            .arg(
                Arg::new("expected")
                    .value_name("PATH")
                    .value_hint(ValueHint::FilePath)
                    .required(true)
                    .help("Path to the expected results JSON")
            )
            .arg(
                Arg::new("tolerances")
                    .short('t')
                    .long("tolerances")
                    .value_name("PATH")
                    .value_hint(ValueHint::FilePath)
                    .help("Path to the tolerances JSON (optional)")
            )
            .arg(
                Arg::new("format")
                    .short('f')
                    .long("format")
                    .value_name("FORMAT")
                    .default_value("text")
                    .help("Output format (text, json)")
            )
    );

    // sdk subcommand
    let mut sdk_cmd = Command::new("sdk")
        .about("SDK code generation commands");

    sdk_cmd = sdk_cmd.subcommand(
        Command::new("codegen")
            .about("Generate SDK skeleton from templates")
            .arg(
                Arg::new("lang")
                    .short('l')
                    .long("lang")
                    .value_name("LANG")
                    .required(true)
                    .help("Target language")
            )
            .arg(
                Arg::new("out")
                    .short('o')
                    .long("out")
                    .value_name("DIR")
                    .value_hint(ValueHint::DirPath)
                    .required(true)
                    .help("Output directory")
            )
            .arg(
                Arg::new("version")
                    .short('v')
                    .long("version")
                    .value_name("VERSION")
                    .default_value("0.1.0")
                    .help("Version string (defaults to current pdftract version)")
            )
    );

    sdk_cmd = sdk_cmd.subcommand(
        Command::new("validate")
            .about("Validate existing SDK against current generator output")
            .arg(
                Arg::new("lang")
                    .short('l')
                    .long("lang")
                    .value_name("LANG")
                    .required(true)
                    .help("Target language")
            )
            .arg(
                Arg::new("sdk_dir")
                    .short('d')
                    .long("sdk-dir")
                    .value_name("DIR")
                    .value_hint(ValueHint::DirPath)
                    .required(true)
                    .help("Path to existing SDK directory")
            )
    );

    cmd = cmd.subcommand(sdk_cmd);

    // list-diagnostics subcommand
    cmd = cmd.subcommand(
        Command::new("list-diagnostics")
            .about("List all diagnostic codes with their metadata")
            .long_about(
                "List all diagnostic codes emitted during PDF parsing and extraction.\n\
                 Each diagnostic includes severity, recoverable flag, phase origin,\n\
                 and suggested action."
            )
    );

    // explain-diagnostic subcommand
    cmd = cmd.subcommand(
        Command::new("explain-diagnostic")
            .about("Explain a specific diagnostic code in detail")
            .arg(
                Arg::new("code")
                    .value_name("CODE")
                    .required(true)
                    .help("Diagnostic code to explain (e.g., STRUCT_MISSING_KEY, STREAM_BOMB)")
            )
    );

    // Generate markdown using clap-markdown
    // clap-markdown 0.1 uses a CommandFactory trait, so we need to capture stdout
    let mut buffer = String::new();
    buffer.push_str("# CLI Reference\n\n");
    buffer.push_str("This page provides comprehensive documentation for all pdftract CLI commands and flags.\n\n");
    buffer.push_str("## Usage\n\n");
    buffer.push_str("```bash\npdftract [OPTIONS] <COMMAND>\n```\n\n");
    buffer.push_str("## Global Options\n\n");
    buffer.push_str("These options are available across all subcommands:\n\n");
    buffer.push_str("- `-h, --help` - Print help information\n");
    buffer.push_str("- `-V, --version` - Print version information\n\n");
    buffer.push_str("## Commands\n\n");

    // Use clap-markdown's CommandFactory API
    // Since the cmd we built implements Command, we need to convert it
    // clap-markdown 0.1 expects to call .command() on a CommandFactory type
    // We'll manually generate the markdown for our custom command

    fn command_to_markdown(cmd: &Command, depth: usize) -> String {
        let mut result = String::new();
        let indent = " ".repeat(depth * 2);

        // Command name and description
        if depth == 0 {
            result.push_str(&format!("### `{}`\n\n", cmd.get_name()));
        } else {
            result.push_str(&format!("{}#### `{}`\n\n", indent, cmd.get_name()));
        }

        // About
        if let Some(about) = cmd.get_about() {
            result.push_str(&format!("{}\n\n", about));
        }

        // Long about
        if let Some(long_about) = cmd.get_long_about() {
            if let Some(about) = cmd.get_about() {
                if long_about != about {
                    result.push_str(&format!("{}\n\n", long_about));
                }
            } else {
                result.push_str(&format!("{}\n\n", long_about));
            }
        }

        // Usage
        let mut usage = String::new();
        usage.push_str(&cmd.get_name());
        if let Some(subcommand) = cmd.get_subcommands().find(|s| s.get_name() == "help") {
            // Skip help subcommand
        }
        result.push_str(&format!("**Usage:**\n\n```bash\npdftract {}\n```\n\n", usage));

        // Arguments
        let positional_args: Vec<_> = cmd.get_positionals()
            .filter(|a| !a.is_hide_set())
            .collect();

        if !positional_args.is_empty() {
            result.push_str("**Arguments:**\n\n");
            for arg in positional_args {
                result.push_str(&format!("- `<{}>`", arg.get_id()));
                if let Some(help) = arg.get_help() {
                    result.push_str(&format!(" - {}", help));
                }
                if arg.is_required_set() {
                    result.push_str(" (required)");
                }
                result.push_str("\n");
            }
            result.push_str("\n");
        }

        // Options
        let options: Vec<_> = cmd.get_opts()
            .filter(|o| !o.is_hide_set())
            .collect();

        if !options.is_empty() {
            result.push_str("**Options:**\n\n");
            for opt in options {
                let mut names = Vec::new();
                if let Some(short) = opt.get_short() {
                    names.push(format!("-{}", short));
                }
                if let Some(long) = opt.get_long() {
                    names.push(format!("--{}", long));
                }
                result.push_str(&format!("- `{}`", names.join(", ")));
                if let Some(value_name) = opt.get_value_names() {
                    result.push_str(&format!(" <{}>", value_name.join(" ")));
                }
                if let Some(help) = opt.get_help() {
                    result.push_str(&format!(" - {}", help));
                }
                if let Some(default) = opt.get_default_values().first() {
                    result.push_str(&format!(" (default: `{}`)", default.to_string_lossy()));
                }
                result.push_str("\n");
            }
            result.push_str("\n");
        }

        // Subcommands
        let subcommands: Vec<_> = cmd.get_subcommands()
            .filter(|s| !s.is_hide_set())
            .collect();

        for subcmd in subcommands {
            result.push_str(&command_to_markdown(subcmd, depth + 1));
        }

        result
    }

    buffer.push_str(&command_to_markdown(&cmd, 0));

    buffer
}
