//! Cross-reference table resolver and traditional xref parser.
//!
//! This module provides:
//! - Traditional xref table parser (20-byte fixed-width entries)
//! - Xref resolver for indirect object resolution
//! - Handling of object streams and circular reference detection

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::borrow::Cow;
use crate::parser::object::{ObjRef, PdfObject, PdfDict};
use crate::parser::stream::PdfSource;

/// Error type for xref resolution.
#[derive(Debug, Clone)]
pub enum ResolveError {
    /// Object not found in xref table
    NotFound(ObjRef),
    /// Circular reference detected
    CircularRef(ObjRef),
    /// I/O error
    Io(String),
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolveError::NotFound(obj_ref) => write!(f, "object {} not found", obj_ref),
            ResolveError::CircularRef(obj_ref) => write!(f, "circular reference at {}", obj_ref),
            ResolveError::Io(msg) => write!(f, "I/O error: {}", msg),
        }
    }
}

impl std::error::Error for ResolveError {}

/// Result type for resolution operations.
pub type ResolveResult<T> = Result<T, ResolveError>;

/// Cross-reference table entry.
#[derive(Debug, Clone, PartialEq)]
pub enum XrefEntry {
    /// Free entry (available for reuse)
    Free { next_free: u32, gen_nr: u16 },
    /// In-use entry at a specific byte offset
    InUse { offset: u64, gen_nr: u16 },
    /// Compressed object in an object stream
    Compressed { obj_stm_nr: u32, index: u32 },
}

/// Diagnostic codes for xref parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XrefDiagCode {
    /// Invalid xref keyword or header
    InvalidXrefHeader,
    /// Malformed xref entry (not 20 bytes, bad format)
    InvalidXrefEntry,
    /// Invalid subsection header (not "start count")
    InvalidSubsectionHeader,
    /// Object 0 is not free (violates PDF spec)
    ObjectZeroNotFree,
    /// Trailer dictionary not found or malformed
    TrailerNotFound,
    /// Truncated xref table (unexpected EOF)
    XrefTruncated,
}

/// A diagnostic message emitted during xref parsing.
#[derive(Debug, Clone, PartialEq)]
pub struct XrefDiagnostic {
    /// The diagnostic code
    pub code: XrefDiagCode,
    /// Byte offset in the input where the error occurred
    pub byte_offset: u64,
    /// Human-readable error message
    pub msg: Cow<'static, str>,
}

impl XrefDiagnostic {
    /// Create a diagnostic with a static message.
    fn with_static(code: XrefDiagCode, byte_offset: u64, msg: &'static str) -> Self {
        XrefDiagnostic {
            code,
            byte_offset,
            msg: Cow::Borrowed(msg),
        }
    }

    /// Create a diagnostic with a dynamic message.
    fn with_dynamic(code: XrefDiagCode, byte_offset: u64, msg: String) -> Self {
        XrefDiagnostic {
            code,
            byte_offset,
            msg: Cow::Owned(msg),
        }
    }
}

/// Result of parsing a traditional xref table.
///
/// Contains the parsed xref entries and the trailer dictionary.
#[derive(Debug, Clone)]
pub struct XrefSection {
    /// Map from object number to xref entry
    pub entries: HashMap<u32, XrefEntry>,
    /// The trailer dictionary
    pub trailer: Option<PdfDict>,
    /// Diagnostics emitted during parsing
    pub diagnostics: Vec<XrefDiagnostic>,
}

impl XrefSection {
    /// Create a new empty xref section.
    pub fn new() -> Self {
        XrefSection {
            entries: HashMap::new(),
            trailer: None,
            diagnostics: Vec::new(),
        }
    }

    /// Add an entry to the xref section.
    pub fn add_entry(&mut self, obj_nr: u32, entry: XrefEntry) {
        self.entries.insert(obj_nr, entry);
    }

    /// Get the number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the xref section is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for XrefSection {
    fn default() -> Self {
        Self::new()
    }
}

/// Cross-reference resolver.
///
/// This resolver tracks the mapping from object numbers to their file locations
/// and handles resolution through object streams. It also detects circular
/// references to prevent infinite loops.
pub struct XrefResolver {
    /// Map from object number to xref entry
    entries: HashMap<u32, XrefEntry>,
    /// Cache of resolved objects (for object streams)
    cache: Arc<RwLock<HashMap<ObjRef, PdfObject>>>,
    /// Per-thread resolution stack for circular reference detection
    resolving: Arc<RwLock<HashSet<ObjRef>>>,
}

