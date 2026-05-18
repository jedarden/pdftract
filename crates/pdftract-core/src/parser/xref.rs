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
use crate::parser::stream::{PdfSource, MemorySource};

// Use memchr for SIMD-accelerated byte searching in forward_scan_xref
use memchr::{memchr, memchr_iter};

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
    /// Forward scan recovered xref entries (EC-07 recovery)
    XrefRepaired,
    /// Forward scan disabled for remote sources (would fetch entire file)
    RemoteNoForwardScan,
    /// Forward scan disabled for linearized files (has partial leading xref)
    LinearizedNoForwardScan,
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
        self.resolving.read()
            .map(|guard| guard.contains(&obj_ref))
            .unwrap_or(false)
    }

    /// Mark an object as being resolved.
    pub fn start_resolving(&self, obj_ref: ObjRef) -> bool {
        match self.resolving.write() {
            Ok(mut resolving) => {
                if resolving.contains(&obj_ref) {
                    return false;
                }
                resolving.insert(obj_ref);
                true
            }
            Err(_) => false, // Lock poisoned - treat as failed to start
        }
    }

    /// Mark an object as finished resolving.
    pub fn finish_resolving(&self, obj_ref: ObjRef) {
        if let Ok(mut resolving) = self.resolving.write() {
            resolving.remove(&obj_ref);
        }
        // If lock is poisoned, ignore - cleanup is optional
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
            match self.cache.read() {
                Ok(cache) => {
                    if let Some(obj) = cache.get(&obj_ref) {
                        self.finish_resolving(obj_ref);
                        return Ok(obj.clone());
                    }
                }
                Err(_) => {
                    // Lock poisoned - clear the poisoned state and continue
                    // The cache is optional, so we can proceed without it
                }
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
        if let Ok(mut cache) = self.cache.write() {
            cache.insert(obj_ref, obj);
        }
        // If lock is poisoned, ignore - caching is optional
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
    // Find it in the raw bytes, accounting for leading whitespace
    let xref_keyword_pos = loop {
        let header_str = match std::str::from_utf8(&header_bytes) {
            Ok(s) => s,
            Err(_) => {
                result.diagnostics.push(XrefDiagnostic::with_static(
                    XrefDiagCode::InvalidXrefHeader,
                    pos,
                    "Invalid UTF-8 in xref header",
                ));
                return result;
            }
        };

        // Skip leading whitespace to find xref
        let trimmed = header_str.trim_start();
        let ws_offset = header_str.len() - trimmed.len();

        if trimmed.starts_with("xref") {
            // Found it! ws_offset is the position of "xref" in header_bytes
            break ws_offset;
        } else {
            result.diagnostics.push(XrefDiagnostic::with_static(
                XrefDiagCode::InvalidXrefHeader,
                pos,
                "xref keyword not found",
            ));
            return result;
        }
    };

    // Advance past "xref" keyword (4 bytes) to the byte after it
    pos += xref_keyword_pos as u64 + 4;

    // Skip the line ending after "xref" (could be \n, \r\n, or \r)
    let line_end_bytes = source.read_at(pos, 2).ok();
    if let Some(chunk) = line_end_bytes {
        if chunk.get(0) == Some(&b'\r') {
            if chunk.get(1) == Some(&b'\n') {
                pos += 2; // CRLF
            } else {
                pos += 1; // CR alone
            }
        } else if chunk.get(0) == Some(&b'\n') {
            pos += 1; // LF alone
        }
        // If no line ending found, continue anyway (might be EOF or next subsection)
    }

    // Track whether we found the trailer keyword
    let mut trailer_found = false;

    // Parse subsections until we hit "trailer"
    loop {
        // Read a chunk to check for trailer or subsection header
        let chunk_bytes = match source.read_at(pos, 100) {
            Ok(bytes) if !bytes.is_empty() => bytes,
            _ => {
                // EOF or error - we're done
                break;
            }
        };

        let chunk_str = match std::str::from_utf8(&chunk_bytes) {
            Ok(s) => s,
            Err(_) => {
                result.diagnostics.push(XrefDiagnostic::with_static(
                    XrefDiagCode::XrefTruncated,
                    pos,
                    "Invalid UTF-8 in xref data",
                ));
                break;
            }
        };

        let trimmed = chunk_str.trim_start();
        let ws_offset = chunk_str.len() - trimmed.len();

        // Check for trailer keyword
        if trimmed.starts_with("trailer") {
            trailer_found = true;
            pos += ws_offset as u64 + 7; // Skip "trailer"
            result.trailer = parse_trailer_dict(source, &mut pos, &mut result.diagnostics);
            break;
        }

        // Otherwise, expect subsection header: "obj_start obj_count"
        let subsection_start = pos + ws_offset as u64;
        let header_line = match read_line_at(source, subsection_start) {
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
            // Skip this line and try to continue
            // Find the line ending length
            let line_bytes = source.read_at(subsection_start, header_line.len() + 2).ok();
            let line_ending_len = if let Some(chunk) = line_bytes {
                if chunk.get(header_line.len()) == Some(&b'\r') {
                    if chunk.get(header_line.len() + 1) == Some(&b'\n') { 2 } else { 1 }
                } else if chunk.get(header_line.len()) == Some(&b'\n') {
                    1
                } else {
                    1 // assume at least 1 byte for line ending
                }
            } else {
                1
            };
            pos = subsection_start + header_line.len() as u64 + line_ending_len as u64;
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
                pos = subsection_start + header_line.len() as u64 + 1;
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
                pos = subsection_start + header_line.len() as u64 + 1;
                continue;
            }
        };

        // Position advances past the subsection header line (including line ending)
        // Find the line ending length
        let line_bytes = source.read_at(subsection_start, header_line.len() + 2).ok();
        let line_ending_len = if let Some(chunk) = line_bytes {
            if chunk.get(header_line.len()) == Some(&b'\r') {
                if chunk.get(header_line.len() + 1) == Some(&b'\n') { 2 } else { 1 }
            } else if chunk.get(header_line.len()) == Some(&b'\n') {
                1
            } else {
                1 // assume at least 1 byte for line ending
            }
        } else {
            1
        };
        pos = subsection_start + header_line.len() as u64 + line_ending_len as u64;

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
                    // Only add in-use entries to the result
                    // Free entries are ignored per pdftract spec (they don't resolve to objects)
                    if matches!(entry, XrefEntry::InUse { .. }) {
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

    // If we exited the loop without finding a trailer, emit a diagnostic
    if !trailer_found {
        result.diagnostics.push(XrefDiagnostic::with_static(
            XrefDiagCode::TrailerNotFound,
            pos,
            "Trailer dictionary not found (xref table may be truncated)",
        ));
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

/// Read a line from the source at a specific position (without updating position).
///
/// Returns None on EOF or error.
fn read_line_at(source: &dyn PdfSource, mut pos: u64) -> Option<String> {
    let mut result = String::new();
    let mut chunk_pos = 0;
    let chunk_size = 256;

    loop {
        let chunk = source.read_at(pos + chunk_pos, chunk_size).ok()?;
        if chunk.is_empty() {
            break;
        }

        // Look for line ending
        for (i, &byte) in chunk.iter().enumerate() {
            if byte == b'\r' {
                // Check for CRLF
                if i + 1 < chunk.len() && chunk[i + 1] == b'\n' {
                    result.push_str(std::str::from_utf8(&chunk[..i]).ok()?);
                    return Some(result);
                }
                // Single CR
                result.push_str(std::str::from_utf8(&chunk[..i]).ok()?);
                return Some(result);
            }
            if byte == b'\n' {
                // Single LF
                result.push_str(std::str::from_utf8(&chunk[..i]).ok()?);
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
        Some(result)
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
    let line = read_line_at(source, *pos)?;
    // Advance position past the line (including line ending)
    // We need to find the actual line ending length
    let chunk = source.read_at(*pos, line.len() + 2).ok()?;
    let line_ending_len = if chunk.get(line.len()) == Some(&b'\r') {
        if chunk.get(line.len() + 1) == Some(&b'\n') {
            2 // CRLF
        } else {
            1 // CR alone
        }
    } else if chunk.get(line.len()) == Some(&b'\n') {
        1 // LF alone
    } else {
        0 // No line ending found (shouldn't happen)
    };
    *pos += line.len() as u64 + line_ending_len as u64;
    Some(line)
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

/// Perform a forward-scan xref recovery (strategy 4 - last resort).
///
/// When all other xref strategies fail, this scans the entire file byte-by-byte
/// looking for indirect-object header patterns (`N G obj`) and builds an xref
/// map from those discoveries.
///
/// # Parameters
/// - `source`: The PDF source to scan
/// - `is_linearized`: If true, forward scan is disabled for linearized files
///
/// # Returns
/// An `XrefSection` containing recovered entries and diagnostics.
///
/// # DISABLED CONDITIONS
/// - **Remote sources**: Would require fetching the entire file. Returns empty
///   XrefSection with `STRUCT_REMOTE_NO_FORWARD_SCAN` diagnostic.
/// - **Linearized files**: Would find the partial first-page xref and incorrectly
///   stop. Returns empty XrefSection with `LINEARIZED_NO_FORWARD_SCAN` diagnostic.
///
/// # Algorithm
/// 1. Use SIMD-optimized search (via `memchr`) to find ` obj` substrings
/// 2. For each candidate, verify preceding bytes match `\d+ \d+ `
/// 3. Parse N (object number) and G (generation number)
/// 4. Record `XrefEntry::InUse { offset, generation }` for each match
/// 5. Forward-scan for the `trailer` keyword and parse the following dict
/// 6. Emit `XREF_REPAIRED` diagnostic with count of recovered objects
///
/// # Performance
/// - O(file_size) time complexity
/// - Expected: ~1 sec for 100 MB on a fast machine
/// - Memory: builds HashMap incrementally; no full-file buffer needed
///
/// # Multi-revision handling
/// - Files with multiple trailer blocks (incremental updates): LAST trailer wins
/// - For each ObjRef, the LAST occurrence in the file wins (highest offset)
pub fn forward_scan_xref(source: &dyn PdfSource, is_linearized: bool) -> XrefSection {
    let mut result = XrefSection::new();

    // Check for linearized file
    if is_linearized {
        result.diagnostics.push(XrefDiagnostic::with_static(
            XrefDiagCode::LinearizedNoForwardScan,
            0,
            "Forward scan disabled for linearized PDF (partial leading xref would cause false results)",
        ));
        return result;
    }

    // TODO: Check for remote source (HttpRangeSource) when implemented
    // For now, MemorySource and FileSource are both local sources
    // Once HttpRangeSource exists, add a trait method like `is_remote()` to PdfSource

    let source_len = match source.len() {
        Ok(len) if len > 0 => len,
        _ => {
            result.diagnostics.push(XrefDiagnostic::with_static(
                XrefDiagCode::XrefTruncated,
                0,
                "Unable to determine source length for forward scan",
            ));
            return result;
        }
    };

    // For large files, use memchr for efficient scanning
    // For smaller files, read entirely into memory for faster processing
    const SMALL_FILE_THRESHOLD: u64 = 1024 * 1024; // 1 MB

    if source_len <= SMALL_FILE_THRESHOLD {
        // Small file: read entirely and scan in memory
        if let Ok(full_data) = source.read_at(0, source_len as usize) {
            return forward_scan_memory(&full_data, source_len);
        }
    }

    // Large file: scan in chunks using memchr for efficient space searching
    let mut entries_found = 0u64;
    const CHUNK_SIZE: usize = 256 * 1024; // 256 KB chunks

    // We search for the pattern " obj" (space followed by "obj")
    // First, find all space positions, then verify if "obj" follows
    let mut pos = 0u64;

    while pos < source_len {
        let to_read = CHUNK_SIZE.min((source_len - pos) as usize);

        match source.read_at(pos, to_read) {
            Ok(chunk) if !chunk.is_empty() => {
                // Use memchr_iter for SIMD-accelerated space search
                let chunk_offset = pos;
                for space_idx in memchr_iter(b' ', &chunk) {
                    let abs_space_idx = space_idx as u64;

                    // Check if "obj" follows this space
                    if space_idx + 4 <= chunk.len() {
                        let after_space = &chunk[space_idx..];
                        if after_space.starts_with(b"obj") {
                            // Found " obj" - verify whitespace after "obj"
                            let obj_end = space_idx + 3;
                            let has_trailing_ws = if obj_end < chunk.len() {
                                let next = chunk[obj_end];
                                next == b'\n' || next == b'\r' || next == b' ' || next == b'\t'
                            } else {
                                // At chunk boundary - check next chunk for this rare case
                                check_trailing_whitespace(source, chunk_offset + abs_space_idx + 3, source_len)
                            };

                            if has_trailing_ws {
                                let obj_offset = chunk_offset + abs_space_idx;
                                if let Some((obj_num, gen_num)) = parse_obj_header_at(source, obj_offset) {
                                    result.entries.insert(obj_num, XrefEntry::InUse {
                                        offset: obj_offset,
                                        gen_nr: gen_num,
                                    });
                                    entries_found += 1;
                                }
                            }
                        }
                    }
                }

                pos += to_read as u64;
                // Slide back to catch " obj" spanning chunk boundaries
                pos = pos.saturating_sub(3);
            }
            Err(_) => break,
            Ok(_) => break, // Empty chunk
        }
    }

    // Forward-scan for the trailer dictionary
    if let Some(trailer) = forward_scan_trailer(source) {
        result.trailer = Some(trailer);
    }

    // Emit XREF_REPAIRED diagnostic with count
    result.diagnostics.push(XrefDiagnostic::with_dynamic(
        XrefDiagCode::XrefRepaired,
        0,
        format!("Forward scan recovered {} object entries", entries_found),
    ));

    result
}

/// Check for trailing whitespace after "obj" at the given offset.
///
/// This is used when "obj" appears at a chunk boundary and we need to
/// verify the next byte in the file.
fn check_trailing_whitespace(source: &dyn PdfSource, offset: u64, source_len: u64) -> bool {
    if offset >= source_len {
        return false;
    }
    match source.read_at(offset, 1) {
        Ok(bytes) if !bytes.is_empty() => {
            let next = bytes[0];
            next == b'\n' || next == b'\r' || next == b' ' || next == b'\t'
        }
        _ => false,
    }
}

/// Forward-scan a memory buffer for xref entries.
///
/// This is a specialized version for small files that can be entirely
/// loaded into memory. Uses memchr for efficient scanning.
fn forward_scan_memory(data: &[u8], source_len: u64) -> XrefSection {
    let mut result = XrefSection::new();
    let mut entries_found = 0u64;

    // Use memchr_iter for SIMD-accelerated space search
    for space_idx in memchr_iter(b' ', data) {
        let abs_space_idx = space_idx as u64;

        // Check if "obj" follows this space
        if space_idx + 4 <= data.len() {
            let after_space = &data[space_idx..];
            if after_space.starts_with(b"obj") {
                // Verify whitespace after "obj"
                let obj_end = space_idx + 3;
                let has_trailing_ws = if obj_end < data.len() {
                    let next = data[obj_end];
                    next == b'\n' || next == b'\r' || next == b' ' || next == b'\t'
                } else {
                    // At EOF - still valid
                    true
                };

                if has_trailing_ws {
                    let obj_offset = abs_space_idx;
                    if let Some((obj_num, gen_num)) = parse_obj_header_at_memory(data, obj_offset) {
                        result.entries.insert(obj_num, XrefEntry::InUse {
                            offset: obj_offset,
                            gen_nr: gen_num,
                        });
                        entries_found += 1;
                    }
                }
            }
        }
    }

    // Emit XREF_REPAIRED diagnostic with count
    result.diagnostics.push(XrefDiagnostic::with_dynamic(
        XrefDiagCode::XrefRepaired,
        0,
        format!("Forward scan recovered {} object entries", entries_found),
    ));

    result
}

/// Parse the object number and generation number from bytes preceding " obj".
///
/// Scans backwards from the given offset (which points to the space before "obj")
/// to find the pattern `\d+ \d+ ` (digits space digits space).
///
/// Returns Some((object_number, generation_number)) if found, None otherwise.
fn parse_obj_header_at(source: &dyn PdfSource, obj_offset: u64) -> Option<(u32, u16)> {
    // Scan backwards to find the start of the pattern
    // Max lookback: 20 bytes for "9999999999 65535 " (max valid per spec)
    const MAX_LOOKBACK: usize = 30;

    let lookback_start = obj_offset.saturating_sub(MAX_LOOKBACK as u64);
    let lookback_len = (obj_offset - lookback_start) as usize;

    let chunk = source.read_at(lookback_start, lookback_len).ok()?;

    // We're looking for: <digits> <space> <digits> <space> obj
    // Work backwards from the end
    let mut idx = chunk.len();

    // Skip trailing space (the one before "obj")
    if idx == 0 || chunk[idx - 1] != b' ' {
        return None;
    }
    idx -= 1;

    // Parse generation number (digits going backwards)
    let gen_end = idx;
    while idx > 0 && chunk[idx - 1].is_ascii_digit() {
        idx -= 1;
    }
    if idx == gen_end {
        return None; // No digits found
    }
    let gen_str = std::str::from_utf8(&chunk[idx..gen_end]).ok()?;
    let gen_num: u16 = gen_str.parse().ok()?;

    // Check for space before generation number
    if idx == 0 || chunk[idx - 1] != b' ' {
        return None;
    }
    idx -= 1;

    // Parse object number (digits going backwards)
    let obj_end = idx;
    while idx > 0 && chunk[idx - 1].is_ascii_digit() {
        idx -= 1;
    }
    if idx == obj_end {
        return None; // No digits found
    }
    let obj_str = std::str::from_utf8(&chunk[idx..obj_end]).ok()?;
    let obj_num: u32 = obj_str.parse().ok()?;

    // Validate: object number should be preceded by start-of-buffer or whitespace
    if idx > 0 {
        let prev = chunk[idx - 1];
        if !prev.is_ascii_whitespace() && prev != b'%' && prev != b'(' && prev != b'<' {
            // Not a valid token boundary
            return None;
        }
    }

    Some((obj_num, gen_num))
}

/// Parse the object number and generation number from a memory buffer.
///
/// This is a variant of `parse_obj_header_at` that works directly with
/// a byte slice instead of a PdfSource, for use with memory-mapped data.
///
/// Scans backwards from the given offset (which points to the space before "obj")
/// to find the pattern `\d+ \d+ ` (digits space digits space).
///
/// Returns Some((object_number, generation_number)) if found, None otherwise.
fn parse_obj_header_at_memory(data: &[u8], obj_offset: u64) -> Option<(u32, u16)> {
    // Scan backwards to find the start of the pattern
    // Max lookback: 20 bytes for "9999999999 65535 " (max valid per spec)
    const MAX_LOOKBACK: usize = 30;

    let lookback_start = obj_offset.saturating_sub(MAX_LOOKBACK as u64) as usize;
    let lookback_len = (obj_offset as usize).saturating_sub(lookback_start);

    let chunk = data.get(lookback_start..(lookback_start + lookback_len))?;

    // We're looking for: <digits> <space> <digits> <space> obj
    // Work backwards from the end
    let mut idx = chunk.len();

    // Skip trailing space (the one before "obj")
    if idx == 0 || chunk[idx - 1] != b' ' {
        return None;
    }
    idx -= 1;

    // Parse generation number (digits going backwards)
    let gen_end = idx;
    while idx > 0 && chunk[idx - 1].is_ascii_digit() {
        idx -= 1;
    }
    if idx == gen_end {
        return None; // No digits found
    }
    let gen_str = std::str::from_utf8(&chunk[idx..gen_end]).ok()?;
    let gen_num: u16 = gen_str.parse().ok()?;

    // Check for space before generation number
    if idx == 0 || chunk[idx - 1] != b' ' {
        return None;
    }
    idx -= 1;

    // Parse object number (digits going backwards)
    let obj_end = idx;
    while idx > 0 && chunk[idx - 1].is_ascii_digit() {
        idx -= 1;
    }
    if idx == obj_end {
        return None; // No digits found
    }
    let obj_str = std::str::from_utf8(&chunk[idx..obj_end]).ok()?;
    let obj_num: u32 = obj_str.parse().ok()?;

    // Validate: object number should be preceded by start-of-buffer or whitespace
    if idx > 0 {
        let prev = chunk[idx - 1];
        if !prev.is_ascii_whitespace() && prev != b'%' && prev != b'(' && prev != b'<' {
            // Not a valid token boundary
            return None;
        }
    }

    Some((obj_num, gen_num))
}

/// Forward-scan for the trailer dictionary.
///
/// Searches the file for the `trailer` keyword (also handles `trailer<<` with no space)
/// and parses the following dictionary.
///
/// Returns Some(PdfDict) if found, None otherwise.
fn forward_scan_trailer(source: &dyn PdfSource) -> Option<PdfDict> {
    let source_len = source.len().ok()?;
    const TRAILER_KEYWORD: &[u8] = b"trailer";

    // Read from the end of the file backwards (trailer is usually near the end)
    // Check last 64KB first
    let scan_start = source_len.saturating_sub(64 * 1024);
    let mut pos = scan_start;

    while pos < source_len {
        let to_read = 4096.min((source_len - pos) as usize);
        let chunk = source.read_at(pos, to_read).ok()?;

        // Search for "trailer" in this chunk
        if let Some(idx) = chunk.windows(TRAILER_KEYWORD.len()).position(|w| w == TRAILER_KEYWORD) {
            let trailer_offset = pos + idx as u64;

            // Verify it's at a token boundary (preceded by whitespace or start)
            let valid_boundary = if idx > 0 {
                chunk[idx - 1].is_ascii_whitespace() || chunk[idx - 1] == b'\n' || chunk[idx - 1] == b'\r'
            } else {
                pos == scan_start // At start of scan area
            };

            if valid_boundary {
                // Parse the trailer dictionary
                let mut dict_pos = trailer_offset + TRAILER_KEYWORD.len() as u64;
                // Skip whitespace before <<
                while dict_pos < source_len {
                    let byte = source.read_at(dict_pos, 1).ok()?;
                    if !byte.is_empty() && byte[0].is_ascii_whitespace() {
                        dict_pos += 1;
                    } else {
                        break;
                    }
                }
                // Try to parse the dict - for now return empty dict
                // Full implementation would use the object parser
                return Some(PdfDict::new());
            }
        }

        pos += to_read as u64;
        // Slide back to catch matches spanning boundaries
        pos = pos.saturating_sub((TRAILER_KEYWORD.len() - 1) as u64);
    }

    None
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

        // Should have parsed 4 in-use entries (objects 0 and 3 are free and ignored)
        assert_eq!(result.len(), 4);

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
        // 19-byte malformed entry (invalid offset format)
        let entry = b"BADENTRIES 00000 n\n";
        let diagnostics = &mut Vec::new();

        // Test with 19-byte stride to match the actual length
        let result = parse_xref_entry(entry, 1, 100, 19, diagnostics);
        assert!(result.is_none());
        assert!(!diagnostics.is_empty());
    }

    // proptest for random byte sequences - never panic
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

            #[test]
            fn proptest_forward_scan_no_panic(data in any::<Vec<u8>>()) {
                // Random byte sequences should never panic forward_scan_xref
                let source = MemorySource::new(data);
                let _ = forward_scan_xref(&source, false);
                // If we get here without panic, the test passes
            }

            #[test]
            fn proptest_forward_scan_linearized_no_panic(data in any::<Vec<u8>>()) {
                // Random byte sequences with linearized flag should never panic
                let source = MemorySource::new(data);
                let _ = forward_scan_xref(&source, true);
                // If we get here without panic, the test passes
            }
        }
    }

    // Forward scan tests

    #[test]
    fn test_forward_scan_simple() {
        // Simple PDF with a few indirect objects
        let pdf_data = b"1 0 obj\n<< /Type /Catalog >>\nendobj\n\
                          2 0 obj\n<< /Type /Pages >>\nendobj\n\
                          3 0 obj\n<< /Type /Page >>\nendobj\n";

        let source = MemorySource::new(pdf_data.to_vec());
        let result = forward_scan_xref(&source, false);

        // Should have found all 3 objects
        assert_eq!(result.len(), 3);
        assert!(result.entries.contains_key(&1));
        assert!(result.entries.contains_key(&2));
        assert!(result.entries.contains_key(&3));

        // Check for XREF_REPAIRED diagnostic
        assert!(result.diagnostics.iter().any(|d| d.code == XrefDiagCode::XrefRepaired));
    }

    #[test]
    fn test_forward_scan_with_generations() {
        // PDF with different generation numbers
        let pdf_data = b"1 0 obj\n<< /Type /Catalog >>\nendobj\n\
                          2 5 obj\n<< /Type /Pages >>\nendobj\n\
                          3 65535 obj\n<< /Type /Page >>\nendobj\n";

        let source = MemorySource::new(pdf_data.to_vec());
        let result = forward_scan_xref(&source, false);

        assert_eq!(result.len(), 3);

        // Check generation numbers
        assert_eq!(result.entries.get(&1), Some(&XrefEntry::InUse { offset: 0, gen_nr: 0 }));
        assert_eq!(result.entries.get(&2), Some(&XrefEntry::InUse { offset: 35, gen_nr: 5 }));
        assert_eq!(result.entries.get(&3), Some(&XrefEntry::InUse { offset: 70, gen_nr: 65535 }));
    }

    #[test]
    fn test_forward_scan_linearized_disabled() {
        // Forward scan should be disabled for linearized files
        let pdf_data = b"1 0 obj\n<< /Type /Catalog >>\nendobj\n";

        let source = MemorySource::new(pdf_data.to_vec());
        let result = forward_scan_xref(&source, true); // is_linearized = true

        // Should have no entries
        assert_eq!(result.len(), 0);

        // Should have LINEARIZED_NO_FORWARD_SCAN diagnostic
        assert!(result.diagnostics.iter().any(|d| d.code == XrefDiagCode::LinearizedNoForwardScan));
    }

    #[test]
    fn test_forward_scan_truncated_file() {
        // Critical test: file truncated after xref
        // Forward scan should find all objects before truncation point
        let pdf_data = b"1 0 obj\n<< /Type /Catalog >>\nendobj\n\
                          2 0 obj\n<< /Type /Pages >>\nendobj\n\
                          3 0 obj\n<< /Type /Page >>\nendobj\n\
                          xref\n\
                          0 4\n\
                          0000000000 65535 f \n\
                          0000000009 00000 n \n\
                          0000000045 00000 n \n\
                          0000000081 00000 n \n\
                          trailer\n\
                          << /Size 4 >>\n\
                          startxref\n\
                          117\n\
                          %%EOF\n\
                          4 0 obj\n\
                          << /Type /Outlines >>\n\
                          endobj\n";

        let source = MemorySource::new(pdf_data.to_vec());
        let result = forward_scan_xref(&source, false);

        // Should find all 4 objects (including the one after the truncated xref)
        assert_eq!(result.len(), 4);

        // Verify offsets are correct
        assert!(result.entries.get(&1).is_some());
        assert!(result.entries.get(&2).is_some());
        assert!(result.entries.get(&3).is_some());
        assert!(result.entries.get(&4).is_some());
    }

    #[test]
    fn test_forward_scan_with_trailer() {
        // PDF with trailer keyword
        let pdf_data = b"1 0 obj\n<< /Type /Catalog >>\nendobj\n\
                          2 0 obj\n<< /Type /Pages >>\nendobj\n\
                          trailer\n\
                          << /Size 3 >>\n\
                          3 0 obj\n\
                          << /Type /Page >>\nendobj\n";

        let source = MemorySource::new(pdf_data.to_vec());
        let result = forward_scan_xref(&source, false);

        // Should have found all 3 objects
        assert_eq!(result.len(), 3);

        // Should have found a trailer (even if empty for now)
        assert!(result.trailer.is_some());
    }

    #[test]
    fn test_forward_scan_multi_revision() {
        // Test multi-revision handling: later occurrences override earlier ones
        let pdf_data = b"1 0 obj\n<< /Type /Catalog /V 1 >>\nendobj\n\
                          2 0 obj\n<< /Type /Pages >>\nendobj\n\
                          1 0 obj\n<< /Type /Catalog /V 2 >>\nendobj\n";

        let source = MemorySource::new(pdf_data.to_vec());
        let result = forward_scan_xref(&source, false);

        // Should have 2 entries (object 1 and 2)
        assert_eq!(result.len(), 2);

        // Object 1 should point to the SECOND occurrence (higher offset)
        let entry1 = result.entries.get(&1);
        assert!(entry1.is_some());
        // The second "1 0 obj" is at offset 70 (after first two objects)
        if let Some(XrefEntry::InUse { offset, .. }) = entry1 {
            assert!(*offset > 50);
        } else {
            panic!("Expected InUse entry");
        }
    }

    #[test]
    fn test_forward_scan_false_positive_handling() {
        // Test that false positives (like "5 0 obj" in a string) are handled
        // The forward scan may find them, but they won't cause crashes
        let pdf_data = b"1 0 obj\n<</Contents (5 0 obj fake)>>\nendobj\n\
                          2 0 obj\n<</Type /Pages>>\nendobj\n";

        let source = MemorySource::new(pdf_data.to_vec());
        let result = forward_scan_xref(&source, false);

        // Should find at least the real objects
        // The false positive in the string may or may not be detected
        // depending on exact byte layout
        assert!(result.len() >= 1);

        // Should not panic
    }

    #[test]
    fn test_forward_scan_empty_file() {
        // Empty file should not crash
        let pdf_data = b"";
        let source = MemorySource::new(pdf_data.to_vec());
        let result = forward_scan_xref(&source, false);

        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_forward_scan_no_objects() {
        // File with no indirect objects
        let pdf_data = b"%PDF-1.4\n\
                          % Some random content\n\
                          %%EOF\n";

        let source = MemorySource::new(pdf_data.to_vec());
        let result = forward_scan_xref(&source, false);

        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_parse_obj_header_at_valid() {
        // Test the helper function for parsing object headers
        let pdf_data = b"1 0 obj\n<< /Type /Catalog >>\nendobj\n";
        let source = MemorySource::new(pdf_data.to_vec());

        // The space before "obj" is at offset 4
        let result = parse_obj_header_at(&source, 4);

        assert_eq!(result, Some((1, 0)));
    }

    #[test]
    fn test_parse_obj_header_at_with_generation() {
        let pdf_data = b"42 5 obj\n<< /Type /Catalog >>\nendobj\n";
        let source = MemorySource::new(pdf_data.to_vec());

        // The space before "obj" is at offset 5
        let result = parse_obj_header_at(&source, 5);

        assert_eq!(result, Some((42, 5)));
    }

    #[test]
    fn test_parse_obj_header_at_invalid() {
        // Test invalid pattern (no space before obj)
        let pdf_data = b"1 0\n<< /Type /Catalog >>\nendobj\n";
        let source = MemorySource::new(pdf_data.to_vec());

        let result = parse_obj_header_at(&source, 3);

        assert_eq!(result, None);
    }

    #[test]
    fn test_forward_scan_carriage_return() {
        // Test with \r line endings
        let pdf_data = b"1 0 obj\r<< /Type /Catalog >>\rendobj\r\
                          2 0 obj\r<< /Type /Pages >>\rendobj\r";

        let source = MemorySource::new(pdf_data.to_vec());
        let result = forward_scan_xref(&source, false);

        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_forward_scan_trailer_no_space() {
        // Test "trailer<<" with no space (common in real PDFs)
        let pdf_data = b"1 0 obj\n<< /Type /Catalog >>\nendobj\n\
                          trailer<<\n/Size 2\n>>\n";

        let source = MemorySource::new(pdf_data.to_vec());
        let result = forward_scan_xref(&source, false);

        // Should find the object
        assert_eq!(result.len(), 1);

        // Should have found a trailer
        assert!(result.trailer.is_some());
    }
}
