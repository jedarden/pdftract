//! 4-level encoding resolution state machine with per-font caching.
//!
//! This module implements the top-level resolver that drives all four levels
//! of the encoding fallback chain:
//! - Level 1: ToUnicode CMap (confidence 1.0)
//! - Level 2: Named encoding + AGL (confidence 0.9)
//! - Level 3: Font fingerprint cache (confidence 0.85)
//! - Level 4: Glyph shape recognition (confidence 0.7, cfg-gated)
//!
//! The resolver maintains a per-font LRU cache of resolved glyphs and emits
//! the GLYPH_UNMAPPED diagnostic exactly once per (font, code) miss.

use std::hash::{Hash, Hasher};
use std::sync::Arc;

use dashmap::DashMap;
use smallvec::SmallVec;

use crate::diagnostics::{DiagCode, Diagnostic};
use crate::font::agl::{unicode_for_glyph_name, unicode_for_glyph_name_multi};
use crate::font::cmap::ToUnicodeMap;
use crate::font::encoding::FontEncoding;
use crate::font::fingerprint::CachedFingerprint;
use crate::font::shape::{lookup_shape, phash_glyph};
use crate::font::type3::Type3Font;
#[cfg(feature = "shape-db")]
use crate::font::type3_rasterizer::{rasterize_type3_glyph, DocumentContext as Type3DocumentContext, StreamResolverFn};
use crate::parser::stream::{decode_stream, ExtractionOptions, PdfSource as ParserPdfSource};
use crate::parser::xref::XrefResolver;

/// A loaded PDF font with encoding resolution capabilities.
///
/// This struct encapsulates all the data needed for the 4-level encoding
/// fallback chain. It owns the per-font resolution cache and tracks which
/// (font, code) pairs have already emitted diagnostics.
pub struct Font {
    /// Unique identifier for this font instance.
    id: FontId,
    /// ToUnicode CMap (Level 1).
    to_unicode: Option<ToUnicodeMap>,
    /// Font encoding (Level 2).
    encoding: Option<FontEncoding>,
    /// Cached font fingerprint (Level 3).
    fingerprint: Option<CachedFingerprint>,
    /// Whether this font has an embedded program (skip L3 if false).
    has_embedded_program: bool,
    /// Per-font resolution cache.
    cache: ResolverCache,
}

impl Font {
    /// Create a new Font instance.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique font identifier
    /// * `to_unicode` - Optional ToUnicode CMap
    /// * `encoding` - Optional font encoding
    /// * `fingerprint` - Optional cached fingerprint
    /// * `has_embedded_program` - Whether font has embedded program
    pub fn new(
        id: FontId,
        to_unicode: Option<ToUnicodeMap>,
        encoding: Option<FontEncoding>,
        fingerprint: Option<CachedFingerprint>,
        has_embedded_program: bool,
    ) -> Self {
        Self {
            id,
            to_unicode,
            encoding,
            fingerprint,
            has_embedded_program,
            cache: ResolverCache::new(),
        }
    }

    /// Get the font ID.
    pub fn id(&self) -> FontId {
        self.id
    }

    /// Get the ToUnicode CMap.
    pub fn to_unicode(&self) -> Option<&ToUnicodeMap> {
        self.to_unicode.as_ref()
    }

    /// Get the font encoding.
    pub fn encoding(&self) -> Option<&FontEncoding> {
        self.encoding.as_ref()
    }

    /// Get the cached fingerprint.
    pub fn fingerprint(&self) -> Option<&CachedFingerprint> {
        self.fingerprint.as_ref()
    }

    /// Check if this font has an embedded program.
    pub fn has_embedded_program(&self) -> bool {
        self.has_embedded_program
    }

    /// Get the resolution cache.
    pub fn cache(&self) -> &ResolverCache {
        &self.cache
    }
}

/// Unique identifier for a font instance.
///
/// This is the Arc pointer cast to usize, ensuring that different
/// Arc clones of the same font instance hash to the same value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontId(usize);

impl FontId {
    /// Create a FontId from an Arc pointer.
    pub fn from_arc<T>(arc: &Arc<T>) -> Self {
        Self(Arc::as_ptr(arc) as usize)
    }

    /// Create a FontId from a usize value (for testing).
    #[cfg(test)]
    pub fn from_usize(id: usize) -> Self {
        Self(id)
    }
}

/// Source of a Unicode glyph mapping.
///
/// Indicates which level of the fallback chain produced this mapping,
/// or whether the mapping came from OCR (Phase 5.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnicodeSource {
    /// Level 1: ToUnicode CMap
    ToUnicode,
    /// Level 2: Adobe Glyph List (named encoding)
    Agl,
    /// Level 3: Font fingerprint cache
    Fingerprint,
    /// Level 4: Shape recognition
    ShapeMatch,
    /// No mapping found (U+FFFD)
    Unknown,
    /// OCR path (Phase 5.4 HOCR)
    Ocr,
}

impl UnicodeSource {
    /// Get the confidence score for this source.
    ///
    /// Per INV-30, confidence is always one of {1.0, 0.9, 0.85, 0.7, 0.0}.
    /// OCR confidence is computed by Tesseract and varies (not in this set).
    pub fn confidence(self) -> f32 {
        match self {
            UnicodeSource::ToUnicode => 1.0,
            UnicodeSource::Agl => 0.9,
            UnicodeSource::Fingerprint => 0.85,
            UnicodeSource::ShapeMatch => 0.7,
            UnicodeSource::Unknown => 0.0,
            UnicodeSource::Ocr => 0.5, // Placeholder: actual OCR confidence comes from Tesseract
        }
    }
}

