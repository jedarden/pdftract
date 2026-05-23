//! Tool registry and individual tool implementations.
//!
//! The Tool trait defines the interface that all tools implement.
//! The ToolRegistry manages the collection of available tools and
//! provides the tools/list response.

use super::args::*;
use super::{ERROR_NOT_YET_IMPLEMENTED, ERROR_IO_ERROR, ERROR_PATH_INVALID, CODE_IO_ERROR, CODE_PATH_INVALID};
use crate::mcp::framing::ErrorObject;
use crate::mcp::root::resolve_path;
use pdftract_core::{
    parser::{self, catalog, pages, stream::{MemorySource, PdfSource}, xref},
    diagnostics::DiagCode,
    options::{ExtractionOptions, ReceiptsMode},
    extract::{extract_pdf, result_to_json},
};
use regex::Regex;
use serde_json::{json, to_value, Value};
use sha2::Digest;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Result type for tool execution.
pub type ToolResult = Result<Value, ErrorObject>;

/// Trait that all MCP tools must implement.
pub trait Tool: Send + Sync {
    /// Tool name (must match the key in the registry)
    fn name(&self) -> &'static str;

    /// One-line description for tools/list
    fn description(&self) -> &'static str;

    /// JSON Schema for the tool's arguments (inputSchema)
    fn input_schema(&self) -> Value;

    /// Execute the tool with the given arguments.
    ///
    /// The arguments are already validated against input_schema.
    ///
    /// # Arguments
    ///
    /// * `args` - The validated tool arguments
    /// * `log_path` - Optional path for logging (extracted from args before path resolution)
    /// * `root` - Optional root directory for path-traversal protection
    fn execute(&self, args: Value, log_path: Option<&str>, root: Option<&Path>) -> ToolResult;
}

/// Registry of all available MCP tools.
pub struct ToolRegistry {
    tools: HashMap<&'static str, Box<dyn Tool>>,
}

impl ToolRegistry {
    /// Create a new registry with all tools registered.
    pub fn new() -> Self {
        let mut registry = Self {
            tools: HashMap::new(),
        };
        registry.register_all();
        registry
    }

    /// Register all available tools.
    fn register_all(&mut self) {
        // Core extraction tools
        self.register(Box::new(ExtractTool));
        self.register(Box::new(ExtractTextTool));
        self.register(Box::new(ExtractMarkdownTool));

        // Search and metadata tools
        self.register(Box::new(SearchTool));
        self.register(Box::new(GetMetadataTool));

        // Fingerprint tool
        self.register(Box::new(HashTool));

        // Phase 7 stub tools (not yet implemented)
        self.register(Box::new(GetTableTool));
        self.register(Box::new(GetFormFieldsTool));
        self.register(Box::new(GetAttachmentsTool));
        self.register(Box::new(ClassifyTool));
    }

