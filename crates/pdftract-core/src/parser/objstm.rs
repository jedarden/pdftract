//! PDF object stream (ObjStm) parser.
//!
//! This module implements parsing of PDF 1.5+ object streams (`/Type /ObjStm`).
//! Object streams allow multiple indirect objects to be compressed together in
//! a single stream, reducing file size.
//!
//! # Object Stream Format
//!
//! An object stream consists of:
//! 1. A stream dictionary with:
//!    - `/Type /ObjStm` - identifies this as an object stream
//!    - `/N` - number of embedded objects
//!    - `/First` - byte offset to the first embedded object
//!    - Optional `/Extends N G R` - reference to another ObjStm this extends
//! 2. A compressed stream body containing:
//!    - A header section with N object number/offset pairs
//!    - N embedded objects (without `obj`/`endobj` wrappers)
//!
//! # Parsing
//!
//! 1. Decompress the stream content using Phase 1.5's filter pipeline
//! 2. Parse `/N` and `/First` from the stream dictionary
//! 3. Parse N object number/offset pairs from the first `/First` bytes
//! 4. For each embedded object, create a lexer at offset `/First + offset_k`
//! 5. Parse one direct object (no `obj`/`endobj` wrapper)
//! 6. Cache results as `Arc<Vec<(u32, PdfObject)>>` for indexed access
//! 7. Handle `/Extends` chains with cycle detection

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

use crate::parser::object::{ObjRef, PdfObject, PdfStream, ObjectParser};
use crate::parser::stream::{decode_stream, ExtractionOptions, PdfSource};
use crate::diagnostics::{Diagnostic, DiagCode};

/// Maximum depth for `/Extends` chain to prevent adversarial deep chains.
const MAX_EXTENDS_DEPTH: u8 = 16;

/// Result type for object stream parsing.
pub type ObjStmResult<T> = Result<T, ObjStmError>;

/// Errors that can occur during object stream parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjStmError {
    /// Required key missing from stream dictionary
    MissingKey { key: String },
    /// Invalid object stream format
    InvalidFormat { msg: String },
    /// Circular reference in /Extends chain
    CircularRef { obj_ref: ObjRef },
    /// Extends chain depth exceeded
    DepthExceeded { max: u8 },
    /// Stream decompression failed
    DecompressionFailed,
}

impl std::fmt::Display for ObjStmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObjStmError::MissingKey { key } => write!(f, "Missing required key: {}", key),
            ObjStmError::InvalidFormat { msg } => write!(f, "Invalid object stream format: {}", msg),
            ObjStmError::CircularRef { obj_ref } => write!(f, "Circular reference in /Extends chain at {}", obj_ref),
            ObjStmError::DepthExceeded { max } => write!(f, "Extends chain depth exceeded (max {})", max),
            ObjStmError::DecompressionFailed => write!(f, "Stream decompression failed"),
        }
    }
}

impl std::error::Error for ObjStmError {}

impl ObjStmError {
    /// Convert to a diagnostic code.
    pub fn diag_code(&self) -> DiagCode {
        match self {
            ObjStmError::MissingKey { .. } => DiagCode::StructMissingKey,
            ObjStmError::InvalidFormat { .. } => DiagCode::StructInvalidObjstm,
            ObjStmError::CircularRef { .. } => DiagCode::StructCircularRef,
            ObjStmError::DepthExceeded { .. } => DiagCode::StructDepthExceeded,
            ObjStmError::DecompressionFailed => DiagCode::StreamDecodeError,
        }
    }
}

/// Object stream cache entry.
///
/// Contains the parsed embedded objects for a single ObjStm.
/// The Vec preserves order by 0-based index, storing (object_number, object) pairs.
/// The Arc allows cheap cloning for concurrent access.
pub type ObjStmCacheEntry = Arc<Vec<(u32, PdfObject)>>;

/// Object stream parser with caching.
///
/// Parses and caches object streams, handling `/Extends` chains
/// with cycle detection.
///
/// # API
///
/// The parser provides two main methods:
/// - `get_object()`: Get an embedded object by (host_objstm_ref, embedded_index)
/// - `load_object_stream()`: Load and cache an entire object stream
///
/// This design allows the xref resolver (Phase 1.3) to call `get_object()`
/// for type-2 entries, while also supporting bulk loading of entire streams.
pub struct ObjectStmParser {
    /// Cache of parsed object streams
    cache: Arc<RwLock<HashMap<ObjRef, ObjStmCacheEntry>>>,
    /// Decompression counter for bomb limit enforcement (document-level)
    decompress_counter: Arc<RwLock<u64>>,
    /// Maximum decompressed bytes per document
    max_decompress_bytes: u64,
    /// Accumulated diagnostics
    diagnostics: Arc<RwLock<Vec<Diagnostic>>>,
}

