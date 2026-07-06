//! Worker function for single-pass per-file PDF grep.
//!
//! This module implements the core worker that processes a single FileWorkItem
//! into MatchEvents via Phase 1 (lexer/object/xref) + Phase 3 (content streams)
//! + Phase 4 span builder (skipping Phase 4.5 reading-order detection).
//!
//! # Architecture
//!
//! The worker is designed to be called from a thread pool and processes one file
//! at a time. It sends results to two channels:
//! - Match events: actual matches found in the PDF
//! - Progress events: file-level progress updates
//!
//! # Performance
//!
//! The worker skips reading-order detection (Phase 4.5) because grep doesn't need
//! it — this cuts per-file CPU by ~30-40% on typical pages.

use super::event::{MatchEvent, ProgressEvent};
use super::expand::{FileWorkItem, PathOrUrl};
use super::matcher::{MatchRange, Matcher};
use super::GrepConfig;
use anyhow::{anyhow, Context, Result};
use pdftract_core::content_stream::{process_with_mode, Glyph, ProcessingMode};
use pdftract_core::diagnostics::Diagnostic;
use pdftract_core::fingerprint::{
    compute_fingerprint, CatalogFlags, ContentStreamData, PageFingerprintData,
};
use pdftract_core::parser::catalog::Catalog;
use pdftract_core::parser::object::PdfObject;
use pdftract_core::parser::pages::{flatten_page_tree, PageDict};
use pdftract_core::parser::resources::ResourceDict;
use pdftract_core::parser::stream::{FileSource, SourceAdapter};
use pdftract_core::parser::xref::{load_xref_with_prev_chain, XrefResolver, XrefSection};
use pdftract_core::source::PdfSource as SourcePdfSource;
use std::sync::Arc;
use std::time::Instant;

#[cfg(feature = "remote")]
use pdftract_core::source::HttpRangeSource;

/// Result of processing a single PDF file.
///
/// Contains the matches found and the total match count.
pub struct WorkerResult {
    /// Match events found in this file.
    pub matches: Vec<MatchEvent>,
    /// Total number of matches.
    pub match_count: usize,
}

