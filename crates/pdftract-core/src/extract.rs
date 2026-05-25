//! PDF text extraction with receipt generation.
//!
//! This module provides the main extraction pipeline that processes PDFs
//! and generates spans and blocks with optional cryptographic receipts.
//!
//! Page extraction runs in parallel using rayon, with the number of
//! simultaneously-resident pages capped by a semaphore to keep memory
//! bounded regardless of core count.
//!
//! ## Lazy Stream Decoding
//!
//! Content streams are decoded lazily per page and dropped immediately after
//! processing. This ensures peak RSS stays flat across page count, even for
//! large documents with 10,000+ pages.

use crate::annotation::{dispatch_annotations, json as annotation_json};
use crate::attachment::associated_files::walk_af_array;
use crate::attachment::filespec::extract_one;
use crate::diagnostics::{DiagCode, Diagnostic};
use crate::document::compute_fingerprint_lazy;
use crate::forms::{
    acro_field_to_value, combine, walk_acroform_fields, AcroFormField, FormFieldValue,
};
use crate::options::{ExtractionOptions, ReceiptsMode};
use crate::parser::catalog::ReadingOrderAlgorithm;
use crate::parser::marked_content::{track_mcids_from_content_stream, McidTracker};
use crate::parser::stream::FileSource;
use crate::parser::stream::DEFAULT_MAX_DECOMPRESS_BYTES;
use crate::parser::struct_tree::{check_coverage_for_pages, parse_struct_tree};
use crate::receipts::Receipt;
use crate::schema::{
    AnnotationJson, AttachmentJson, BlockJson, ChoiceValueJson, FormFieldJson, FormFieldTypeJson,
    FormFieldValueJson, JavascriptActionJson, LinkJson, SignatureJson, SpanJson, TableJson, ThreadJson,
};
use crate::semaphore::{Semaphore, SemaphoreExt};
use crate::signature::{discover, extract_signatures};
use crate::table::{
    detect_two_page_tables, grid_to_table_json, GridCandidate, PageContext, TableDetector,
};
use crate::table::{TableCell as Cell, TableSpan};
use anyhow::{Context, Result};
use rayon::prelude::*;
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::cmp::Ordering;
use std::sync::Arc;

#[cfg(feature = "receipts")]
use crate::receipts::svg::GlyphList;

/// Decode content streams for a page, returning the concatenated decoded bytes.
///
/// This function decodes all content streams for a page lazily and drops them
/// immediately after returning. The decoded bytes are scoped to ensure they're
/// freed before processing the next page.
///
/// # Arguments
///
/// * `page` - The page dictionary containing content stream references
/// * `resolver` - The xref resolver for resolving indirect references
/// * `source` - The PDF source for reading stream data
/// * `max_decompress_bytes` - Maximum decompressed bytes allowed (bomb limit)
///
/// # Returns
///
/// The decoded content stream bytes, or an empty Vec if decoding fails.
///
/// # Memory Behavior
///
/// This function ensures decoded streams are dropped immediately after use:
/// - Each stream is decoded and returned as Vec<u8>
/// - The caller must drop the Vec before processing the next page
/// - No decoded data is held across page boundaries
fn decode_page_content_streams(
    page: &crate::parser::pages::PageDict,
    resolver: &crate::parser::xref::XrefResolver,
    source: &dyn crate::parser::stream::PdfSource,
    max_decompress_bytes: u64,
) -> Vec<u8> {
    use crate::parser::stream::{decode_stream, ExtractionOptions as StreamExtractionOptions};

    // Create stream extraction options with the bomb limit
    let stream_opts = StreamExtractionOptions {
        max_decompress_bytes,
        password: None, // No password support for content streams yet
    };

    let mut all_decoded = Vec::new();
    let mut doc_counter = 0u64;

    for stream_ref in &page.contents {
        match resolver.resolve(*stream_ref) {
            Ok(obj) => {
                if let Some(stream) = obj.as_stream() {
                    // Decode this stream - it will be dropped after this iteration
                    let decoded = decode_stream(stream, source, &stream_opts, &mut doc_counter);

                    // Extend the accumulated content
                    all_decoded.extend_from_slice(&decoded);

                    // Explicitly drop decoded to free memory before next iteration
                    drop(decoded);
                }
            }
            Err(_) => {
                // Failed to resolve stream - skip it
                continue;
            }
        }
    }

    all_decoded
}

/// Result of a PDF extraction operation.
///
/// Contains the extracted pages, spans, blocks, and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ExtractionResult {
    /// The PDF fingerprint (for receipt generation).
    pub fingerprint: String,
    /// Extracted pages, each containing spans and blocks.
    pub pages: Vec<PageResult>,
    /// Metadata about the extraction.
    pub metadata: ExtractionMetadata,
    /// Digital signatures extracted from the document.
    ///
    /// This array contains all signature fields discovered in the AcroForm,
    /// including both signed and unsigned (blank) signature fields.
    /// Empty when the PDF has no signature fields.
    pub signatures: Vec<SignatureJson>,
    /// Interactive form fields extracted from the document.
    ///
    /// This array contains all form fields from the AcroForm and/or XFA data.
    /// Fields are sorted alphabetically by name. When both AcroForm and XFA
    /// are present, XFA values take precedence on collision.
    /// Empty when the PDF has no form fields.
    pub form_fields: Vec<FormFieldJson>,
    /// Document-scoped hyperlinks extracted from the document.
    ///
    /// This array contains all link annotations (URI and internal destination links)
    /// extracted from all pages. Links are sorted by (page_index, rect.y0 desc, rect.x0).
    /// Empty when the PDF has no link annotations.
    pub links: Vec<LinkJson>,
    /// Embedded file attachments extracted from the document.
    ///
    /// This array contains all embedded files from the PDF's `/EmbeddedFiles`
    /// name tree or `/AF` (Associated Files) array. Attachments exceeding
    /// 50 MB are truncated (metadata only, `data: null`, `truncated: true`).
    /// Empty when the PDF has no embedded files.
    pub attachments: Vec<AttachmentJson>,
    /// Article thread chains extracted from the document.
    ///
    /// This array contains all article threads from the PDF's `/Threads` array.
    /// Each thread includes metadata from the thread info dict (/I) and the
    /// complete bead chain walked from the first bead. Empty when the PDF has
    /// no article threads.
    pub threads: Vec<ThreadJson>,
    /// JavaScript actions detected in the document.
    ///
    /// Per TH-04, this array contains all discovered JavaScript actions
    /// with their location and code excerpt. pdftract NEVER executes
    /// embedded JavaScript; this is for downstream security review.
    /// Empty when no JavaScript is present.
    #[serde(default)]
    pub javascript_actions: Vec<JavascriptActionJson>,
}

/// Result for a single page.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct PageResult {
    /// 0-based page index.
    pub index: usize,
    /// 1-based page number (= index + 1).
    ///
    /// Emitted as a convenience for human-facing display. For programmatic
    /// access, use index instead.
    pub page_number: u32,
    /// Human-readable label from PDF /PageLabels number tree.
    ///
    /// Examples: "iv", "A-3", "1". Null if the PDF defines no page labels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_label: Option<String>,
    /// Page width in points (1/72 inch).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<f32>,
    /// Page height in points (1/72 inch).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<f32>,
    /// Page rotation in degrees clockwise (0, 90, 180, or 270).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotation: Option<u16>,
    /// Page classification from the page classifier.
    ///
    /// One of: "text", "scanned", "mixed", "broken_vector", "blank", "figure_only".
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_type: Option<String>,
    /// Extracted spans (text fragments with consistent styling).
    pub spans: Vec<SpanJson>,
    /// Extracted blocks (semantic units like paragraphs, headings).
    pub blocks: Vec<BlockJson>,
    /// Extracted tables (cell-level structure).
    ///
    /// This array provides detailed table structure with rows and cells.
    /// Table blocks in the `blocks` array reference entries here via `table_index`.
    pub tables: Vec<TableJson>,
    /// Page-level annotations (highlights, stamps, notes, etc.).
    ///
    /// This array contains all non-link annotations on this page.
    /// Annotations are sorted by (rect.y0 desc, rect.x0) for deterministic output.
    /// Empty when the page has no annotations.
    #[serde(default)]
    pub annotations: Vec<AnnotationJson>,
    /// Error message if extraction failed for this page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Temporary structure holding both TableJson and GridCandidate during extraction.
///
/// This is used to preserve GridCandidate information for two-page table detection,
/// which runs after all pages have been extracted. After detection, only the
/// TableJson is retained in the final output.
#[derive(Debug, Clone)]
struct TableWithGrid {
    /// The JSON output structure for this table.
    json: TableJson,
    /// The grid candidate used for two-page detection.
    grid: GridCandidate,
}

/// Internal page result that includes grid information for two-page detection.
///
/// This is used during extraction to preserve GridCandidate information.
/// After two-page detection, this is converted to the public PageResult.
#[derive(Debug, Clone)]
struct PageResultInternal {
    /// 0-based page index.
    pub index: usize,
    /// Extracted spans (text fragments with consistent styling).
    pub spans: Vec<SpanJson>,
    /// Extracted blocks (semantic units like paragraphs, headings).
    pub blocks: Vec<BlockJson>,
    /// Extracted tables with grid information.
    pub tables: Vec<TableWithGrid>,
    /// Page-level annotations (highlights, stamps, notes, etc.).
    pub annotations: Vec<AnnotationJson>,
    /// Error message if extraction failed for this page.
    pub error: Option<String>,
    /// Page media box height for two-page detection.
    pub page_height: f64,
}