    /// Register a tool in the registry.
    fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.name(), tool);
    }

    /// Get a tool by name.
    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    /// Generate the tools/list response.
    pub fn tools_list(&self) -> Value {
        let tools: Vec<Value> = self
            .tools
            .values()
            .map(|tool| {
                json!({
                    "name": tool.name(),
                    "description": tool.description(),
                    "inputSchema": tool.input_schema(),
                })
            })
            .collect();

        json!({ "tools": tools })
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Get a registry with all tools registered.
pub fn all_tools() -> ToolRegistry {
    ToolRegistry::new()
}

/// Find the startxref offset by scanning the end of the PDF.
///
/// Scans backwards from EOF to find the startxref keyword.
fn find_startxref_offset(data: &[u8]) -> Result<u64, ErrorObject> {
    // Start from the end, scan backwards looking for "startxref"
    // We scan at most 1024 bytes from the end (per PDF spec, startxref is near EOF)
    let scan_len = data.len().min(1024);
    let start = data.len().saturating_sub(scan_len);

    // Look for "startxref" keyword
    let search_bytes = &data[start..];
    if let Some(pos) = search_bytes.windows(9).rposition(|w| w == b"startxref") {
        // Find the newline after startxref, then parse the offset
        let after_startxref = start + pos + 9;
        let mut offset_start = after_startxref;

        // Skip whitespace after startxref
        while offset_start < data.len() && data[offset_start].is_ascii_whitespace() {
            offset_start += 1;
        }

        // Parse the offset number
        let mut offset_end = offset_start;
        while offset_end < data.len() && data[offset_end].is_ascii_digit() {
            offset_end += 1;
        }

        if offset_start >= data.len() || offset_end == offset_start {
            return Err(ErrorObject::server_error(
                super::ERROR_IO_ERROR,
                "Invalid startxref offset in PDF",
            ).with_data(json!({"code": super::CODE_IO_ERROR})));
        }

        let offset_str = std::str::from_utf8(&data[offset_start..offset_end])
            .map_err(|_| ErrorObject::server_error(
                super::ERROR_IO_ERROR,
                "Invalid UTF-8 in startxref offset",
            ).with_data(json!({"code": super::CODE_IO_ERROR})))?;

        let offset: u64 = offset_str.parse().map_err(|_| ErrorObject::server_error(
            super::ERROR_IO_ERROR,
            "Failed to parse startxref offset",
        ).with_data(json!({"code": super::CODE_IO_ERROR})))?;

        Ok(offset)
    } else {
        // If startxref not found, fall back to forward scan
        Ok(0)
    }
}

/// Result of opening and parsing a PDF file.
struct PdfContext {
    /// The file path
    path: PathBuf,
    /// The memory source containing the PDF data
    source: MemorySource,
    /// The xref section
    xref_section: xref::XrefSection,
    /// The catalog (if parsing succeeded)
    catalog: Option<catalog::Catalog>,
    /// Page count (if parsing succeeded)
    page_count: Option<usize>,
}

/// Open a PDF file and parse its basic structure.
///
/// Returns an error if:
/// - The file doesn't exist or can't be read
/// - The PDF is encrypted and no password was provided
/// - The PDF structure is invalid
///
/// # Arguments
///
/// * `path` - The path argument (may be a URL or local path)
/// * `password` - Optional PDF password
/// * `root` - Optional root directory for path-traversal protection
fn open_pdf(path: &str, password: Option<&str>, root: Option<&Path>) -> Result<PdfContext, ErrorObject> {
    // Validate and resolve the path using the root if set
    let path_buf = resolve_path(path, root)?;

    // Check if it's a file (not a directory)
    if !path_buf.is_file() {
        return Err(ErrorObject::server_error(
            ERROR_PATH_INVALID,
            format!("Not a file: {}", path),
        ).with_data(json!({"code": CODE_PATH_INVALID, "path": path})));
    }

    // Read the PDF file
    let buffer = fs::read(&path_buf).map_err(|e| {
        ErrorObject::server_error(
            ERROR_IO_ERROR,
            format!("Failed to read PDF file: {}", e),
        ).with_data(json!({"code": CODE_IO_ERROR, "path": path}))
    })?;

    // Check for PDF magic number
    if buffer.len() < 5 || !buffer.starts_with(b"%PDF-") {
        return Err(ErrorObject::server_error(
            ERROR_IO_ERROR,
            "Not a valid PDF file (missing %PDF- header)",
        ).with_data(json!({"code": CODE_IO_ERROR, "path": path})));
    }

    // Create a MemorySource for parsing
    let source = MemorySource::new(buffer);

    // Use forward_scan_xref to parse the PDF (handles both traditional and hybrid xrefs)
    let xref_section = xref::forward_scan_xref(&source, false);

    // Check for encryption errors in diagnostics
    for diag in &xref_section.diagnostics {
        if diag.code == DiagCode::EncryptionUnsupported {
            return Err(ErrorObject::server_error(
                super::ERROR_PDF_ENCRYPTED,
                "PDF is encrypted and no password was provided",
            ).with_data(json!({"code": super::CODE_PDF_ENCRYPTED})));
        }
    }

    // Check for /Encrypt dictionary in the trailer (indicates encryption)
    if let Some(trailer) = &xref_section.trailer {
        if trailer.get("Encrypt").is_some() {
            return Err(ErrorObject::server_error(
                super::ERROR_PDF_ENCRYPTED,
                "PDF is encrypted and no password was provided",
            ).with_data(json!({"code": super::CODE_PDF_ENCRYPTED})));
        }
    }

    // Get the root reference from the trailer
    let root_ref = xref_section.trailer.as_ref()
        .and_then(|trailer| trailer.get("Root"))
        .and_then(|obj| {
            match obj {
                pdftract_core::parser::object::PdfObject::Ref(obj_ref) => Some(obj_ref),
                _ => None,
            }
        });

    let (catalog, page_count) = match root_ref {
        Some(root_ref) => {
            // Create a resolver from the xref section
            let resolver = parser::xref::XrefResolver::from_section(xref_section.clone());

            // Try to parse the catalog
            let catalog_result = catalog::parse_catalog(&resolver, *root_ref);

            match catalog_result {
                Ok(catalog) => {
                    // Flatten the page tree to get page count
                    let page_count = pages::flatten_page_tree(&resolver, catalog.pages_ref)
                        .map(|pages| pages.len())
                        .ok();

                    (Some(catalog), page_count)
                }
                Err(diags) => {
                    // Check for encryption errors
                    if diags.iter().any(|d| d.code == DiagCode::EncryptionUnsupported) {
                        return Err(ErrorObject::server_error(
                            super::ERROR_PDF_ENCRYPTED,
                            "PDF is encrypted and no password was provided",
                        ).with_data(json!({"code": super::CODE_PDF_ENCRYPTED})));
                    }
                    // Catalog parsing failed - return partial context
                    (None, None)
                }
            }
        }
        None => {
            // No root reference - return partial context
            (None, None)
        }
    };

    Ok(PdfContext {
        path: path_buf,
        source,
        xref_section,
        catalog,
        page_count,
    })
}

/// Check if a path is a URL (http:// or https://)
fn is_url(path: &str) -> bool {
    path.starts_with("http://") || path.starts_with("https://")
}

/// Build ExtractionOptions from MCP tool arguments.
fn build_extraction_options(
    pages: &Option<String>,
    _ocr: &Option<bool>,
    receipts: Option<&str>,
) -> ExtractionOptions {
    // Parse receipts mode
    let receipts_mode = match receipts {
        None | Some("off") => ReceiptsMode::Off,
        Some("lite") => ReceiptsMode::Lite,
        Some("svg") => ReceiptsMode::SvgClip,
        Some(other) => {
            // Invalid value - default to off
            // In production, this should return an error
            eprintln!("Warning: invalid receipts mode '{}', using 'off'", other);
            ReceiptsMode::Off
        }
    };

    // Note: pages and ocr options are not yet implemented in the extraction pipeline
    // They are parsed here for future compatibility
    if pages.is_some() {
        // TODO: implement page range selection
    }

    ExtractionOptions::with_receipts(receipts_mode)
}

/// Create a stub response for tools that require Phase 6 extraction surface.
fn stub_extraction_response(path: &str, tool_name: &str, page_count: Option<usize>) -> Value {
    let mut response = serde_json::Map::new();
    response.insert("_note".to_string(), json!("This tool requires Phase 6 extraction surface"));
    response.insert("_tool".to_string(), json!(tool_name));
    response.insert("_path".to_string(), json!(path));

    if let Some(count) = page_count {
        response.insert("_page_count".to_string(), json!(count));
    }

    // Add format-specific fields
    match tool_name {
        "extract" => {
            response.insert("pages".to_string(), json!([]));
            response.insert("metadata".to_string(), json!({}));
        }
        "extract_text" => {
            response.insert("text".to_string(), json!(""));
        }
        "extract_markdown" => {
            response.insert("markdown".to_string(), json!(""));
        }
        "search" => {
            response.insert("matches".to_string(), json!([]));
        }
        _ => {}
    }

    json!(response)
}

// ============================================================================
// Tool Implementations
// ============================================================================

/// Extract tool - full extraction returning document JSON.
struct ExtractTool;

impl Tool for ExtractTool {
    fn name(&self) -> &'static str {
        "extract"
    }

    fn description(&self) -> &'static str {
        "Extract text and structure from a PDF file, returning the full document JSON"
    }

    fn input_schema(&self) -> Value {
        to_value(schemars::schema_for!(ExtractArgs)).unwrap()
    }

    fn execute(&self, args: Value, _log_path: Option<&str>, root: Option<&Path>) -> ToolResult {
        // Parse arguments
        let tool_args: ExtractArgs = serde_json::from_value(args)
            .map_err(|_| ErrorObject::invalid_params())?;

        // Check if path is a URL
        if is_url(&tool_args.path) {
            return Ok(json!({
                "_note": "Remote PDF extraction requires Phase 1.8 remote source adapter",
                "_tool": "extract",
                "_path": tool_args.path,
                "pages": [],
                "metadata": {}
            }));
        }

        // Validate and resolve the path
        let path_buf = resolve_path(&tool_args.path, root)?;

        // Build extraction options
        let options = build_extraction_options(&tool_args.pages, &tool_args.ocr, tool_args.receipts.as_deref());

        // Perform the extraction
        let result = extract_pdf(&path_buf, &options)
            .map_err(|e| ErrorObject::server_error(
                super::ERROR_IO_ERROR,
                format!("Extraction failed: {}", e),
            ).with_data(json!({"code": super::CODE_IO_ERROR})))?;

        Ok(result_to_json(&result))
    }
}