/// Result of resolving a character code to Unicode.
///
/// Contains the resolved Unicode characters (1-4 chars for ligatures),
/// the source of the mapping, and the confidence score.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedGlyph {
    /// Unicode characters (1-4 for ligature expansion)
    pub chars: SmallVec<[char; 4]>,
    /// Source of this mapping
    pub source: UnicodeSource,
    /// Confidence score (derived from source)
    pub confidence: f32,
}

impl ResolvedGlyph {
    /// Create a new resolved glyph.
    fn new(chars: SmallVec<[char; 4]>, source: UnicodeSource) -> Self {
        let confidence = source.confidence();
        Self {
            chars,
            source,
            confidence,
        }
    }

    /// Create a failure result (U+FFFD, unknown source).
    fn failure() -> Self {
        Self::new(SmallVec::from_slice(&['\u{FFFD}']), UnicodeSource::Unknown)
    }

    /// Check if this is a failure result (U+FFFD with unknown source).
    pub fn is_failure(&self) -> bool {
        self.source == UnicodeSource::Unknown
    }
}

/// Cache key for per-font glyph resolution.
///
/// Combines the font ID and the character code bytes into a single hashable key.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CacheKey {
    font_id: FontId,
    char_code: SmallVec<[u8; 4]>,
}

impl Hash for CacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.font_id.hash(state);
        // Hash the bytes directly
        for byte in &self.char_code {
            byte.hash(state);
        }
    }
}

/// Per-font resolution cache with miss tracking.
///
/// Maintains:
/// - A DashMap for thread-safe cached resolutions
/// - A HashSet of (font_id, char_code) keys that have already emitted diagnostics
pub struct ResolverCache {
    /// Cached resolutions: (font_id, char_code) -> ResolvedGlyph
    cache: DashMap<CacheKey, ResolvedGlyph>,
    /// Set of (font_id, char_code) that have already emitted GLYPH_UNMAPPED
    emitted_misses: DashMap<(FontId, SmallVec<[u8; 4]>), ()>,
}

impl ResolverCache {
    /// Create a new empty resolver cache.
    pub fn new() -> Self {
        Self {
            cache: DashMap::new(),
            emitted_misses: DashMap::new(),
        }
    }

    /// Look up a cached resolution.
    pub fn get(&self, font_id: FontId, char_code: &[u8]) -> Option<ResolvedGlyph> {
        let key = CacheKey {
            font_id,
            char_code: SmallVec::from_slice(char_code),
        };
        self.cache.get(&key).map(|entry| entry.clone())
    }

    /// Insert a resolution into the cache.
    pub fn insert(&self, font_id: FontId, char_code: &[u8], result: &ResolvedGlyph) {
        let key = CacheKey {
            font_id,
            char_code: SmallVec::from_slice(char_code),
        };
        self.cache.insert(key, result.clone());
    }

    /// Check if a miss diagnostic has already been emitted for this (font, code).
    pub fn has_emitted_miss(&self, font_id: FontId, char_code: &[u8]) -> bool {
        let key = (font_id, SmallVec::from_slice(char_code));
        self.emitted_misses.contains_key(&key)
    }

    /// Mark this (font, code) as having emitted a miss diagnostic.
    pub fn mark_emitted_miss(&self, font_id: FontId, char_code: &[u8]) {
        let key = (font_id, SmallVec::from_slice(char_code));
        self.emitted_misses.insert(key, ());
    }

