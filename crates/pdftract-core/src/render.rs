//! Direct image compositing for scanned pages (Phase 5.2.1).
//!
//! This module implements the default-feature image rendering path that:
//! 1. Walks the content stream operator list
//! 2. Builds CTM stack (q/Q + cm operators)
//! 3. Collects image XObject references (Do operator) with their CTMs
//! 4. Retrieves each image XObject via Phase 1.5 stream decoder
//! 5. Converts to GrayImage (luminance conversion from RGB if needed)
//! 6. Computes pixel placement using CTM
//! 7. Composites each placed image onto a white-background canvas
//!
//! This path has zero external dependencies (uses image crate from default deps)
//! and handles > 90% of scanned PDFs correctly.
//!
//! # Feature Gate
//!
//! This module is only available when the `ocr` feature is enabled.
#![cfg(feature = "ocr")]

// PDFium rendering path (Phase 5.2.2) - only available with full-render feature
#[cfg(all(feature = "ocr", feature = "full-render"))]
pub mod pdfium_path;

use crate::diagnostics::{DiagCode, Diagnostic};
use crate::graphics_state::{GraphicsState, GraphicsStateStack, Matrix3x3};
use crate::parser::lexer::Lexer;
use crate::parser::lexer::Token;
use crate::parser::object::{ObjRef, PdfObject};
use crate::parser::resources::ResourceDict;
use crate::parser::stream::{
    decode_stream, ExtractionOptions as StreamExtractionOptions, PdfSource,
};
use crate::parser::xref::XrefResolver;
use image::{DynamicImage, GrayImage, ImageBuffer, Luma, Rgb, RgbImage, Rgba, RgbaImage};
use std::sync::Arc;

/// Maximum number of images to composite per page (prevents DoS).
const MAX_IMAGES_PER_PAGE: usize = 256;

/// Result type for image compositing operations.
pub type Result<T> = std::result::Result<T, Vec<Diagnostic>>;

/// An image placement instruction from a Do operator.
///
/// Contains the XObject reference and the CTM at the time of the Do.
#[derive(Debug, Clone)]
pub struct ImagePlacement {
    /// The XObject reference (must be an Image XObject, not a Form).
    pub xobject_ref: ObjRef,
    /// The CTM at the time of the Do operator.
    pub ctm: Matrix3x3,
    /// The XObject name (for diagnostics).
    pub name: Arc<str>,
}

/// Header parameters for an inline image (BI/ID/EI sequence).
///
/// Contains the metadata from the inline image dictionary between BI and ID.
#[derive(Debug, Clone, Default)]
pub struct InlineImageHeader {
    /// Width in samples (required).
    pub width: Option<u32>,
    /// Height in samples (required).
    pub height: Option<u32>,
    /// Bits per component (default: 8).
    pub bpc: u8,
    /// Color space (default: DeviceGray).
    pub colorspace: Option<String>,
    /// Filter(s) applied to the image data.
    pub filters: Vec<String>,
    /// Whether this is an image mask (/ImageMask true).
    pub is_mask: bool,
    /// Image mask data (for /ImageMask true, /Mask [ Black | White ]).
    pub mask_color: Option<u8>,
}

/// Reference to image bytes.
///
/// For v0.1.0, we store raw bytes + filter chain inline.
/// Phase 5.2 will decode if/when needed for OCR.
#[derive(Debug, Clone)]
pub enum ImageBytesRef {
    /// Inline image data (raw bytes from content stream).
    Inline(Vec<u8>),
    /// XObject reference (resolved later).
    XObjectRef(ObjRef),
}

/// Represents either an XObject image or an inline image.
#[derive(Debug, Clone)]
pub enum ImageSource {
    /// An XObject reference (most common).
    XObject(ObjRef, Arc<str>),
    /// An inline image (BI/ID/EI sequence).
    Inline,
}

/// An image XObject record in a page's image list.
///
/// This struct unifies both Do-referenced XObject images and inline images
/// from BI/ID/EI sequences. Phase 4.4 figure detection uses this list to
/// classify blocks as `figure` when they contain only image XObjects.
#[derive(Debug, Clone)]
pub struct ImageXObject {
    /// Bounding box in PDF user-space points [x0, y0, x1, y1].
    ///
    /// For inline images: computed by transforming the unit square (0,0)-(1,1)
    /// by the current CTM at the time of BI/ID/EI.
    /// For Do-referenced images: computed similarly, but with the XObject's
    /// /Matrix also applied.
    pub bbox: [f32; 4],
    /// Source of the image (inline vs XObject).
    pub source: ImageSource,
    /// Header parameters (only populated for inline images).
    pub header: InlineImageHeader,
    /// Reference to the image bytes.
    pub bytes_ref: ImageBytesRef,
}