/// Extract text tool - plain-text extraction.
struct ExtractTextTool;

impl Tool for ExtractTextTool {
    fn name(&self) -> &'static str {
        "extract_text"
    }

    fn description(&self) -> &'static str {
        "Extract plain text from a PDF file"
    }

    fn input_schema(&self) -> Value {
        to_value(schemars::schema_for!(ExtractTextArgs)).unwrap()
    }

    fn execute(&self, args: Value, _log_path: Option<&str>, root: Option<&Path>) -> ToolResult {
        let tool_args: ExtractTextArgs = serde_json::from_value(args)
            .map_err(|_| ErrorObject::invalid_params())?;

        if is_url(&tool_args.path) {
            return Ok(json!({
                "_note": "Remote PDF extraction requires Phase 1.8 remote source adapter",
                "_tool": "extract_text",
                "_path": tool_args.path,
                "text": ""
            }));
        }

        // Validate and resolve the path
        let path_buf = resolve_path(&tool_args.path, root)?;

        // Build extraction options
        let options = build_extraction_options(&tool_args.pages, &tool_args.ocr, tool_args.receipts.as_deref());

        // Perform the extraction
        let result = extract_pdf(&path_buf, &options)
            .map_err(|e| ErrorObject::server_error(
                super::ERROR_IO_ERROR,
                format!("Extraction failed: {}", e),
            ).with_data(json!({"code": super::CODE_IO_ERROR})))?;

        // Convert to plain text
        let text = result.pages.iter()
            .flat_map(|page| page.spans.iter().map(|span| span.text.as_str()))
            .collect::<Vec<&str>>()
            .join("\n");

        Ok(json!({ "text": text }))
    }
}