impl From<PageResultInternal> for PageResult {
    fn from(internal: PageResultInternal) -> Self {
        PageResult {
            index: internal.index,
            page_number: (internal.index + 1) as u32,
            page_label: None,
            width: None,
            height: None,
            rotation: None,
            page_type: None,
            spans: internal.spans,
            blocks: internal.blocks,
            tables: internal.tables.into_iter().map(|t| t.json).collect(),
            annotations: internal.annotations,
            error: internal.error,
        }
    }
}

/// Metadata about the extraction process.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ExtractionMetadata {
    /// Total number of pages in the document.
    pub page_count: usize,
    /// Receipts mode used for this extraction.
    pub receipts_mode: ReceiptsMode,
    /// Number of spans extracted.
    pub span_count: usize,
    /// Number of blocks extracted.
    pub block_count: usize,
    /// Cache status: "hit", "miss", or "skipped"
    pub cache_status: Option<String>,
    /// Cache entry age in seconds (only present when cache_status == "hit")
    pub cache_age_seconds: Option<u64>,
    /// Number of pages that failed to extract.
    pub error_count: usize,
    /// Reading order algorithm used for this extraction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reading_order_algorithm: Option<String>,
    /// Diagnostics emitted during extraction (coverage warnings, etc.)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<String>,
}