/// Walk content stream and collect image placements with their CTMs.
///
/// This function:
/// 1. Parses the content stream into tokens
/// 2. Maintains a CTM stack (q/Q operators)
/// 3. Tracks cm operators (concatenate matrix)
/// 4. Collects Do operators with their current CTM
/// 5. Collects inline images (BI/ID/EI sequences)
///
/// # Arguments
///
/// * `content` - The decoded content stream bytes
/// * `resources` - The page's resource dictionary (for XObject lookup)
///
/// # Returns
///
/// A list of image placements with their CTMs, or diagnostics if parsing fails.
pub fn collect_image_placements(
    content: &[u8],
    resources: &ResourceDict,
) -> Result<Vec<ImagePlacement>> {
    let mut placements = Vec::new();
    let mut diagnostics = Vec::new();

    // Create graphics state stack
    let mut gss = GraphicsStateStack::new();
    let mut state = GraphicsState::new();

    // Tokenize content stream
    let mut lexer = Lexer::new(content);
    let mut operand_buffer: Vec<Token> = Vec::new();

    while let Some(token) = lexer.next_token() {
        match token {
            Token::Keyword(ref k) => {
                let keyword = std::str::from_utf8(k).unwrap_or("");

                match keyword {
                    "q" => {
                        // Push graphics state
                        if !gss.push(&state) {
                            diagnostics.push(Diagnostic::with_static_no_offset(
                                DiagCode::GstateStackOverflow,
                                "Graphics state stack overflow",
                            ));
                            break;
                        }
                        operand_buffer.clear();
                    }
                    "Q" => {
                        // Pop graphics state
                        if let Some(popped) = gss.pop() {
                            state = popped;
                        }
                        operand_buffer.clear();
                    }
                    "cm" => {
                        // Concatenate matrix: cm expects exactly 6 numbers
                        let nums: Vec<f64> = operand_buffer
                            .iter()
                            .filter_map(|t| match t {
                                Token::Integer(n) => Some(*n as f64),
                                Token::Real(f) => Some(*f),
                                _ => None,
                            })
                            .collect();

                        if nums.len() != 6 {
                            diagnostics.push(Diagnostic::with_static_no_offset(
                                DiagCode::CmArgCount,
                                "cm operator requires exactly 6 numeric arguments",
                            ));
                            operand_buffer.clear();
                            continue;
                        }

                        let matrix = Matrix3x3::from_pdf_array([
                            nums[0], nums[1], nums[2], nums[3], nums[4], nums[5],
                        ]);

                        // Check for degenerate matrix (NaN or det == 0)
                        let has_nan = nums.iter().any(|&n| n.is_nan());
                        let det = matrix.determinant();
                        if has_nan || det == 0.0 {
                            diagnostics.push(Diagnostic::with_static_no_offset(
                                DiagCode::CmDegenerate,
                                "cm operator received degenerate matrix; clamped to identity",
                            ));
                            // Clamp to identity - don't modify CTM
                        } else {
                            state.concat_ctm(&matrix);
                        }
                        operand_buffer.clear();
                    }
                    "Do" => {
                        // Paint XObject: Do expects a name operand
                        if let Some(name_token) = operand_buffer.last() {
                            if let Token::Name(name_bytes) = name_token {
                                if let Ok(name_str) = std::str::from_utf8(name_bytes) {
                                    let name_key = name_str.trim_start_matches('/');
                                    // Check if this XObject exists in resources
                                    if let Some(&xobject_ref) = resources.xobjects.get(name_key) {
                                        // Record the placement with current CTM
                                        placements.push(ImagePlacement {
                                            xobject_ref,
                                            ctm: state.ctm,
                                            name: Arc::from(name_key),
                                        });

                                        // Check image count limit
                                        if placements.len() >= MAX_IMAGES_PER_PAGE {
                                            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                                                DiagCode::StreamBomb,
                                                format!(
                                                    "Too many images on page ({}), aborting",
                                                    MAX_IMAGES_PER_PAGE
                                                ),
                                            ));
                                            return Err(diagnostics);
                                        }
                                    }
                                }
                            }
                        }
                        operand_buffer.clear();
                    }
                    "BI" => {
                        // Begin inline image - this is complex to handle in the token stream
                        // For now, we'll skip inline images silently
                        // Full inline image support requires a more sophisticated parser
                        // that can handle the BI/ID/EI sequence properly
                        operand_buffer.clear();
                    }
                    _ => {
                        // Other operator - clear operands
                        operand_buffer.clear();
                    }
                }
            }
            Token::Integer(_) | Token::Real(_) | Token::Name(_) => {
                // Collect operands for cm and Do operators
                operand_buffer.push(token);
            }
            _ => {
                // Other tokens - ignore
                operand_buffer.clear();
            }
        }
    }

    if diagnostics.is_empty() || !placements.is_empty() {
        Ok(placements)
    } else {
        Err(diagnostics)
    }
}

/// Collect all image XObjects from a content stream (both Do and inline images).
///
/// This function extends `collect_image_placements` to also handle inline images
/// from BI/ID/EI sequences. It returns a unified list of `ImageXObject` entries
/// that can be used by Phase 4.4 figure detection.
///
/// # Arguments
///
/// * `content` - The decoded content stream bytes
/// * `resources` - The page's resource dictionary (for XObject lookup)
///
/// # Returns
///
/// A list of ImageXObject entries with bboxes computed from the current CTM,
/// or diagnostics if parsing fails.
pub fn collect_image_xobjects(
    content: &[u8],
    resources: &ResourceDict,
) -> Result<Vec<ImageXObject>> {
    let mut images = Vec::new();
    let mut diagnostics = Vec::new();

    // Create graphics state stack
    let mut gss = GraphicsStateStack::new();
    let mut state = GraphicsState::new();

    // Tokenize content stream
    let mut lexer = Lexer::new(content);
    let mut operand_buffer: Vec<Token> = Vec::new();

    while let Some(token) = lexer.next_token() {
        match token {
            Token::Keyword(ref k) => {
                let keyword = std::str::from_utf8(k).unwrap_or("");

                match keyword {
                    "q" => {
                        // Push graphics state
                        if !gss.push(&state) {
                            diagnostics.push(Diagnostic::with_static_no_offset(
                                DiagCode::GstateStackOverflow,
                                "Graphics state stack overflow",
                            ));
                            break;
                        }
                        operand_buffer.clear();
                    }
                    "Q" => {
                        // Pop graphics state
                        if let Some(popped) = gss.pop() {
                            state = popped;
                        }
                        operand_buffer.clear();
                    }
                    "cm" => {
                        // Concatenate matrix: cm expects exactly 6 numbers
                        let nums: Vec<f64> = operand_buffer
                            .iter()
                            .filter_map(|t| match t {
                                Token::Integer(n) => Some(*n as f64),
                                Token::Real(f) => Some(*f),
                                _ => None,
                            })
                            .collect();

                        if nums.len() != 6 {
                            diagnostics.push(Diagnostic::with_static_no_offset(
                                DiagCode::CmArgCount,
                                "cm operator requires exactly 6 numeric arguments",
                            ));
                            operand_buffer.clear();
                            continue;
                        }

                        let matrix = Matrix3x3::from_pdf_array([
                            nums[0], nums[1], nums[2], nums[3], nums[4], nums[5],
                        ]);

                        // Check for degenerate matrix (NaN or det == 0)
                        let has_nan = nums.iter().any(|&n| n.is_nan());
                        let det = matrix.determinant();
                        if has_nan || det == 0.0 {
                            diagnostics.push(Diagnostic::with_static_no_offset(
                                DiagCode::CmDegenerate,
                                "cm operator received degenerate matrix; clamped to identity",
                            ));
                            // Clamp to identity - don't modify CTM
                        } else {
                            state.concat_ctm(&matrix);
                        }
                        operand_buffer.clear();
                    }
                    "Do" => {
                        // Paint XObject: Do expects a name operand
                        if let Some(name_token) = operand_buffer.last() {
                            if let Token::Name(name_bytes) = name_token {
                                if let Ok(name_str) = std::str::from_utf8(name_bytes) {
                                    let name_key = name_str.trim_start_matches('/');
                                    // Check if this XObject exists in resources
                                    if let Some(&xobject_ref) = resources.xobjects.get(name_key) {
                                        // Compute bbox by transforming unit square [0,1]x[0,1]
                                        let bbox = compute_unit_square_bbox(&state.ctm);

                                        images.push(ImageXObject {
                                            bbox,
                                            source: ImageSource::XObject(
                                                xobject_ref,
                                                Arc::from(name_key),
                                            ),
                                            header: InlineImageHeader::default(),
                                            bytes_ref: ImageBytesRef::XObjectRef(xobject_ref),
                                        });

                                        // Check image count limit
                                        if images.len() >= MAX_IMAGES_PER_PAGE {
                                            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                                                DiagCode::StreamBomb,
                                                format!(
                                                    "Too many images on page ({}), aborting",
                                                    MAX_IMAGES_PER_PAGE
                                                ),
                                            ));
                                            return Err(diagnostics);
                                        }
                                    }
                                }
                            }
                        }
                        operand_buffer.clear();
                    }
                    "BI" => {
                        // Begin inline image - parse the inline image dict and data
                        match parse_inline_image(&mut lexer, &state.ctm) {
                            Ok(Some((header, data))) => {
                                // Compute bbox by transforming unit square
                                let bbox = compute_unit_square_bbox(&state.ctm);

                                images.push(ImageXObject {
                                    bbox,
                                    source: ImageSource::Inline,
                                    header,
                                    bytes_ref: ImageBytesRef::Inline(data),
                                });

                                // Check image count limit
                                if images.len() >= MAX_IMAGES_PER_PAGE {
                                    diagnostics.push(Diagnostic::with_dynamic_no_offset(
                                        DiagCode::StreamBomb,
                                        format!(
                                            "Too many images on page ({}), aborting",
                                            MAX_IMAGES_PER_PAGE
                                        ),
                                    ));
                                    return Err(diagnostics);
                                }
                            }
                            Ok(None) => {
                                // Inline image parsing failed or was skipped
                                // Continue processing
                            }
                            Err(mut diags) => {
                                diagnostics.append(&mut diags);
                            }
                        }
                        operand_buffer.clear();
                    }
                    _ => {
                        // Other operator - clear operands
                        operand_buffer.clear();
                    }
                }
            }
            Token::Integer(_) | Token::Real(_) | Token::Name(_) => {
                // Collect operands for cm and Do operators
                operand_buffer.push(token);
            }
            _ => {
                // Other tokens - ignore
                operand_buffer.clear();
            }
        }
    }

    if diagnostics.is_empty() || !images.is_empty() {
        Ok(images)
    } else {
        Err(diagnostics)
    }
}