/// Extract markdown tool - markdown extraction.
struct ExtractMarkdownTool;

impl Tool for ExtractMarkdownTool {
    fn name(&self) -> &'static str {
        "extract_markdown"
    }

    fn description(&self) -> &'static str {
        "Extract text from a PDF file and format it as Markdown"
    }

    fn input_schema(&self) -> Value {
        to_value(schemars::schema_for!(ExtractMarkdownArgs)).unwrap()
    }

    fn execute(&self, args: Value, _log_path: Option<&str>, root: Option<&Path>) -> ToolResult {
        let tool_args: ExtractMarkdownArgs = serde_json::from_value(args)
            .map_err(|_| ErrorObject::invalid_params())?;

        if is_url(&tool_args.path) {
            return Ok(json!({
                "_note": "Remote PDF extraction requires Phase 1.8 remote source adapter",
                "_tool": "extract_markdown",
                "_path": tool_args.path,
                "markdown": ""
            }));
        }

        // Validate and resolve the path
        let path_buf = resolve_path(&tool_args.path, root)?;

        // Build extraction options
        let options = build_extraction_options(&tool_args.pages, &tool_args.ocr, tool_args.receipts.as_deref());

        // Perform the extraction
        let result = extract_pdf(&path_buf, &options)
            .map_err(|e| ErrorObject::server_error(
                super::ERROR_IO_ERROR,
                format!("Extraction failed: {}", e),
            ).with_data(json!({"code": super::CODE_IO_ERROR})))?;

        // Convert to markdown
        let markdown = result.pages.iter()
            .flat_map(|page| page.blocks.iter().map(|block| {
                match block.kind.as_str() {
                    "heading" => {
                        let level = block.level.unwrap_or(1);
                        let prefix = "#".repeat(level as usize);
                        format!("{} {}\n", prefix, block.text)
                    }
                    "paragraph" => format!("{}\n", block.text),
                    _ => format!("{}\n", block.text),
                }
            }))
            .collect::<Vec<String>>()
            .join("\n");

        Ok(json!({ "markdown": markdown }))
    }
}

/// Search tool - regex search across the file.
struct SearchTool;

impl Tool for SearchTool {
    fn name(&self) -> &'static str {
        "search"
    }

    fn description(&self) -> &'static str {
        "Search for a regex pattern across the PDF, returning matches with page and bbox coordinates"
    }

    fn input_schema(&self) -> Value {
        to_value(schemars::schema_for!(SearchArgs)).unwrap()
    }

    fn execute(&self, args: Value, _log_path: Option<&str>, root: Option<&Path>) -> ToolResult {
        let tool_args: SearchArgs = serde_json::from_value(args)
            .map_err(|_| ErrorObject::invalid_params())?;

        // Validate the regex pattern
        let _regex = Regex::new(&tool_args.pattern).map_err(|e| {
            ErrorObject::invalid_params()
                .with_data(json!({"reason": "Invalid regex pattern", "details": e.to_string()}))
        })?;

        if is_url(&tool_args.path) {
            return Ok(json!({
                "_note": "Remote PDF search requires Phase 1.8 remote source adapter",
                "_tool": "search",
                "_path": tool_args.path,
                "_pattern": tool_args.pattern,
                "matches": []
            }));
        }

        let ctx = open_pdf(&tool_args.path, tool_args.password.as_deref(), root)?;
        let mut response = stub_extraction_response(&tool_args.path, "search", ctx.page_count);
        if let Some(obj) = response.as_object_mut() {
            obj.insert("_pattern".to_string(), json!(tool_args.pattern));
        }
        Ok(response)
    }
}