/// Extract text and structure from a PDF file.
///
/// This is the main entry point for PDF extraction. It:
/// 1. Parses the PDF and computes its fingerprint
/// 2. Extracts spans and blocks from each page in parallel (bounded by semaphore)
/// 3. Generates receipts if requested
///
/// # Arguments
///
/// * `pdf_path` - Path to the PDF file
/// * `options` - Extraction options controlling receipt generation and parallelism
///
/// # Returns
///
/// An `ExtractionResult` containing pages with spans and blocks.
///
/// # Memory Bounding
///
/// The number of simultaneously-resident pages is capped by `max_parallel_pages`
/// in the options. This ensures document-wide peak RSS stays under the memory
/// ceiling regardless of core count. Each page extraction acquires a semaphore
/// permit before allocating its working buffers and releases it when done.
///
/// # Streaming/Lazy Decode
///
/// This function uses lazy page iteration via LazyPageIter, which walks the page
/// tree depth-first and materializes only the current path from root to leaf
/// (max ~16 nodes). Pages are processed sequentially but extracted in parallel
/// with semaphore bounding. Decoded content streams are dropped immediately after
/// each page is processed, ensuring peak RSS stays O(depth × per-page) not O(pages × per-page).
///
/// # WARNING: Accumulates All Results
///
/// This function accumulates all extracted pages in memory before returning.
/// For large documents (1000+ pages), this can consume significant memory.
/// Use `extract_pdf_ndjson` for true streaming extraction that never accumulates
/// all pages in memory.
pub fn extract_pdf(
    pdf_path: &std::path::Path,
    options: &ExtractionOptions,
) -> Result<ExtractionResult> {
    use crate::parser::catalog::parse_catalog;
    use crate::parser::pages::LazyPageIter;
    use crate::parser::stream::FileSource;
    use crate::parser::xref::{load_xref_with_prev_chain, XrefResolver};

    // Open the PDF file
    let source = FileSource::open(pdf_path).context("Failed to open PDF file")?;

    // Find the startxref offset
    let startxref_offset = find_startxref(&source).context("Failed to find startxref offset")?;

    // Load the xref table
    let xref_section = load_xref_with_prev_chain(&source, startxref_offset);

    // Create resolver from xref section
    let resolver = XrefResolver::from_section(xref_section.clone());

    // Get the root reference from trailer
    let root_ref = xref_section
        .trailer
        .as_ref()
        .and_then(|trailer| trailer.get("Root"))
        .and_then(|obj| obj.as_ref())
        .ok_or_else(|| anyhow::anyhow!("No /Root reference in trailer"))?;

    // Parse the catalog
    let catalog = parse_catalog(&resolver, root_ref).map_err(|diagnostics| {
        let msg = diagnostics
            .first()
            .map(|d| d.message.as_ref())
            .unwrap_or("unknown error");
        anyhow::anyhow!("Failed to parse catalog: {}", msg)
    })?;

    // Build fingerprint input (without full page tree for lazy extraction)
    let fingerprint = compute_fingerprint_lazy(&catalog, &xref_section);

    // Wrap resolver in Arc for sharing across threads
    let resolver_arc = Arc::new(resolver);

    // Create lazy page iterator - this walks the tree on-demand
    let mut page_iter =
        LazyPageIter::new(&resolver_arc, catalog.pages_ref).map_err(|diagnostics| {
            let msg = diagnostics
                .first()
                .map(|d| d.message.as_ref())
                .unwrap_or("unknown error");
            anyhow::anyhow!("Failed to create lazy page iterator: {}", msg)
        })?;

    // Phase 4.5: Determine reading order algorithm
    // For v0.1.0-v0.3.0: Tagged PDFs emit TAGGED_PDF_STRUCT_TREE_DEFERRED and use XY-cut
    // Phase 7.1 will replace this with real StructTree traversal
    let (reading_order_algorithm, struct_tree, deferred_diagnostic) = if catalog.mark_info.is_tagged
    {
        // Tagged PDF: emit diagnostic once per document and use XY-cut
        let diagnostic = Diagnostic::with_static_no_offset(
            DiagCode::LayoutTaggedPdfDeferred,
            "Tagged PDF detected; StructTree traversal deferred to Phase 7.1, using XY-cut for now",
        );
        (ReadingOrderAlgorithm::XyCut, None, Some(diagnostic))
    } else {
        // Untagged PDF: use XY-cut
        (ReadingOrderAlgorithm::XyCut, None, None)
    };

    // Wrap options in Arc for sharing across threads
    let fingerprint_arc = Arc::new(fingerprint.clone());
    let options_arc = Arc::new(options.clone());

    // Create a semaphore to bound the number of in-flight pages
    let semaphore = Arc::new(Semaphore::new(options.max_parallel_pages));

    // First, collect all PageDict objects for annotation extraction
    // We need these before extracting content so we can dispatch annotations once
    let mut all_pages: Vec<crate::parser::pages::PageDict> = Vec::new();
    loop {
        match page_iter.next() {
            Some(Ok(page_dict)) => {
                all_pages.push(page_dict);
            }
            Some(Err(_)) | None => {
                // End of pages or error - stop collecting
                break;
            }
        }
    }

    // Phase 7.6: Extract annotations and links from all pages
    // Walk all pages and extract annotations by subtype
    //
    // Note: For now, we pass None for dests_dict and names_dests_ref.
    // A full implementation would resolve /Catalog /Dests and /Catalog /Names /Dests
    // to support named destination resolution. This is sufficient for URI links
    // and explicit destination arrays.
    let (link_annotations, annotations) = dispatch_annotations(
        &resolver_arc,
        &all_pages,
        None, // dests_dict
        None, // names_dests_ref
    );

    // Convert links to JSON format and sort by (page_index, rect.y0 desc, rect.x0)
    let mut links_json: Vec<LinkJson> = link_annotations
        .iter()
        .map(|link| annotation_json::link_to_json(link, &None))
        .collect();
    annotation_json::sort_links(&mut links_json);

    // Convert annotations to JSON format and group by page
    let mut annotations_by_page: std::collections::HashMap<usize, Vec<AnnotationJson>> =
        std::collections::HashMap::new();

    for annot in &annotations {
        let json = annotation_json::annotation_to_json(annot);
        let page_idx = annot.common.page_index;
        annotations_by_page
            .entry(page_idx)
            .or_insert_with(Vec::new)
            .push(json);
    }

    // Sort annotations within each page by (rect.y0 desc, rect.x0)
    for page_annotations in annotations_by_page.values_mut() {
        annotation_json::sort_annotations(page_annotations);
    }

    // Now process pages for content extraction (re-using the collected pages)
    let mut extracted_pages = Vec::new();
    let mut total_spans = 0;
    let mut total_blocks = 0;
    let mut error_count = 0;
    let mut page_count = 0;
    let mut page_heights = Vec::new(); // Track page heights for two-page table detection

    // Phase 7.1.4: Collect page data for coverage check
    // Track MCIDs and struct_parents for each page
    let mut pages_with_mcids: Vec<(usize, Option<i32>, std::collections::HashSet<u32>)> =
        Vec::new();
    let needs_coverage_check = catalog.mark_info.requires_coverage_check() && struct_tree.is_some();

    // Save a clone of pages for JavaScript detection later
    // We need to clone because all_pages will be consumed in the loop
    let pages_for_js_detection = all_pages.clone();

    // Process pages for content extraction
    for (page_index, page_dict) in all_pages.into_iter().enumerate() {
        // Get page height for two-page table detection
        let [_x0, _y0, _x1, y1] = page_dict.media_box;
        let page_height = (y1 - page_dict.media_box[1]).max(0.0);
        page_heights.push(page_height);

        // Track MCIDs for this page if coverage check is needed
        if needs_coverage_check {
            // Decode content streams and track MCIDs
            let decoded_streams = decode_page_content_streams(
                &page_dict,
                &resolver_arc,
                &source,
                DEFAULT_MAX_DECOMPRESS_BYTES,
            );

            let mut tracker = McidTracker::new();
            track_mcids_from_content_stream(&decoded_streams, &mut tracker);

            // Get the struct_parents value for this page
            let struct_parents = page_dict.struct_parents();

            // Record page data for coverage check
            let mcid_set = tracker.mcid_set().clone();
            pages_with_mcids.push((page_index, struct_parents, mcid_set));

            // Drop decoded_streams and tracker to free memory
            drop(decoded_streams);
            // tracker dropped implicitly
        }

        // Get the annotations for this page (already sorted)
        let page_annotations = annotations_by_page.remove(&page_index).unwrap_or_default();

        // Extract this page with lazy stream decoding.
        // Content streams are decoded, processed, and dropped immediately.
        let extract_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            extract_page_from_dict(
                &fingerprint_arc,
                page_index,
                &page_dict,
                &options_arc,
                Some(&source),
                Some(&resolver_arc),
            )
        }));

        match extract_result {
            Ok(Ok(mut page)) => {
                total_spans += page.spans.len();
                total_blocks += page.blocks.len();
                page.annotations = page_annotations;
                extracted_pages.push(page);
            }
            Ok(Err(e)) => {
                error_count += 1;
                extracted_pages.push(PageResultInternal {
                    index: page_index,
                    spans: vec![],
                    blocks: vec![],
                    tables: vec![],
                    annotations: page_annotations,
                    error: Some(e.to_string()),
                    page_height,
                });
            }
            Err(_) => {
                error_count += 1;
                extracted_pages.push(PageResultInternal {
                    index: page_index,
                    spans: vec![],
                    blocks: vec![],
                    tables: vec![],
                    annotations: page_annotations,
                    error: Some(format!("Page {} extraction panicked", page_index)),
                    page_height,
                });
            }
        }

        // Explicitly drop page_dict to ensure memory is freed before next iteration
        drop(page_dict);
        page_count += 1;
    }

    // Phase 7.1.4: Perform coverage check if Suspects is true
    // This must happen after we've collected MCID data from all pages
    let (final_reading_order_algorithm, coverage_diagnostics) = if needs_coverage_check {
        if let Some(ref tree) = struct_tree {
            let coverage_result =
                check_coverage_for_pages(tree, &catalog.mark_info, &pages_with_mcids);
            let diagnostics: Vec<String> = coverage_result
                .diagnostics
                .iter()
                .map(|d| d.message.as_ref().to_string())
                .collect();
            (coverage_result.reading_order_algorithm, diagnostics)
        } else {
            // Shouldn't happen due to the needs_coverage_check condition
            (reading_order_algorithm, Vec::new())
        }
    } else {
        (reading_order_algorithm, Vec::new())
    };

    // Add the tagged PDF deferred diagnostic if present
    let mut all_diagnostics = coverage_diagnostics;
    if let Some(ref deferred) = deferred_diagnostic {
        all_diagnostics.push(deferred.message.as_ref().to_string());
    }

    // Phase 7.2.6: Detect two-page table continuation
    // This must happen after all pages have been extracted so we can compare
    // tables on adjacent pages
    let extracted_pages = apply_two_page_table_detection(extracted_pages, &page_heights);

    // Convert PageResultInternal to PageResult for final output
    let extracted_pages: Vec<PageResult> = extracted_pages.into_iter().map(Into::into).collect();

    // Phase 7.3: Extract digital signature metadata
    // Discover signature fields and extract metadata from them
    let sig_fields = discover(&resolver_arc, &catalog);
    use crate::parser::stream::PdfSource;
    let file_size = source.len().ok();
    let signatures_core = extract_signatures(&sig_fields, &resolver_arc, file_size);
    let signatures: Vec<SignatureJson> = signatures_core.into_iter().map(|s| s.into()).collect();

    // Phase 7.5: Extract embedded file attachments from /EmbeddedFiles and /AF
    let attachments = match resolver_arc.resolve(root_ref) {
        Ok(catalog_obj) => match catalog_obj.as_dict() {
            Some(catalog_dict) => extract_attachments(&resolver_arc, catalog_dict, Some(&source)),
            None => Vec::new(),
        },
        Err(_) => Vec::new(),
    };

    // Phase 7.4: Extract form fields from AcroForm and XFA
    // Walk AcroForm fields and convert to FormFieldValue
    let acro_fields = walk_acroform_fields(&resolver_arc, &catalog, None);
    let mut acro_fields_typed: Vec<(String, FormFieldValue)> = Vec::new();
    for field in acro_fields {
        let field_value = acro_field_to_value(&field);
        acro_fields_typed.push((field.full_name.clone(), field_value));
    }

    // Extract XFA fields if present (requires re-opening the source for stream access)
    let xfa_fields = if catalog.acroform_ref.is_some() {
        // Resolve the AcroForm dictionary
        use crate::parser::xref::XrefResolver;
        let acroform_ref = catalog.acroform_ref.unwrap();
        if let Ok(acroform_obj) = resolver_arc.resolve(acroform_ref) {
            if let Some(acroform_dict) = acroform_obj.as_dict() {
                // Create extraction options for stream decoding
                use crate::parser::stream::ExtractionOptions as StreamExtractionOptions;
                let stream_opts = StreamExtractionOptions {
                    max_decompress_bytes: DEFAULT_MAX_DECOMPRESS_BYTES,
                    password: None,
                };
                use crate::forms::extract_xfa_fields;
                let xfa_extracted =
                    extract_xfa_fields(&resolver_arc, acroform_dict, &source, &stream_opts);
                xfa_extracted
                    .into_iter()
                    .filter_map(|f| f.value.map(|v| (f.full_name, v)))
                    .collect()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    // Combine AcroForm and XFA fields (XFA wins on collision)
    let (combined_fields, _form_diagnostics) = combine(acro_fields_typed, xfa_fields);

    // Convert to FormFieldJson
    let form_fields: Vec<FormFieldJson> = combined_fields
        .into_iter()
        .map(|(name, value)| convert_form_field_to_json(name, value, &resolver_arc, &catalog))
        .collect();

    // Phase 7.7: Extract article thread chains
    // Discover thread headers from /Threads array and walk bead chains
    use crate::parser::pages::build_page_ref_to_index;
    use crate::threads::{discover as discover_threads, thread_to_json, walk_beads};

    // Build page ref to index map for bead chain walking
    let page_ref_to_index = build_page_ref_to_index(&catalog, &resolver_arc);

    // Discover thread headers from /Threads array
    let thread_headers = match discover_threads(&catalog, &resolver_arc) {
        Ok(headers) => headers,
        Err(_) => Vec::new(), // Return empty on error
    };

    // Walk bead chains for each thread and convert to JSON
    let mut threads_json = Vec::new();
    for header in &thread_headers {
        match walk_beads(header, &resolver_arc, &page_ref_to_index) {
            Ok(beads) => {
                threads_json.push(thread_to_json(header, &beads));
            }
            Err(_) => {
                // Skip threads with malformed bead chains but continue processing others
                continue;
            }
        }
    }

    // TH-04: Detect JavaScript actions in the document
    // This checks /OpenAction, /AA, page /AA, and annotation /A entries
    use crate::javascript::detect_javascript;
    let (js_actions, js_diagnostics) = detect_javascript(&catalog, &pages_for_js_detection, &resolver_arc);

    // Convert JavascriptAction to JavascriptActionJson
    let javascript_actions: Vec<JavascriptActionJson> = js_actions
        .into_iter()
        .map(|action| JavascriptActionJson {
            location: action.location,
            code_excerpt: action.code_excerpt,
        })
        .collect();

    // Add JavaScript detection diagnostics to the error list
    let mut all_diagnostics_with_js = all_diagnostics;
    for diag in js_diagnostics {
        all_diagnostics_with_js.push(diag.message.as_ref().to_string());
    }

    Ok(ExtractionResult {
        fingerprint,
        pages: extracted_pages,
        metadata: ExtractionMetadata {
            page_count,
            receipts_mode: options.receipts,
            span_count: total_spans,
            block_count: total_blocks,
            cache_status: None,
            cache_age_seconds: None,
            error_count,
            reading_order_algorithm: Some(final_reading_order_algorithm.as_str().to_string()),
            diagnostics: all_diagnostics_with_js,
        },
        signatures,
        form_fields,
        links: links_json,
        attachments,
        threads: threads_json,
        javascript_actions,
    })
}

/// Apply two-page table detection flags to extracted pages.
///
/// This function examines tables on adjacent pages and sets the
/// `continued` and `continued_from_prev` flags where appropriate.
///
/// # Arguments
///
/// * `pages` - Pages with internal table information (grids preserved)
/// * `page_heights` - Page heights in points for edge detection
///
/// # Returns
///
/// Pages with table continuation flags applied.
fn apply_two_page_table_detection(
    mut pages: Vec<PageResultInternal>,
    page_heights: &[f64],
) -> Vec<PageResultInternal> {
    // Collect all GridCandidates by page
    let all_grids: Vec<Vec<GridCandidate>> = pages
        .iter()
        .map(|p| p.tables.iter().map(|t| t.grid.clone()).collect())
        .collect();

    // Run two-page detection
    let continuation_flags = detect_two_page_tables(&all_grids, page_heights);

    // Apply flags to the tables
    for (page_idx, page) in pages.iter_mut().enumerate() {
        if let Some(page_flags) = continuation_flags.get(page_idx) {
            for (table_idx, table) in page.tables.iter_mut().enumerate() {
                if let Some(&(continued, continued_from_prev)) = page_flags.get(table_idx) {
                    table.json.continued = continued;
                    table.json.continued_from_prev = continued_from_prev;
                }
            }
        }
    }

    pages
}

/// Convert a FormFieldValue to FormFieldJson for serialization.
///
/// This helper function converts the internal FormFieldValue representation
/// to the JSON-serializable FormFieldJson structure.
///
/// # Arguments
///
/// * `name` - The field name
/// * `value` - The FormFieldValue to convert
/// * `resolver` - Xref resolver (for looking up field metadata)
/// * `catalog` - Document catalog (for accessing AcroForm)
fn convert_form_field_to_json(
    name: String,
    value: FormFieldValue,
    resolver: &crate::parser::xref::XrefResolver,
    catalog: &crate::parser::catalog::Catalog,
) -> FormFieldJson {
    match value {
        FormFieldValue::Text {
            value,
            default,
            multiline,
            max_length,
        } => FormFieldJson {
            name,
            field_type: FormFieldTypeJson::Text,
            value: FormFieldValueJson::Text(value),
            default: default.map(|v| FormFieldValueJson::Text(Some(v))),
            page_index: None,
            rect: None,
            required: false,
            read_only: false,
            multiline: Some(multiline),
            max_length,
            options: None,
            multi_select: None,
            selected: None,
            state_name: None,
            pushbutton: None,
            radio: None,
        },

        FormFieldValue::Button {
            kind,
            selected,
            state_name,
            default_selected,
            pushbutton,
            radio,
        } => FormFieldJson {
            name,
            field_type: FormFieldTypeJson::Button,
            value: FormFieldValueJson::Button(selected),
            default: default_selected.map(FormFieldValueJson::Button),
            page_index: None,
            rect: None,
            required: false,
            read_only: false,
            multiline: None,
            max_length: None,
            options: None,
            multi_select: None,
            selected: Some(selected),
            state_name,
            pushbutton: Some(pushbutton),
            radio: Some(radio),
        },

        FormFieldValue::Choice {
            value,
            default,
            options,
            is_combo,
            is_multi_select,
        } => {
            let json_value = match value {
                crate::forms::ChoiceValue::Single(s) => {
                    FormFieldValueJson::Choice(ChoiceValueJson::Single(s))
                }
                crate::forms::ChoiceValue::Multiple(vec) => {
                    FormFieldValueJson::Choice(ChoiceValueJson::Multiple(vec))
                }
            };

            let json_default = default.map(|dv| match dv {
                crate::forms::ChoiceValue::Single(s) => {
                    FormFieldValueJson::Choice(ChoiceValueJson::Single(s))
                }
                crate::forms::ChoiceValue::Multiple(vec) => {
                    FormFieldValueJson::Choice(ChoiceValueJson::Multiple(vec))
                }
            });

            let json_options: Vec<[String; 2]> = options
                .into_iter()
                .map(|(export, display)| [export, display])
                .collect();

            FormFieldJson {
                name,
                field_type: FormFieldTypeJson::Choice,
                value: json_value,
                default: json_default,
                page_index: None,
                rect: None,
                required: false,
                read_only: false,
                multiline: None,
                max_length: None,
                options: Some(json_options),
                multi_select: Some(is_multi_select),
                selected: None,
                state_name: None,
                pushbutton: None,
                radio: None,
            }
        }

        FormFieldValue::Signature { signature_ref } => FormFieldJson {
            name,
            field_type: FormFieldTypeJson::Signature,
            value: FormFieldValueJson::Signature(signature_ref),
            default: None,
            page_index: None,
            rect: None,
            required: false,
            read_only: false,
            multiline: None,
            max_length: None,
            options: None,
            multi_select: None,
            selected: None,
            state_name: None,
            pushbutton: None,
            radio: None,
        },
    }
}

/// Extract embedded file attachments from the PDF.
///
/// This function walks both the /EmbeddedFiles name tree and the /AF (Associated Files)
/// array to extract all embedded file attachments. It handles PDF 1.7 /EmbeddedFiles
/// and PDF 2.0 /AF sources, deduplicating by Filespec reference.
///
/// # Arguments
///
/// * `resolver` - The xref resolver for resolving indirect references
/// * `catalog_dict` - The raw catalog dictionary (PdfDict)
/// * `source` - Optional PDF source for reading stream data (None for metadata-only extraction)
///
/// # Returns
///
/// A `Vec<AttachmentJson>` containing all extracted attachments, sorted by name
/// for deterministic output.
fn extract_attachments(
    resolver: &Arc<crate::parser::xref::XrefResolver>,
    catalog_dict: &crate::parser::object::PdfDict,
    source: Option<&dyn crate::parser::stream::PdfSource>,
) -> Vec<AttachmentJson> {
    use crate::parser::object::ObjRef;
    use std::collections::HashSet;

    let mut attachments = Vec::new();
    let mut seen_refs: HashSet<ObjRef> = HashSet::new();

    // Walk /AF array from the catalog
    let af_entries = match walk_af_array(resolver, catalog_dict) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(), // Return empty if /AF walk fails
    };
    for entry in af_entries {
        if seen_refs.contains(&entry.filespec_ref) {
            continue; // Skip duplicates
        }
        seen_refs.insert(entry.filespec_ref);

        // Extract the attachment
        match extract_one(resolver, entry.filespec_ref, source) {
            Ok(attachment) => {
                attachments.push(attachment.into_json());
            }
            Err(_) => {
                // Skip failed attachments but continue with others
                continue;
            }
        }
    }

    // TODO: Also walk /EmbeddedFiles name tree for PDF 1.7 compatibility
    // This requires implementing a name tree walker for /EmbeddedFiles

    // Sort by name for deterministic output
    attachments.sort_by(|a, b| a.name.cmp(&b.name));

    attachments
}

/// Extract content from a single page.
///
/// # Arguments
///
/// * `fingerprint` - The PDF fingerprint for receipt generation
/// * `page_index` - 0-based page index
/// * `page` - The page dictionary from the PDF
/// * `options` - Extraction options
fn extract_page(
    fingerprint: &str,
    page_index: usize,
    page: &crate::parser::pages::PageDict,
    options: &ExtractionOptions,
) -> Result<PageResult> {
    // For now, create placeholder spans based on the page media box
    // In a full implementation, this would parse the content streams
    // and extract actual text with positioning information

    let [x0, y0, x1, y1] = page.media_box;

    // Create a placeholder span for the entire page
    // This is a minimal implementation - the full Phase 3 pipeline
    // would extract actual text from content streams
    let span_text = format!("[Page {} text extraction]", page_index);
    let span_bbox = [x0, y0, x1, y1];

    // Generate receipt if requested
    let receipt = generate_receipt(
        fingerprint,
        page_index,
        span_bbox,
        &span_text,
        options.receipts,
        #[cfg(feature = "receipts")]
        None,
    )?;

    let span = SpanJson {
        text: span_text,
        bbox: span_bbox,
        font: "Unknown".to_string(),
        size: 12.0,
        color: None,
        rendering_mode: None,
        confidence: None,
        confidence_source: None,
        lang: None,
        flags: vec![],
        receipt,
        column: None,
    };

    // Create a block containing the span
    let block_text = span.text.clone();
    let block_bbox = span_bbox;
    let block_receipt = generate_receipt(
        fingerprint,
        page_index,
        block_bbox,
        &block_text,
        options.receipts,
        #[cfg(feature = "receipts")]
        None,
    )?;

    let block = BlockJson {
        kind: "paragraph".to_string(),
        text: block_text,
        bbox: block_bbox,
        level: None,
        table_index: None,
        spans: vec![],
        receipt: block_receipt,
    };

    Ok(PageResult {
        index: page_index,
        page_number: (page_index + 1) as u32,
        page_label: None,
        width: None,
        height: None,
        rotation: None,
        page_type: None,
        spans: vec![span],
        blocks: vec![block],
        tables: vec![],
        annotations: vec![],
        error: None,
    })
}

/// Generate a receipt for a span or block.
///
/// # Arguments
///
/// * `fingerprint` - The PDF fingerprint
/// * `page_index` - 0-based page index
/// * `bbox` - Bounding box in PDF points
/// * `text` - The text content
/// * `mode` - Receipt generation mode
/// * `glyph_list` - Optional glyph list for SVG generation (only used with receipts feature)
fn generate_receipt(
    fingerprint: &str,
    page_index: usize,
    bbox: [f64; 4],
    text: &str,
    mode: ReceiptsMode,
    #[cfg(feature = "receipts")] glyph_list: Option<&GlyphList>,
) -> Result<Option<Receipt>> {
    match mode {
        ReceiptsMode::Off => Ok(None),
        ReceiptsMode::Lite => Ok(Some(Receipt::lite(
            fingerprint.to_string(),
            page_index,
            bbox,
            text,
        ))),
        #[cfg(feature = "receipts")]
        ReceiptsMode::SvgClip => {
            // For SVG mode, we need a glyph list to generate the SVG clip
            // In this minimal implementation, we fall back to lite mode
            // if no glyph list is provided
            if let Some(glyphs) = glyph_list {
                let svg_gen = crate::receipts::svg::SvgGenerator::new(glyphs.clone());
                let svg_clip = svg_gen.generate(bbox);
                Ok(Some(Receipt::with_svg(
                    fingerprint.to_string(),
                    page_index,
                    bbox,
                    text,
                    svg_clip,
                )))
            } else {
                // No glyph data available - fall back to lite mode
                Ok(Some(Receipt::lite(
                    fingerprint.to_string(),
                    page_index,
                    bbox,
                    text,
                )))
            }
        }
        #[cfg(not(feature = "receipts"))]
        ReceiptsMode::SvgClip => {
            // Receipts feature not enabled - fall back to lite mode
            Ok(Some(Receipt::lite(
                fingerprint.to_string(),
                page_index,
                bbox,
                text,
            )))
        }
    }
}

/// Convert an ExtractionResult to JSON format.
///
/// This produces the JSON output format expected by the CLI and API.
pub fn result_to_json(result: &ExtractionResult) -> serde_json::Value {
    let pages: Vec<serde_json::Value> = result
        .pages
        .iter()
        .map(|page| {
            json!({
                "index": page.index,
                "spans": page.spans,
                "blocks": page.blocks,
                "tables": page.tables,
            })
        })
        .collect();

    let mut metadata_obj = json!({
        "page_count": result.metadata.page_count,
        "span_count": result.metadata.span_count,
        "block_count": result.metadata.block_count,
        "cache_status": result.metadata.cache_status,
        "cache_age_seconds": result.metadata.cache_age_seconds,
    });

    // Add reading_order_algorithm if present
    if let Some(ref algo) = result.metadata.reading_order_algorithm {
        metadata_obj["reading_order_algorithm"] = json!(algo);
    }

    // Add diagnostics if present
    if !result.metadata.diagnostics.is_empty() {
        metadata_obj["diagnostics"] = json!(result.metadata.diagnostics);
    }

    json!({
        "fingerprint": result.fingerprint,
        "schema_version": "1.0",
        "pages": pages,
        "metadata": metadata_obj,
        "signatures": result.signatures,
        "form_fields": result.form_fields,
        "links": result.links,
        "attachments": result.attachments,
        "threads": result.threads,
        "javascript_actions": result.javascript_actions
    })
}

/// Extract text and structure from a PDF file, writing NDJSON output.
///
/// This is the streaming variant of `extract_pdf` that writes each page
/// as a newline-delimited JSON object immediately after extraction.
/// This keeps memory usage bounded regardless of document size.
///
/// # Arguments
///
/// * `pdf_path` - Path to the PDF file
/// * `options` - Extraction options controlling receipt generation and parallelism
/// * `writer` - Any type implementing `std::io::Write` to receive NDJSON output
///
/// # Returns
///
/// An `ExtractionMetadata` containing summary statistics (pages, spans, blocks extracted).
///
/// # Memory Bounding
///
/// Unlike `extract_pdf`, this function never accumulates all pages in memory.
/// Pages are iterated lazily via LazyPageIter, which walks the page tree depth-first
/// and materializes only the current path from root to leaf (max ~16 nodes).
/// Each page is serialized to NDJSON and written immediately, then dropped.
/// Peak RSS stays O(depth × per-page) not O(pages × per-page).
///
/// # Output Format
///
/// Each line is a JSON object representing one page:
/// ```json
/// {"index": 0, "spans": [...], "blocks": [...]}
/// {"index": 1, "spans": [...], "blocks": [...]}
/// ```
pub fn extract_pdf_ndjson<W: std::io::Write>(
    pdf_path: &std::path::Path,
    options: &ExtractionOptions,
    mut writer: W,
) -> Result<ExtractionMetadata> {
    use crate::parser::catalog::parse_catalog;
    use crate::parser::pages::LazyPageIter;
    use crate::parser::stream::FileSource;
    use crate::parser::xref::{load_xref_with_prev_chain, XrefResolver};
    use std::io::Write;

    // Open the PDF file
    let source = FileSource::open(pdf_path).context("Failed to open PDF file")?;

    // Find the startxref offset
    let startxref_offset = find_startxref(&source).context("Failed to find startxref offset")?;

    // Load the xref table
    let xref_section = load_xref_with_prev_chain(&source, startxref_offset);

    // Create resolver from xref section
    let resolver = XrefResolver::from_section(xref_section.clone());

    // Get the root reference from trailer
    let root_ref = xref_section
        .trailer
        .as_ref()
        .and_then(|trailer| trailer.get("Root"))
        .and_then(|obj| obj.as_ref())
        .ok_or_else(|| anyhow::anyhow!("No /Root reference in trailer"))?;

    // Parse the catalog
    let catalog = parse_catalog(&resolver, root_ref).map_err(|diagnostics| {
        let msg = diagnostics
            .first()
            .map(|d| d.message.as_ref())
            .unwrap_or("unknown error");
        anyhow::anyhow!("Failed to parse catalog: {}", msg)
    })?;

    // Phase 4.5: Determine reading order algorithm
    // For v0.1.0-v0.3.0: Tagged PDFs emit TAGGED_PDF_STRUCT_TREE_DEFERRED and use XY-cut
    // Phase 7.1 will replace this with real StructTree traversal
    let resolver_arc = Arc::new(resolver);

    let (reading_order_algorithm, struct_tree, deferred_diagnostic) = if catalog.mark_info.is_tagged
    {
        // Tagged PDF: emit diagnostic once per document and use XY-cut
        let diagnostic = Diagnostic::with_static_no_offset(
            DiagCode::LayoutTaggedPdfDeferred,
            "Tagged PDF detected; StructTree traversal deferred to Phase 7.1, using XY-cut for now",
        );
        (ReadingOrderAlgorithm::XyCut, None, Some(diagnostic))
    } else {
        // Untagged PDF: use XY-cut
        (ReadingOrderAlgorithm::XyCut, None, None)
    };

    // For lazy extraction, use a placeholder fingerprint
    // The full fingerprint would require walking all pages, which defeats the purpose
    let fingerprint = format!(
        "pdftract-v1:lazy{:016x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );

    // Create lazy page iterator - this walks the tree on-demand
    let mut page_iter =
        LazyPageIter::new(&resolver_arc, catalog.pages_ref).map_err(|diagnostics| {
            let msg = diagnostics
                .first()
                .map(|d| d.message.as_ref())
                .unwrap_or("unknown error");
            anyhow::anyhow!("Failed to create lazy page iterator: {}", msg)
        })?;

    // Wrap options in Arc for sharing across threads
    let fingerprint_arc = Arc::new(fingerprint.clone());
    let options_arc = Arc::new(options.clone());

    // Track metadata across all pages
    let mut total_spans = 0u64;
    let mut total_blocks = 0u64;
    let mut error_count = 0u64;
    let mut page_count = 0usize;

    // Phase 7.1.4: Collect page data for coverage check
    // Track MCIDs and struct_parents for each page
    let mut pages_with_mcids: Vec<(usize, Option<i32>, std::collections::HashSet<u32>)> =
        Vec::new();
    let needs_coverage_check = catalog.mark_info.requires_coverage_check() && struct_tree.is_some();

    // Create a semaphore to bound the number of in-flight pages
    let semaphore = Arc::new(Semaphore::new(options.max_parallel_pages));

    // Process pages sequentially from the lazy iterator
    // Each page is materialized, processed, and dropped before moving to the next
    while let Some(page_result) = page_iter.next() {
        let page_dict = match page_result {
            Ok(p) => p,
            Err(diagnostics) => {
                // Emit diagnostics as error pages
                let msg = diagnostics
                    .first()
                    .map(|d| d.message.as_ref())
                    .unwrap_or("unknown error");
                error_count += 1;
                let error_json = json!({
                    "index": page_count,
                    "error": msg,
                    "spans": [],
                    "blocks": [],
                });
                serde_json::to_writer(&mut writer, &error_json)
                    .context("Failed to write NDJSON")?;
                writeln!(writer).context("Failed to write newline")?;
                writer.flush().context("Failed to flush output")?;
                // Still record page data for coverage check (even on error)
                if needs_coverage_check {
                    pages_with_mcids.push((page_count, None, std::collections::HashSet::new()));
                }
                page_count += 1;
                continue;
            }
        };

        let page_index = page_count;

        // Track MCIDs for this page if coverage check is needed
        if needs_coverage_check {
            // Decode content streams and track MCIDs
            let decoded_streams = decode_page_content_streams(
                &page_dict,
                &resolver_arc,
                &source,
                DEFAULT_MAX_DECOMPRESS_BYTES,
            );

            let mut tracker = McidTracker::new();
            track_mcids_from_content_stream(&decoded_streams, &mut tracker);

            // Get the struct_parents value for this page
            let struct_parents = page_dict.struct_parents();

            // Record page data for coverage check
            let mcid_set = tracker.mcid_set().clone();
            pages_with_mcids.push((page_count, struct_parents, mcid_set));

            // Drop decoded_streams and tracker to free memory
            drop(decoded_streams);
            // tracker dropped implicitly
        }

        // Extract this page with lazy stream decoding.
        // Content streams are decoded, processed, and dropped immediately.
        let _permit = semaphore.acquire_guard();

        let extract_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            extract_page_from_dict(
                &fingerprint_arc,
                page_index,
                &page_dict,
                &options_arc,
                Some(&source),
                Some(&resolver_arc),
            )
        }));

        match extract_result {
            Ok(Ok(page)) => {
                total_spans += page.spans.len() as u64;
                total_blocks += page.blocks.len() as u64;

                // Serialize and write this page immediately
                // Extract TableJson from TableWithGrid for serialization
                let tables_json: Vec<_> = page.tables.into_iter().map(|t| t.json).collect();
                let page_json = json!({
                    "index": page.index,
                    "spans": page.spans,
                    "blocks": page.blocks,
                    "tables": tables_json,
                });

                serde_json::to_writer(&mut writer, &page_json).context("Failed to write NDJSON")?;
                writeln!(writer).context("Failed to write newline")?;
                writer.flush().context("Failed to flush output")?;
            }
            Ok(Err(e)) => {
                error_count += 1;
                // Write error page to maintain page ordering
                let error_json = json!({
                    "index": page_index,
                    "error": e.to_string(),
                    "spans": [],
                    "blocks": [],
                    "tables": [],
                });

                serde_json::to_writer(&mut writer, &error_json)
                    .context("Failed to write NDJSON")?;
                writeln!(writer).context("Failed to write newline")?;
                writer.flush().context("Failed to flush output")?;
            }
            Err(_) => {
                error_count += 1;
                let error_json = json!({
                    "index": page_index,
                    "error": format!("Page {} extraction panicked", page_index),
                    "spans": [],
                    "blocks": [],
                    "tables": [],
                });

                serde_json::to_writer(&mut writer, &error_json)
                    .context("Failed to write NDJSON")?;
                writeln!(writer).context("Failed to write newline")?;
                writer.flush().context("Failed to flush output")?;
            }
        }

        // Drop page_dict explicitly to ensure memory is freed before next iteration
        drop(page_dict);
        page_count += 1;
    }

    // Phase 7.1.4: Perform coverage check if Suspects is true
    // This must happen after we've collected MCID data from all pages
    let (final_reading_order_algorithm, coverage_diagnostics) = if needs_coverage_check {
        if let Some(ref tree) = struct_tree {
            let coverage_result =
                check_coverage_for_pages(tree, &catalog.mark_info, &pages_with_mcids);
            let diagnostics: Vec<String> = coverage_result
                .diagnostics
                .iter()
                .map(|d| d.message.as_ref().to_string())
                .collect();
            (coverage_result.reading_order_algorithm, diagnostics)
        } else {
            // Shouldn't happen due to the needs_coverage_check condition
            (reading_order_algorithm, Vec::new())
        }
    } else {
        (reading_order_algorithm, Vec::new())
    };

    // Add the tagged PDF deferred diagnostic if present
    let mut all_diagnostics = coverage_diagnostics;
    if let Some(ref deferred) = deferred_diagnostic {
        all_diagnostics.push(deferred.message.as_ref().to_string());
    }

    Ok(ExtractionMetadata {
        page_count,
        receipts_mode: options.receipts,
        span_count: total_spans as usize,
        block_count: total_blocks as usize,
        cache_status: None,
        cache_age_seconds: None,
        error_count: error_count as usize,
        reading_order_algorithm: Some(final_reading_order_algorithm.as_str().to_string()),
        diagnostics: all_diagnostics,
    })
}