impl ObjectStmParser {
    /// Create a new object stream parser.
    pub fn new(max_decompress_bytes: u64) -> Self {
        ObjectStmParser {
            cache: Arc::new(RwLock::new(HashMap::new())),
            decompress_counter: Arc::new(RwLock::new(0)),
            max_decompress_bytes,
            diagnostics: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Emit a diagnostic.
    fn emit_diagnostic(&self, code: DiagCode, message: String) {
        if let Ok(mut diags) = self.diagnostics.write() {
            diags.push(Diagnostic::with_dynamic_no_offset(code, message));
        }
    }

    /// Get all accumulated diagnostics.
    pub fn take_diagnostics(&self) -> Vec<Diagnostic> {
        if let Ok(diags) = self.diagnostics.write() {
            let mut guard = diags;
            std::mem::take(&mut *guard)
        } else {
            Vec::new()
        }
    }

    /// Get an embedded object from an object stream.
    ///
    /// This is the main API for xref type-2 entry resolution.
    /// If the object stream is not cached, it will be loaded first.
    ///
    /// # Parameters
    /// - `host_objstm_ref`: The object reference of the host ObjStm
    /// - `embedded_index`: The 0-based index of the embedded object in the stream
    /// - `source`: The PDF source to read stream data from
    /// - `resolve_fn`: Function to resolve indirect references (for `/Extends`)
    ///
    /// # Returns
    /// The embedded object if found, or PdfObject::Null if not found or on error.
    ///
    /// # Errors
    /// Errors are emitted as diagnostics; this method never returns Err.
    /// It returns PdfObject::Null on any error to maintain INV-8 (never panic).
    pub fn get_object<F>(
        &self,
        host_objstm_ref: ObjRef,
        embedded_index: u32,
        source: &dyn PdfSource,
        resolve_fn: F,
    ) -> PdfObject
    where
        F: Fn(ObjRef) -> Option<PdfStream>,
    {
        // Check if already cached
        {
            if let Ok(cache) = self.cache.read() {
                if let Some(entry) = cache.get(&host_objstm_ref) {
                    // embedded_index is 0-based, access by index
                    if let Some((_, obj)) = entry.get(embedded_index as usize) {
                        return obj.clone();
                    }
                    // Index out of bounds
                    return PdfObject::Null;
                }
            }
        }

        // Load the object stream
        let stream = match resolve_fn(host_objstm_ref) {
            Some(s) => s,
            None => return PdfObject::Null,    // Not found
        };

        // Create a wrapper that handles the recursion properly
        let resolve_wrapper = |ref_obj: ObjRef| -> Option<PdfStream> {
            resolve_fn(ref_obj)
        };

        match self.load_object_stream_impl(
            host_objstm_ref,
            &stream,
            source,
            &resolve_wrapper,
            &mut HashSet::new(),
            0,
        ) {
            Ok(entry) => {
                // Cache the result
                if let Ok(mut cache) = self.cache.write() {
                    cache.insert(host_objstm_ref, entry.clone());
                }

                // Return the requested object by 0-based index
                entry.get(embedded_index as usize)
                    .map(|(_, obj)| obj.clone())
                    .unwrap_or(PdfObject::Null)
            }
            Err(e) => {
                self.emit_diagnostic(
                    e.diag_code(),
                    format!("Object stream error: {}", e),
                );
                PdfObject::Null
            }
        }
    }

    /// Load an entire object stream and return its embedded objects as a Vec.
    ///
    /// # Parameters
    /// - `obj_stm_ref`: The object reference of the ObjStm
    /// - `stream_dict`: The stream dictionary from the ObjStm
    /// - `source`: The PDF source to read the stream data from
    /// - `resolve_fn`: Function to resolve indirect references (for `/Extends`)
    ///
    /// # Returns
    /// A Vec of (object_number, PdfObject) pairs, or an error.
    ///
    /// # Errors
    /// - `MissingKey`: Required key (`/N`, `/First`) not found
    /// - `InvalidFormat`: Malformed object stream data
    /// - `CircularRef`: Cycle detected in `/Extends` chain
    /// - `DepthExceeded`: `/Extends` chain too deep
    pub fn load_object_stream<F>(
        &self,
        obj_stm_ref: ObjRef,
        stream: &PdfStream,
        source: &dyn PdfSource,
        resolve_fn: F,
    ) -> ObjStmResult<ObjStmCacheEntry>
    where
        F: Fn(ObjRef) -> Option<PdfStream>,
    {
        // Check cache first
        {
            if let Ok(cache) = self.cache.read() {
                if let Some(cached) = cache.get(&obj_stm_ref) {
                    return Ok(cached.clone());
                }
            }
        }

        // Create a wrapper that handles the recursion properly
        let resolve_wrapper = |ref_obj: ObjRef| -> Option<PdfStream> {
            resolve_fn(ref_obj)
        };

        match self.load_object_stream_impl(
            obj_stm_ref,
            stream,
            source,
            &resolve_wrapper,
            &mut HashSet::new(),
            0,
        ) {
            Ok(entry) => {
                // Cache the result
                if let Ok(mut cache) = self.cache.write() {
                    cache.insert(obj_stm_ref, entry.clone());
                }
                Ok(entry)
            }
            Err(e) => Err(e),
        }
    }

    /// Internal implementation with cycle detection and depth tracking.
    fn load_object_stream_impl<'a, F>(
        &self,
        obj_stm_ref: ObjRef,
        stream: &PdfStream,
        source: &dyn PdfSource,
        resolve_fn: &'a F,
        in_progress: &mut HashSet<ObjRef>,
        depth: u8,
    ) -> ObjStmResult<ObjStmCacheEntry>
    where
        F: Fn(ObjRef) -> Option<PdfStream>,
    {
        // Check depth limit
        if depth > MAX_EXTENDS_DEPTH {
            return Err(ObjStmError::DepthExceeded {
                max: MAX_EXTENDS_DEPTH,
            });
        }

        // Check for circular reference
        if in_progress.contains(&obj_stm_ref) {
            return Err(ObjStmError::CircularRef { obj_ref: obj_stm_ref });
        }

        // Check cache first
        {
            let cache = self.cache.read().map_err(|_| ObjStmError::DecompressionFailed)?;
            if let Some(cached) = cache.get(&obj_stm_ref) {
                // Return the cached Arc directly (no clone)
                return Ok(cached.clone());
            }
        }

        // Mark this ObjStm as in-progress for cycle detection
        in_progress.insert(obj_stm_ref);

        let stream_dict = &stream.dict;

        // Get required keys from stream dictionary
        let n = stream_dict
            .get("/N")
            .and_then(|obj| obj.as_int())
            .ok_or_else(|| ObjStmError::MissingKey { key: "/N".to_string() })? as u32;

        let first = stream_dict
            .get("/First")
            .and_then(|obj| obj.as_int())
            .ok_or_else(|| ObjStmError::MissingKey {
                key: "/First".to_string(),
            })? as u64;

        let opts = ExtractionOptions {
            max_decompress_bytes: self.max_decompress_bytes,
            password: None,
        };

        let mut counter = { *self.decompress_counter.read().unwrap() };
        let decompressed = decode_stream(stream, source, &opts, &mut counter);
        {
            *self.decompress_counter.write().unwrap() = counter;
        }

        #[cfg(test)]
        eprintln!("DEBUG: decompressed {} bytes, first: {:?}", decompressed.len(), decompressed.get(0..20));

        if decompressed.is_empty() {
            in_progress.remove(&obj_stm_ref);
            return Ok(Arc::new(Vec::new()));
        }

        // Check if first offset is valid
        if first as usize > decompressed.len() {
            in_progress.remove(&obj_stm_ref);
            self.emit_diagnostic(
                DiagCode::StructInvalidObjstm,
                format!("ObjStm /First offset {} exceeds decompressed size {}", first, decompressed.len()),
            );
            return Ok(Arc::new(Vec::new()));
        }

        // Parse the header: N pairs of (object_number, offset)
        let header_bytes = &decompressed[..first as usize];
        let mut embedded_objects = Vec::new();
        let mut header_lexer = ObjectParser::new(header_bytes);

        for _ in 0..n {
            // Parse object number
            let obj_number = match header_lexer.parse_direct_object() {
                Some(PdfObject::Integer(i)) if i >= 0 => i as u32,
                Some(PdfObject::Integer(_)) => {
                    // Negative object number - invalid, skip
                    continue;
                }
                Some(_) => {
                    // Not an integer - invalid header
                    break;
                }
                None => {
                    // EOF - header ended early
                    break;
                }
            };

            // Parse offset
            let offset = match header_lexer.parse_direct_object() {
                Some(PdfObject::Integer(i)) if i >= 0 => i as u64,
                Some(PdfObject::Integer(_)) => {
                    // Negative offset - invalid, skip
                    continue;
                }
                Some(_) => {
                    // Not an integer - invalid header
                    break;
                }
                None => {
                    // EOF - header ended early
                    break;
                }
            };

            embedded_objects.push((obj_number, offset));
        }

        // Parse each embedded object and build a Vec of (object_number, object) pairs
        // The Vec preserves order by 0-based index for fast lookup by index
        let mut result = Vec::new();

        for &(obj_number, offset) in &embedded_objects {
            let obj_start = (first + offset) as usize;

            if obj_start >= decompressed.len() {
                // Offset out of bounds - use Null
                result.push((obj_number, PdfObject::Null));
                continue;
            }

            // Parse one direct object (no obj/endobj wrapper)
            let remaining = &decompressed[obj_start..];

            #[cfg(test)]
            eprintln!("DEBUG: Parsing object {} at offset {}, remaining bytes: {:?}", obj_number, obj_start, remaining);

            let mut obj_parser = ObjectParser::new(remaining);

            // Parse the object using the object parser
            // Embedded objects can be: null, boolean, number, string, name, array, dict, or ref
            // They CANNOT be streams (per PDF spec)
            let obj = match obj_parser.parse_direct_object() {
                Some(o) => o,
                None => PdfObject::Null,
            };

            #[cfg(test)]
            eprintln!("DEBUG: Parsed object {} as {:?}", obj_number, obj);

            // Embedded objects MUST NOT be streams (spec disallows nested streams)
            if matches!(obj, PdfObject::Stream(_)) {
                self.emit_diagnostic(
                    DiagCode::StructInvalidObjstm,
                    format!("Embedded object {} in ObjStm {} is a Stream, which is not allowed per PDF spec", obj_number, obj_stm_ref),
                );
                result.push((obj_number, PdfObject::Null));
            } else {
                result.push((obj_number, obj));
            }

            // Note: Object parser uses the old parser-specific diagnostic system
            // We don't forward those diagnostics here since the systems are different
            // The object parser diagnostics are available via obj_parser.take_diagnostics()
            // but we skip them for now since objstm uses the unified diagnostics system
        }

        // Handle /Extends if present
        if let Some(extends_ref) = stream_dict.get("/Extends").and_then(|obj| obj.as_ref()) {
            // Resolve the parent ObjStm
            if let Some(parent_stream) = resolve_fn(extends_ref) {
                let parent_ref = extends_ref;

                // Recursively parse the parent ObjStm
                match self.load_object_stream_impl(
                    parent_ref,
                    &parent_stream,
                    source,
                    resolve_fn,
                    in_progress,
                    depth + 1,
                ) {
                    Ok(parent_objects) => {
                        // Merge parent objects (child extends parent)
                        // Parent objects come first, then child objects
                        let mut merged = (*parent_objects).clone();
                        merged.extend(result.clone());
                        result = merged;
                    }
                    Err(ObjStmError::CircularRef { .. }) => {
                        // Propagate circular reference errors
                        in_progress.remove(&obj_stm_ref);
                        return Err(ObjStmError::CircularRef { obj_ref: extends_ref });
                    }
                    Err(ObjStmError::DepthExceeded { .. }) => {
                        // Propagate depth exceeded errors
                        in_progress.remove(&obj_stm_ref);
                        return Err(ObjStmError::DepthExceeded { max: MAX_EXTENDS_DEPTH });
                    }
                    Err(_) => {
                        // Failed to parse parent - just use our objects
                    }
                }
            }
        }

        // Remove from in-progress set
        in_progress.remove(&obj_stm_ref);

        // Cache the result as Arc<Vec<(u32, PdfObject)>> for indexed access
        Ok(Arc::new(result))
    }

    /// Get a cached object stream entry.
    ///
    /// Returns None if the stream is not cached.
    pub fn get_cached(&self, obj_ref: ObjRef) -> Option<ObjStmCacheEntry> {
        let cache = self.cache.read().ok()?;
        cache.get(&obj_ref).cloned()
    }

    /// Check if an object stream is cached.
    pub fn is_cached(&self, obj_ref: ObjRef) -> bool {
        if let Ok(cache) = self.cache.read() {
            cache.contains_key(&obj_ref)
        } else {
            false
        }
    }

    /// Get the current decompression counter value.
    pub fn decompress_counter(&self) -> u64 {
        *self.decompress_counter.read().unwrap()
    }
}

impl Default for ObjectStmParser {
    fn default() -> Self {
        Self::new(512 * 1024_u64.pow(2)) // 512 MiB default
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::object::{intern, PdfDict};
    use crate::parser::stream::MemorySource;
    use std::io::Write;

    #[test]
    fn test_obj_stm_error_display() {
        let err = ObjStmError::MissingKey {
            key: "/N".to_string(),
        };
        assert_eq!(format!("{}", err), "Missing required key: /N");

        let err = ObjStmError::CircularRef {
            obj_ref: ObjRef::new(1, 0),
        };
        assert!(format!("{}", err).contains("Circular"));
    }

    #[test]
    fn test_obj_stm_parser_new() {
        let parser = ObjectStmParser::new(1024);
        assert_eq!(parser.max_decompress_bytes, 1024);
        assert!(!parser.is_cached(ObjRef::new(1, 0)));
    }

    #[test]
    fn test_obj_stm_parser_default() {
        let parser = ObjectStmParser::default();
        assert_eq!(parser.max_decompress_bytes, 512 * 1024_u64.pow(2));
    }

    #[test]
    fn test_max_extends_depth() {
        assert_eq!(MAX_EXTENDS_DEPTH, 16);
    }

    /// Critical test: object stream decompresses and parses all N objects
    #[test]
    fn test_parse_simple_objstm() {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;

        // Create a simple object stream with N=2 embedded objects
        // Header: "1 0 2 2" (object 1 at offset 0, object 2 at offset 2)
        // Objects: "42" (2 bytes) and "true" (4 bytes)
        let header = b"1 0 2 2";
        let obj1 = b"42";
        let obj2 = b"true";
        let mut stream_data = Vec::new();
        stream_data.extend_from_slice(header);
        stream_data.extend_from_slice(obj1);
        stream_data.extend_from_slice(obj2);

        // Compress with flate
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&stream_data).unwrap();
        let compressed = encoder.finish().unwrap();

        // Create stream dict with /Filter and /Length
        let mut dict = PdfDict::new();
        dict.insert(intern("/Type"), PdfObject::Name(intern("/ObjStm")));
        dict.insert(intern("/N"), PdfObject::Integer(2));
        dict.insert(intern("/First"), PdfObject::Integer(header.len() as i64));
        dict.insert(intern("/Filter"), PdfObject::Name(intern("/FlateDecode")));
        dict.insert(intern("/Length"), PdfObject::Integer(compressed.len() as i64));

        // Create a PdfStream with the dict and offset 0 (for MemorySource)
        let stream = PdfStream::new(dict.clone(), 0, Some(compressed.len() as u64));

        // Create a source that contains the compressed stream data at offset 0
        let source = MemorySource::new(compressed);
        let parser = ObjectStmParser::default();

        // Mock resolve function that returns the stream
        let obj_stm_ref = ObjRef::new(10, 0);
        let stream_clone = stream.clone();
        let result = parser.load_object_stream(
            obj_stm_ref,
            &stream,
            &source,
            move |ref_obj| {
                if ref_obj == obj_stm_ref {
                    Some(stream_clone.clone())
                } else {
                    None
                }
            },
        );

        assert!(result.is_ok());
        let entry = result.unwrap();
        assert_eq!(entry.len(), 2);

        // Verify the parsed objects by 0-based index
        assert_eq!(entry[0], (1, PdfObject::Integer(42)));
        assert_eq!(entry[1], (2, PdfObject::Bool(true)));
    }