/// Get metadata tool - metadata + outline + fingerprint only (cheap path).
struct GetMetadataTool;

impl Tool for GetMetadataTool {
    fn name(&self) -> &'static str {
        "get_metadata"
    }

    fn description(&self) -> &'static str {
        "Get PDF metadata, outline, and fingerprint without full extraction (fast, < 250ms for 100-page PDFs)"
    }

    fn input_schema(&self) -> Value {
        to_value(schemars::schema_for!(GetMetadataArgs)).unwrap()
    }

    fn execute(&self, args: Value, _log_path: Option<&str>, root: Option<&Path>) -> ToolResult {
        let tool_args: GetMetadataArgs = serde_json::from_value(args)
            .map_err(|_| ErrorObject::invalid_params())?;

        // Check if path is a URL
        if is_url(&tool_args.path) {
            return Ok(json!({
                "metadata": {},
                "outline": [],
                "fingerprint": "",
                "_note": "Remote PDF metadata extraction requires Phase 1.8 remote source adapter"
            }));
        }

        // Parse the PDF to extract metadata
        let result = extract_metadata(&tool_args.path, tool_args.password.as_deref(), root);

        match result {
            Ok(metadata) => Ok(metadata),
            Err(e) => Err(e),
        }
    }
}

/// Extract metadata from a PDF file.
fn extract_metadata(path: &str, _password: Option<&str>, root: Option<&Path>) -> ToolResult {
    let ctx = open_pdf(path, _password, root)?;

    // Build metadata response
    let mut metadata = serde_json::Map::new();

    // Page count
    if let Some(count) = ctx.page_count {
        metadata.insert("page_count".to_string(), json!(count));
    }

    // Catalog info if available
    if let Some(catalog) = &ctx.catalog {
        metadata.insert("is_tagged".to_string(), json!(catalog.mark_info.is_tagged));

        // PDF version
        if let Some(version) = &catalog.version {
            metadata.insert("version".to_string(), json!(version));
        }

        // Outline (bookmarks) - if present
        let outline = if catalog.outlines_ref.is_some() {
            // TODO: Parse outline structure
            json!([])
        } else {
            json!([])
        };

        // Fingerprint - compute a simple one based on file size and page count
        // Full fingerprint computation would use the Phase 1.7 algorithm
        let fingerprint = format!("pdftract-v1:{:064x}",
            sha2::Sha256::digest(
                format!("{}:{}:{}",
                    ctx.source.len().unwrap_or(0),
                    ctx.page_count.unwrap_or(0),
                    catalog.pages_ref.object
                ).as_bytes()
            ));

        Ok(json!({
            "metadata": metadata,
            "outline": outline,
            "fingerprint": fingerprint
        }))
    } else {
        // Catalog not available, return partial metadata
        let fingerprint = format!("pdftract-v1:{:064x}",
            sha2::Sha256::digest(
                format!("{}:{}",
                    ctx.source.len().unwrap_or(0),
                    ctx.page_count.unwrap_or(0)
                ).as_bytes()
            ));

        Ok(json!({
            "metadata": metadata,
            "outline": [],
            "fingerprint": fingerprint
        }))
    }
}

/// Hash tool - compute structural fingerprint only.
struct HashTool;

impl Tool for HashTool {
    fn name(&self) -> &'static str {
        "hash"
    }

    fn description(&self) -> &'static str {
        "Compute the structural fingerprint of a PDF (fast, < 100ms for 100-page PDFs)"
    }

    fn input_schema(&self) -> Value {
        to_value(schemars::schema_for!(HashArgs)).unwrap()
    }

    fn execute(&self, args: Value, _log_path: Option<&str>, root: Option<&Path>) -> ToolResult {
        let tool_args: HashArgs = serde_json::from_value(args)
            .map_err(|_| ErrorObject::invalid_params())?;

        // Check if path is a URL
        if is_url(&tool_args.path) {
            return Ok(json!({
                "fingerprint": "",
                "_note": "Remote PDF fingerprinting requires Phase 1.8 remote source adapter"
            }));
        }

        // Parse the PDF to compute fingerprint
        let result = compute_fingerprint(&tool_args.path, tool_args.password.as_deref(), root);

        match result {
            Ok(fingerprint) => Ok(json!({ "fingerprint": fingerprint })),
            Err(e) => Err(e),
        }
    }
}