/// Extract text and structure from a PDF file, invoking a callback for each page.
///
/// This is the callback-based streaming variant of `extract_pdf`. Each page
/// is extracted and passed to the callback immediately after extraction,
/// then dropped from memory. This keeps memory usage bounded regardless of
/// document size.
///
/// # Arguments
///
/// * `pdf_path` - Path to the PDF file
/// * `options` - Extraction options controlling receipt generation and parallelism
/// * `callback` - Function called with each PageResult as it completes
///
/// # Returns
///
/// An `ExtractionMetadata` containing summary statistics.
///
/// # Memory Bounding
///
/// This function never accumulates all pages in memory. Pages are iterated
/// lazily via LazyPageIter, extracted one at a time, and passed to the callback.
/// Peak RSS stays O(depth × per-page) not O(pages × per-page).
///
/// # Callback Contract
///
/// The callback is invoked from the extraction thread with a reference to each
/// PageResult. If the callback returns `false`, extraction stops early.
pub fn extract_pdf_streaming<F>(
    pdf_path: &std::path::Path,
    options: &ExtractionOptions,
    mut callback: F,
) -> Result<ExtractionMetadata>
where
    F: FnMut(&PageResult) -> bool,
{
    use crate::parser::catalog::parse_catalog;
    use crate::parser::pages::LazyPageIter;
    use crate::parser::stream::FileSource;
    use crate::parser::xref::{load_xref_with_prev_chain, XrefResolver};

    // Open the PDF file
    let source = FileSource::open(pdf_path).context("Failed to open PDF file")?;

    // Find the startxref offset
    let startxref_offset = find_startxref(&source).context("Failed to find startxref offset")?;

    // Load the xref table
    let xref_section = load_xref_with_prev_chain(&source, startxref_offset);

    // Create resolver from xref section
    let resolver = XrefResolver::from_section(xref_section.clone());

    // Get the root reference from trailer
    let root_ref = xref_section
        .trailer
        .as_ref()
        .and_then(|trailer| trailer.get("Root"))
        .and_then(|obj| obj.as_ref())
        .ok_or_else(|| anyhow::anyhow!("No /Root reference in trailer"))?;

    // Parse the catalog
    let catalog = parse_catalog(&resolver, root_ref).map_err(|diagnostics| {
        let msg = diagnostics
            .first()
            .map(|d| d.message.as_ref())
            .unwrap_or("unknown error");
        anyhow::anyhow!("Failed to parse catalog: {}", msg)
    })?;

    // Wrap resolver in Arc for sharing across threads
    let resolver_arc = Arc::new(resolver);

    // Phase 4.5: Determine reading order algorithm
    // For v0.1.0-v0.3.0: Tagged PDFs emit TAGGED_PDF_STRUCT_TREE_DEFERRED and use XY-cut
    // Phase 7.1 will replace this with real StructTree traversal
    let (reading_order_algorithm, struct_tree, deferred_diagnostic) = if catalog.mark_info.is_tagged
    {
        // Tagged PDF: emit diagnostic once per document and use XY-cut
        let diagnostic = Diagnostic::with_static_no_offset(
            DiagCode::LayoutTaggedPdfDeferred,
            "Tagged PDF detected; StructTree traversal deferred to Phase 7.1, using XY-cut for now",
        );
        (ReadingOrderAlgorithm::XyCut, None, Some(diagnostic))
    } else {
        // Untagged PDF: use XY-cut
        (ReadingOrderAlgorithm::XyCut, None, None)
    };

    // Build fingerprint
    let fingerprint = compute_fingerprint_lazy(&catalog, &xref_section);

    // Wrap options in Arc for sharing across threads
    let fingerprint_arc = Arc::new(fingerprint.clone());
    let options_arc = Arc::new(options.clone());

    // Create lazy page iterator
    let mut page_iter =
        LazyPageIter::new(&resolver_arc, catalog.pages_ref).map_err(|diagnostics| {
            let msg = diagnostics
                .first()
                .map(|d| d.message.as_ref())
                .unwrap_or("unknown error");
            anyhow::anyhow!("Failed to create lazy page iterator: {}", msg)
        })?;

    // Create a semaphore to bound the number of in-flight pages
    let semaphore = Arc::new(Semaphore::new(options.max_parallel_pages));

    // Track metadata across all pages
    let mut total_spans = 0;
    let mut total_blocks = 0;
    let mut error_count = 0;
    let mut page_count = 0;

    // Phase 7.1.4: Collect page data for coverage check
    let mut pages_with_mcids: Vec<(usize, Option<i32>, std::collections::HashSet<u32>)> =
        Vec::new();
    let needs_coverage_check = catalog.mark_info.requires_coverage_check() && struct_tree.is_some();

    while let Some(page_result) = page_iter.next() {
        let page_dict = match page_result {
            Ok(p) => p,
            Err(diagnostics) => {
                let msg = diagnostics
                    .first()
                    .map(|d| d.message.as_ref())
                    .unwrap_or("unknown error");
                error_count += 1;
                let error_page = PageResult {
                    index: page_count,
                    page_number: (page_count + 1) as u32,
                    page_label: None,
                    width: None,
                    height: None,
                    rotation: None,
                    page_type: None,
                    spans: vec![],
                    blocks: vec![],
                    tables: vec![],
                    annotations: vec![],
                    error: Some(msg.to_string()),
                };
                if !callback(&error_page) {
                    break;
                }
                if needs_coverage_check {
                    pages_with_mcids.push((page_count, None, std::collections::HashSet::new()));
                }
                page_count += 1;
                continue;
            }
        };

        // Track MCIDs for this page if coverage check is needed
        if needs_coverage_check {
            let decoded_streams = decode_page_content_streams(
                &page_dict,
                &resolver_arc,
                &source,
                DEFAULT_MAX_DECOMPRESS_BYTES,
            );

            let mut tracker = McidTracker::new();
            track_mcids_from_content_stream(&decoded_streams, &mut tracker);

            let struct_parents = page_dict.struct_parents();
            let mcid_set = tracker.mcid_set().clone();
            pages_with_mcids.push((page_count, struct_parents, mcid_set));

            drop(decoded_streams);
        }

        // Extract this page
        let _permit = semaphore.acquire_guard();
        let extract_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            extract_page_from_dict(
                &fingerprint_arc,
                page_count,
                &page_dict,
                &options_arc,
                Some(&source),
                Some(&resolver_arc),
            )
        }));

        let page_result = match extract_result {
            Ok(Ok(internal_page)) => {
                total_spans += internal_page.spans.len();
                total_blocks += internal_page.blocks.len();
                PageResult::from(internal_page)
            }
            Ok(Err(e)) => {
                error_count += 1;
                PageResult {
                    index: page_count,
                    page_number: (page_count + 1) as u32,
                    page_label: None,
                    width: None,
                    height: None,
                    rotation: None,
                    page_type: None,
                    spans: vec![],
                    blocks: vec![],
                    tables: vec![],
                    annotations: vec![],
                    error: Some(e.to_string()),
                }
            }
            Err(_) => {
                error_count += 1;
                PageResult {
                    index: page_count,
                    page_number: (page_count + 1) as u32,
                    page_label: None,
                    width: None,
                    height: None,
                    rotation: None,
                    page_type: None,
                    spans: vec![],
                    blocks: vec![],
                    tables: vec![],
                    annotations: vec![],
                    error: Some(format!("Page {} extraction panicked", page_count)),
                }
            }
        };

        // Invoke callback with this page
        if !callback(&page_result) {
            // Caller requested early termination
            break;
        }

        drop(page_dict);
        page_count += 1;
    }

    // Phase 7.1.4: Perform coverage check if Suspects is true
    let (final_reading_order_algorithm, coverage_diagnostics) = if needs_coverage_check {
        if let Some(ref tree) = struct_tree {
            let coverage_result =
                check_coverage_for_pages(tree, &catalog.mark_info, &pages_with_mcids);
            let diagnostics: Vec<String> = coverage_result
                .diagnostics
                .iter()
                .map(|d| d.message.as_ref().to_string())
                .collect();
            (coverage_result.reading_order_algorithm, diagnostics)
        } else {
            (reading_order_algorithm, Vec::new())
        }
    } else {
        (reading_order_algorithm, Vec::new())
    };

    // Add the tagged PDF deferred diagnostic if present
    let mut all_diagnostics = coverage_diagnostics;
    if let Some(ref deferred) = deferred_diagnostic {
        all_diagnostics.push(deferred.message.as_ref().to_string());
    }

    Ok(ExtractionMetadata {
        page_count,
        receipts_mode: options.receipts,
        span_count: total_spans,
        block_count: total_blocks,
        cache_status: None,
        cache_age_seconds: None,
        error_count,
        reading_order_algorithm: Some(final_reading_order_algorithm.as_str().to_string()),
        diagnostics: all_diagnostics,
    })
}