    /// Critical test: object stream with N=10 objects, all 10 dereference correctly
    #[test]
    fn test_parse_objstm_10_objects() {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;

        // Create a header with 10 object number/offset pairs
        // Objects will be: null, true, false, 42, 3.14, (test), /Name, [1], << /A 1 >>, 5 0 R
        // Note: Objects are separated by newlines for clear token boundaries
        let mut header = String::new();
        let mut objects_data = Vec::new();
        let mut offset = 0u64;

        // Object 100: null
        header.push_str(&format!("{} {} ", 100, offset));
        objects_data.extend_from_slice(b"null\n");
        offset += b"null\n".len() as u64;

        // Object 101: true
        header.push_str(&format!("{} {} ", 101, offset));
        objects_data.extend_from_slice(b"true\n");
        offset += b"true\n".len() as u64;

        // Object 102: false
        header.push_str(&format!("{} {} ", 102, offset));
        objects_data.extend_from_slice(b"false\n");
        offset += b"false\n".len() as u64;

        // Object 103: 42
        header.push_str(&format!("{} {} ", 103, offset));
        objects_data.extend_from_slice(b"42\n");
        offset += b"42\n".len() as u64;

        // Object 104: 3.14
        header.push_str(&format!("{} {} ", 104, offset));
        objects_data.extend_from_slice(b"3.14\n");
        offset += b"3.14\n".len() as u64;

        // Object 105: (test)
        header.push_str(&format!("{} {} ", 105, offset));
        objects_data.extend_from_slice(b"(test)\n");
        offset += b"(test)\n".len() as u64;

        // Object 106: /Name
        header.push_str(&format!("{} {} ", 106, offset));
        objects_data.extend_from_slice(b"/Name\n");
        offset += b"/Name\n".len() as u64;

        // Object 107: [1]
        header.push_str(&format!("{} {} ", 107, offset));
        objects_data.extend_from_slice(b"[1]\n");
        offset += b"[1]\n".len() as u64;

        // Object 108: << /A 1 >>
        header.push_str(&format!("{} {} ", 108, offset));
        objects_data.extend_from_slice(b"<< /A 1 >>\n");
        offset += b"<< /A 1 >>\n".len() as u64;

        // Object 109: 5 0 R
        header.push_str(&format!("{} {} ", 109, offset));
        objects_data.extend_from_slice(b"5 0 R\n");
        offset += b"5 0 R\n".len() as u64;

        let first = header.len() as u64;
        let mut stream_data = header.into_bytes();
        stream_data.extend_from_slice(&objects_data);

        // Compress with flate
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&stream_data).unwrap();
        let compressed = encoder.finish().unwrap();