/// Process a single PDF file and emit match and progress events.
///
/// This is the main worker function that:
/// 1. Opens the PDF file
/// 2. Checks for encryption (skips with diagnostic if encrypted without password)
/// 3. For each page, extracts spans via content stream processing
/// 4. Applies the matcher to each span
/// 5. Emits match events for found matches
/// 6. Emits progress events for observability
///
/// # Arguments
///
/// * `item` - The file work item to process
/// * `matcher` - The pattern matcher
/// * `config` - The grep configuration
/// * `match_sink` - Channel to send match events
/// * `progress_sink` - Channel to send progress events
///
/// # Errors
///
/// Returns an error if:
/// - The file cannot be opened
/// - The PDF is malformed
/// - Encryption is detected without a password
pub fn worker_run(
    item: &FileWorkItem,
    matcher: &Arc<Matcher>,
    config: &Arc<GrepConfig>,
    match_sink: &crossbeam_channel::Sender<MatchEvent>,
    progress_sink: &crossbeam_channel::Sender<ProgressEvent>,
) -> Result<()> {
    let start_time = Instant::now();

    // Get the path string and whether it's a URL
    let (path_str, is_remote) = match &item.path {
        PathOrUrl::Local(p) => (p.to_string_lossy().to_string(), false),
        PathOrUrl::Remote(url) => (url.clone(), true),
    };

    // Emit file start event
    progress_sink.send(ProgressEvent::FileStart {
        path: item.path.display(),
        size_hint: item.size_hint,
    })?;

    // Open the PDF source (local or remote)
    let source: Box<dyn SourcePdfSource> = if is_remote {
        #[cfg(feature = "remote")]
        {
            // Convert headers HashMap to Vec<(String, String)>
            let headers_vec: Vec<(String, String)> = config.headers.clone().into_iter().collect();

            match HttpRangeSource::with_headers(&path_str, headers_vec) {
                Ok(s) => Box::new(s),
                Err(e) => {
                    progress_sink.send(ProgressEvent::FileSkipped {
                        path: item.path.display(),
                        reason: format!("failed to open remote PDF: {}", e),
                    })?;
                    return Ok(());
                }
            }
        }
        #[cfg(not(feature = "remote"))]
        {
            progress_sink.send(ProgressEvent::FileSkipped {
                path: item.path.display(),
                reason: "remote URL support not compiled in".to_string(),
            })?;
            return Ok(());
        }
    } else {
        match FileSource::open(&path_str) {
            Ok(s) => Box::new(s),
            Err(e) => {
                progress_sink.send(ProgressEvent::FileSkipped {
                    path: item.path.display(),
                    reason: format!("failed to open: {}", e),
                })?;
                return Ok(());
            }
        }
    };

    // Adapt source for parser functions
    let adapted_source = SourceAdapter::new(source);

    // Find the startxref offset
    let startxref_offset = match find_startxref(adapted_source.inner()) {
        Ok(offset) => offset,
        Err(e) => {
            progress_sink.send(ProgressEvent::FileSkipped {
                path: item.path.display(),
                reason: format!("invalid PDF: {}", e),
            })?;
            return Ok(());
        }
    };

    // Load the xref table
    let xref_section = load_xref_with_prev_chain(&adapted_source, startxref_offset);

    // Check for encryption
    if let Some(trailer) = &xref_section.trailer {
        if let Some(_encrypt) = trailer.get("/Encrypt") {
            // Encrypted PDF without password support - skip with diagnostic
            eprintln!("{}: encrypted (skipped)", item.path.display());
            progress_sink.send(ProgressEvent::FileSkipped {
                path: item.path.display(),
                reason: "encrypted (no password provided)".to_string(),
            })?;
            return Ok(());
        }
    }

    // Create resolver from xref section
    let resolver = XrefResolver::from_section(xref_section.clone());

    // Get the root reference from trailer
    let root_ref = match xref_section
        .trailer
        .as_ref()
        .and_then(|trailer| trailer.get("/Root"))
    {
        Some(PdfObject::Ref(root_ref)) => *root_ref,
        _ => {
            progress_sink.send(ProgressEvent::FileSkipped {
                path: path_str.clone(),
                reason: "no /Root in trailer".to_string(),
            })?;
            return Ok(());
        }
    };

    // Parse the catalog
    let catalog = match parse_catalog_with_resolver(&resolver, root_ref, &adapted_source) {
        Ok(c) => c,
        Err(diagnostics) => {
            let msg = diagnostics
                .first()
                .map(|d| d.message.as_ref())
                .unwrap_or("unknown error");
            progress_sink.send(ProgressEvent::FileSkipped {
                path: path_str.clone(),
                reason: format!("failed to parse catalog: {}", msg),
            })?;
            return Ok(());
        }
    };

    // Flatten the page tree
    let pages = match flatten_page_tree(&resolver, catalog.pages_ref) {
        Ok(p) => p,
        Err(diagnostics) => {
            let msg = diagnostics
                .first()
                .map(|d| d.message.as_ref())
                .unwrap_or("unknown error");
            progress_sink.send(ProgressEvent::FileSkipped {
                path: path_str.clone(),
                reason: format!("failed to parse page tree: {}", msg),
            })?;
            return Ok(());
        }
    };

    let pages_total = pages.len();

    // Parse page range if specified
    let page_filter: Option<std::collections::BTreeSet<usize>> = if let Some(ref range_str) =
        config.pages
    {
        let mut page_range_diagnostics = Vec::new();
        match pdftract_core::pages::parse_pages(range_str, pages_total, &mut page_range_diagnostics)
        {
            Ok(filter) => {
                // Emit diagnostics for out-of-range pages
                for diag in page_range_diagnostics {
                    eprintln!("Warning: {}", diag.message);
                }
                Some(filter)
            }
            Err(e) => {
                // Invalid page range syntax - emit error and skip all pages
                eprintln!("Error: {}", e);
                return Ok(());
            }
        }
    } else {
        None
    };

    // Compute fingerprint once per file
    let fingerprint = compute_fingerprint_for_grep(&catalog, &pages, &xref_section, &resolver);

    let mut total_match_count = 0;

    // Process each page
    for (page_index, page) in pages.iter().enumerate() {
        // Skip if page filter is set and this page is not in the filter
        if let Some(ref filter) = page_filter {
            if !filter.contains(&page_index) {
                continue;
            }
        }
        // Emit page progress
        progress_sink.send(ProgressEvent::FileProgress {
            path: path_str.clone(),
            pages_done: page_index,
            pages_total,
        })?;

        // Extract spans from this page
        let spans = match extract_spans_from_page(page, &resolver, &adapted_source) {
            Ok(s) => s,
            Err(e) => {
                // Log error but continue with next page
                eprintln!(
                    "Warning: failed to extract spans from page {}: {}",
                    page_index, e
                );
                continue;
            }
        };

        // Apply matcher to each span
        for span in spans {
            let matches_in_span = process_span(
                &span,
                std::path::Path::new(&path_str),
                page_index as u32,
                &fingerprint,
                matcher,
                &config,
            );

            total_match_count += matches_in_span.len();

            // Emit match events
            for match_event in matches_in_span {
                match_sink.send(match_event)?;
            }
        }
    }

    // Emit file done event
    let duration_ms = start_time.elapsed().as_millis();
    progress_sink.send(ProgressEvent::FileDone {
        path: path_str.clone(),
        matches: total_match_count,
        duration_ms,
    })?;

    Ok(())
}

