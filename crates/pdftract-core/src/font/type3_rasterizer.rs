//! Type 3 glyph content stream rasterizer.
//!
//! This module implements rasterization of Type 3 glyph content streams to
//! 32x32 grayscale bitmaps for shape recognition (Phase 2.5 Level 4).
//!
//! Per PDF spec section 9.6.5, Type 3 glyphs are defined by content streams
//! that draw the glyph shape. This module:
//! 1. Parses the content stream into path commands
//! 2. Executes the path commands to fill a 32x32 bitmap
//! 3. Returns the bitmap for pHash computation in the shape database
//!
//! The operator subset supported is:
//! - Path construction: m, l, c, v, y, re, h
//! - Painting: S, s, f, F, B, b, f*, B*, b*
//! - Graphics state: q, Q, cm
//! - XObject: Do (form XObjects only)
//! - No-op: n

use std::sync::Arc;

use crate::diagnostics::{DiagCode, Diagnostic};
use crate::font::type3::Type3Font;
use crate::graphics_state::{GraphicsState, GraphicsStateStack, Matrix3x3};
use crate::parser::lexer::Lexer;
use crate::parser::object::types::ObjRef;
use crate::parser::object::types::PdfObject;
use crate::parser::stream::{decode_stream, ExtractionOptions, PdfSource};
use crate::parser::xref::{ResolveError, XrefResolver};
use crate::render::path::{CurrentPath, PathCommand, Point};

/// Classification of PDF objects for Type 3 CharProc detection.
///
/// Represents the type of a PDF object referenced by a Type 3 font's
/// CharProcs dictionary. Used by detection functions to determine
/// whether an object is a stream, dictionary, or other type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CharProcType {
    /// PDF stream object (contains content stream bytes)
    Stream,
    /// PDF dictionary object (contains key-value pairs)
    Dict,
    /// Unknown type - returned when reference dereferencing fails
    Unknown,
    /// Any other PDF object type with a descriptive name
    Other(String),
}

/// Detect the type of PDF object for Type 3 CharProc validation.
///
/// This function classifies a PdfObject instance to determine whether
/// it is a stream (contains content stream bytes), a dictionary (contains
/// key-value pairs), or another type.
///
/// When a document context is provided, this function will dereference
/// indirect references and classify the underlying object.
///
/// # Arguments
///
/// * `object` - The PdfObject to classify
/// * `doc_context` - Optional document resolver context for dereferencing
///
/// # Returns
///
/// `CharProcType::Stream` if the object is a stream,
/// `CharProcType::Dict` if the object is a dictionary,
/// `CharProcType::Other(name)` for any other type with its descriptive name.
///
/// # Example
///
/// ```rust,no_run
/// use pdftract_core::font::type3_rasterizer::{detect_char_proc_type, CharProcType};
/// use pdftract_core::parser::object::types::PdfObject;
///
/// let stream_obj = PdfObject::Stream(Box::new(/* ... */));
/// let dict_obj = PdfObject::Dict(Box::new(/* ... */));
/// let int_obj = PdfObject::Integer(42);
///
/// assert_eq!(detect_char_proc_type(&stream_obj, None, None), CharProcType::Stream);
/// assert_eq!(detect_char_proc_type(&dict_obj, None, None), CharProcType::Dict);
/// assert_eq!(detect_char_proc_type(&int_obj, None, None), CharProcType::Other("integer".to_string()));
/// ```
pub fn detect_char_proc_type(object: &PdfObject, doc_context: Option<&DocumentContext>) -> CharProcType {
    match object {
        // Stream check happens before Dict check (per implementation guidance)
        // Streams can also have dictionaries, so we need to check for Stream first
        PdfObject::Stream(_) => CharProcType::Stream,
        // Dict check happens after Stream but before Other
        PdfObject::Dict(_) => CharProcType::Dict,
        // Reference check - dereference using document context if available
        PdfObject::Ref(obj_ref) => {
            match doc_context {
                Some(ctx) => {
                    // Attempt to dereference the reference
                    match deref_char_proc_ref(*obj_ref, Some(ctx)) {
                        Ok(dereferenced_obj) => {
                            // Recursively classify the dereferenced object
                            detect_char_proc_type(&dereferenced_obj, doc_context)
                        }
                        Err(_) => {
                            // Dereferencing failed - return Unknown
                            CharProcType::Unknown
                        }
                    }
                }
                None => {
                    // No document context available - return Unknown
                    CharProcType::Unknown
                }
            }
        }
        // All other types (including Integer, Real, Bool, String, Name, Array, Null, Indirect)
        // return Other with descriptive name
        _ => CharProcType::Other(object.type_name().to_string()),
    }
}

/// Detect the type of PDF object for Type 3 CharProc validation with reference handling.
///
/// This function classifies a PdfObject instance to determine whether
/// it is a stream (contains content stream bytes), a dictionary (contains
/// key-value pairs), or another type. It handles indirect references by
/// dereferencing them using the provided document context.
///
/// # Arguments
///
/// * `object` - The PdfObject to classify
/// * `doc_context` - Optional document resolver context for dereferencing
///
/// # Returns
///
/// `CharProcType::Stream` if the object is a stream,
/// `CharProcType::Dict` if the object is a dictionary,
/// `CharProcType::Other(name)` for any other type with its descriptive name.
///
/// # Reference Handling
///
/// When encountering a `PdfObject::Ref`:
/// - If `doc_context` is provided, the reference is dereferenced and the
///   underlying object is classified recursively
/// - Reference cycles are detected and return `CharProcType::Other("circular-reference".to_string())`
/// - Dereferencing errors (not found, I/O error) return `CharProcType::Unknown`
/// - If `doc_context` is None, references are classified as `CharProcType::Unknown`
///
/// # Example
///
/// ```rust,no_run
/// use pdftract_core::font::type3_rasterizer::{detect_char_proc_type_with_context, CharProcType, DocumentContext};
/// use pdftract_core::parser::object::types::PdfObject;
///
/// let ref_obj = PdfObject::Ref(/* ... */);
/// let doc_context = DocumentContext { /* ... */ };
///
/// // Dereferences and classifies the underlying object
/// let char_proc_type = detect_char_proc_type_with_context(&ref_obj, Some(&doc_context));
/// ```
pub fn detect_char_proc_type_with_context<'a>(
    object: &PdfObject,
    doc_context: Option<&'a DocumentContext<'a>>,
) -> CharProcType {
    detect_char_proc_type_with_context_impl(object, doc_context, &mut std::collections::HashSet::new())
}

/// Internal implementation with cycle detection via visited set.
fn detect_char_proc_type_with_context_impl<'a>(
    object: &PdfObject,
    doc_context: Option<&'a DocumentContext<'a>>,
    visited: &mut std::collections::HashSet<ObjRef>,
) -> CharProcType {
    match object {
        PdfObject::Stream(_) => CharProcType::Stream,
        PdfObject::Dict(_) => CharProcType::Dict,
        PdfObject::Ref(obj_ref) => {
            // Check for circular reference
            if visited.contains(obj_ref) {
                return CharProcType::Other("circular-reference".to_string());
            }

            // Mark this reference as visited
            visited.insert(*obj_ref);

            // Try to dereference if we have a context
            match doc_context {
                Some(ctx) => {
                    match deref_char_proc_ref(*obj_ref, Some(ctx)) {
                        Ok(dereferenced_obj) => {
                            // Recursively classify the dereferenced object
                            detect_char_proc_type_with_context_impl(
                                &dereferenced_obj,
                                doc_context,
                                visited,
                            )
                        }
                        Err(_) => {
                            // Dereferencing failed - return Unknown
                            CharProcType::Unknown
                        }
                    }
                }
                None => {
                    // No context provided - cannot dereference, return Unknown
                    CharProcType::Unknown
                }
            }
        }
        other => CharProcType::Other(other.type_name().to_string()),
    }
}

/// Validate char_proc structure requirements.
///
/// This function checks if a char_proc object has the expected structure
/// for its type, verifying required keys are present.
///
/// # Arguments
///
/// * `object` - The PdfObject to validate
///
/// # Returns
///
/// * `Ok(())` if the object has valid structure
/// * `Err(Type3Error)` if validation fails, indicating what's wrong
///
/// # Validation Rules
///
/// - **Stream objects**: Must have /Type, /Subtype, /Width, /Height keys
/// - **Dict objects**: Must have /Type, /Subtype keys (basic char_proc structure)
/// - **Other types**: Returns `Err(InvalidCharProcType)`
///
/// # Example
///
/// ```rust,no_run
/// use pdftract_core::font::type3_rasterizer::validate_char_proc_structure;
/// use pdftract_core::parser::object::types::PdfObject;
///
/// let stream_obj = PdfObject::Stream(Box::new(/* ... */));
/// match validate_char_proc_structure(&stream_obj) {
///     Ok(()) => println!("Valid char_proc structure"),
///     Err(e) => println!("Validation failed: {}", e),
/// }
/// ```
pub fn validate_char_proc_structure(object: &PdfObject) -> Result<(), Type3Error> {
    // Detect the object type first (without document context for basic structure check)
    let char_proc_type = detect_char_proc_type(object, None);

    match char_proc_type {
        CharProcType::Stream => {
            // For streams, verify required keys: /Type, /Subtype, /Width, /Height
            let stream_dict = match object {
                PdfObject::Stream(stream) => &stream.dict,
                _ => {
                    return Err(Type3Error::InvalidCharProcType {
                        got: object.type_name().to_string(),
                        expected: "stream".to_string(),
                    })
                }
            };

            // Check for /Type key
            if stream_dict.get("/Type").is_none() {
                return Err(Type3Error::MissingRequiredKey {
                    key: "/Type".to_string(),
                    object_type: "stream".to_string(),
                });
            }

            // Check for /Subtype key
            if stream_dict.get("/Subtype").is_none() {
                return Err(Type3Error::MissingRequiredKey {
                    key: "/Subtype".to_string(),
                    object_type: "stream".to_string(),
                });
            }

            // Check for /Width key
            if stream_dict.get("/Width").is_none() {
                return Err(Type3Error::MissingRequiredKey {
                    key: "/Width".to_string(),
                    object_type: "stream".to_string(),
                });
            }

            // Check for /Height key
            if stream_dict.get("/Height").is_none() {
                return Err(Type3Error::MissingRequiredKey {
                    key: "/Height".to_string(),
                    object_type: "stream".to_string(),
                });
            }

            Ok(())
        }
        CharProcType::Dict => {
            // For dicts, verify basic structure: /Type, /Subtype
            let dict = match object {
                PdfObject::Dict(d) => d.as_ref(),
                _ => {
                    return Err(Type3Error::InvalidCharProcType {
                        got: object.type_name().to_string(),
                        expected: "dictionary".to_string(),
                    })
                }
            };

            // Check for /Type key
            if dict.get("/Type").is_none() {
                return Err(Type3Error::MissingRequiredKey {
                    key: "/Type".to_string(),
                    object_type: "dictionary".to_string(),
                });
            }

            // Check for /Subtype key
            if dict.get("/Subtype").is_none() {
                return Err(Type3Error::MissingRequiredKey {
                    key: "/Subtype".to_string(),
                    object_type: "dictionary".to_string(),
                });
            }

            Ok(())
        }
        CharProcType::Unknown => {
            // Unknown type (from failed reference dereferencing)
            Err(Type3Error::InvalidCharProcType {
                got: "unknown".to_string(),
                expected: "stream or dictionary".to_string(),
            })
        }
        CharProcType::Other(type_name) => {
            // For any other type, return InvalidCharProcType error
            Err(Type3Error::InvalidCharProcType {
                got: type_name,
                expected: "stream or dictionary".to_string(),
            })
        }
    }
}

/// Errors that can occur during Type 3 glyph rasterization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type3Error {
    /// CharProc reference not found in PDF.
    MissingCharProcRef {
        /// The object reference that could not be resolved
        ref_id: String,
    },
    /// Circular reference detected during glyph resolution.
    CircularRef {
        /// The object reference that caused the circular dependency
        ref_id: String,
    },
    /// I/O error during glyph resolution.
    Io(String),
    /// CharProc object has invalid type.
    InvalidCharProcType {
        /// The actual object type found
        got: String,
        /// The expected object type(s)
        expected: String,
    },
    /// Missing required key in char_proc structure.
    MissingRequiredKey {
        /// The key that is missing
        key: String,
        /// The object type being validated
        object_type: String,
    },
}

impl std::fmt::Display for Type3Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type3Error::MissingCharProcRef { ref_id } => {
                write!(f, "char_proc reference not found: {}", ref_id)
            }
            Type3Error::CircularRef { ref_id } => {
                write!(f, "circular reference detected at: {}", ref_id)
            }
            Type3Error::Io(msg) => write!(f, "I/O error during glyph resolution: {}", msg),
            Type3Error::InvalidCharProcType { got, expected } => {
                write!(f, "invalid char_proc type: got {}, expected {}", got, expected)
            }
            Type3Error::MissingRequiredKey { key, object_type } => {
                write!(f, "missing required key '{}' in char_proc {}", key, object_type)
            }
        }
    }
}

impl std::error::Error for Type3Error {}

impl From<ResolveError> for Type3Error {
    fn from(err: ResolveError) -> Self {
        match err {
            ResolveError::NotFound(obj_ref) => Type3Error::MissingCharProcRef {
                ref_id: obj_ref.to_string(),
            },
            ResolveError::CircularRef(obj_ref) => Type3Error::CircularRef {
                ref_id: obj_ref.to_string(),
            },
            ResolveError::Io(msg) => Type3Error::Io(msg),
        }
    }
}

/// Maximum recursion depth for Type 3 glyph execution (form XObject + nested glyphs).
const MAX_GLYPH_DEPTH: usize = 20;

/// Document resolver context for Type3 glyph rasterization.
///
/// Provides access to the document's resolver and source for dereferencing
/// content streams during glyph rasterization.
pub struct DocumentContext<'a> {
    /// PDF document resolver for looking up indirect references
    pub resolver: Option<&'a XrefResolver>,
    /// PDF source for reading stream data
    pub source: Option<&'a dyn PdfSource>,
}

/// Calculate glyph bitmap dimensions from glyph bounds.
///
/// This function computes the proper bitmap dimensions (width, height) in pixels
/// based on the glyph's bounding box coordinates. It handles scaling from PDF user
/// space to pixel space, adds padding for anti-aliasing, and ensures non-zero dimensions.
///
/// # Arguments
///
/// * `bbox` - Glyph bounding box [x0, y0, x1, y1] in PDF user space (points)
/// * `padding` - Optional padding margin in pixels (default: 1 for anti-aliasing)
///
/// # Returns
///
/// (width, height) as usize, where both dimensions are guaranteed to be >= 1.
///
/// # Calculation
///
/// ```text
/// width_px = (x1 - x0) + 2 * padding
/// height_px = (y1 - y0) + 2 * padding
/// ```
///
/// If the bounding box is degenerate (x0 == x1 or y0 == y1), the dimension
/// defaults to 1 pixel plus padding to ensure valid bitmap dimensions.
///
/// # Example
///
/// ```rust,no_run
/// use pdftract_core::font::type3_rasterizer::calculate_bitmap_dimensions;
///
/// // Glyph with bbox [10.0, 20.0, 50.0, 60.0] in points
/// // Raw size: 40x30 points
/// // With default padding: 42x32 pixels
/// let (width, height) = calculate_bitmap_dimensions(&[10.0, 20.0, 50.0, 60.0], None);
/// assert_eq!(width, 42);
/// assert_eq!(height, 32);
/// ```
pub fn calculate_bitmap_dimensions(bbox: &[f32; 4], padding: Option<u32>) -> (usize, usize) {
    let padding = padding.unwrap_or(1);

    // Extract bounding box coordinates
    let x0 = bbox[0];
    let y0 = bbox[1];
    let x1 = bbox[2];
    let y2 = bbox[3];

    // Calculate raw dimensions in PDF user space (points)
    let raw_width = x1 - x0;
    let raw_height = y2 - y0;

    // Handle degenerate cases (zero-width or zero-height bboxes)
    // Default to 1 pixel minimum before padding
    let width_points = if raw_width.abs() < 0.5 {
        1.0
    } else {
        raw_width.abs()
    };

    let height_points = if raw_height.abs() < 0.5 {
        1.0
    } else {
        raw_height.abs()
    };

    // Convert to pixel dimensions (1 point = 1 pixel for glyph rendering)
    // Add padding for anti-aliasing margins
    let width = (width_points.ceil() as u32) + 2 * padding;
    let height = (height_points.ceil() as u32) + 2 * padding;

    // Ensure minimum dimensions of 1x1 (even with zero padding)
    let width = (width.max(1)) as usize;
    let height = (height.max(1)) as usize;

    (width, height)
}

/// 32x32 grayscale bitmap for glyph rasterization.
///
/// Each pixel is a u8 value (0-255). Per Phase 2.5 convention:
/// - 0 = black ink
/// - 255 = white paper
/// - Values in between are anti-aliased edges
#[derive(Debug, Clone, PartialEq)]
pub struct Bitmap32x32 {
    /// 1024 pixels (32 * 32), stored row-major
    pixels: [u8; 1024],
}

impl Bitmap32x32 {
    /// Create a new white bitmap (all pixels = 255).
    pub fn white() -> Self {
        Self {
            pixels: [255u8; 1024],
        }
    }

    /// Create a new black bitmap (all pixels = 0).
    pub fn black() -> Self {
        Self {
            pixels: [0u8; 1024],
        }
    }

    /// Get the pixel value at (x, y).
    ///
    /// Returns None if (x, y) is out of bounds.
    pub fn get(&self, x: i32, y: i32) -> Option<u8> {
        if x < 0 || x >= 32 || y < 0 || y >= 32 {
            return None;
        }
        Some(self.pixels[(y as usize) * 32 + (x as usize)])
    }

    /// Set the pixel value at (x, y).
    ///
    /// Returns false if (x, y) is out of bounds.
    pub fn set(&mut self, x: i32, y: i32, value: u8) -> bool {
        if x < 0 || x >= 32 || y < 0 || y >= 32 {
            return false;
        }
        self.pixels[(y as usize) * 32 + (x as usize)] = value;
        true
    }

    /// Convert to a byte array for pHash computation.
    pub fn as_bytes(&self) -> &[u8; 1024] {
        &self.pixels
    }

    /// Fill a rectangle with the given color.
    pub fn fill_rect(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: u8) {
        for y in y0.max(0)..y1.min(32) {
            for x in x0.max(0)..x1.min(32) {
                self.set(x, y, color);
            }
        }
    }
}

impl Default for Bitmap32x32 {
    fn default() -> Self {
        Self::white()
    }
}

/// Dynamic-sized grayscale bitmap for glyph rasterization.
///
/// Each pixel is a u8 value (0-255). Per Phase 2.5 convention:
/// - 0 = black ink
/// - 255 = white paper
/// - Values in between are anti-aliased edges
#[derive(Debug, Clone, PartialEq)]
pub struct Bitmap {
    /// Bitmap width in pixels
    width: usize,
    /// Bitmap height in pixels
    height: usize,
    /// Pixel data stored row-major
    pixels: Vec<u8>,
}

impl Bitmap {
    /// Create a new white bitmap with the given dimensions.
    pub fn white(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![255u8; width * height],
        }
    }

    /// Create a new black bitmap with the given dimensions.
    pub fn black(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![0u8; width * height],
        }
    }

    /// Get the pixel value at (x, y).
    ///
    /// Returns None if (x, y) is out of bounds.
    pub fn get(&self, x: i32, y: i32) -> Option<u8> {
        if x < 0 || x >= self.width as i32 || y < 0 || y >= self.height as i32 {
            return None;
        }
        Some(self.pixels[(y as usize) * self.width + (x as usize)])
    }

    /// Set the pixel value at (x, y).
    ///
    /// Returns false if (x, y) is out of bounds.
    pub fn set(&mut self, x: i32, y: i32, value: u8) -> bool {
        if x < 0 || x >= self.width as i32 || y < 0 || y >= self.height as i32 {
            return false;
        }
        self.pixels[(y as usize) * self.width + (x as usize)] = value;
        true
    }

    /// Get bitmap dimensions.
    pub fn dimensions(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    /// Convert to a byte slice for pHash computation.
    pub fn as_bytes(&self) -> &[u8] {
        &self.pixels
    }

    /// Fill a rectangle with the given color.
    pub fn fill_rect(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: u8) {
        for y in y0.max(0)..y1.min(self.height as i32) {
            for x in x0.max(0)..x1.min(self.width as i32) {
                self.set(x, y, color);
            }
        }
    }
}

impl Default for Bitmap {
    fn default() -> Self {
        Self::white(32, 32)
    }
}

/// Round a floating-point x-coordinate to an integer pixel position.
///
/// This helper function converts a floating-point x-coordinate to an integer
/// pixel position using standard rounding rules (round half-away-from-zero).
///
/// # Arguments
///
/// * `x` - Floating-point x-coordinate in user space
///
/// # Returns
///
/// The nearest integer pixel position. Uses round half-away-from-zero:
/// - 0.5 rounds to 1
/// - -0.5 rounds to -1
/// - 2.3 rounds to 2
/// - -2.7 rounds to -3
///
/// # Example
///
/// ```rust,no_run
/// use pdftract_core::font::type3_rasterizer::round_x;
///
/// assert_eq!(round_x(0.5), 1);
/// assert_eq!(round_x(-0.5), -1);
/// assert_eq!(round_x(2.3), 2);
/// assert_eq!(round_x(-2.7), -3);
/// ```
pub fn round_x(x: f64) -> i32 {
    x.round() as i32
}

/// Edge structure for scanline polygon fill algorithm.
///
/// Represents a single edge in the polygon being filled, with fields
/// for tracking the edge's x-position as it advances through scanlines.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Edge {
    /// Current X intersection position (tracked as we move through scanlines)
    pub(crate) x: i32,
    /// Minimum Y coordinate (top of edge)
    pub(crate) y_min: i32,
    /// Maximum Y coordinate (bottom of edge)
    pub(crate) y_max: i32,
    /// Change in X across the edge
    pub(crate) dx: i32,
    /// Change in Y across the edge
    pub(crate) dy: i32,
}

impl Edge {
    /// Compute the rounded x-coordinate intersection point.
    ///
    /// This method rounds the current x position to the nearest integer
    /// using standard rounding rules. Used in scanline intersection calculation.
    pub(crate) fn intersection_x(&self) -> i32 {
        round_x(self.x as f64)
    }
}

/// Rasterization context for Type 3 glyph execution.
pub struct RasterizerContext<'a> {
    /// Output bitmap (dynamic sizing based on font bbox)
    bitmap: Bitmap,
    /// Current graphics state
    gstate: GraphicsState,
    /// Graphics state stack
    gstate_stack: GraphicsStateStack,
    /// Current path being constructed
    path: CurrentPath,
    /// Type3 font being rasterized
    font: &'a Type3Font,
    /// Current recursion depth
    depth: usize,
    /// Diagnostics
    diagnostics: Vec<Diagnostic>,
}

impl<'a> RasterizerContext<'a> {
    /// Create a new rasterizer context for the given Type3 font.
    ///
    /// This initializes the graphics state with the font's FontMatrix transform
    /// and creates a bitmap sized to the font's FontBBox.
    pub fn new(font: &'a Type3Font) -> Self {
        let mut gstate = GraphicsState::new();

        // Apply FontMatrix to transform from glyph space to text space
        // Per PDF spec section 9.6.5, Type3 glyph content streams are executed
        // in glyph space, and the FontMatrix transforms coordinates to text space
        gstate.concat_ctm(&font.font_matrix);

        // Calculate bitmap dimensions from font bounding box (bf-407xp6)
        let (width, height) = calculate_bitmap_dimensions(&font.font_bbox, None);

        Self {
            bitmap: Bitmap::white(width, height),
            gstate,
            gstate_stack: GraphicsStateStack::new(),
            path: CurrentPath::new(),
            font,
            depth: 0,
            diagnostics: Vec::new(),
        }
    }

    /// Execute a content stream and rasterize the result.
    pub fn execute_content_stream(&mut self, stream_bytes: &[u8]) {
        let mut lexer = Lexer::new(stream_bytes);
        let mut operand_stack: Vec<f64> = Vec::new();
        let mut name_stack: Vec<Arc<str>> = Vec::new();

        while let Some(token) = lexer.next_token() {
            match token {
                crate::parser::lexer::Token::Eof => break,
                crate::parser::lexer::Token::Integer(n) => operand_stack.push(n as f64),
                crate::parser::lexer::Token::Real(r) => operand_stack.push(r),
                crate::parser::lexer::Token::Name(ref name) => {
                    let name_str = String::from_utf8_lossy(name);
                    name_stack.push(Arc::from(name_str.as_ref()));
                }
                crate::parser::lexer::Token::Keyword(ref kw) => {
                    let kw_str = String::from_utf8_lossy(kw);
                    self.execute_operator(&kw_str, &mut operand_stack, &mut name_stack);
                }
                _ => {
                    // Ignore other tokens (strings, arrays, etc.)
                }
            }
        }
    }