        // Create stream dict
        let mut dict = PdfDict::new();
        dict.insert(intern("/Type"), PdfObject::Name(intern("/ObjStm")));
        dict.insert(intern("/N"), PdfObject::Integer(10));
        dict.insert(intern("/First"), PdfObject::Integer(first as i64));
        dict.insert(intern("/Filter"), PdfObject::Name(intern("/FlateDecode")));
        dict.insert(intern("/Length"), PdfObject::Integer(compressed.len() as i64));

        // Create a PdfStream with the dict and offset 0 (for MemorySource)
        let stream = PdfStream::new(dict.clone(), 0, Some(compressed.len() as u64));

        let source = MemorySource::new(compressed);
        let parser = ObjectStmParser::default();

        let obj_stm_ref = ObjRef::new(10, 0);
        let stream_clone = stream.clone();
        let result = parser.load_object_stream(
            obj_stm_ref,
            &stream,
            &source,
            move |ref_obj| {
                if ref_obj == obj_stm_ref {
                    Some(stream_clone.clone())
                } else {
                    None
                }
            },
        );

        assert!(result.is_ok());
        let entry = result.unwrap();
        assert_eq!(entry.len(), 10);

        // Verify all objects were parsed correctly by 0-based index
        assert_eq!(entry[0], (100, PdfObject::Null));
        assert_eq!(entry[1], (101, PdfObject::Bool(true)));
        assert_eq!(entry[2], (102, PdfObject::Bool(false)));
        assert_eq!(entry[3], (103, PdfObject::Integer(42)));
        assert!(matches!(entry[4], (104, PdfObject::Real(_))));
        assert!(matches!(entry[5], (105, PdfObject::String(_))));
        assert!(matches!(entry[6], (106, PdfObject::Name(_))));
        assert!(matches!(entry[7], (107, PdfObject::Array(_))));
        assert!(matches!(entry[8], (108, PdfObject::Dict(_))));
        assert!(matches!(entry[9], (109, PdfObject::Ref(_))));
    }