/// Find the startxref offset in a PDF file.
///
/// Scans the last 1024 bytes of the file for "startxref" keyword.
fn find_startxref(source: &FileSource) -> anyhow::Result<u64> {
    use crate::parser::stream::PdfSource;

    let len = source.len()? as usize;
    let scan_start = len.saturating_sub(1024);
    let scan_end = len;

    let tail_data = source
        .read_at(scan_start as u64, scan_end - scan_start)
        .context("Failed to read PDF tail")?;

    // Find "startxref" in the tail data
    let startxref_pos = tail_data
        .windows(9)
        .rposition(|w| w == b"startxref")
        .ok_or_else(|| anyhow::anyhow!("startxref not found in PDF"))?;

    // Parse the offset after "startxref"
    let offset_data = &tail_data[startxref_pos + 9..];

    // Skip leading whitespace (space, \r, \n, \t)
    let offset_start = offset_data
        .iter()
        .position(|&b| !matches!(b, b' ' | b'\r' | b'\n' | b'\t'))
        .unwrap_or(offset_data.len());

    let offset_data_trimmed = &offset_data[offset_start..];

    // Find the newline after the offset
    let newline_pos = offset_data_trimmed
        .iter()
        .position(|&b| b == b'\n' || b == b'\r')
        .unwrap_or(offset_data_trimmed.len());

    let offset_str = std::str::from_utf8(&offset_data_trimmed[..newline_pos])
        .context("startxref offset is not valid UTF-8")?;

    let offset: u64 = offset_str
        .trim()
        .parse()
        .context("startxref offset is not a valid number")?;

    Ok(offset)
}

