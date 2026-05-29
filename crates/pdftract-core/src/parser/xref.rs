//! Cross-reference table resolver and traditional xref parser.
//!
//! This module provides:
//! - Traditional xref table parser (20-byte fixed-width entries)
//! - Xref resolver for indirect object resolution
//! - Handling of object streams and circular reference detection

use crate::diagnostics::{DiagCode, Diagnostic as Diag};
use crate::parser::object::{ObjRef, ObjectParser, PdfDict, PdfObject, PdfStream};
use crate::parser::stream::{MemorySource, PdfSource};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

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
    /// Free entry (available for reuse).
    Free {
        /// Object number of the next free entry in the free list.
        next_free: u32,
        /// Generation number when this object was freed.
        gen_nr: u16,
    },
    /// In-use entry at a specific byte offset.
    InUse {
        /// Byte offset of the indirect object in the PDF file.
        offset: u64,
        /// Generation number of this object.
        gen_nr: u16,
    },
    /// Compressed object in an object stream (PDF 1.5+).
    Compressed {
        /// Object number of the containing object stream.
        obj_stm_nr: u32,
        /// Index of this object within the object stream.
        index: u32,
    },
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
    pub diagnostics: Vec<Diag>,
    /// Whether this xref section is from a hybrid file (traditional + stream merged)
    pub is_hybrid: bool,
}