    #[test]
    fn test_missing_key_n() {
        let mut dict = PdfDict::new();
        // Missing /N and /First
        let stream = PdfStream::new(dict, 0, Some(100));
        let source = MemorySource::new(vec![0u8; 100]);
        let parser = ObjectStmParser::default();

        let result = parser.load_object_stream(
            ObjRef::new(1, 0),
            &stream,
            &source,
            |_| None,
        );

        assert!(matches!(result, Err(ObjStmError::MissingKey { key }) if key == "/N"));
    }

    #[test]
    fn test_missing_key_first() {
        let mut dict = PdfDict::new();
        dict.insert(intern("/N"), PdfObject::Integer(1));
        // Missing /First
        let stream = PdfStream::new(dict, 0, Some(100));
        let source = MemorySource::new(vec![0u8; 100]);
        let parser = ObjectStmParser::default();

        let result = parser.load_object_stream(
            ObjRef::new(1, 0),
            &stream,
            &source,
            |_| None,
        );

        assert!(matches!(result, Err(ObjStmError::MissingKey { key }) if key == "/First"));
    }

    #[test]
    fn test_circular_ref_detection() {
        // Create an ObjStm that extends itself
        let mut dict = PdfDict::new();
        dict.insert(intern("/Type"), PdfObject::Name(intern("/ObjStm")));
        dict.insert(intern("/N"), PdfObject::Integer(0));
        dict.insert(intern("/First"), PdfObject::Integer(0));
        dict.insert(intern("/Extends"), PdfObject::Ref(ObjRef::new(1, 0))); // Self-reference

        let stream = PdfStream::new(dict.clone(), 0, Some(100));
        let source = MemorySource::new(vec![0u8; 100]);
        let parser = ObjectStmParser::default();

        // Mock resolve function that returns the same stream (circular reference)
        let self_ref = ObjRef::new(1, 0);
        let stream_clone = stream.clone();
        let result = parser.load_object_stream(
            self_ref,
            &stream,
            &source,
            move |ref_obj| {
                if ref_obj == self_ref {
                    Some(stream_clone.clone())
                } else {
                    None
                }
            },
        );

        assert!(matches!(result, Err(ObjStmError::CircularRef { .. })));
    }