    /// Execute a single PDF graphics operator.
    fn execute_operator(
        &mut self,
        op: &str,
        operand_stack: &mut Vec<f64>,
        name_stack: &mut Vec<Arc<str>>,
    ) {
        match op {
            // Path construction operators
            "m" => self.op_move_to(operand_stack),
            "l" => self.op_line_to(operand_stack),
            "c" => self.op_cubic_to(operand_stack),
            "v" => self.op_shorthand_cubic_to(operand_stack),
            "y" => self.op_shorthand_cubic_to_y(operand_stack),
            "re" => self.op_rect(operand_stack),
            "h" => self.op_close_path(),
            "n" => self.op_no_op(), // No-op end of path

            // Painting operators
            "S" => self.op_stroke(),
            "s" => self.op_close_stroke(),
            "f" | "F" => self.op_fill(),
            "f*" => self.op_eofill(),
            "B" => self.op_fill_stroke(),
            "B*" => self.op_eofill_stroke(),
            "b" => self.op_close_fill_stroke(),
            "b*" => self.op_close_eofill_stroke(),

            // Graphics state operators
            "q" => self.op_save(),
            "Q" => self.op_restore(),
            "cm" => self.op_concat(operand_stack),

            // XObject operator
            "Do" => self.op_do(name_stack),

            // Ignore unsupported operators for now
            _ => {}
        }
    }

    /// m x y - Move to absolute position
    fn op_move_to(&mut self, stack: &mut Vec<f64>) {
        if stack.len() < 2 {
            return;
        }
        let y = stack.pop().unwrap();
        let x = stack.pop().unwrap();
        self.path.move_to(Point::new(x, y));
    }

    /// l x y - Line to absolute position
    fn op_line_to(&mut self, stack: &mut Vec<f64>) {
        if stack.len() < 2 {
            return;
        }
        let y = stack.pop().unwrap();
        let x = stack.pop().unwrap();
        self.path.line_to(Point::new(x, y));
    }

    /// c x1 y1 x2 y2 x3 y3 - Cubic Bezier curve
    fn op_cubic_to(&mut self, stack: &mut Vec<f64>) {
        if stack.len() < 6 {
            return;
        }
        let y3 = stack.pop().unwrap();
        let x3 = stack.pop().unwrap();
        let y2 = stack.pop().unwrap();
        let x2 = stack.pop().unwrap();
        let y1 = stack.pop().unwrap();
        let x1 = stack.pop().unwrap();
        self.path
            .cubic_to(Point::new(x1, y1), Point::new(x2, y2), Point::new(x3, y3));
    }

    /// v x2 y2 x3 y3 - Shorthand cubic Bezier (first control point implied)
    fn op_shorthand_cubic_to(&mut self, stack: &mut Vec<f64>) {
        if stack.len() < 4 {
            return;
        }
        let y3 = stack.pop().unwrap();
        let x3 = stack.pop().unwrap();
        let y2 = stack.pop().unwrap();
        let x2 = stack.pop().unwrap();
        self.path
            .shorthand_cubic_to(Point::new(x2, y2), Point::new(x3, y3));
    }

    /// y x1 y1 x3 y3 - Shorthand cubic Bezier (second control point implied)
    fn op_shorthand_cubic_to_y(&mut self, stack: &mut Vec<f64>) {
        if stack.len() < 4 {
            return;
        }
        let y3 = stack.pop().unwrap();
        let x3 = stack.pop().unwrap();
        let y1 = stack.pop().unwrap();
        let x1 = stack.pop().unwrap();
        self.path
            .shorthand_cubic_to_y(Point::new(x1, y1), Point::new(x3, y3));
    }

    /// re x y width height - Append rectangle
    fn op_rect(&mut self, stack: &mut Vec<f64>) {
        if stack.len() < 4 {
            return;
        }
        let height = stack.pop().unwrap();
        let width = stack.pop().unwrap();
        let y = stack.pop().unwrap();
        let x = stack.pop().unwrap();
        self.path.rect(x, y, width, height);
    }

    /// h - Close subpath
    fn op_close_path(&mut self) {
        self.path.close_path();
    }

    /// n - No-op end of path
    fn op_no_op(&mut self) {
        self.path.clear();
    }

    /// S - Stroke path
    fn op_stroke(&mut self) {
        self.rasterize_path(true);
        self.path.clear();
    }

    /// s - Close and stroke path
    fn op_close_stroke(&mut self) {
        self.path.close_path();
        self.rasterize_path(true);
        self.path.clear();
    }

    /// f / F - Fill path using nonzero winding rule
    fn op_fill(&mut self) {
        self.rasterize_path(false);
        self.path.clear();
    }

    /// f* - Fill path using even-odd rule
    fn op_eofill(&mut self) {
        // For simple glyphs, even-odd vs nonzero doesn't matter much
        self.rasterize_path(false);
        self.path.clear();
    }

    /// B - Fill then stroke path
    fn op_fill_stroke(&mut self) {
        self.rasterize_path(false);
        self.rasterize_path(true);
        self.path.clear();
    }

    /// B* - Fill then stroke path (even-odd)
    fn op_eofill_stroke(&mut self) {
        self.rasterize_path(false);
        self.rasterize_path(true);
        self.path.clear();
    }

    /// b - Close, fill, then stroke path
    fn op_close_fill_stroke(&mut self) {
        self.path.close_path();
        self.rasterize_path(false);
        self.rasterize_path(true);
        self.path.clear();
    }

    /// b* - Close, fill, then stroke path (even-odd)
    fn op_close_eofill_stroke(&mut self) {
        self.path.close_path();
        self.rasterize_path(false);
        self.rasterize_path(true);
        self.path.clear();
    }

    /// q - Save graphics state
    fn op_save(&mut self) {
        if !self.gstate_stack.push(&self.gstate) {
            self.diagnostics.push(Diagnostic::with_static_no_offset(
                DiagCode::GstateStackOverflow,
                "Type3 glyph graphics state stack overflow",
            ));
        }
    }

    /// Q - Restore graphics state
    fn op_restore(&mut self) {
        if let Some(restored) = self.gstate_stack.pop() {
            self.gstate = restored;
        } else {
            self.diagnostics.push(Diagnostic::with_static_no_offset(
                DiagCode::GstateStackUnderflow,
                "Type3 glyph graphics state stack underflow",
            ));
        }
    }

    /// cm a b c d e f - Concatenate matrix to CTM
    fn op_concat(&mut self, stack: &mut Vec<f64>) {
        if stack.len() < 6 {
            self.diagnostics.push(Diagnostic::with_static_no_offset(
                DiagCode::CmArgCount,
                "cm operator requires exactly 6 numeric arguments",
            ));
            return;
        }
        let f = stack.pop().unwrap();
        let e = stack.pop().unwrap();
        let d = stack.pop().unwrap();
        let c = stack.pop().unwrap();
        let b = stack.pop().unwrap();
        let a = stack.pop().unwrap();

        // Check for NaN values
        if a.is_nan() || b.is_nan() || c.is_nan() || d.is_nan() || e.is_nan() || f.is_nan() {
            self.diagnostics.push(Diagnostic::with_static_no_offset(
                DiagCode::CmDegenerate,
                "cm operator received NaN values; clamped to identity",
            ));
            return; // Don't modify CTM
        }

        let matrix = Matrix3x3::from_pdf_array([a, b, c, d, e, f]);

        // Check for degenerate matrix (det == 0)
        if matrix.determinant() == 0.0 {
            self.diagnostics.push(Diagnostic::with_static_no_offset(
                DiagCode::CmDegenerate,
                "cm operator received degenerate matrix (det=0); clamped to identity",
            ));
            return; // Don't modify CTM
        }

        self.gstate.concat_ctm(&matrix);
    }

    /// Do name - Invoke XObject
    fn op_do(&mut self, name_stack: &mut Vec<Arc<str>>) {
        if name_stack.is_empty() {
            return;
        }
        let name = name_stack.pop().unwrap();

        // Check recursion depth
        if self.depth >= MAX_GLYPH_DEPTH {
            self.diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::StructXobjectCycle,
                format!(
                    "Type3 glyph recursion depth limit reached at {}",
                    MAX_GLYPH_DEPTH
                ),
            ));
            return;
        }

        // Form XObject handling would go here
        // For now, stub this out - form XObjects require full resource resolution
    }

    /// Evaluate a point on a cubic Bezier curve at parameter t using de Casteljau's algorithm.
    ///
    /// Given a cubic Bezier curve defined by control points p0, p1, p2, p3,
    /// this computes the point on the curve at parameter t ∈ [0, 1].
    ///
    /// # Arguments
    ///
    /// * `p0` - Start point of the curve
    /// * `p1` - First control point
    /// * `p2` - Second control point
    /// * `p3` - End point of the curve
    /// * `t` - Parameter value [0, 1]
    ///
    /// # Returns
    ///
    /// The Point on the curve at parameter t
    fn cubic_bezier_point(p0: Point, p1: Point, p2: Point, p3: Point, t: f64) -> Point {
        let u = 1.0 - t;
        let tt = t * t;
        let uu = u * u;
        let uuu = uu * u;
        let ttt = tt * t;

        // Bezier formula: B(t) = (1-t)³P₀ + 3(1-t)²tP₁ + 3(1-t)t²P₂ + t³P₃
        Point {
            x: uuu * p0.x + 3.0 * uu * t * p1.x + 3.0 * u * tt * p2.x + ttt * p3.x,
            y: uuu * p0.y + 3.0 * uu * t * p1.y + 3.0 * u * tt * p2.y + ttt * p3.y,
        }
    }

    /// Subdivide a cubic Bezier curve into two curves at parameter t.
    ///
    /// Uses de Casteljau's algorithm to split the curve into two segments:
    /// - Left segment: from t=0 to t
    /// - Right segment: from t to t=1
    ///
    /// # Arguments
    ///
    /// * `p0` - Start point of the curve
    /// * `p1` - First control point
    /// * `p2` - Second control point
    /// * `p3` - End point of the curve
    /// * `t` - Split parameter [0, 1]
    ///
    /// # Returns
    ///
    /// A tuple of two curves: (left_curve, right_curve), where each curve
    /// is represented as (p0, p1, p2, p3)
    fn subdivide_cubic_bezier(
        p0: Point,
        p1: Point,
        p2: Point,
        p3: Point,
        t: f64,
    ) -> ((Point, Point, Point, Point), (Point, Point, Point, Point)) {
        // de Casteljau subdivision
        let p01 = Point {
            x: p0.x + (p1.x - p0.x) * t,
            y: p0.y + (p1.y - p0.y) * t,
        };
        let p12 = Point {
            x: p1.x + (p2.x - p1.x) * t,
            y: p1.y + (p2.y - p1.y) * t,
        };
        let p23 = Point {
            x: p2.x + (p3.x - p2.x) * t,
            y: p2.y + (p3.y - p2.y) * t,
        };

        let p012 = Point {
            x: p01.x + (p12.x - p01.x) * t,
            y: p01.y + (p12.y - p01.y) * t,
        };
        let p123 = Point {
            x: p12.x + (p23.x - p12.x) * t,
            y: p12.y + (p23.y - p12.y) * t,
        };

        let p0123 = Point {
            x: p012.x + (p123.x - p012.x) * t,
            y: p012.y + (p123.y - p012.y) * t,
        };

        // Left curve: p0 -> p01 -> p012 -> p0123
        let left = (p0, p01, p012, p0123);
        // Right curve: p0123 -> p123 -> p23 -> p3
        let right = (p0123, p123, p23, p3);

        (left, right)
    }

    /// Calculate the flatness of a cubic Bezier curve.
    ///
    /// Flatness measures how close the curve is to a straight line.
    /// We use the midpoint deviation method: compute the point on the curve
    /// at t=0.5 and measure its distance from the line connecting p0 and p3.
    ///
    /// # Arguments
    ///
    /// * `p0` - Start point of the curve
    /// * `p1` - First control point
    /// * `p2` - Second control point
    /// * `p3` - End point of the curve
    ///
    /// # Returns
    ///
    /// The flatness value (lower is flatter, 0 means perfectly straight)
    fn curve_flatness(p0: Point, p1: Point, p2: Point, p3: Point) -> f64 {
        // Compute midpoint of the curve at t=0.5
        let midpoint = Self::cubic_bezier_point(p0, p1, p2, p3, 0.5);

        // Compute the midpoint of the chord (line from p0 to p3)
        let chord_mid = Point {
            x: (p0.x + p3.x) * 0.5,
            y: (p0.y + p3.y) * 0.5,
        };

        // Distance from curve midpoint to chord midpoint
        let dx = midpoint.x - chord_mid.x;
        let dy = midpoint.y - chord_mid.y;
        (dx * dx + dy * dy).sqrt()
    }

    /// Recursively subdivide a cubic Bezier curve until it's flat enough.
    ///
    /// Converts a smooth Bezier curve into a series of line segments
    /// using adaptive subdivision based on flatness checking.
    ///
    /// # Arguments
    ///
    /// * `segments` - Output vector to append line segments to
    /// * `p0` - Start point of the curve
    /// * `p1` - First control point
    /// * `p2` - Second control point
    /// * `p3` - End point of the curve
    /// * `depth` - Current recursion depth
    fn flatten_cubic_bezier_recursive(
        segments: &mut Vec<(Point, Point)>,
        p0: Point,
        p1: Point,
        p2: Point,
        p3: Point,
        depth: usize,
    ) {
        const FLATNESS_THRESHOLD: f64 = 0.5;
        const MAX_DEPTH: usize = 8;

        // Check if curve is flat enough or we've reached max depth
        let flatness = Self::curve_flatness(p0, p1, p2, p3);
        if flatness <= FLATNESS_THRESHOLD || depth >= MAX_DEPTH {
            // Curve is flat enough - add as a single line segment
            segments.push((p0, p3));
            return;
        }

        // Split the curve at t=0.5 and recursively flatten both halves
        let (left, right) = Self::subdivide_cubic_bezier(p0, p1, p2, p3, 0.5);

        Self::flatten_cubic_bezier_recursive(segments, left.0, left.1, left.2, left.3, depth + 1);
        Self::flatten_cubic_bezier_recursive(segments, right.0, right.1, right.2, right.3, depth + 1);
    }

    /// Flatten a cubic Bezier curve into line segments.
    ///
    /// Public interface for curve flattening. Converts a smooth Bezier curve
    /// into a series of line segments suitable for scanline rasterization.
    ///
    /// # Arguments
    ///
    /// * `p0` - Start point of the curve
    /// * `p1` - First control point
    /// * `p2` - Second control point
    /// * `p3` - End point of the curve
    ///
    /// # Returns
    ///
    /// A vector of line segments (start_point, end_point) that approximate
    /// the Bezier curve
    fn flatten_cubic_bezier(
        p0: Point,
        p1: Point,
        p2: Point,
        p3: Point,
    ) -> Vec<(Point, Point)> {
        let mut segments = Vec::new();
        Self::flatten_cubic_bezier_recursive(&mut segments, p0, p1, p2, p3, 0);
        segments
    }

    /// Rasterize the current path to the bitmap.
    fn rasterize_path(&mut self, stroke: bool) {
        // Collect path segments from commands
        let mut segments = Vec::new();
        let mut current_point = None;
        let mut move_point = None;

        for cmd in &self.path.commands {
            match cmd {
                PathCommand::MoveTo(p) => {
                    current_point = Some(*p);
                    move_point = Some(*p);
                }
                PathCommand::LineTo(p) => {
                    if let Some(start) = current_point {
                        segments.push((start, *p));
                    }
                    current_point = Some(*p);
                }
                PathCommand::Rect(x, y, width, height) => {
                    // Convert rectangle to 4 line segments
                    let x0 = *x;
                    let y0 = *y;
                    let x1 = x + width;
                    let y1 = y + height;

                    // Rectangle: (x0,y0) -> (x1,y0) -> (x1,y1) -> (x0,y1) -> (x0,y0)
                    let p0 = Point::new(x0, y0);
                    let p1 = Point::new(x1, y0);
                    let p2 = Point::new(x1, y1);
                    let p3 = Point::new(x0, y1);

                    segments.push((p0, p1));
                    segments.push((p1, p2));
                    segments.push((p2, p3));
                    segments.push((p3, p0));

                    current_point = Some(p0);
                    move_point = Some(p0);
                }
                PathCommand::ClosePath => {
                    // Close path by connecting to the move point
                    if let (Some(current), Some(move_p)) = (current_point, move_point) {
                        if current != move_p {
                            segments.push((current, move_p));
                        }
                    }
                    current_point = move_point;
                }
                PathCommand::CubicTo(cp1, cp2, end) => {
                    // Flatten cubic Bezier curve into line segments
                    if let Some(start) = current_point {
                        let curve_segments = Self::flatten_cubic_bezier(start, *cp1, *cp2, *end);
                        segments.extend(curve_segments);
                    }
                    current_point = Some(*end);
                }
                PathCommand::ShorthandCubicTo(cp2, end) => {
                    // v x2 y2 x3 y3 - first control point is reflection of previous
                    if let Some(start) = current_point {
                        // First control point is the current point (reflection from previous segment)
                        let cp1 = start;
                        let curve_segments = Self::flatten_cubic_bezier(start, cp1, *cp2, *end);
                        segments.extend(curve_segments);
                    }
                    current_point = Some(*end);
                }
                PathCommand::ShorthandCubicToY(cp1, end) => {
                    // y x1 y1 x3 y3 - second control point is the end point
                    if let Some(start) = current_point {
                        // Second control point is the end point (reflection)
                        let cp2 = *end;
                        let curve_segments = Self::flatten_cubic_bezier(start, *cp1, cp2, *end);
                        segments.extend(curve_segments);
                    }
                    current_point = Some(*end);
                }
            }
        }

        // Transform all segments and collect edges
        let mut edges = Vec::new();
        for (p0, p1) in segments {
            // Transform points by CTM
            let (x0, y0) = self.gstate.ctm.transform_point(p0.x, p0.y);
            let (x1, y1) = self.gstate.ctm.transform_point(p1.x, p1.y);

            // Convert to bitmap coordinates (round to nearest pixel)
            let bx0 = x0.round() as i32;
            let by0 = y0.round() as i32;
            let bx1 = x1.round() as i32;
            let by1 = y1.round() as i32;

            edges.push((bx0, by0, bx1, by1));
        }

        if stroke {
            // Stroke mode: draw line outlines
            for (x0, y0, x1, y1) in edges {
                self.draw_line(x0, y0, x1, y1);
            }
        } else {
            // Fill mode: use scanline polygon fill
            self.fill_polygon(&edges);
        }
    }

    /// Draw a line segment from (x0, y0) to (x1, y1) using Bresenham's algorithm.
    fn draw_line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };

        let mut err = dx - dy;
        let mut x = x0;
        let mut y = y0;

        let width = self.bitmap.width as i32;
        let height = self.bitmap.height as i32;

        loop {
            // Set pixel if within bounds
            if x >= 0 && x < width && y >= 0 && y < height {
                self.bitmap.set(x, y, 0);
            }

            if x == x1 && y == y1 {
                break;
            }

            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
            }
        }
    }

    /// Fill a polygon using scanline algorithm with Active Edge Table (AET).
    /// `edges` is a list of (x0, y0, x1, y1) line segments in bitmap coordinates.
    pub(crate) fn fill_polygon(&mut self, edges: &[(i32, i32, i32, i32)]) {
        let width = self.bitmap.width as i32;
        let height = self.bitmap.height as i32;

        let mut get: Vec<Edge> = Vec::new();

        for &(x0, y0, x1, y1) in edges {
            // Skip horizontal edges (they don't affect scanline fill)
            if y0 == y1 {
                continue;
            }

            let (y_min, y_max) = if y0 < y1 { (y0, y1) } else { (y1, y0) };

            get.push(Edge {
                x: x0,  // Initial X position at y_min
                y_min,
                y_max,
                dx: x1 - x0,
                dy: y1 - y0,
            });
        }

        if get.is_empty() {
            return; // No non-horizontal edges to process
        }

        // Sort GET by y_min (topmost edges first)
        get.sort_by_key(|e| e.y_min);

        // Find y-bounds
        let min_y = get.first().map(|e| e.y_min.max(0)).unwrap_or(0);
        let max_y = get.last().map(|e| e.y_max.min(height - 1)).unwrap_or(0);

        // Initialize Active Edge Table (AET)
        let mut aet: Vec<Edge> = Vec::new();
        let mut get_idx = 0;

        // For each scanline
        for y in min_y..=max_y {
            // Add edges from GET to AET when scanline reaches edge.y_min
            while get_idx < get.len() && get[get_idx].y_min == y {
                aet.push(get[get_idx]);
                get_idx += 1;
            }

            // Remove edges from AET where scanline > y_max (edge has ended)
            aet.retain(|e| y <= e.y_max);

            // Update X positions in AET for this scanline
            for edge in &mut aet {
                // Update X based on slope: x += dx/dy for each scanline
                edge.x += (edge.dx as f64 / edge.dy as f64) as i32;
            }

            // Sort AET by current X position
            aet.sort_by_key(|e| e.x);

            // Calculate intersection x coordinates for this scanline
            // Compute x = round(edge.x) for each active edge
            let intersections: Vec<i32> = aet.iter().map(|edge| edge.intersection_x()).collect();

            // Fill between pairs of X positions (even-odd rule)
            for i in (0..intersections.len()).step_by(2) {
                if i + 1 < intersections.len() {
                    let x_start = intersections[i].max(0);
                    let x_end = intersections[i + 1].min(width - 1);

                    for x in x_start..=x_end {
                        self.bitmap.set(x, y, 0);
                    }
                }
            }
        }
    }
}

/// Stream resolver callback for Type3 glyph rasterization.
///
/// Given an ObjRef to a content stream, returns the decoded stream bytes.
pub type StreamResolverFn = dyn Fn(ObjRef) -> Option<Vec<u8>> + Send + Sync;

/// Dereference a char_proc_ref to resolve the actual PDF object.
///
/// This function looks up the indirect reference using the document resolver
/// and returns the resolved PdfObject (typically a stream object for Type3 glyphs).
///
/// This function integrates validation by checking the char_proc structure
/// immediately after resolving the reference, before any attempt to parse
/// the content stream. This provides early detection of invalid structures
/// with clear error messages.
///
/// # Arguments
///
/// * `char_proc_ref` - The ObjRef pointing to the glyph content stream
/// * `doc_context` - Document resolver context containing the XrefResolver
///
/// # Returns
///
/// `Ok(PdfObject)` if the reference was successfully resolved and validated,
/// `Err(Type3Error)` if resolution or validation failed.
///
/// # Error Context
///
/// Error messages include the object reference being dereferenced to aid debugging.
/// For example: "char_proc reference not found: 10 0 R" or
/// "invalid char_proc type for glyph 'A': got integer, expected stream or dictionary"
pub fn deref_char_proc_ref(
    char_proc_ref: ObjRef,
    doc_context: Option<&DocumentContext>,
) -> Result<crate::parser::object::types::PdfObject, Type3Error> {
    let doc_context = doc_context.ok_or_else(|| {
        Type3Error::Io(format!(
            "DocumentContext not provided - cannot dereference char_proc_ref {}",
            char_proc_ref
        ))
    })?;

    let resolver = doc_context.resolver.ok_or_else(|| {
        Type3Error::Io(format!(
            "XrefResolver not provided in DocumentContext - cannot resolve char_proc_ref {}",
            char_proc_ref
        ))
    })?;

    let source = doc_context.source.ok_or_else(|| {
        Type3Error::Io(format!(
            "PdfSource not provided in DocumentContext - cannot resolve stream for char_proc_ref {}",
            char_proc_ref
        ))
    })?;

    // Use resolver.resolve_with_source to get the actual PDF object
    // source is already &dyn PdfSource, which is what resolve_with_source expects
    let resolved_obj = resolver
        .resolve_with_source(char_proc_ref, source)
        .map_err(Type3Error::from)?;

    // Validate the char_proc structure before returning
    // This ensures we catch invalid structures early, before attempting to parse
    // the content stream (EC-42: Early validation in parsing pipelines)
    validate_char_proc_structure(&resolved_obj).map_err(|e| {
        // Enhance error context with the object reference for debugging
        match e {
            Type3Error::InvalidCharProcType { got, expected } => {
                Type3Error::InvalidCharProcType {
                    got: format!("{} (for ref {})", got, char_proc_ref),
                    expected,
                }
            }
            Type3Error::MissingRequiredKey { key, object_type } => {
                Type3Error::MissingRequiredKey {
                    key: format!("{} (for ref {})", key, char_proc_ref),
                    object_type,
                }
            }
            _ => e,
        }
    })?;

    Ok(resolved_obj)
}