/// Compute fingerprint for grep mode.
///
/// This is a simplified fingerprint computation that uses the catalog,
/// pages, and xref_section to compute the document fingerprint.
fn compute_fingerprint_for_grep(
    catalog: &Catalog,
    pages: &[PageDict],
    xref_section: &XrefSection,
    resolver: &XrefResolver,
) -> String {
    use pdftract_core::fingerprint::FingerprintInput;

    // Build fingerprint input from catalog and pages
    let page_count = pages.len() as u32;

    let fingerprint_pages = pages
        .iter()
        .map(|page| PageFingerprintData {
            content_streams: page
                .contents
                .iter()
                .map(|&obj_ref| ContentStreamData::Indirect(obj_ref))
                .collect(),
            resources: None, // Skip resources for grep mode (performance)
            media_box: page.media_box,
            crop_box: page.crop_box,
            rotate: page.rotate,
        })
        .collect();

    // Build catalog flags
    let catalog_flags = CatalogFlags {
        is_encrypted: false, // Already checked earlier
        contains_javascript: catalog.open_action.is_some() || catalog.aa.is_some(),
        contains_xfa: false, // Not detected in grep mode
        ocg_present: catalog
            .oc_properties
            .as_ref()
            .map(|props| props.present)
            .unwrap_or(false),
    };

    let fingerprint_input = FingerprintInput {
        page_count,
        pages: fingerprint_pages,
        struct_tree_root_ref: catalog.struct_tree_root_ref,
        is_tagged: catalog.mark_info.is_tagged,
        catalog_flags,
    };

    compute_fingerprint(&fingerprint_input, resolver, None)
}

/// A span of text extracted from a PDF.
#[derive(Debug, Clone)]
struct Span {
    /// The text content.
    pub text: String,
    /// Bounding box [x0, y0, x1, y1].
    pub bbox: [f32; 4],
    /// Page index (0-based).
    pub page_index: u32,
    /// Confidence score (0.0 to 1.0).
    pub confidence: f32,
    /// Font name.
    pub font: String,
    /// Font size in points.
    pub font_size: f32,
}

/// Extract spans from a single page via content stream processing.
///
/// This runs Phase 3 (content stream parsing) to extract text with bounding boxes.
/// It skips Phase 4.5 (reading-order detection) as grep doesn't need it.
fn extract_spans_from_page(
    page: &PageDict,
    resolver: &XrefResolver,
    source: &SourceAdapter,
) -> Result<Vec<Span>> {
    // Get page resources (already resolved in PageDict)
    let resources = (*page.resources).clone();

    // Decode and process content streams
    let decoded = decode_page_streams(page, resolver, source)?;

    // Process content stream to extract glyphs
    let glyphs = process_with_mode(&decoded, &resources, ProcessingMode::Normal, None).map_err(
        |diagnostics| {
            let msg = diagnostics
                .first()
                .map(|d| d.message.as_ref())
                .unwrap_or("unknown error");
            anyhow!("failed to process content stream: {}", msg)
        },
    )?;

    // Group glyphs into spans (consecutive glyphs with same font)
    let spans = group_glyphs_into_spans(glyphs);

    Ok(spans)
}