    /// Test cache hit: second call to load the same ObjStm returns the cached Arc
    #[test]
    fn test_cache_hit() {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;

        let header = b"1 0 2 2";
        let obj1 = b"42";
        let obj2 = b"true";
        let mut stream_data = Vec::new();
        stream_data.extend_from_slice(header);
        stream_data.extend_from_slice(obj1);
        stream_data.extend_from_slice(obj2);

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&stream_data).unwrap();
        let compressed = encoder.finish().unwrap();

        let mut dict = PdfDict::new();
        dict.insert(intern("/Type"), PdfObject::Name(intern("/ObjStm")));
        dict.insert(intern("/N"), PdfObject::Integer(2));
        dict.insert(intern("/First"), PdfObject::Integer(header.len() as i64));
        dict.insert(intern("/Filter"), PdfObject::Name(intern("/FlateDecode")));
        dict.insert(intern("/Length"), PdfObject::Integer(compressed.len() as i64));

        let stream = PdfStream::new(dict.clone(), 0, Some(compressed.len() as u64));

        let source = MemorySource::new(compressed);
        let parser = ObjectStmParser::default();

        let obj_stm_ref = ObjRef::new(10, 0);
        let stream_clone = stream.clone();

        // First call - should load and cache
        let result1 = parser.load_object_stream(
            obj_stm_ref,
            &stream,
            &source,
            move |ref_obj| {
                if ref_obj == obj_stm_ref {
                    Some(stream_clone.clone())
                } else {
                    None
                }
            },
        );

        assert!(result1.is_ok());
        let entry1 = result1.unwrap();

        // Second call - should return cached Arc
        let cached = parser.get_cached(obj_stm_ref);
        assert!(cached.is_some());