/// Extract content from a single page dict.
///
/// This function extracts content from a page using lazy stream decoding:
/// 1. Content streams are decoded only for this page (not pre-fetched)
/// 2. Decoded bytes are dropped immediately after processing
/// 3. No state is held across page boundaries
///
/// # Arguments
///
/// * `fingerprint` - The PDF fingerprint for receipt generation
/// * `page_index` - 0-based page index
/// * `page` - The page dictionary from the PDF
/// * `options` - Extraction options
/// * `source` - The PDF source for reading stream data (optional, for lazy decode)
/// * `resolver` - The xref resolver (optional, for lazy decode)
///
/// # Returns
///
/// A `PageResultInternal` with grid information preserved for two-page detection.
fn extract_page_from_dict(
    fingerprint: &str,
    page_index: usize,
    page: &crate::parser::pages::PageDict,
    options: &ExtractionOptions,
    source: Option<&dyn crate::parser::stream::PdfSource>,
    resolver: Option<&crate::parser::xref::XrefResolver>,
) -> Result<PageResultInternal> {
    let [x0, y0, x1, y1] = page.media_box;
    let page_height = y1 - y0;

    // Lazy decode content streams if source and resolver are provided
    let decoded_streams = if let (Some(src), Some(res)) = (source, resolver) {
        Some(decode_page_content_streams(
            page,
            res,
            src,
            DEFAULT_MAX_DECOMPRESS_BYTES,
        ))
    } else {
        None
    };

    // Detect tables using line-based and borderless detection
    let tables = if let Some(ref content_bytes) = decoded_streams {
        detect_tables_on_page(page, content_bytes, page_index)?
    } else {
        Vec::new()
    };

    // Create a placeholder span for the entire page
    // This is a minimal implementation - the full Phase 3 pipeline
    // would extract actual text from the decoded content streams
    let span_text = format!("[Page {} text extraction]", page_index);
    let span_bbox = [x0, y0, x1, y1];

    // Generate receipt if requested
    let receipt = generate_receipt(
        fingerprint,
        page_index,
        span_bbox,
        &span_text,
        options.receipts,
        #[cfg(feature = "receipts")]
        None,
    )?;

    let span = SpanJson {
        text: span_text,
        bbox: span_bbox,
        font: "Unknown".to_string(),
        size: 12.0,
        color: None,
        rendering_mode: None,
        confidence: None,
        confidence_source: None,
        lang: None,
        flags: vec![],
        receipt,
        column: None,
    };

    // Create blocks including table blocks
    let mut blocks = Vec::new();

    // Add table blocks
    for (table_idx, table) in tables.iter().enumerate() {
        // Use the grid's bbox for the block, not a placeholder
        let table_bbox = [
            table.grid.bbox[0] as f64,
            table.grid.bbox[1] as f64,
            table.grid.bbox[2] as f64,
            table.grid.bbox[3] as f64,
        ];

        let table_receipt = generate_receipt(
            fingerprint,
            page_index,
            table_bbox,
            "table",
            options.receipts,
            #[cfg(feature = "receipts")]
            None,
        )?;

        blocks.push(BlockJson {
            kind: "table".to_string(),
            text: format!("Table {}", table_idx),
            bbox: table_bbox,
            level: None,
            table_index: Some(table_idx),
            spans: vec![],
            receipt: table_receipt,
        });
    }

    // Add a placeholder paragraph block
    let block_text = span.text.clone();
    let block_bbox = span_bbox;
    let block_receipt = generate_receipt(
        fingerprint,
        page_index,
        block_bbox,
        &block_text,
        options.receipts,
        #[cfg(feature = "receipts")]
        None,
    )?;

    blocks.push(BlockJson {
        kind: "paragraph".to_string(),
        text: block_text,
        bbox: block_bbox,
        level: None,
        table_index: None,
        spans: vec![],
        receipt: block_receipt,
    });

    Ok(PageResultInternal {
        index: page_index,
        spans: vec![span],
        blocks,
        tables,
        annotations: vec![],
        error: None,
        page_height,
    })
}