/// Group consecutive glyphs into spans based on font proximity.
///
/// This is a simplified span builder that groups glyphs that are:
/// - From the same font
/// - At similar Y positions (same line)
/// - Close together horizontally (within 2x font size)
///
/// This is sufficient for grep use cases without full reading-order detection.
fn group_glyphs_into_spans(glyphs: Vec<Glyph>) -> Vec<Span> {
    if glyphs.is_empty() {
        return Vec::new();
    }

    let mut spans = Vec::new();
    let mut current_span_glyphs = Vec::new();
    let mut last_font: Option<String> = None;
    let mut last_y: Option<f64> = None;
    let mut last_x_end: Option<f64> = None;
    let mut last_font_size: Option<f64> = None;

    for glyph in glyphs {
        let font = glyph.font.clone().unwrap_or_else(|| "unknown".to_string());
        let y = glyph.bbox[1]; // Bottom of bbox
        let x_end = glyph.bbox[2]; // Right of bbox
        let font_size = glyph.size.unwrap_or(12.0);

        // Check if we should start a new span
        let should_start_new = if last_font.is_none() {
            false
        } else {
            // Different font?
            let font_changed = last_font.as_ref() != Some(&font);

            // Different line? (Y position differs by more than 20% of font size)
            let line_changed = last_y.map_or(false, |ly| (ly - y).abs() > font_size * 0.2);

            // Too far horizontally? (gap > 2x font size)
            let too_far = last_x_end.map_or(false, |lx| glyph.bbox[0] - lx > font_size * 2.0);

            font_changed || line_changed || too_far
        };

        if should_start_new {
            // Finalize current span
            if !current_span_glyphs.is_empty() {
                spans.push(create_span_from_glyphs(&current_span_glyphs));
                current_span_glyphs.clear();
            }
        }

        // Add glyph to current span
        current_span_glyphs.push(glyph.clone());

        // Update tracking state
        last_font = Some(font);
        last_y = Some(y);
        last_x_end = Some(x_end);
        last_font_size = Some(font_size);
    }

    // Don't forget the last span
    if !current_span_glyphs.is_empty() {
        spans.push(create_span_from_glyphs(&current_span_glyphs));
    }

    spans
}

/// Create a span from a group of glyphs.
fn create_span_from_glyphs(glyphs: &[Glyph]) -> Span {
    if glyphs.is_empty() {
        return Span {
            text: String::new(),
            bbox: [0.0, 0.0, 0.0, 0.0],
            page_index: 0,
            confidence: 1.0,
            font: "unknown".to_string(),
            font_size: 12.0,
        };
    }

    // Concatenate text
    let text: String = glyphs.iter().map(|g| g.unicode).collect();

    // Compute union bbox
    let mut x0 = f64::MAX;
    let mut y0 = f64::MAX;
    let mut x1 = f64::MIN;
    let mut y1 = f64::MIN;

    for glyph in glyphs {
        x0 = x0.min(glyph.bbox[0]);
        y0 = y0.min(glyph.bbox[1]);
        x1 = x1.max(glyph.bbox[2]);
        y1 = y1.max(glyph.bbox[3]);
    }

    // Get font and size from first glyph
    let font = glyphs[0]
        .font
        .clone()
        .unwrap_or_else(|| "unknown".to_string());
    let font_size = glyphs[0].size.unwrap_or(12.0);

    // Compute confidence as minimum of all glyphs
    let confidence = glyphs.iter().map(|g| g.confidence).fold(1.0, f32::min);

    Span {
        text,
        bbox: [x0 as f32, y0 as f32, x1 as f32, y1 as f32],
        page_index: 0, // Will be set by caller
        confidence,
        font,
        font_size: font_size as f32,
    }
}