    /// Get the number of cached resolutions.
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

impl Default for ResolverCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Resolve a character code to Unicode using the 4-level fallback chain.
///
/// This is the main entry point for Phase 2 encoding resolution. Given a font
/// and a character code (as raw bytes), it attempts to map to Unicode using
/// all four levels of the fallback chain.
///
/// # Arguments
///
/// * `font` - The font to resolve from
/// * `char_code` - Character code bytes (1-4 bytes for multi-byte encodings)
/// * `glyph_id` - Optional glyph ID for Level 3 fingerprint lookup
/// * `diagnostics` - Diagnostics list for emitting GLYPH_UNMAPPED
///
/// # Returns
///
/// A `ResolvedGlyph` containing the mapped characters, source, and confidence.
pub fn resolve_unicode(
    font: &Font,
    char_code: &[u8],
    glyph_id: Option<u16>,
    diagnostics: &mut Vec<Diagnostic>,
) -> ResolvedGlyph {
    let font_id = font.id();
    let cache = &font.cache;

    // Check cache first
    if let Some(cached) = cache.get(font_id, char_code) {
        return cached;
    }

    // Level 1: ToUnicode CMap
    let result = resolve_level1(char_code, font.to_unicode());

    let result = if !result.is_failure() {
        result
    } else {
        // Level 2: Named encoding + AGL
        let result = resolve_level2(char_code, font.encoding());
        if !result.is_failure() {
            result
        } else {
            // Level 3: Font fingerprint (skip for Standard 14 fonts)
            if font.has_embedded_program() {
                let result = resolve_level3(char_code, glyph_id, font.fingerprint());
                if !result.is_failure() {
                    result
                } else {
                    // Level 4: Shape recognition (cfg-gated)
                    #[cfg(feature = "shape-db")]
                    {
                        let result = resolve_level4(char_code, glyph_id, font.fingerprint());
                        if !result.is_failure() {
                            result
                        } else {
                            // All levels failed
                            emit_miss_diagnostic(font_id, char_code, cache, diagnostics);
                            ResolvedGlyph::failure()
                        }
                    }
                    #[cfg(not(feature = "shape-db"))]
                    {
                        // Level 4 not available, emit miss and return failure
                        emit_miss_diagnostic(font_id, char_code, cache, diagnostics);
                        ResolvedGlyph::failure()
                    }
                }
            } else {
                // No embedded program, skip to Level 4
                #[cfg(feature = "shape-db")]
                {
                    let result = resolve_level4(char_code, glyph_id, font.fingerprint());
                    if !result.is_failure() {
                        result
                    } else {
                        emit_miss_diagnostic(font_id, char_code, cache, diagnostics);
                        ResolvedGlyph::failure()
                    }
                }
                #[cfg(not(feature = "shape-db"))]
                {
                    emit_miss_diagnostic(font_id, char_code, cache, diagnostics);
                    ResolvedGlyph::failure()
                }
            }
        }
    };

    // Cache the result
    cache.insert(font_id, char_code, &result);

    result
}

/// Level 1: ToUnicode CMap lookup.
///
/// Returns the mapped characters if found and non-empty/non-U+FFFD.
/// Otherwise returns a failure result to fall through to Level 2.
fn resolve_level1(char_code: &[u8], to_unicode: Option<&ToUnicodeMap>) -> ResolvedGlyph {
    let Some(cmap) = to_unicode else {
        return ResolvedGlyph::failure();
    };

    let Some(chars) = cmap.lookup(char_code) else {
        return ResolvedGlyph::failure();
    };

    // Empty result or U+FFFD only -> fall through
    if chars.is_empty() || (chars.len() == 1 && chars[0] == '\u{FFFD}') {
        return ResolvedGlyph::failure();
    }

    // Multi-codepoint result from ligature expansion
    // MARKER: ToUnicode entry creation point - Level 1 CMap lookup success.
    // Creates ResolvedGlyph with UnicodeSource::ToUnicode (confidence 1.0).
    // See notes/bf-2nob5-child-1.md for documentation.
    ResolvedGlyph::new(SmallVec::from_slice(chars), UnicodeSource::ToUnicode)
}

/// Level 2: Named encoding + AGL lookup.
///
/// Maps character code to glyph name via encoding, then glyph name to Unicode via AGL.
fn resolve_level2(char_code: &[u8], encoding: Option<&FontEncoding>) -> ResolvedGlyph {
    let Some(enc) = encoding else {
        return ResolvedGlyph::failure();
    };

    // Single-byte codes only for named encodings
    if char_code.len() != 1 {
        return ResolvedGlyph::failure();
    }

    let code = char_code[0];

    // Get glyph name from encoding
    let Some(glyph_name) = enc.glyph_name_for(code) else {
        return ResolvedGlyph::failure();
    };

    // Look up in AGL
    // Try multi-codepoint first (ligatures like "fi" as separate chars)
    if let Some(chars) = unicode_for_glyph_name_multi(&glyph_name) {
        return ResolvedGlyph::new(SmallVec::from_slice(chars), UnicodeSource::Agl);
    }

    // Try single-codepoint
    if let Some(ch) = unicode_for_glyph_name(&glyph_name) {
        return ResolvedGlyph::new(SmallVec::from_slice(&[ch]), UnicodeSource::Agl);
    }

    // Not in AGL
    ResolvedGlyph::failure()
}

/// Level 3: Font fingerprint cache lookup.
///
/// Looks up a glyph ID in the cached fingerprint database. This requires
/// the glyph ID (not the character code) because fingerprint mappings are
/// per-glyph, not per-character-code.
///
/// When glyph_id is None (e.g., before char_code -> GID mapping in Phase 3),
/// Level 3 falls through to Level 4.
fn resolve_level3(
    _char_code: &[u8],
    glyph_id: Option<u16>,
    fingerprint: Option<&CachedFingerprint>,
) -> ResolvedGlyph {
    let Some(gid) = glyph_id else {
        // No glyph ID available - fall through to Level 4
        return ResolvedGlyph::failure();
    };

    let Some(fp) = fingerprint else {
        return ResolvedGlyph::failure();
    };

    // Look up the glyph ID in the fingerprint cache
    let Some(ch) = fp.lookup(gid) else {
        return ResolvedGlyph::failure();
    };

    ResolvedGlyph::new(SmallVec::from_slice(&[ch]), UnicodeSource::Fingerprint)
}

/// Level 4: Glyph shape recognition.
///
/// This is a stub that returns failure. The actual implementation would
/// render the glyph to a bitmap and look up the shape in the database.
/// This requires the `shape-db` feature and is part of Phase 2.5.
#[cfg(feature = "shape-db")]
fn resolve_level4(
    _char_code: &[u8],
    _glyph_id: Option<u16>,
    _fingerprint: Option<&CachedFingerprint>,
) -> ResolvedGlyph {
    // Stub: Level 4 (shape recognition) is Phase 2.5, not yet implemented
    ResolvedGlyph::failure()
}

/// Resolve a Type 3 font character code to Unicode using the Type 3-specific chain.
///
/// Type 3 fonts use a modified fallback chain:
/// - Level 1: ToUnicode CMap (same as regular fonts)
/// - Level 2: Encoding + AGL (same as regular fonts)
/// - Level 3: SKIPPED (Type 3 fonts have no embedded program)
/// - Level 4: Shape recognition (rasterize glyph + pHash + shape DB lookup)
///
/// # Arguments
///
/// * `font` - The Type3 font containing the glyph
/// * `to_unicode` - Optional ToUnicode CMap (Level 1)
/// * `char_code` - Character code (single byte for Type 3)
/// * `resolver` - Optional XrefResolver for dereferencing content streams
/// * `source` - Optional PdfSource for reading stream data
/// * `doc_decompress_counter` - Optional decompression counter (bomb protection)
/// * `diagnostics` - Diagnostics list for emitting GLYPH_UNMAPPED
///
/// # Returns
///
/// A `ResolvedGlyph` containing the mapped characters, source, and confidence.
///
/// # Type 3 Resolution Chain
///
/// 1. **Level 1 (ToUnicode)**: Try the `/ToUnicode` CMap if present.
///    If found and non-empty, return with confidence 1.0.
///
/// 2. **Level 2 (AGL)**: Try `/Encoding` → glyph name → AGL lookup.
///    If found, return with confidence 0.9.
///
/// 3. **Level 3 (SKIPPED)**: Type 3 fonts have no embedded font program,
///    so fingerprint-based lookup is not applicable.
///
/// 4. **Level 4 (Shape)**: Rasterize the glyph content stream to a 32×32 bitmap,
///    compute pHash, and look up in the shape database. Returns with confidence 0.7
///    if a match is found (Hamming distance ≤ 8).
///
/// 5. **Failure**: If all levels fail, return U+FFFD with confidence 0.0
///    and emit TYPE3_GLYPH_UNMAPPED diagnostic.
///
/// # Special Cases
///
/// - Arbitrary glyph names: If Level 2 returns a glyph name that's not in AGL,
///   escalate to Level 4 (shape recognition).
/// - Missing glyph in /CharProcs: Escalate to Level 4 with a warning diagnostic.
/// - No ToUnicode and no Encoding: Skip directly to Level 4.
pub fn resolve_type3(
    font: &Type3Font,
    to_unicode: Option<&ToUnicodeMap>,
    char_code: u8,
    resolver: Option<&XrefResolver>,
    source: Option<&dyn ParserPdfSource>,
    doc_decompress_counter: Option<&mut u64>,
    diagnostics: &mut Vec<Diagnostic>,
) -> ResolvedGlyph {
    // Level 1: ToUnicode CMap
    let char_code_slice = [char_code];
    let result = resolve_level1(&char_code_slice, to_unicode);

    if !result.is_failure() {
        return result;
    }

    // Level 2: Encoding + AGL
    let encoding = &font.encoding;
    let result = resolve_level2(&char_code_slice, Some(encoding));

    if !result.is_failure() {
        return result;
    }

    // Check if we have a glyph name from encoding that's not in AGL
    // This is the heuristic for "arbitrary glyph name" that requires L4
    let glyph_name_for_l4 = encoding.glyph_name_for(char_code);

    // Level 3: SKIPPED for Type 3 fonts (no embedded program)
    // Per the plan: "Type 3 fonts have no embedded program; L3 fingerprinting not applicable"

    // Level 4: Shape recognition
    #[cfg(feature = "shape-db")]
    {
        let result = resolve_type3_level4(
            font,
            char_code,
            glyph_name_for_l4,
            resolver,
            source,
            doc_decompress_counter,
            diagnostics,
        );
        if !result.is_failure() {
            return result;
        }

        // All levels failed
        diagnostics.push(Diagnostic::with_dynamic_no_offset(
            DiagCode::FontGlyphUnmapped,
            format!(
                "Type3 font: character code 0x{:02X} could not be resolved to Unicode",
                char_code
            ),
        ));
        ResolvedGlyph::failure()
    }
    #[cfg(not(feature = "shape-db"))]
    {
        // Level 4 not available, emit miss and return failure
        diagnostics.push(Diagnostic::with_dynamic_no_offset(
            DiagCode::FontGlyphUnmapped,
            format!(
                "Type3 font: character code 0x{:02X} could not be resolved (shape recognition disabled)",
                char_code
            ),
        ));
        ResolvedGlyph::failure()
    }
}

/// Level 4 shape recognition for Type 3 fonts.
///
/// Rasterizes the glyph content stream to a 32×32 bitmap, computes pHash,
/// and looks up the shape in the database.
///
/// # Arguments
///
/// * `font` - The Type3 font containing the glyph
/// * `char_code` - Character code (single byte)
/// * `glyph_name` - Optional glyph name from encoding (for diagnostics)
/// * `resolver` - Optional XrefResolver for dereferencing content streams
/// * `source` - Optional PdfSource for reading stream data
/// * `doc_decompress_counter` - Optional decompression counter (bomb protection)
/// * `diagnostics` - Diagnostics list
///
/// # Returns
///
/// A `ResolvedGlyph` with confidence 0.7 if a shape match is found,
/// otherwise a failure result.
#[cfg(feature = "shape-db")]
fn resolve_type3_level4(
    font: &Type3Font,
    char_code: u8,
    glyph_name: Option<Arc<str>>,
    resolver: Option<&XrefResolver>,
    source: Option<&dyn ParserPdfSource>,
    doc_decompress_counter: Option<&mut u64>,
    diagnostics: &mut Vec<Diagnostic>,
) -> ResolvedGlyph {
    // Get the glyph name from encoding if we don't have it
    let glyph_name = match glyph_name {
        Some(name) => name,
        None => match font.encoding.glyph_name_for(char_code) {
            Some(name) => name,
            None => {
                // No glyph name available - can't rasterize
                diagnostics.push(Diagnostic::with_dynamic_no_offset(
                    DiagCode::FontGlyphUnmapped,
                    format!(
                        "Type3 font: character code 0x{:02X} has no glyph name in encoding",
                        char_code
                    ),
                ));
                return ResolvedGlyph::failure();
            }
        },
    };

    // Check if glyph exists in /CharProcs
    if !font.has_glyph(&glyph_name) {
        diagnostics.push(Diagnostic::with_dynamic_no_offset(
            DiagCode::FontGlyphUnmapped,
            format!(
                "Type3 font: glyph '{}' not found in /CharProcs for code 0x{:02X}",
                glyph_name, char_code
            ),
        ));
        return ResolvedGlyph::failure();
    }

    // Create stream resolver callback if document context is available
    // Helper function to resolve a stream reference to decoded bytes
    fn resolve_stream_bytes(
        obj_ref: crate::parser::object::ObjRef,
        resolver: &XrefResolver,
        source: &dyn ParserPdfSource,
        counter: &mut u64,
    ) -> Option<Vec<u8>> {
        use crate::parser::object::PdfObject;

        // Resolve the object reference
        let obj = resolver.resolve_with_source(obj_ref, source).ok()?;

        // Extract the stream
        let stream = match obj {
            PdfObject::Stream(s) => *s,
            _ => return None,
        };

        // Decode the stream
        let bytes = decode_stream(
            &stream,
            source,
            &ExtractionOptions::default(),
            counter,
        );

        Some(bytes)
    }

    let bitmap = if let (Some(resolver), Some(source), Some(counter)) = (resolver, source, doc_decompress_counter) {
        // Create document context for Type3 rasterization
        let doc_ctx = Type3DocumentContext { resolver: Some(resolver), source: Some(source) };

        // Use helper function to create a closure-compatible callback
        // This is a workaround for lifetime issues with closures capturing references
        let callback = |obj_ref: crate::parser::object::ObjRef| -> Option<Vec<u8>> {
            resolve_stream_bytes(obj_ref, resolver, source, counter)
        };

        rasterize_type3_glyph(font, &glyph_name, Some(&doc_ctx), Some(&callback))
    } else {
        // No document context available - cannot resolve stream, will return None
        rasterize_type3_glyph(font, &glyph_name, None::<&Type3DocumentContext>, None::<&StreamResolverFn>)
    };

    let bitmap = match bitmap {
        Some(bm) => bm,
        None => {
            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::FontGlyphUnmapped,
                format!(
                    "Type3 font: failed to rasterize glyph '{}' for code 0x{:02X}",
                    glyph_name, char_code
                ),
            ));
            return ResolvedGlyph::failure();
        }
    };

    // Compute pHash
    let phash = phash_glyph(&bitmap);

    // Look up in shape database
    match lookup_shape(phash) {
        Some(matched) if matched.is_acceptable() => ResolvedGlyph::new(
            SmallVec::from_slice(&[matched.ch]),
            UnicodeSource::ShapeMatch,
        ),
        Some(matched) => {
            // Match found but outside threshold - emit diagnostic and fall through
            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::FontGlyphUnmapped,
                format!(
                    "Type3 font: shape match for '{}' (code 0x{:02X}) found but distance {} exceeds threshold",
                    glyph_name, char_code, matched.distance
                ),
            ));
            ResolvedGlyph::failure()
        }
        None => ResolvedGlyph::failure(),
    }
}