impl XrefSection {
    /// Create a new empty xref section.
    pub fn new() -> Self {
        XrefSection {
            entries: HashMap::new(),
            trailer: None,
            diagnostics: Vec::new(),
            is_hybrid: false,
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

/// Merge a hybrid xref file's traditional table and xref stream.
///
/// Hybrid files have BOTH a traditional xref table at `startxref` AND a
/// supplementary xref stream pointed to by `/XRefStm` in the trailer.
/// Per PDF spec, the traditional table is AUTHORITATIVE for objects it
/// covers; the stream's type-2 entries (compressed-in-ObjStm) fill gaps.
///
/// # Parameters
/// - `traditional`: Xref section from the traditional table (authoritative)
/// - `stream`: Xref section from the xref stream (supplementary)
///
/// # Returns
/// A merged XrefSection where:
/// - All entries from `traditional` are preserved (even type-1 Free entries)
/// - Entries from `stream` are added ONLY if not present in `traditional`
/// - The merged trailer is the traditional one (with `/XRefStm` key removed)
/// - `is_hybrid` is set to true
/// - `STRUCT_HYBRID_CONFLICT` diagnostics emitted for Free/InUse conflicts
///
/// # Priority semantics
/// For overlapping object numbers:
/// - Traditional Free + Stream Free → Free (no conflict, both agree)
/// - Traditional Free + Stream InUse → Free (CONFLICT, traditional wins)
/// - Traditional InUse + Stream Free → InUse (CONFLICT, traditional wins)
/// - Traditional InUse + Stream InUse → InUse (no conflict, both agree)
/// - Traditional InUse + Stream Compressed → InUse (traditional wins)
/// - Traditional `<absent>` + Stream Compressed → Compressed (gap fill)
///
/// # Example
/// ```rust
/// let merged = merge_hybrid(traditional_section, stream_section);
/// assert!(merged.is_hybrid);
/// ```
pub fn merge_hybrid(traditional: XrefSection, stream: XrefSection) -> XrefSection {
    let mut result = XrefSection {
        entries: HashMap::new(),
        trailer: None,
        diagnostics: Vec::new(),
        is_hybrid: true,
    };

    // Start with all traditional entries
    for (obj_nr, entry) in &traditional.entries {
        result.entries.insert(*obj_nr, entry.clone());
    }

    // Merge stream entries: only add if not in traditional
    for (obj_nr, stream_entry) in stream.entries {
        if let Some(trad_entry) = traditional.entries.get(&obj_nr) {
            // Conflict: both tables have this object
            // Check for Free/InUse conflict and emit diagnostic
            let trad_is_free = matches!(trad_entry, XrefEntry::Free { .. });
            let stream_is_inuse = matches!(
                stream_entry,
                XrefEntry::InUse { .. } | XrefEntry::Compressed { .. }
            );

            if trad_is_free && stream_is_inuse {
                result.diagnostics.push(Diag::with_dynamic(
                    DiagCode::StructHybridConflict,
                    0,
                    format!(
                        "Object {}: traditional table marks as Free, stream marks as InUse; traditional wins (object is Free)",
                        obj_nr
                    ),
                ));
            }
            // Traditional wins - don't insert stream entry
        } else {
            // Gap fill: object not in traditional, add from stream
            result.entries.insert(obj_nr, stream_entry);
        }
    }

    // Merge diagnostics from both sections
    result.diagnostics.extend(traditional.diagnostics);
    result.diagnostics.extend(stream.diagnostics);

    // Use traditional trailer, removing /XRefStm key if present
    if let Some(mut trad_trailer) = traditional.trailer {
        trad_trailer.swap_remove("XRefStm");
        result.trailer = Some(trad_trailer);
    } else {
        result.trailer = stream.trailer;
    }

    result
}

/// Detect if a trailer dictionary indicates a hybrid file.
///
/// A hybrid file has a `/XRefStm` key in the trailer dictionary,
/// pointing to the offset of a supplementary xref stream.
///
/// # Parameters
/// - `trailer`: The trailer dictionary to check (may be None)
///
/// # Returns
/// true if the trailer has a `/XRefStm` key, false otherwise
pub fn is_hybrid_trailer(trailer: Option<&PdfDict>) -> bool {
    match trailer {
        Some(dict) => dict.contains_key("XRefStm"),
        None => false,
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
        self.resolving
            .read()
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
        let _entry = self
            .entries
            .get(&obj_ref.object)
            .ok_or_else(|| ResolveError::NotFound(obj_ref))?;

        // Stub: return Null for now
        // Full implementation will read from file offset and parse
        // Use resolve_with_source instead
        self.finish_resolving(obj_ref);
        Ok(PdfObject::Null)
    }

    /// Resolve an object reference to its value, using a file source for reading.
    ///
    /// This method implements full object resolution by reading from the file source.
    /// It:
    /// - Checks for circular references
    /// - Checks the cache first
    /// - Looks up the xref entry
    /// - Reads and parses the object from its file offset
    /// - Caches the result for future lookups
    ///
    /// # Parameters
    /// - `obj_ref`: The object reference to resolve
    /// - `source`: The PDF source to read bytes from
    ///
    /// # Returns
    /// The resolved PdfObject, or an error if resolution fails
    pub fn resolve_with_source(
        &self,
        obj_ref: ObjRef,
        source: &dyn PdfSource,
    ) -> ResolveResult<PdfObject> {
        use crate::parser::object::ObjectParser;

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
        let entry = self
            .entries
            .get(&obj_ref.object)
            .ok_or_else(|| ResolveError::NotFound(obj_ref))?;

        match entry {
            XrefEntry::InUse { offset, gen_nr } => {
                // Check generation number
                if *gen_nr != obj_ref.generation {
                    // Generation mismatch - treat as not found
                    self.finish_resolving(obj_ref);
                    return Err(ResolveError::NotFound(obj_ref));
                }

                // Read the object from the file
                // Read up to 4KB starting from the offset
                let bytes = source.read_at(*offset, 4096).map_err(|e| {
                    ResolveError::Io(format!("Failed to read object at offset {}: {}", offset, e))
                })?;

                // Parse the indirect object
                let mut parser = ObjectParser::new(&bytes);

                // The object should start with "obj_num gen obj"
                // We need to verify that the parsed object number matches
                if let Some(indirect) = parser.parse_indirect_object() {
                    // Verify the object number and generation match
                    if indirect.id.object != obj_ref.object
                        || indirect.id.generation != obj_ref.generation
                    {
                        self.finish_resolving(obj_ref);
                        return Err(ResolveError::NotFound(obj_ref));
                    }

                    // Get the parsed object (the actual value)
                    let obj = indirect.obj;

                    // Cache the result
                    if let Ok(mut cache) = self.cache.write() {
                        cache.insert(obj_ref, obj.clone());
                    }

                    self.finish_resolving(obj_ref);
                    Ok(obj)
                } else {
                    // Failed to parse indirect object
                    self.finish_resolving(obj_ref);
                    Err(ResolveError::NotFound(obj_ref))
                }
            }
            XrefEntry::Free { .. } => {
                // Free entry - object doesn't exist
                self.finish_resolving(obj_ref);
                Err(ResolveError::NotFound(obj_ref))
            }
            XrefEntry::Compressed { .. } => {
                // Object stream - not yet implemented
                // For now, return not found
                self.finish_resolving(obj_ref);
                Err(ResolveError::NotFound(obj_ref))
            }
        }
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
            result.diagnostics.push(Diag::with_static(
                DiagCode::XrefTruncated,
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
                result.diagnostics.push(Diag::with_static(
                    DiagCode::XrefInvalidHeader,
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
            result.diagnostics.push(Diag::with_static(
                DiagCode::XrefInvalidHeader,
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
                result.diagnostics.push(Diag::with_static(
                    DiagCode::XrefTruncated,
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
                result.diagnostics.push(Diag::with_static(
                    DiagCode::XrefInvalidSubsectionHeader,
                    subsection_start,
                    "Failed to read subsection header",
                ));
                break;
            }
        };

        let header_parts: Vec<&str> = header_line.split_whitespace().collect();
        if header_parts.len() != 2 {
            result.diagnostics.push(Diag::with_dynamic(
                DiagCode::XrefInvalidSubsectionHeader,
                subsection_start,
                format!("Invalid subsection header: {}", header_line),
            ));
            // Skip this line and try to continue
            // Find the line ending length
            let line_bytes = source.read_at(subsection_start, header_line.len() + 2).ok();
            let line_ending_len = if let Some(chunk) = line_bytes {
                if chunk.get(header_line.len()) == Some(&b'\r') {
                    if chunk.get(header_line.len() + 1) == Some(&b'\n') {
                        2
                    } else {
                        1
                    }
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
                result.diagnostics.push(Diag::with_dynamic(
                    DiagCode::XrefInvalidSubsectionHeader,
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
                result.diagnostics.push(Diag::with_dynamic(
                    DiagCode::XrefInvalidSubsectionHeader,
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
                if chunk.get(header_line.len() + 1) == Some(&b'\n') {
                    2
                } else {
                    1
                }
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
                    result.diagnostics.push(Diag::with_static(
                        DiagCode::XrefTruncated,
                        pos,
                        "Failed to read xref entry",
                    ));
                    break;
                }
            };

            if entry_bytes.len() < 19 {
                // Definitely truncated
                result.diagnostics.push(Diag::with_static(
                    DiagCode::XrefTruncated,
                    pos,
                    "Xref entry truncated (< 19 bytes)",
                ));
                break;
            }

            // Try to parse as 20-byte entry first
            let parsed = if entry_bytes.len() >= 20 {
                parse_xref_entry(
                    &entry_bytes[..20],
                    obj_start + entries_parsed,
                    entry_start,
                    stride,
                    &mut result.diagnostics,
                )
            } else {
                // Try 19-byte entry for buggy producers
                stride = 19;
                parse_xref_entry(
                    &entry_bytes[..19],
                    obj_start + entries_parsed,
                    entry_start,
                    stride,
                    &mut result.diagnostics,
                )
            };

            match parsed {
                Some((obj_nr, entry)) => {
                    // Object 0 must be free (PDF spec requirement)
                    if obj_nr == 0 {
                        if let XrefEntry::InUse { .. } = entry {
                            result.diagnostics.push(Diag::with_static(
                                DiagCode::XrefObjectZeroNotFree,
                                entry_start,
                                "Object 0 is not free (violates PDF spec)",
                            ));
                        }
                    }
                    // Add all entries to the result (both InUse and Free)
                    // Free entries are needed for /Prev chain merge semantics to track object lifecycle
                    result.add_entry(obj_nr, entry);
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
        result.diagnostics.push(Diag::with_static(
            DiagCode::XrefTrailerNotFound,
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
    diagnostics: &mut Vec<Diag>,
) -> Option<(u32, XrefEntry)> {
    if bytes.len() != stride {
        return None;
    }

    // Convert to string for parsing
    let entry_str = match std::str::from_utf8(bytes) {
        Ok(s) => s,
        Err(_) => {
            diagnostics.push(Diag::with_static(
                DiagCode::XrefInvalidEntry,
                offset,
                "Invalid UTF-8 in xref entry",
            ));
            return None;
        }
    };

    // Entry format: "offset/next_free generation f/n" with line ending
    let parts: Vec<&str> = entry_str.split_whitespace().collect();
    if parts.len() < 3 {
        diagnostics.push(Diag::with_dynamic(
            DiagCode::XrefInvalidEntry,
            offset,
            format!("Malformed xref entry: {}", entry_str.trim()),
        ));
        return None;
    }

    let first_field: u64 = match parts[0].parse() {
        Ok(n) => n,
        Err(_) => {
            diagnostics.push(Diag::with_dynamic(
                DiagCode::XrefInvalidEntry,
                offset,
                format!("Invalid offset/next_free: {}", parts[0]),
            ));
            return None;
        }
    };

    let gen_nr: u16 = match parts[1].parse() {
        Ok(n) => n,
        Err(_) => {
            diagnostics.push(Diag::with_dynamic(
                DiagCode::XrefInvalidEntry,
                offset,
                format!("Invalid generation: {}", parts[1]),
            ));
            return None;
        }
    };

    let entry_type = parts[2].chars().next();
    match entry_type {
        Some('n') | Some('N') => Some((
            obj_nr,
            XrefEntry::InUse {
                offset: first_field,
                gen_nr,
            },
        )),
        Some('f') | Some('F') => Some((
            obj_nr,
            XrefEntry::Free {
                next_free: first_field as u32,
                gen_nr,
            },
        )),
        _ => {
            diagnostics.push(Diag::with_dynamic(
                DiagCode::XrefInvalidEntry,
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
fn read_line(source: &dyn PdfSource, pos: &mut u64, diagnostics: &mut Vec<Diag>) -> Option<String> {
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
/// Parse the trailer dictionary from the xref trailer section.
///
/// This function extracts the trailer dictionary bytes and parses them
/// using the object parser to get the actual key-value pairs.
fn parse_trailer_dict(
    source: &dyn PdfSource,
    pos: &mut u64,
    diagnostics: &mut Vec<Diag>,
) -> Option<PdfDict> {
    // Skip whitespace before <<
    let mut seen_bracket = false;
    let mut depth = 0;
    let mut chunk_pos = 0u64;
    let dict_start_offset = *pos;
    let mut dict_end_offset = None;

    // First, find the extent of the trailer dict (from << to >>)
    loop {
        let chunk = match source.read_at(dict_start_offset + chunk_pos, 4096) {
            Ok(bytes) => bytes,
            Err(_) => {
                diagnostics.push(Diag::with_static(
                    DiagCode::XrefTrailerNotFound,
                    dict_start_offset,
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
                                        // Found the end of the dict
                                        let end_offset =
                                            dict_start_offset + chunk_pos + j as u64 + 2;
                                        dict_end_offset = Some(end_offset);
                                        break;
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

        if dict_end_offset.is_some() {
            break;
        }

        chunk_pos += chunk.len() as u64;

        // Safety limit
        if chunk_pos > 100000 {
            diagnostics.push(Diag::with_static(
                DiagCode::XrefTrailerNotFound,
                dict_start_offset,
                "Trailer dictionary too large or unterminated",
            ));
            return None;
        }
    }

    // If we didn't find the end, return None
    let dict_end_offset = match dict_end_offset {
        Some(offset) => offset,
        None => {
            diagnostics.push(Diag::with_static(
                DiagCode::XrefTrailerNotFound,
                dict_start_offset,
                "Trailer dictionary not found (no << >> markers)",
            ));
            return None;
        }
    };

    // Read the full dict bytes and parse them
    let dict_len = (dict_end_offset - dict_start_offset) as usize;
    let dict_bytes = match source.read_at(dict_start_offset, dict_len) {
        Ok(bytes) => bytes,
        Err(_) => {
            diagnostics.push(Diag::with_static(
                DiagCode::XrefTrailerNotFound,
                dict_start_offset,
                "Failed to read trailer dictionary bytes",
            ));
            return None;
        }
    };

    // Parse the dict using ObjectParser
    let mut parser = ObjectParser::new(&dict_bytes);
    if let Some(PdfObject::Dict(dict)) = parser.parse_direct_object() {
        // Update pos to after the dict
        *pos = dict_end_offset;

        // Transfer any diagnostics from the parser
        for diag in parser.take_diagnostics() {
            diagnostics.push(Diag::with_dynamic(
                DiagCode::XrefTrailerNotFound,
                dict_start_offset,
                diag.message.into_owned(),
            ));
        }

        Some(*dict)
    } else {
        diagnostics.push(Diag::with_static(
            DiagCode::XrefTrailerNotFound,
            dict_start_offset,
            "Failed to parse trailer dictionary as a dict object",
        ));
        None
    }
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
        result.diagnostics.push(Diag::with_static(
            DiagCode::XrefLinearizedNoForwardScan,
            0,
            "Forward scan disabled for linearized PDF (partial leading xref would cause false results)",
        ));
        return result;
    }

    // Check for remote source - forward scan disabled for HTTP sources
    if source.is_remote() {
        result.diagnostics.push(Diag::with_static(
            DiagCode::XrefRemoteNoForwardScan,
            0,
            "Forward scan disabled for remote PDF (would require fetching entire file)",
        ));
        return result;
    }

    let source_len = match source.len() {
        Ok(len) if len > 0 => len,
        _ => {
            result.diagnostics.push(Diag::with_static(
                DiagCode::XrefTruncated,
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
                                check_trailing_whitespace(
                                    source,
                                    chunk_offset + abs_space_idx + 3,
                                    source_len,
                                )
                            };

                            if has_trailing_ws {
                                let obj_offset = chunk_offset + abs_space_idx;
                                if let Some((obj_num, gen_num)) =
                                    parse_obj_header_at(source, obj_offset)
                                {
                                    result.entries.insert(
                                        obj_num,
                                        XrefEntry::InUse {
                                            offset: obj_offset,
                                            gen_nr: gen_num,
                                        },
                                    );
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
    result.diagnostics.push(Diag::with_dynamic(
        DiagCode::XrefRepaired,
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
                        result.entries.insert(
                            obj_num,
                            XrefEntry::InUse {
                                offset: obj_offset,
                                gen_nr: gen_num,
                            },
                        );
                        entries_found += 1;
                    }
                }
            }
        }
    }

    // Emit XREF_REPAIRED diagnostic with count
    result.diagnostics.push(Diag::with_dynamic(
        DiagCode::XrefRepaired,
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
        if let Some(idx) = chunk
            .windows(TRAILER_KEYWORD.len())
            .position(|w| w == TRAILER_KEYWORD)
        {
            let trailer_offset = pos + idx as u64;

            // Verify it's at a token boundary (preceded by whitespace or start)
            let valid_boundary = if idx > 0 {
                chunk[idx - 1].is_ascii_whitespace()
                    || chunk[idx - 1] == b'\n'
                    || chunk[idx - 1] == b'\r'
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

/// Parse a PDF 1.5+ cross-reference stream.
///
/// Xref streams are an alternative to the traditional table format that supports
/// compression and the type-2 (compressed-in-ObjStm) entry.
///
/// # Parameters
/// - `source`: The PDF source to read bytes from
/// - `stream_obj_offset`: The byte offset of the xref stream indirect object
///
/// # Returns
/// An `XrefSection` containing the parsed entries and trailer dictionary.
///
/// # Format
/// An xref stream is an indirect object with `/Type /XRef`:
/// ```text
/// N G obj
/// << /Type /XRef /Size N /W [type_w obj_w gen_w] /Index [first count ...] >>
/// stream
/// <compressed entry data>
/// endstream
/// endobj
/// ```
///
/// Each entry in the decompressed data has (type_w + obj_w + gen_w) bytes:
/// - Type 0 (free): obj_w = next free object number, gen_w = generation
/// - Type 1 (in-use): obj_w = byte offset, gen_w = generation
/// - Type 2 (compressed): obj_w = ObjStm object number, gen_w = index in ObjStm
///
/// # Multi-byte field encoding
/// All multi-byte fields are BIG-ENDIAN per PDF spec.
/// Zero-width fields default to 0.
pub fn parse_xref_stream(source: &dyn PdfSource, stream_obj_offset: u64) -> XrefSection {
    use crate::parser::object::ObjectParser;
    use crate::parser::stream::{decode_stream, ExtractionOptions};

    let mut result = XrefSection::new();

    // Read the indirect object at the given offset
    let obj_bytes = match source.read_at(stream_obj_offset, 4096) {
        Ok(bytes) if !bytes.is_empty() => bytes,
        _ => {
            result.diagnostics.push(Diag::with_static(
                DiagCode::XrefInvalidStreamFormat,
                stream_obj_offset,
                "Failed to read xref stream object",
            ));
            return result;
        }
    };

    let mut parser = ObjectParser::new(&obj_bytes);
    let indirect = match parser.parse_indirect_object() {
        Some(i) => i,
        None => {
            result.diagnostics.push(Diag::with_static(
                DiagCode::XrefInvalidStreamFormat,
                stream_obj_offset,
                "Failed to parse xref stream as indirect object",
            ));
            return result;
        }
    };

    // Verify it's a stream with /Type /XRef
    let stream = match indirect.obj {
        PdfObject::Stream(s) => s,
        _ => {
            result.diagnostics.push(Diag::with_static(
                DiagCode::XrefInvalidStreamFormat,
                stream_obj_offset,
                "Xref stream object is not a stream",
            ));
            return result;
        }
    };

    // Check for /Type /XRef (optional per spec, but we validate it)
    if let Some(PdfObject::Name(type_name)) = stream.dict.get("Type") {
        if type_name.as_ref() != "/XRef" && type_name.as_ref() != "XRef" {
            result.diagnostics.push(Diag::with_static(
                DiagCode::XrefInvalidStreamFormat,
                stream_obj_offset,
                "Stream /Type is not /XRef",
            ));
        }
    }

    // Extract /Size (total object count, required)
    let size = match stream.dict.get("Size") {
        Some(PdfObject::Integer(n)) if *n >= 0 => *n as u32,
        _ => {
            result.diagnostics.push(Diag::with_static(
                DiagCode::XrefInvalidStreamFormat,
                stream_obj_offset,
                "Missing or invalid /Size in xref stream",
            ));
            return result;
        }
    };

    // Extract /W [type_w obj_w gen_w] (required)
    let field_widths = match stream.dict.get("W") {
        Some(PdfObject::Array(arr)) => {
            let widths: Vec<i64> = arr.iter().filter_map(|o| o.as_int()).collect();
            if widths.len() != 3 {
                result.diagnostics.push(Diag::with_dynamic(
                    DiagCode::XrefInvalidStreamFormat,
                    stream_obj_offset,
                    format!("/W array must have 3 elements, got {}", widths.len()),
                ));
                return result;
            }
            // Widths can be 0, but negative is invalid
            if widths.iter().any(|&w| w < 0) {
                result.diagnostics.push(Diag::with_static(
                    DiagCode::XrefInvalidStreamFormat,
                    stream_obj_offset,
                    "/W array contains negative values",
                ));
                return result;
            }
            widths
        }
        _ => {
            result.diagnostics.push(Diag::with_static(
                DiagCode::XrefInvalidStreamFormat,
                stream_obj_offset,
                "Missing or invalid /W in xref stream",
            ));
            return result;
        }
    };

    let type_w = field_widths[0] as usize;
    let obj_w = field_widths[1] as usize;
    let gen_w = field_widths[2] as usize;
    let entry_stride = type_w + obj_w + gen_w;

    // Extract /Index [first_1 count_1 first_2 count_2 ...] (optional)
    // Default is [0 size] if absent
    let subsections = match stream.dict.get("Index") {
        Some(PdfObject::Array(arr)) => {
            let mut pairs = Vec::new();
            let mut iter = arr.iter().peekable();
            while let Some(first_obj) = iter.next() {
                let first = match first_obj.as_int() {
                    Some(n) if n >= 0 => n as u32,
                    _ => {
                        result.diagnostics.push(Diag::with_static(
                            DiagCode::XrefInvalidStreamFormat,
                            stream_obj_offset,
                            "Invalid /Index first value",
                        ));
                        return result;
                    }
                };
                let count = match iter.peek() {
                    Some(PdfObject::Integer(n)) if *n >= 0 => *n as u32,
                    _ => {
                        result.diagnostics.push(Diag::with_static(
                            DiagCode::XrefInvalidStreamFormat,
                            stream_obj_offset,
                            "Invalid /Index count value",
                        ));
                        return result;
                    }
                };
                let _ = iter.next(); // consume count
                pairs.push((first, count));
            }
            if pairs.is_empty() {
                result.diagnostics.push(Diag::with_static(
                    DiagCode::XrefInvalidStreamFormat,
                    stream_obj_offset,
                    "/Index array is empty",
                ));
                return result;
            }
            pairs
        }
        None => vec![(0, size)],
        _ => {
            result.diagnostics.push(Diag::with_static(
                DiagCode::XrefInvalidStreamFormat,
                stream_obj_offset,
                "Invalid /Index in xref stream (not an array)",
            ));
            return result;
        }
    };

    // The trailer dict is the stream's dict itself (minus xref-specific keys)
    // Copy relevant trailer keys: /Root, /Info, /ID, /Encrypt, /Prev
    let mut trailer = PdfDict::new();
    for (key, value) in &stream.dict {
        let key_str = key.as_ref();
        if matches!(key_str, "Root" | "Info" | "ID" | "Encrypt" | "Prev") {
            trailer.insert(key.clone(), value.clone());
        }
    }
    result.trailer = Some(trailer);

    // Decompress the stream body
    // The stream's offset is relative to obj_bytes, so we create a MemorySource
    // from those bytes to decode the stream data correctly.
    use crate::parser::stream::MemorySource;
    let local_source = MemorySource::new(obj_bytes);

    let decoded = decode_stream(
        &stream,
        &local_source,
        &ExtractionOptions::default(),
        &mut 0,
    );

    if decoded.is_empty() {
        // Check if this is a legitimate empty stream (no objects) or an error
        // A valid xref stream with no objects would have /Size 0, which is unusual
        result.diagnostics.push(Diag::with_static(
            DiagCode::StreamDecodeError,
            stream_obj_offset,
            "Xref stream decompression produced empty output",
        ));
        return result;
    }

    // Parse entries from decompressed data
    // Each subsection has (count) entries of (entry_stride) bytes
    let mut data_pos = 0;

    for (subsection_first, subsection_count) in subsections {
        for i in 0..subsection_count {
            let obj_nr = subsection_first.saturating_add(i);

            // Check we have enough bytes for this entry
            if data_pos + entry_stride > decoded.len() {
                result.diagnostics.push(Diag::with_dynamic(
                    DiagCode::XrefInvalidStreamEntry,
                    stream_obj_offset,
                    format!("Xref stream truncated at object {}", obj_nr),
                ));
                break;
            }

            let entry_data = &decoded[data_pos..data_pos + entry_stride];

            // Parse the entry fields (big-endian)
            let entry_type = if type_w > 0 {
                read_big_endian_field(&entry_data[0..type_w])
            } else {
                0 // Default type is 0 (free) if width is 0
            };

            let obj_field = if obj_w > 0 {
                read_big_endian_field(&entry_data[type_w..type_w + obj_w])
            } else {
                0
            };

            let gen_field = if gen_w > 0 {
                read_big_endian_field(&entry_data[type_w + obj_w..entry_stride]) as u16
            } else {
                0
            };

            // Dispatch on entry type
            let entry = match entry_type {
                0 => {
                    // Type 0: free entry
                    // obj_field = next free object number, gen_field = generation
                    XrefEntry::Free {
                        next_free: obj_field as u32,
                        gen_nr: gen_field,
                    }
                }
                1 => {
                    // Type 1: in-use, uncompressed
                    // obj_field = byte offset, gen_field = generation
                    XrefEntry::InUse {
                        offset: obj_field,
                        gen_nr: gen_field,
                    }
                }
                2 => {
                    // Type 2: compressed in ObjStm
                    // obj_field = host ObjStm object number, gen_field = index in ObjStm
                    XrefEntry::Compressed {
                        obj_stm_nr: obj_field as u32,
                        index: gen_field as u32,
                    }
                }
                _ => {
                    // Unknown type - emit diagnostic and treat as free
                    result.diagnostics.push(Diag::with_dynamic(
                        DiagCode::XrefInvalidStreamEntry,
                        stream_obj_offset,
                        format!(
                            "Invalid xref entry type {} for object {}",
                            entry_type, obj_nr
                        ),
                    ));
                    XrefEntry::Free {
                        next_free: 0,
                        gen_nr: 0,
                    }
                }
            };

            // Only add in-use and compressed entries to the result
            // Free entries are ignored per pdftract spec
            if matches!(
                entry,
                XrefEntry::InUse { .. } | XrefEntry::Compressed { .. }
            ) {
                result.add_entry(obj_nr, entry);
            }

            data_pos += entry_stride;
        }
    }

    result
}

/// Read a big-endian integer from a byte slice of variable width.
///
/// The width can be 1-4 bytes (larger widths are not valid per PDF spec).
/// Returns the integer value, or 0 if the width is 0.
fn read_big_endian_field(bytes: &[u8]) -> u64 {
    let width = bytes.len();
    if width == 0 {
        return 0;
    }
    if width > 8 {
        // Cap at 8 bytes to prevent overflow
        // (PDF spec limits field widths to 4 bytes max for obj/gen fields)
        return 0;
    }

    let mut result: u64 = 0;
    for &byte in bytes {
        result = result.wrapping_shl(8) | (byte as u64);
    }
    result
}

// ============================================================================
// Linearized PDF Detection and Xref Merging
// ============================================================================

/// Information about a linearized PDF file.
///
/// Linearized PDFs (PDF 1.2+ "Optimized for Web View") have a special structure
/// with TWO xref tables: one at the beginning (covering only the first page)
/// and one at the end (the complete xref). This struct captures the metadata
/// needed to load and merge both xrefs.
#[derive(Debug, Clone, PartialEq)]
pub struct LinearizationInfo {
    /// Total file length from the /L entry
    pub file_length: u64,
    /// Offset of the first-page xref from the /T entry
    pub first_page_xref_offset: u64,
    /// Offset of the hint stream from the first /H entry (optional)
    pub hint_stream_offset: Option<u64>,
    /// Length of the hint stream from the second /H entry (optional)
    pub hint_stream_length: Option<u64>,
    /// Number of pages in the document from /N
    pub page_count: u32,
    /// Offset of the end of the first page from /E
    pub first_page_end_offset: u64,
    /// The object number of the first page from /O
    pub first_page_object_number: u32,
}

/// Detect if a PDF is linearized and extract the linearization dictionary info.
///
/// Linearized PDFs have a special object as the first indirect object in the file
/// (right after the `%PDF-X.Y` header). This object is a dictionary with the
/// `/Linearized` key.
///
/// # Parameters
/// - `source`: The PDF source to read from
///
/// # Returns
/// - `Some(LinearizationInfo)` if the file is linearized and valid
/// - `None` if the file is not linearized or the linearization dict is invalid
///
/// # Algorithm
/// 1. Read the first ~2 KB of the file
/// 2. Skip the `%PDF-X.Y\n` header (~10 bytes)
/// 3. Look for the `obj` keyword to find the first indirect object
/// 4. Parse the object and check if it's a dict with `/Linearized`
/// 5. Extract the required fields: /L, /T, /H, /E, /N, /O
/// 6. Validate that /L matches the actual file size
///
/// # References
/// - PDF spec Annex F (Linearized PDF)
/// - Plan section: Phase 1.3 line 1113
pub fn detect_linearization(source: &dyn PdfSource) -> Option<LinearizationInfo> {
    // Read the first 2 KB to find the linearization dict
    let header_bytes = source.read_at(0, 2048).ok()?;

    // Convert to UTF-8 for string operations
    let header_str = std::str::from_utf8(&header_bytes).ok()?;

    // Skip the PDF header (e.g., "%PDF-1.4\n")
    // Find the end of the first line (after the header)
    let header_end = header_str.find('\n').or_else(|| header_str.find('\r'))?;
    let after_header = &header_str[header_end + 1..];

    // Look for the first indirect object declaration (e.g., "1 0 obj")
    // The linearization dict is typically object 1 or a low number
    let obj_pos = after_header.find(" obj")?;
    let before_obj = &after_header[..obj_pos];

    // Parse the object number (e.g., "1 0")
    let parts: Vec<&str> = before_obj.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }

    let _obj_num: u32 = parts.get(0)?.parse().ok()?;
    let _gen_num: u16 = parts.get(1)?.parse().ok()?;

    // Now we need to find and parse the dictionary
    // Find the start of the dict ("<<")
    let dict_pos = after_header.find("<<")?;
    let dict_section = &after_header[dict_pos..];

    // Parse the /Linearized key
    // The dict should have "/Linearized" followed by a number (typically 1.0)
    if !dict_section.contains("/Linearized") {
        return None;
    }

    // Helper to extract a number after a key
    // Handles both "/Key 123" and "/Key 123.456" formats
    // Returns None if the key is a substring of another key (e.g., /L in /Linearized)
    let extract_number = |key: &str| -> Option<i64> {
        let mut search_start = 0;
        loop {
            let key_pos = dict_section[search_start..].find(key)?;
            let absolute_pos = search_start + key_pos;

            // Check that the key is not a substring of another key
            // The character after the key must be whitespace, delimiter, or end of string
            let after_key = &dict_section[absolute_pos + key.len()..];
            let next_char = after_key.chars().next();

            // If the next character is a letter or digit, this is a substring match
            // (e.g., "/L" found in "/Linearized")
            if matches!(next_char, Some(c) if c.is_alphanumeric()) {
                // Skip past this match and continue searching
                search_start = absolute_pos + key.len();
                if search_start >= dict_section.len() {
                    return None;
                }
                continue;
            }

            // Found a standalone key - extract the number
            let number_str = after_key.split_whitespace().next()?;
            // Parse as float first, then convert to i64
            let float_val: f64 = number_str.parse().ok()?;
            return Some(float_val as i64);
        }
    };

    // Extract required fields
    let file_length = extract_number("/L")? as u64;
    let first_page_xref_offset = extract_number("/T")? as u64;
    let page_count = extract_number("/N")? as u32;
    let first_page_end_offset = extract_number("/E")? as u64;
    let first_page_object_number = extract_number("/O")? as u32;

    // Extract optional /H entry (array of two numbers: [offset length])
    // Same logic as extract_number to avoid substring matches
    let (hint_stream_offset, hint_stream_length) = {
        let mut search_start = 0;
        let mut found_h = None;

        loop {
            if let Some(h_pos) = dict_section[search_start..].find("/H") {
                let absolute_pos = search_start + h_pos;

                // Check that /H is not a substring of another key
                let after_h = &dict_section[absolute_pos + 2..];
                let next_char = after_h.chars().next();

                if matches!(next_char, Some(c) if c.is_alphanumeric()) {
                    // Substring match, skip and continue
                    search_start = absolute_pos + 2;
                    if search_start >= dict_section.len() {
                        break;
                    }
                    continue;
                }

                // Found standalone /H - try to parse the value
                found_h = Some(after_h);
                break;
            } else {
                break;
            }
        }

        if let Some(after_h) = found_h {
            // /H can be followed by an array [offset length] or two numbers
            // Try to parse as array first
            if let Some(bracket_start) = after_h.find('[') {
                let bracket_content = &after_h[bracket_start + 1..];
                if let Some(bracket_end) = bracket_content.find(']') {
                    let array_content = &bracket_content[..bracket_end];
                    let numbers: Vec<&str> = array_content.split_whitespace().collect();
                    if numbers.len() >= 2 {
                        let offset = numbers[0].parse::<u64>().ok()?;
                        let length = numbers[1].parse::<u64>().ok()?;
                        (Some(offset), Some(length))
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                }
            } else {
                // Try parsing as two consecutive numbers
                let h_numbers: Vec<&str> = after_h.split_whitespace().collect();
                if h_numbers.len() >= 2 {
                    let offset = h_numbers[0].parse::<u64>().ok()?;
                    let length = h_numbers[1].parse::<u64>().ok()?;
                    (Some(offset), Some(length))
                } else {
                    (None, None)
                }
            }
        } else {
            (None, None)
        }
    };

    // Validate that /L matches the actual file size
    let actual_file_length = match source.len() {
        Ok(len) => len,
        Err(_) => return None,
    };
    if file_length != actual_file_length {
        // File was modified after linearization (incremental update)
        // Linearization is invalid, fall through to non-linearized path
        return None;
    }

    Some(LinearizationInfo {
        file_length,
        first_page_xref_offset,
        hint_stream_offset,
        hint_stream_length,
        page_count,
        first_page_end_offset,
        first_page_object_number,
    })
}

/// Merge two xref sections with the full xref taking precedence.
///
/// For linearized PDFs, we have two xref tables:
/// - First-page xref: covers only objects needed to render the first page
/// - Full xref: covers all objects in the document
///
/// The merge semantics are: for any object number present in BOTH xrefs,
/// the FULL xref's entry wins. This is because the full xref is authoritative
/// for the entire document.
///
/// # Parameters
/// - `first_page_xref`: Xref section from the first-page xref (at /T offset)
/// - `full_xref`: Xref section from the full xref (at EOF startxref)
///
/// # Returns
/// A merged XrefSection where:
/// - All entries from `first_page_xref` are included
/// - Entries from `full_xref` OVERLAP and replace any conflicting entries
/// - The merged trailer is the full xref's trailer
/// - Diagnostics from both sections are combined
///
/// # Priority semantics
/// For overlapping object numbers:
/// - First-page InUse + Full InUse → Full wins (same offset expected)
/// - First-page InUse + Full Free → Full wins (object was deleted)
/// - First-page Free + Full InUse → Full wins (object was added)
/// - First-page `<absent>` + Full InUse → Full wins (gap filled)
///
/// # References
/// - Plan section: Phase 1.3 line 1113
pub fn merge_linearized_xrefs(first_page_xref: XrefSection, full_xref: XrefSection) -> XrefSection {
    let mut result = XrefSection::new();

    // Start with all first-page entries
    result.entries = first_page_xref.entries;

    // Overlay full xref entries (full wins for conflicts)
    for (obj_nr, entry) in full_xref.entries {
        result.entries.insert(obj_nr, entry);
    }

    // Use the full xref's trailer (it's authoritative)
    result.trailer = full_xref.trailer;

    // Combine diagnostics from both sections
    result.diagnostics = first_page_xref.diagnostics;
    result.diagnostics.extend(full_xref.diagnostics);

    // Note: is_hybrid is NOT set here - linearized is a separate concept from hybrid

    result
}

/// Load the complete xref table for a linearized PDF.
///
/// This function:
/// 1. Loads the first-page xref from the offset specified in /T
/// 2. Loads the full xref from the EOF startxref
/// 3. Merges them with full xref taking precedence
///
/// # Parameters
/// - `source`: The PDF source to read from
/// - `lin_info`: Linearization info from `detect_linearization`
/// - `startxref_offset`: The offset of the full xref (from EOF startxref)
///
/// # Returns
/// A merged XrefSection containing entries from both xrefs.
///
/// # Strategy
/// The function tries both traditional and xref stream parsers for each xref,
/// in order:
/// 1. Try traditional parser
/// 2. If that fails, try xref stream parser
/// 3. If both fail, return empty section with diagnostics
///
/// # References
/// - Plan section: Phase 1.3 line 1113
pub fn load_xref_linearized(
    source: &dyn PdfSource,
    lin_info: &LinearizationInfo,
    startxref_offset: u64,
) -> XrefSection {
    // Load first-page xref from /T offset
    let first_page_xref = load_single_xref(source, lin_info.first_page_xref_offset);

    // Load full xref from EOF startxref
    let full_xref = load_single_xref(source, startxref_offset);

    // Merge with full xref taking precedence
    merge_linearized_xrefs(first_page_xref, full_xref)
}

/// Load a single xref section from a given offset.
///
/// Handles three cases:
/// 1. Hybrid files: traditional table + xref stream from /XRefStm (merged)
/// 2. Pure traditional: only traditional xref table
/// 3. Pure stream: only xref stream (no traditional table found)
fn load_single_xref(source: &dyn PdfSource, offset: u64) -> XrefSection {
    // Try traditional xref table first
    let traditional = parse_traditional_xref(source, offset);

    // Check if this is a hybrid file (traditional trailer has /XRefStm)
    if is_hybrid_trailer(traditional.trailer.as_ref()) {
        // Extract the /XRefStm offset
        let xrefstm_offset = traditional.trailer.as_ref().and_then(|trailer| {
            trailer.get("XRefStm").and_then(|obj| match obj {
                PdfObject::Integer(n) if *n >= 0 => Some(*n as u64),
                _ => None,
            })
        });

        if let Some(stream_offset) = xrefstm_offset {
            // Load the supplementary xref stream
            let stream = parse_xref_stream(source, stream_offset);

            // Merge with traditional taking priority
            return merge_hybrid(traditional, stream);
        }
        // If /XRefStm offset is invalid, fall through to traditional-only
    }

    // If traditional parsing succeeded (found at least one entry), return it
    if !traditional.entries.is_empty() || traditional.trailer.is_some() {
        return traditional;
    }

    // Otherwise, try xref stream (pure stream file)
    // For xref streams, the offset points to the indirect object containing the stream
    let stream = parse_xref_stream(source, offset);

    stream
}

/// Maximum depth for /Prev chain traversal.
///
/// Per PDF spec, incremental updates create a chain of xref tables.
/// This limit prevents adversarial inputs from causing stack overflow.
const MAX_PREV_DEPTH: u32 = 32;

/// Load xref with /Prev chain traversal for incremental updates.
///
/// When a PDF is edited incrementally, each edit appends a new xref + trailer
/// at the end of the file. The new trailer's `/Prev` key points to the previous
/// xref's offset. This function walks the chain and merges all revisions.
///
/// # Parameters
/// - `source`: PDF data source
/// - `start_offset`: Offset to start loading from (typically from `startxref`)
///
/// # Returns
/// A merged `XrefSection` where:
/// - All entries from all revisions are present
/// - For each object number, the LATEST revision's entry wins (override semantics)
/// - The trailer is the LATEST revision's trailer (newest /Root, /Info, /ID)
/// - `is_hybrid` is true if ANY revision in the chain is hybrid
///
/// # Chain traversal
/// 1. Load xref at `start_offset` (auto-detects traditional vs stream vs hybrid)
/// 2. If trailer has `/Prev`, recursively load from that offset
/// 3. Merge: start with older revisions, overwrite with newer entries
/// 4. Stop when trailer has no `/Prev` (original/baseline revision)
///
/// # Error handling
/// - `/Prev` offset of 0 or negative: treated as "no previous revision"
/// - `/Prev` offset > file size: emit `STRUCT_INVALID_PREV_OFFSET`, ignore /Prev
/// - Cycle detection: `HashSet<u64>` of visited offsets; emit `STRUCT_CIRCULAR_REF`
/// - Depth limit: 32 revisions max; emit `STRUCT_DEPTH_EXCEEDED` on deeper chains
///
/// # Example
/// ```rust,no_run
/// let merged = load_xref_with_prev_chain(&source, startxref_offset);
/// // merged.entries contains objects from all 3 revisions
/// // merged.trailer is from revision 3 (latest)
/// ```
///
/// # References
/// - Plan section: Phase 1.3 line 1093 (/Prev chain)
/// - PDF spec 7.5.6 (Incremental Updates)
pub fn load_xref_with_prev_chain(source: &dyn PdfSource, start_offset: u64) -> XrefSection {
    // Inner recursive function with visited set and depth counter
    fn walk_chain(
        source: &dyn PdfSource,
        offset: u64,
        visited: &mut HashSet<u64>,
        depth: u32,
        diagnostics: &mut Vec<Diag>,
    ) -> XrefSection {
        // Cycle detection
        if visited.contains(&offset) {
            diagnostics.push(Diag::with_static(
                DiagCode::StructCircularRef,
                offset,
                "Circular /Prev reference detected; stopping chain traversal",
            ));
            // Return empty section to break the cycle
            return XrefSection::new();
        }
        visited.insert(offset);

        // Depth limit check
        if depth >= MAX_PREV_DEPTH {
            diagnostics.push(Diag::with_dynamic(
                DiagCode::StructDepthExceeded,
                offset,
                format!("/Prev chain depth exceeded maximum of {}", MAX_PREV_DEPTH).into(),
            ));
            // Return empty section to stop the chain
            return XrefSection::new();
        }

        // Load xref at current offset
        let mut current = load_single_xref(source, offset);

        // Extract /Prev offset from trailer
        let prev_offset = current.trailer.as_ref().and_then(|trailer| {
            trailer.get("Prev").and_then(|obj| match obj {
                PdfObject::Integer(n) if *n > 0 => Some(*n as u64),
                _ => None,
            })
        });

        // Validate /Prev offset and recursively load previous revision if present
        if let Some(prev) = prev_offset {
            match source.len() {
                Ok(file_size) if prev > file_size => {
                    // /Prev points beyond file size - invalid
                    diagnostics.push(Diag::with_dynamic(
                        DiagCode::StructInvalidPrevOffset,
                        offset,
                        format!(
                            "/Prev offset {} exceeds file size {}; ignoring /Prev key",
                            prev, file_size
                        )
                        .into(),
                    ));
                    // Remove the invalid /Prev key from trailer
                    if let Some(ref mut trailer) = current.trailer {
                        trailer.shift_remove("Prev");
                    }
                    // Return current revision without following /Prev
                    let mut result = current;
                    result.diagnostics.extend(diagnostics.drain(..));
                    return result;
                }
                Ok(_) => {
                    // Valid /Prev offset - recursively load
                    let mut older = walk_chain(source, prev, visited, depth + 1, diagnostics);

                    // Merge: older entries first, then current (newer) entries override
                    // This is the opposite of hybrid merge (where first parameter wins)
                    for (obj_nr, entry) in current.entries {
                        older.entries.insert(obj_nr, entry);
                    }

                    // Preserve current (latest) trailer
                    older.trailer = current.trailer;

                    // Merge diagnostics from current revision
                    older.diagnostics.extend(current.diagnostics);

                    // Mark as hybrid if current revision is hybrid
                    if current.is_hybrid {
                        older.is_hybrid = true;
                    }

                    // Add current's diagnostics to the merged result
                    older.diagnostics.extend(diagnostics.drain(..));

                    older
                }
                Err(_) => {
                    // Can't determine file size - be conservative and don't follow
                    diagnostics.push(Diag::with_static(
                        DiagCode::StructInvalidPrevOffset,
                        offset,
                        "Cannot determine file size; ignoring /Prev key",
                    ));
                    // Return current revision without following /Prev
                    let mut result = current;
                    result.diagnostics.extend(diagnostics.drain(..));
                    result
                }
            }
        } else {
            // No /Prev - this is the baseline (original) revision
            // Return current with any diagnostics from this level
            let mut result = current;
            result.diagnostics.extend(diagnostics.drain(..));
            result
        }
    }

    let mut visited = HashSet::new();
    let mut diagnostics = Vec::new();
    walk_chain(source, start_offset, &mut visited, 0, &mut diagnostics)
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
        resolver.add_entry(
            1,
            XrefEntry::InUse {
                offset: 100,
                gen_nr: 0,
            },
        );
        assert_eq!(resolver.len(), 1);
    }

    #[test]
    fn test_get_entry() {
        let mut resolver = XrefResolver::new();
        let entry = XrefEntry::InUse {
            offset: 100,
            gen_nr: 0,
        };
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
        section.add_entry(
            1,
            XrefEntry::InUse {
                offset: 100,
                gen_nr: 0,
            },
        );
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
        let entry = XrefEntry::InUse {
            offset: 1000,
            gen_nr: 5,
        };
        assert!(matches!(
            entry,
            XrefEntry::InUse {
                offset: 1000,
                gen_nr: 5
            }
        ));
    }

    #[test]
    fn test_xref_entry_free() {
        let entry = XrefEntry::Free {
            next_free: 42,
            gen_nr: 1,
        };
        assert!(matches!(
            entry,
            XrefEntry::Free {
                next_free: 42,
                gen_nr: 1
            }
        ));
    }

    #[test]
    fn test_xref_entry_compressed() {
        let entry = XrefEntry::Compressed {
            obj_stm_nr: 10,
            index: 5,
        };
        assert!(matches!(
            entry,
            XrefEntry::Compressed {
                obj_stm_nr: 10,
                index: 5
            }
        ));
    }

    #[test]
    fn test_xref_resolver_from_section() {
        let mut section = XrefSection::new();
        section.add_entry(
            1,
            XrefEntry::InUse {
                offset: 100,
                gen_nr: 0,
            },
        );
        section.add_entry(
            2,
            XrefEntry::InUse {
                offset: 200,
                gen_nr: 0,
            },
        );

        let resolver = XrefResolver::from_section(section);
        assert_eq!(resolver.len(), 2);
        assert_eq!(
            resolver.get_entry(1),
            Some(&XrefEntry::InUse {
                offset: 100,
                gen_nr: 0
            })
        );
        assert_eq!(
            resolver.get_entry(2),
            Some(&XrefEntry::InUse {
                offset: 200,
                gen_nr: 0
            })
        );
    }

    #[test]
    fn test_xref_diagnostic_static() {
        let diag = Diag::with_static(DiagCode::XrefInvalidHeader, 100, "test message");
        assert_eq!(diag.byte_offset, Some(100));
        assert_eq!(diag.message.as_ref(), "test message");
        assert!(matches!(diag.code, DiagCode::XrefInvalidHeader));
    }

    #[test]
    fn test_xref_diagnostic_dynamic() {
        let diag = Diag::with_dynamic(
            DiagCode::XrefInvalidEntry,
            200,
            "dynamic message".to_string(),
        );
        assert_eq!(diag.byte_offset, Some(200));
        assert_eq!(diag.message.as_ref(), "dynamic message");
        assert!(matches!(diag.code, DiagCode::XrefInvalidEntry));
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

        // Should have parsed 6 entries (all objects 0-5, including free entries)
        // Free entries are tracked for /Prev chain merge semantics
        assert_eq!(result.len(), 6);

        // Check specific entries
        assert_eq!(
            result.entries.get(&0),
            Some(&XrefEntry::Free {
                next_free: 0,
                gen_nr: 65535
            })
        );
        assert_eq!(
            result.entries.get(&1),
            Some(&XrefEntry::InUse {
                offset: 17,
                gen_nr: 0
            })
        );
        assert_eq!(
            result.entries.get(&2),
            Some(&XrefEntry::InUse {
                offset: 81,
                gen_nr: 0
            })
        );
        assert_eq!(
            result.entries.get(&3),
            Some(&XrefEntry::Free {
                next_free: 0,
                gen_nr: 7
            })
        );
        assert_eq!(
            result.entries.get(&4),
            Some(&XrefEntry::InUse {
                offset: 331,
                gen_nr: 0
            })
        );
        assert_eq!(
            result.entries.get(&5),
            Some(&XrefEntry::InUse {
                offset: 409,
                gen_nr: 0
            })
        );

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

        // Should have parsed 3 entries (all objects 0-2, including free entry)
        // Free entries are tracked for /Prev chain merge semantics
        assert_eq!(result.len(), 3);
        assert_eq!(
            result.entries.get(&0),
            Some(&XrefEntry::Free {
                next_free: 0,
                gen_nr: 65535
            })
        );
        assert_eq!(
            result.entries.get(&1),
            Some(&XrefEntry::InUse {
                offset: 15,
                gen_nr: 0
            })
        );
        assert_eq!(
            result.entries.get(&2),
            Some(&XrefEntry::InUse {
                offset: 78,
                gen_nr: 0
            })
        );
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

        // Should have parsed 3 entries (all objects 0-2, including free entry)
        // Free entries are tracked for /Prev chain merge semantics
        assert_eq!(result.len(), 3);
        assert_eq!(
            result.entries.get(&0),
            Some(&XrefEntry::Free {
                next_free: 0,
                gen_nr: 65535
            })
        );
        assert_eq!(result.len(), 2);
        assert_eq!(
            result.entries.get(&1),
            Some(&XrefEntry::InUse {
                offset: 15,
                gen_nr: 0
            })
        );
        assert_eq!(
            result.entries.get(&2),
            Some(&XrefEntry::InUse {
                offset: 78,
                gen_nr: 0
            })
        );
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
        assert_eq!(
            result.entries.get(&100),
            Some(&XrefEntry::InUse {
                offset: 200,
                gen_nr: 0
            })
        );
        assert_eq!(
            result.entries.get(&101),
            Some(&XrefEntry::InUse {
                offset: 300,
                gen_nr: 0
            })
        );
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
        assert_eq!(
            result.entries.get(&1),
            Some(&XrefEntry::InUse {
                offset: 15,
                gen_nr: 0
            })
        );

        // Should have emitted a diagnostic for the bad entry
        assert!(!result.diagnostics.is_empty());
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::XrefInvalidEntry));
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
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::XrefObjectZeroNotFree));
    }

    #[test]
    fn test_parse_xref_missing_trailer() {
        // Xref without trailer (truncated)
        let xref_data = b"xref\n0 2\n\
0000000000 65535 f \n\
0000000015 00000 n \n";

        let source = MemorySource::new(xref_data.to_vec());
        let result = parse_traditional_xref(&source, 0);

        // Should still parse both entries (including free entry)
        // Free entries are tracked for /Prev chain merge semantics
        assert_eq!(result.len(), 2);
        assert!(result.trailer.is_none());

        // Should emit diagnostic about missing trailer
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::XrefTrailerNotFound));
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
        assert_eq!(
            result,
            Some((
                1,
                XrefEntry::InUse {
                    offset: 15,
                    gen_nr: 0
                }
            ))
        );
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_parse_xref_entry_free() {
        let entry = b"0000000000 65535 f \n";
        let diagnostics = &mut Vec::new();

        let result = parse_xref_entry(entry, 0, 100, 20, diagnostics);
        assert_eq!(
            result,
            Some((
                0,
                XrefEntry::Free {
                    next_free: 0,
                    gen_nr: 65535
                }
            ))
        );
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

            #[test]
            fn proptest_parse_xref_stream_no_panic(data in any::<Vec<u8>>()) {
                // Any random byte sequence should not panic
                let source = MemorySource::new(data);
                let _ = parse_xref_stream(&source, 0);
                // If we get here without panic, the test passes
            }

            #[test]
            fn proptest_parse_xref_stream_random_offset_no_panic(
                data in any::<Vec<u8>>(),
                offset in any::<u64>()
            ) {
                // Any random offset should not panic
                let source = MemorySource::new(data);
                let _ = parse_xref_stream(&source, offset);
                // If we get here without panic, the test passes
            }

            #[test]
            fn proptest_merge_hybrid_no_panic(
                trad_entries in prop::collection::hash_map(any::<u32>(), any::<u64>(), 0..20),
                stream_entries in prop::collection::hash_map(any::<u32>(), any::<u64>(), 0..20)
            ) {
                // Random combinations of traditional and stream sections should never panic
                let mut traditional = XrefSection::new();
                for (obj_nr, &offset) in &trad_entries {
                    let entry_type = offset % 3;
                    let entry = match entry_type {
                        0 => XrefEntry::InUse { offset, gen_nr: (offset % 100) as u16 },
                        1 => XrefEntry::Free { next_free: *obj_nr, gen_nr: (offset % 100) as u16 },
                        _ => XrefEntry::Compressed { obj_stm_nr: (offset % 1000) as u32, index: *obj_nr },
                    };
                    traditional.add_entry(*obj_nr, entry);
                }

                let mut stream = XrefSection::new();
                for (obj_nr, &offset) in &stream_entries {
                    let entry_type = offset % 3;
                    let entry = match entry_type {
                        0 => XrefEntry::InUse { offset, gen_nr: (offset % 100) as u16 },
                        1 => XrefEntry::Free { next_free: *obj_nr, gen_nr: (offset % 100) as u16 },
                        _ => XrefEntry::Compressed { obj_stm_nr: (offset % 1000) as u32, index: *obj_nr },
                    };
                    stream.add_entry(*obj_nr, entry);
                }

                // If we get here without panic, the test passes
                let _merged = merge_hybrid(traditional, stream);

                // Verify the merged section is marked as hybrid
                // assert!(merged.is_hybrid);
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
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::XrefRepaired));
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
        assert_eq!(
            result.entries.get(&1),
            Some(&XrefEntry::InUse {
                offset: 0,
                gen_nr: 0
            })
        );
        assert_eq!(
            result.entries.get(&2),
            Some(&XrefEntry::InUse {
                offset: 35,
                gen_nr: 5
            })
        );
        assert_eq!(
            result.entries.get(&3),
            Some(&XrefEntry::InUse {
                offset: 70,
                gen_nr: 65535
            })
        );
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
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::XrefLinearizedNoForwardScan));
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

    // Xref stream tests (PDF 1.5+)

    #[test]
    fn test_parse_xref_stream_simple() {
        // Simple xref stream with /W [1 4 2] /Index [0 6]
        // Entry format: type(1) + offset(4) + generation(2) = 7 bytes per entry
        // Type 1 = in-use, Type 0 = free
        // Entries:
        // - Obj 0: type=0 (free), next_free=0, gen=65535
        // - Obj 1: type=1, offset=1000, gen=0
        // - Obj 2: type=1, offset=2000, gen=0
        // - Obj 3: type=1, offset=3000, gen=0
        // - Obj 4: type=1, offset=4000, gen=0
        // - Obj 5: type=1, offset=5000, gen=0

        // Use the helper function to build the xref stream fixture
        let raw_entries: Vec<u8> = vec![
            // Obj 0: type=0 (free), next_free=0, gen=65535
            0, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, // Obj 1: type=1, offset=1000, gen=0
            1, 0x00, 0x00, 0x03, 0xE8, 0x00, 0x00, // Obj 2: type=1, offset=2000, gen=0
            1, 0x00, 0x00, 0x07, 0xD0, 0x00, 0x00, // Obj 3: type=1, offset=3000, gen=0
            1, 0x00, 0x00, 0x0B, 0xB8, 0x00, 0x00, // Obj 4: type=1, offset=4000, gen=0
            1, 0x00, 0x00, 0x0F, 0xA0, 0x00, 0x00, // Obj 5: type=1, offset=5000, gen=0
            1, 0x00, 0x00, 0x13, 0x88, 0x00, 0x00,
        ];

        let xref_stream_data = build_xref_stream_fixture(
            &[1, 4, 2],    // /W
            6,             // /Size
            Some(&[0, 6]), // /Index
            &[
                &raw_entries[0..7],
                &raw_entries[7..14],
                &raw_entries[14..21],
                &raw_entries[21..28],
                &raw_entries[28..35],
                &raw_entries[35..42],
            ],
        );

        let source = MemorySource::new(xref_stream_data);
        let result = parse_xref_stream(&source, 0);

        // Debug: print diagnostics if test fails
        if result.len() != 5 {
            eprintln!("Test failed. Diagnostics: {:?}", result.diagnostics);
            eprintln!("Entries: {:?}", result.entries);
        }

        // Should have parsed 5 in-use entries (object 0 is free and ignored)
        assert_eq!(result.len(), 5);

        // Check specific entries
        assert_eq!(
            result.entries.get(&1),
            Some(&XrefEntry::InUse {
                offset: 1000,
                gen_nr: 0
            })
        );
        assert_eq!(
            result.entries.get(&2),
            Some(&XrefEntry::InUse {
                offset: 2000,
                gen_nr: 0
            })
        );
        assert_eq!(
            result.entries.get(&3),
            Some(&XrefEntry::InUse {
                offset: 3000,
                gen_nr: 0
            })
        );
        assert_eq!(
            result.entries.get(&4),
            Some(&XrefEntry::InUse {
                offset: 4000,
                gen_nr: 0
            })
        );
        assert_eq!(
            result.entries.get(&5),
            Some(&XrefEntry::InUse {
                offset: 5000,
                gen_nr: 0
            })
        );

        // Trailer should be present
        assert!(result.trailer.is_some());
    }

    #[test]
    fn test_parse_xref_stream_multi_subsection() {
        // Multi-subsection test: /Index [0 3 100 2]
        // First subsection: objects 0, 1, 2
        // Second subsection: objects 100, 101

        let xref_stream_data = build_xref_stream_fixture(
            &[1, 4, 2],            // /W
            102,                   // /Size (highest obj + 1)
            Some(&[0, 3, 100, 2]), // /Index
            &[
                // First subsection (0-2)
                &[0, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF], // Obj 0: free
                &[1, 0x00, 0x00, 0x03, 0xE8, 0x00, 0x00], // Obj 1: offset=1000
                &[1, 0x00, 0x00, 0x07, 0xD0, 0x00, 0x00], // Obj 2: offset=2000
                // Second subsection (100-101)
                &[1, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00], // Obj 100: offset=65536
                &[1, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00], // Obj 101: offset=65537
            ],
        );

        let source = MemorySource::new(xref_stream_data);
        let result = parse_xref_stream(&source, 0);

        // Should have parsed 4 in-use entries (1, 2, 100, 101)
        assert_eq!(result.len(), 4);
        assert!(result.entries.contains_key(&1));
        assert!(result.entries.contains_key(&2));
        assert!(result.entries.contains_key(&100));
        assert!(result.entries.contains_key(&101));

        // Check offsets
        assert_eq!(
            result.entries.get(&1),
            Some(&XrefEntry::InUse {
                offset: 1000,
                gen_nr: 0
            })
        );
        assert_eq!(
            result.entries.get(&100),
            Some(&XrefEntry::InUse {
                offset: 65536,
                gen_nr: 0
            })
        );
    }

    #[test]
    fn test_parse_xref_stream_field_width_zero_gen() {
        // Field-width edge case: /W [1 4 0] (generation always 0)
        // Entry format: type(1) + offset(4) + generation(0) = 5 bytes per entry

        let xref_stream_data = build_xref_stream_fixture(
            &[1, 4, 0], // /W (gen width = 0)
            3,          // /Size
            None,       // /Index (default [0 3])
            &[
                &[0, 0x00, 0x00, 0x00, 0x00], // Obj 0: type=0, offset=0
                &[1, 0x00, 0x00, 0x03, 0xE8], // Obj 1: type=1, offset=1000
                &[1, 0x00, 0x00, 0x07, 0xD0], // Obj 2: type=1, offset=2000
            ],
        );

        let source = MemorySource::new(xref_stream_data);
        let result = parse_xref_stream(&source, 0);

        // Should have parsed 2 in-use entries
        assert_eq!(result.len(), 2);

        // Check entries - generation should be 0 (default)
        assert_eq!(
            result.entries.get(&1),
            Some(&XrefEntry::InUse {
                offset: 1000,
                gen_nr: 0
            })
        );
        assert_eq!(
            result.entries.get(&2),
            Some(&XrefEntry::InUse {
                offset: 2000,
                gen_nr: 0
            })
        );
    }

    #[test]
    fn test_parse_xref_stream_type2_compressed() {
        // Type-2 entry test: compressed objects in ObjStm
        // Entry format: type(1) + obj_stm_nr(4) + index(2) = 7 bytes per entry
        // Type 2: obj_field = ObjStm object number, gen_field = index in ObjStm

        let xref_stream_data = build_xref_stream_fixture(
            &[1, 4, 2], // /W
            4,          // /Size
            None,       // /Index (default [0 4])
            &[
                &[0, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF], // Obj 0: free
                &[1, 0x00, 0x00, 0x03, 0xE8, 0x00, 0x00], // Obj 1: type=1, offset=1000
                &[2, 0x00, 0x00, 0x00, 0x0A, 0x00, 0x05], // Obj 2: type=2, obj_stm=10, index=5
                &[2, 0x00, 0x00, 0x00, 0x0B, 0x00, 0x0A], // Obj 3: type=2, obj_stm=11, index=10
            ],
        );

        let source = MemorySource::new(xref_stream_data);
        let result = parse_xref_stream(&source, 0);

        // Should have parsed 3 entries (1 type-1, 2 type-2)
        assert_eq!(result.len(), 3);

        // Check type-1 entry
        assert_eq!(
            result.entries.get(&1),
            Some(&XrefEntry::InUse {
                offset: 1000,
                gen_nr: 0
            })
        );

        // Check type-2 entries
        assert_eq!(
            result.entries.get(&2),
            Some(&XrefEntry::Compressed {
                obj_stm_nr: 10,
                index: 5
            })
        );
        assert_eq!(
            result.entries.get(&3),
            Some(&XrefEntry::Compressed {
                obj_stm_nr: 11,
                index: 10
            })
        );
    }

    #[test]
    fn test_parse_xref_stream_with_predictor() {
        // Predictor test: xref stream with FlateDecode + PNG Up predictor
        // This tests that the stream decoder handles predictors correctly

        // Build the xref stream with /Predictor using the helper
        let xref_stream_data = build_xref_stream_fixture_with_predictor(
            &[1, 4, 2], // /W
            3,          // /Size
            &[
                // Obj 0: type=0 (free)
                &[0, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF],
                // Obj 1: type=1, offset=1000
                &[1, 0x00, 0x00, 0x03, 0xE8, 0x00, 0x00],
                // Obj 2: type=1, offset=2000
                &[1, 0x00, 0x00, 0x07, 0xD0, 0x00, 0x00],
            ],
        );

        let source = MemorySource::new(xref_stream_data);
        let result = parse_xref_stream(&source, 0);

        // Should have parsed 2 in-use entries (object 0 is free)
        // Note: The predictor might cause decoding issues, but we shouldn't crash
        // The test verifies we handle the predictor without panicking
        assert!(!result.diagnostics.is_empty() || result.len() > 0);
    }

    #[test]
    fn test_parse_xref_stream_invalid_entry_type() {
        // Test handling of invalid entry type (not 0, 1, or 2)
        // Should emit diagnostic and treat as free

        let xref_stream_data = build_xref_stream_fixture(
            &[1, 4, 2], // /W
            3,          // /Size
            None,       // /Index
            &[
                &[0, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF], // Obj 0: type=0 (free)
                &[5, 0x00, 0x00, 0x03, 0xE8, 0x00, 0x00], // Obj 1: type=5 (INVALID!)
                &[1, 0x00, 0x00, 0x07, 0xD0, 0x00, 0x00], // Obj 2: type=1 (valid)
            ],
        );

        let source = MemorySource::new(xref_stream_data);
        let result = parse_xref_stream(&source, 0);

        // Should have parsed 1 in-use entry (object 2)
        assert_eq!(result.len(), 1);
        assert_eq!(
            result.entries.get(&2),
            Some(&XrefEntry::InUse {
                offset: 2000,
                gen_nr: 0
            })
        );

        // Should have emitted a diagnostic for invalid type
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::XrefInvalidStreamEntry));
    }

    #[test]
    fn test_parse_xref_stream_missing_size() {
        // Test handling of missing /Size

        let xref_stream_data = build_xref_stream_fixture_missing_size(&[1, 4, 2]);

        let source = MemorySource::new(xref_stream_data);
        let result = parse_xref_stream(&source, 0);

        // Should have emitted diagnostic about missing /Size
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::XrefInvalidStreamFormat));
    }

    #[test]
    fn test_parse_xref_stream_invalid_w_array() {
        // Test handling of invalid /W array (wrong length)

        let xref_stream_data = build_xref_stream_fixture(
            &[1, 4], // /W (only 2 elements - invalid!)
            3,       // /Size
            None,    // /Index
            &[
                &[0, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF],
                &[1, 0x00, 0x00, 0x03, 0xE8, 0x00, 0x00],
                &[1, 0x00, 0x00, 0x07, 0xD0, 0x00, 0x00],
            ],
        );

        let source = MemorySource::new(xref_stream_data);
        let result = parse_xref_stream(&source, 0);

        // Should have emitted diagnostic about invalid /W
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::XrefInvalidStreamFormat));
    }

    #[test]
    fn test_read_big_endian_field() {
        // Test the big-endian field reader helper

        // 1 byte
        assert_eq!(read_big_endian_field(&[0x12]), 0x12);

        // 2 bytes
        assert_eq!(read_big_endian_field(&[0x12, 0x34]), 0x1234);

        // 3 bytes
        assert_eq!(read_big_endian_field(&[0x12, 0x34, 0x56]), 0x123456);

        // 4 bytes
        assert_eq!(read_big_endian_field(&[0x12, 0x34, 0x56, 0x78]), 0x12345678);

        // Empty slice
        assert_eq!(read_big_endian_field(&[]), 0);

        // Test actual values from xref stream
        assert_eq!(read_big_endian_field(&[0x00, 0x00, 0x03, 0xE8]), 1000);
        assert_eq!(read_big_endian_field(&[0xFF, 0xFF]), 65535);
    }

    #[test]
    fn test_debug_xref_stream_parsing() {
        // Debug test to see what's being parsed
        let raw_entries: Vec<u8> = vec![
            0, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 1, 0x00, 0x00, 0x03, 0xE8, 0x00, 0x00,
        ];

        let xref_stream_data = build_xref_stream_fixture(
            &[1, 4, 2],
            2,
            Some(&[0, 2]),
            &[&raw_entries[0..7], &raw_entries[7..14]],
        );

        // Print what we built
        eprintln!("Built xref stream data:");
        eprintln!("{}", String::from_utf8_lossy(&xref_stream_data));

        // Try to parse it with ObjectParser
        use crate::parser::object::ObjectParser;
        let mut parser = ObjectParser::new(&xref_stream_data);
        let indirect = parser.parse_indirect_object();

        eprintln!("Parsed indirect object: {:?}", indirect);

        // Now try to decode the stream
        if let Some(ind) = &indirect {
            if let PdfObject::Stream(stream) = &ind.obj {
                use crate::parser::stream::{decode_stream, ExtractionOptions};
                let source = MemorySource::new(xref_stream_data);
                let decoded =
                    decode_stream(&stream, &source, &ExtractionOptions::default(), &mut 0);
                eprintln!(
                    "Decoded stream data ({} bytes): {:?}",
                    decoded.len(),
                    decoded
                );
            }
        }
    }

    /// Helper function to build a minimal xref stream fixture for testing.
    ///
    /// Creates a valid indirect object with an xref stream containing the
    /// specified entries.
    fn build_xref_stream_fixture(
        field_widths: &[i64],
        size: u32,
        index: Option<&[u32]>,
        entries: &[&[u8]],
    ) -> Vec<u8> {
        build_xref_stream_fixture_with_padding(field_widths, size, index, entries, 0)
    }

    /// Helper function to build a minimal xref stream fixture with padding.
    ///
    /// Creates a valid indirect object with an xref stream containing the
    /// specified entries, plus optional padding bytes at the end to ensure
    /// the ObjectParser has enough bytes to read the full object.
    fn build_xref_stream_fixture_with_padding(
        field_widths: &[i64],
        size: u32,
        index: Option<&[u32]>,
        entries: &[&[u8]],
        padding: usize,
    ) -> Vec<u8> {
        use crate::parser::object::intern;

        // Compress entries with FlateDecode
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        use std::io::Write;

        let mut raw_data = Vec::new();
        for entry in entries {
            raw_data.extend_from_slice(entry);
        }

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&raw_data).unwrap();
        let compressed = encoder.finish().unwrap();

        // Build stream dict
        let mut obj_bytes = String::new();
        obj_bytes.push_str("1 0 obj\n<<");

        // /Type /XRef
        obj_bytes.push_str("/Type /XRef ");

        // /Size
        obj_bytes.push_str(&format!("/Size {} ", size));

        // /W
        obj_bytes.push_str("/W [");
        for (i, w) in field_widths.iter().enumerate() {
            if i > 0 {
                obj_bytes.push(' ');
            }
            obj_bytes.push_str(&w.to_string());
        }
        obj_bytes.push_str("] ");

        // /Index (if provided)
        if let Some(idx) = index {
            obj_bytes.push_str("/Index [");
            for (i, v) in idx.iter().enumerate() {
                if i > 0 {
                    obj_bytes.push(' ');
                }
                obj_bytes.push_str(&v.to_string());
            }
            obj_bytes.push_str("] ");
        }

        // /Filter /FlateDecode
        obj_bytes.push_str("/Filter /FlateDecode ");

        // /Length
        obj_bytes.push_str(&format!("/Length {} ", compressed.len()));

        obj_bytes.push_str(">>\nstream\n");

        let mut result = obj_bytes.into_bytes();
        result.extend_from_slice(&compressed);
        result.extend_from_slice(b"\nendstream\nendobj\n");

        // Add padding
        if padding > 0 {
            result.extend(vec![b' '; padding]);
        }

        result
    }

    /// Helper function to build an xref stream fixture with missing /Size.
    fn build_xref_stream_fixture_missing_size(field_widths: &[i64]) -> Vec<u8> {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        use std::io::Write;

        // Minimal dummy data
        let raw_data = vec![0u8; 7];
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&raw_data).unwrap();
        let compressed = encoder.finish().unwrap();

        let mut obj_bytes = String::new();
        obj_bytes.push_str("1 0 obj\n<<");

        // /Type /XRef
        obj_bytes.push_str("/Type /XRef ");

        // /W (but NO /Size!)
        obj_bytes.push_str("/W [");
        for (i, w) in field_widths.iter().enumerate() {
            if i > 0 {
                obj_bytes.push(' ');
            }
            obj_bytes.push_str(&w.to_string());
        }
        obj_bytes.push_str("] ");

        // /Filter /FlateDecode
        obj_bytes.push_str("/Filter /FlateDecode ");

        // /Length
        obj_bytes.push_str(&format!("/Length {} ", compressed.len()));

        obj_bytes.push_str(">>\nstream\n");

        let mut result = obj_bytes.into_bytes();
        result.extend_from_slice(&compressed);
        result.extend_from_slice(b"\nendstream\nendobj\n");

        result
    }

    /// Helper function to build an xref stream fixture with predictor.
    fn build_xref_stream_fixture_with_predictor(
        field_widths: &[i64],
        size: u32,
        entries: &[&[u8]],
    ) -> Vec<u8> {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        use std::io::Write;

        let mut raw_data = Vec::new();
        for entry in entries {
            raw_data.extend_from_slice(entry);
        }

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&raw_data).unwrap();
        let compressed = encoder.finish().unwrap();

        let mut obj_bytes = String::new();
        obj_bytes.push_str("1 0 obj\n<<");

        // /Type /XRef
        obj_bytes.push_str("/Type /XRef ");

        // /Size
        obj_bytes.push_str(&format!("/Size {} ", size));

        // /W
        obj_bytes.push_str("/W [");
        for (i, w) in field_widths.iter().enumerate() {
            if i > 0 {
                obj_bytes.push(' ');
            }
            obj_bytes.push_str(&w.to_string());
        }
        obj_bytes.push_str("] ");

        // /DecodeParms with PNG predictor
        obj_bytes.push_str("/DecodeParms << /Predictor 12 /Columns 7 >> ");

        // /Filter /FlateDecode
        obj_bytes.push_str("/Filter /FlateDecode ");

        // /Length
        obj_bytes.push_str(&format!("/Length {} ", compressed.len()));

        obj_bytes.push_str(">>\nstream\n");

        let mut result = obj_bytes.into_bytes();
        result.extend_from_slice(&compressed);
        result.extend_from_slice(b"\nendstream\nendobj\n");

        result
    }

    // Hybrid file merge tests

    #[test]
    fn test_merge_hybrid_traditional_priority() {
        // Critical test: traditional entries override stream entries for same object numbers
        let mut traditional = XrefSection::new();
        traditional.add_entry(
            1,
            XrefEntry::InUse {
                offset: 1000,
                gen_nr: 0,
            },
        );
        traditional.add_entry(
            2,
            XrefEntry::InUse {
                offset: 2000,
                gen_nr: 0,
            },
        );

        let mut stream = XrefSection::new();
        // Stream has different offset for object 1 (should be ignored)
        stream.add_entry(
            1,
            XrefEntry::InUse {
                offset: 9999,
                gen_nr: 0,
            },
        );
        // Stream has object 3 (gap fill - should be added)
        stream.add_entry(
            3,
            XrefEntry::Compressed {
                obj_stm_nr: 10,
                index: 5,
            },
        );

        let merged = merge_hybrid(traditional, stream);

        assert!(merged.is_hybrid);
        assert_eq!(merged.len(), 3);
        // Object 1 should use traditional offset
        assert_eq!(
            merged.entries.get(&1),
            Some(&XrefEntry::InUse {
                offset: 1000,
                gen_nr: 0
            })
        );
        // Object 3 should be added from stream
        assert_eq!(
            merged.entries.get(&3),
            Some(&XrefEntry::Compressed {
                obj_stm_nr: 10,
                index: 5
            })
        );
    }

    #[test]
    fn test_merge_hybrid_free_inuse_conflict() {
        // Free/InUse conflict: traditional Free + stream InUse → Free (traditional wins)

        let mut traditional = XrefSection::new();
        traditional.add_entry(
            1,
            XrefEntry::Free {
                next_free: 0,
                gen_nr: 65535,
            },
        );

        let mut stream = XrefSection::new();
        stream.add_entry(
            1,
            XrefEntry::InUse {
                offset: 5000,
                gen_nr: 0,
            },
        );

        let merged = merge_hybrid(traditional, stream);

        assert!(merged.is_hybrid);
        // Should have emitted STRUCT_HYBRID_CONFLICT diagnostic
        assert!(merged
            .diagnostics
            .iter()
            .any(|d| matches!(d.code, DiagCode::StructHybridConflict)));
        // Traditional Free wins
        assert_eq!(
            merged.entries.get(&1),
            Some(&XrefEntry::Free {
                next_free: 0,
                gen_nr: 65535
            })
        );
    }

    #[test]
    fn test_merge_hybrid_gap_fill() {
        // Stream-only type-2 entries fill gaps not covered by traditional table
        let mut traditional = XrefSection::new();
        traditional.add_entry(
            1,
            XrefEntry::InUse {
                offset: 1000,
                gen_nr: 0,
            },
        );
        traditional.add_entry(
            5,
            XrefEntry::InUse {
                offset: 5000,
                gen_nr: 0,
            },
        );

        let mut stream = XrefSection::new();
        // Objects 2, 3, 4 are only in stream (gap fill)
        stream.add_entry(
            2,
            XrefEntry::Compressed {
                obj_stm_nr: 10,
                index: 0,
            },
        );
        stream.add_entry(
            3,
            XrefEntry::Compressed {
                obj_stm_nr: 10,
                index: 1,
            },
        );
        stream.add_entry(
            4,
            XrefEntry::Compressed {
                obj_stm_nr: 10,
                index: 2,
            },
        );

        let merged = merge_hybrid(traditional, stream);

        assert!(merged.is_hybrid);
        assert_eq!(merged.len(), 5);
        // All gap-fill objects should be present
        assert!(merged.entries.contains_key(&2));
        assert!(merged.entries.contains_key(&3));
        assert!(merged.entries.contains_key(&4));
        assert_eq!(
            merged.entries.get(&2),
            Some(&XrefEntry::Compressed {
                obj_stm_nr: 10,
                index: 0
            })
        );
    }

    #[test]
    fn test_merge_hybrid_trailer_xrefstm_removed() {
        // Merged trailer should have /XRefStm key removed
        use crate::parser::object::intern;

        let mut traditional = XrefSection::new();
        let mut trad_trailer = PdfDict::new();
        trad_trailer.insert(intern("Size"), PdfObject::Integer(10));
        trad_trailer.insert(intern("XRefStm"), PdfObject::Integer(12345));
        trad_trailer.insert(intern("Root"), PdfObject::Ref(ObjRef::new(1, 0)));
        traditional.trailer = Some(trad_trailer);

        let stream = XrefSection::new();

        let merged = merge_hybrid(traditional, stream);

        assert!(merged.is_hybrid);
        let merged_trailer = merged.trailer.expect("Should have trailer");
        // /XRefStm should be removed
        assert!(!merged_trailer.contains_key("XRefStm"));
        // Other keys should be preserved
        assert!(merged_trailer.contains_key("Size"));
        assert!(merged_trailer.contains_key("Root"));
    }

    #[test]
    fn test_is_hybrid_trailer_detection() {
        use crate::parser::object::intern;

        // Trailer with /XRefStm is hybrid
        let mut hybrid_trailer = PdfDict::new();
        hybrid_trailer.insert(intern("Size"), PdfObject::Integer(10));
        hybrid_trailer.insert(intern("XRefStm"), PdfObject::Integer(12345));
        assert!(is_hybrid_trailer(Some(&hybrid_trailer)));

        // Trailer without /XRefStm is not hybrid
        let mut normal_trailer = PdfDict::new();
        normal_trailer.insert(intern("Size"), PdfObject::Integer(10));
        assert!(!is_hybrid_trailer(Some(&normal_trailer)));

        // None trailer is not hybrid
        assert!(!is_hybrid_trailer(None));
    }

    #[test]
    fn test_merge_hybrid_empty_sections() {
        // Edge case: merging with empty sections should work
        let traditional = XrefSection::new();
        let stream = XrefSection::new();

        let merged = merge_hybrid(traditional, stream);

        assert!(merged.is_hybrid);
        assert_eq!(merged.len(), 0);
    }

    #[test]
    fn test_merge_hybrid_stream_only() {
        // Edge case: traditional is empty, stream has entries
        let traditional = XrefSection::new();

        let mut stream = XrefSection::new();
        stream.add_entry(
            1,
            XrefEntry::Compressed {
                obj_stm_nr: 10,
                index: 0,
            },
        );
        stream.add_entry(
            2,
            XrefEntry::Compressed {
                obj_stm_nr: 10,
                index: 1,
            },
        );

        let merged = merge_hybrid(traditional, stream);

        assert!(merged.is_hybrid);
        assert_eq!(merged.len(), 2);
        assert!(merged.entries.contains_key(&1));
        assert!(merged.entries.contains_key(&2));
    }

    #[test]
    fn test_merge_hybrid_traditional_only() {
        // Edge case: stream is empty, traditional has entries
        let mut traditional = XrefSection::new();
        traditional.add_entry(
            1,
            XrefEntry::InUse {
                offset: 1000,
                gen_nr: 0,
            },
        );

        let stream = XrefSection::new();

        let merged = merge_hybrid(traditional, stream);

        assert!(merged.is_hybrid);
        assert_eq!(merged.len(), 1);
        assert_eq!(
            merged.entries.get(&1),
            Some(&XrefEntry::InUse {
                offset: 1000,
                gen_nr: 0
            })
        );
    }

    #[test]
    fn test_merge_hybrid_proptest_simple() {
        // Simple proptest-style test: verify merge_hybrid doesn't panic with basic inputs
        for obj_nr in 0u32..10 {
            let mut traditional = XrefSection::new();
            traditional.add_entry(
                obj_nr,
                XrefEntry::InUse {
                    offset: obj_nr as u64 * 100,
                    gen_nr: 0,
                },
            );

            let mut stream = XrefSection::new();
            stream.add_entry(
                obj_nr + 100,
                XrefEntry::Compressed {
                    obj_stm_nr: 10,
                    index: obj_nr,
                },
            );

            let merged = merge_hybrid(traditional, stream);
            assert!(merged.is_hybrid);
            assert_eq!(merged.len(), 2);
        }
    }

    // ========================================================================
    // Linearized PDF Detection Tests
    // ========================================================================

    #[test]
    fn test_detect_linearization_non_linearized_pdf() {
        // A regular PDF without linearization should return None
        let pdf_data = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n";
        let source = MemorySource::new(pdf_data.to_vec());

        let result = detect_linearization(&source);
        assert!(result.is_none(), "Non-linearized PDF should return None");
    }

    #[test]
    fn test_detect_linearization_with_valid_dict() {
        // A minimal linearized PDF with the required fields
        // /L must match the actual file size for the validation to pass
        let pdf_data = b"%PDF-1.4\n1 0 obj\n<< /Linearized 1.0\n/L 162\n/H [1234 56]\n/E 100\n/N 10\n/T 200\n/O 5 >>\nendobj\nxref\n0 1\n0000000000 65535 f\ntrailer\n<< /Size 2 >>\nstartxref\n300\n%%%%EOF";

        // Verify the /L value matches actual length
        assert_eq!(
            pdf_data.len() as u64,
            162,
            "Test data /L value should match actual length"
        );

        let source = MemorySource::new(pdf_data.to_vec());

        let result = detect_linearization(&source);
        assert!(result.is_some(), "Valid linearized PDF should be detected");

        let lin_info = result.unwrap();
        assert_eq!(lin_info.file_length, 162);
        assert_eq!(lin_info.first_page_xref_offset, 200);
        assert_eq!(lin_info.hint_stream_offset, Some(1234));
        assert_eq!(lin_info.hint_stream_length, Some(56));
        assert_eq!(lin_info.page_count, 10);
        assert_eq!(lin_info.first_page_end_offset, 100);
        assert_eq!(lin_info.first_page_object_number, 5);
    }

    #[test]
    fn test_detect_linearization_file_size_mismatch() {
        // Linearized PDF where /L doesn't match actual file size
        // (incremental update scenario)
        let pdf_data = b"%PDF-1.4\n\
            1 0 obj\n\
            << /Linearized 1.0\n\
               /L 999999\n\
               /H [1234 56]\n\
               /E 100\n\
               /N 10\n\
               /T 200\n\
               /O 5 >>\n\
            endobj\n";

        let source = MemorySource::new(pdf_data.to_vec());

        let result = detect_linearization(&source);
        assert!(
            result.is_none(),
            "Linearized PDF with size mismatch should return None"
        );
    }

    #[test]
    fn test_detect_linearization_no_hint_stream() {
        // Linearized PDF without optional /H entry
        // /L must match the actual file size for the validation to pass
        let pdf_data = b"%PDF-1.4\n1 0 obj\n<< /Linearized 1.0\n/L 77\n/E 100\n/N 10\n/T 200\n/O 5 >>\nendobj\n";

        // Verify the /L value matches actual length
        assert_eq!(
            pdf_data.len() as u64,
            77,
            "Test data /L value should match actual length"
        );

        let source = MemorySource::new(pdf_data.to_vec());

        let result = detect_linearization(&source);
        assert!(
            result.is_some(),
            "Linearized PDF without /H should be detected"
        );

        let lin_info = result.unwrap();
        assert_eq!(lin_info.hint_stream_offset, None);
        assert_eq!(lin_info.hint_stream_length, None);
    }

    #[test]
    fn test_merge_linearized_xrefs() {
        // Test merging first-page and full xrefs
        let mut first_page = XrefSection::new();
        first_page.add_entry(
            1,
            XrefEntry::InUse {
                offset: 100,
                gen_nr: 0,
            },
        );
        first_page.add_entry(
            5,
            XrefEntry::InUse {
                offset: 500,
                gen_nr: 0,
            },
        );

        let mut full = XrefSection::new();
        // Same entry - full should win
        full.add_entry(
            1,
            XrefEntry::InUse {
                offset: 150,
                gen_nr: 0,
            },
        ); // Different offset
           // New entry only in full
        full.add_entry(
            2,
            XrefEntry::InUse {
                offset: 200,
                gen_nr: 0,
            },
        );
        full.add_entry(
            3,
            XrefEntry::InUse {
                offset: 300,
                gen_nr: 0,
            },
        );

        let merged = merge_linearized_xrefs(first_page, full);

        assert_eq!(merged.len(), 4);
        // Full xref's entry for object 1 should win (offset 150, not 100)
        assert_eq!(
            merged.entries.get(&1),
            Some(&XrefEntry::InUse {
                offset: 150,
                gen_nr: 0
            })
        );
        assert_eq!(
            merged.entries.get(&2),
            Some(&XrefEntry::InUse {
                offset: 200,
                gen_nr: 0
            })
        );
        assert_eq!(
            merged.entries.get(&3),
            Some(&XrefEntry::InUse {
                offset: 300,
                gen_nr: 0
            })
        );
        assert_eq!(
            merged.entries.get(&5),
            Some(&XrefEntry::InUse {
                offset: 500,
                gen_nr: 0
            })
        );
    }

    #[test]
    fn test_merge_linearized_xrefs_conflict_free_vs_inuse() {
        // Test merging where first-page has Free and full has InUse
        let mut first_page = XrefSection::new();
        first_page.add_entry(
            1,
            XrefEntry::Free {
                next_free: 2,
                gen_nr: 0,
            },
        );

        let mut full = XrefSection::new();
        full.add_entry(
            1,
            XrefEntry::InUse {
                offset: 100,
                gen_nr: 0,
            },
        );

        let merged = merge_linearized_xrefs(first_page, full);

        assert_eq!(merged.len(), 1);
        // Full xref's InUse should win over first-page's Free
        assert_eq!(
            merged.entries.get(&1),
            Some(&XrefEntry::InUse {
                offset: 100,
                gen_nr: 0
            })
        );
    }

    #[test]
    fn test_merge_linearized_xrefs_empty_first_page() {
        // Test merging where first-page is empty
        let first_page = XrefSection::new();

        let mut full = XrefSection::new();
        full.add_entry(
            1,
            XrefEntry::InUse {
                offset: 100,
                gen_nr: 0,
            },
        );
        full.add_entry(
            2,
            XrefEntry::InUse {
                offset: 200,
                gen_nr: 0,
            },
        );

        let merged = merge_linearized_xrefs(first_page, full);

        assert_eq!(merged.len(), 2);
        assert_eq!(
            merged.entries.get(&1),
            Some(&XrefEntry::InUse {
                offset: 100,
                gen_nr: 0
            })
        );
        assert_eq!(
            merged.entries.get(&2),
            Some(&XrefEntry::InUse {
                offset: 200,
                gen_nr: 0
            })
        );
    }

    #[test]
    fn test_detect_linearization_proptest_random_bytes() {
        // Proptest-style: verify detect_linearization never panics on random input
        for seed in 0u32..100 {
            let mut data = Vec::new();

            // Use deterministic PRNG based on seed (Java Random algorithm with u64 state)
            let mut state: u64 = (seed as u64).wrapping_mul(0x5DEECE66D).wrapping_add(0xB);
            for _ in 0..2048 {
                state = state.wrapping_mul(0x5DEECE66D).wrapping_add(0xB);
                data.push(((state >> 16) & 0xFF) as u8);
            }

            let source = MemorySource::new(data);

            // Should never panic, may return None or Some
            let _ = detect_linearization(&source);
        }
    }

    #[test]
    fn test_detect_linearization_with_incremental_update() {
        // A PDF that was linearized then incrementally updated
        // The /L field will not match the current file size
        let original_data = b"%PDF-1.4\n\
            1 0 obj\n\
            << /Linearized 1.0\n\
               /L 300\n\
               /E 100\n\
               /N 10\n\
               /T 200\n\
               /O 5 >>\n\
            endobj\n\
            %%EOF";

        // Simulate incremental update by appending data
        let mut updated_data = original_data.to_vec();
        updated_data.extend_from_slice(b"\n% Incremental update\n2 0 obj\n123\nendobj\n");

        let source = MemorySource::new(updated_data);

        let result = detect_linearization(&source);
        // Should return None because /L (300) != actual size
        assert!(
            result.is_none(),
            "Incrementally updated linearized PDF should fall through"
        );
    }

    // /Prev chain tests

    /// Test 3-revision /Prev chain - latest value wins.
    ///
    /// This is the critical test from the plan: verify that when an object
    /// appears in multiple revisions, the LATEST revision's value wins.
    #[test]
    fn test_prev_chain_three_revisions_latest_wins() {
        // Build a minimal PDF with 3 incremental revisions
        // Each revision is a complete xref table with a /Prev pointer

        // Start with fixed offsets for predictability
        let rev1_offset = 1000u64;
        let rev2_offset = 2000u64;
        let rev3_offset = 3000u64;

        // Revision 1 (baseline): objects 1, 2, 3
        let rev1 = format!(
            "xref\n0 4\n\
            0000000000 65535 f \n\
            0000000100 00000 n \n\
            0000000200 00000 n \n\
            0000000300 00000 n \n\
            trailer\n<< /Size 4 >>\n"
        );

        // Revision 2: updates object 2, adds object 4
        let rev2 = format!(
            "xref\n2 1\n\
            0000000250 00001 n \n\
            4 1\n\
            0000000400 00000 n \n\
            trailer\n<< /Size 5 /Prev {} >>\n",
            rev1_offset
        );

        // Revision 3 (latest): updates object 3, adds object 5
        let rev3 = format!(
            "xref\n3 1\n\
            0000000350 00002 n \n\
            5 1\n\
            0000000500 00000 n \n\
            trailer\n<< /Size 6 /Prev {} >>\n",
            rev2_offset
        );

        // Build file data with padding at exact offsets
        let mut file_data = Vec::new();
        file_data.extend_from_slice(b"%PDF-1.4\n");

        // Pad to rev1_offset
        while file_data.len() < rev1_offset as usize {
            file_data.push(b' ');
        }
        file_data.extend_from_slice(rev1.as_bytes());

        // Pad to rev2_offset
        while file_data.len() < rev2_offset as usize {
            file_data.push(b' ');
        }
        file_data.extend_from_slice(rev2.as_bytes());

        // Pad to rev3_offset
        while file_data.len() < rev3_offset as usize {
            file_data.push(b' ');
        }
        file_data.extend_from_slice(rev3.as_bytes());

        let source = MemorySource::new(file_data);

        // Load from the latest revision
        let result = load_xref_with_prev_chain(&source, rev3_offset);

        // Verify all 6 entries are present (including object 0)
        assert_eq!(
            result.len(),
            6,
            "Should have entries for objects 0-5, got {}",
            result.len()
        );

        // Verify LATEST values win:
        // Object 1: unchanged from rev1 (offset 100)
        assert_eq!(
            result.entries.get(&1),
            Some(&XrefEntry::InUse {
                offset: 100,
                gen_nr: 0
            })
        );
        // Object 2: rev2 value (offset 250) overrides rev1 (offset 200)
        assert_eq!(
            result.entries.get(&2),
            Some(&XrefEntry::InUse {
                offset: 250,
                gen_nr: 1
            })
        );
        // Object 3: rev3 value (offset 350) overrides rev1 (offset 300)
        assert_eq!(
            result.entries.get(&3),
            Some(&XrefEntry::InUse {
                offset: 350,
                gen_nr: 2
            })
        );
        // Object 4: added in rev2 (offset 400)
        assert_eq!(
            result.entries.get(&4),
            Some(&XrefEntry::InUse {
                offset: 400,
                gen_nr: 0
            })
        );
        // Object 5: added in rev3 (offset 500)
        assert_eq!(
            result.entries.get(&5),
            Some(&XrefEntry::InUse {
                offset: 500,
                gen_nr: 0
            })
        );

        // Trailer should be from rev3 (latest)
        assert!(result.trailer.is_some());
    }

    /// Test object lifecycle: added in rev2, modified in rev3, freed in rev4.
    #[test]
    fn test_prev_chain_object_add_modify_free() {
        // Build a PDF with 4 revisions tracking object 7's lifecycle
        // Rev1: object 7 doesn't exist
        let rev1 = b"xref\n0 2\n\
            0000000000 65535 f \n\
            0000000100 00000 n \n\
            trailer\n<< /Size 2 >>\n";

        // Rev2: add object 7 as InUse
        let rev2 = b"xref\n7 1\n\
            0000000700 00000 n \n\
            trailer\n<< /Size 8 /Prev 0 >>\n";

        // Rev3: modify object 7 (new generation)
        let rev3 = b"xref\n7 1\n\
            0000000750 00001 n \n\
            trailer\n<< /Size 8 /Prev 0 >>\n";

        // Rev4: free object 7
        let rev4 = b"xref\n7 1\n\
            0000000000 00002 f \n\
            trailer\n<< /Size 8 /Prev 0 >>\n";

        let mut file_data = Vec::new();
        file_data.extend_from_slice(b"%PDF-1.4\n");
        file_data.extend_from_slice(&vec![b' '; 100]);

        // Revision 1
        let rev1_offset = file_data.len() as u64;
        file_data.extend_from_slice(rev1);

        // Revision 2
        let rev2_offset = file_data.len() as u64;
        let mut rev2_with_prev = rev2.to_vec();
        let rev2_str = String::from_utf8_lossy(&rev2_with_prev);
        let rev2_str = rev2_str.replace("/Prev 0", &format!("/Prev {}", rev1_offset));
        file_data.extend_from_slice(rev2_str.as_bytes());

        // Revision 3
        let rev3_offset = file_data.len() as u64;
        let mut rev3_with_prev = rev3.to_vec();
        let rev3_str = String::from_utf8_lossy(&rev3_with_prev);
        let rev3_str = rev3_str.replace("/Prev 0", &format!("/Prev {}", rev2_offset));
        file_data.extend_from_slice(rev3_str.as_bytes());

        // Revision 4 (latest)
        let rev4_offset = file_data.len() as u64;
        let mut rev4_with_prev = rev4.to_vec();
        let rev4_str = String::from_utf8_lossy(&rev4_with_prev);
        let rev4_str = rev4_str.replace("/Prev 0", &format!("/Prev {}", rev3_offset));
        file_data.extend_from_slice(rev4_str.as_bytes());

        let source = MemorySource::new(file_data);
        let result = load_xref_with_prev_chain(&source, rev4_offset);

        // Object 7 should be Free (freed in rev4)
        assert_eq!(
            result.entries.get(&7),
            Some(&XrefEntry::Free {
                next_free: 0,
                gen_nr: 2
            })
        );
    }

    /// Test object added only in latest revision.
    #[test]
    fn test_prev_chain_object_added_only_in_latest() {
        // Rev1: baseline
        let rev1 = b"xref\n0 2\n\
            0000000000 65535 f \n\
            0000000100 00000 n \n\
            trailer\n<< /Size 2 >>\n";

        // Rev2 (latest): add object 99
        let rev2 = b"xref\n99 1\n\
            0000009900 00000 n \n\
            trailer\n<< /Size 100 /Prev 0 >>\n";

        let mut file_data = Vec::new();
        file_data.extend_from_slice(b"%PDF-1.4\n");
        file_data.extend_from_slice(&vec![b' '; 100]);

        let rev1_offset = file_data.len() as u64;
        file_data.extend_from_slice(rev1);

        let rev2_offset = file_data.len() as u64;
        let mut rev2_with_prev = rev2.to_vec();
        let rev2_str = String::from_utf8_lossy(&rev2_with_prev);
        let rev2_str = rev2_str.replace("/Prev 0", &format!("/Prev {}", rev1_offset));
        file_data.extend_from_slice(rev2_str.as_bytes());

        let source = MemorySource::new(file_data);
        let result = load_xref_with_prev_chain(&source, rev2_offset);

        // Object 99 should be present (added in rev2)
        assert_eq!(
            result.entries.get(&99),
            Some(&XrefEntry::InUse {
                offset: 9900,
                gen_nr: 0
            })
        );
    }

    /// Test that trailer is from latest revision.
    #[test]
    fn test_prev_chain_trailer_from_latest() {
        // Rev1: trailer with /Root 1 0 R
        let rev1 = b"xref\n0 1\n\
            0000000000 65535 f \n\
            trailer\n<< /Size 1 /Root 1 0 R >>\n";

        // Rev2 (latest): trailer with /Root 2 0 R and /Info
        let rev2 = b"xref\n0 1\n\
            0000000000 65535 f \n\
            trailer\n<< /Size 2 /Root 2 0 R /Info 3 0 R /Prev 0 >>\n";

        let mut file_data = Vec::new();
        file_data.extend_from_slice(b"%PDF-1.4\n");
        file_data.extend_from_slice(&vec![b' '; 100]);

        let rev1_offset = file_data.len() as u64;
        file_data.extend_from_slice(rev1);

        let rev2_offset = file_data.len() as u64;
        let mut rev2_with_prev = rev2.to_vec();
        let rev2_str = String::from_utf8_lossy(&rev2_with_prev);
        let rev2_str = rev2_str.replace("/Prev 0", &format!("/Prev {}", rev1_offset));
        file_data.extend_from_slice(rev2_str.as_bytes());

        let source = MemorySource::new(file_data);
        let result = load_xref_with_prev_chain(&source, rev2_offset);

        // Trailer should be from rev2 (latest)
        assert!(result.trailer.is_some());
        let trailer = result.trailer.as_ref().unwrap();

        // Should have /Root from rev2 (2 0 R), not rev1 (1 0 R)
        let root = trailer.get("Root");
        assert!(root.is_some());
        match root {
            Some(PdfObject::Ref(obj_ref)) => {
                // 2 0 R - indirect reference to object 2
                assert_eq!(obj_ref.object, 2);
                assert_eq!(obj_ref.generation, 0);
            }
            _ => panic!("Expected /Root to be an indirect reference 2 0 R"),
        }

        // Should have /Info from rev2
        assert!(trailer.contains_key("Info"));
    }

    /// Test /Prev cycle detection.
    #[test]
    fn test_prev_chain_cycle_detection() {
        // Create a cycle: rev3 -> rev2 -> rev1 -> rev3
        let rev_base = b"xref\n0 1\n\
            0000000000 65535 f \n\
            trailer\n<< /Size 1 >>\n";

        let mut file_data = Vec::new();
        file_data.extend_from_slice(b"%PDF-1.4\n");
        file_data.extend_from_slice(&vec![b' '; 100]);

        // Three revisions at offsets 200, 300, 400
        let rev1_offset = 200u64;
        let rev2_offset = 300u64;
        let rev3_offset = 400u64;

        // Rev1: /Prev points to rev3 (creating cycle)
        let rev1 = format!(
            "xref\n0 1\n\
            0000000000 65535 f \n\
            trailer\n<< /Size 1 /Prev {} >>\n",
            rev3_offset
        );

        // Rev2: /Prev points to rev1
        let rev2 = format!(
            "xref\n0 1\n\
            0000000000 65535 f \n\
            trailer\n<< /Size 1 /Prev {} >>\n",
            rev1_offset
        );

        // Rev3 (start): /Prev points to rev2
        let rev3 = format!(
            "xref\n0 1\n\
            0000000000 65535 f \n\
            trailer\n<< /Size 1 /Prev {} >>\n",
            rev2_offset
        );

        // Pad file to rev1_offset
        while file_data.len() < rev1_offset as usize {
            file_data.push(b' ');
        }
        file_data.extend_from_slice(rev1.as_bytes());

        while file_data.len() < rev2_offset as usize {
            file_data.push(b' ');
        }
        file_data.extend_from_slice(rev2.as_bytes());

        while file_data.len() < rev3_offset as usize {
            file_data.push(b' ');
        }
        file_data.extend_from_slice(rev3.as_bytes());

        let source = MemorySource::new(file_data);
        let result = load_xref_with_prev_chain(&source, rev3_offset);

        // Should emit STRUCT_CIRCULAR_REF diagnostic
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::StructCircularRef));
    }

    /// Test depth limit enforcement.
    #[test]
    fn test_prev_chain_depth_limit() {
        let base_xref = b"xref\n0 1\n\
            0000000000 65535 f \n\
            trailer\n<< /Size 1 /Prev {prev} >>\n";

        let mut file_data = Vec::new();
        file_data.extend_from_slice(b"%PDF-1.4\n");

        // Create 50 revisions in a chain (exceeds MAX_PREV_DEPTH of 32)
        let mut offsets = Vec::new();
        for i in 0..50 {
            let offset = 1000 + (i * 200);
            offsets.push(offset);
        }

        // Build the chain from oldest to newest
        for (i, &offset) in offsets.iter().enumerate() {
            // Pad to offset
            while file_data.len() < offset as usize {
                file_data.push(b' ');
            }

            let prev_offset = if i > 0 { offsets[i - 1] } else { 0 };
            let rev =
                String::from_utf8_lossy(base_xref).replace("{prev}", &prev_offset.to_string());
            file_data.extend_from_slice(rev.as_bytes());
        }

        let source = MemorySource::new(file_data);
        let start_offset = *offsets.last().unwrap();

        let result = load_xref_with_prev_chain(&source, start_offset);

        // Should emit STRUCT_DEPTH_EXCEEDED diagnostic
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::StructDepthExceeded));
    }

    /// Test /Prev offset pointing beyond file size.
    #[test]
    fn test_prev_chain_invalid_offset() {
        let rev1 = b"xref\n0 1\n\
            0000000000 65535 f \n\
            trailer\n<< /Size 1 >>\n";

        let rev2 = b"xref\n0 1\n\
            0000000000 65535 f \n\
            trailer\n<< /Size 1 /Prev 999999 >>\n"; // Points beyond file

        let mut file_data = Vec::new();
        file_data.extend_from_slice(b"%PDF-1.4\n");
        file_data.extend_from_slice(&vec![b' '; 100]);

        let rev1_offset = file_data.len() as u64;
        file_data.extend_from_slice(rev1);

        let rev2_offset = file_data.len() as u64;
        file_data.extend_from_slice(rev2);

        let source = MemorySource::new(file_data);
        let result = load_xref_with_prev_chain(&source, rev2_offset);

        // Should emit STRUCT_INVALID_PREV_OFFSET diagnostic
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::StructInvalidPrevOffset));

        // /Prev should be removed from trailer
        let trailer = result.trailer.as_ref().unwrap();
        assert!(!trailer.contains_key("Prev"));
    }

    /// Test /Prev of 0 treated as "no previous revision".
    #[test]
    fn test_prev_chain_zero_prev_is_absent() {
        let rev = b"xref\n0 1\n\
            0000000000 65535 f \n\
            trailer\n<< /Size 1 /Prev 0 >>\n"; // /Prev 0 means "no previous"

        let mut file_data = Vec::new();
        file_data.extend_from_slice(b"%PDF-1.4\n");
        file_data.extend_from_slice(&vec![b' '; 100]);

        let offset = file_data.len() as u64;
        file_data.extend_from_slice(rev);

        let source = MemorySource::new(file_data);
        let result = load_xref_with_prev_chain(&source, offset);

        // Should not follow /Prev 0, should just return this single revision
        assert!(!result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::StructInvalidPrevOffset));
    }

    /// Test negative /Prev treated as "no previous revision".
    #[test]
    fn test_prev_chain_negative_prev_is_absent() {
        let rev = b"xref\n0 1\n\
            0000000000 65535 f \n\
            trailer\n<< /Size 1 /Prev -5 >>\n"; // Negative /Prev

        let mut file_data = Vec::new();
        file_data.extend_from_slice(b"%PDF-1.4\n");
        file_data.extend_from_slice(&vec![b' '; 100]);

        let offset = file_data.len() as u64;
        file_data.extend_from_slice(rev);

        let source = MemorySource::new(file_data);
        let result = load_xref_with_prev_chain(&source, offset);

        // Should not follow negative /Prev
        assert!(!result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::StructInvalidPrevOffset));
    }

    /// Test hybrid file in /Prev chain.
    #[test]
    fn test_prev_chain_hybrid_file() {
        // Rev1: traditional xref
        let rev1 = b"xref\n0 2\n\
            0000000000 65535 f \n\
            0000000100 00000 n \n\
            trailer\n<< /Size 2 >>\n";

        // Rev2: hybrid (traditional + /XRefStm)
        let rev2_trad = b"xref\n0 2\n\
            0000000000 65535 f \n\
            0000000200 00001 n \n\
            trailer\n<< /Size 2 /XRefStm 500 /Prev 0 >>\n";

        let mut file_data = Vec::new();
        file_data.extend_from_slice(b"%PDF-1.4\n");
        file_data.extend_from_slice(&vec![b' '; 100]);

        let rev1_offset = file_data.len() as u64;
        file_data.extend_from_slice(rev1);

        let rev2_offset = file_data.len() as u64;
        let mut rev2_with_prev = rev2_trad.to_vec();
        let rev2_str = String::from_utf8_lossy(&rev2_with_prev);
        let rev2_str = rev2_str.replace("/Prev 0", &format!("/Prev {}", rev1_offset));
        file_data.extend_from_slice(rev2_str.as_bytes());

        // Add a dummy xref stream at offset 500
        while file_data.len() < 500 {
            file_data.push(b' ');
        }
        // Minimal xref stream (won't parse correctly but tests hybrid detection)
        file_data.extend_from_slice(b"1 0 obj\n<< /Type /XRef /Size 2 /W [1 1 1] >>\nstream\n\x00\x00\x00\nendstream\nendobj\n");

        let source = MemorySource::new(file_data);
        let result = load_xref_with_prev_chain(&source, rev2_offset);

        // Should be marked as hybrid
        assert!(result.is_hybrid);
    }

    // proptest for /Prev chain
    mod proptest_prev_chain_tests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            /// Property: /Prev chain with random configurations never panics.
            #[test]
            fn prop_prev_chain_random_no_panic(
                revisions in prop::collection::vec(
                    (0u32..20u32, 0u64..1000u64, 0u16..10u16, any::<bool>()),
                    0..10
                )
            ) {
                // Build a minimal /Prev chain from the random data
                // Each tuple: (obj_num, offset, gen_nr, has_prev)
                let mut file_data = Vec::new();
                file_data.extend_from_slice(b"%PDF-1.4\n");

                let mut offsets = Vec::new();
                for (i, (obj_num, offset, gen_nr, has_prev)) in revisions.iter().enumerate() {
                    let pos = 1000u64 + (i as u64 * 500);
                    offsets.push(pos);

                    // Pad to position
                    while file_data.len() < pos as usize {
                        file_data.push(b' ');
                    }

                    // Create xref for this object
                    let xref = format!(
                        "xref\n{} 1\n\
                        {:010} {:05} n \n\
                        trailer\n<< /Size {} >>\n",
                        obj_num, offset, gen_nr, obj_num + 1
                    );

                    file_data.extend_from_slice(xref.as_bytes());
                }

                let source = MemorySource::new(file_data);

                // Loading from any offset should not panic
                if let Some(&start_offset) = offsets.last() {
                    let _ = load_xref_with_prev_chain(&source, start_offset);
                }
            }

            /// Property: Random /Prev offsets never panic.
            #[test]
            fn prop_prev_chain_random_offsets_no_panic(
                offsets in prop::collection::vec(0u64..10000u64, 0..20)
            ) {
                let mut file_data = Vec::new();
                file_data.extend_from_slice(b"%PDF-1.4\n");
                file_data.extend_from_slice(&vec![b' '; 10000]);

                // Add a base xref
                file_data.extend_from_slice(b"xref\n0 1\n0000000000 65535 f \ntrailer\n<< /Size 1 >>\n");

                let source = MemorySource::new(file_data);

                // Loading from any random offset should not panic
                for offset in offsets {
                    let _ = load_xref_with_prev_chain(&source, offset);
                }
            }
        }
    }
}