/// Decode all content streams for a page.
fn decode_page_streams(
    page: &PageDict,
    resolver: &XrefResolver,
    source: &SourceAdapter,
) -> Result<Vec<u8>> {
    use pdftract_core::parser::stream::{
        decode_stream, ExtractionOptions as StreamExtractionOptions,
    };

    let stream_opts = StreamExtractionOptions {
        max_decompress_bytes: pdftract_core::parser::stream::DEFAULT_MAX_DECOMPRESS_BYTES,
        password: None,
    };

    let mut all_decoded = Vec::new();
    let mut doc_counter = 0u64;

    for stream_ref in &page.contents {
        match resolver.resolve(*stream_ref) {
            Ok(obj) => {
                if let Some(stream) = obj.as_stream() {
                    let decoded = decode_stream(stream, source, &stream_opts, &mut doc_counter);
                    all_decoded.extend_from_slice(&decoded);
                }
            }
            Err(_) => continue,
        }
    }

    Ok(all_decoded)
}

/// Process a single span and emit match events.
///
/// Applies the matcher to the span text and emits match events for each match.
/// Handles --invert-match by emitting synthetic events for spans with zero matches.
fn process_span(
    span: &Span,
    path: &std::path::Path,
    page_index: u32,
    fingerprint: &str,
    matcher: &Matcher,
    config: &GrepConfig,
) -> Vec<MatchEvent> {
    let path_str = path.display().to_string();

    // Find matches in this span
    let matches: Vec<MatchRange> = matcher
        .find_iter_with_word_boundary(&span.text, config.word_regexp)
        .collect();

    // Handle --invert-match: emit synthetic event for spans with zero matches
    if config.invert_match {
        if matches.is_empty() {
            return vec![MatchEvent::new(
                path_str,
                page_index,
                span.bbox,
                span.text.clone(),
                span.text.clone(),
                span.confidence,
                fingerprint.to_string(),
                false,
            )];
        } else {
            // Invert mode: skip spans that have matches
            return Vec::new();
        }
    }

    // Normal mode: emit events for each match
    matches
        .into_iter()
        .map(|m| {
            let match_text = span.text[m.start..m.end].to_string();
            MatchEvent::new(
                path_str.clone(),
                page_index,
                span.bbox,
                match_text,
                span.text.clone(),
                span.confidence,
                fingerprint.to_string(),
                false, // crosses_spans is always false in single-span mode
            )
        })
        .collect()
}

/// Find the startxref offset in a PDF file.
fn find_startxref(source: &dyn SourcePdfSource) -> Result<u64> {
    let len = source.len() as usize;
    let scan_start = len.saturating_sub(1024);
    let scan_end = len;

    let tail_data = source
        .read_range(scan_start as u64, scan_end - scan_start)
        .context("Failed to read PDF tail")?;

    // Find "startxref" in the tail data
    let startxref_pos = tail_data
        .windows(9)
        .rposition(|w| w == b"startxref")
        .ok_or_else(|| anyhow!("startxref not found in PDF"))?;

    // Parse the offset after "startxref"
    let offset_data = &tail_data[startxref_pos + 9..];

    // Skip leading whitespace
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

/// Parse the catalog with a given resolver.
fn parse_catalog_with_resolver(
    resolver: &XrefResolver,
    root_ref: pdftract_core::parser::object::ObjRef,
    source: &SourceAdapter,
) -> Result<Catalog, Vec<Diagnostic>> {
    pdftract_core::parser::catalog::parse_catalog(resolver, root_ref, Some(source))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_find_startxref() {
        // Create a minimal PDF with startxref
        let temp_dir = TempDir::new().unwrap();
        let pdf_path = temp_dir.path().join("test.pdf");

        let pdf_content = b"%PDF-1.4\n...\nstartxref\n12345\n%%EOF\n";
        File::create(&pdf_path)
            .unwrap()
            .write_all(pdf_content)
            .unwrap();

        let source = FileSource::open(&pdf_path).unwrap();
        let offset = find_startxref(&source).unwrap();
        assert_eq!(offset, 12345);
    }
}