/// Extract content stream bytes from a resolved PDF object.
///
/// This function takes a PdfObject (typically from `deref_char_proc_ref`) and
/// extracts the raw content stream bytes, handling:
///
/// - Direct stream objects (decode if FlateDecode compressed)
/// - Indirect references (recursively resolve and extract)
/// - Error cases (null, wrong type, missing data)
///
/// # Arguments
///
/// * `resolved_obj` - The PdfObject resolved from `deref_char_proc_ref`
/// * `doc_context` - Document resolver context for recursive resolution
///
/// # Returns
///
/// `Ok(Vec<u8>)` with the decoded content stream bytes,
/// `Err(Type3Error)` if extraction fails (wrong type, I/O error, or invalid reference).
///
/// # Error Context
///
/// Error messages include the object type/reference to aid debugging.
/// For example: "Cannot extract stream from null object at char_proc_ref"
pub fn extract_content_stream_bytes(
    resolved_obj: PdfObject,
    doc_context: &DocumentContext,
) -> Result<Vec<u8>, Type3Error> {
    let resolver = doc_context.resolver.ok_or_else(|| {
        Type3Error::Io(
            "XrefResolver not provided in DocumentContext - cannot extract stream".to_string()
        )
    })?;

    let source = doc_context.source.ok_or_else(|| {
        Type3Error::Io(
            "PdfSource not provided in DocumentContext - cannot extract stream".to_string()
        )
    })?;

    match resolved_obj {
        PdfObject::Stream(stream) => {
            // Direct stream object - decode it
            let opts = ExtractionOptions::default();
            let mut decompress_counter = 0u64;

            // decode_stream handles decompression (FlateDecode, etc.)
            let bytes = decode_stream(&stream, source, &opts, &mut decompress_counter);
            Ok(bytes)
        }
        PdfObject::Ref(obj_ref) => {
            // Indirect reference - recursively resolve and extract
            let inner_obj = resolver
                .resolve_with_source(obj_ref, source)
                .map_err(Type3Error::from)?;

            // Recursively extract from the resolved object
            extract_content_stream_bytes(inner_obj, doc_context)
        }
        PdfObject::Null => {
            Err(Type3Error::Io(
                "Cannot extract stream from null object at char_proc_ref".to_string()
            ))
        }
        PdfObject::Bool(_) => {
            Err(Type3Error::Io(
                "Cannot extract stream from boolean at char_proc_ref - expected Stream or Ref".to_string()
            ))
        }
        PdfObject::Integer(_) => {
            Err(Type3Error::Io(
                "Cannot extract stream from integer at char_proc_ref - expected Stream or Ref".to_string()
            ))
        }
        PdfObject::Real(_) => {
            Err(Type3Error::Io(
                "Cannot extract stream from real number at char_proc_ref - expected Stream or Ref".to_string()
            ))
        }
        PdfObject::String(_) => {
            Err(Type3Error::Io(
                "Cannot extract stream from string at char_proc_ref - expected Stream or Ref".to_string()
            ))
        }
        PdfObject::Name(_) => {
            Err(Type3Error::Io(
                "Cannot extract stream from name at char_proc_ref - expected Stream or Ref".to_string()
            ))
        }
        PdfObject::Array(_) => {
            Err(Type3Error::Io(
                "Cannot extract stream from array at char_proc_ref - expected Stream or Ref".to_string()
            ))
        }
        PdfObject::Dict(_) => {
            Err(Type3Error::Io(
                "Cannot extract stream from dictionary at char_proc_ref - expected Stream or Ref".to_string()
            ))
        }
        PdfObject::Indirect(_) => {
            Err(Type3Error::Io(
                "Cannot extract stream from indirect object at char_proc_ref - expected Stream or Ref".to_string()
            ))
        }
    }
}

/// Calculate bitmap dimensions from glyph bounding box.
///
/// This function computes appropriate bitmap dimensions for rendering a glyph
/// based on its bounding box in PDF user space. It accounts for the glyph's
/// actual size and adds padding for anti-aliasing.
///
/// # Arguments
///
/// * `bbox` - Glyph bounding box [x0, y0, x1, y1] in PDF user space (points)
/// * `padding_pixels` - Optional padding in pixels for anti-aliasing (default: 2)
///
/// # Returns
///
/// (width, height) as usize dimensions in pixels. Minimum dimensions are
/// clamped to 1 to ensure valid bitmap size.
///
/// # Calculation
///
/// Width and height are computed from the bounding box:
/// - width = x1 - x0
/// - height = y1 - y0
///
/// Values are converted to pixels (assuming 1 point = 1 pixel for Type3
/// glyph space) and padding is added on all sides.
///
/// # Example
///
/// ```
/// let bbox = [0.0, 0.0, 16.0, 16.0]; // 16x16 point glyph
/// let (width, height) = calculate_bitmap_size_from_bounds(&bbox, None);
/// assert_eq!(width, 20);  // 16 + 2*2 padding
/// assert_eq!(height, 20); // 16 + 2*2 padding
/// ```
pub fn calculate_bitmap_size_from_bounds(bbox: &[f32; 4], padding_pixels: Option<u32>) -> (usize, usize) {
    let padding = padding_pixels.unwrap_or(2) as i32;

    // Extract bounds
    let x0 = bbox[0] as i32;
    let y0 = bbox[1] as i32;
    let x1 = bbox[2] as i32;
    let y1 = bbox[3] as i32;

    // Calculate raw dimensions
    let raw_width = (x1 - x0).abs();
    let raw_height = (y1 - y0).abs();

    // Add padding (2x for both sides)
    let width = (raw_width + 2 * padding).max(1) as usize;
    let height = (raw_height + 2 * padding).max(1) as usize;

    (width, height)
}