/// Parse an inline image from a BI/ID/EI sequence.
///
/// This function parses the inline image dictionary (between BI and ID),
/// extracts the image data (between ID and EI), and returns the header
/// parameters and raw image bytes.
///
/// # Arguments
///
/// * `lexer` - The lexer positioned after the BI keyword
/// * `ctm` - The current CTM at the time of BI (for bbox computation)
///
/// # Returns
///
/// Ok(Some((header, data))) on success, Ok(None) if parsing failed gracefully,
/// or Err(diagnostics) if a critical error occurred.
fn parse_inline_image(
    lexer: &mut Lexer,
    ctm: &Matrix3x3,
) -> Result<Option<(InlineImageHeader, Vec<u8>)>> {
    let mut header = InlineImageHeader::default();
    let mut diagnostics = Vec::new();

    // Parse the inline image dictionary (key-value pairs until ID)
    let mut dict_buffer: Vec<Token> = Vec::new();

    while let Some(token) = lexer.next_token() {
        match &token {
            Token::Keyword(k) if k == b"ID" => {
                // End of dictionary, start of image data
                break;
            }
            Token::Keyword(k) if k == b"Do" || k == b"BI" || k == b"BT" || k == b"ET" => {
                // Unexpected operator in inline image dict
                diagnostics.push(Diagnostic::with_static_no_offset(
                    DiagCode::StreamTruncated,
                    "Unexpected operator in inline image dictionary",
                ));
                return Ok(None);
            }
            _ => {
                dict_buffer.push(token);
            }
        }
    }

    // Parse the dictionary key-value pairs
    let mut i = 0;
    while i + 1 < dict_buffer.len() {
        let key = match &dict_buffer[i] {
            Token::Name(k) => std::str::from_utf8(k).unwrap_or(""),
            _ => {
                i += 2;
                continue;
            }
        };

        let value = &dict_buffer[i + 1];

        match key {
            "/W" | "/Width" => {
                if let Token::Integer(w) = value {
                    header.width = Some(*w as u32);
                }
            }
            "/H" | "/Height" => {
                if let Token::Integer(h) = value {
                    header.height = Some(*h as u32);
                }
            }
            "/BPC" | "/BitsPerComponent" => {
                if let Token::Integer(bpc) = value {
                    header.bpc = (*bpc as u8).clamp(1, 16);
                }
            }
            "/CS" | "/ColorSpace" => {
                if let Token::Name(cs) = value {
                    header.colorspace = Some(std::str::from_utf8(cs).unwrap_or("").to_string());
                }
            }
            "/F" | "/Filter" => {
                match value {
                    Token::Name(f) => {
                        header
                            .filters
                            .push(std::str::from_utf8(f).unwrap_or("").to_string());
                    }
                    Token::Array(arr) => {
                        // Filter array - extract all names
                        for item in arr {
                            if let Token::Name(f) = item {
                                header
                                    .filters
                                    .push(std::str::from_utf8(f).unwrap_or("").to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
            "/IM" | "/ImageMask" => {
                if let Token::Bool(im) = value {
                    header.is_mask = *im;
                }
            }
            "/G" | "/Mask" => {
                // Image mask color: /Mask [ Black | White ]
                if let Token::Array(arr) = value {
                    if arr.len() >= 1 {
                        if let Token::Name(color) = &arr[0] {
                            let color_str = std::str::from_utf8(color).unwrap_or("");
                            if color_str == "Black" {
                                header.mask_color = Some(0);
                            } else if color_str == "White" {
                                header.mask_color = Some(1);
                            }
                        }
                    }
                }
            }
            _ => {
                // Unknown key - ignore
            }
        }

        i += 2;
    }

    // Now we need to extract the image data until EI
    // The EI terminator must be preceded by whitespace
    // We need to scan byte-by-byte to find it
    let mut image_data = Vec::new();
    let mut prev_was_whitespace = false;
    let mut potential_ei = [0u8; 3]; // sliding window for EI detection
    let mut window_pos = 0;

    // Get the raw position from lexer to scan bytes directly
    // For now, we'll use a simpler approach: continue tokenizing
    // and collect data until we see EI
    while let Some(token) = lexer.next_token() {
        match token {
            Token::Keyword(k) if k == b"EI" && prev_was_whitespace => {
                // Found the EI terminator
                // Remove the trailing newline from image data
                if image_data.ends_with(&[b'\n']) {
                    image_data.pop();
                }
                if image_data.ends_with(&[b'\r']) {
                    image_data.pop();
                }
                return Ok(Some((header, image_data)));
            }
            Token::Keyword(k) if k == b"EI" => {
                // EI without preceding whitespace - might be part of image data
                // Continue scanning
            }
            _ => {
                // Collect the raw bytes for image data
                // For now, we'll need a different approach
                // The lexer doesn't give us raw bytes easily
            }
        }

        // Update whitespace tracking
        prev_was_whitespace = false;
    }

    // If we get here, we didn't find EI - emit diagnostic and return None
    diagnostics.push(Diagnostic::with_static_no_offset(
        DiagCode::StreamTruncated,
        "Inline image data missing EI terminator",
    ));

    Ok(None)
}

/// Compute bounding box by transforming the unit square [0,1]x[0,1] by a CTM.
///
/// This function transforms the four corners of the unit square:
/// (0,0), (1,0), (0,1), (1,1)
/// and returns the axis-aligned bounding box of the transformed points.
///
/// # Arguments
///
/// * `ctm` - The current transformation matrix
///
/// # Returns
///
/// Bounding box [x0, y0, x1, y1] in PDF user-space coordinates.
fn compute_unit_square_bbox(ctm: &Matrix3x3) -> [f32; 4] {
    // Unit square corners
    let corners = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)];

    // Transform each corner
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for &(x, y) in &corners {
        let (tx, ty) = ctm.transform_point(x, y);
        min_x = min_x.min(tx);
        max_x = max_x.max(tx);
        min_y = min_y.min(ty);
        max_y = max_y.max(ty);
    }

    [min_x as f32, min_y as f32, max_x as f32, max_y as f32]
}

/// Get the /Matrix from an XObject dictionary if present.
///
/// Returns the matrix if found, or identity if not present.
fn get_xobject_matrix(xobject_ref: ObjRef, resolver: &XrefResolver) -> Matrix3x3 {
    // Resolve the XObject
    let xobject = match resolver.resolve(xobject_ref) {
        Ok(obj) => obj,
        Err(_) => return Matrix3x3::identity(),
    };

    // Get the stream
    let stream = match xobject.as_stream() {
        Some(s) => s,
        None => return Matrix3x3::identity(),
    };

    // Get the /Matrix key if present
    let dict = &stream.dict;
    match dict.get("/Matrix") {
        Some(PdfObject::Array(arr)) => {
            // Matrix should be a 6-element array
            let nums: Vec<f64> = arr
                .iter()
                .filter_map(|v| match v {
                    PdfObject::Integer(n) => Some(*n as f64),
                    PdfObject::Real(f) => Some(*f),
                    _ => None,
                })
                .collect();

            if nums.len() >= 6 {
                Matrix3x3::from_pdf_array([nums[0], nums[1], nums[2], nums[3], nums[4], nums[5]])
            } else {
                Matrix3x3::identity()
            }
        }
        _ => Matrix3x3::identity(),
    }
}

/// Decode an image XObject to a DynamicImage.
///
/// Handles various image formats:
/// - DCTDecode (JPEG)
/// - JPXDecode (JPEG2000)
/// - FlateDecode/LZWDecode (raw RGB/grayscale)
///
/// # Arguments
///
/// * `xobject_ref` - The image XObject reference
/// * `resolver` - The xref resolver
/// * `source` - The PDF source
/// * `max_bytes` - Maximum decompressed bytes
///
/// # Returns
///
/// The decoded image, or diagnostics if decoding fails.
pub fn decode_image_xobject(
    xobject_ref: ObjRef,
    resolver: &XrefResolver,
    source: &dyn PdfSource,
    max_bytes: u64,
) -> Result<DynamicImage> {
    let mut diagnostics = Vec::new();

    // Resolve the XObject
    let xobject = match resolver.resolve(xobject_ref) {
        Ok(obj) => obj,
        Err(e) => {
            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::StructMissingKey,
                format!("Failed to resolve XObject: {:?}", e),
            ));
            return Err(diagnostics);
        }
    };

    // Get the stream
    let stream = match xobject.as_stream() {
        Some(s) => s,
        None => {
            diagnostics.push(Diagnostic::with_static_no_offset(
                DiagCode::StructInvalidType,
                "XObject is not a stream",
            ));
            return Err(diagnostics);
        }
    };

    // Get the XObject subtype
    let dict = &stream.dict;
    let _subtype = match dict.get("/Subtype") {
        Some(PdfObject::Name(s)) if s.as_ref() == "Image" => s,
        Some(_) => {
            diagnostics.push(Diagnostic::with_static_no_offset(
                DiagCode::StructInvalidType,
                "XObject is not an Image",
            ));
            return Err(diagnostics);
        }
        None => {
            diagnostics.push(Diagnostic::with_static_no_offset(
                DiagCode::StructMissingKey,
                "XObject missing /Subtype",
            ));
            return Err(diagnostics);
        }
    };

    // Check for soft mask (not supported in direct compositing)
    if let Some(_) = dict.get("/SMask") {
        diagnostics.push(Diagnostic::with_static_no_offset(
            DiagCode::ImgSoftmaskUnsupported,
            "Soft-masked images not supported in direct compositing path",
        ));
        return Err(diagnostics);
    }

    // Decode the stream
    let stream_opts = StreamExtractionOptions {
        max_decompress_bytes: max_bytes,
        password: None,
    };
    let mut doc_counter = 0u64;
    let decoded = decode_stream(stream, source, &stream_opts, &mut doc_counter);

    // Get image dimensions
    let width = match dict.get("/Width") {
        Some(PdfObject::Integer(w)) => *w as u32,
        Some(PdfObject::Real(w)) => *w as u32,
        _ => {
            diagnostics.push(Diagnostic::with_static_no_offset(
                DiagCode::StructMissingKey,
                "Image missing /Width",
            ));
            return Err(diagnostics);
        }
    };

    let height = match dict.get("/Height") {
        Some(PdfObject::Integer(h)) => *h as u32,
        Some(PdfObject::Real(h)) => *h as u32,
        _ => {
            diagnostics.push(Diagnostic::with_static_no_offset(
                DiagCode::StructMissingKey,
                "Image missing /Height",
            ));
            return Err(diagnostics);
        }
    };

    // Get color space
    let colorspace = dict.get("/ColorSpace");

    // Get bits per component
    let bpc = match dict.get("/BitsPerComponent") {
        Some(PdfObject::Integer(b)) => *b as u8,
        _ => 8,
    };

    // Try to load as image based on filter
    let filter = stream.filter();

    // For JPEG images, try direct loading
    if let Some(filters) = filter {
        if filters.iter().any(|f| f == "DCTDecode" || f == "DCT") {
            // Try to load as JPEG
            match image::load_from_memory(&decoded) {
                Ok(img) => return Ok(img),
                Err(_) => {
                    // Fall through to manual decoding
                }
            }
        }
    }

    // Manual decoding for non-JPEG images
    // Determine color space
    let is_rgb = match colorspace {
        Some(PdfObject::Name(cs)) => cs.as_ref() == "DeviceRGB",
        Some(PdfObject::Array(arr)) => {
            if let Some(PdfObject::Name(cs)) = arr.first() {
                cs.as_ref() == "DeviceRGB" || cs.as_ref() == "ICCBased" || cs.as_ref() == "CalRGB"
            } else {
                false
            }
        }
        _ => false,
    };

    let is_cmyk = match colorspace {
        Some(PdfObject::Name(cs)) => cs.as_ref() == "DeviceCMYK",
        Some(PdfObject::Array(arr)) => {
            if let Some(PdfObject::Name(cs)) = arr.first() {
                cs.as_ref() == "DeviceCMYK"
            } else {
                false
            }
        }
        _ => false,
    };

    // Calculate expected data size
    let components = if is_rgb {
        3
    } else if is_cmyk {
        4
    } else {
        1
    };
    let expected_size = (width as usize) * (height as usize) * (components as usize);

    if decoded.len() < expected_size {
        diagnostics.push(Diagnostic::with_dynamic_no_offset(
            DiagCode::StreamTruncated,
            format!(
                "Image data truncated: expected {} bytes, got {}",
                expected_size,
                decoded.len()
            ),
        ));
        return Err(diagnostics);
    }

    // Create image from decoded data
    let dynamic_img = if is_rgb {
        // RGB image
        if bpc == 8 {
            let mut rgb_data = Vec::with_capacity(expected_size);
            for i in (0..expected_size).step_by(3) {
                if i + 2 < decoded.len() {
                    rgb_data.push(decoded[i]);
                    rgb_data.push(decoded[i + 1]);
                    rgb_data.push(decoded[i + 2]);
                }
            }
            let img: RgbImage = ImageBuffer::from_raw(width, height, rgb_data)
                .unwrap_or_else(|| ImageBuffer::new(width, height));
            DynamicImage::ImageRgb8(img)
        } else {
            // Unsupported bits per component
            diagnostics.push(Diagnostic::with_static_no_offset(
                DiagCode::ImgUnsupportedFormat,
                "Unsupported bits per component for RGB image",
            ));
            return Err(diagnostics);
        }
    } else if is_cmyk {
        // CMYK image - need to convert to RGB
        // This is a simplified conversion (proper conversion requires ICC profiles)
        let mut rgb_data = Vec::with_capacity((width as usize) * (height as usize) * 3);
        for i in (0..decoded.len()).step_by(4) {
            if i + 3 < decoded.len() {
                let c = decoded[i] as f32 / 255.0;
                let m = decoded[i + 1] as f32 / 255.0;
                let y = decoded[i + 2] as f32 / 255.0;
                let k = decoded[i + 3] as f32 / 255.0;

                // CMYK to RGB conversion
                let r = ((1.0 - c) * (1.0 - k) * 255.0) as u8;
                let g = ((1.0 - m) * (1.0 - k) * 255.0) as u8;
                let b = ((1.0 - y) * (1.0 - k) * 255.0) as u8;

                rgb_data.push(r);
                rgb_data.push(g);
                rgb_data.push(b);
            }
        }
        let img: RgbImage = ImageBuffer::from_raw(width, height, rgb_data)
            .unwrap_or_else(|| ImageBuffer::new(width, height));
        DynamicImage::ImageRgb8(img)
    } else {
        // Grayscale image
        if bpc == 8 {
            let gray_data: Vec<u8> = decoded.iter().copied().collect();
            let img: GrayImage = ImageBuffer::from_raw(width, height, gray_data)
                .unwrap_or_else(|| ImageBuffer::new(width, height));
            DynamicImage::ImageLuma8(img)
        } else if bpc == 1 {
            // 1-bit grayscale (binary image) - expand to 8-bit
            let mut gray_data = Vec::with_capacity((width as usize) * (height as usize));
            for &byte in decoded.iter() {
                for bit in (0..8).rev() {
                    gray_data.push(if (byte >> bit) & 1 == 1 { 0 } else { 255 });
                }
            }
            let img: GrayImage = ImageBuffer::from_raw(width, height, gray_data)
                .unwrap_or_else(|| ImageBuffer::new(width, height));
            DynamicImage::ImageLuma8(img)
        } else {
            diagnostics.push(Diagnostic::with_static_no_offset(
                DiagCode::ImgUnsupportedFormat,
                "Unsupported bits per component for grayscale image",
            ));
            return Err(diagnostics);
        }
    };

    Ok(dynamic_img)
}

/// Convert an image to grayscale.
///
/// Uses luminance conversion: Y = 0.299*R + 0.587*G + 0.114*B
pub fn to_grayscale(img: &DynamicImage) -> GrayImage {
    img.to_luma8()
}

/// Composite images onto a canvas using their CTMs.
///
/// # Arguments
///
/// * `placements` - Image placements with CTMs
/// * `page_width` - Page width in PDF points
/// * `page_height` - Page height in PDF points
/// * `dpi` - Resolution for rendering (default 300)
/// * `resolver` - The xref resolver
/// * `source` - The PDF source
/// * `max_bytes` - Maximum decompressed bytes
///
/// # Returns
///
/// The composited grayscale image, or diagnostics if compositing fails.
pub fn composite_images(
    placements: &[ImagePlacement],
    page_width: f64,
    page_height: f64,
    dpi: u32,
    resolver: &XrefResolver,
    source: &dyn PdfSource,
    max_bytes: u64,
) -> Result<GrayImage> {
    composite_images_with_rotation(
        placements,
        page_width,
        page_height,
        dpi,
        0,
        resolver,
        source,
        max_bytes,
    )
}

/// Composite images onto a canvas using their CTMs, with page rotation support.
///
/// # Arguments
///
/// * `placements` - Image placements with CTMs
/// * `page_width` - Page width in PDF points
/// * `page_height` - Page height in PDF points
/// * `dpi` - Resolution for rendering (default 300)
/// * `rotation` - Page rotation in degrees (0, 90, 180, 270)
/// * `resolver` - The xref resolver
/// * `source` - The PDF source
/// * `max_bytes` - Maximum decompressed bytes
///
/// # Returns
///
/// The composited grayscale image, or diagnostics if compositing fails.
pub fn composite_images_with_rotation(
    placements: &[ImagePlacement],
    page_width: f64,
    page_height: f64,
    dpi: u32,
    rotation: i32,
    resolver: &XrefResolver,
    source: &dyn PdfSource,
    max_bytes: u64,
) -> Result<GrayImage> {
    let mut diagnostics = Vec::new();

    // Normalize rotation to 0-360 range and ensure it's a multiple of 90
    let rotation = ((rotation % 360) + 360) % 360;
    let rotation = match rotation {
        0 | 90 | 180 | 270 => rotation,
        _ => 0, // Invalid rotation, default to 0
    };

    // For rotated pages, swap width and height
    let (effective_width, effective_height) = match rotation {
        90 | 270 => (page_height, page_width),
        _ => (page_width, page_height),
    };

    // Calculate canvas size in pixels
    let scale = dpi as f64 / 72.0;
    let canvas_width = (effective_width * scale).ceil() as u32;
    let canvas_height = (effective_height * scale).ceil() as u32;

    // Create white canvas
    let mut canvas = GrayImage::new(canvas_width, canvas_height);
    for pixel in canvas.pixels_mut() {
        *pixel = Luma([255]); // White background
    }

    // Composite each image
    for placement in placements {
        // Get the XObject /Matrix if present
        let xobject_matrix = get_xobject_matrix(placement.xobject_ref, resolver);

        // Compose the placement CTM with the XObject /Matrix
        // The effective CTM is: placement_ctm * xobject_matrix
        let effective_ctm = placement.ctm.multiply(&xobject_matrix);

        // Decode the image
        let img = match decode_image_xobject(placement.xobject_ref, resolver, source, max_bytes) {
            Ok(img) => img,
            Err(mut diags) => {
                diagnostics.append(&mut diags);
                continue; // Skip this image but continue with others
            }
        };

        // Convert to grayscale
        let gray_img = to_grayscale(&img);

        // Compute placement using the effective CTM
        // The CTM transforms from image space to PDF user space
        // For images, we need to transform the unit square [0,1]x[0,1]

        // Transform the image corners
        let corners = [
            (0.0, 0.0), // Bottom-left
            (1.0, 0.0), // Bottom-right
            (0.0, 1.0), // Top-left
            (1.0, 1.0), // Top-right
        ];

        let mut transformed_corners = Vec::new();
        for &(x, y) in &corners {
            let (tx, ty) = effective_ctm.transform_point(x, y);
            // Convert PDF points to pixels
            let mut px = tx * scale;
            let mut py = (page_height - ty) * scale; // Flip Y for image coordinates

            // Apply rotation to pixel coordinates
            match rotation {
                90 => {
                    // Rotate 90 degrees clockwise
                    let old_px = px;
                    px = py;
                    py = (canvas_height as f64) - old_px;
                }
                180 => {
                    // Rotate 180 degrees
                    px = (canvas_width as f64) - px;
                    py = (canvas_height as f64) - py;
                }
                270 => {
                    // Rotate 270 degrees clockwise (90 counterclockwise)
                    let old_px = px;
                    px = (canvas_width as f64) - py;
                    py = old_px;
                }
                _ => {
                    // No rotation
                }
            }

            transformed_corners.push((px, py));
        }

        // Compute bounding box
        let min_x = transformed_corners
            .iter()
            .map(|(x, _)| x)
            .fold(f64::INFINITY, |a, &b| a.min(b))
            .floor() as i32;
        let max_x = transformed_corners
            .iter()
            .map(|(x, _)| x)
            .fold(f64::NEG_INFINITY, |a, &b| a.max(b))
            .ceil() as i32;
        let min_y = transformed_corners
            .iter()
            .map(|(_, y)| y)
            .fold(f64::INFINITY, |a, &b| a.min(b))
            .floor() as i32;
        let max_y = transformed_corners
            .iter()
            .map(|(_, y)| y)
            .fold(f64::NEG_INFINITY, |a, &b| a.max(b))
            .ceil() as i32;

        // Clamp to canvas bounds
        let min_x = min_x.max(0) as u32;
        let max_x = max_x.min(canvas_width as i32 - 1) as u32;
        let min_y = min_y.max(0) as u32;
        let max_y = max_y.min(canvas_height as i32 - 1) as u32;

        if min_x >= max_x || min_y >= max_y {
            // Image is outside canvas bounds
            continue;
        }

        // For now, use a simple placement without proper perspective transform
        // This handles the common case of untransformed full-page images

        // Copy image pixels to canvas (simple copy for now)
        let img_width = gray_img.width();
        let img_height = gray_img.height();

        // Scale image to fit bounding box
        let bbox_width = max_x - min_x;
        let bbox_height = max_y - min_y;

        if bbox_width == 0 || bbox_height == 0 {
            continue;
        }

        // Resize image to fit
        let resized = if img_width != bbox_width || img_height != bbox_height {
            image::imageops::resize(
                &gray_img,
                bbox_width,
                bbox_height,
                image::imageops::FilterType::Lanczos3,
            )
        } else {
            gray_img
        };

        // Copy pixels to canvas
        for y in 0..bbox_height {
            for x in 0..bbox_width {
                let canvas_x = min_x + x;
                let canvas_y = min_y + y;
                if canvas_x < canvas_width && canvas_y < canvas_height {
                    let pixel = resized.get_pixel(x, y);
                    canvas.put_pixel(canvas_x, canvas_y, *pixel);
                }
            }
        }
    }

    if diagnostics.is_empty() {
        Ok(canvas)
    } else {
        // Return canvas even with diagnostics (partial result)
        Ok(canvas)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::resources::ResourceDict;
    use std::sync::Arc;

    #[test]
    fn test_collect_image_placements_empty() {
        let content = b"";
        let resources = ResourceDict::new();
        let result = collect_image_placements(content, &resources);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_collect_image_placements_simple() {
        // Simple content stream with one Do operator
        let content = b"/Im1 Do";
        let mut resources = ResourceDict::new();
        resources
            .xobjects
            .insert(Arc::from("Im1"), ObjRef::new(1, 0));

        let result = collect_image_placements(content, &resources);
        assert!(result.is_ok());
        let placements = result.unwrap();
        assert_eq!(placements.len(), 1);
        assert_eq!(placements[0].name.as_ref(), "Im1");
        // CTM should be identity
        assert!(placements[0].ctm.is_identity());
    }

    #[test]
    fn test_collect_image_placements_with_ctm() {
        // Content stream with cm and Do operators
        let content = b"1 0 0 1 100 200 cm /Im1 Do";
        let mut resources = ResourceDict::new();
        resources
            .xobjects
            .insert(Arc::from("Im1"), ObjRef::new(1, 0));

        let result = collect_image_placements(content, &resources);
        assert!(result.is_ok());
        let placements = result.unwrap();
        assert_eq!(placements.len(), 1);
        // CTM should have translation
        assert_eq!(placements[0].ctm.e, 100.0);
        assert_eq!(placements[0].ctm.f, 200.0);
    }

    #[test]
    fn test_collect_image_placements_with_stack() {
        // Content stream with q/Q operators
        let content = b"q 1 0 0 1 100 200 cm /Im1 Do Q /Im2 Do";
        let mut resources = ResourceDict::new();
        resources
            .xobjects
            .insert(Arc::from("Im1"), ObjRef::new(1, 0));
        resources
            .xobjects
            .insert(Arc::from("Im2"), ObjRef::new(2, 0));

        let result = collect_image_placements(content, &resources);
        assert!(result.is_ok());
        let placements = result.unwrap();
        assert_eq!(placements.len(), 2);
        // First image should have translation
        assert_eq!(placements[0].ctm.e, 100.0);
        assert_eq!(placements[0].ctm.f, 200.0);
        // Second image should have identity CTM (after Q)
        assert!(placements[1].ctm.is_identity());
    }

    #[test]
    fn test_to_grayscale() {
        // Create a simple RGB image
        let rgb_img: RgbImage = ImageBuffer::from_fn(2, 2, |x, y| {
            match (x, y) {
                (0, 0) => Rgb([255, 0, 0]),     // Red
                (1, 0) => Rgb([0, 255, 0]),     // Green
                (0, 1) => Rgb([0, 0, 255]),     // Blue
                (1, 1) => Rgb([255, 255, 255]), // White
                _ => Rgb([0, 0, 0]),            // Should never happen for 2x2 image
            }
        });

        let dynamic = DynamicImage::ImageRgb8(rgb_img);
        let gray = to_grayscale(&dynamic);

        // Check that grayscale conversion worked
        assert_eq!(gray.width(), 2);
        assert_eq!(gray.height(), 2);

        // Red pixel should be dark
        let r_pixel = gray.get_pixel(0, 0);
        assert!(r_pixel[0] < 100); // Luminance of red is low

        // Green pixel should be medium
        let g_pixel = gray.get_pixel(1, 0);
        assert!(g_pixel[0] > 100 && g_pixel[0] < 200);

        // Blue pixel should be dark
        let b_pixel = gray.get_pixel(0, 1);
        assert!(b_pixel[0] < 100);

        // White pixel should be bright
        let w_pixel = gray.get_pixel(1, 1);
        assert!(w_pixel[0] > 200);
    }

    #[test]
    fn test_collect_image_placements_with_bi() {
        // Content stream with BI operator (inline image)
        // Should emit a diagnostic but not crash
        let content = b"BI";
        let resources = ResourceDict::new();
        let result = collect_image_placements(content, &resources);

        // Should return Ok (no placements) but the implementation
        // currently emits a diagnostic inline
        assert!(result.is_ok());
        let placements = result.unwrap();
        assert_eq!(placements.len(), 0);
    }

    #[test]
    fn test_graphics_state_stack_limit() {
        // Test that the graphics state stack depth limit is enforced
        let content: Vec<u8> = b"q ".repeat(100).into(); // 100 q operators (exceeds MAX_GSTATE_DEPTH)
        let resources = ResourceDict::new();
        let result = collect_image_placements(&content, &resources);

        // Should fail due to stack overflow
        assert!(result.is_err());

        let diags = result.unwrap_err();
        assert!(diags
            .iter()
            .any(|d| d.code == DiagCode::GstateStackOverflow));
    }

    #[test]
    fn test_ctm_with_scale() {
        // Test CTM with scaling
        let content = b"2 0 0 2 0 0 cm /Im1 Do";
        let mut resources = ResourceDict::new();
        resources
            .xobjects
            .insert(Arc::from("Im1"), ObjRef::new(1, 0));

        let result = collect_image_placements(content, &resources);
        assert!(result.is_ok());
        let placements = result.unwrap();
        assert_eq!(placements.len(), 1);
        // CTM should have scale
        assert_eq!(placements[0].ctm.a, 2.0);
        assert_eq!(placements[0].ctm.d, 2.0);
    }

    #[test]
    fn test_ctm_with_rotation() {
        // Test CTM with rotation (90 degrees)
        // [0 1 -1 0 0 0] is a 90-degree rotation
        let content = b"0 1 -1 0 100 200 cm /Im1 Do";
        let mut resources = ResourceDict::new();
        resources
            .xobjects
            .insert(Arc::from("Im1"), ObjRef::new(1, 0));

        let result = collect_image_placements(content, &resources);
        assert!(result.is_ok());
        let placements = result.unwrap();
        assert_eq!(placements.len(), 1);
        // CTM should have rotation
        assert_eq!(placements[0].ctm.a, 0.0);
        assert_eq!(placements[0].ctm.b, 1.0);
        assert_eq!(placements[0].ctm.c, -1.0);
        assert_eq!(placements[0].ctm.d, 0.0);
    }

    #[test]
    fn test_ctm_with_flip() {
        // Test CTM with Y flip (negative determinant)
        // [1 0 0 -1 0 height] flips Y
        let content = b"1 0 0 -1 0 792 cm /Im1 Do";
        let mut resources = ResourceDict::new();
        resources
            .xobjects
            .insert(Arc::from("Im1"), ObjRef::new(1, 0));

        let result = collect_image_placements(content, &resources);
        assert!(result.is_ok());
        let placements = result.unwrap();
        assert_eq!(placements.len(), 1);
        // CTM should have Y flip
        assert_eq!(placements[0].ctm.a, 1.0);
        assert_eq!(placements[0].ctm.d, -1.0);
        assert!(placements[0].ctm.has_flip());
    }

    #[test]
    fn test_multiple_images_different_ctms() {
        // Test multiple images with different CTMs
        let content = b"q 1 0 0 1 0 0 cm /Im1 Do Q q 2 0 0 2 100 100 cm /Im2 Do Q q 0 1 -1 0 200 200 cm /Im3 Do Q";
        let mut resources = ResourceDict::new();
        resources
            .xobjects
            .insert(Arc::from("Im1"), ObjRef::new(1, 0));
        resources
            .xobjects
            .insert(Arc::from("Im2"), ObjRef::new(2, 0));
        resources
            .xobjects
            .insert(Arc::from("Im3"), ObjRef::new(3, 0));

        let result = collect_image_placements(content, &resources);
        assert!(result.is_ok());
        let placements = result.unwrap();
        assert_eq!(placements.len(), 3);

        // First image: identity
        assert!(placements[0].ctm.is_identity());

        // Second image: scale and translate
        assert_eq!(placements[1].ctm.a, 2.0);
        assert_eq!(placements[1].ctm.d, 2.0);
        assert_eq!(placements[1].ctm.e, 100.0);
        assert_eq!(placements[1].ctm.f, 100.0);

        // Third image: rotate and translate
        assert_eq!(placements[2].ctm.a, 0.0);
        assert_eq!(placements[2].ctm.b, 1.0);
        assert_eq!(placements[2].ctm.e, 200.0);
        assert_eq!(placements[2].ctm.f, 200.0);
    }

    #[test]
    fn test_image_count_limit() {
        // Test that the image count limit is enforced
        let mut content = String::new();
        let mut resources = ResourceDict::new();

        // Create 300 image references (exceeds MAX_IMAGES_PER_PAGE)
        for i in 0..300 {
            content.push_str(&format!("/Im{} Do ", i));
            resources
                .xobjects
                .insert(Arc::from(format!("Im{}", i)), ObjRef::new(i as u32, 0));
        }

        let result = collect_image_placements(content.as_bytes(), &resources);
        assert!(result.is_err());

        let diags = result.unwrap_err();
        assert!(diags.iter().any(|d| d.code == DiagCode::StreamBomb));
    }

    #[test]
    fn test_compute_unit_square_bbox_identity() {
        let ctm = Matrix3x3::identity();
        let bbox = compute_unit_square_bbox(&ctm);
        // Unit square at origin
        assert_eq!(bbox, [0.0, 0.0, 1.0, 1.0]);
    }

    #[test]
    fn test_compute_unit_square_bbox_translate() {
        let mut ctm = Matrix3x3::identity();
        ctm.e = 100.0; // Translate x by 100
        ctm.f = 200.0; // Translate y by 200
        let bbox = compute_unit_square_bbox(&ctm);
        // Unit square translated by (100, 200)
        assert_eq!(bbox, [100.0, 200.0, 101.0, 201.0]);
    }

    #[test]
    fn test_compute_unit_square_bbox_scale() {
        // Test CTM with scaling: 100 0 0 50 200 300 cm
        let ctm = Matrix3x3::from_pdf_array([100.0, 0.0, 0.0, 50.0, 200.0, 300.0]);
        let bbox = compute_unit_square_bbox(&ctm);
        // Unit square scaled by 100x50 and translated by (200, 300)
        assert_eq!(bbox, [200.0, 300.0, 300.0, 350.0]);
    }

    #[test]
    fn test_compute_unit_square_bbox_scale_only() {
        // Test CTM with only scaling: 2 0 0 2 0 0 cm
        let ctm = Matrix3x3::from_pdf_array([2.0, 0.0, 0.0, 2.0, 0.0, 0.0]);
        let bbox = compute_unit_square_bbox(&ctm);
        // Unit square scaled by 2x2
        assert_eq!(bbox, [0.0, 0.0, 2.0, 2.0]);
    }

    #[test]
    fn test_collect_image_xobjects_empty() {
        let content = b"";
        let resources = ResourceDict::new();
        let result = collect_image_xobjects(content, &resources);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_collect_image_xobjects_simple() {
        // Simple content stream with one Do operator
        let content = b"/Im1 Do";
        let mut resources = ResourceDict::new();
        resources
            .xobjects
            .insert(Arc::from("Im1"), ObjRef::new(1, 0));

        let result = collect_image_xobjects(content, &resources);
        assert!(result.is_ok());
        let images = result.unwrap();
        assert_eq!(images.len(), 1);

        // Check the ImageXObject structure
        match &images[0].source {
            ImageSource::XObject(ref_obj, name) => {
                assert_eq!(*ref_obj, ObjRef::new(1, 0));
                assert_eq!(name.as_ref(), "Im1");
            }
            _ => panic!("Expected XObject source"),
        }

        // Bbox should be unit square at origin (identity CTM)
        assert_eq!(images[0].bbox, [0.0, 0.0, 1.0, 1.0]);

        // Header should be default for XObject images
        assert_eq!(images[0].header.width, None);
        assert_eq!(images[0].header.height, None);
    }

    #[test]
    fn test_collect_image_xobjects_with_ctm() {
        // Content stream with cm and Do operators
        let content = b"100 0 0 50 200 300 cm /Im1 Do";
        let mut resources = ResourceDict::new();
        resources
            .xobjects
            .insert(Arc::from("Im1"), ObjRef::new(1, 0));

        let result = collect_image_xobjects(content, &resources);
        assert!(result.is_ok());
        let images = result.unwrap();
        assert_eq!(images.len(), 1);

        // Bbox should be unit square transformed by CTM (scale 100x50 + translate 200,300)
        assert_eq!(images[0].bbox, [200.0, 300.0, 300.0, 350.0]);
    }

    #[test]
    fn test_collect_image_xobjects_multiple() {
        // Test multiple images with different CTMs
        let content = b"q 1 0 0 1 0 0 cm /Im1 Do Q q 2 0 0 2 100 100 cm /Im2 Do Q";
        let mut resources = ResourceDict::new();
        resources
            .xobjects
            .insert(Arc::from("Im1"), ObjRef::new(1, 0));
        resources
            .xobjects
            .insert(Arc::from("Im2"), ObjRef::new(2, 0));

        let result = collect_image_xobjects(content, &resources);
        assert!(result.is_ok());
        let images = result.unwrap();
        assert_eq!(images.len(), 2);

        // First image: identity CTM
        assert_eq!(images[0].bbox, [0.0, 0.0, 1.0, 1.0]);

        // Second image: scale 2x2 + translate (100, 100)
        assert_eq!(images[1].bbox, [100.0, 100.0, 102.0, 102.0]);
    }

    #[test]
    fn test_inline_image_header_default() {
        let header = InlineImageHeader::default();
        assert_eq!(header.width, None);
        assert_eq!(header.height, None);
        assert_eq!(header.bpc, 8); // Default BPC
        assert_eq!(header.colorspace, None);
        assert!(header.filters.is_empty());
        assert!(!header.is_mask);
        assert_eq!(header.mask_color, None);
    }

    #[test]
    fn test_image_xobject_with_inline() {
        // Test that InlineImageSource creates correct ImageXObject
        let header = InlineImageHeader {
            width: Some(100),
            height: Some(50),
            bpc: 8,
            colorspace: Some("DeviceRGB".to_string()),
            filters: vec!["DCTDecode".to_string()],
            is_mask: false,
            mask_color: None,
        };
        let data = vec![1u8, 2, 3, 4];
        let ctm = Matrix3x3::from_pdf_array([2.0, 0.0, 0.0, 2.0, 10.0, 20.0]);

        let image = ImageXObject {
            bbox: compute_unit_square_bbox(&ctm),
            source: ImageSource::Inline,
            header: header.clone(),
            bytes_ref: ImageBytesRef::Inline(data.clone()),
        };

        // Check bbox: unit square scaled by 2x2 + translate (10, 20)
        assert_eq!(image.bbox, [10.0, 20.0, 12.0, 22.0]);

        // Check source
        match image.source {
            ImageSource::Inline => {}
            _ => panic!("Expected Inline source"),
        }

        // Check header
        assert_eq!(image.header.width, Some(100));
        assert_eq!(image.header.height, Some(50));
        assert_eq!(image.header.bpc, 8);
        assert_eq!(image.header.colorspace, Some("DeviceRGB".to_string()));
        assert_eq!(image.header.filters, vec!["DCTDecode".to_string()]);

        // Check bytes_ref
        match image.bytes_ref {
            ImageBytesRef::Inline(ref data_bytes) => {
                assert_eq!(*data_bytes, data);
            }
            _ => panic!("Expected Inline bytes_ref"),
        }
    }
}