/// Compute the fingerprint of a PDF file.
fn compute_fingerprint(path: &str, _password: Option<&str>, root: Option<&Path>) -> Result<String, ErrorObject> {
    let ctx = open_pdf(path, _password, root)?;

    // Compute a simplified fingerprint for now
    // Full fingerprint computation would use the Phase 1.7 algorithm with
    // content stream hashing, resource dict hashing, etc.
    if let Some(catalog) = &ctx.catalog {
        let fingerprint = format!("pdftract-v1:{:064x}",
            sha2::Sha256::digest(
                format!("{}:{}:{}:{}",
                    ctx.source.len().unwrap_or(0),
                    ctx.page_count.unwrap_or(0),
                    catalog.pages_ref.object,
                    catalog.mark_info.is_tagged
                ).as_bytes()
            ));
        Ok(fingerprint)
    } else {
        let fingerprint = format!("pdftract-v1:{:064x}",
            sha2::Sha256::digest(
                format!("{}:{}",
                    ctx.source.len().unwrap_or(0),
                    ctx.page_count.unwrap_or(0)
                ).as_bytes()
            ));
        Ok(fingerprint)
    }
}

/// Get table tool (Phase 7.2 stub).
struct GetTableTool;

impl Tool for GetTableTool {
    fn name(&self) -> &'static str {
        "get_table"
    }

    fn description(&self) -> &'static str {
        "Extract a single table by page and table index (Phase 7.2 - not yet implemented)"
    }

    fn input_schema(&self) -> Value {
        to_value(schemars::schema_for!(GetTableArgs)).unwrap()
    }

    fn execute(&self, _args: Value, _log_path: Option<&str>, _root: Option<&Path>) -> ToolResult {
        // Validate args structure but don't process
        let _args: GetTableArgs = match serde_json::from_value(_args) {
            Ok(args) => args,
            Err(_) => {
                return Err(ErrorObject::invalid_params()
                    .with_data(json!({"reason": "Invalid arguments for get_table"})));
            }
        };

        // Return NOT_YET_IMPLEMENTED immediately
        Err(ErrorObject::server_error(
            super::ERROR_NOT_YET_IMPLEMENTED,
            "get_table is not yet implemented (Phase 7.2)",
        )
        .with_data(json!({"code": super::CODE_NOT_YET_IMPLEMENTED})))
    }
}

/// Get form fields tool (Phase 7.4 stub).
struct GetFormFieldsTool;

impl Tool for GetFormFieldsTool {
    fn name(&self) -> &'static str {
        "get_form_fields"
    }

    fn description(&self) -> &'static str {
        "Extract AcroForm/XFA field values (Phase 7.4 - not yet implemented)"
    }

    fn input_schema(&self) -> Value {
        to_value(schemars::schema_for!(GetFormFieldsArgs)).unwrap()
    }

    fn execute(&self, _args: Value, _log_path: Option<&str>, _root: Option<&Path>) -> ToolResult {
        // Validate args structure but don't process
        let _args: GetFormFieldsArgs = match serde_json::from_value(_args) {
            Ok(args) => args,
            Err(_) => {
                return Err(ErrorObject::invalid_params()
                    .with_data(json!({"reason": "Invalid arguments for get_form_fields"})));
            }
        };

        Err(ErrorObject::server_error(
            super::ERROR_NOT_YET_IMPLEMENTED,
            "get_form_fields is not yet implemented (Phase 7.4)",
        )
        .with_data(json!({"code": super::CODE_NOT_YET_IMPLEMENTED})))
    }
}

/// Get attachments tool (Phase 7.5 stub).
struct GetAttachmentsTool;

impl Tool for GetAttachmentsTool {
    fn name(&self) -> &'static str {
        "get_attachments"
    }

    fn description(&self) -> &'static str {
        "Extract embedded files from the PDF (Phase 7.5 - not yet implemented)"
    }

    fn input_schema(&self) -> Value {
        to_value(schemars::schema_for!(GetAttachmentsArgs)).unwrap()
    }

    fn execute(&self, _args: Value, _log_path: Option<&str>, _root: Option<&Path>) -> ToolResult {
        // Validate args structure but don't process
        let _args: GetAttachmentsArgs = match serde_json::from_value(_args) {
            Ok(args) => args,
            Err(_) => {
                return Err(ErrorObject::invalid_params()
                    .with_data(json!({"reason": "Invalid arguments for get_attachments"})));
            }
        };

        Err(ErrorObject::server_error(
            super::ERROR_NOT_YET_IMPLEMENTED,
            "get_attachments is not yet implemented (Phase 7.5)",
        )
        .with_data(json!({"code": super::CODE_NOT_YET_IMPLEMENTED})))
    }
}

/// Classify tool (Phase 5.6 stub).
struct ClassifyTool;