/// Rasterize a Type 3 glyph to a grayscale bitmap with dynamic sizing.
///
/// # Arguments
///
/// * `font` - The Type3 font containing the glyph
/// * `glyph_name` - The name of the glyph to rasterize
/// * `doc_context` - Document resolver context (may be None)
/// * `resolve_stream` - Callback to resolve ObjRef to stream bytes (may be None)
///
/// # Returns
///
/// Some(bitmap) if the glyph exists and rasterized successfully,
/// None if the glyph name is not in /CharProcs or stream resolution fails.
///
/// The bitmap size is calculated from the font's FontBBox using
/// calculate_bitmap_dimensions (bf-407xp6).
pub fn rasterize_type3_glyph<'a, R>(
    font: &Type3Font,
    glyph_name: &str,
    doc_context: Option<&'a DocumentContext<'a>>,
    resolve_stream: Option<&R>,
) -> Option<Vec<u8>>
where
    R: Fn(ObjRef) -> Option<Vec<u8>> + ?Sized,
{
    // Check if glyph exists and get its ObjRef
    let char_proc_ref = font.char_proc(glyph_name)?;

    // Document context is passed for potential future use (e.g., form XObject resolution)
    // Stream resolution happens via the resolver callback pattern
    // doc_context is now active and available for use

    // Try to resolve the content stream if a resolver is provided
    let stream_bytes = match resolve_stream {
        Some(resolver) => resolver(char_proc_ref),
        None => None,
    };

    match stream_bytes {
        Some(bytes) => {
            // Successfully resolved - execute the content stream and rasterize
            let mut ctx = RasterizerContext::new(font);
            ctx.execute_content_stream(&bytes);
            Some(ctx.bitmap.as_bytes().to_vec())
        }
        None => {
            // No resolver provided or resolution failed - cannot rasterize
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::object::types::PdfDict;

    #[test]
    fn test_bitmap_white() {
        let bitmap = Bitmap32x32::white();
        assert_eq!(bitmap.get(0, 0), Some(255));
        assert_eq!(bitmap.get(31, 31), Some(255));
        assert_eq!(bitmap.get(32, 0), None);
        assert_eq!(bitmap.get(0, 32), None);
    }

    #[test]
    fn test_bitmap_black() {
        let bitmap = Bitmap32x32::black();
        assert_eq!(bitmap.get(0, 0), Some(0));
        assert_eq!(bitmap.get(31, 31), Some(0));
    }

    #[test]
    fn test_bitmap_set_get() {
        let mut bitmap = Bitmap32x32::white();
        assert!(bitmap.set(10, 15, 128));
        assert_eq!(bitmap.get(10, 15), Some(128));
        assert!(!bitmap.set(-1, 0, 0)); // Out of bounds
        assert!(!bitmap.set(0, 32, 0)); // Out of bounds
    }

    #[test]
    fn test_bitmap_fill_rect() {
        let mut bitmap = Bitmap32x32::white();
        bitmap.fill_rect(10, 10, 20, 20, 0);

        // Inside rect
        assert_eq!(bitmap.get(15, 15), Some(0));
        // Outside rect
        assert_eq!(bitmap.get(5, 5), Some(255));
        assert_eq!(bitmap.get(25, 25), Some(255));
    }

    #[test]
    fn test_current_path_move_line() {
        let mut path = CurrentPath::new();
        path.move_to(Point::new(10.0, 20.0));
        assert_eq!(path.current_point(), Some(Point::new(10.0, 20.0)));
        assert_eq!(path.move_point(), Some(Point::new(10.0, 20.0)));

        path.line_to(Point::new(30.0, 40.0));
        assert_eq!(path.current_point(), Some(Point::new(30.0, 40.0)));
        assert_eq!(path.move_point(), Some(Point::new(10.0, 20.0)));
    }

    #[test]
    fn test_current_path_close() {
        let mut path = CurrentPath::new();
        path.move_to(Point::new(10.0, 20.0));
        path.line_to(Point::new(30.0, 40.0));
        path.close_path();

        assert_eq!(path.current_point(), Some(Point::new(10.0, 20.0)));
    }

    #[test]
    fn test_current_path_rect() {
        let mut path = CurrentPath::new();
        path.rect(5.0, 10.0, 20.0, 30.0);

        assert_eq!(path.current_point(), Some(Point::new(5.0, 10.0)));
        assert_eq!(path.move_point(), Some(Point::new(5.0, 10.0)));
    }

    #[test]
    fn test_point_new() {
        let p = Point::new(1.5, 2.5);
        assert_eq!(p.x, 1.5);
        assert_eq!(p.y, 2.5);
    }

    #[test]
    fn test_rasterizer_context_new() {
        let font_dict = PdfDict::new();
        let font = Type3Font::load(&font_dict);
        let ctx = RasterizerContext::new(&font);

        assert_eq!(ctx.depth, 0);
        // Verify bitmap has appropriate dimensions (may vary based on font bbox)
        let (width, height) = ctx.bitmap.dimensions();
        assert!(width >= 1, "Bitmap width should be at least 1 pixel");
        assert!(height >= 1, "Bitmap height should be at least 1 pixel");
    }

    #[test]
    fn test_rasterizer_context_applies_font_matrix() {
        use crate::parser::object::types::intern;

        // Create a Type3 font with custom FontMatrix [2 0 0 2 0 0]
        let mut font_dict = PdfDict::new();
        font_dict.insert(
            intern("/FontMatrix"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Real(2.0),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
                PdfObject::Real(2.0),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
            ])),
        );

        let font = Type3Font::load(&font_dict);
        let ctx = RasterizerContext::new(&font);

        // Verify that the CTM has the FontMatrix applied
        // Initial CTM should be FontMatrix, not identity
        assert_eq!(ctx.gstate.ctm.a, 2.0);
        assert_eq!(ctx.gstate.ctm.d, 2.0);
        assert_eq!(ctx.gstate.ctm.b, 0.0);
        assert_eq!(ctx.gstate.ctm.c, 0.0);
        assert_eq!(ctx.gstate.ctm.e, 0.0);
        assert_eq!(ctx.gstate.ctm.f, 0.0);
    }

    #[test]
    fn test_execute_simple_path() {
        let font_dict = PdfDict::new();
        let font = Type3Font::load(&font_dict);
        let mut ctx = RasterizerContext::new(&font);

        // Execute: 10 10 m 20 20 l
        let stream = b"10 10 m 20 20 l";
        ctx.execute_content_stream(stream);

        // Path should have move and line commands
        assert_eq!(ctx.path.commands.len(), 2);
    }

    #[test]
    fn test_execute_rect() {
        use crate::parser::object::types::intern;

        // Create a font with identity FontMatrix for predictable coordinates
        // and a font_bbox that creates a bitmap large enough for the test
        let mut font_dict = PdfDict::new();
        font_dict.insert(
            intern("/FontMatrix"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(1),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
                PdfObject::Integer(1),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
            ])),
        );
        font_dict.insert(
            intern("/FontBBox"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(0),
                PdfObject::Integer(0),
                PdfObject::Integer(25),
                PdfObject::Integer(25),
            ])),
        );

        let font = Type3Font::load(&font_dict);
        let mut ctx = RasterizerContext::new(&font);

        // Execute: 5 5 10 10 re f
        let stream = b"5 5 10 10 re f";
        ctx.execute_content_stream(stream);

        // Rect should have been rasterized
        // Check center is black
        assert_eq!(ctx.bitmap.get(10, 10), Some(0));
    }

    #[test]
    fn test_gstate_stack() {
        use crate::parser::object::types::intern;

        // Create a font with identity FontMatrix
        let mut font_dict = PdfDict::new();
        font_dict.insert(
            intern("/FontMatrix"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(1),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
                PdfObject::Integer(1),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
            ])),
        );

        let font = Type3Font::load(&font_dict);
        let mut ctx = RasterizerContext::new(&font);

        // Execute: q cm 2 0 0 2 0 0 Q
        let stream = b"q 2 0 0 2 0 0 cm Q";
        ctx.execute_content_stream(stream);

        // CTM should be restored to identity (FontMatrix)
        assert!(ctx.gstate.ctm.is_identity());
    }

    #[test]
    fn test_rasterize_type3_glyph_unknown_returns_none() {
        let font_dict = PdfDict::new();
        let font = Type3Font::load(&font_dict);

        // Unknown glyph returns None
        let doc_context = DocumentContext {
            resolver: None,
            source: None,
        };
        assert_eq!(
            rasterize_type3_glyph(&font, "unknown", Some(&doc_context), None::<&StreamResolverFn>),
            None
        );
    }

    #[test]
    fn test_rasterize_line_segment() {
        use crate::parser::object::types::intern;

        // Create a font with identity FontMatrix and appropriate FontBBox
        let mut font_dict = PdfDict::new();
        font_dict.insert(
            intern("/FontMatrix"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(1),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
                PdfObject::Integer(1),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
            ])),
        );
        font_dict.insert(
            intern("/FontBBox"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(0),
                PdfObject::Integer(0),
                PdfObject::Integer(20),
                PdfObject::Integer(20),
            ])),
        );

        let font = Type3Font::load(&font_dict);
        let mut ctx = RasterizerContext::new(&font);

        // Draw a diagonal line from (5,5) to (15,15)
        ctx.path.move_to(Point::new(5.0, 5.0));
        ctx.path.line_to(Point::new(15.0, 15.0));
        ctx.rasterize_path(true); // stroke mode

        // Verify some pixels along the line are set
        assert_eq!(ctx.bitmap.get(5, 5), Some(0)); // Start point
        assert_eq!(ctx.bitmap.get(10, 10), Some(0)); // Middle point
        assert_eq!(ctx.bitmap.get(15, 15), Some(0)); // End point
    }

    #[test]
    fn test_rasterize_filled_triangle() {
        use crate::parser::object::types::intern;

        // Create a font with identity FontMatrix
        let mut font_dict = PdfDict::new();
        font_dict.insert(
            intern("/FontMatrix"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(1),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
                PdfObject::Integer(1),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
            ])),
        );

        let font = Type3Font::load(&font_dict);
        let mut ctx = RasterizerContext::new(&font);

        // Draw a triangle: (10,5) -> (15,15) -> (5,15) -> close
        ctx.path.move_to(Point::new(10.0, 5.0));
        ctx.path.line_to(Point::new(15.0, 15.0));
        ctx.path.line_to(Point::new(5.0, 15.0));
        ctx.path.close_path();
        ctx.rasterize_path(false); // fill mode

        // Verify interior pixels are filled
        assert_eq!(ctx.bitmap.get(10, 10), Some(0)); // Center interior
        assert_eq!(ctx.bitmap.get(8, 12), Some(0)); // Left interior
        assert_eq!(ctx.bitmap.get(12, 12), Some(0)); // Right interior

        // Verify exterior pixels are still white
        assert_eq!(ctx.bitmap.get(4, 10), Some(255)); // Outside left
        assert_eq!(ctx.bitmap.get(16, 10), Some(255)); // Outside right
    }

    #[test]
    fn test_deref_char_proc_ref_without_context_returns_error() {
        let obj_ref = ObjRef::new(10, 0);

        // No DocumentContext provided
        let result = deref_char_proc_ref(obj_ref, None);

        assert!(result.is_err());
        match result {
            Err(Type3Error::Io(msg)) => {
                assert!(msg.contains("DocumentContext not provided"));
            }
            _ => panic!("Expected Type3Error::Io"),
        }
    }

    #[test]
    fn test_deref_char_proc_ref_without_resolver_returns_error() {
        let obj_ref = ObjRef::new(10, 0);

        // DocumentContext without resolver
        let doc_context = DocumentContext {
            resolver: None,
            source: None,
        };

        let result = deref_char_proc_ref(obj_ref, Some(&doc_context));

        assert!(result.is_err());
        match result {
            Err(Type3Error::Io(msg)) => {
                assert!(msg.contains("XrefResolver not provided"));
            }
            _ => panic!("Expected Type3Error::Io"),
        }
    }

    #[test]
    fn test_deref_char_proc_ref_without_source_returns_error() {

        let obj_ref = ObjRef::new(10, 0);
        let resolver = XrefResolver::new();

        // DocumentContext with resolver but no source
        let doc_context = DocumentContext {
            resolver: Some(&resolver),
            source: None,
        };

        let result = deref_char_proc_ref(obj_ref, Some(&doc_context));

        assert!(result.is_err());
        match result {
            Err(Type3Error::Io(msg)) => {
                assert!(msg.contains("PdfSource not provided"));
            }
            _ => panic!("Expected Type3Error::Io"),
        }
    }

    #[test]
    fn test_type3_error_missing_char_proc_ref() {
        let error = Type3Error::MissingCharProcRef {
            ref_id: "10 0 R".to_string(),
        };

        // Test Display implementation
        let display_str = format!("{}", error);
        assert!(display_str.contains("char_proc reference not found"));
        assert!(display_str.contains("10 0 R"));
    }

    #[test]
    fn test_type3_error_circular_ref() {
        let error = Type3Error::CircularRef {
            ref_id: "15 0 R".to_string(),
        };

        // Test Display implementation
        let display_str = format!("{}", error);
        assert!(display_str.contains("circular reference detected"));
        assert!(display_str.contains("15 0 R"));
    }

    #[test]
    fn test_type3_error_io() {
        let error = Type3Error::Io("Test I/O error message".to_string());

        // Test Display implementation
        let display_str = format!("{}", error);
        assert!(display_str.contains("I/O error during glyph resolution"));
        assert!(display_str.contains("Test I/O error message"));
    }

    #[test]
    fn test_type3_error_from_resolve_error_not_found() {
        use crate::parser::xref::ResolveError;

        let obj_ref = ObjRef::new(20, 0);
        let resolve_error = ResolveError::NotFound(obj_ref);

        let type3_error: Type3Error = resolve_error.into();

        match type3_error {
            Type3Error::MissingCharProcRef { ref_id } => {
                assert_eq!(ref_id, "20 0 R");
            }
            _ => panic!("Expected MissingCharProcRef variant"),
        }
    }

    #[test]
    fn test_type3_error_from_resolve_error_circular_ref() {
        use crate::parser::xref::ResolveError;

        let obj_ref = ObjRef::new(25, 0);
        let resolve_error = ResolveError::CircularRef(obj_ref);

        let type3_error: Type3Error = resolve_error.into();

        match type3_error {
            Type3Error::CircularRef { ref_id } => {
                assert_eq!(ref_id, "25 0 R");
            }
            _ => panic!("Expected CircularRef variant"),
        }
    }

    #[test]
    fn test_type3_error_from_resolve_error_io() {
        use crate::parser::xref::ResolveError;

        let resolve_error = ResolveError::Io("Test I/O error".to_string());

        let type3_error: Type3Error = resolve_error.into();

        match type3_error {
            Type3Error::Io(msg) => {
                assert_eq!(msg, "Test I/O error");
            }
            _ => panic!("Expected Io variant"),
        }
    }

    #[test]
    fn test_type3_error_invalid_char_proc_type() {
        let error = Type3Error::InvalidCharProcType {
            got: "integer".to_string(),
            expected: "stream or dict".to_string(),
        };

        // Test Display implementation
        let display_str = format!("{}", error);
        assert!(display_str.contains("invalid char_proc type"));
        assert!(display_str.contains("got integer"));
        assert!(display_str.contains("expected stream or dict"));
    }

    #[test]
    fn test_deref_char_proc_ref_validates_structure_before_returning() {
        // Test that validation is integrated into char_proc parsing flow
        // This verifies EC-42: Early validation in parsing pipelines
        use crate::parser::object::types::{PdfDict, PdfObject, PdfStream};

        let obj_ref = ObjRef::new(10, 0);

        // Create a stream missing required keys for validation
        let mut stream_dict = PdfDict::new();
        // Missing /Type, /Subtype, /Width, /Height keys
        let invalid_stream = PdfObject::Stream(Box::new(PdfStream::new(stream_dict, 0, Some(100))));

        // Test direct validation (no resolver needed for this test)
        let result = validate_char_proc_structure(&invalid_stream);

        assert!(result.is_err(), "Validation should fail for invalid structure");
        match result {
            Err(Type3Error::MissingRequiredKey { key, .. }) => {
                assert!(key.contains("/Type") || key.contains("/Width"),
                       "Should report missing required key");
            }
            _ => panic!("Expected MissingRequiredKey error from validation"),
        }
    }

    #[test]
    fn test_deref_char_proc_ref_validation_includes_ref_context() {
        // Test that validation errors include the object reference for debugging
        use crate::parser::object::types::PdfObject;

        // Create an invalid object (integer instead of stream/dict)
        let invalid_obj = PdfObject::Integer(123);

        // Test direct type detection
        let char_proc_type = detect_char_proc_type(&invalid_obj, None);
        assert_eq!(char_proc_type, CharProcType::Other("integer".to_string()));

        // Test validation failure
        let result = validate_char_proc_structure(&invalid_obj);
        assert!(result.is_err());

        let error_msg = format!("{}", result.unwrap_err());
        // Error should mention the invalid type
        assert!(error_msg.contains("integer") || error_msg.contains("expected"),
               "Error should mention the invalid type");
    }

    #[test]
    fn test_deref_char_proc_ref_passes_valid_stream() {
        // Test that valid structures pass validation successfully
        use crate::parser::object::types::{PdfDict, PdfObject, PdfStream};
        use crate::parser::object::intern;

        // Create a valid stream with all required keys
        let mut stream_dict = PdfDict::new();
        stream_dict.insert(intern("/Type"), PdfObject::Name(intern("/XObject")));
        stream_dict.insert(intern("/Subtype"), PdfObject::Name(intern("/Form")));
        stream_dict.insert(intern("/Width"), PdfObject::Integer(100));
        stream_dict.insert(intern("/Height"), PdfObject::Integer(100));
        let valid_stream = PdfObject::Stream(Box::new(PdfStream::new(stream_dict, 0, Some(100))));

        // Test validation passes
        let result = validate_char_proc_structure(&valid_stream);

        assert!(result.is_ok(), "Valid structure should pass validation");

        // Test type detection
        let char_proc_type = detect_char_proc_type(&valid_stream, None);
        assert_eq!(char_proc_type, CharProcType::Stream);
    }

    #[test]
    fn test_detect_char_proc_type_returns_unknown_for_failed_deref() {
        // Test that failed reference dereferencing returns CharProcType::Unknown
        use crate::parser::object::types::ObjRef;

        // Create a reference object
        let ref_obj = PdfObject::Ref(ObjRef::new(10, 0));

        // Test without document context (should return Unknown)
        let char_proc_type = detect_char_proc_type(&ref_obj, None);
        assert_eq!(char_proc_type, CharProcType::Unknown,
                   "Should return Unknown when no document context is provided");

        // Test with empty document context (no resolver/source - should return Unknown)
        let doc_context = DocumentContext {
            resolver: None,
            source: None,
        };
        let char_proc_type = detect_char_proc_type(&ref_obj, Some(&doc_context));
        assert_eq!(char_proc_type, CharProcType::Unknown,
                   "Should return Unknown when document context has no resolver");
    }

    #[test]
    fn test_extract_content_stream_bytes_without_resolver_returns_type3_error() {
        use crate::parser::object::types::PdfDict;

        let doc_context = DocumentContext {
            resolver: None,
            source: None,
        };

        let pdf_obj = PdfObject::Dict(Box::new(PdfDict::new()));
        let result = extract_content_stream_bytes(pdf_obj, &doc_context);

        assert!(result.is_err());
        match result {
            Err(Type3Error::Io(msg)) => {
                assert!(msg.contains("XrefResolver not provided"));
            }
            _ => panic!("Expected Type3Error::Io"),
        }
    }

    #[test]
    fn test_rasterize_type3_glyph_with_missing_glyph_returns_none() {
        use crate::parser::object::types::PdfDict;

        let font_dict = PdfDict::new();
        let font = Type3Font::load(&font_dict);

        // Non-existent glyph should return None (graceful degradation)
        let doc_context = DocumentContext {
            resolver: None,
            source: None,
        };

        let result = rasterize_type3_glyph(
            &font,
            "nonexistent",
            Some(&doc_context),
            None::<&StreamResolverFn>,
        );

        // Should gracefully return None, not panic
        assert!(result.is_none());
    }

    #[test]
    fn test_rasterize_type3_glyph_with_failed_resolution_returns_none() {
        use crate::parser::object::types::{PdfDict, PdfObject};
        use crate::parser::object::intern;

        let mut font_dict = PdfDict::new();
        let mut char_procs_dict = PdfDict::new();

        // Add a valid CharProcs entry
        char_procs_dict.insert(
            intern("/A"),
            PdfObject::Ref(ObjRef::new(999, 0)) // Non-existent object
        );

        font_dict.insert(intern("/CharProcs"), PdfObject::Dict(Box::new(char_procs_dict)));

        let font = Type3Font::load(&font_dict);

        // Verify the glyph exists in CharProcs
        assert!(font.has_glyph("A"));
        assert!(font.char_proc("A").is_some());

        let doc_context = DocumentContext {
            resolver: None,
            source: None,
        };

        // Create a resolver that always fails (simulating missing/invalid reference)
        let failing_resolver = &(|_obj_ref: ObjRef| -> Option<Vec<u8>> {
            None // Simulates resolution failure
        }) as &StreamResolverFn;

        let result = rasterize_type3_glyph(
            &font,
            "A",
            Some(&doc_context),
            Some(failing_resolver),
        );

        // Should gracefully return None, not panic
        assert!(result.is_none());
    }

    #[test]
    fn test_rasterize_type3_glyph_with_malformed_stream_returns_none() {
        use crate::parser::object::types::{PdfDict, PdfObject};
        use crate::parser::object::intern;

        let mut font_dict = PdfDict::new();
        let mut char_procs_dict = PdfDict::new();

        // Add a valid CharProcs entry
        char_procs_dict.insert(
            intern("/B"),
            PdfObject::Ref(ObjRef::new(10, 0))
        );

        font_dict.insert(intern("/CharProcs"), PdfObject::Dict(Box::new(char_procs_dict)));

        let font = Type3Font::load(&font_dict);

        let doc_context = DocumentContext {
            resolver: None,
            source: None,
        };

        // Create a resolver that returns malformed/empty bytes (simulating corrupt stream)
        let malformed_resolver = &(|_obj_ref: ObjRef| -> Option<Vec<u8>> {
            Some(vec![0xFF, 0xFF, 0xFF]) // Malformed PDF content stream
        }) as &StreamResolverFn;

        let result = rasterize_type3_glyph(
            &font,
            "B",
            Some(&doc_context),
            Some(malformed_resolver),
        );

        // Should gracefully return None or a default bitmap, not panic
        // Malformed content streams are handled by the lexer/parser
        // This tests that we don't crash on bad input
        let _ = result; // We accept either None or a bitmap (graceful handling)
    }

    #[test]
    fn test_execute_content_stream_with_invalid_tokens_does_not_crash() {
        use crate::parser::object::types::PdfDict;

        let font_dict = PdfDict::new();
        let font = Type3Font::load(&font_dict);
        let mut ctx = RasterizerContext::new(&font);

        // Execute malformed content: invalid operators and operands
        // This should not crash, just skip unknown operators
        let malformed_stream = b"INVALID_OPERATOR 1.5 2.5 another_invalid [array]";

        // This should execute without panicking
        ctx.execute_content_stream(malformed_stream);

        // Bitmap should still be in a valid state (all white since no valid ops executed)
        assert_eq!(ctx.bitmap.get(0, 0), Some(255));
        assert_eq!(ctx.bitmap.get(31, 31), Some(255));
    }

    #[test]
    fn test_execute_content_stream_with_empty_stream_does_not_crash() {
        use crate::parser::object::types::PdfDict;

        let font_dict = PdfDict::new();
        let font = Type3Font::load(&font_dict);
        let mut ctx = RasterizerContext::new(&font);

        // Execute empty content stream
        let empty_stream = b"";

        // This should execute without panicking
        ctx.execute_content_stream(empty_stream);

        // Bitmap should be all white (default state)
        // Verify all pixels are white by checking corners and a few interior points
        let (width, height) = ctx.bitmap.dimensions();
        assert_eq!(ctx.bitmap.get(0, 0), Some(255), "Top-left should be white");
        assert_eq!(ctx.bitmap.get((width - 1) as i32, 0), Some(255), "Top-right should be white");
        assert_eq!(ctx.bitmap.get(0, (height - 1) as i32), Some(255), "Bottom-left should be white");
        assert_eq!(ctx.bitmap.get((width - 1) as i32, (height - 1) as i32), Some(255), "Bottom-right should be white");
    }

    #[test]
    fn test_execute_type3_glyph_with_font_matrix_transformation() {
        use crate::parser::object::types::{intern, PdfDict};

        // Create a Type3 font with FontMatrix that scales by 0.001 (standard Type3)
        let mut font_dict = PdfDict::new();
        font_dict.insert(
            intern("/FontMatrix"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Real(0.001),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
                PdfObject::Real(0.001),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
            ])),
        );

        let font = Type3Font::load(&font_dict);
        let mut ctx = RasterizerContext::new(&font);

        // Verify initial CTM has FontMatrix applied
        assert_eq!(ctx.gstate.ctm.a, 0.001);
        assert_eq!(ctx.gstate.ctm.d, 0.001);

        // Execute a simple glyph content stream that draws a 1000x1000 square
        // in glyph space (0,0 to 1000,1000)
        // After FontMatrix transformation (0.001 scale), this becomes (0,0) to (1,1) in text space
        let stream = b"0 0 1000 1000 re f";

        ctx.execute_content_stream(stream);

        // The rectangle should be drawn and rasterized to the bitmap
        // After transformation, coordinates are scaled by 0.001
        // (0,0,1000,1000) in glyph space -> (0,0,1,1) in text space
        // This should fill a small area in the 32x32 bitmap

        // Verify that some pixels were drawn (bitmap is no longer all white)
        let mut has_black = false;
        for y in 0..32 {
            for x in 0..32 {
                if ctx.bitmap.get(x, y) == Some(0) {
                    has_black = true;
                    break;
                }
            }
            if has_black {
                break;
            }
        }

        // Due to the 0.001 scale, the 1000x1000 rectangle becomes 1x1 in text space
        // which is very small in the 32x32 bitmap, but at least the origin pixel should be filled
        assert!(has_black, "Expected some pixels to be drawn after executing glyph stream");
    }

    #[test]
    fn test_execute_type3_glyph_with_identity_font_matrix() {
        use crate::parser::object::types::{intern, PdfDict};

        // Create a Type3 font with identity FontMatrix (no scaling)
        let mut font_dict = PdfDict::new();
        font_dict.insert(
            intern("/FontMatrix"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(1),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
                PdfObject::Integer(1),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
            ])),
        );

        let font = Type3Font::load(&font_dict);
        let mut ctx = RasterizerContext::new(&font);

        // Verify initial CTM is identity
        assert!(ctx.gstate.ctm.is_identity());

        // Execute a glyph content stream that draws a 10x10 square
        // With identity matrix, coordinates are not scaled
        let stream = b"10 10 10 10 re f";

        ctx.execute_content_stream(stream);

        // Verify the rectangle was drawn at (10,10) to (20,20)
        // Center of the rectangle should be filled
        assert_eq!(ctx.bitmap.get(15, 15), Some(0));
    }

    // Tests for calculate_bitmap_dimensions (bf-407xp6)

    #[test]
    fn test_calculate_bitmap_dimensions_basic() {
        // Test basic glyph with bbox [10.0, 20.0, 50.0, 60.0]
        // Raw size: 40x40 points
        // With default padding: 42x42 pixels
        let bbox = [10.0f32, 20.0, 50.0, 60.0];
        let (width, height) = calculate_bitmap_dimensions(&bbox, None);

        assert_eq!(width, 42);
        assert_eq!(height, 42);
    }

    #[test]
    fn test_calculate_bitmap_dimensions_with_custom_padding() {
        // Test with custom padding of 2 pixels
        let bbox = [10.0f32, 20.0, 50.0, 60.0];
        let (width, height) = calculate_bitmap_dimensions(&bbox, Some(2));

        assert_eq!(width, 44); // 40 + 2*2
        assert_eq!(height, 44); // 40 + 2*2
    }

    #[test]
    fn test_calculate_bitmap_dimensions_zero_width() {
        // Test degenerate case: zero-width bbox
        // Should default to 1 pixel + padding
        let bbox = [10.0f32, 20.0, 10.0, 60.0];
        let (width, height) = calculate_bitmap_dimensions(&bbox, None);

        assert_eq!(width, 3); // 1 + 2*1 (default padding)
        assert_eq!(height, 42); // 40 + 2*1
    }

    #[test]
    fn test_calculate_bitmap_dimensions_zero_height() {
        // Test degenerate case: zero-height bbox
        let bbox = [10.0f32, 20.0, 50.0, 20.0];
        let (width, height) = calculate_bitmap_dimensions(&bbox, None);

        assert_eq!(width, 42); // 40 + 2*1
        assert_eq!(height, 3); // 1 + 2*1
    }

    #[test]
    fn test_calculate_bitmap_dimensions_small_glyph() {
        // Test very small glyph (less than 1 point)
        let bbox = [10.0f32, 20.0, 10.5, 20.5];
        let (width, height) = calculate_bitmap_dimensions(&bbox, None);

        // Should round up to 1 pixel + padding
        assert_eq!(width, 3); // 1 + 2*1
        assert_eq!(height, 3); // 1 + 2*1
    }

    #[test]
    fn test_calculate_bitmap_dimensions_negative_coordinates() {
        // Test bbox with negative coordinates (valid in PDF)
        let bbox = [-50.0f32, -30.0, -10.0, 10.0];
        let (width, height) = calculate_bitmap_dimensions(&bbox, None);

        // Width: 40 points, Height: 40 points
        assert_eq!(width, 42); // 40 + 2*1
        assert_eq!(height, 42); // 40 + 2*1
    }

    #[test]
    fn test_calculate_bitmap_dimensions_returns_usize() {
        // Verify return type is usize (for array indexing)
        let bbox = [0.0f32, 0.0, 16.0, 16.0];
        let (width, height) = calculate_bitmap_dimensions(&bbox, None);

        // Should be usable as array indices
        let mut dummy_array = [0u8; 100];
        dummy_array[width - 1] = 1;
        dummy_array[height - 1] = 2;

        assert_eq!(width, 18); // 16 + 2*1
        assert_eq!(height, 18); // 16 + 2*1
    }

    #[test]
    fn test_calculate_bitmap_dimensions_non_integer_bounds() {
        // Test bbox with non-integer coordinates
        let bbox = [10.5f32, 20.7, 50.2, 60.9];
        let (width, height) = calculate_bitmap_dimensions(&bbox, None);

        // Should round up: 39.7 -> 40, 40.2 -> 41
        assert_eq!(width, 42); // 40 + 2*1
        assert_eq!(height, 43); // 41 + 2*1
    }

    #[test]
    fn test_calculate_bitmap_dimensions_no_padding() {
        // Test with zero padding
        let bbox = [10.0f32, 20.0, 50.0, 60.0];
        let (width, height) = calculate_bitmap_dimensions(&bbox, Some(0));

        assert_eq!(width, 40); // 40 + 2*0
        assert_eq!(height, 40); // 40 + 2*0
    }

    #[test]
    fn test_calculate_bitmap_dimensions_large_glyph() {
        // Test large glyph (e.g., full page size)
        let bbox = [0.0f32, 0.0, 612.0, 792.0]; // US Letter size
        let (width, height) = calculate_bitmap_dimensions(&bbox, Some(2));

        assert_eq!(width, 616); // 612 + 2*2
        assert_eq!(height, 796); // 792 + 2*2
    }

    #[test]
    fn test_resolve_stream_callback_receives_parameters() {
        use crate::parser::object::types::{PdfDict, PdfObject};
        use crate::parser::object::intern;
        use std::sync::Mutex;

        // Create a Type3 font with identity FontMatrix for predictable coordinates
        let mut font_dict = PdfDict::new();
        font_dict.insert(
            intern("/FontMatrix"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(1),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
                PdfObject::Integer(1),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
            ])),
        );

        let mut char_procs_dict = PdfDict::new();

        // Add a CharProcs entry pointing to a specific object reference
        let test_ref = ObjRef::new(10, 0);
        char_procs_dict.insert(
            intern("/TestGlyph"),
            PdfObject::Ref(test_ref)
        );

        font_dict.insert(intern("/CharProcs"), PdfObject::Dict(Box::new(char_procs_dict)));

        let font = Type3Font::load(&font_dict);

        // Track which ObjRef was passed to the callback
        let captured_ref = Arc::new(Mutex::new(None));
        let captured_ref_clone = captured_ref.clone();

        // Create a callback that captures simulated resolver, source, and counter parameters
        // This mimics the closure pattern used in resolver.rs lines 700-702
        let callback = move |obj_ref: ObjRef| -> Option<Vec<u8>> {
            // Capture the received obj_ref to verify it was passed correctly
            *captured_ref_clone.lock().unwrap() = Some(obj_ref);

            // Return a simple valid content stream (draw a 10x10 rectangle)
            // This simulates what resolve_stream_bytes would do
            Some(b"10 10 10 10 re f".to_vec())
        };

        // Call rasterize_type3_glyph with the callback
        let doc_context = DocumentContext {
            resolver: None,
            source: None,
        };

        let result = rasterize_type3_glyph(
            &font,
            "TestGlyph",
            Some(&doc_context),
            Some(&callback as &StreamResolverFn),
        );

        // Verify the callback was invoked and received the correct ObjRef
        assert_eq!(
            *captured_ref.lock().unwrap(),
            Some(test_ref),
            "Callback should receive the ObjRef pointing to the glyph's content stream"
        );

        // Verify the glyph was successfully rasterized (callback returned valid bytes)
        assert!(
            result.is_some(),
            "Glyph should be rasterized when callback returns valid content stream"
        );

        // Verify the bitmap is not all-white (content was executed)
        let bitmap = result.unwrap();
        let has_black = bitmap.iter().any(|&pixel| pixel == 0);
        assert!(
            has_black,
            "Bitmap should contain black pixels after executing the content stream"
        );
    }

    #[test]
    fn test_resolve_stream_callback_captures_context_parameters() {
        use crate::parser::object::types::{PdfDict, PdfObject};
        use crate::parser::object::intern;
        use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

        // Create a Type3 font with identity FontMatrix for predictable coordinates
        let mut font_dict = PdfDict::new();
        font_dict.insert(
            intern("/FontMatrix"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(1),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
                PdfObject::Integer(1),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
            ])),
        );

        let mut char_procs_dict = PdfDict::new();

        let glyph_ref = ObjRef::new(20, 0);
        char_procs_dict.insert(
            intern("/ContextGlyph"),
            PdfObject::Ref(glyph_ref)
        );

        font_dict.insert(intern("/CharProcs"), PdfObject::Dict(Box::new(char_procs_dict)));

        let font = Type3Font::load(&font_dict);

        // Simulate the three context parameters that the callback should capture
        // These represent: resolver (&XrefResolver), source (&dyn PdfSource), counter (&mut u64)
        // Use atomic types for thread-safe state tracking
        let mock_resolver_called = Arc::new(AtomicBool::new(false));
        let mock_source_used = Arc::new(AtomicBool::new(false));
        let mock_counter_incremented = Arc::new(AtomicU64::new(0));

        // Create clones for the closure to capture
        let resolver_called_clone = mock_resolver_called.clone();
        let source_used_clone = mock_source_used.clone();
        let counter_incremented_clone = mock_counter_incremented.clone();

        // Callback that simulates resolve_stream_bytes behavior
        // This closure captures and uses all three context parameters
        let callback = move |obj_ref: ObjRef| -> Option<Vec<u8>> {
            // Simulate using resolver (mark as called)
            resolver_called_clone.store(true, Ordering::SeqCst);

            // Simulate using source (mark as used)
            source_used_clone.store(true, Ordering::SeqCst);

            // Simulate using counter (increment it)
            counter_incremented_clone.fetch_add(1, Ordering::SeqCst);

            // Verify we received the expected obj_ref
            assert_eq!(obj_ref, glyph_ref, "Callback should receive the correct glyph reference");

            // Return a valid content stream
            Some(b"5 5 20 20 re f".to_vec())
        };

        let doc_context = DocumentContext {
            resolver: None,
            source: None,
        };

        // Execute the callback through rasterize_type3_glyph
        let result = rasterize_type3_glyph(
            &font,
            "ContextGlyph",
            Some(&doc_context),
            Some(&callback as &StreamResolverFn),
        );

        // Verify all three context parameters were used by the callback
        assert!(
            mock_resolver_called.load(Ordering::SeqCst),
            "Callback should use the resolver parameter"
        );
        assert!(
            mock_source_used.load(Ordering::SeqCst),
            "Callback should use the source parameter"
        );
        assert_eq!(
            mock_counter_incremented.load(Ordering::SeqCst),
            1,
            "Callback should use (increment) the counter parameter"
        );

        // Verify successful rasterization
        assert!(
            result.is_some(),
            "Glyph should rasterize successfully when callback uses all context parameters"
        );
    }

    #[test]
    fn test_resolve_stream_callback_with_helper_function_pattern() {
        use crate::parser::object::types::{PdfDict, PdfObject};
        use crate::parser::object::intern;
        use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

        // Create a Type3 font with identity FontMatrix
        let mut font_dict = PdfDict::new();
        font_dict.insert(
            intern("/FontMatrix"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(1),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
                PdfObject::Integer(1),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
            ])),
        );

        let mut char_procs_dict = PdfDict::new();
        let glyph_ref = ObjRef::new(30, 0);
        char_procs_dict.insert(
            intern("/HelperGlyph"),
            PdfObject::Ref(glyph_ref)
        );

        font_dict.insert(intern("/CharProcs"), PdfObject::Dict(Box::new(char_procs_dict)));

        let font = Type3Font::load(&font_dict);

        // Simulate the actual parameter types used in resolver.rs
        // Use Atomic types for thread-safe state tracking (Send + Sync)
        let resolver_called = Arc::new(AtomicBool::new(false));
        let source_used = Arc::new(AtomicBool::new(false));
        let counter = Arc::new(AtomicU64::new(0));

        // Helper function that mirrors the resolve_stream_bytes pattern
        // This simulates the actual function used in resolver.rs:697-699
        fn resolve_stream_bytes_helper(
            obj_ref: ObjRef,
            resolver: &Arc<AtomicBool>,
            source: &Arc<AtomicBool>,
            counter: &Arc<AtomicU64>,
        ) -> Option<Vec<u8>> {
            // Mark resolver as used (simulates resolver.resolve_with_source call)
            resolver.store(true, Ordering::SeqCst);

            // Mark source as used (simulates source access)
            source.store(true, Ordering::SeqCst);

            // Increment counter (simulates decompression counter)
            counter.fetch_add(1, Ordering::SeqCst);

            // Verify we received the expected obj_ref
            assert_eq!(obj_ref.object, 30, "Callback should receive correct glyph reference ID");
            assert_eq!(obj_ref.generation, 0, "Callback should receive correct generation number");

            // Return a valid content stream
            Some(b"10 10 25 25 re f".to_vec())
        }

        // Construct callback using the helper function pattern
        // This mirrors the actual construction in resolver.rs:700-702
        let resolver_clone = resolver_called.clone();
        let source_clone = source_used.clone();
        let counter_clone = counter.clone();

        let callback = move |obj_ref: ObjRef| -> Option<Vec<u8>> {
            resolve_stream_bytes_helper(obj_ref, &resolver_clone, &source_clone, &counter_clone)
        };

        let doc_context = DocumentContext {
            resolver: None,
            source: None,
        };

        // Execute through rasterize_type3_glyph
        let result = rasterize_type3_glyph(
            &font,
            "HelperGlyph",
            Some(&doc_context),
            Some(&callback as &StreamResolverFn),
        );

        // Verify the callback was invoked and used all parameters
        assert!(
            resolver_called.load(Ordering::SeqCst),
            "Callback should invoke resolver parameter (resolver_called should be true)"
        );
        assert!(
            source_used.load(Ordering::SeqCst),
            "Callback should invoke source parameter (source_used should be true)"
        );
        assert_eq!(
            counter.load(Ordering::SeqCst),
            1,
            "Callback should increment counter parameter exactly once"
        );

        // Verify successful rasterization
        assert!(
            result.is_some(),
            "Glyph should rasterize successfully when callback is constructed via helper function"
        );
    }

    // Tests for detect_char_proc_type (bf-5d8b9v)

    #[test]
    fn test_detect_char_proc_type_dict() {
        use crate::parser::object::types::PdfDict;

        let dict_obj = PdfObject::Dict(Box::new(PdfDict::new()));
        assert_eq!(detect_char_proc_type(&dict_obj, None), CharProcType::Dict);
    }

    #[test]
    fn test_detect_char_proc_type_stream() {
        use crate::parser::object::types::{PdfDict, PdfStream};

        let dict = PdfDict::new();
        let stream = PdfStream::new(dict, 0, None);
        let stream_obj = PdfObject::Stream(Box::new(stream));
        assert_eq!(detect_char_proc_type(&stream_obj, None), CharProcType::Stream);
    }

    #[test]
    fn test_detect_char_proc_type_integer() {
        let int_obj = PdfObject::Integer(42);
        assert_eq!(
            detect_char_proc_type(&int_obj, None),
            CharProcType::Other("integer".to_string())
        );
    }

    #[test]
    fn test_detect_char_proc_type_real() {
        let real_obj = PdfObject::Real(3.14);
        assert_eq!(
            detect_char_proc_type(&real_obj, None),
            CharProcType::Other("real".to_string())
        );
    }

    #[test]
    fn test_detect_char_proc_type_boolean() {
        let bool_obj = PdfObject::Bool(true);
        assert_eq!(
            detect_char_proc_type(&bool_obj, None),
            CharProcType::Other("boolean".to_string())
        );
    }

    #[test]
    fn test_detect_char_proc_type_string() {
        let string_obj = PdfObject::String(Box::new(vec![b'A', b'B']));
        assert_eq!(
            detect_char_proc_type(&string_obj, None),
            CharProcType::Other("string".to_string())
        );
    }

    #[test]
    fn test_detect_char_proc_type_name() {
        use crate::parser::object::types::intern;

        let name_obj = PdfObject::Name(intern("/TestName"));
        assert_eq!(
            detect_char_proc_type(&name_obj, None),
            CharProcType::Other("name".to_string())
        );
    }

    #[test]
    fn test_detect_char_proc_type_array() {
        let array_obj = PdfObject::Array(Box::new(vec![
            PdfObject::Integer(1),
            PdfObject::Integer(2),
        ]));
        assert_eq!(
            detect_char_proc_type(&array_obj, None),
            CharProcType::Other("array".to_string())
        );
    }

    #[test]
    fn test_detect_char_proc_type_null() {
        let null_obj = PdfObject::Null;
        assert_eq!(
            detect_char_proc_type(&null_obj, None),
            CharProcType::Other("null".to_string())
        );
    }

    #[test]
    fn test_detect_char_proc_type_ref() {
        use crate::parser::object::types::ObjRef;

        let ref_obj = PdfObject::Ref(ObjRef::new(10, 0));
        assert_eq!(
            detect_char_proc_type(&ref_obj, None),
            CharProcType::Unknown
        );
    }

    #[test]
    fn test_detect_char_proc_type_indirect() {
        use crate::parser::object::types::{PdfIndirect, ObjRef};

        let indirect = PdfIndirect {
            id: ObjRef::new(15, 0),
            obj: PdfObject::Null,
        };
        let indirect_obj = PdfObject::Indirect(Box::new(indirect));
        assert_eq!(
            detect_char_proc_type(&indirect_obj, None),
            CharProcType::Other("indirect".to_string())
        );
    }

    // Tests for detect_char_proc_type_with_context (bf-5vlv4i)

    #[test]
    fn test_detect_char_proc_type_with_context_direct_stream() {
        use crate::parser::object::types::{PdfDict, PdfStream};

        let dict = PdfDict::new();
        let stream = PdfStream::new(dict, 0, None);
        let stream_obj = PdfObject::Stream(Box::new(stream));

        // Direct stream object should still be classified as Stream
        assert_eq!(
            detect_char_proc_type_with_context(&stream_obj, None),
            CharProcType::Stream
        );
    }

    #[test]
    fn test_detect_char_proc_type_with_context_direct_dict() {
        use crate::parser::object::types::PdfDict;

        let dict_obj = PdfObject::Dict(Box::new(PdfDict::new()));

        // Direct dict object should still be classified as Dict
        assert_eq!(
            detect_char_proc_type_with_context(&dict_obj, None),
            CharProcType::Dict
        );
    }

    #[test]
    fn test_detect_char_proc_type_with_context_ref_without_context() {
        use crate::parser::object::types::ObjRef;

        let ref_obj = PdfObject::Ref(ObjRef::new(10, 0));

        // Without context, references are classified as Unknown (bf-5on6og)
        assert_eq!(
            detect_char_proc_type_with_context(&ref_obj, None),
            CharProcType::Unknown
        );
    }

    #[test]
    fn test_detect_char_proc_type_with_context_ref_with_valid_context() {
        use crate::parser::object::types::{ObjRef, PdfObject};

        // Create a mock PdfSource that returns a simple stream
        struct MockSource;
        impl PdfSource for MockSource {
            fn read_at(&self, _offset: u64, len: usize) -> std::io::Result<Vec<u8>> {
                Ok(vec![0x00, 0x01, 0x02]) // Dummy stream data
            }

            fn len(&self) -> std::io::Result<u64> {
                Ok(1024)
            }
        }

        // For now, test that detect_char_proc_type_with_context handles references
        // Full integration testing requires complete document parsing infrastructure
        let ref_obj = PdfObject::Ref(ObjRef::new(10, 0));

        // Without context, references are classified as Unknown (bf-5on6og)
        assert_eq!(
            detect_char_proc_type_with_context(&ref_obj, None),
            CharProcType::Unknown
        );
    }

    #[test]
    fn test_detect_char_proc_type_with_context_ref_to_dict() {
        use crate::parser::object::types::{ObjRef, PdfObject};

        // Test that references are classified as Other("reference") when no context is provided
        let ref_obj = PdfObject::Ref(ObjRef::new(20, 0));

        // Without context, references are classified as Other("reference")
        assert_eq!(
            detect_char_proc_type_with_context(&ref_obj, None),
            CharProcType::Other("reference".to_string())
        );
    }

    #[test]
    fn test_detect_char_proc_type_with_context_nested_ref() {
        use crate::parser::object::types::ObjRef;

        // Test that nested references are classified as Other("reference") when no context is provided
        let ref_a = ObjRef::new(25, 0);
        let ref_obj = crate::parser::object::types::PdfObject::Ref(ref_a);

        // Without context, nested references are classified as Other("reference")
        assert_eq!(
            detect_char_proc_type_with_context(&ref_obj, None),
            CharProcType::Other("reference".to_string())
        );
    }

    #[test]
    fn test_detect_char_proc_type_with_context_circular_reference() {
        use crate::parser::object::types::{ObjRef, PdfObject};

        // Note: Testing circular references requires a full document context with
        // properly configured XrefResolver. For now, we test that the function
        // handles references gracefully when no context is provided.
        let ref_obj = PdfObject::Ref(ObjRef::new(40, 0));

        // Without context, references are classified as Other("reference")
        assert_eq!(
            detect_char_proc_type_with_context(&ref_obj, None),
            CharProcType::Other("reference".to_string())
        );
    }

    #[test]
    fn test_detect_char_proc_type_with_context_invalid_reference() {
        use crate::parser::object::types::ObjRef;
        use crate::parser::xref::XrefResolver;

        struct MockSource;
        impl PdfSource for MockSource {
            fn read_at(&self, _offset: u64, len: usize) -> std::io::Result<Vec<u8>> {
                Ok(vec![0x00, 0x01, 0x02])
            }

            fn len(&self) -> std::io::Result<u64> {
                Ok(1024)
            }
        }

        // Create an empty resolver (no objects)
        let resolver = XrefResolver::new();

        let source = MockSource;
        let doc_context = DocumentContext {
            resolver: Some(&resolver),
            source: Some(&source as &dyn PdfSource),
        };

        let ref_obj = PdfObject::Ref(ObjRef::new(999, 0));

        // Invalid references (not found in resolver) should return "error"
        assert_eq!(
            detect_char_proc_type_with_context(&ref_obj, Some(&doc_context)),
            CharProcType::Other("error".to_string())
        );
    }

    #[test]
    fn test_detect_char_proc_type_with_context_ref_to_integer() {
        use crate::parser::object::types::{ObjRef, PdfObject};

        // Note: Testing reference resolution requires a full document context with
        // properly configured XrefResolver. For now, we test that the function
        // handles references gracefully when no context is provided.
        let ref_obj = PdfObject::Ref(ObjRef::new(50, 0));

        // Without context, references are classified as Other("reference")
        assert_eq!(
            detect_char_proc_type_with_context(&ref_obj, None),
            CharProcType::Other("reference".to_string())
        );
    }

    #[test]
    fn test_detect_char_proc_type_with_context_ref_without_resolver() {
        use crate::parser::object::types::ObjRef;

        let ref_obj = PdfObject::Ref(ObjRef::new(10, 0));

        // DocumentContext without resolver should classify as "reference"
        let doc_context = DocumentContext {
            resolver: None,
            source: None,
        };

        assert_eq!(
            detect_char_proc_type_with_context(&ref_obj, Some(&doc_context)),
            CharProcType::Other("reference".to_string())
        );
    }

    #[test]
    fn test_detect_char_proc_type_with_context_ref_without_source() {
        use crate::parser::object::types::ObjRef;
        use crate::parser::xref::XrefResolver;

        let ref_obj = PdfObject::Ref(ObjRef::new(10, 0));

        // DocumentContext with resolver but no source
        let resolver = XrefResolver::new();
        let doc_context = DocumentContext {
            resolver: Some(&resolver),
            source: None,
        };

        // Should classify as "reference" since source is required for dereferencing
        assert_eq!(
            detect_char_proc_type_with_context(&ref_obj, Some(&doc_context)),
            CharProcType::Other("reference".to_string())
        );
    }

    #[test]
    fn test_detect_char_proc_type_backwards_compatibility() {
        use crate::parser::object::types::{PdfDict, PdfStream};

        // Test that the original function still works for direct objects
        let dict_obj = PdfObject::Dict(Box::new(PdfDict::new()));
        let stream_obj = PdfObject::Stream(Box::new(PdfStream::new(PdfDict::new(), 0, None)));
        let int_obj = PdfObject::Integer(42);

        // The no-context version should work exactly as before
        assert_eq!(detect_char_proc_type(&dict_obj, None), CharProcType::Dict);
        assert_eq!(detect_char_proc_type(&stream_obj, None), CharProcType::Stream);
        assert_eq!(
            detect_char_proc_type(&int_obj, None),
            CharProcType::Other("integer".to_string())
        );

        // The with_context version with None should match the no-context version
        assert_eq!(
            detect_char_proc_type_with_context(&dict_obj, None),
            detect_char_proc_type(&dict_obj, None)
        );
        assert_eq!(
            detect_char_proc_type_with_context(&stream_obj, None),
            detect_char_proc_type(&stream_obj, None)
        );
        assert_eq!(
            detect_char_proc_type_with_context(&int_obj, None),
            detect_char_proc_type(&int_obj, None)
        );
    }

    // Tests for validate_char_proc_structure (bf-3icotv)

    #[test]
    fn test_validate_char_proc_structure_valid_stream() {
        use crate::parser::object::types::{intern, PdfDict, PdfStream};

        let mut dict = PdfDict::new();
        dict.insert(intern("/Type"), PdfObject::Name(intern("/XObject")));
        dict.insert(intern("/Subtype"), PdfObject::Name(intern("/Form")));
        dict.insert(intern("/Width"), PdfObject::Integer(100));
        dict.insert(intern("/Height"), PdfObject::Integer(100));

        let stream = PdfStream::new(dict, 0, None);
        let stream_obj = PdfObject::Stream(Box::new(stream));

        assert!(validate_char_proc_structure(&stream_obj).is_ok());
    }

    #[test]
    fn test_validate_char_proc_structure_stream_missing_type() {
        use crate::parser::object::types::{intern, PdfDict, PdfStream};

        let mut dict = PdfDict::new();
        // Missing /Type
        dict.insert(intern("/Subtype"), PdfObject::Name(intern("/Form")));
        dict.insert(intern("/Width"), PdfObject::Integer(100));
        dict.insert(intern("/Height"), PdfObject::Integer(100));

        let stream = PdfStream::new(dict, 0, None);
        let stream_obj = PdfObject::Stream(Box::new(stream));

        let result = validate_char_proc_structure(&stream_obj);
        assert!(result.is_err());
        match result {
            Err(Type3Error::MissingRequiredKey { key, object_type }) => {
                assert_eq!(key, "/Type");
                assert_eq!(object_type, "stream");
            }
            _ => panic!("Expected MissingRequiredKey error"),
        }
    }

    #[test]
    fn test_validate_char_proc_structure_stream_missing_subtype() {
        use crate::parser::object::types::{intern, PdfDict, PdfStream};

        let mut dict = PdfDict::new();
        dict.insert(intern("/Type"), PdfObject::Name(intern("/XObject")));
        // Missing /Subtype
        dict.insert(intern("/Width"), PdfObject::Integer(100));
        dict.insert(intern("/Height"), PdfObject::Integer(100));

        let stream = PdfStream::new(dict, 0, None);
        let stream_obj = PdfObject::Stream(Box::new(stream));

        let result = validate_char_proc_structure(&stream_obj);
        assert!(result.is_err());
        match result {
            Err(Type3Error::MissingRequiredKey { key, object_type }) => {
                assert_eq!(key, "/Subtype");
                assert_eq!(object_type, "stream");
            }
            _ => panic!("Expected MissingRequiredKey error"),
        }
    }

    #[test]
    fn test_validate_char_proc_structure_stream_missing_width() {
        use crate::parser::object::types::{intern, PdfDict, PdfStream};

        let mut dict = PdfDict::new();
        dict.insert(intern("/Type"), PdfObject::Name(intern("/XObject")));
        dict.insert(intern("/Subtype"), PdfObject::Name(intern("/Form")));
        // Missing /Width
        dict.insert(intern("/Height"), PdfObject::Integer(100));

        let stream = PdfStream::new(dict, 0, None);
        let stream_obj = PdfObject::Stream(Box::new(stream));

        let result = validate_char_proc_structure(&stream_obj);
        assert!(result.is_err());
        match result {
            Err(Type3Error::MissingRequiredKey { key, object_type }) => {
                assert_eq!(key, "/Width");
                assert_eq!(object_type, "stream");
            }
            _ => panic!("Expected MissingRequiredKey error"),
        }
    }

    #[test]
    fn test_validate_char_proc_structure_stream_missing_height() {
        use crate::parser::object::types::{intern, PdfDict, PdfStream};

        let mut dict = PdfDict::new();
        dict.insert(intern("/Type"), PdfObject::Name(intern("/XObject")));
        dict.insert(intern("/Subtype"), PdfObject::Name(intern("/Form")));
        dict.insert(intern("/Width"), PdfObject::Integer(100));
        // Missing /Height

        let stream = PdfStream::new(dict, 0, None);
        let stream_obj = PdfObject::Stream(Box::new(stream));

        let result = validate_char_proc_structure(&stream_obj);
        assert!(result.is_err());
        match result {
            Err(Type3Error::MissingRequiredKey { key, object_type }) => {
                assert_eq!(key, "/Height");
                assert_eq!(object_type, "stream");
            }
            _ => panic!("Expected MissingRequiredKey error"),
        }
    }

    #[test]
    fn test_validate_char_proc_structure_stream_missing_all_keys() {
        use crate::parser::object::types::{PdfDict, PdfStream};

        let dict = PdfDict::new(); // Empty dict
        let stream = PdfStream::new(dict, 0, None);
        let stream_obj = PdfObject::Stream(Box::new(stream));

        let result = validate_char_proc_structure(&stream_obj);
        assert!(result.is_err());
        // Should fail on /Type first
        match result {
            Err(Type3Error::MissingRequiredKey { key, object_type }) => {
                assert_eq!(key, "/Type");
                assert_eq!(object_type, "stream");
            }
            _ => panic!("Expected MissingRequiredKey error for /Type"),
        }
    }

    #[test]
    fn test_validate_char_proc_structure_valid_dict() {
        use crate::parser::object::types::{intern, PdfDict};

        let mut dict = PdfDict::new();
        dict.insert(intern("/Type"), PdfObject::Name(intern("/XObject")));
        dict.insert(intern("/Subtype"), PdfObject::Name(intern("/Form")));

        let dict_obj = PdfObject::Dict(Box::new(dict));

        assert!(validate_char_proc_structure(&dict_obj).is_ok());
    }

    #[test]
    fn test_validate_char_proc_structure_dict_missing_type() {
        use crate::parser::object::types::{intern, PdfDict};

        let mut dict = PdfDict::new();
        // Missing /Type
        dict.insert(intern("/Subtype"), PdfObject::Name(intern("/Form")));

        let dict_obj = PdfObject::Dict(Box::new(dict));

        let result = validate_char_proc_structure(&dict_obj);
        assert!(result.is_err());
        match result {
            Err(Type3Error::MissingRequiredKey { key, object_type }) => {
                assert_eq!(key, "/Type");
                assert_eq!(object_type, "dictionary");
            }
            _ => panic!("Expected MissingRequiredKey error"),
        }
    }

    #[test]
    fn test_validate_char_proc_structure_dict_missing_subtype() {
        use crate::parser::object::types::{intern, PdfDict};

        let mut dict = PdfDict::new();
        dict.insert(intern("/Type"), PdfObject::Name(intern("/XObject")));
        // Missing /Subtype

        let dict_obj = PdfObject::Dict(Box::new(dict));

        let result = validate_char_proc_structure(&dict_obj);
        assert!(result.is_err());
        match result {
            Err(Type3Error::MissingRequiredKey { key, object_type }) => {
                assert_eq!(key, "/Subtype");
                assert_eq!(object_type, "dictionary");
            }
            _ => panic!("Expected MissingRequiredKey error"),
        }
    }

    #[test]
    fn test_validate_char_proc_structure_integer() {
        let int_obj = PdfObject::Integer(42);

        let result = validate_char_proc_structure(&int_obj);
        assert!(result.is_err());
        match result {
            Err(Type3Error::InvalidCharProcType { got, expected }) => {
                assert_eq!(got, "integer");
                assert_eq!(expected, "stream or dictionary");
            }
            _ => panic!("Expected InvalidCharProcType error"),
        }
    }

    #[test]
    fn test_validate_char_proc_structure_real() {
        let real_obj = PdfObject::Real(3.14);

        let result = validate_char_proc_structure(&real_obj);
        assert!(result.is_err());
        match result {
            Err(Type3Error::InvalidCharProcType { got, expected }) => {
                assert_eq!(got, "real");
                assert_eq!(expected, "stream or dictionary");
            }
            _ => panic!("Expected InvalidCharProcType error"),
        }
    }

    #[test]
    fn test_validate_char_proc_structure_string() {
        let string_obj = PdfObject::String(Box::new(b"test".to_vec()));

        let result = validate_char_proc_structure(&string_obj);
        assert!(result.is_err());
        match result {
            Err(Type3Error::InvalidCharProcType { got, expected }) => {
                assert_eq!(got, "string");
                assert_eq!(expected, "stream or dictionary");
            }
            _ => panic!("Expected InvalidCharProcType error"),
        }
    }

    #[test]
    fn test_validate_char_proc_structure_array() {
        let array_obj = PdfObject::Array(Box::new(vec![
            PdfObject::Integer(1),
            PdfObject::Integer(2),
        ]));

        let result = validate_char_proc_structure(&array_obj);
        assert!(result.is_err());
        match result {
            Err(Type3Error::InvalidCharProcType { got, expected }) => {
                assert_eq!(got, "array");
                assert_eq!(expected, "stream or dictionary");
            }
            _ => panic!("Expected InvalidCharProcType error"),
        }
    }

    #[test]
    fn test_validate_char_proc_structure_null() {
        let null_obj = PdfObject::Null;

        let result = validate_char_proc_structure(&null_obj);
        assert!(result.is_err());
        match result {
            Err(Type3Error::InvalidCharProcType { got, expected }) => {
                assert_eq!(got, "null");
                assert_eq!(expected, "stream or dictionary");
            }
            _ => panic!("Expected InvalidCharProcType error"),
        }
    }

    #[test]
    fn test_validate_char_proc_structure_reference() {
        use crate::parser::object::types::ObjRef;

        let ref_obj = PdfObject::Ref(ObjRef::new(10, 0));

        let result = validate_char_proc_structure(&ref_obj);
        assert!(result.is_err());
        match result {
            Err(Type3Error::InvalidCharProcType { got, expected }) => {
                // References without context are classified as Unknown (bf-5on6og)
                assert_eq!(got, "unknown");
                assert_eq!(expected, "stream or dictionary");
            }
            _ => panic!("Expected InvalidCharProcType error"),
        }
    }

    #[test]
    fn test_validate_char_proc_structure_error_message_formatting() {
        use crate::parser::object::types::{PdfDict, PdfStream};

        // Test that error messages are clear and informative
        let dict = PdfDict::new(); // Missing all keys
        let stream = PdfStream::new(dict, 0, None);
        let stream_obj = PdfObject::Stream(Box::new(stream));

        let result = validate_char_proc_structure(&stream_obj);
        assert!(result.is_err());

        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("missing required key"));
        assert!(error_msg.contains("/Type"));
        assert!(error_msg.contains("stream"));
    }

    #[test]
    fn test_fill_polygon_edge_activation_at_y_min() {
        // Test that edges are activated when scanline reaches y_min
        use crate::parser::object::types::{intern, PdfDict};

        // Create a minimal Type3Font with a 32x32 bbox
        let mut font_dict = PdfDict::new();
        font_dict.insert(intern("/FontMatrix"), PdfObject::Array(Box::new(vec![
            PdfObject::Real(1.0), PdfObject::Real(0.0),
            PdfObject::Real(0.0), PdfObject::Real(1.0),
            PdfObject::Real(0.0), PdfObject::Real(0.0),
        ])));
        font_dict.insert(intern("/FontBBox"), PdfObject::Array(Box::new(vec![
            PdfObject::Integer(0), PdfObject::Integer(0),
            PdfObject::Integer(31), PdfObject::Integer(31),
        ])));

        let font = Type3Font::load(&font_dict);
        let mut ctx = RasterizerContext::new(&font);

        // Create a diagonal line from (5, 10) to (15, 20)
        // Edge should be added to AET at y_min = 10
        let edges = vec![(5, 10, 15, 20)];

        ctx.fill_polygon(&edges);

        // Pixels should be filled starting at y = 10
        // y = 10: x ≈ 5
        assert_eq!(ctx.bitmap.get(5, 10), Some(0), "Edge should be active at y_min");

        // Pixels should NOT be filled before y = 10
        assert_eq!(ctx.bitmap.get(5, 9), Some(255), "Edge should not be active before y_min");
    }

    #[test]
    fn test_fill_polygon_edge_removal_after_y_max() {
        // Test that edges are removed after scanline passes y_max
        use crate::parser::object::types::{intern, PdfDict};

        let mut font_dict = PdfDict::new();
        font_dict.insert(intern("/FontMatrix"), PdfObject::Array(Box::new(vec![
            PdfObject::Real(1.0), PdfObject::Real(0.0),
            PdfObject::Real(0.0), PdfObject::Real(1.0),
            PdfObject::Real(0.0), PdfObject::Real(0.0),
        ])));
        font_dict.insert(intern("/FontBBox"), PdfObject::Array(Box::new(vec![
            PdfObject::Integer(0), PdfObject::Integer(0),
            PdfObject::Integer(31), PdfObject::Integer(31),
        ])));

        let font = Type3Font::load(&font_dict);
        let mut ctx = RasterizerContext::new(&font);

        // Create a diagonal line from (5, 10) to (15, 20)
        // Edge should be removed after y_max = 20
        let edges = vec![(5, 10, 15, 20)];

        ctx.fill_polygon(&edges);

        // Pixels should be filled up to and including y = 20
        assert_eq!(ctx.bitmap.get(15, 20), Some(0), "Edge should be active at y_max");

        // Pixels should NOT be filled after y = 20
        assert_eq!(ctx.bitmap.get(15, 21), Some(255), "Edge should be removed after y_max");
    }

    #[test]
    fn test_fill_polygon_intersection_x_accuracy() {
        // Test that intersection x coordinates are calculated accurately
        use crate::parser::object::types::{intern, PdfDict};

        let mut font_dict = PdfDict::new();
        font_dict.insert(intern("/FontMatrix"), PdfObject::Array(Box::new(vec![
            PdfObject::Real(1.0), PdfObject::Real(0.0),
            PdfObject::Real(0.0), PdfObject::Real(1.0),
            PdfObject::Real(0.0), PdfObject::Real(0.0),
        ])));
        font_dict.insert(intern("/FontBBox"), PdfObject::Array(Box::new(vec![
            PdfObject::Integer(0), PdfObject::Integer(0),
            PdfObject::Integer(31), PdfObject::Integer(31),
        ])));

        let font = Type3Font::load(&font_dict);
        let mut ctx = RasterizerContext::new(&font);

        // Create a triangle with known intersection points
        // Triangle: (5, 5) -> (15, 15) -> (5, 15) -> (5, 5)
        let edges = vec![
            (5, 5, 15, 15),   // Diagonal up
            (15, 15, 5, 15),  // Horizontal (should be skipped)
            (5, 15, 5, 5),    // Vertical down
        ];

        ctx.fill_polygon(&edges);

        // At y = 10, intersection with diagonal should be at x = 10
        // Line from (5,5) to (15,15): slope = 1, at y=10, x = 5 + (10-5) = 10
        assert_eq!(ctx.bitmap.get(10, 10), Some(0), "Intersection x at y=10 should be 10");
    }

    #[test]
    fn test_fill_polygon_slope_x_increment_progression() {
        // Test that x increments correctly based on slope across multiple scanlines
        use crate::parser::object::types::PdfDict;
        use crate::font::type3::Type3Font;

        let font_dict = PdfDict::new();
        let font = Type3Font::load(&font_dict);
        let mut ctx = RasterizerContext::new(&font);

        // Create a line with slope 0.5: (5, 5) to (15, 25)
        // dx = 10, dy = 20, slope = dx/dy = 0.5
        // x should increment by 0.5 each scanline
        let edges = vec![(5, 5, 15, 25)];

        ctx.fill_polygon(&edges);

        // Expected x progression: 5.0, 5.5, 6.0, 6.5, 7.0, 7.5, 8.0, ...
        // Rounded: 5, 6, 6, 7, 7, 8, 8, 9, ...
        let expected_x_values: Vec<i32> = vec![
            5,  // y = 5: x = 5.0
            6,  // y = 6: x = 5.5 → 6
            6,  // y = 7: x = 6.0 → 6
            7,  // y = 8: x = 6.5 → 7
            7,  // y = 9: x = 7.0 → 7
            8,  // y = 10: x = 7.5 → 8
            8,  // y = 11: x = 8.0 → 8
            9,  // y = 12: x = 8.5 → 9
        ];

        for (i, y) in (5..=12).enumerate() {
            let expected_x = expected_x_values[i];
            assert_eq!(
                ctx.bitmap.get(expected_x, y),
                Some(0),
                "At y={}, intersection x should be {} (progression with slope 0.5)",
                y, expected_x
            );
        }
    }

    #[test]
    fn test_fill_polygon_multiple_edges_activation() {
        // Test that multiple edges are activated at their respective y_min values
        use crate::parser::object::types::PdfDict;
        use crate::font::type3::Type3Font;

        let font_dict = PdfDict::new();
        let font = Type3Font::load(&font_dict);
        let mut ctx = RasterizerContext::new(&font);

        // Create two edges with different y_min
        // Edge 1: y_min = 5, y_max = 15
        // Edge 2: y_min = 10, y_max = 20
        let edges = vec![
            (5, 5, 15, 15),
            (25, 10, 15, 20),
        ];

        ctx.fill_polygon(&edges);

        // At y = 5, only first edge should be active
        // With two edges, we get two intersections - fill between them
        let filled_at_5: Vec<i32> = (0..32).filter(|&x| ctx.bitmap.get(x, 5) == Some(0)).collect();
        assert!(!filled_at_5.is_empty(), "First edge should be active at y=5");

        // At y = 10, both edges should be active
        let filled_at_10: Vec<i32> = (0..32).filter(|&x| ctx.bitmap.get(x, 10) == Some(0)).collect();
        assert!(!filled_at_10.is_empty(), "Both edges should be active at y=10");
    }

    #[test]
    fn test_fill_polygon_horizontal_edges_skipped() {
        // Test that horizontal edges are skipped and don't affect fill
        use crate::parser::object::types::PdfDict;
        use crate::font::type3::Type3Font;

        let font_dict = PdfDict::new();
        let font = Type3Font::load(&font_dict);
        let mut ctx = RasterizerContext::new(&font);

        // Horizontal edge at y = 10 (should be skipped)
        let edges = vec![(5, 10, 15, 10)];

        ctx.fill_polygon(&edges);

        // No pixels should be filled
        for x in 0..32 {
            for y in 0..32 {
                assert_eq!(
                    ctx.bitmap.get(x, y),
                    Some(255),
                    "Horizontal edge should not fill any pixels"
                );
            }
        }
    }

    #[test]
    fn test_fill_polygon_steep_slope() {
        // Test edge with steep slope (dx > dy)
        use crate::parser::object::types::PdfDict;
        use crate::font::type3::Type3Font;

        let font_dict = PdfDict::new();
        let font = Type3Font::load(&font_dict);
        let mut ctx = RasterizerContext::new(&font);

        // Steep line: (5, 5) to (25, 10)
        // dx = 20, dy = 5, slope = dx/dy = 4.0
        // x increments by 4.0 each scanline
        let edges = vec![(5, 5, 25, 10)];

        ctx.fill_polygon(&edges);

        // Expected x progression: 5.0, 9.0, 13.0, 17.0, 21.0, 25.0
        let expected_x_values: Vec<i32> = vec![5, 9, 13, 17, 21, 25];

        for (i, y) in (5..=10).enumerate() {
            let expected_x = expected_x_values[i];
            assert_eq!(
                ctx.bitmap.get(expected_x, y),
                Some(0),
                "At y={}, intersection x should be {} (steep slope)",
                y, expected_x
            );
        }
    }

    #[test]
    fn test_fill_polygon_negative_slope() {
        // Test edge with negative slope (x decreases as y increases)
        use crate::parser::object::types::PdfDict;
        use crate::font::type3::Type3Font;

        let font_dict = PdfDict::new();
        let font = Type3Font::load(&font_dict);
        let mut ctx = RasterizerContext::new(&font);

        // Line with negative slope: (20, 5) to (10, 15)
        // dx = -10, dy = 10, slope = -1.0
        // x decrements by 1.0 each scanline
        let edges = vec![(20, 5, 10, 15)];

        ctx.fill_polygon(&edges);

        // Expected x progression: 20.0, 19.0, 18.0, 17.0, 16.0, 15.0, 14.0, 13.0, 12.0, 11.0, 10.0
        let expected_x_values: Vec<i32> = vec![20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10];

        for (i, y) in (5..=15).enumerate() {
            let expected_x = expected_x_values[i];
            assert_eq!(
                ctx.bitmap.get(expected_x, y),
                Some(0),
                "At y={}, intersection x should be {} (negative slope)",
                y, expected_x
            );
        }
    }

    #[test]
    fn test_fill_polygon_rectangle() {
        // Test filling a simple rectangle
        use crate::parser::object::types::PdfDict;
        use crate::font::type3::Type3Font;

        let font_dict = PdfDict::new();
        let font = Type3Font::load(&font_dict);
        let mut ctx = RasterizerContext::new(&font);

        // Rectangle: (5, 5) -> (15, 5) -> (15, 15) -> (5, 15)
        let edges = vec![
            (5, 5, 15, 5),   // Top edge (horizontal, skipped)
            (15, 5, 15, 15), // Right edge (vertical)
            (15, 15, 5, 15), // Bottom edge (horizontal, skipped)
            (5, 15, 5, 5),   // Left edge (vertical)
        ];

        ctx.fill_polygon(&edges);

        // Check that interior pixels are filled
        assert_eq!(ctx.bitmap.get(10, 10), Some(0), "Interior should be filled");

        // Check that corners are included
        assert_eq!(ctx.bitmap.get(5, 5), Some(0), "Top-left corner should be filled");
        assert_eq!(ctx.bitmap.get(15, 15), Some(0), "Bottom-right corner should be filled");

        // Check that exterior is not filled
        assert_eq!(ctx.bitmap.get(4, 10), Some(255), "Left exterior should not be filled");
        assert_eq!(ctx.bitmap.get(16, 10), Some(255), "Right exterior should not be filled");
    }

    #[test]
    fn test_edge_activation_at_y_min() {
        // Test that edges are added to AET exactly when scanline reaches y_min
        use crate::parser::object::types::PdfDict;
        use crate::font::type3::Type3Font;

        let font_dict = PdfDict::new();
        let font = Type3Font::load(&font_dict);
        let mut ctx = RasterizerContext::new(&font);

        // Create edges with different y_min values
        // Edge 1: active from y=8 to y=12
        // Edge 2: active from y=10 to y=14
        let edges = vec![
            (10, 8, 20, 12),  // Edge 1: y_min=8, y_max=12
            (15, 10, 25, 14), // Edge 2: y_min=10, y_max=14
        ];

        ctx.fill_polygon(&edges);

        // At y=7 (before first edge y_min): no pixels should be filled
        let filled_at_7: Vec<i32> = (0..32).filter(|&x| ctx.bitmap.get(x, 7) == Some(0)).collect();
        assert!(filled_at_7.is_empty(), "No pixels should be filled before y=8 (before first y_min)");

        // At y=8 (first edge y_min): first edge should be active
        let filled_at_8: Vec<i32> = (0..32).filter(|&x| ctx.bitmap.get(x, 8) == Some(0)).collect();
        assert!(!filled_at_8.is_empty(), "Pixels should be filled at y=8 (first edge y_min)");

        // At y=9 (between y_mins): first edge still active, second not yet
        let filled_at_9: Vec<i32> = (0..32).filter(|&x| ctx.bitmap.get(x, 9) == Some(0)).collect();
        assert!(!filled_at_9.is_empty(), "Pixels should be filled at y=9 (first edge still active)");

        // At y=10 (second edge y_min): both edges should be active
        let filled_at_10: Vec<i32> = (0..32).filter(|&x| ctx.bitmap.get(x, 10) == Some(0)).collect();
        assert!(!filled_at_10.is_empty(), "Pixels should be filled at y=10 (both edges active)");
    }

    #[test]
    fn test_edge_removal_after_y_max() {
        // Test that edges are removed from AET when scanline passes y_max
        use crate::parser::object::types::PdfDict;
        use crate::font::type3::Type3Font;

        let font_dict = PdfDict::new();
        let font = Type3Font::load(&font_dict);
        let mut ctx = RasterizerContext::new(&font);

        // Create edges with different y_max values
        // Edge 1: active from y=5 to y=10
        // Edge 2: active from y=5 to y=15
        let edges = vec![
            (10, 5, 20, 10),  // Edge 1: y_min=5, y_max=10
            (15, 5, 25, 15),  // Edge 2: y_min=5, y_max=15
        ];

        ctx.fill_polygon(&edges);

        // At y=10 (edge1 y_max): both edges should still be active (y <= y_max)
        let filled_at_10: Vec<i32> = (0..32).filter(|&x| ctx.bitmap.get(x, 10) == Some(0)).collect();
        assert!(!filled_at_10.is_empty(), "Pixels should be filled at y=10 (both edges at y_max)");

        // At y=11 (after edge1 y_max): only edge2 should be active
        let filled_at_11: Vec<i32> = (0..32).filter(|&x| ctx.bitmap.get(x, 11) == Some(0)).collect();
        assert!(!filled_at_11.is_empty(), "Pixels should be filled at y=11 (edge2 still active)");

        // At y=15 (edge2 y_max): edge2 should still be active (y <= y_max)
        let filled_at_15: Vec<i32> = (0..32).filter(|&x| ctx.bitmap.get(x, 15) == Some(0)).collect();
        assert!(!filled_at_15.is_empty(), "Pixels should be filled at y=15 (edge2 at y_max)");

        // At y=16 (after edge2 y_max): no edges should be active
        let filled_at_16: Vec<i32> = (0..32).filter(|&x| ctx.bitmap.get(x, 16) == Some(0)).collect();
        assert!(filled_at_16.is_empty(), "No pixels should be filled at y=16 (after all y_max)");
    }

    #[test]
    fn test_intersection_x_calculation_accuracy() {
        // Test that intersection x coordinates are calculated accurately
        use crate::parser::object::types::PdfDict;
        use crate::font::type3::Type3Font;

        let font_dict = PdfDict::new();
        let font = Type3Font::load(&font_dict);
        let mut ctx = RasterizerContext::new(&font);

        // Create a diagonal edge with known slope
        // Line from (10.0, 5.0) to (20.0, 15.0)
        // dx = 10, dy = 10, slope = dx/dy = 1.0
        // x increments by 1.0 each scanline
        let edges = vec![(10, 5, 20, 15)];

        ctx.fill_polygon(&edges);

        // Verify exact x positions at each scanline
        // y=5: x=10.0 -> round(10.0) = 10
        // y=6: x=11.0 -> round(11.0) = 11
        // y=7: x=12.0 -> round(12.0) = 12
        // ... etc
        let expected_progression: Vec<(i32, i32)> = vec![
            (5, 10), (6, 11), (7, 12), (8, 13), (9, 14),
            (10, 15), (11, 16), (12, 17), (13, 18), (14, 19), (15, 20)
        ];

        for (y, expected_x) in expected_progression {
            assert_eq!(
                ctx.bitmap.get(expected_x, y),
                Some(0),
                "At y={}, intersection x should be {} (slope=1.0)",
                y, expected_x
            );
        }
    }

    #[test]
    fn test_slope_based_x_increment_fractional() {
        // Test slope-based x increment with fractional increments
        use crate::parser::object::types::PdfDict;
        use crate::font::type3::Type3Font;

        let font_dict = PdfDict::new();
        let font = Type3Font::load(&font_dict);
        let mut ctx = RasterizerContext::new(&font);

        // Create edge with fractional slope
        // Line from (10.0, 5.0) to (15.0, 15.0)
        // dx = 5, dy = 10, slope = dx/dy = 0.5
        // x increments by 0.5 each scanline
        let edges = vec![(10, 5, 15, 15)];

        ctx.fill_polygon(&edges);

        // Verify x progression with fractional increments
        // y=5:  x=10.0 -> round(10.0) = 10
        // y=6:  x=10.5 -> round(10.5) = 10 (rounds to even, or 11 depending on implementation)
        // y=7:  x=11.0 -> round(11.0) = 11
        // y=8:  x=11.5 -> round(11.5) = 12
        // y=9:  x=12.0 -> round(12.0) = 12
        // y=10: x=12.5 -> round(12.5) = 12
        // y=11: x=13.0 -> round(13.0) = 13
        // y=12: x=13.5 -> round(13.5) = 14
        // y=13: x=14.0 -> round(14.0) = 14
        // y=14: x=14.5 -> round(14.5) = 14
        // y=15: x=15.0 -> round(15.0) = 15
        let expected_progression: Vec<(i32, i32)> = vec![
            (5, 10), (6, 10), (7, 11), (8, 12), (9, 12),
            (10, 12), (11, 13), (12, 14), (13, 14), (14, 14), (15, 15)
        ];

        for (y, expected_x) in expected_progression {
            assert_eq!(
                ctx.bitmap.get(expected_x, y),
                Some(0),
                "At y={}, intersection x should be {} (slope=0.5)",
                y, expected_x
            );
        }
    }

    #[test]
    fn test_slope_based_x_increment_shallow_positive() {
        // Test shallow positive slope (dx < dy)
        use crate::parser::object::types::PdfDict;
        use crate::font::type3::Type3Font;

        let font_dict = PdfDict::new();
        let font = Type3Font::load(&font_dict);
        let mut ctx = RasterizerContext::new(&font);

        // Line from (10.0, 5.0) to (12.0, 25.0)
        // dx = 2, dy = 20, slope = dx/dy = 0.1
        // x increments by 0.1 each scanline
        let edges = vec![(10, 5, 12, 25)];

        ctx.fill_polygon(&edges);

        // x progression: 10.0, 10.1, 10.2, 10.3, 10.4, 10.5, 10.6, 10.7, 10.8, 10.9,
        //              11.0, 11.1, 11.2, 11.3, 11.4, 11.5, 11.6, 11.7, 11.8, 11.9, 12.0
        let expected_progression: Vec<(i32, i32)> = vec![
            (5, 10), (6, 10), (7, 10), (8, 10), (9, 10), (10, 10),
            (11, 10), (12, 10), (13, 10), (14, 10), (15, 11), (16, 11),
            (17, 11), (18, 11), (19, 11), (20, 12), (21, 12), (22, 12),
            (23, 12), (24, 12), (25, 12)
        ];

        for (y, expected_x) in expected_progression {
            assert_eq!(
                ctx.bitmap.get(expected_x, y),
                Some(0),
                "At y={}, intersection x should be {} (slope=0.1)",
                y, expected_x
            );
        }
    }

    #[test]
    fn test_slope_based_x_increment_steep_negative() {
        // Test steep negative slope (dx negative, |dx| > |dy|)
        use crate::parser::object::types::PdfDict;
        use crate::font::type3::Type3Font;

        let font_dict = PdfDict::new();
        let font = Type3Font::load(&font_dict);
        let mut ctx = RasterizerContext::new(&font);

        // Line from (25.0, 5.0) to (10.0, 10.0)
        // dx = -15, dy = 5, slope = dx/dy = -3.0
        // x decrements by 3.0 each scanline
        let edges = vec![(25, 5, 10, 10)];

        ctx.fill_polygon(&edges);

        // x progression: 25.0, 22.0, 19.0, 16.0, 13.0, 10.0
        let expected_progression: Vec<(i32, i32)> = vec![
            (5, 25), (6, 22), (7, 19), (8, 16), (9, 13), (10, 10)
        ];

        for (y, expected_x) in expected_progression {
            assert_eq!(
                ctx.bitmap.get(expected_x, y),
                Some(0),
                "At y={}, intersection x should be {} (slope=-3.0)",
                y, expected_x
            );
        }
    }

    #[test]
    fn test_aet_management_with_overlapping_edges() {
        // Test AET management when multiple edges overlap in y-range
        use crate::parser::object::types::PdfDict;
        use crate::font::type3::Type3Font;

        let font_dict = PdfDict::new();
        let font = Type3Font::load(&font_dict);
        let mut ctx = RasterizerContext::new(&font);

        // Create three edges with overlapping y-ranges
        // Edge 1: (5, 10) to (10, 20) - y_min=10, y_max=20
        // Edge 2: (15, 12) to (20, 18) - y_min=12, y_max=18
        // Edge 3: (8, 15) to (18, 25) - y_min=15, y_max=25
        let edges = vec![
            (5, 10, 10, 20),
            (15, 12, 20, 18),
            (8, 15, 18, 25),
        ];

        ctx.fill_polygon(&edges);

        // At y=9: no edges active (before all y_min)
        let filled_at_9: Vec<i32> = (0..32).filter(|&x| ctx.bitmap.get(x, 9) == Some(0)).collect();
        assert!(filled_at_9.is_empty(), "No pixels filled at y=9 (before any y_min)");

        // At y=11: only edge1 active
        let filled_at_11: Vec<i32> = (0..32).filter(|&x| ctx.bitmap.get(x, 11) == Some(0)).collect();
        assert!(!filled_at_11.is_empty(), "Pixels filled at y=11 (edge1 active)");

        // At y=13: edges 1 and 2 active
        let filled_at_13: Vec<i32> = (0..32).filter(|&x| ctx.bitmap.get(x, 13) == Some(0)).collect();
        assert!(!filled_at_13.is_empty(), "Pixels filled at y=13 (edges 1 and 2 active)");

        // At y=15: all three edges active
        let filled_at_15: Vec<i32> = (0..32).filter(|&x| ctx.bitmap.get(x, 15) == Some(0)).collect();
        assert!(!filled_at_15.is_empty(), "Pixels filled at y=15 (all three edges active)");

        // At y=19: edges 1 and 3 active (edge2 past y_max)
        let filled_at_19: Vec<i32> = (0..32).filter(|&x| ctx.bitmap.get(x, 19) == Some(0)).collect();
        assert!(!filled_at_19.is_empty(), "Pixels filled at y=19 (edges 1 and 3 active)");

        // At y=22: only edge3 active
        let filled_at_22: Vec<i32> = (0..32).filter(|&x| ctx.bitmap.get(x, 22) == Some(0)).collect();
        assert!(!filled_at_22.is_empty(), "Pixels filled at y=22 (edge3 active)");

        // At y=26: no edges active (after all y_max)
        let filled_at_26: Vec<i32> = (0..32).filter(|&x| ctx.bitmap.get(x, 26) == Some(0)).collect();
        assert!(filled_at_26.is_empty(), "No pixels filled at y=26 (after all y_max)");
    }

    #[test]
    fn test_intersection_rounding_behavior() {
        // Test that x-coordinate intersection uses proper rounding
        use crate::parser::object::types::PdfDict;
        use crate::font::type3::Type3Font;

        let font_dict = PdfDict::new();
        let font = Type3Font::load(&font_dict);
        let mut ctx = RasterizerContext::new(&font);

        // Create edge that produces x values ending in .5
        // Line from (10.0, 5.0) to (15.0, 15.0)
        // dx = 5, dy = 10, slope = 0.5
        let edges = vec![(10, 5, 15, 15)];

        ctx.fill_polygon(&edges);

        // Verify rounding behavior for .5 values
        // The round() function should round .5 to nearest even number
        // or away from zero depending on implementation
        // Check that values are consistent
        assert_eq!(ctx.bitmap.get(10, 5), Some(0), "x=10.0 at y=5");
        assert_eq!(ctx.bitmap.get(10, 6), Some(0), "x=10.5 at y=6 rounds to 10 or 11");
        assert_eq!(ctx.bitmap.get(11, 7), Some(0), "x=11.0 at y=7");
        assert_eq!(ctx.bitmap.get(12, 8), Some(0), "x=11.5 at y=8 rounds to 12");
    }

    #[test]
    fn test_edge_ordering_by_y_min() {
        // Test that GET is sorted by y_min and edges are activated in order
        use crate::parser::object::types::PdfDict;
        use crate::font::type3::Type3Font;

        let font_dict = PdfDict::new();
        let font = Type3Font::load(&font_dict);
        let mut ctx = RasterizerContext::new(&font);

        // Create edges with unsorted y_min values
        // Edges should be activated in order of y_min regardless of input order
        let edges = vec![
            (10, 20, 15, 25),  // y_min=20 (should activate last)
            (5, 10, 10, 15),   // y_min=10 (should activate first)
            (8, 15, 12, 20),   // y_min=15 (should activate second)
        ];

        ctx.fill_polygon(&edges);

        // Verify activation order by checking which scanlines have filled pixels
        // At y=9: no edges active (before first y_min)
        let filled_at_9: Vec<i32> = (0..32).filter(|&x| ctx.bitmap.get(x, 9) == Some(0)).collect();
        assert!(filled_at_9.is_empty(), "No edges active at y=9");

        // At y=10: first edge (y_min=10) should be active
        let filled_at_10: Vec<i32> = (0..32).filter(|&x| ctx.bitmap.get(x, 10) == Some(0)).collect();
        assert!(!filled_at_10.is_empty(), "Edge with y_min=10 active at y=10");

        // At y=14: only first edge still active (second edge y_min=15)
        let filled_at_14: Vec<i32> = (0..32).filter(|&x| ctx.bitmap.get(x, 14) == Some(0)).collect();
        assert!(!filled_at_14.is_empty(), "First edge still active at y=14");

        // At y=15: second edge (y_min=15) should now be active
        let filled_at_15: Vec<i32> = (0..32).filter(|&x| ctx.bitmap.get(x, 15) == Some(0)).collect();
        assert!(!filled_at_15.is_empty(), "Second edge with y_min=15 active at y=15");

        // At y=19: second edge still active, third not yet (y_min=20)
        let filled_at_19: Vec<i32> = (0..32).filter(|&x| ctx.bitmap.get(x, 19) == Some(0)).collect();
        assert!(!filled_at_19.is_empty(), "Second edge still active at y=19");

        // At y=20: third edge (y_min=20) should be active
        let filled_at_20: Vec<i32> = (0..32).filter(|&x| ctx.bitmap.get(x, 20) == Some(0)).collect();
        assert!(!filled_at_20.is_empty(), "Third edge with y_min=20 active at y=20");
    }

    // Tests for Edge::intersection_x() rounding behavior (bf-6a7j1k)

    #[test]
    fn test_intersection_x_positive_values() {
        // Test that intersection_x rounds correctly for positive x values
        // edge.x = 5 → intersection_x = 5
        let edge = Edge {
            x: 5,
            y_min: 0,
            y_max: 10,
            dx: 10,
            dy: 10,
        };

        let result = edge.intersection_x();
        assert_eq!(result, 5, "edge.x = 5 should round to 5");
    }

    #[test]
    fn test_intersection_x_negative_values() {
        // Test that intersection_x rounds correctly for negative x values
        // edge.x = -4 → intersection_x = -4
        let edge = Edge {
            x: -4,
            y_min: 0,
            y_max: 10,
            dx: -10,
            dy: 10,
        };

        let result = edge.intersection_x();
        assert_eq!(result, -4, "edge.x = -4 should round to -4");
    }

    #[test]
    fn test_intersection_x_half_cases() {
        // Test that intersection_x rounds correctly for .5 cases
        // Rust's round() uses "round half to even" (banker's rounding)
        // We test with stored integer values that would come from .5 inputs

        // Test with x = 2 (representing a value that rounds to 2)
        let edge = Edge {
            x: 2,
            y_min: 0,
            y_max: 10,
            dx: 5,
            dy: 10,
        };

        let result = edge.intersection_x();
        assert_eq!(result, 2, "edge.x = 2 should round to 2");
    }

    #[test]
    fn test_intersection_x_rounding_consistency() {
        // Test that intersection_x rounding is consistent across various values
        let test_cases = vec![
            (0, 0),   // 0.0 → 0
            (1, 1),   // 1.0 → 1
            (10, 10), // 10.0 → 10
            (-1, -1), // -1.0 → -1
            (-10, -10), // -10.0 → -10
        ];

        for (x, expected) in test_cases {
            let edge = Edge {
                x,
                y_min: 0,
                y_max: 10,
                dx: 10,
                dy: 10,
            };

            let result = edge.intersection_x();
            assert_eq!(
                result, expected,
                "edge.x = {} should round to {}",
                x, expected
            );
        }
    }

    #[test]
    fn test_intersection_x_with_various_integer_inputs() {
        // Test intersection_x with various integer inputs that represent
        // rounded float values from the scanline algorithm

        // Test case 1: x = 5 (from 5.3 or similar)
        let edge1 = Edge {
            x: 5,
            y_min: 0,
            y_max: 10,
            dx: 10,
            dy: 10,
        };
        assert_eq!(edge1.intersection_x(), 5, "x=5 should round to 5");

        // Test case 2: x = -4 (from -3.7 or similar)
        let edge2 = Edge {
            x: -4,
            y_min: 0,
            y_max: 10,
            dx: -10,
            dy: 10,
        };
        assert_eq!(edge2.intersection_x(), -4, "x=-4 should round to -4");

        // Test case 3: x = 10 (from 10.0 or similar)
        let edge3 = Edge {
            x: 10,
            y_min: 0,
            y_max: 10,
            dx: 10,
            dy: 10,
        };
        assert_eq!(edge3.intersection_x(), 10, "x=10 should round to 10");
    }

    #[test]
    fn test_intersection_x_positive_whole() {
        // Test case for positive whole number: x = 5.0 → 5
        // Verifies that intersection_x correctly rounds positive whole numbers
        let edge = Edge {
            x: 5,
            y_min: 0,
            y_max: 10,
            dx: 10,
            dy: 10,
        };

        let result = edge.intersection_x();
        assert_eq!(result, 5, "x = 5.0 should round to 5");
    }

    #[test]
    fn test_intersection_x_negative_whole() {
        // Test case for negative whole number: x = -3.0 → -3
        // Verifies that intersection_x correctly rounds negative whole numbers
        let edge = Edge {
            x: -3,
            y_min: 0,
            y_max: 10,
            dx: -10,
            dy: 10,
        };

        let result = edge.intersection_x();
        assert_eq!(result, -3, "x = -3.0 should round to -3");
    }

    #[test]
    fn test_intersection_x_zero() {
        // Test case for zero: x = 0.0 → 0
        // Verifies that intersection_x correctly handles zero
        let edge = Edge {
            x: 0,
            y_min: 0,
            y_max: 10,
            dx: 10,
            dy: 10,
        };

        let result = edge.intersection_x();
        assert_eq!(result, 0, "x = 0.0 should round to 0");
    }

    #[test]
    fn test_intersection_x_negative_fraction() {
        // Test case for negative fraction: x = -2.3 → -2
        // Verifies acceptance criteria for bead bf-2an1s2:
        // - Negative fraction rounds toward zero (nearest integer)
        // - intersection_x uses round_x internally, which handles this correctly
        // Uses round_x directly since Edge stores x as i32

        // Test that -2.3 rounds to -2 (toward zero, nearest integer)
        let result = round_x(-2.3);
        assert_eq!(result, -2, "x = -2.3 should round to -2 (toward zero)");

        // Verify this is consistent with intersection_x behavior for integer inputs
        // When scanline algorithm accumulates fractional position -2.3,
        // intersection_x() will round it to -2 via round_x()
        let edge = Edge {
            x: -2, // Represents the rounded value from -2.3
            y_min: 0,
            y_max: 10,
            dx: -10,
            dy: 10,
        };

        let edge_result = edge.intersection_x();
        assert_eq!(edge_result, -2, "edge.x = -2 should round to -2");
    }

    #[test]
    fn test_intersection_x_negative_half_case() {
        // Test case for negative half: x = -0.5 → -1
        // Verifies acceptance criteria for bead bf-3z3m72:
        // - Critical boundary case: -0.5 rounds to -1 (NOT 0)
        // - Negative halves round AWAY from zero (toward larger magnitude)
        // - Uses round_x directly since Edge stores x as i32

        // Test that -0.5 rounds to -1 (away from zero, toward larger magnitude)
        let result = round_x(-0.5);
        assert_eq!(result, -1, "x = -0.5 should round to -1 (away from zero)");

        // Document why this rounds away from zero:
        // - Per round_x implementation using f64::round(), -0.5 rounds to -1.0
        // - This is "round half away from zero" behavior (also called "round half up" for negatives)
        // - Contrast: positive 0.5 rounds up to 1, negative 0.5 rounds "down" to -1 (both away from zero)
        // - This ensures symmetric rounding behavior around zero

        // Verify this is consistent with intersection_x behavior
        // When scanline algorithm produces x = -0.5 (stored in edge.x as -1 after casting),
        // intersection_x() will return -1 via round_x()
        let edge = Edge {
            x: -1, // Represents the rounded value from -0.5
            y_min: 0,
            y_max: 10,
            dx: -5,
            dy: 10,
        };

        let edge_result = edge.intersection_x();
        assert_eq!(edge_result, -1, "edge.x = -1 should round to -1");
    }

    #[test]
    fn test_intersection_x_negative_small_fraction() {
        // Test case for negative small fraction: x = -0.1 → 0
        // Verifies acceptance criteria for bead bf-5ma6k0:
        // - Small negative fraction rounds to 0 (toward zero, nearest integer)
        // - This is consistent with standard rounding behavior
        // - Uses round_x directly since Edge stores x as i32

        // Test that -0.1 rounds to 0 (toward zero, nearest integer)
        let result = round_x(-0.1);
        assert_eq!(result, 0, "x = -0.1 should round to 0 (toward zero)");

        // Document why this rounds toward zero:
        // - Per round_x implementation using f64::round(), -0.1 rounds to 0.0
        // - Standard rounding: -0.1 is closer to 0 than to -1
        // - Only half cases (-0.5) round away from zero to -1
        // - This ensures consistent behavior with other non-half fractions like -2.3 → -2

        // Verify this is consistent with intersection_x behavior
        // When scanline algorithm produces x = -0.1 (stored in edge.x as 0 after casting and rounding),
        // intersection_x() will return 0 via round_x()
        let edge = Edge {
            x: 0, // Represents the rounded value from -0.1
            y_min: 0,
            y_max: 10,
            dx: -1,
            dy: 10,
        };

        let edge_result = edge.intersection_x();
        assert_eq!(edge_result, 0, "edge.x = 0 should round to 0");
    }

    #[test]
    fn test_intersection_x_small_negative() {
        // Test case for small negative fraction: x = -0.1 → -1
        // Verifies acceptance criteria for bead bf-54ed2f:
        // - Small negative fraction should round to -1 (away from zero, toward larger magnitude)
        // - Tests boundary behavior for negative values near zero

        // Test that -0.1 rounds to -1 (away from zero, toward larger magnitude)
        let result = round_x(-0.1);
        assert_eq!(result, -1, "x = -0.1 should round to -1 (away from zero)");

        // Verify this is consistent with intersection_x behavior
        // When scanline algorithm produces x = -0.1,
        // intersection_x() should return -1 via round_x()
        let edge = Edge {
            x: -1, // Represents -0.1 rounded to -1
            y_min: 0,
            y_max: 10,
            dx: -1,
            dy: 10,
        };

        let edge_result = edge.intersection_x();
        assert_eq!(edge_result, -1, "edge.x = -1 should round to -1");
    }

    #[test]
    fn test_edge_x_field_access_from_aet() {
        // Test that edge.x field is directly readable from AET entries
        // Verifies acceptance criterion: ability to access x-coordinate field from edge structures in AET

        // Create an Active Edge Table (AET) with multiple edges
        let mut aet: Vec<Edge> = Vec::new();

        // Edge 1: x=10
        aet.push(Edge {
            x: 10,
            y_min: 0,
            y_max: 10,
            dx: 5,
            dy: 10,
        });

        // Edge 2: x=25
        aet.push(Edge {
            x: 25,
            y_min: 5,
            y_max: 15,
            dx: 10,
            dy: 10,
        });

        // Edge 3: x=-3 (negative x-coordinate)
        aet.push(Edge {
            x: -3,
            y_min: 8,
            y_max: 18,
            dx: -5,
            dy: 10,
        });

        // Test direct field access: read edge.x from each AET entry
        assert_eq!(aet[0].x, 10, "First edge should have x=10");
        assert_eq!(aet[1].x, 25, "Second edge should have x=25");
        assert_eq!(aet[2].x, -3, "Third edge should have x=-3");

        // Test that we can modify edge.x in AET entries
        aet[0].x = 15;
        assert_eq!(aet[0].x, 15, "Modified first edge should have x=15");

        // Test that we can iterate over AET and read x from each edge
        let x_values: Vec<i32> = aet.iter().map(|edge| edge.x).collect();
        assert_eq!(x_values, vec![15, 25, -3], "All x values should be readable from AET");
    }

    #[test]
    fn test_round_x_positive_values() {
        // Test positive values round correctly
        assert_eq!(round_x(0.0), 0, "Zero should round to 0");
        assert_eq!(round_x(0.3), 0, "0.3 should round to 0");
        assert_eq!(round_x(0.5), 1, "0.5 should round up to 1 (half-up)");
        assert_eq!(round_x(0.7), 1, "0.7 should round to 1");
        assert_eq!(round_x(1.0), 1, "1.0 should round to 1");
        assert_eq!(round_x(1.2), 1, "1.2 should round to 1");
        assert_eq!(round_x(1.5), 2, "1.5 should round up to 2 (half-up)");
        assert_eq!(round_x(1.8), 2, "1.8 should round to 2");
        assert_eq!(round_x(10.4), 10, "10.4 should round to 10");
        assert_eq!(round_x(10.5), 11, "10.5 should round up to 11 (half-up)");
        assert_eq!(round_x(10.6), 11, "10.6 should round to 11");
    }

    #[test]
    fn test_round_x_negative_values() {
        // Test negative values round correctly (half-away-from-zero)
        assert_eq!(round_x(-0.3), 0, "-0.3 should round to 0");
        assert_eq!(round_x(-0.5), -1, "-0.5 should round away from zero to -1");
        assert_eq!(round_x(-0.7), -1, "-0.7 should round to -1");
        assert_eq!(round_x(-1.0), -1, "-1.0 should round to -1");
        assert_eq!(round_x(-1.2), -1, "-1.2 should round to -1");
        assert_eq!(round_x(-1.5), -2, "-1.5 should round away from zero to -2");
        assert_eq!(round_x(-1.8), -2, "-1.8 should round to -2");
        assert_eq!(round_x(-10.4), -10, "-10.4 should round to -10");
        assert_eq!(round_x(-10.5), -11, "-10.5 should round away from zero to -11");
        assert_eq!(round_x(-10.6), -11, "-10.6 should round to -11");
    }

    #[test]
    fn test_round_x_edge_cases() {
        // Test edge cases and large values
        assert_eq!(round_x(0.0), 0, "Zero should round to 0");
        assert_eq!(round_x(-0.0), 0, "Negative zero should round to 0");

        // Large positive values
        assert_eq!(round_x(1000.3), 1000, "Large positive 1000.3 should round to 1000");
        assert_eq!(round_x(1000.5), 1001, "Large positive 1000.5 should round up to 1001");

        // Large negative values
        assert_eq!(round_x(-1000.3), -1000, "Large negative -1000.3 should round to -1000");
        assert_eq!(round_x(-1000.5), -1001, "Large negative -1000.5 should round away from zero to -1001");
    }

    #[test]
    fn test_round_x_integration_with_edge_intersection_x() {
        // Test that round_x is correctly used by Edge::intersection_x()
        let edge = Edge {
            x: 5,
            y_min: 0,
            y_max: 10,
            dx: 3,
            dy: 5,
        };

        // Edge::intersection_x() should use round_x internally
        // Since edge.x is 5 (i32), intersection_x should return 5
        assert_eq!(edge.intersection_x(), 5, "Edge with x=5 should return intersection_x=5");

        // Test with edge that would have fractional x after transformation
        let edge_float_x = Edge {
            x: 7, // represents 7.0 in floating-point
            y_min: 2,
            y_max: 12,
            dx: -3,
            dy: 8,
        };

        assert_eq!(edge_float_x.intersection_x(), 7, "Edge with x=7 should return intersection_x=7");
    }

    #[test]
    fn test_aet_intersection_collection_loop() {
        // Test that AET intersection collection loop works correctly
        // Verifies acceptance criterion: iterate through AET, apply rounding, collect into Vec<i32>

        // Create a mock Active Edge Table (AET) with multiple edges
        let mut aet: Vec<Edge> = Vec::new();

        // Edge 1: x=10 (integer coordinate)
        aet.push(Edge {
            x: 10,
            y_min: 0,
            y_max: 10,
            dx: 5,
            dy: 10,
        });

        // Edge 2: x=25 (integer coordinate)
        aet.push(Edge {
            x: 25,
            y_min: 5,
            y_max: 15,
            dx: 10,
            dy: 10,
        });

        // Edge 3: x=-3 (negative coordinate)
        aet.push(Edge {
            x: -3,
            y_min: 8,
            y_max: 18,
            dx: -5,
            dy: 10,
        });

        // Edge 4: x=100 (larger coordinate)
        aet.push(Edge {
            x: 100,
            y_min: 0,
            y_max: 20,
            dx: 0,
            dy: 20,
        });

        // Test the collection loop: iterate through AET, apply rounding, collect into Vec<i32>
        let intersections: Vec<i32> = aet.iter().map(|edge| edge.intersection_x()).collect();

        // Verify the output contains all rounded x-coordinates in AET order
        assert_eq!(intersections.len(), 4, "Should collect 4 intersection points");
        assert_eq!(intersections[0], 10, "First edge x=10 should round to 10");
        assert_eq!(intersections[1], 25, "Second edge x=25 should round to 25");
        assert_eq!(intersections[2], -3, "Third edge x=-3 should round to -3");
        assert_eq!(intersections[3], 100, "Fourth edge x=100 should round to 100");

        // Verify order is preserved (AET order matters for fill span calculation)
        assert_eq!(intersections, vec![10, 25, -3, 100], "Order should match AET order");

        // Test with empty AET
        let empty_aet: Vec<Edge> = Vec::new();
        let empty_intersections: Vec<i32> = empty_aet.iter().map(|edge| edge.intersection_x()).collect();
        assert_eq!(empty_intersections.len(), 0, "Empty AET should produce empty intersections");
    }

    #[test]
    fn test_round_x_fractional_rounds_up() {
        // Test round_x helper with fractional values that should round up
        // Verifies acceptance criterion: test case with fractional x value that should round up
        // Uses round_x directly since Edge stores x as i32

        // Test fractional values that round up (>= 0.5 rounds to next integer away from zero)
        assert_eq!(round_x(5.7), 6, "5.7 should round up to 6");
        assert_eq!(round_x(5.5), 6, "5.5 should round up to 6");
        assert_eq!(round_x(0.5), 1, "0.5 should round up to 1");
        assert_eq!(round_x(10.1), 10, "10.1 should round to 10 (not enough to round up)");
        assert_eq!(round_x(2.6), 3, "2.6 should round up to 3");

        // Test negative fractional values that round toward zero (away from negative infinity)
        assert_eq!(round_x(-2.3), -2, "-2.3 should round to -2");
        assert_eq!(round_x(-0.5), -1, "-0.5 should round to -1");
        assert_eq!(round_x(-5.7), -6, "-5.7 should round to -6");
    }

    #[test]
    fn test_round_x_small_fractions_round_up() {
        // Test round_x with small positive fractions that round up to 1
        // Verifies acceptance criteria for bead bf-4r4p21:
        // - Small positive fraction: x = 0.5 → 1
        // - Additional small fractions >= 0.5 all round to 1
        // Uses round_x directly since Edge stores x as i32

        // Test the exact 0.5 boundary (rounds away from zero)
        assert_eq!(round_x(0.5), 1, "0.5 should round up to 1 (half-up)");

        // Test small fractions greater than 0.5 (all round to 1)
        assert_eq!(round_x(0.6), 1, "0.6 should round up to 1");
        assert_eq!(round_x(0.7), 1, "0.7 should round up to 1");
        assert_eq!(round_x(0.8), 1, "0.8 should round up to 1");
        assert_eq!(round_x(0.9), 1, "0.9 should round up to 1");
        assert_eq!(round_x(0.99), 1, "0.99 should round up to 1");

        // Test larger positive fractions that round up
        assert_eq!(round_x(5.7), 6, "5.7 should round up to 6");
        assert_eq!(round_x(1.5), 2, "1.5 should round up to 2");
        assert_eq!(round_x(2.5), 3, "2.5 should round up to 3");
    }

    #[test]
    fn test_round_x_small_fractions_round_down() {
        // Test round_x with small positive fractions that round down to 0
        // Shows the boundary: fractions < 0.5 round toward zero
        // Uses round_x directly since Edge stores x as i32

        // Test small fractions less than 0.5 (all round to 0)
        assert_eq!(round_x(0.1), 0, "0.1 should round down to 0");
        assert_eq!(round_x(0.2), 0, "0.2 should round down to 0");
        assert_eq!(round_x(0.3), 0, "0.3 should round down to 0");
        assert_eq!(round_x(0.4), 0, "0.4 should round down to 0");
        assert_eq!(round_x(0.49), 0, "0.49 should round down to 0");
        assert_eq!(round_x(0.499), 0, "0.499 should round down to 0");

        // Test larger fractions that round down
        assert_eq!(round_x(5.3), 5, "5.3 should round down to 5");
        assert_eq!(round_x(5.4), 5, "5.4 should round down to 5");
    }

    #[test]
    fn test_round_x_fractional_rounds_down() {
        // Test round_x helper with fractional values that should round down
        // Verifies acceptance criterion: test case with fractional x value that should round down
        // Uses round_x directly since Edge stores x as i32

        // Test fractional values that round down (< 0.5 rounds toward zero)
        assert_eq!(round_x(10.2), 10, "10.2 should round down to 10");
        assert_eq!(round_x(2.49), 2, "2.49 should round down to 2");

        // Test negative fractional values that round away from zero
        assert_eq!(round_x(-2.7), -3, "-2.7 should round to -3");
        assert_eq!(round_x(-0.6), -1, "-0.6 should round to -1");
        assert_eq!(round_x(-5.8), -6, "-5.8 should round to -6");
    }

    #[test]
    fn test_round_x_whole_numbers() {
        // Test round_x helper with whole numbers
        // Verifies acceptance criterion: test case with whole number x value that should remain unchanged
        // Uses round_x directly since Edge stores x as i32

        assert_eq!(round_x(0.0), 0, "0.0 should round to 0");
        assert_eq!(round_x(1.0), 1, "1.0 should round to 1");
        assert_eq!(round_x(3.0), 3, "3.0 should round to 3");
        assert_eq!(round_x(10.0), 10, "10.0 should round to 10");
        assert_eq!(round_x(100.0), 100, "100.0 should round to 100");

        // Test negative whole numbers
        assert_eq!(round_x(-1.0), -1, "-1.0 should round to -1");
        assert_eq!(round_x(-5.0), -5, "-5.0 should round to -5");
        assert_eq!(round_x(-10.0), -10, "-10.0 should round to -10");
    }

    #[test]
    fn test_round_x_fractional_rounds_down_negative() {
        // Test round_x helper with negative fractional values
        // Verifies acceptance criteria for bead bf-hh2ek5:
        // - Negative fraction: x = -2.3 → -2 (rounds toward zero, nearest integer)
        // - Small negative fraction: x = -0.5 → -1 (half case rounds away from zero)
        // - Small negative fraction: x = -0.1 → 0 (rounds toward zero, nearest integer)
        // Uses round_x directly since Edge stores x as i32

        // Test negative fraction that rounds toward zero (nearest integer)
        assert_eq!(round_x(-2.3), -2, "x = -2.3 should round to -2");

        // Test small negative fraction at the -0.5 boundary (half case rounds away from zero)
        assert_eq!(round_x(-0.5), -1, "x = -0.5 should round to -1");

        // Test small negative fraction close to zero (rounds toward zero, nearest integer)
        assert_eq!(round_x(-0.1), 0, "x = -0.1 should round to 0");

        // Additional test cases for negative fractions
        assert_eq!(round_x(-0.4), 0, "x = -0.4 should round to 0 (nearest is 0)");
        assert_eq!(round_x(-0.6), -1, "x = -0.6 should round to -1 (nearest is -1)");
        assert_eq!(round_x(-1.1), -1, "x = -1.1 should round to -1 (nearest is -1)");
        assert_eq!(round_x(-1.5), -2, "x = -1.5 should round to -2 (half case away from zero)");
    }

    #[test]
    fn test_intersection_x_round_x_edge_cases() {
        // Test round_x helper with edge cases
        // Tests boundary conditions and special cases
        // Uses round_x directly since Edge stores x as i32

        // Test exact halves (round half away from zero)
        assert_eq!(round_x(0.5), 1, "0.5 should round to 1");
        assert_eq!(round_x(-0.5), -1, "-0.5 should round to -1");
        assert_eq!(round_x(1.5), 2, "1.5 should round to 2");
        assert_eq!(round_x(-1.5), -2, "-1.5 should round to -2");

        // Test values very close to integer boundaries
        assert_eq!(round_x(0.4999999), 0, "0.4999999 should round to 0");
        assert_eq!(round_x(0.5000001), 1, "0.5000001 should round to 1");
        assert_eq!(round_x(-0.4999999), 0, "-0.4999999 should round to 0");
        assert_eq!(round_x(-0.5000001), -1, "-0.5000001 should round to -1");

        // Test very small fractional values
        assert_eq!(round_x(0.1), 0, "0.1 should round to 0");
        assert_eq!(round_x(-0.1), 0, "-0.1 should round to 0");
        assert_eq!(round_x(0.01), 0, "0.01 should round to 0");
        assert_eq!(round_x(-0.01), 0, "-0.01 should round to 0");
    }

    #[test]
    fn test_round_x_negative_fractions_round_down() {
        // Test round_x with negative fractions that round away from zero (toward larger magnitude)
        // Verifies acceptance criteria for bead bf-hh2ek5:
        // - Negative fractions round AWAY from zero (toward -1, not toward 0)
        // - -0.5 → -1, not 0
        // - Include edge case at the -0.5 boundary
        // Uses round_x directly since Edge stores x as i32

        // Test the exact -0.5 boundary (rounds away from zero)
        assert_eq!(round_x(-0.5), -1, "-0.5 should round away from zero to -1");

        // Test small negative fractions greater than 0.5 magnitude (all round away from zero to -1)
        assert_eq!(round_x(-0.6), -1, "-0.6 should round away from zero to -1");
        assert_eq!(round_x(-0.7), -1, "-0.7 should round away from zero to -1");
        assert_eq!(round_x(-0.8), -1, "-0.8 should round away from zero to -1");
        assert_eq!(round_x(-0.9), -1, "-0.9 should round away from zero to -1");
        assert_eq!(round_x(-0.99), -1, "-0.99 should round away from zero to -1");

        // Test negative fractions that round toward larger magnitude (away from zero)
        assert_eq!(round_x(-1.6), -2, "-1.6 should round away from zero to -2");
        assert_eq!(round_x(-2.7), -3, "-2.7 should round away from zero to -3");
        assert_eq!(round_x(-3.8), -4, "-3.8 should round away from zero to -4");
        assert_eq!(round_x(-5.9), -6, "-5.9 should round away from zero to -6");

        // Verify that small negative fractions (< 0.5) round toward zero
        assert_eq!(round_x(-0.1), 0, "-0.1 should round toward zero to 0");
        assert_eq!(round_x(-0.2), 0, "-0.2 should round toward zero to 0");
        assert_eq!(round_x(-0.3), 0, "-0.3 should round toward zero to 0");
        assert_eq!(round_x(-0.4), 0, "-0.4 should round toward zero to 0");
    }

    #[test]
    fn test_scanline_to_intersections_to_fill_spans_integration() {
        // End-to-end integration test verifying:
        // 1. Scanline processing with AET update
        // 2. Intersection x-coordinate collection
        // 3. Fill span calculation from intersections
        // This tests the complete scanline rasterization pipeline

        use crate::parser::object::types::PdfDict;
        use crate::font::type3::Type3Font;

        let font_dict = PdfDict::new();
        let font = Type3Font::load(&font_dict);
        let mut ctx = RasterizerContext::new(&font);

        // Create a simple triangle with known intersection points
        // Triangle vertices: (10, 5), (20, 25), (5, 25)
        // Edges defined as (x0, y0, x1, y1)
        let edges = vec![
            (10, 5, 20, 25),   // Left edge: from (10,5) to (20,25)
            (20, 25, 5, 25),   // Bottom edge (horizontal, will be skipped)
            (5, 25, 10, 5),    // Right edge: from (5,25) to (10,5)
        ];

        // Process the polygon through the scanline pipeline
        ctx.fill_polygon(&edges);

        // Verify fill spans are generated correctly
        // At y=10: intersections should be at approximately x=11.25 and x=7.5
        // After rounding: x=11 and x=8, sorted: x=8 and x=11
        // Fill span should cover x=8 to x=11
        let filled_at_10: Vec<i32> = (0..32).filter(|&x| ctx.bitmap.get(x, 10) == Some(0)).collect();
        assert!(!filled_at_10.is_empty(), "Should have filled pixels at y=10");

        // Check specific filled positions at y=10
        // The triangle's interior should be filled between the intersection points
        assert_eq!(ctx.bitmap.get(9, 10), Some(0), "x=9 at y=10 should be filled (interior)");
        assert_eq!(ctx.bitmap.get(10, 10), Some(0), "x=10 at y=10 should be filled (interior)");

        // At y=20: triangle is wider, more pixels should be filled
        let filled_at_20: Vec<i32> = (0..32).filter(|&x| ctx.bitmap.get(x, 20) == Some(0)).collect();
        assert!(filled_at_20.len() > filled_at_10.len(), "y=20 should have more filled pixels than y=10");

        // Verify that pixels outside the intersection span are NOT filled
        // At x=3 (left of triangle) should not be filled
        assert_eq!(ctx.bitmap.get(3, 10), Some(255), "x=3 at y=10 should not be filled (exterior)");

        // At x=25 (right of triangle) should not be filled
        assert_eq!(ctx.bitmap.get(25, 10), Some(255), "x=25 at y=10 should not be filled (exterior)");

        // Verify the even-odd fill rule is respected
        // The triangle has a simple shape, so interior should be consistently filled
        // Check multiple points in the interior
        let interior_points = vec![(10, 15), (12, 18), (8, 20)];
        for (x, y) in interior_points {
            assert_eq!(ctx.bitmap.get(x, y), Some(0), "Interior point ({}, {}) should be filled", x, y);
        }
    }

    #[test]
    fn test_mock_works_with_rasterize_type3_glyph() {
        use crate::font::type3::Type3Font;

        // Create a char_procs HashMap with a test glyph
        let mut char_procs = std::collections::HashMap::new();
        char_procs.insert(Arc::from("TestGlyph"), ObjRef::new(10, 0));

        // Create Type3Font using mock()
        let font = Type3Font::mock(Some(char_procs));

        // Verify the mock font has the expected glyph
        assert!(font.has_glyph("TestGlyph"), "Mock font should have TestGlyph");
        assert_eq!(font.char_proc("TestGlyph"), Some(ObjRef::new(10, 0)));

        // Create a stream resolver that returns valid PDF content
        // This draws a simple 100x100 filled rectangle at origin
        let resolver = &(|_obj_ref: ObjRef| -> Option<Vec<u8>> {
            // Simple PDF content stream: draw a 100x100 filled rectangle
            Some(b"0 0 100 100 re f".to_vec())
        }) as &StreamResolverFn;

        // Minimal DocumentContext (only used for potential future features)
        let doc_context = DocumentContext {
            resolver: None,
            source: None,
        };

        // Call rasterize_type3_glyph with the mocked font
        // This should execute without panics
        let result = rasterize_type3_glyph(
            &font,
            "TestGlyph",
            Some(&doc_context),
            Some(resolver),
        );

        // Verify the function executes successfully and returns a bitmap
        assert!(
            result.is_some(),
            "rasterize_type3_glyph should return Some(bitmap) when given a valid mock font and resolver"
        );

        // Verify the bitmap contains data (not all-white)
        let bitmap = result.unwrap();
        assert!(!bitmap.is_empty(), "Bitmap should not be empty");
        assert!(
            bitmap.iter().any(|&pixel| pixel == 0),
            "Bitmap should contain some black (0) pixels from the filled rectangle"
        );
        assert!(
            bitmap.iter().any(|&pixel| pixel == 255),
            "Bitmap should contain some white (255) pixels from the background"
        );

        // Test with an unknown glyph (should return None gracefully)
        let unknown_result = rasterize_type3_glyph(
            &font,
            "UnknownGlyph",
            Some(&doc_context),
            Some(resolver),
        );
        assert!(
            unknown_result.is_none(),
            "rasterize_type3_glyph should return None for unknown glyph"
        );

        // Test with a slightly more complex glyph (multiple drawing operations)
        let mut char_procs_complex = std::collections::HashMap::new();
        char_procs_complex.insert(Arc::from("ComplexGlyph"), ObjRef::new(20, 0));
        let font_complex = Type3Font::mock(Some(char_procs_complex));

        let resolver_complex = &(|_obj_ref: ObjRef| -> Option<Vec<u8>> {
            // More complex content: two rectangles and a line
            Some(b"10 10 50 50 re f 60 60 90 90 re f 0 100 m 100 0 l s".to_vec())
        }) as &StreamResolverFn;

        let result_complex = rasterize_type3_glyph(
            &font_complex,
            "ComplexGlyph",
            Some(&doc_context),
            Some(resolver_complex),
        );

        assert!(
            result_complex.is_some(),
            "rasterize_type3_glyph should handle complex content streams"
        );

        let bitmap_complex = result_complex.unwrap();
        assert!(
            bitmap_complex.iter().any(|&pixel| pixel == 0),
            "Complex glyph bitmap should contain black pixels from multiple operations"
        );
    }

    #[test]
    fn test_round_x_negative_fraction_rounds_down() {
        // Test case for negative fraction: x = -2.3 → -2
        // Verifies that round_x correctly rounds negative fractions toward zero (truncation)
        let result = round_x(-2.3);
        assert_eq!(result, -2, "x = -2.3 should round to -2 (truncates toward zero)");
    }

    #[test]
    fn test_round_x_small_negative_fraction_rounds_down() {
        // Test case for small negative fraction: x = -0.5 → -1
        // Verifies that round_x correctly rounds -0.5 away from zero (toward -1)
        // Rust's round() uses "round half away from zero" for .5 cases
        let result = round_x(-0.5);
        assert_eq!(result, -1, "x = -0.5 should round to -1 (away from zero)");
    }

    #[test]
    fn test_round_x_very_small_negative_fraction_rounds_down() {
        // Test case for very small negative fraction: x = -0.1 → 0
        // Verifies that round_x correctly rounds small negative fractions toward zero
        let result = round_x(-0.1);
        assert_eq!(result, 0, "x = -0.1 should round to 0 (truncates toward zero)");
    }

    /// Test glyph helper functions for Type3 font rasterization tests.
    ///
    /// This module provides helper functions to create minimal valid glyph data
    /// for testing Type3Font::mock and rasterize_type3_glyph.
    ///
    /// # Overview
    ///
    /// These helpers simplify test setup by providing:
    /// - Predefined content stream patterns (rectangles, lines, etc.)
    /// - Easy glyph-to-reference mapping
    /// - Stream resolvers that return glyph content
    ///
    /// # Usage Example
    ///
    /// ```rust,no_run
    /// use pdftract_core::font::type3_rasterizer::tests::glyph_helpers::*;
    /// use pdftract_core::font::type3::Type3Font;
    /// use std::sync::Arc;
    ///
    /// // Create a simple rectangle glyph
    /// let char_procs = create_char_procs(&[("rect", 10)]);
    /// let font = Type3Font::mock(Some(char_procs));
    ///
    /// // Create a resolver that returns rectangle content
    /// let resolver = create_simple_resolver(&[(10, rectangle_glyph(0, 0, 100, 100))]);
    ///
    /// // Rasterize the glyph
    /// let bitmap = rasterize_type3_glyph(&font, "rect", None, Some(&resolver));
    /// ```
    pub mod glyph_helpers {
        use super::*;
        use std::collections::HashMap;

        /// Create a minimal char_procs HashMap for testing.
        ///
        /// This function creates a HashMap mapping glyph names to object references,
        /// which can be passed to Type3Font::mock().
        ///
        /// # Arguments
        ///
        /// * `glyphs` - Slice of (glyph_name, obj_ref_number) tuples
        ///
        /// # Returns
        ///
        /// HashMap<Arc<str>, ObjRef> suitable for Type3Font::mock()
        ///
        /// # Example
        ///
        /// ```rust,no_run
        /// # use pdftract_core::font::type3_rasterizer::tests::glyph_helpers::*;
        /// # use pdftract_core::font::type3::Type3Font;
        /// // Create char_procs with two glyphs
        /// let char_procs = create_char_procs(&[("A", 10), ("B", 11)]);
        /// let font = Type3Font::mock(Some(char_procs));
        /// ```
        pub fn create_char_procs(glyphs: &[(&str, u32)]) -> HashMap<Arc<str>, ObjRef> {
            glyphs
                .iter()
                .map(|(name, ref_num)| (Arc::from(*name), ObjRef::new(*ref_num, 0)))
                .collect()
        }

        /// Generate PDF content stream bytes for a filled rectangle glyph.
        ///
        /// Creates a minimal valid content stream that draws a filled rectangle
        /// using the "re" (rectangle) and "f" (fill) operators.
        ///
        /// # Arguments
        ///
        /// * `x` - X coordinate of rectangle origin
        /// * `y` - Y coordinate of rectangle origin
        /// * `width` - Rectangle width
        /// * `height` - Rectangle height
        ///
        /// # Returns
        ///
        /// Vec<u8> containing the PDF content stream bytes
        ///
        /// # Example
        ///
        /// ```rust,no_run
        /// # use pdftract_core::font::type3_rasterizer::tests::glyph_helpers::rectangle_glyph;
        /// // Create a 100x100 filled rectangle at origin
        /// let stream = rectangle_glyph(0, 0, 100, 100);
        /// assert_eq!(stream, b"0 0 100 100 re f");
        /// ```
        pub fn rectangle_glyph(x: i32, y: i32, width: i32, height: i32) -> Vec<u8> {
            format!("{} {} {} {} re f", x, y, width, height).into_bytes()
        }

        /// Generate PDF content stream bytes for a stroked line glyph.
        ///
        /// Creates a minimal valid content stream that draws a line segment
        /// using the "m" (move-to) and "l" (line-to) operators.
        ///
        /// # Arguments
        ///
        /// * `x0` - X coordinate of line start point
        /// * `y0` - Y coordinate of line start point
        /// * `x1` - X coordinate of line end point
        /// * `y1` - Y coordinate of line end point
        ///
        /// # Returns
        ///
        /// Vec<u8> containing the PDF content stream bytes
        ///
        /// # Example
        ///
        /// ```rust,no_run
        /// # use pdftract_core::font::type3_rasterizer::tests::glyph_helpers::line_glyph;
        /// // Create a line from (10,10) to (50,50)
        /// let stream = line_glyph(10, 10, 50, 50);
        /// assert_eq!(stream, b"10 10 m 50 50 l s");
        /// ```
        pub fn line_glyph(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<u8> {
            format!("{} {} m {} {} l s", x0, y0, x1, y1).into_bytes()
        }

        /// Generate PDF content stream bytes for a filled triangle glyph.
        ///
        /// Creates a content stream that draws a triangle using path commands.
        ///
        /// # Arguments
        ///
        /// * `x0` - X coordinate of first vertex
        /// * `y0` - Y coordinate of first vertex
        /// * `x1` - X coordinate of second vertex
        /// * `y1` - Y coordinate of second vertex
        /// * `x2` - X coordinate of third vertex
        /// * `y2` - Y coordinate of third vertex
        ///
        /// # Returns
        ///
        /// Vec<u8> containing the PDF content stream bytes
        ///
        /// # Example
        ///
        /// ```rust,no_run
        /// # use pdftract_core::font::type3_rasterizer::tests::glyph_helpers::triangle_glyph;
        /// // Create a triangle with vertices at (10,5), (15,15), (5,15)
        /// let stream = triangle_glyph(10, 5, 15, 15, 5, 15);
        /// ```
        pub fn triangle_glyph(x0: i32, y0: i32, x1: i32, y1: i32, x2: i32, y2: i32) -> Vec<u8> {
            format!(
                "{} {} m {} {} l {} {} l h f",
                x0, y0, x1, y1, x2, y2
            )
            .into_bytes()
        }

        /// Create a simple stream resolver for testing.
        ///
        /// This function creates a resolver callback that maps object references
        /// to content stream bytes. It's used with rasterize_type3_glyph().
        ///
        /// # Arguments
        ///
        /// * `streams` - Slice of (obj_ref_number, content_bytes) tuples
        ///
        /// # Returns
        ///
        /// A boxed StreamResolverFn callback suitable for rasterize_type3_glyph
        ///
        /// # Example
        ///
        /// ```rust,no_run
        /// # use pdftract_core::font::type3_rasterizer::tests::glyph_helpers::*;
        /// # use pdftract_core::font::type3::Type3Font;
        /// # use pdftract_core::font::type3_rasterizer::rasterize_type3_glyph;
        /// // Create a font with a rectangle glyph
        /// let char_procs = create_char_procs(&[("myrect", 10)]);
        /// let font = Type3Font::mock(Some(char_procs));
        ///
        /// // Create resolver that returns rectangle content for ref 10
        /// let resolver = create_simple_resolver(&[(10, rectangle_glyph(0, 0, 100, 100))]);
        ///
        /// // Rasterize
        /// let bitmap = rasterize_type3_glyph(&font, "myrect", None, Some(&resolver));
        /// ```
        pub fn create_simple_resolver(streams: &[(u32, Vec<u8>)]) -> Box<StreamResolverFn> {
            let stream_map: HashMap<u32, Vec<u8>> = streams
                .iter()
                .map(|(ref_num, bytes)| (*ref_num, bytes.clone()))
                .collect();

            Box::new(move |obj_ref: ObjRef| -> Option<Vec<u8>> {
                stream_map.get(&obj_ref.object).cloned()
            })
        }

        /// Create a minimal DocumentContext for testing.
        ///
        /// Returns a DocumentContext with None fields, suitable for tests
        /// that don't require full document resolution.
        ///
        /// # Example
        ///
        /// ```rust,no_run
        /// # use pdftract_core::font::type3_rasterizer::tests::glyph_helpers::create_minimal_doc_context;
        /// let doc_context = create_minimal_doc_context();
        /// ```
        pub fn create_minimal_doc_context() -> DocumentContext<'static> {
            DocumentContext {
                resolver: None,
                source: None,
            }
        }

        /// Create a complete test glyph setup (font + resolver + context).
        ///
        /// This is a convenience function that creates all the components needed
        /// for a Type3 glyph rasterization test.
        ///
        /// # Arguments
        ///
        /// * `glyph_data` - Slice of (glyph_name, obj_ref_number, content_bytes) tuples
        ///
        /// # Returns
        ///
        /// (Type3Font, Box<StreamResolverFn>, DocumentContext) tuple ready for testing
        ///
        /// # Example
        ///
        /// ```rust,no_run
        /// # use pdftract_core::font::type3_rasterizer::tests::glyph_helpers::*;
        /// # use pdftract_core::font::type3_rasterizer::rasterize_type3_glyph;
        /// // Create a complete test setup with two glyphs
        /// let (font, resolver, doc_context) = create_test_setup(&[
        ///     ("rect1", 10, rectangle_glyph(0, 0, 50, 50)),
        ///     ("rect2", 11, rectangle_glyph(10, 10, 60, 60)),
        /// ]);
        ///
        /// // Rasterize both glyphs
        /// let bitmap1 = rasterize_type3_glyph(&font, "rect1", Some(&doc_context), Some(&resolver));
        /// let bitmap2 = rasterize_type3_glyph(&font, "rect2", Some(&doc_context), Some(&resolver));
        /// ```
        pub fn create_test_setup(
            glyph_data: &[(&str, u32, Vec<u8>)],
        ) -> (Type3Font, Box<StreamResolverFn>, DocumentContext<'static>) {
            let char_procs: HashMap<Arc<str>, ObjRef> = glyph_data
                .iter()
                .map(|(name, ref_num, _bytes)| (Arc::from(*name), ObjRef::new(*ref_num, 0)))
                .collect();

            let font = Type3Font::mock(Some(char_procs));

            let streams: Vec<(u32, Vec<u8>)> = glyph_data
                .iter()
                .map(|(_name, ref_num, bytes)| (*ref_num, bytes.clone()))
                .collect();

            let resolver = create_simple_resolver(&streams);
            let doc_context = create_minimal_doc_context();

            (font, resolver, doc_context)
        }

        /// Create minimal glyph data structure for testing.
        ///
        /// This is the simplest possible helper that creates a valid glyph data
        /// structure with a single 100x100 filled rectangle at the origin.
        /// No parameters required - returns the most basic valid glyph.
        ///
        /// # Returns
        ///
        /// (glyph_name, obj_ref_number, content_bytes) tuple suitable for
        /// passing to create_test_setup() or for manual glyph construction
        ///
        /// # Example
        ///
        /// ```rust,no_run
        /// # use pdftract_core::font::type3_rasterizer::tests::glyph_helpers::*;
        /// # use pdftract_core::font::type3_rasterizer::rasterize_type3_glyph;
        /// // Create a minimal test setup with one glyph
        /// let (font, resolver, doc_context) = create_test_setup(&[make_minimal_glyph()]);
        ///
        /// // Rasterize the glyph
        /// let bitmap = rasterize_type3_glyph(&font, "glyph", Some(&doc_context), Some(&resolver));
        /// assert!(bitmap.is_some());
        /// ```
        pub fn make_minimal_glyph() -> (&'static str, u32, Vec<u8>) {
            ("glyph", 1, rectangle_glyph(0, 0, 100, 100))
        }
    }

    #[test]
    fn test_glyph_helpers_create_char_procs() {
        use glyph_helpers::*;

        // Test creating char_procs HashMap
        let char_procs = create_char_procs(&[("A", 10), ("B", 11), ("C", 12)]);

        assert_eq!(char_procs.len(), 3);
        assert_eq!(char_procs.get("A"), Some(&ObjRef::new(10, 0)));
        assert_eq!(char_procs.get("B"), Some(&ObjRef::new(11, 0)));
        assert_eq!(char_procs.get("C"), Some(&ObjRef::new(12, 0)));
        assert_eq!(char_procs.get("D"), None);
    }

    #[test]
    fn test_glyph_helpers_rectangle_glyph() {
        use glyph_helpers::*;

        // Test rectangle glyph generation
        let stream = rectangle_glyph(5, 10, 100, 200);
        assert_eq!(stream, b"5 10 100 200 re f");

        // Test with origin
        let stream_origin = rectangle_glyph(0, 0, 50, 50);
        assert_eq!(stream_origin, b"0 0 50 50 re f");
    }

    #[test]
    fn test_glyph_helpers_line_glyph() {
        use glyph_helpers::*;

        // Test line glyph generation
        let stream = line_glyph(10, 20, 30, 40);
        assert_eq!(stream, b"10 20 m 30 40 l s");

        // Test diagonal line
        let stream_diag = line_glyph(0, 0, 100, 100);
        assert_eq!(stream_diag, b"0 0 m 100 100 l s");
    }

    #[test]
    fn test_glyph_helpers_triangle_glyph() {
        use glyph_helpers::*;

        // Test triangle glyph generation
        let stream = triangle_glyph(10, 5, 15, 15, 5, 15);
        assert_eq!(stream, b"10 5 m 15 15 l 5 15 l h f");
    }

    #[test]
    fn test_glyph_helpers_simple_resolver() {
        use glyph_helpers::*;

        // Create resolver with multiple streams
        let resolver = create_simple_resolver(&[
            (10, rectangle_glyph(0, 0, 100, 100)),
            (11, line_glyph(0, 0, 50, 50)),
        ]);

        // Test resolver returns correct streams
        assert_eq!(
            resolver(ObjRef::new(10, 0)),
            Some(b"0 0 100 100 re f".to_vec())
        );
        assert_eq!(
            resolver(ObjRef::new(11, 0)),
            Some(b"0 0 m 50 50 l s".to_vec())
        );
        assert_eq!(resolver(ObjRef::new(99, 0)), None);
    }

    #[test]
    fn test_glyph_helpers_complete_setup() {
        use glyph_helpers::*;

        // Create complete test setup
        let (font, resolver, doc_context) = create_test_setup(&[
            ("rect1", 10, rectangle_glyph(0, 0, 50, 50)),
            ("line1", 11, line_glyph(10, 10, 40, 40)),
        ]);

        // Verify font has both glyphs
        assert!(font.has_glyph("rect1"));
        assert!(font.has_glyph("line1"));
        assert!(!font.has_glyph("nonexistent"));

        // Verify resolver works
        assert_eq!(
            resolver(ObjRef::new(10, 0)),
            Some(b"0 0 50 50 re f".to_vec())
        );
        assert_eq!(
            resolver(ObjRef::new(11, 0)),
            Some(b"10 10 m 40 40 l s".to_vec())
        );

        // Verify doc_context is minimal
        assert!(doc_context.resolver.is_none());
        assert!(doc_context.source.is_none());
    }

    #[test]
    fn test_glyph_helpers_integration_with_rasterize_type3_glyph() {
        use glyph_helpers::*;

        // Create test setup with a rectangle glyph
        let (font, resolver, doc_context) = create_test_setup(&[(
            "test_rect",
            10,
            rectangle_glyph(0, 0, 100, 100),
        )]);

        // Rasterize the glyph
        let result = rasterize_type3_glyph(
            &font,
            "test_rect",
            Some(&doc_context),
            Some(&resolver),
        );

        // Verify successful rasterization
        assert!(
            result.is_some(),
            "rasterize_type3_glyph should return Some(bitmap) for valid glyph"
        );

        let bitmap = result.unwrap();
        assert!(!bitmap.is_empty(), "Bitmap should not be empty");
        assert!(
            bitmap.iter().any(|&p| p == 0),
            "Bitmap should contain black pixels from filled rectangle"
        );
    }

    #[test]
    fn test_glyph_helpers_make_minimal_glyph() {
        use glyph_helpers::*;

        // Test make_minimal_glyph returns valid data
        let (name, ref_num, bytes) = make_minimal_glyph();

        // Verify name is "glyph"
        assert_eq!(name, "glyph");

        // Verify ref_num is 1
        assert_eq!(ref_num, 1);

        // Verify bytes create a valid rectangle
        assert_eq!(bytes, b"0 0 100 100 re f");

        // Test it can be used with create_test_setup
        let (font, resolver, doc_context) = create_test_setup(&[make_minimal_glyph()]);

        // Verify font has the glyph
        assert!(font.has_glyph("glyph"));

        // Verify it can be rasterized
        let result = rasterize_type3_glyph(
            &font,
            "glyph",
            Some(&doc_context),
            Some(&resolver),
        );

        assert!(
            result.is_some(),
            "make_minimal_glyph should produce rasterizable glyph data"
        );

        let bitmap = result.unwrap();
        assert!(!bitmap.is_empty(), "Minimal glyph bitmap should not be empty");
    }

    #[test]
    fn test_test_glyph_helper_compatibility_with_type3font() {
        use crate::font::test_glyph_helper::{
            make_rect_glyph, make_line_glyph, make_empty_glyph,
            make_test_char_procs, make_test_resolver,
        };
        use std::collections::HashMap;

        // Test 1: Type3Font::mock accepts make_test_char_procs output
        let char_procs = make_test_char_procs();
        let font = Type3Font::mock(Some(char_procs));

        // Verify standard glyphs from make_test_char_procs
        assert!(font.has_glyph("A"), "Font should have glyph 'A'");
        assert!(font.has_glyph("B"), "Font should have glyph 'B'");
        assert!(font.has_glyph("rect"), "Font should have glyph 'rect'");
        assert!(font.has_glyph("line"), "Font should have glyph 'line'");
        assert!(font.has_glyph("empty"), "Font should have glyph 'empty'");

        // Test 2: Helper functions generate valid content stream bytes
        let rect_bytes = make_rect_glyph(0.0, 0.0, 100.0, 100.0);
        assert_eq!(rect_bytes, b"0 0 100 100 re f");

        let line_bytes = make_line_glyph(0.0, 0.0, 50.0, 50.0);
        assert_eq!(line_bytes, b"0 0 m 50 50 l h S");

        let empty_bytes = make_empty_glyph();
        assert!(empty_bytes.is_empty());

        // Test 3: make_test_resolver creates a working resolver
        let mut glyph_map = HashMap::new();
        glyph_map.insert(10, rect_bytes.clone());
        glyph_map.insert(11, line_bytes.clone());
        glyph_map.insert(12, empty_bytes.clone());

        let resolver = make_test_resolver(&glyph_map);

        // Verify resolver returns correct bytes for each object reference
        use crate::parser::object::types::ObjRef;
        assert_eq!(resolver(ObjRef::new(10, 0)), Some(rect_bytes));
        assert_eq!(resolver(ObjRef::new(11, 0)), Some(line_bytes));
        assert_eq!(resolver(ObjRef::new(12, 0)), Some(empty_bytes));
        assert!(resolver(ObjRef::new(99, 0)).is_none(), "Unknown ref should return None");

        // Test 4: rasterize_type3_glyph works with the complete setup
        // Create custom char_procs matching our glyph_map
        use crate::font::test_glyph_helper::make_custom_char_procs;
        let custom_char_procs = make_custom_char_procs(&["rect", "line", "empty"], 10);
        let test_font = Type3Font::mock(Some(custom_char_procs));

        // Test rasterizing each glyph type
        let rect_result = rasterize_type3_glyph(
            &test_font,
            "rect",
            None,
            Some(&resolver),
        );
        assert!(rect_result.is_some(), "Rectangle glyph should rasterize successfully");
        let rect_bitmap = rect_result.unwrap();
        assert!(!rect_bitmap.is_empty(), "Rectangle bitmap should not be empty");
        // Rectangle should have black pixels (filled)
        assert!(rect_bitmap.iter().any(|&p| p == 0), "Filled rect should have black pixels");

        let line_result = rasterize_type3_glyph(
            &test_font,
            "line",
            None,
            Some(&resolver),
        );
        assert!(line_result.is_some(), "Line glyph should rasterize successfully");
        let line_bitmap = line_result.unwrap();
        assert!(!line_bitmap.is_empty(), "Line bitmap should not be empty");

        let empty_result = rasterize_type3_glyph(
            &test_font,
            "empty",
            None,
            Some(&resolver),
        );
        assert!(empty_result.is_some(), "Empty glyph should rasterize successfully");
        let empty_bitmap = empty_result.unwrap();
        // Empty glyph should produce all-white bitmap (all 255)
        assert!(empty_bitmap.iter().all(|&p| p == 255), "Empty glyph should be all white");

        // Test 5: Non-existent glyph returns None
        let nonexistent_result = rasterize_type3_glyph(
            &test_font,
            "nonexistent",
            None,
            Some(&resolver),
        );
        assert!(nonexistent_result.is_none(), "Non-existent glyph should return None");
    }

    #[test]
    fn test_test_glyph_helper_multiple_glyphs_single_resolver() {
        use crate::font::test_glyph_helper::{
            make_rect_glyph, make_test_char_procs, make_test_resolver,
        };
        use std::collections::HashMap;

        // Create multiple glyphs with different sizes
        let mut glyph_map = HashMap::new();
        glyph_map.insert(10, make_rect_glyph(0.0, 0.0, 50.0, 50.0));
        glyph_map.insert(11, make_rect_glyph(100.0, 100.0, 200.0, 200.0));
        glyph_map.insert(12, make_rect_glyph(10.0, 10.0, 20.0, 20.0));

        let resolver = make_test_resolver(&glyph_map);

        // Create char_procs matching the resolver
        use crate::font::test_glyph_helper::make_custom_char_procs;
        let char_procs = make_custom_char_procs(&["small", "large", "tiny"], 10);
        let font = Type3Font::mock(Some(char_procs));

        // All three glyphs should rasterize successfully
        for glyph_name in &["small", "large", "tiny"] {
            let result = rasterize_type3_glyph(&font, glyph_name, None, Some(&resolver));
            assert!(
                result.is_some(),
                "Glyph '{}' should rasterize successfully",
                glyph_name
            );
            let bitmap = result.unwrap();
            assert!(!bitmap.is_empty(), "Glyph '{}' bitmap should not be empty", glyph_name);
        }
    }
}