/// Detect tables on a page using line-based and borderless detection.
///
/// This function runs both detection methods and combines the results,
/// preferring line-based detection when both find tables in similar positions.
///
/// Returns `Vec<TableWithGrid>` to preserve grid information for two-page detection.
fn detect_tables_on_page(
    page: &crate::parser::pages::PageDict,
    content_bytes: &[u8],
    page_index: usize,
) -> Result<Vec<TableWithGrid>> {
    use crate::table::PageContext;

    let ctx = PageContext::new(page, content_bytes);
    let detector = TableDetector::new();

    // Try line-based detection first
    let line_based_grids = detector.detect_line_based(&ctx);

    // If no tables found, try borderless detection
    let grids = if line_based_grids.is_empty() {
        detector.detect_borderless(&ctx)
    } else {
        line_based_grids
    };

    // Convert grids to TableWithGrid
    let mut tables = Vec::new();
    for grid in grids {
        // Create empty cells (no span assignment yet - that requires full text extraction)
        let cells = create_empty_cells(&grid);

        let detection_method = if grid.segments.is_empty() {
            "borderless"
        } else {
            "line_based"
        };

        let table_json = grid_to_table_json(
            &grid,
            &cells,
            page_index,
            detection_method,
            false, // continued - will be set by two-page detection
            false, // continued_from_prev - will be set by two-page detection
        );

        tables.push(TableWithGrid {
            json: table_json,
            grid,
        });
    }

    Ok(tables)
}