impl XrefResolver {
    /// Create a new xref resolver.
    pub fn new() -> Self {
        XrefResolver {
            entries: HashMap::new(),
            cache: Arc::new(RwLock::new(HashMap::new())),
            resolving: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Create a new xref resolver from an XrefSection.
    pub fn from_section(section: XrefSection) -> Self {
        XrefResolver {
            entries: section.entries,
            cache: Arc::new(RwLock::new(HashMap::new())),
            resolving: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Add an xref entry.
    pub fn add_entry(&mut self, obj_nr: u32, entry: XrefEntry) {
        self.entries.insert(obj_nr, entry);
    }

    /// Get the xref entry for an object number.
    pub fn get_entry(&self, obj_nr: u32) -> Option<&XrefEntry> {
        self.entries.get(&obj_nr)
    }

    /// Check if a resolution is in progress (for circular reference detection).
    pub fn is_resolving(&self, obj_ref: ObjRef) -> bool {
        self.resolving.read().unwrap().contains(&obj_ref)
    }

    /// Mark an object as being resolved.
    pub fn start_resolving(&self, obj_ref: ObjRef) -> bool {
        let mut resolving = self.resolving.write().unwrap();
        if resolving.contains(&obj_ref) {
            return false;
        }
        resolving.insert(obj_ref);
        true
    }

    /// Mark an object as finished resolving.
    pub fn finish_resolving(&self, obj_ref: ObjRef) {
        self.resolving.write().unwrap().remove(&obj_ref);
    }

    /// Resolve an object reference to its value.
    ///
    /// This is a stub implementation that returns Null. The full implementation
    /// (Phase 1.3) will:
    /// - Check for circular references
    /// - Look up the xref entry
    /// - Read and parse the object from its offset
    /// - Handle object streams
    /// - Cache resolved objects
    pub fn resolve(&self, obj_ref: ObjRef) -> ResolveResult<PdfObject> {
        // Check for circular reference
        if !self.start_resolving(obj_ref) {
            return Err(ResolveError::CircularRef(obj_ref));
        }

        // Check cache first
        {
            let cache = self.cache.read().unwrap();
            if let Some(obj) = cache.get(&obj_ref) {
                self.finish_resolving(obj_ref);
                return Ok(obj.clone());
            }
        }

        // Look up the xref entry
        let _entry = self.entries.get(&obj_ref.object)
            .ok_or_else(|| ResolveError::NotFound(obj_ref))?;

        // Stub: return Null for now
        // Full implementation will read from file offset and parse
        self.finish_resolving(obj_ref);
        Ok(PdfObject::Null)
    }

    /// Cache a resolved object.
    pub fn cache_object(&self, obj_ref: ObjRef, obj: PdfObject) {
        self.cache.write().unwrap().insert(obj_ref, obj);
    }

    /// Get the number of entries in the xref table.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the xref table is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for XrefResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a traditional PDF xref table starting from the given offset.
///
/// # Parameters
/// - `source`: The PDF source to read bytes from
/// - `start_offset`: The byte offset where the xref table begins (from `startxref`)
///
/// # Returns
/// An `XrefSection` containing the parsed entries and trailer dictionary.
///
/// # Format
/// The xref table has the following format:
/// ```text
/// xref
/// 0 6
/// 0000000003 65535 f
/// 0000000017 00000 n
/// ...
/// trailer
/// << /Size 6 /Root 1 0 R >>
/// ```
///
/// Each entry is exactly 20 bytes:
/// - 10 digits: byte offset (for `n`) or next-free-object number (for `f`)
/// - 1 space
/// - 5 digits: generation number
/// - 1 space
/// - 1 byte: `n` (in use) or `f` (free)
/// - 2 bytes: line ending (`\r\n` or ` \n`)
///
/// Some buggy producers use `\n` alone (19 bytes), which is detected and handled.
pub fn parse_traditional_xref(source: &dyn PdfSource, start_offset: u64) -> XrefSection {
    let mut result = XrefSection::new();
    let mut pos = start_offset;

    // Read initial chunk to look for xref keyword
    let header_bytes = match source.read_at(pos, 1024) {
        Ok(bytes) if !bytes.is_empty() => bytes,
        _ => {
            result.diagnostics.push(XrefDiagnostic::with_static(
                XrefDiagCode::XrefTruncated,
                pos,
                "Failed to read xref header",
            ));
            return result;
        }
    };

    // Look for xref keyword (case-sensitive per PDF spec)
    let header_str = std::str::from_utf8(&header_bytes);
    let xref_start = match header_str {
        Ok(s) => {
            // Skip leading whitespace
            let s = s.trim_start();
            if s.starts_with("xref") {
                s.len() - s["xref".len()..].len()
            } else {
                result.diagnostics.push(XrefDiagnostic::with_static(
                    XrefDiagCode::InvalidXrefHeader,
                    pos,
                    "xref keyword not found",
                ));
                return result;
            }
        }
        Err(_) => {
            result.diagnostics.push(XrefDiagnostic::with_static(
                XrefDiagCode::InvalidXrefHeader,
                pos,
                "Invalid UTF-8 in xref header",
            ));
            return result;
        }
    };

    pos += xref_start as u64 + 3; // Skip "xref"

    // Parse subsections until we hit "trailer"
    loop {
        // Skip whitespace before subsection header or trailer
        let ws_bytes = match source.read_at(pos, 100) {
            Ok(bytes) => bytes,
            _ => {
                result.diagnostics.push(XrefDiagnostic::with_static(
                    XrefDiagCode::XrefTruncated,
                    pos,
                    "Failed to read before subsection/trailer",
                ));
                break;
            }
        };

        // Check for trailer keyword
        let ws_str = std::str::from_utf8(&ws_bytes);
        if let Ok(s) = ws_str {
            let trimmed = s.trim_start();
            if trimmed.starts_with("trailer") {
                // Found trailer - parse it and we're done
                pos += (s.len() - trimmed.len()) as u64 + 7; // Skip "trailer"
                result.trailer = parse_trailer_dict(source, &mut pos, &mut result.diagnostics);
                break;
            }
        }

        // Parse subsection header: "obj_start obj_count"
        let subsection_start = pos;
        let header_line = match read_line(source, &mut pos, &mut result.diagnostics) {
            Some(line) => line,
            None => {
                result.diagnostics.push(XrefDiagnostic::with_static(
                    XrefDiagCode::InvalidSubsectionHeader,
                    subsection_start,
                    "Failed to read subsection header",
                ));
                break;
            }
        };

        let header_parts: Vec<&str> = header_line.split_whitespace().collect();
        if header_parts.len() != 2 {
            result.diagnostics.push(XrefDiagnostic::with_dynamic(
                XrefDiagCode::InvalidSubsectionHeader,
                subsection_start,
                format!("Invalid subsection header: {}", header_line),
            ));
            // Try to continue - might be trailer
            if header_line.trim().starts_with("trailer") {
                result.trailer = parse_trailer_dict(source, &mut pos, &mut result.diagnostics);
                break;
            }
            continue;
        }

        let obj_start: u32 = match header_parts[0].parse() {
            Ok(n) => n,
            Err(_) => {
                result.diagnostics.push(XrefDiagnostic::with_dynamic(
                    XrefDiagCode::InvalidSubsectionHeader,
                    subsection_start,
                    format!("Invalid subsection start: {}", header_parts[0]),
                ));
                continue;
            }
        };

        let obj_count: u32 = match header_parts[1].parse() {
            Ok(n) => n,
            Err(_) => {
                result.diagnostics.push(XrefDiagnostic::with_dynamic(
                    XrefDiagCode::InvalidSubsectionHeader,
                    subsection_start,
                    format!("Invalid subsection count: {}", header_parts[1]),
                ));
                continue;
            }
        };

        // Parse subsection entries
        // We need to detect stride (20 vs 19 bytes) by trying the first entry
        let mut stride = 20; // Default to 20 bytes
        let mut entries_parsed = 0u32;

        while entries_parsed < obj_count {
            let entry_start = pos;

            // Read a candidate entry (try 20 bytes first, fall back to 19)
            let entry_bytes = match source.read_at(pos, 20) {
                Ok(bytes) => bytes,
                _ => {
                    result.diagnostics.push(XrefDiagnostic::with_static(
                        XrefDiagCode::XrefTruncated,
                        pos,
                        "Failed to read xref entry",
                    ));
                    break;
                }
            };

            if entry_bytes.len() < 19 {
                // Definitely truncated
                result.diagnostics.push(XrefDiagnostic::with_static(
                    XrefDiagCode::XrefTruncated,
                    pos,
                    "Xref entry truncated (< 19 bytes)",
                ));
                break;
            }

            // Try to parse as 20-byte entry first
            let parsed = if entry_bytes.len() >= 20 {
                parse_xref_entry(&entry_bytes[..20], obj_start + entries_parsed, entry_start, stride, &mut result.diagnostics)
            } else {
                // Try 19-byte entry for buggy producers
                stride = 19;
                parse_xref_entry(&entry_bytes[..19], obj_start + entries_parsed, entry_start, stride, &mut result.diagnostics)
            };

            match parsed {
                Some((obj_nr, entry)) => {
                    // Object 0 must be free (PDF spec requirement)
                    if obj_nr == 0 {
                        if let XrefEntry::InUse { .. } = entry {
                            result.diagnostics.push(XrefDiagnostic::with_static(
                                XrefDiagCode::ObjectZeroNotFree,
                                entry_start,
                                "Object 0 is not free (violates PDF spec)",
                            ));
                        }
                    }
                    // Only add in-use entries (free entries are ignored per task description)
                    if let XrefEntry::InUse { .. } = entry {
                        result.add_entry(obj_nr, entry);
                    }
                    pos += stride as u64;
                    entries_parsed += 1;
                }
                None => {
                    // Failed to parse - try 19-byte stride if we haven't yet
                    if stride == 20 && entry_bytes.len() >= 19 {
                        stride = 19;
                        continue;
                    }
                    // Skip this entry and move on
                    pos += stride as u64;
                    entries_parsed += 1;
                }
            }
        }
    }

    result
}

/// Parse a single xref entry.
///
/// Returns Some((obj_nr, entry)) on success, None on failure.
fn parse_xref_entry(
    bytes: &[u8],
    obj_nr: u32,
    offset: u64,
    stride: usize,
    diagnostics: &mut Vec<XrefDiagnostic>,
) -> Option<(u32, XrefEntry)> {
    if bytes.len() != stride {
        return None;
    }

    // Convert to string for parsing
    let entry_str = match std::str::from_utf8(bytes) {
        Ok(s) => s,
        Err(_) => {
            diagnostics.push(XrefDiagnostic::with_static(
                XrefDiagCode::InvalidXrefEntry,
                offset,
                "Invalid UTF-8 in xref entry",
            ));
            return None;
        }
    };

    // Entry format: "offset/next_free generation f/n" with line ending
    let parts: Vec<&str> = entry_str.split_whitespace().collect();
    if parts.len() < 3 {
        diagnostics.push(XrefDiagnostic::with_dynamic(
            XrefDiagCode::InvalidXrefEntry,
            offset,
            format!("Malformed xref entry: {}", entry_str.trim()),
        ));
        return None;
    }

    let first_field: u64 = match parts[0].parse() {
        Ok(n) => n,
        Err(_) => {
            diagnostics.push(XrefDiagnostic::with_dynamic(
                XrefDiagCode::InvalidXrefEntry,
                offset,
                format!("Invalid offset/next_free: {}", parts[0]),
            ));
            return None;
        }
    };

    let gen_nr: u16 = match parts[1].parse() {
        Ok(n) => n,
        Err(_) => {
            diagnostics.push(XrefDiagnostic::with_dynamic(
                XrefDiagCode::InvalidXrefEntry,
                offset,
                format!("Invalid generation: {}", parts[1]),
            ));
            return None;
        }
    };

    let entry_type = parts[2].chars().next();
    match entry_type {
        Some('n') | Some('N') => Some((obj_nr, XrefEntry::InUse { offset: first_field, gen_nr })),
        Some('f') | Some('F') => Some((obj_nr, XrefEntry::Free { next_free: first_field as u32, gen_nr })),
        _ => {
            diagnostics.push(XrefDiagnostic::with_dynamic(
                XrefDiagCode::InvalidXrefEntry,
                offset,
                format!("Invalid entry type: {}", parts[2]),
            ));
            None
        }
    }
}

/// Read a line from the source, updating the position.
///
/// Returns None on EOF or error.
fn read_line(
    source: &dyn PdfSource,
    pos: &mut u64,
    diagnostics: &mut Vec<XrefDiagnostic>,
) -> Option<String> {
    let mut result = String::new();
    let mut chunk_pos = 0;
    let chunk_size = 256;

    loop {
        let chunk = match source.read_at(*pos + chunk_pos, chunk_size) {
            Ok(bytes) => bytes,
            Err(_) => {
                diagnostics.push(XrefDiagnostic::with_static(
                    XrefDiagCode::XrefTruncated,
                    *pos,
                    "I/O error reading line",
                ));
                return None;
            }
        };

        if chunk.is_empty() {
            break;
        }

        // Look for line ending
        for (i, &byte) in chunk.iter().enumerate() {
            if byte == b'\r' {
                // Check for CRLF
                if i + 1 < chunk.len() && chunk[i + 1] == b'\n' {
                    result.push_str(std::str::from_utf8(&chunk[..i]).ok()?);
                    *pos += chunk_pos + i as u64 + 2;
                    return Some(result);
                }
                // Single CR
                result.push_str(std::str::from_utf8(&chunk[..i]).ok()?);
                *pos += chunk_pos + i as u64 + 1;
                return Some(result);
            }
            if byte == b'\n' {
                // Single LF
                result.push_str(std::str::from_utf8(&chunk[..i]).ok()?);
                *pos += chunk_pos + i as u64 + 1;
                return Some(result);
            }
        }

        // No line ending found - add chunk and continue
        result.push_str(std::str::from_utf8(&chunk).ok()?);
        chunk_pos += chunk.len() as u64;

        // Safety: don't read forever
        if chunk_pos > 10000 {
            break;
        }
    }

    if result.is_empty() {
        None
    } else {
        *pos += chunk_pos;
        Some(result)
    }
}

/// Parse the trailer dictionary.
///
/// This is a simplified implementation that reads until the end of the
/// dictionary (>>) and returns a placeholder dict object.
/// The full implementation will use the object parser from Phase 1.2.
fn parse_trailer_dict(
    source: &dyn PdfSource,
    pos: &mut u64,
    diagnostics: &mut Vec<XrefDiagnostic>,
) -> Option<PdfDict> {
    // Skip whitespace before <<
    let mut seen_bracket = false;
    let mut depth = 0;
    let mut chunk_pos = 0u64;

    loop {
        let chunk = match source.read_at(*pos + chunk_pos, 1024) {
            Ok(bytes) => bytes,
            Err(_) => {
                diagnostics.push(XrefDiagnostic::with_static(
                    XrefDiagCode::TrailerNotFound,
                    *pos,
                    "I/O error reading trailer",
                ));
                return None;
            }
        };

        if chunk.is_empty() {
            break;
        }

        for (i, &byte) in chunk.iter().enumerate() {
            if !seen_bracket {
                if byte == b'<' {
                    // Check for << (dict start)
                    if i + 1 < chunk.len() && chunk[i + 1] == b'<' {
                        seen_bracket = true;
                        depth = 1;
                        chunk_pos += i as u64 + 2;
                        // Start fresh scan after <<
                        let remaining = &chunk[i + 2..];
                        for (j, &b) in remaining.iter().enumerate() {
                            if b == b'<' {
                                if j + 1 < remaining.len() && remaining[j + 1] == b'<' {
                                    depth += 1;
                                }
                            } else if b == b'>' {
                                if j + 1 < remaining.len() && remaining[j + 1] == b'>' {
                                    depth -= 1;
                                    if depth == 0 {
                                        *pos += chunk_pos + j as u64 + 2;
                                        return Some(PdfDict::new());
                                    }
                                }
                            }
                        }
                        break;
                    }
                }
                continue;
            }
        }

        chunk_pos += chunk.len() as u64;

        // Safety limit
        if chunk_pos > 100000 {
            diagnostics.push(XrefDiagnostic::with_static(
                XrefDiagCode::TrailerNotFound,
                *pos,
                "Trailer dictionary too large or unterminated",
            ));
            return None;
        }
    }

    diagnostics.push(XrefDiagnostic::with_static(
        XrefDiagCode::TrailerNotFound,
        *pos,
        "Trailer dictionary not found",
    ));
    None
}

/// Parse a direct PDF object (for trailer dictionary parsing).
///
/// This is a stub implementation that will be completed in Phase 1.2.
/// For now, it returns null for all inputs.
#[allow(dead_code)]
fn parse_direct_object(_source: &dyn PdfSource, _pos: &mut u64) -> Option<PdfObject> {
    // Stub: return null for now
    // Full implementation will parse the actual PDF object
    Some(PdfObject::Null)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_obj_ref() {
        let obj_ref = ObjRef::new(1, 0);
        assert_eq!(obj_ref.object, 1);
        assert_eq!(obj_ref.generation, 0);
    }

    #[test]
    fn test_xref_resolver_new() {
        let resolver = XrefResolver::new();
        assert!(resolver.is_empty());
        assert_eq!(resolver.len(), 0);
    }

    #[test]
    fn test_add_entry() {
        let mut resolver = XrefResolver::new();
        resolver.add_entry(1, XrefEntry::InUse { offset: 100, gen_nr: 0 });
        assert_eq!(resolver.len(), 1);
    }

    #[test]
    fn test_get_entry() {
        let mut resolver = XrefResolver::new();
        let entry = XrefEntry::InUse { offset: 100, gen_nr: 0 };
        resolver.add_entry(1, entry.clone());
        assert_eq!(resolver.get_entry(1), Some(&entry));
    }

    #[test]
    fn test_circular_ref_detection() {
        let resolver = XrefResolver::new();
        let obj_ref = ObjRef::new(1, 0);

        assert!(resolver.start_resolving(obj_ref));
        assert!(resolver.is_resolving(obj_ref));
        assert!(!resolver.start_resolving(obj_ref)); // Second call fails

        resolver.finish_resolving(obj_ref);
        assert!(!resolver.is_resolving(obj_ref));
        assert!(resolver.start_resolving(obj_ref)); // Can start again
    }

    #[test]
    fn test_resolve_not_found() {
        let resolver = XrefResolver::new();
        let obj_ref = ObjRef::new(999, 0);
        assert!(matches!(
            resolver.resolve(obj_ref),
            Err(ResolveError::NotFound(_))
        ));
    }

    #[test]
    fn test_cache_object() {
        let resolver = XrefResolver::new();
        let obj_ref = ObjRef::new(1, 0);
        let obj = PdfObject::Integer(42);

        resolver.cache_object(obj_ref, obj.clone());

        // Resolve should return cached object
        let resolved = resolver.resolve(obj_ref).unwrap();
        assert!(matches!(resolved, PdfObject::Integer(42)));
    }

    // Traditional xref parsing tests

    #[test]
    fn test_xref_section_new() {
        let section = XrefSection::new();
        assert!(section.is_empty());
        assert_eq!(section.len(), 0);
        assert!(section.trailer.is_none());
        assert!(section.diagnostics.is_empty());
    }

    #[test]
    fn test_xref_section_add_entry() {
        let mut section = XrefSection::new();
        section.add_entry(1, XrefEntry::InUse { offset: 100, gen_nr: 0 });
        assert_eq!(section.len(), 1);
        assert!(section.entries.contains_key(&1));
    }

    #[test]
    fn test_xref_section_default() {
        let section = XrefSection::default();
        assert!(section.is_empty());
        assert!(section.trailer.is_none());
        assert!(section.diagnostics.is_empty());
    }

    #[test]
    fn test_xref_entry_in_use() {
        let entry = XrefEntry::InUse { offset: 1000, gen_nr: 5 };
        assert!(matches!(entry, XrefEntry::InUse { offset: 1000, gen_nr: 5 }));
    }

    #[test]
    fn test_xref_entry_free() {
        let entry = XrefEntry::Free { next_free: 42, gen_nr: 1 };
        assert!(matches!(entry, XrefEntry::Free { next_free: 42, gen_nr: 1 }));
    }

    #[test]
    fn test_xref_entry_compressed() {
        let entry = XrefEntry::Compressed { obj_stm_nr: 10, index: 5 };
        assert!(matches!(entry, XrefEntry::Compressed { obj_stm_nr: 10, index: 5 }));
    }

    #[test]
    fn test_xref_resolver_from_section() {
        let mut section = XrefSection::new();
        section.add_entry(1, XrefEntry::InUse { offset: 100, gen_nr: 0 });
        section.add_entry(2, XrefEntry::InUse { offset: 200, gen_nr: 0 });

        let resolver = XrefResolver::from_section(section);
        assert_eq!(resolver.len(), 2);
        assert_eq!(resolver.get_entry(1), Some(&XrefEntry::InUse { offset: 100, gen_nr: 0 }));
        assert_eq!(resolver.get_entry(2), Some(&XrefEntry::InUse { offset: 200, gen_nr: 0 }));
    }

    #[test]
    fn test_xref_diagnostic_static() {
        let diag = XrefDiagnostic::with_static(
            XrefDiagCode::InvalidXrefHeader,
            100,
            "test message",
        );
        assert_eq!(diag.byte_offset, 100);
        assert_eq!(diag.msg.as_ref(), "test message");
        assert!(matches!(diag.code, XrefDiagCode::InvalidXrefHeader));
    }

    #[test]
    fn test_xref_diagnostic_dynamic() {
        let diag = XrefDiagnostic::with_dynamic(
            XrefDiagCode::InvalidXrefEntry,
            200,
            "dynamic message".to_string(),
        );
        assert_eq!(diag.byte_offset, 200);
        assert_eq!(diag.msg.as_ref(), "dynamic message");
        assert!(matches!(diag.code, XrefDiagCode::InvalidXrefEntry));
    }

    #[test]
    fn test_parse_simple_xref_space_newline() {
        // Well-formed xref with standard " \n" line endings (20-byte entries)
        let xref_data = b"xref\n0 6\n\
0000000000 65535 f \n\
0000000017 00000 n \n\
0000000081 00000 n \n\
0000000000 00007 f \n\
0000000331 00000 n \n\
0000000409 00000 n \n\
trailer\n<< /Size 6 >>\n";

        let source = MemorySource::new(xref_data.to_vec());
        let result = parse_traditional_xref(&source, 0);

        // Should have parsed 5 in-use entries (object 0 is free and ignored)
        assert_eq!(result.len(), 5);

        // Check specific entries
        assert_eq!(result.entries.get(&1), Some(&XrefEntry::InUse { offset: 17, gen_nr: 0 }));
        assert_eq!(result.entries.get(&2), Some(&XrefEntry::InUse { offset: 81, gen_nr: 0 }));
        assert_eq!(result.entries.get(&4), Some(&XrefEntry::InUse { offset: 331, gen_nr: 0 }));
        assert_eq!(result.entries.get(&5), Some(&XrefEntry::InUse { offset: 409, gen_nr: 0 }));

        // Trailer should be present (empty dict for now)
        assert!(result.trailer.is_some());
    }

    #[test]
    fn test_parse_xref_carriage_return_newline() {
        // Xref with \r\n line endings (20-byte entries)
        let xref_data = b"xref\r\n0 3\r\n\
0000000000 65535 f\r\n\
0000000015 00000 n\r\n\
0000000078 00000 n\r\n\
trailer\r\n<< /Size 3 >>\r\n";

        let source = MemorySource::new(xref_data.to_vec());
        let result = parse_traditional_xref(&source, 0);

        // Should have parsed 2 in-use entries
        assert_eq!(result.len(), 2);
        assert_eq!(result.entries.get(&1), Some(&XrefEntry::InUse { offset: 15, gen_nr: 0 }));
        assert_eq!(result.entries.get(&2), Some(&XrefEntry::InUse { offset: 78, gen_nr: 0 }));
    }

    #[test]
    fn test_parse_xref_lf_only_19_byte_entries() {
        // Xref with bare \n (buggy producer, 19-byte entries)
        let xref_data = b"xref\n0 3\n\
0000000000 65535 f\n\
0000000015 00000 n\n\
0000000078 00000 n\n\
trailer\n<< /Size 3 >>\n";

        let source = MemorySource::new(xref_data.to_vec());
        let result = parse_traditional_xref(&source, 0);

        // Should have parsed 2 in-use entries
        assert_eq!(result.len(), 2);
        assert_eq!(result.entries.get(&1), Some(&XrefEntry::InUse { offset: 15, gen_nr: 0 }));
        assert_eq!(result.entries.get(&2), Some(&XrefEntry::InUse { offset: 78, gen_nr: 0 }));
    }

    #[test]
    fn test_parse_multi_subsection_xref() {
        // Xref with two subsections: 0 3 and 100 2
        let xref_data = b"xref\n0 3\n\
0000000000 65535 f \n\
0000000015 00000 n \n\
0000000078 00000 n \n\
100 2\n\
0000000200 00000 n \n\
0000000300 00000 n \n\
trailer\n<< /Size 102 >>\n";

        let source = MemorySource::new(xref_data.to_vec());
        let result = parse_traditional_xref(&source, 0);

        // Should have parsed 4 in-use entries (1, 2, 100, 101)
        assert_eq!(result.len(), 4);
        assert!(result.entries.contains_key(&1));
        assert!(result.entries.contains_key(&2));
        assert!(result.entries.contains_key(&100));
        assert!(result.entries.contains_key(&101));

        // Check offset for object 100
        assert_eq!(result.entries.get(&100), Some(&XrefEntry::InUse { offset: 200, gen_nr: 0 }));
        assert_eq!(result.entries.get(&101), Some(&XrefEntry::InUse { offset: 300, gen_nr: 0 }));
    }

    #[test]
    fn test_parse_xref_with_malformed_entry() {
        // Xref with one malformed entry in the middle
        let xref_data = b"xref\n0 4\n\
0000000000 65535 f \n\
0000000015 00000 n \n\
BAD_ENTRY_BAD n \n\
0000000078 00000 n \n\
trailer\n<< /Size 4 >>\n";

        let source = MemorySource::new(xref_data.to_vec());
        let result = parse_traditional_xref(&source, 0);

        // Should have parsed at least the valid entry
        assert!(result.len() >= 1);
        assert_eq!(result.entries.get(&1), Some(&XrefEntry::InUse { offset: 15, gen_nr: 0 }));

        // Should have emitted a diagnostic for the bad entry
        assert!(!result.diagnostics.is_empty());
        assert!(result.diagnostics.iter().any(|d| d.code == XrefDiagCode::InvalidXrefEntry));
    }

    #[test]
    fn test_parse_xref_object_zero_not_free() {
        // Xref where object 0 is not free (violates PDF spec)
        let xref_data = b"xref\n0 3\n\
0000000015 00000 n \n\
0000000015 00000 n \n\
0000000078 00000 n \n\
trailer\n<< /Size 3 >>\n";

        let source = MemorySource::new(xref_data.to_vec());
        let result = parse_traditional_xref(&source, 0);

        // Should emit diagnostic for object 0 not being free
        assert!(result.diagnostics.iter().any(|d| d.code == XrefDiagCode::ObjectZeroNotFree));
    }

    #[test]
    fn test_parse_xref_missing_trailer() {
        // Xref without trailer (truncated)
        let xref_data = b"xref\n0 2\n\
0000000000 65535 f \n\
0000000015 00000 n \n";

        let source = MemorySource::new(xref_data.to_vec());
        let result = parse_traditional_xref(&source, 0);

        // Should still parse the entry
        assert_eq!(result.len(), 1);
        assert!(result.trailer.is_none());

        // Should emit diagnostic about missing trailer
        assert!(result.diagnostics.iter().any(|d| d.code == XrefDiagCode::TrailerNotFound));
    }

    #[test]
    fn test_read_line_simple() {
        let data = b"Hello World\nNext line";
        let source = MemorySource::new(data.to_vec());
        let mut pos = 0;
        let diagnostics = &mut Vec::new();

        let line = read_line(&source, &mut pos, diagnostics).unwrap();
        assert_eq!(line, "Hello World");

        let line2 = read_line(&source, &mut pos, diagnostics).unwrap();
        assert_eq!(line2, "Next line");
    }

    #[test]
    fn test_read_line_with_crlf() {
        let data = b"Hello World\r\nNext line";
        let source = MemorySource::new(data.to_vec());
        let mut pos = 0;
        let diagnostics = &mut Vec::new();

        let line = read_line(&source, &mut pos, diagnostics).unwrap();
        assert_eq!(line, "Hello World");

        let line2 = read_line(&source, &mut pos, diagnostics).unwrap();
        assert_eq!(line2, "Next line");
    }

    #[test]
    fn test_parse_xref_entry_20_byte() {
        let entry = b"0000000015 00000 n \n";
        let diagnostics = &mut Vec::new();

        let result = parse_xref_entry(entry, 1, 100, 20, diagnostics);
        assert_eq!(result, Some((1, XrefEntry::InUse { offset: 15, gen_nr: 0 })));
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_parse_xref_entry_free() {
        let entry = b"0000000000 65535 f \n";
        let diagnostics = &mut Vec::new();

        let result = parse_xref_entry(entry, 0, 100, 20, diagnostics);
        assert_eq!(result, Some((0, XrefEntry::Free { next_free: 0, gen_nr: 65535 })));
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_parse_xref_entry_malformed() {
        let entry = b"BAD_ENTRY_BAD n \n";
        let diagnostics = &mut Vec::new();

        let result = parse_xref_entry(entry, 1, 100, 20, diagnostics);
        assert!(result.is_none());
        assert!(!diagnostics.is_empty());
    }

    // proptest for random byte sequences - never panic
    #[cfg(feature = "proptest")]
    mod proptest_tests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn proptest_random_bytes_no_panic(data in any::<Vec<u8>>()) {
                // Any random byte sequence should not panic
                let source = MemorySource::new(data.clone());
                let _ = parse_traditional_xref(&source, 0);
                // If we get here without panic, the test passes
            }

            #[test]
            fn proptest_random_offset_no_panic(
                data in any::<Vec<u8>>(),
                offset in any::<u64>()
            ) {
                // Any random offset should not panic
                let source = MemorySource::new(data);
                let _ = parse_traditional_xref(&source, offset);
                // If we get here without panic, the test passes
            }
        }
    }
}