/// Emit the GLYPH_UNMAPPED diagnostic exactly once per (font, code) miss.
fn emit_miss_diagnostic(
    font_id: FontId,
    char_code: &[u8],
    cache: &ResolverCache,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Only emit once per (font, code) pair
    if cache.has_emitted_miss(font_id, char_code) {
        return;
    }

    // Format char_code as hex string
    let hex_string: String = char_code.iter().map(|b| format!("{:02X}", b)).collect();

    let message = format!(
        "Character code {} could not be resolved to Unicode (font ID: {:?})",
        hex_string, font_id
    );

    diagnostics.push(Diagnostic::with_dynamic_no_offset(
        DiagCode::FontGlyphUnmapped,
        message,
    ));

    // Mark as emitted
    cache.mark_emitted_miss(font_id, char_code);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::cmap::parse_to_unicode;
    use crate::font::encoding::{FontEncoding, NamedEncoding};

    #[test]
    fn test_unicode_source_confidence() {
        assert_eq!(
            UnicodeSource::ToUnicode.confidence(),
            1.0,
            "ToUnicode confidence should be 1.0 (highest). \
             Expected: 1.0. \
             Found: {}. \
             Why this matters: ToUnicode CMap is the Level 1 (highest-confidence) source for \
             glyph-to-codepoint resolution; it should always have maximum confidence.",
            UnicodeSource::ToUnicode.confidence()
        );
        assert_eq!(
            UnicodeSource::Agl.confidence(),
            0.9,
            "AGL confidence should be 0.9 (high fallback). \
             Expected: 0.9. \
             Found: {}. \
             Why this matters: AGL (Adobe Glyph List) is the Level 2 fallback; slightly lower \
             confidence reflects it's less reliable than ToUnicode but still authoritative.",
            UnicodeSource::Agl.confidence()
        );
        assert_eq!(
            UnicodeSource::Fingerprint.confidence(),
            0.85,
            "Fingerprint confidence should be 0.85 (moderate-high fallback). \
             Expected: 0.85. \
             Found: {}. \
             Why this matters: Font fingerprint matching is Level 3; confidence is lower than \
             direct mappings because it relies on font program SHA-256 database lookups.",
            UnicodeSource::Fingerprint.confidence()
        );
        assert_eq!(
            UnicodeSource::ShapeMatch.confidence(),
            0.7,
            "ShapeMatch confidence should be 0.7 (moderate fallback). \
             Expected: 0.7. \
             Found: {}. \
             Why this matters: Glyph shape recognition is Level 4; lower confidence reflects \
             it's a heuristic based on visual similarity rather than exact mapping data.",
            UnicodeSource::ShapeMatch.confidence()
        );
        assert_eq!(
            UnicodeSource::Unknown.confidence(),
            0.0,
            "Unknown confidence should be 0.0 (no confidence). \
             Expected: 0.0. \
             Found: {}. \
             Why this matters: When all resolution levels fail, we produce U+FFFD with zero \
             confidence to signal complete uncertainty about the correct Unicode value.",
            UnicodeSource::Unknown.confidence()
        );
    }

    #[test]
    fn test_resolved_glyph_failure() {
        let glyph = ResolvedGlyph::failure();
        assert!(
            glyph.is_failure(),
            "ResolvedGlyph::failure() should produce a failure state. \
             Expected: is_failure() == true. \
             Found: {}. \
             Why this matters: Failure state indicates no valid Unicode mapping was found; \
             this is used when all 4 resolution levels (ToUnicode, AGL, fingerprint, shape) fail.",
            glyph.is_failure()
        );
        assert_eq!(
            glyph.chars.as_slice(),
            ['\u{FFFD}'],
            "Failed glyph should contain U+FFFD replacement character. \
             Expected: ['\u{FFFD}']. \
             Found: {:?}. \
             Why this matters: U+FFFD is the standard Unicode replacement character; it signals \
             the glyph could not be resolved to any valid Unicode value.",
            glyph.chars.as_slice()
        );
        assert_eq!(
            glyph.source,
            UnicodeSource::Unknown,
            "Failed glyph should have Unknown source. \
             Expected: UnicodeSource::Unknown. \
             Found: {:?}. \
             Why this matters: Unknown source indicates none of the 4 resolution levels succeeded.",
            glyph.source
        );
        assert_eq!(
            glyph.confidence,
            0.0,
            "Failed glyph should have zero confidence. \
             Expected: 0.0. \
             Found: {}. \
             Why this matters: Zero confidence reflects complete uncertainty; this glyph should \
             not be trusted for any critical text processing.",
            glyph.confidence
        );
    }

    #[test]
    fn test_resolved_glyph_new() {
        let chars = SmallVec::from_slice(&['A', 'B']);
        let glyph = ResolvedGlyph::new(chars.clone(), UnicodeSource::ToUnicode);
        assert_eq!(
            glyph.chars,
            chars,
            "ResolvedGlyph::new() should preserve the provided characters. \
             Expected: ['A', 'B']. \
             Found: {:?}. \
             Why this matters: The constructor must store exactly the characters resolved from \
             the font; any deviation would corrupt the extraction output.",
            glyph.chars
        );
        assert_eq!(
            glyph.source,
            UnicodeSource::ToUnicode,
            "ResolvedGlyph::new() should store the provided UnicodeSource. \
             Expected: UnicodeSource::ToUnicode. \
             Found: {:?}. \
             Why this matters: The source must be preserved so consumers can judge reliability \
             and implement filtering based on confidence source.",
            glyph.source
        );
        assert_eq!(
            glyph.confidence,
            1.0,
            "ResolvedGlyph::new() with ToUnicode should have maximum confidence. \
             Expected: 1.0. \
             Found: {}. \
             Why this matters: ToUnicode is the highest-confidence source; the constructor \
             should automatically use the confidence value associated with that source.",
            glyph.confidence
        );
    }

    #[test]
    fn test_font_id_from_arc() {
        let arc = Arc::new(42);
        let id1 = FontId::from_arc(&arc);
        let id2 = FontId::from_arc(&arc);
        assert_eq!(
            id1, id2,
            "FontId::from_arc should return the same ID for the same Arc. \
             Expected: id1 == id2. \
             Found: id1 != id2. \
             Why this matters: Font IDs are cached per Arc pointer; calling from_arc twice on \
             the same Arc must return identical IDs to avoid duplicate entries in the resolver cache.",
        );

        let arc2 = Arc::new(42);
        let id3 = FontId::from_arc(&arc2);
        assert_ne!(
            id1, id3,
            "FontId::from_arc should return different IDs for different Arc instances. \
             Expected: id1 != id3. \
             Found: id1 == id3. \
             Why this matters: Even though both Arcs contain the same value (42), they are \
             different heap allocations; different Arcs must have different IDs to maintain cache \
             isolation between distinct font instances."
        );
    }

    #[test]
    fn test_resolver_cache_basic() {
        let cache = ResolverCache::new();
        let font_id = FontId::from_arc(&Arc::new("test"));
        let char_code = vec![0x41];
        let result = ResolvedGlyph::new(SmallVec::from_slice(&['A']), UnicodeSource::ToUnicode);

        assert!(
            cache.get(font_id, &char_code).is_none(),
            "Cache should return None for non-existent key. \
             Expected: None. \
             Found: Some entry. \
             Why this matters: A fresh cache must not contain any entries; getting a key before \
             insertion should always return None.",
        );

        cache.insert(font_id, &char_code, &result);
        let cached = cache.get(font_id, &char_code);
        assert!(
            cached.is_some(),
            "Cache should return Some after insertion. \
             Expected: Some(entry). \
             Found: None. \
             Why this matters: The insert operation must store the entry; subsequent get must \
             retrieve it successfully.",
        );
        assert_eq!(
            cached.as_ref().unwrap().chars,
            SmallVec::<[char; 4]>::from_slice(&['A']),
            "Cached entry should contain the correct characters. \
             Expected: ['A']. \
             Found: {:?}. \
             Why this matters: The cache must preserve the exact ResolvedGlyph data that was \
             inserted; any corruption would produce incorrect text extraction.",
            cached.as_ref().unwrap().chars
        );
    }

    #[test]
    fn test_resolver_cache_miss_tracking() {
        let cache = ResolverCache::new();
        let font_id = FontId::from_arc(&Arc::new("test"));
        let char_code = vec![0x41];

        assert!(!cache.has_emitted_miss(font_id, &char_code));
        cache.mark_emitted_miss(font_id, &char_code);
        assert!(cache.has_emitted_miss(font_id, &char_code));
    }

    #[test]
    fn test_resolve_level1_tounicode() {
        let cmap_data = b"beginbfchar 1 <00> <0041> endbfchar";
        let cmap = parse_to_unicode(cmap_data);
        let result = resolve_level1(&[0x00], Some(&cmap));

        assert!(!result.is_failure());
        assert_eq!(result.chars.as_slice(), ['A']);
        assert_eq!(result.source, UnicodeSource::ToUnicode);
        assert_eq!(result.confidence, 1.0);
    }

    #[test]
    fn test_resolve_level1_ligature() {
        // fi ligature as two separate chars
        let cmap_data = b"beginbfchar 1 <00> <00660069> endbfchar";
        let cmap = parse_to_unicode(cmap_data);
        let result = resolve_level1(&[0x00], Some(&cmap));

        assert!(!result.is_failure());
        assert_eq!(result.chars.as_slice(), ['f', 'i']);
        assert_eq!(result.source, UnicodeSource::ToUnicode);
    }

    #[test]
    fn test_resolve_level1_fallback_on_empty() {
        // Empty mapping falls through
        let cmap_data = b"beginbfchar 1 <00> <> endbfchar";
        let cmap = parse_to_unicode(cmap_data);
        let result = resolve_level1(&[0x00], Some(&cmap));

        assert!(result.is_failure());
    }

    #[test]
    fn test_resolve_level1_fallback_on_fffd() {
        // U+FFFD falls through
        let cmap_data = b"beginbfchar 1 <00> <FFFD> endbfchar";
        let cmap = parse_to_unicode(cmap_data);
        let result = resolve_level1(&[0x00], Some(&cmap));

        assert!(result.is_failure());
    }

    #[test]
    fn test_resolve_level1_no_cmap() {
        let result = resolve_level1(&[0x41], None);
        assert!(result.is_failure());
    }

    #[test]
    fn test_resolve_level1_not_in_cmap() {
        let cmap_data = b"beginbfchar 1 <00> <0041> endbfchar";
        let cmap = parse_to_unicode(cmap_data);
        let result = resolve_level1(&[0x01], Some(&cmap));

        assert!(result.is_failure());
    }

    #[test]
    fn test_resolve_level2_agl() {
        let encoding = FontEncoding::new(Some(NamedEncoding::WinAnsi));
        let result = resolve_level2(&[0x41], Some(&encoding));

        // 0x41 in WinAnsi is 'A'
        assert!(!result.is_failure());
        assert_eq!(result.source, UnicodeSource::Agl);
        assert_eq!(result.confidence, 0.9);
    }

    #[test]
    fn test_resolve_level2_multi_byte_fails() {
        // Multi-byte codes not supported in Level 2
        let encoding = FontEncoding::new(Some(NamedEncoding::WinAnsi));
        let result = resolve_level2(&[0x00, 0x41], Some(&encoding));
        assert!(result.is_failure());
    }

    #[test]
    fn test_resolve_level2_no_encoding() {
        let result = resolve_level2(&[0x41], None);
        assert!(result.is_failure());
    }

    #[test]
    fn test_resolve_level2_unmapped_code() {
        // Most codes in StandardEncoding are unmapped above 0x7F
        let encoding = FontEncoding::new(Some(NamedEncoding::Standard));
        let result = resolve_level2(&[0x80], Some(&encoding));
        assert!(
            result.is_failure(),
            "Level 2 resolution should fail for unmapped character codes. \
             Expected: result.is_failure() == true. \
             Found: false. \
             Why this matters: Most codes in StandardEncoding above 0x7F are unmapped and should fail Level 2 (encoding + AGL) resolution per build/unmapped-glyph-names.json filtering."
        );
    }

    #[test]
    fn test_resolve_unicode_full_hit() {
        let mut diagnostics = Vec::new();
        let font_id = FontId::from_arc(&Arc::new("test"));

        // Set up ToUnicode
        let cmap_data = b"beginbfchar 1 <41> <0041> endbfchar";
        let cmap = parse_to_unicode(cmap_data);

        let font = Font::new(font_id, Some(cmap), None, None, false);

        let result = resolve_unicode(&font, &[0x41], None, &mut diagnostics);

        assert!(!result.is_failure());
        assert_eq!(result.source, UnicodeSource::ToUnicode);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_resolve_unicode_caching() {
        let mut diagnostics = Vec::new();
        let font_id = FontId::from_arc(&Arc::new("test"));

        // First call - not cached
        let cmap_data = b"beginbfchar 1 <41> <0041> endbfchar";
        let cmap = parse_to_unicode(cmap_data);

        let font = Font::new(font_id, Some(cmap), None, None, false);

        let result1 = resolve_unicode(&font, &[0x41], None, &mut diagnostics);

        // Second call - cached
        let result2 = resolve_unicode(&font, &[0x41], None, &mut diagnostics);

        assert_eq!(result1.chars, result2.chars);
        assert_eq!(font.cache().len(), 1);
    }

    #[test]
    fn test_resolve_unicode_miss_emits_once() {
        let mut diagnostics = Vec::new();
        let font_id = FontId::from_arc(&Arc::new("test"));

        // No ToUnicode, no encoding -> miss
        let font = Font::new(font_id, None, None, None, false);

        let result = resolve_unicode(&font, &[0x41], None, &mut diagnostics);

        assert!(result.is_failure());
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, DiagCode::FontGlyphUnmapped);

        // Second call for same code should not emit again
        let result2 = resolve_unicode(&font, &[0x41], None, &mut diagnostics);

        assert!(result2.is_failure());
        assert_eq!(diagnostics.len(), 1); // Still 1
    }

    #[test]
    fn test_resolve_unicode_different_fonts_separate_misses() {
        let mut diagnostics = Vec::new();
        let font_id1 = FontId::from_arc(&Arc::new("font1"));
        let font_id2 = FontId::from_arc(&Arc::new("font2"));

        let font1 = Font::new(font_id1, None, None, None, false);
        let font2 = Font::new(font_id2, None, None, None, false);

        // Both fonts miss on same code
        let result1 = resolve_unicode(&font1, &[0x41], None, &mut diagnostics);
        let result2 = resolve_unicode(&font2, &[0x41], None, &mut diagnostics);

        assert!(result1.is_failure());
        assert!(result2.is_failure());
        assert_eq!(diagnostics.len(), 2); // One per font
    }

    #[test]
    fn test_resolve_unicode_fallback_chain() {
        let mut diagnostics = Vec::new();
        let font_id = FontId::from_arc(&Arc::new("test"));

        // L1: No ToUnicode -> fall through
        // L2: WinAnsi encoding with 'A' at 0x41
        let encoding = FontEncoding::new(Some(NamedEncoding::WinAnsi));

        let font = Font::new(font_id, None, Some(encoding), None, false);

        let result = resolve_unicode(&font, &[0x41], None, &mut diagnostics);

        assert!(!result.is_failure());
        assert_eq!(result.source, UnicodeSource::Agl);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_resolve_unicode_level3_with_glyph_id() {
        let mut diagnostics = Vec::new();
        let font_id = FontId::from_arc(&Arc::new("test"));

        // Create a mock fingerprint with a known glyph
        // Note: This test requires a real fingerprint database entry to pass
        // For now, we test that the API works correctly
        let font = Font::new(font_id, None, None, None, true);

        // No glyph_id -> L3 should fall through
        let result = resolve_unicode(&font, &[0x41], None, &mut diagnostics);

        assert!(result.is_failure());
    }

    #[test]
    fn test_resolve_type3_with_tounicode() {
        // Type 3 font with ToUnicode mapping code 0x41 -> 'A'
        let mut diagnostics = Vec::new();
        let mut font_dict = crate::parser::object::types::PdfDict::new();
        font_dict.insert(
            crate::parser::object::types::intern("/Subtype"),
            crate::parser::object::types::PdfObject::Name(crate::parser::object::types::intern(
                "/Type3",
            )),
        );
        font_dict.insert(
            crate::parser::object::types::intern("/FirstChar"),
            crate::parser::object::types::PdfObject::Integer(0),
        );
        font_dict.insert(
            crate::parser::object::types::intern("/LastChar"),
            crate::parser::object::types::PdfObject::Integer(255),
        );

        let font = Type3Font::load(&font_dict);

        // Create ToUnicode CMap with 0x41 -> 'A'
        let cmap_data = b"beginbfchar 1 <41> <0041> endbfchar";
        let cmap = parse_to_unicode(cmap_data);

        let result = resolve_type3(&font, Some(&cmap), 0x41, None, None, None, &mut diagnostics);

        assert!(!result.is_failure());
        assert_eq!(result.chars.as_slice(), ['A']);
        assert_eq!(result.source, UnicodeSource::ToUnicode);
        assert_eq!(result.confidence, 1.0);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_resolve_type3_with_agl() {
        // Type 3 font with standard glyph name 'A' via Encoding, no ToUnicode
        let mut diagnostics = Vec::new();
        let mut font_dict = crate::parser::object::types::PdfDict::new();
        font_dict.insert(
            crate::parser::object::types::intern("/Subtype"),
            crate::parser::object::types::PdfObject::Name(crate::parser::object::types::intern(
                "/Type3",
            )),
        );
        font_dict.insert(
            crate::parser::object::types::intern("/Encoding"),
            crate::parser::object::types::PdfObject::Name(crate::parser::object::types::intern(
                "/WinAnsiEncoding",
            )),
        );
        font_dict.insert(
            crate::parser::object::types::intern("/FirstChar"),
            crate::parser::object::types::PdfObject::Integer(0),
        );
        font_dict.insert(
            crate::parser::object::types::intern("/LastChar"),
            crate::parser::object::types::PdfObject::Integer(255),
        );

        let font = Type3Font::load(&font_dict);

        // No ToUnicode, use encoding + AGL
        let result = resolve_type3(&font, None, 0x41, None, None, None, &mut diagnostics);

        // 0x41 in WinAnsi is 'A' which maps to 'A' via AGL
        assert!(!result.is_failure());
        assert_eq!(result.chars.as_slice(), ['A']);
        assert_eq!(result.source, UnicodeSource::Agl);
        assert_eq!(result.confidence, 0.9);
    }

    #[test]
    fn test_resolve_type3_fallback_to_fffd() {
        // Type 3 font with arbitrary glyph name and no ToUnicode
        // Should fall through all levels and return U+FFFD
        let mut diagnostics = Vec::new();
        let mut font_dict = crate::parser::object::types::PdfDict::new();
        font_dict.insert(
            crate::parser::object::types::intern("/Subtype"),
            crate::parser::object::types::PdfObject::Name(crate::parser::object::types::intern(
                "/Type3",
            )),
        );
        font_dict.insert(
            crate::parser::object::types::intern("/FirstChar"),
            crate::parser::object::types::PdfObject::Integer(0),
        );
        font_dict.insert(
            crate::parser::object::types::intern("/LastChar"),
            crate::parser::object::types::PdfObject::Integer(255),
        );

        let font = Type3Font::load(&font_dict);

        // No ToUnicode, encoding has no glyph for 0x41, no /CharProcs
        let result = resolve_type3(&font, None, 0x41, None, None, None, &mut diagnostics);

        assert!(result.is_failure());
        assert_eq!(result.chars.as_slice(), ['\u{FFFD}']);
        assert_eq!(result.source, UnicodeSource::Unknown);
        assert_eq!(result.confidence, 0.0);
        // Should have emitted diagnostic
        assert!(!diagnostics.is_empty());
    }
}