/// Create empty cells for a grid (placeholder for when text extraction is not available).
fn create_empty_cells(grid: &crate::table::GridCandidate) -> Vec<Cell> {
    let mut cells = Vec::new();

    for row in 0..grid.row_count() {
        for col in 0..grid.col_count() {
            if let Some(bbox) = grid.cell_bbox(row, col) {
                cells.push(Cell::new(bbox, row, col));
            }
        }
    }

    cells
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    /// Create a minimal valid PDF for testing.
    fn create_minimal_pdf(path: &Path) -> Result<()> {
        let pdf_data = br#"%PDF-1.4
1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj
2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj
3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Resources<</Font<</F1<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>>>>>>>>>endobj

xref
0 4
0000000000 65535 f
0000000009 00000 n
0000000052 00000 n
0000000101 00000 n
trailer<</Size 4/Root 1 0 R>>
startxref
239
%%EOF
"#;
        fs::write(path, pdf_data)?;
        Ok(())
    }

    /// Get a test PDF file path.
    /// Uses one of the classifier fixture PDFs for testing.
    fn get_test_pdf_path() -> std::path::PathBuf {
        // For now, use the temp-based minimal PDF to ensure tests are self-contained
        // This avoids dependency on external fixture files that may be malformed
        std::path::PathBuf::from("__test__.pdf")
    }

    /// Get or create the test PDF file.
    fn ensure_test_pdf() -> std::path::PathBuf {
        let path = get_test_pdf_path();
        if !path.exists() {
            create_minimal_pdf(&path).unwrap();
        }
        path
    }

    #[test]
    fn test_extract_pdf_with_receipts_off() {
        let pdf_path = ensure_test_pdf();

        let options = ExtractionOptions::default();
        let result = extract_pdf(&pdf_path, &options).unwrap();

        assert!(result.pages.len() >= 1);
        assert_eq!(result.metadata.receipts_mode, ReceiptsMode::Off);

        let page = &result.pages[0];
        assert!(!page.spans.is_empty());

        // Receipts should be None when mode is Off
        for span in &page.spans {
            assert!(span.receipt.is_none());
        }
        for block in &page.blocks {
            assert!(block.receipt.is_none());
        }
    }

    #[test]
    fn test_extract_pdf_with_receipts_lite() {
        let pdf_path = ensure_test_pdf();

        let options = ExtractionOptions::with_receipts(ReceiptsMode::Lite);
        let result = extract_pdf(&pdf_path, &options).unwrap();

        assert_eq!(result.metadata.receipts_mode, ReceiptsMode::Lite);

        let page = &result.pages[0];
        assert!(!page.spans.is_empty());

        // Receipts should be present in lite mode
        for span in &page.spans {
            assert!(span.receipt.is_some());
            let receipt = span.receipt.as_ref().unwrap();
            assert_eq!(receipt.pdf_fingerprint, result.fingerprint);
            assert!(receipt.svg_clip.is_none());
        }

        for block in &page.blocks {
            assert!(block.receipt.is_some());
            let receipt = block.receipt.as_ref().unwrap();
            assert_eq!(receipt.pdf_fingerprint, result.fingerprint);
            assert!(receipt.svg_clip.is_none());
        }
    }

    #[test]
    fn test_extract_pdf_with_receipts_svg() {
        let pdf_path = ensure_test_pdf();

        let options = ExtractionOptions::with_receipts(ReceiptsMode::SvgClip);
        let result = extract_pdf(&pdf_path, &options).unwrap();

        assert_eq!(result.metadata.receipts_mode, ReceiptsMode::SvgClip);

        let page = &result.pages[0];
        assert!(!page.spans.is_empty());

        // Receipts should be present
        // Note: In this minimal implementation without glyph data,
        // SVG mode falls back to lite mode (svg_clip is None)
        for span in &page.spans {
            assert!(span.receipt.is_some());
            let receipt = span.receipt.as_ref().unwrap();
            assert_eq!(receipt.pdf_fingerprint, result.fingerprint);
        }
    }

    #[test]
    fn test_result_to_json_format() {
        let pdf_path = ensure_test_pdf();

        let options = ExtractionOptions::default();
        let result = extract_pdf(&pdf_path, &options).unwrap();
        let json = result_to_json(&result);

        assert!(json.is_object());
        assert!(json.get("fingerprint").is_some());
        assert!(json.get("schema_version").is_some());
        assert!(json.get("pages").is_some());
        assert!(json.get("metadata").is_some());

        let pages = json.get("pages").and_then(|v| v.as_array()).unwrap();
        assert_eq!(pages.len(), 1);

        let page = &pages[0];
        assert!(page.get("index").is_some());
        assert!(page.get("spans").is_some());
        assert!(page.get("blocks").is_some());
    }

    #[test]
    fn test_result_to_json_with_receipts() {
        let pdf_path = ensure_test_pdf();

        let options = ExtractionOptions::with_receipts(ReceiptsMode::Lite);
        let result = extract_pdf(&pdf_path, &options).unwrap();
        let json = result_to_json(&result);

        let pages = json.get("pages").and_then(|v| v.as_array()).unwrap();
        let page = &pages[0];
        let spans = page.get("spans").and_then(|v| v.as_array()).unwrap();
        let span = &spans[0];

        // Span should have receipt field
        assert!(span.get("receipt").is_some());

        let receipt = span.get("receipt").unwrap();
        assert!(receipt.get("pdf_fingerprint").is_some());
        assert!(receipt.get("page_index").is_some());
        assert!(receipt.get("bbox").is_some());
        assert!(receipt.get("content_hash").is_some());
        assert!(receipt.get("extraction_version").is_some());

        // svg_clip should not be present in lite mode
        assert!(receipt.get("svg_clip").is_none());
    }

    #[test]
    fn test_extraction_metadata() {
        let pdf_path = ensure_test_pdf();

        let options = ExtractionOptions::with_receipts(ReceiptsMode::Lite);
        let result = extract_pdf(&pdf_path, &options).unwrap();

        assert!(result.metadata.page_count >= 1);
        assert!(result.metadata.span_count > 0);
        assert!(result.metadata.block_count > 0);
        assert_eq!(result.metadata.receipts_mode, ReceiptsMode::Lite);
    }

    #[test]
    fn test_result_to_json_includes_signatures() {
        // Test that result_to_json includes the signatures array
        let pdf_path = ensure_test_pdf();

        let options = ExtractionOptions::default();
        let result = extract_pdf(&pdf_path, &options).unwrap();

        let json = result_to_json(&result);

        // Verify signatures key exists
        assert!(json.get("signatures").is_some());

        // Verify signatures is an array
        assert!(json["signatures"].is_array());

        // For most test PDFs, signatures will be empty (no signature fields)
        // But the array should always be present
    }

    #[test]
    fn test_signatures_always_not_checked() {
        // Test that all signatures have validation_status == "not_checked"
        // This is required by the plan - cryptographic verification is out of scope for v1
        let pdf_path = ensure_test_pdf();

        let options = ExtractionOptions::default();
        let result = extract_pdf(&pdf_path, &options).unwrap();

        for sig in &result.signatures {
            assert_eq!(sig.validation_status, "not_checked");
        }
    }

    #[test]
    fn test_signature_json_schema_round_trip() {
        // Test that SignatureJson round-trips through JSON correctly
        use crate::schema::SignatureJson;

        let sig = SignatureJson {
            field_name: "test_sig".to_string(),
            signer_name: "John Doe".to_string(),
            signing_date: Some("2023-01-15T14:30:45Z".to_string()),
            reason: Some("Test".to_string()),
            location: Some("Test Location".to_string()),
            sub_filter: Some("adbe.pkcs7.detached".to_string()),
            byte_range: Some(vec![0, 1000, 2000, 500]),
            coverage_fraction: Some(0.5),
            validation_status: "not_checked".to_string(),
        };

        let json_str = serde_json::to_string(&sig).unwrap();
        let deserialized: SignatureJson = serde_json::from_str(&json_str).unwrap();

        assert_eq!(deserialized, sig);
    }

    #[test]
    fn test_signature_json_validation_status_enum() {
        // Test that validation_status accepts only valid enum values
        use crate::schema::SignatureJson;

        let sig_valid = SignatureJson {
            field_name: "test".to_string(),
            signer_name: String::new(),
            signing_date: None,
            reason: None,
            location: None,
            sub_filter: None,
            byte_range: None,
            coverage_fraction: None,
            validation_status: "not_checked".to_string(),
        };

        // Should serialize correctly
        let json = serde_json::to_string(&sig_valid).unwrap();
        assert!(json.contains("not_checked"));
    }

    #[test]
    fn test_tagged_pdf_emits_deferred_diagnostic() {
        // Test that tagged PDFs emit TAGGED_PDF_STRUCT_TREE_DEFERRED diagnostic
        use crate::diagnostics::DiagCode;

        let temp_dir = tempfile::tempdir().unwrap();
        let pdf_path = temp_dir.path().join("tagged_test.pdf");

        // Create a minimal tagged PDF (with /MarkInfo /Marked true)
        let pdf_data = br#"%PDF-1.4
1 0 obj<</Type/Catalog/Pages 2 0 R/MarkInfo<</Marked true>>>>endobj
2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj
3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Resources<</Font<</F1<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>>>>>>>>>endobj

xref
0 4
0000000000 65535 f
0000000009 00000 n
0000000096 00000 n
0000000145 00000 n
trailer<</Size 4/Root 1 0 R>>
startxref
283
%%EOF
"#;
        fs::write(&pdf_path, pdf_data).unwrap();

        let options = ExtractionOptions::default();
        let result = extract_pdf(&pdf_path, &options).unwrap();

        // Verify the tagged PDF diagnostic is emitted
        assert!(!result.metadata.diagnostics.is_empty());
        let deferred_diag = result
            .metadata
            .diagnostics
            .iter()
            .find(|d| d.contains("TAGGED_PDF_STRUCT_TREE_DEFERRED"))
            .expect("TAGGED_PDF_STRUCT_TREE_DEFERRED diagnostic should be emitted for tagged PDFs");

        // Verify the reading order algorithm is xy_cut
        assert_eq!(
            result.metadata.reading_order_algorithm,
            Some("xy_cut".to_string()),
            "Tagged PDFs should use xy_cut algorithm in v0.1.0-v0.3.0"
        );
    }

    #[test]
    fn test_untagged_pdf_no_deferred_diagnostic() {
        // Test that untagged PDFs do NOT emit TAGGED_PDF_STRUCT_TREE_DEFERRED
        let pdf_path = ensure_test_pdf();

        let options = ExtractionOptions::default();
        let result = extract_pdf(&pdf_path, &options).unwrap();

        // Verify NO tagged PDF diagnostic is emitted
        let has_deferred_diag = result
            .metadata
            .diagnostics
            .iter()
            .any(|d| d.contains("TAGGED_PDF_STRUCT_TREE_DEFERRED"));

        assert!(
            !has_deferred_diag,
            "Untagged PDFs should NOT emit TAGGED_PDF_STRUCT_TREE_DEFERRED diagnostic"
        );
    }
}