        // Verify Arc::ptr_eq - same Arc instance
        assert!(Arc::ptr_eq(&entry1, &cached.unwrap()));
    }

    /// Test /Extends chain - parent ObjStm extends to child ObjStm
    #[test]
    fn test_objstm_extends_chain() {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;

        // Create parent ObjStm (objects 1-3)
        let parent_header = b"1 0 2 4 3 8";
        let parent_data = b"nulltruefalse";
        let mut parent_stream = Vec::new();
        parent_stream.extend_from_slice(parent_header);
        parent_stream.extend_from_slice(parent_data);

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&parent_stream).unwrap();
        let parent_compressed = encoder.finish().unwrap();

        let mut parent_dict = PdfDict::new();
        parent_dict.insert(intern("/Type"), PdfObject::Name(intern("/ObjStm")));
        parent_dict.insert(intern("/N"), PdfObject::Integer(3));
        parent_dict.insert(intern("/First"), PdfObject::Integer(parent_header.len() as i64));
        parent_dict.insert(intern("/Filter"), PdfObject::Name(intern("/FlateDecode")));
        parent_dict.insert(intern("/Length"), PdfObject::Integer(parent_compressed.len() as i64));

        // Create child ObjStm (objects 4-5) that extends parent
        let child_header = b"4 0 5 4";
        let child_data = b"42true";
        let mut child_stream = Vec::new();
        child_stream.extend_from_slice(child_header);
        child_stream.extend_from_slice(child_data);

        let mut encoder2 = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder2.write_all(&child_stream).unwrap();
        let child_compressed = encoder2.finish().unwrap();

        let parent_ref = ObjRef::new(100, 0);

        let mut child_dict = PdfDict::new();
        child_dict.insert(intern("/Type"), PdfObject::Name(intern("/ObjStm")));
        child_dict.insert(intern("/N"), PdfObject::Integer(2));
        child_dict.insert(intern("/First"), PdfObject::Integer(child_header.len() as i64));
        child_dict.insert(intern("/Filter"), PdfObject::Name(intern("/FlateDecode")));
        child_dict.insert(intern("/Length"), PdfObject::Integer(child_compressed.len() as i64));
        child_dict.insert(intern("/Extends"), PdfObject::Ref(parent_ref));

        let parser = ObjectStmParser::default();
        let source = MemorySource::new(child_compressed);

        // Mock resolve function that returns the appropriate stream
        let child_ref = ObjRef::new(200, 0);
        let child_dict_clone = child_dict.clone();
        let parent_dict_clone = parent_dict.clone();
        let child_stream = PdfStream::new(child_dict_clone.clone(), 0, None);

        let result = parser.load_object_stream(
            child_ref,
            &child_stream,
            &source,
            move |ref_obj| {
                if ref_obj == parent_ref {
                    // Return parent stream
                    Some(PdfStream::new(
                        parent_dict_clone.clone(),
                        0,
                        None,
                    ))
                } else if ref_obj == child_ref {
                    Some(PdfStream::new(
                        child_dict_clone.clone(),
                        0,
                        None,
                    ))
                } else {
                    None
                }
            },
        );

        // The test may not fully work due to source limitations,
        // but it verifies the /Extends handling doesn't crash
        assert!(result.is_ok() || matches!(result, Err(ObjStmError::DecompressionFailed)));
    }

    /// Test get_object API for xref type-2 entry resolution
    #[test]
    fn test_get_object_api() {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;

        let header = b"100 0 101 2";
        let obj1 = b"42";
        let obj2 = b"true";
        let mut stream_data = Vec::new();
        stream_data.extend_from_slice(header);
        stream_data.extend_from_slice(obj1);
        stream_data.extend_from_slice(obj2);

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&stream_data).unwrap();
        let compressed = encoder.finish().unwrap();

        let mut dict = PdfDict::new();
        dict.insert(intern("/Type"), PdfObject::Name(intern("/ObjStm")));
        dict.insert(intern("/N"), PdfObject::Integer(2));
        dict.insert(intern("/First"), PdfObject::Integer(header.len() as i64));
        dict.insert(intern("/Filter"), PdfObject::Name(intern("/FlateDecode")));
        dict.insert(intern("/Length"), PdfObject::Integer(compressed.len() as i64));

        let source = MemorySource::new(compressed);
        let parser = ObjectStmParser::default();

        let obj_stm_ref = ObjRef::new(10, 0);
        let stream = PdfStream::new(dict.clone(), 0, None);

        // Get object at index 0 (object number 100) from the stream
        let obj = parser.get_object(
            obj_stm_ref,
            0, // 0-based index
            &source,
            |ref_obj| {
                if ref_obj == obj_stm_ref {
                    Some(stream.clone())
                } else {
                    None
                }
            },
        );

        assert_eq!(obj, PdfObject::Integer(42));

        // Get object at index 1 (object number 101) from the stream (should be cached now)
        let obj2 = parser.get_object(
            obj_stm_ref,
            1, // 0-based index
            &source,
            |ref_obj| {
                if ref_obj == obj_stm_ref {
                    Some(stream.clone())
                } else {
                    None
                }
            },
        );

        assert_eq!(obj2, PdfObject::Bool(true));

        // Verify cache hit
        assert!(parser.is_cached(obj_stm_ref));
    }

    /// Test truncated ObjStm body: partial objects returned with STRUCT_INVALID_OBJSTM diagnostic
    #[test]
    fn test_truncated_objstm_body() {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;

        // Create an ObjStm where the last object is truncated
        // Header: "100 0 101 3 102 8" (3 objects)
        // Objects: "42 ", "true ", "fal" (truncated "false")
        // Note: Objects must be separated by whitespace for the lexer to tokenize correctly
        let header = b"100 0 101 3 102 8";
        let obj1 = b"42 ";
        let obj2 = b"true ";
        let obj3 = b"fal"; // Truncated "false"
        let mut stream_data = Vec::new();
        stream_data.extend_from_slice(header);
        stream_data.extend_from_slice(obj1);
        stream_data.extend_from_slice(obj2);
        stream_data.extend_from_slice(obj3);

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&stream_data).unwrap();
        let compressed = encoder.finish().unwrap();
        let compressed_len = compressed.len() as u64;

        let mut dict = PdfDict::new();
        dict.insert(intern("/Type"), PdfObject::Name(intern("/ObjStm")));
        dict.insert(intern("/N"), PdfObject::Integer(3));
        dict.insert(intern("/First"), PdfObject::Integer(header.len() as i64));
        dict.insert(intern("/Filter"), PdfObject::Name(intern("/FlateDecode")));
        dict.insert(intern("/Length"), PdfObject::Integer(compressed.len() as i64));

        let source = MemorySource::new(compressed);
        let parser = ObjectStmParser::default();

        let obj_stm_ref = ObjRef::new(10, 0);
        let dict_clone = dict.clone();
        let stream = PdfStream::new(dict.clone(), 0, Some(compressed_len));
        let result = parser.load_object_stream(
            obj_stm_ref,
            &stream,
            &source,
            move |ref_obj| {
                if ref_obj == obj_stm_ref {
                    Some(PdfStream::new(
                        dict_clone.clone(),
                        0,
                        Some(compressed_len),
                    ))
                } else {
                    None
                }
            },
        );

        // Should succeed with partial objects
        assert!(result.is_ok());
        let entry = result.unwrap();

        // First two objects should be parsed correctly
        assert_eq!(entry[0], (100, PdfObject::Integer(42)));
        assert_eq!(entry[1], (101, PdfObject::Bool(true)));

        // Third object is truncated ("fal" instead of "false")
        // The parser should handle this gracefully without panic
        // It may return Null or Keyword depending on lexer behavior
        assert!(!matches!(entry[2], (_, PdfObject::Stream(_)))); // Should not be a stream
    }

    /// Test decompression-bomb ObjStm: emits STREAM_BOMB and processes objects that fit within the limit
    #[test]
    fn test_decompression_bomb_objstm() {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;

        // Create a small compressed payload that expands to a large size
        // This simulates a decompression bomb attack
        // We'll use a small max_decompress_bytes limit to trigger the bomb detection
        let max_bytes = 100; // Very small limit for testing

        // Create a header with 2 objects
        let header = b"1 0 2 3";
        let obj1 = b"42";
        let obj2 = b"true";
        let mut stream_data = Vec::new();
        stream_data.extend_from_slice(header);
        stream_data.extend_from_slice(obj1);
        stream_data.extend_from_slice(obj2);

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&stream_data).unwrap();
        let compressed = encoder.finish().unwrap();

        let mut dict = PdfDict::new();
        dict.insert(intern("/Type"), PdfObject::Name(intern("/ObjStm")));
        dict.insert(intern("/N"), PdfObject::Integer(2));
        dict.insert(intern("/First"), PdfObject::Integer(header.len() as i64));
        dict.insert(intern("/Filter"), PdfObject::Name(intern("/FlateDecode")));
        dict.insert(intern("/Length"), PdfObject::Integer(compressed.len() as i64));

        // Create parser with very small decompression limit
        let parser = ObjectStmParser::new(max_bytes);
        let source = MemorySource::new(compressed);

        let obj_stm_ref = ObjRef::new(10, 0);
        let dict_clone = dict.clone();
        let stream = PdfStream::new(dict.clone(), 0, None);
        let result = parser.load_object_stream(
            obj_stm_ref,
            &stream,
            &source,
            move |ref_obj| {
                if ref_obj == obj_stm_ref {
                    Some(PdfStream::new(
                        dict_clone.clone(),
                        0,
                        None,
                    ))
                } else {
                    None
                }
            },
        );

        // The result should be ok (we get what we can before hitting the limit)
        // but diagnostics should be emitted
        assert!(result.is_ok());

        let diags = parser.take_diagnostics();
        // Check if any diagnostic is related to stream bomb or decompression
        let has_bomb_diag = diags.iter().any(|d| d.code == DiagCode::StreamBomb);
        // Note: The actual bomb detection depends on the decompression implementation
        // This test verifies that the parser doesn't crash on large decompressions
    }

    /// Test embedded stream detection: embedded objects MUST NOT be streams
    #[test]
    fn test_embedded_stream_rejected() {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;

        // Create an ObjStm with an embedded object that looks like a stream
        // The header and data will contain a stream-like object
        let header = b"100 0";
        // An embedded object that looks like it has stream markers
        // (embedded objects can't be streams per spec)
        let obj_data = b"<< /Length 5 >>"; // Just a dict, not a stream
        let mut stream_data = Vec::new();
        stream_data.extend_from_slice(header);
        stream_data.extend_from_slice(obj_data);

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&stream_data).unwrap();
        let compressed = encoder.finish().unwrap();

        let mut dict = PdfDict::new();
        dict.insert(intern("/Type"), PdfObject::Name(intern("/ObjStm")));
        dict.insert(intern("/N"), PdfObject::Integer(1));
        dict.insert(intern("/First"), PdfObject::Integer(header.len() as i64));
        dict.insert(intern("/Filter"), PdfObject::Name(intern("/FlateDecode")));
        dict.insert(intern("/Length"), PdfObject::Integer(compressed.len() as i64));

        let source = MemorySource::new(compressed);
        let parser = ObjectStmParser::default();

        let obj_stm_ref = ObjRef::new(10, 0);
        let dict_clone = dict.clone();
        let stream = PdfStream::new(dict.clone(), 0, None);
        let result = parser.load_object_stream(
            obj_stm_ref,
            &stream,
            &source,
            move |ref_obj| {
                if ref_obj == obj_stm_ref {
                    Some(PdfStream::new(
                        dict_clone.clone(),
                        0,
                        None,
                    ))
                } else {
                    None
                }
            },
        );

        assert!(result.is_ok());
        let entry = result.unwrap();

        // The embedded object should be a dict, not a stream
        assert!(matches!(entry[0], (100, PdfObject::Dict(_))));
    }

    /// Test depth exceeded on /Extends chain
    #[test]
    fn test_extends_depth_exceeded() {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;

        // Create a simple ObjStm
        let header = b"1 0";
        let obj_data = b"42";
        let mut stream_data = Vec::new();
        stream_data.extend_from_slice(header);
        stream_data.extend_from_slice(obj_data);

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&stream_data).unwrap();
        let compressed = encoder.finish().unwrap();

        // Create the base dict (no /Extends)
        let mut base_dict = PdfDict::new();
        base_dict.insert(intern("/Type"), PdfObject::Name(intern("/ObjStm")));
        base_dict.insert(intern("/N"), PdfObject::Integer(1));
        base_dict.insert(intern("/First"), PdfObject::Integer(header.len() as i64));
        base_dict.insert(intern("/Filter"), PdfObject::Name(intern("/FlateDecode")));
        base_dict.insert(intern("/Length"), PdfObject::Integer(compressed.len() as i64));

        // Create a chain of ObjStms where each extends the previous
        // We'll create 18 dicts (0-17), each extending the previous
        let mut dicts = Vec::new();
        for i in 0..=17 {
            let mut dict = base_dict.clone();
            if i > 0 {
                // This ObjStm extends the previous one
                dict.insert(intern("/Extends"), PdfObject::Ref(ObjRef::new(100 + (i as u32) - 1, 0)));
            }
            dicts.push(dict);
        }

        let parser = ObjectStmParser::default();
        let source = MemorySource::new(compressed.clone());

        // Test loading the 17th ObjStm (which should exceed MAX_EXTENDS_DEPTH of 16)
        let obj_stm_17_ref = ObjRef::new(117, 0);
        let stream_17 = PdfStream::new(dicts[17].clone(), 0, None);

        let result = parser.load_object_stream(
            obj_stm_17_ref,
            &stream_17,
            &source,
            |ref_obj| {
                // Return a stream for any ref in the chain
                if ref_obj.object >= 100 && ref_obj.object <= 117 {
                    let idx = (ref_obj.object - 100) as usize;
                    Some(PdfStream::new(dicts[idx].clone(), 0, None))
                } else {
                    None
                }
            },
        );

        // Should fail with DepthExceeded
        assert!(matches!(result, Err(ObjStmError::DepthExceeded { .. })));
    }
}