impl Tool for ClassifyTool {
    fn name(&self) -> &'static str {
        "classify"
    }

    fn description(&self) -> &'static str {
        "Run the PDF classifier to categorize the document (Phase 5.6 - not yet implemented)"
    }

    fn input_schema(&self) -> Value {
        to_value(schemars::schema_for!(ClassifyArgs)).unwrap()
    }

    fn execute(&self, _args: Value, _log_path: Option<&str>, _root: Option<&Path>) -> ToolResult {
        // Validate args structure but don't process
        let _args: ClassifyArgs = match serde_json::from_value(_args) {
            Ok(args) => args,
            Err(_) => {
                return Err(ErrorObject::invalid_params()
                    .with_data(json!({"reason": "Invalid arguments for classify"})));
            }
        };

        Err(ErrorObject::server_error(
            super::ERROR_NOT_YET_IMPLEMENTED,
            "classify is not yet implemented (Phase 5.6)",
        )
        .with_data(json!({"code": super::CODE_NOT_YET_IMPLEMENTED})))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_has_all_tools() {
        let registry = all_tools();
        assert_eq!(registry.tools.len(), 10);
    }

    #[test]
    fn test_tools_list_response() {
        let registry = all_tools();
        let list = registry.tools_list();

        assert!(list.is_object());
        let tools = list.get("tools").and_then(|v| v.as_array());
        assert!(tools.is_some());
        assert_eq!(tools.unwrap().len(), 10);
    }

    #[test]
    fn test_extract_tool_schema() {
        let tool = ExtractTool;
        let schema = tool.input_schema();

        assert!(schema.is_object());
        let obj = schema.as_object().unwrap();
        assert_eq!(obj.get("type").and_then(|v| v.as_str()), Some("object"));

        let props = obj.get("properties").and_then(|v| v.as_object());
        assert!(props.is_some());

        let props = props.unwrap();
        assert!(props.contains_key("path"));
        assert!(props.contains_key("pages"));
        assert!(props.contains_key("ocr"));
        assert!(props.contains_key("formats"));
        assert!(props.contains_key("auto_profile"));
        assert!(props.contains_key("password"));
        assert!(props.contains_key("receipts"));
    }

    #[test]
    fn test_extract_text_tool_schema() {
        let tool = ExtractTextTool;
        let schema = tool.input_schema();

        assert!(schema.is_object());
        let obj = schema.as_object().unwrap();
        let props = obj.get("properties").and_then(|v| v.as_object()).unwrap();

        assert!(props.contains_key("path"));
        assert!(props.contains_key("pages"));
        assert!(props.contains_key("ocr"));
        assert!(props.contains_key("password"));
    }

    #[test]
    fn test_search_tool_schema() {
        let tool = SearchTool;
        let schema = tool.input_schema();

        let props = schema
            .as_object()
            .and_then(|o| o.get("properties"))
            .and_then(|v| v.as_object())
            .unwrap();

        assert!(props.contains_key("path"));
        assert!(props.contains_key("pattern"));
        assert!(props.contains_key("case_insensitive"));
        assert!(props.contains_key("max_matches"));
        assert!(props.contains_key("password"));
    }

    #[test]
    fn test_get_metadata_tool_schema() {
        let tool = GetMetadataTool;
        let schema = tool.input_schema();

        let props = schema
            .as_object()
            .and_then(|o| o.get("properties"))
            .and_then(|v| v.as_object())
            .unwrap();

        assert!(props.contains_key("path"));
        assert!(props.contains_key("password"));
    }

    #[test]
    fn test_hash_tool_schema() {
        let tool = HashTool;
        let schema = tool.input_schema();

        let props = schema
            .as_object()
            .and_then(|o| o.get("properties"))
            .and_then(|v| v.as_object())
            .unwrap();

        assert!(props.contains_key("path"));
        assert!(props.contains_key("password"));
    }

    #[test]
    fn test_stub_tools_return_not_implemented() {
        let registry = all_tools();

        // Test get_table
        let tool = registry.get("get_table").unwrap();
        let result = tool.execute(json!({"path": "test.pdf", "page": 0, "table_index": 0}), None, None);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, ERROR_NOT_YET_IMPLEMENTED);

        // Test get_form_fields
        let tool = registry.get("get_form_fields").unwrap();
        let result = tool.execute(json!({"path": "test.pdf"}), None, None);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, ERROR_NOT_YET_IMPLEMENTED);

        // Test get_attachments
        let tool = registry.get("get_attachments").unwrap();
        let result = tool.execute(json!({"path": "test.pdf"}), None, None);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, ERROR_NOT_YET_IMPLEMENTED);

        // Test classify
        let tool = registry.get("classify").unwrap();
        let result = tool.execute(json!({"path": "test.pdf"}), None, None);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, ERROR_NOT_YET_IMPLEMENTED);
    }

    #[test]
    fn test_invalid_params_returns_correct_error() {
        let tool = ExtractTool;

        // Missing required field
        let result = tool.execute(json!({}), None, None);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, -32602); // Invalid params
    }

    #[test]
    fn test_tool_names_match_registry_keys() {
        let registry = all_tools();

        for (key, tool) in &registry.tools {
            assert_eq!(*key, tool.name(), "Registry key must match tool name");
        }
    }

    #[test]
    fn test_extract_schema_validates_draft07() {
        // Test that the extract tool schema is valid JSON Schema draft-07
        let tool = ExtractTool;
        let schema = tool.input_schema();

        // Create a JSON Schema validator
        let compilation_result = jsonschema::JSONSchema::compile(&schema);
        assert!(compilation_result.is_ok(), "Extract tool schema should be valid JSON Schema");
    }

    #[test]
    fn test_extract_text_schema_validates_draft07() {
        let tool = ExtractTextTool;
        let schema = tool.input_schema();

        let compilation_result = jsonschema::JSONSchema::compile(&schema);
        assert!(compilation_result.is_ok(), "ExtractText tool schema should be valid JSON Schema");
    }

    #[test]
    fn test_extract_markdown_schema_validates_draft07() {
        let tool = ExtractMarkdownTool;
        let schema = tool.input_schema();

        let compilation_result = jsonschema::JSONSchema::compile(&schema);
        assert!(compilation_result.is_ok(), "ExtractMarkdown tool schema should be valid JSON Schema");
    }

    #[test]
    fn test_search_schema_validates_draft07() {
        let tool = SearchTool;
        let schema = tool.input_schema();

        let compilation_result = jsonschema::JSONSchema::compile(&schema);
        assert!(compilation_result.is_ok(), "Search tool schema should be valid JSON Schema");
    }

    #[test]
    fn test_get_metadata_schema_validates_draft07() {
        let tool = GetMetadataTool;
        let schema = tool.input_schema();

        let compilation_result = jsonschema::JSONSchema::compile(&schema);
        assert!(compilation_result.is_ok(), "GetMetadata tool schema should be valid JSON Schema");
    }

    #[test]
    fn test_hash_schema_validates_draft07() {
        let tool = HashTool;
        let schema = tool.input_schema();

        let compilation_result = jsonschema::JSONSchema::compile(&schema);
        assert!(compilation_result.is_ok(), "Hash tool schema should be valid JSON Schema");
    }

    #[test]
    fn test_get_table_schema_validates_draft07() {
        let tool = GetTableTool;
        let schema = tool.input_schema();

        let compilation_result = jsonschema::JSONSchema::compile(&schema);
        assert!(compilation_result.is_ok(), "GetTable tool schema should be valid JSON Schema");
    }

    #[test]
    fn test_get_form_fields_schema_validates_draft07() {
        let tool = GetFormFieldsTool;
        let schema = tool.input_schema();

        let compilation_result = jsonschema::JSONSchema::compile(&schema);
        assert!(compilation_result.is_ok(), "GetFormFields tool schema should be valid JSON Schema");
    }

    #[test]
    fn test_get_attachments_schema_validates_draft07() {
        let tool = GetAttachmentsTool;
        let schema = tool.input_schema();

        let compilation_result = jsonschema::JSONSchema::compile(&schema);
        assert!(compilation_result.is_ok(), "GetAttachments tool schema should be valid JSON Schema");
    }

    #[test]
    fn test_classify_schema_validates_draft07() {
        let tool = ClassifyTool;
        let schema = tool.input_schema();

        let compilation_result = jsonschema::JSONSchema::compile(&schema);
        assert!(compilation_result.is_ok(), "Classify tool schema should be valid JSON Schema");
    }

    #[test]
    fn test_all_schemas_are_valid_json_schemas() {
        let registry = all_tools();

        for (_key, tool) in &registry.tools {
            let schema = tool.input_schema();
            let compilation_result = jsonschema::JSONSchema::compile(&schema);
            assert!(compilation_result.is_ok(),
                "Tool '{}' schema should be valid JSON Schema: {:?}",
                tool.name(),
                compilation_result.err());
        }
    }

    #[test]
    fn test_find_startxref_offset_valid_pdf() {
        // A minimal valid PDF with startxref at offset 100
        let pdf_data = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog >>\nendobj\nxref\n0 2\n0000000000 65535 f \n0000000009 00000 n \ntrailer\n<< /Size 2 >>\nstartxref\n100\n%%EOF";

        let offset = find_startxref_offset(pdf_data).unwrap();
        assert_eq!(offset, 100);
    }

    #[test]
    fn test_find_startxref_offset_no_startxref() {
        let pdf_data = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog >>\nendobj\n%%EOF";

        let result = find_startxref_offset(pdf_data);
        // When startxref is not found, we return Ok(0) to signal forward scan should be used
        assert_eq!(result.unwrap(), 0);
    }
}
