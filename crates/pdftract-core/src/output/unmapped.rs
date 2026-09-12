//! Parser for `FONT_GLYPH_UNMAPPED` diagnostics in captured extraction output.
//!
//! This module is the *consumer* side of the unmapped-glyph diagnostics that
//! [`crate::font::resolver`] emits: it parses a captured extraction output
//! (the JSON document produced by the full-schema output mode) and turns every
//! `FONT_GLYPH_UNMAPPED` entry in its `errors` array into a structured
//! [`UnmappedGlyph`] record carrying the glyph identifier and the associated
//! wire metadata (severity, page index, object location, hint).
//!
//! # Message formats
//!
//! The diagnostic `message` text is the only place the glyph identifier is
//! encoded on the wire. There are exactly seven producer sites, all in
//! `crate::font::resolver`; the templates below are copied verbatim from
//! that module's header documentation.
//!
//! Regular fonts (Type1, TrueType, CIDFont, Type0) get one generic form, with
//! the whole byte string rendered as concatenated uppercase hex and the font
//! id in `Debug` form:
//!
//! ```text
//! Character code {HEX} could not be resolved to Unicode (font ID: {FONT_ID:?})
//! ```
//!
//! Type 3 fonts get specific forms, always with a single byte as `0x{02X}`
//! and no font id (the font is implicit in the call):
//!
//! ```text
//! Type3 font: character code 0x{02X} could not be resolved to Unicode
//! Type3 font: character code 0x{02X} could not be resolved (shape recognition disabled)
//! Type3 font: character code 0x{02X} has no glyph name in encoding
//! Type3 font: glyph '{NAME}' not found in /CharProcs for code 0x{02X}
//! Type3 font: failed to rasterize glyph '{NAME}' for code 0x{02X}
//! Type3 font: shape match for '{NAME}' (code 0x{02X}) found but distance {DIST} exceeds threshold
//! ```
//!
//! # Robustness contract
//!
//! * A capture with no `errors` key at all (an extraction that produced no
//!   diagnostics, or a capture from before the field existed) parses
//!   successfully with zero entries.
//! * Entries whose `code` is not `FONT_GLYPH_UNMAPPED` are counted in
//!   [`UnmappedGlyphScan::other_diagnostics`] and otherwise ignored.
//! * Entries with `code == "FONT_GLYPH_UNMAPPED"` whose `message` does not
//!   match any documented template — lowercase hex, odd-length hex, trailing
//!   whitespace, a `distance` that overflows `u32`, or plain format drift —
//!   are reported in [`UnmappedGlyphScan::malformed`] rather than dropped,
//!   so drift between the producer sites and this parser stays visible.
//! * Missing or wrongly-typed optional metadata fields (severity,
//!   `page_index`, `location`, `hint`) are represented as `None`; they do not
//!   make an entry malformed.
//!
//! # Example
//!
//! ```
//! use pdftract_core::output::unmapped::{scan_capture, MissReason};
//!
//! let capture = r#"{
//!     "schema_version": "1.0",
//!     "errors": [
//!         {"code": "FONT_GLYPH_UNMAPPED",
//!          "message": "Character code 41 could not be resolved to Unicode (font ID: FontId(7))",
//!          "severity": "warning"}
//!     ]
//! }"#;
//! let scan = scan_capture(capture).unwrap();
//! assert_eq!(scan.glyphs.len(), 1);
//! assert_eq!(scan.glyphs[0].code_bytes, vec![0x41]);
//! assert_eq!(scan.glyphs[0].font_id.as_deref(), Some("FontId(7)"));
//! assert_eq!(scan.glyphs[0].reason, MissReason::UnresolvedUnicode);
//! ```

use std::sync::OnceLock;

use regex::Regex;
use serde_json::Value;

/// Diagnostic code this module extracts (string form of
/// `DiagCode::FontGlyphUnmapped`, severity `warning`, recoverable).
pub const FONT_GLYPH_UNMAPPED_CODE: &str = "FONT_GLYPH_UNMAPPED";

/// The seven message templates, in the order they are matched.
///
/// Group meanings per form:
/// - 0 (generic font): `1` = concatenated uppercase hex, `2` = font id in
///   `Debug` form (e.g. `FontId(7)`)
/// - 1–3 (Type3, code only): `1` = two uppercase hex digits
/// - 4–5 (Type3, name + code): `1` = glyph name, `2` = two hex digits
/// - 6 (Type3, shape distance): `1` = glyph name, `2` = two hex digits,
///   `3` = Hamming distance
const MESSAGE_PATTERNS: [&str; 7] = [
    r"^Character code ([0-9A-F]+) could not be resolved to Unicode \(font ID: (.+)\)$",
    r"^Type3 font: character code 0x([0-9A-F]{2}) could not be resolved to Unicode$",
    r"^Type3 font: character code 0x([0-9A-F]{2}) could not be resolved \(shape recognition disabled\)$",
    r"^Type3 font: character code 0x([0-9A-F]{2}) has no glyph name in encoding$",
    r"^Type3 font: glyph '([^']*)' not found in /CharProcs for code 0x([0-9A-F]{2})$",
    r"^Type3 font: failed to rasterize glyph '([^']*)' for code 0x([0-9A-F]{2})$",
    r"^Type3 font: shape match for '([^']*)' \(code 0x([0-9A-F]{2})\) found but distance ([0-9]+) exceeds threshold$",
];

/// Compiled [`MESSAGE_PATTERNS`], built once on first use.
fn compiled_patterns() -> &'static [Regex] {
    static COMPILED: OnceLock<Vec<Regex>> = OnceLock::new();
    COMPILED.get_or_init(|| {
        MESSAGE_PATTERNS
            .iter()
            .map(|p| Regex::new(p).expect("invalid unmapped-glyph message regex"))
            .collect()
    })
}

/// Why a glyph could not be mapped, discriminated by message template.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissReason {
    /// Regular font: all four resolution levels missed.
    UnresolvedUnicode,
    /// Type3: all levels missed.
    Type3UnresolvedUnicode,
    /// Type3: Level 4 compiled out (no `shape-db` feature).
    Type3ShapeRecognitionDisabled,
    /// Type3: the encoding has no glyph name for the code.
    Type3NoGlyphNameInEncoding,
    /// Type3: glyph name present but absent from `/CharProcs` (malformed PDF).
    Type3GlyphNotInCharProcs,
    /// Type3: `/CharProcs` stream failed to rasterize (malformed PDF).
    Type3RasterizeFailed,
    /// Type3: Level 4 found a candidate but rejected it — Hamming distance
    /// above the threshold.
    Type3ShapeDistanceExceeded,
}

/// PDF object reference parsed from a diagnostic's `location` metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectLocation {
    /// Object number (zero-based index in the xref table).
    pub object_number: u32,
    /// Generation number (incremented on each save).
    pub generation_number: u16,
}

/// One `FONT_GLYPH_UNMAPPED` diagnostic, fully parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnmappedGlyph {
    /// Character-code bytes, hex-decoded from the message (`[0x1A, 0x2B]`).
    ///
    /// Single byte for the Type3 forms; the whole code string for the
    /// generic font form (multi-byte for CID two-byte codes).
    pub code_bytes: Vec<u8>,
    /// Uppercase hex rendering of the code, without the `0x` prefix,
    /// uniform across all seven forms ("1A2B", "41"); round-trips with the
    /// producer's concatenated `{:02X}` formatting.
    pub code_hex: String,
    /// Glyph name for the Type3 forms that carry one, else `None`.
    pub glyph_name: Option<String>,
    /// Font identifier in `Debug` form ("FontId(7)") for the generic font
    /// form; `None` for Type3 (the font is implicit in the call).
    ///
    /// Kept as a string rather than an integer: `FontId` wraps a
    /// pointer-derived `usize`, so the number routinely exceeds `u32`.
    pub font_id: Option<String>,
    /// Hamming distance for [`MissReason::Type3ShapeDistanceExceeded`],
    /// else `None`.
    pub distance: Option<u32>,
    /// Which producer template this message matched.
    pub reason: MissReason,

    // ---- wire metadata (`DiagnosticJson` passthrough) ----
    /// Severity as it appeared on the entry ("warning" for this code);
    /// `None` when absent or not a string.
    pub severity: Option<String>,
    /// Page index, or `None` for document-level events.
    pub page_index: Option<usize>,
    /// PDF object reference where the issue originated.
    pub location: Option<ObjectLocation>,
    /// Optional resolution hint.
    pub hint: Option<String>,
}

/// Why a `FONT_GLYPH_UNMAPPED` entry could not be parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MalformedReason {
    /// The `errors` array element is not a JSON object, so its code cannot
    /// be determined; reported for visibility rather than skipped.
    EntryNotAnObject,
    /// The entry has no `message` field.
    MessageMissing,
    /// The entry's `message` is not a string.
    MessageNotAString,
    /// The message matches none of the seven documented templates.
    MessageFormUnknown,
    /// Generic form: the hex code has odd length and cannot decode to bytes.
    HexOddLength,
    /// Shape-distance form: the distance does not fit in `u32`.
    DistanceOverflow,
}

/// A `FONT_GLYPH_UNMAPPED` entry that failed to parse, kept for diagnosis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MalformedEntry {
    /// The verbatim message text, when one was present and was a string.
    pub message: Option<String>,
    /// Why the entry was rejected.
    pub reason: MalformedReason,
}

/// Result of scanning a captured output for unmapped glyphs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UnmappedGlyphScan {
    /// The capture's `schema_version`, when present.
    pub schema_version: Option<String>,
    /// Successfully parsed entries, in `errors` order.
    pub glyphs: Vec<UnmappedGlyph>,
    /// Entries with the right code but an unparseable message.
    pub malformed: Vec<MalformedEntry>,
    /// Number of `errors` entries carrying some other diagnostic code.
    pub other_diagnostics: usize,
}

/// Structural failure of the capture document itself.
#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    /// The capture is not valid JSON.
    #[error("captured output is not valid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),
    /// `errors` is present but is not an array (and not `null`).
    #[error("`errors` is present but is not an array")]
    ErrorsNotAnArray,
}

/// Parse one `FONT_GLYPH_UNMAPPED` message text.
///
/// Matching is strict: the whole message must equal one of the seven
/// documented templates (no leading/trailing slack, uppercase hex only).
/// Wire metadata fields are left `None`; the [`scan_value`]/[`scan_capture`]
/// entry scanners fill them from the surrounding JSON object.
///
/// # Errors
///
/// Returns the [`MalformedReason`] describing why the message did not parse;
/// [`MalformedReason::MessageFormUnknown`] is the catch-all for text that
/// matches no template.
pub fn parse_message(message: &str) -> Result<UnmappedGlyph, MalformedReason> {
    let regexes = compiled_patterns();

    let bare = || UnmappedGlyph {
        code_bytes: Vec::new(),
        code_hex: String::new(),
        glyph_name: None,
        font_id: None,
        distance: None,
        reason: MissReason::UnresolvedUnicode,
        severity: None,
        page_index: None,
        location: None,
        hint: None,
    };

    // Form 0: generic regular-font miss (multi-byte capable, font id present).
    if let Some(caps) = regexes[0].captures(message) {
        let code_hex = caps[1].to_string();
        if code_hex.len() % 2 != 0 {
            return Err(MalformedReason::HexOddLength);
        }
        let code_bytes = decode_hex_pairs(&code_hex)
            .map_err(|_| MalformedReason::MessageFormUnknown)?;
        return Ok(UnmappedGlyph {
            code_bytes,
            code_hex,
            font_id: Some(caps[2].to_string()),
            ..bare()
        });
    }

    // Forms 1–3: Type3 "character code 0x{02X}" variants without a glyph name.
    if let Some(caps) = regexes[1].captures(message) {
        return type3_code_only(&caps[1], MissReason::Type3UnresolvedUnicode, bare());
    }
    if let Some(caps) = regexes[2].captures(message) {
        return type3_code_only(
            &caps[1],
            MissReason::Type3ShapeRecognitionDisabled,
            bare(),
        );
    }
    if let Some(caps) = regexes[3].captures(message) {
        return type3_code_only(&caps[1], MissReason::Type3NoGlyphNameInEncoding, bare());
    }

    // Forms 4–5: Type3 variants carrying a glyph name.
    if let Some(caps) = regexes[4].captures(message) {
        return type3_named(
            &caps[1],
            &caps[2],
            MissReason::Type3GlyphNotInCharProcs,
            bare(),
        );
    }
    if let Some(caps) = regexes[5].captures(message) {
        return type3_named(&caps[1], &caps[2], MissReason::Type3RasterizeFailed, bare());
    }

    // Form 6: Type3 shape match rejected past the Hamming threshold.
    if let Some(caps) = regexes[6].captures(message) {
        let mut glyph = type3_named(
            &caps[1],
            &caps[2],
            MissReason::Type3ShapeDistanceExceeded,
            bare(),
        )?;
        glyph.distance = Some(
            caps[3]
                .parse::<u32>()
                .map_err(|_| MalformedReason::DistanceOverflow)?,
        );
        return Ok(glyph);
    }

    Err(MalformedReason::MessageFormUnknown)
}

/// Fill a code-only Type3 record from a two-hex-digit capture group.
fn type3_code_only(
    hex: &str,
    reason: MissReason,
    base: UnmappedGlyph,
) -> Result<UnmappedGlyph, MalformedReason> {
    let byte = u8::from_str_radix(hex, 16).map_err(|_| MalformedReason::MessageFormUnknown)?;
    Ok(UnmappedGlyph {
        code_bytes: vec![byte],
        code_hex: hex.to_string(),
        reason,
        ..base
    })
}

/// Fill a name+code Type3 record from its capture groups.
fn type3_named(
    name: &str,
    hex: &str,
    reason: MissReason,
    base: UnmappedGlyph,
) -> Result<UnmappedGlyph, MalformedReason> {
    let byte = u8::from_str_radix(hex, 16).map_err(|_| MalformedReason::MessageFormUnknown)?;
    Ok(UnmappedGlyph {
        code_bytes: vec![byte],
        code_hex: hex.to_string(),
        glyph_name: Some(name.to_string()),
        reason,
        ..base
    })
}

/// Decode an even-length uppercase-hex string into bytes.
///
/// The regex prevalidates the digits, so failure here is unreachable today;
/// it maps to [`MalformedReason::MessageFormUnknown`] so a future regex edit
/// degrades to a malformed entry instead of a panic.
fn decode_hex_pairs(hex: &str) -> Result<Vec<u8>, ()> {
    hex.as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).map_err(|_| ())?, 16).map_err(|_| ()))
        .collect()
}

/// Scan an already-parsed JSON value for `FONT_GLYPH_UNMAPPED` entries.
///
/// The value is expected to be an object carrying an `errors` array — the
/// full-document [`crate::schema::Output`] envelope, or any single frame of
/// the streaming NDJSON output (page frames and the footer both carry
/// `errors`). A value with no `errors` key scans to zero entries; `null` is
/// treated as absent.
///
/// # Errors
///
/// [`ScanError::ErrorsNotAnArray`] if `errors` is present with some other
/// JSON type.
pub fn scan_value(value: &Value) -> Result<UnmappedGlyphScan, ScanError> {
    let mut scan = UnmappedGlyphScan::default();
    let Some(obj) = value.as_object() else {
        return Ok(scan);
    };

    scan.schema_version = obj
        .get("schema_version")
        .and_then(Value::as_str)
        .map(str::to_owned);

    match obj.get("errors") {
        None | Some(Value::Null) => {}
        Some(Value::Array(errors)) => scan_errors_array(errors, &mut scan),
        Some(_) => return Err(ScanError::ErrorsNotAnArray),
    }
    Ok(scan)
}

/// Scan a captured full-document JSON extraction output for unmapped glyphs.
///
/// # Errors
///
/// [`ScanError::InvalidJson`] if the text is not valid JSON;
/// [`ScanError::ErrorsNotAnArray`] if `errors` has a non-array type.
pub fn scan_capture(json_text: &str) -> Result<UnmappedGlyphScan, ScanError> {
    let value: Value = serde_json::from_str(json_text)?;
    scan_value(&value)
}

/// Fold one `errors` array into `scan`.
fn scan_errors_array(errors: &[Value], scan: &mut UnmappedGlyphScan) {
    for entry in errors {
        let Some(obj) = entry.as_object() else {
            scan.malformed.push(MalformedEntry {
                message: None,
                reason: MalformedReason::EntryNotAnObject,
            });
            continue;
        };

        let code = obj.get("code").and_then(Value::as_str);
        if code != Some(FONT_GLYPH_UNMAPPED_CODE) {
            scan.other_diagnostics += 1;
            continue;
        }

        let message = match obj.get("message") {
            None | Some(Value::Null) => {
                scan.malformed.push(MalformedEntry {
                    message: None,
                    reason: MalformedReason::MessageMissing,
                });
                continue;
            }
            Some(Value::String(s)) => s.as_str(),
            Some(_) => {
                scan.malformed.push(MalformedEntry {
                    message: None,
                    reason: MalformedReason::MessageNotAString,
                });
                continue;
            }
        };

        match parse_message(message) {
            Ok(mut glyph) => {
                fill_metadata(obj, &mut glyph);
                scan.glyphs.push(glyph);
            }
            Err(reason) => {
                scan.malformed.push(MalformedEntry {
                    message: Some(message.to_string()),
                    reason,
                });
            }
        }
    }
}

/// Copy the `DiagnosticJson` metadata fields of `obj` onto `glyph`.
///
/// Missing or wrongly-typed fields stay `None`; they never malform the entry.
fn fill_metadata(obj: &serde_json::Map<String, Value>, glyph: &mut UnmappedGlyph) {
    glyph.severity = obj
        .get("severity")
        .and_then(Value::as_str)
        .map(str::to_owned);
    glyph.page_index = obj
        .get("page_index")
        .and_then(Value::as_u64)
        .and_then(|v| usize::try_from(v).ok());
    glyph.location = parse_location(obj.get("location"));
    glyph.hint = obj.get("hint").and_then(Value::as_str).map(str::to_owned);
}

/// Parse an `ObjectLocationJson`-shaped value.
fn parse_location(value: Option<&Value>) -> Option<ObjectLocation> {
    let obj = value?.as_object()?;
    Some(ObjectLocation {
        object_number: u32::try_from(obj.get("object_number")?.as_u64()?).ok()?,
        generation_number: u16::try_from(obj.get("generation_number")?.as_u64()?).ok()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The real capture from `notes/bf-3j4ec-json-output.txt` (2026-09-01),
    /// verbatim: a schema-1.0 document with an empty extraction and **no
    /// `errors` key at all** — the documented zero-diagnostic capture shape.
    const REAL_CAPTURE: &str = r#"{
  "attachments": [],
  "fingerprint": "pdftract-v1:ab24a95f44ceca5d2aed4b6d056adddd8539f44c6cd6ca506534e830c82ea8a8",
  "form_fields": [],
  "javascript_actions": [],
  "links": [],
  "metadata": {
    "block_count": 0,
    "cache_age_seconds": null,
    "cache_status": "skipped",
    "page_count": 0,
    "reading_order_algorithm": "xy_cut",
    "span_count": 0
  },
  "pages": [],
  "schema_version": "1.0",
  "signatures": [],
  "threads": []
}"#;

    // ---- parse_message: the seven documented forms ----

    #[test]
    fn parse_generic_form_single_byte() {
        let glyph = parse_message(
            "Character code 41 could not be resolved to Unicode (font ID: FontId(7))",
        )
        .unwrap();
        assert_eq!(glyph.code_bytes, vec![0x41]);
        assert_eq!(glyph.code_hex, "41");
        assert_eq!(glyph.font_id.as_deref(), Some("FontId(7)"));
        assert_eq!(glyph.glyph_name, None);
        assert_eq!(glyph.distance, None);
        assert_eq!(glyph.reason, MissReason::UnresolvedUnicode);
        // Bare-message parse carries no wire metadata.
        assert_eq!(glyph.severity, None);
        assert_eq!(glyph.page_index, None);
        assert_eq!(glyph.location, None);
        assert_eq!(glyph.hint, None);
    }

    #[test]
    fn parse_generic_form_multi_byte_cid_code() {
        // Two-byte CID code: concatenated {:02X} of the whole byte string.
        let glyph = parse_message(
            "Character code 1A2B could not be resolved to Unicode (font ID: FontId(140234553156816))",
        )
        .unwrap();
        assert_eq!(glyph.code_bytes, vec![0x1A, 0x2B]);
        assert_eq!(glyph.code_hex, "1A2B");
        assert_eq!(glyph.font_id.as_deref(), Some("FontId(140234553156816)"));
        assert_eq!(glyph.reason, MissReason::UnresolvedUnicode);
    }

    #[test]
    fn parse_generic_form_leading_zero_byte() {
        // A single 0x05 byte renders as "05", not "5".
        let glyph = parse_message(
            "Character code 05 could not be resolved to Unicode (font ID: FontId(0))",
        )
        .unwrap();
        assert_eq!(glyph.code_bytes, vec![0x05]);
        assert_eq!(glyph.code_hex, "05");
    }

    #[test]
    fn parse_type3_code_only_forms() {
        let cases = [
            (
                "Type3 font: character code 0x41 could not be resolved to Unicode",
                MissReason::Type3UnresolvedUnicode,
                0x41u8,
            ),
            (
                "Type3 font: character code 0x0A could not be resolved (shape recognition disabled)",
                MissReason::Type3ShapeRecognitionDisabled,
                0x0A,
            ),
            (
                "Type3 font: character code 0xFF has no glyph name in encoding",
                MissReason::Type3NoGlyphNameInEncoding,
                0xFF,
            ),
        ];
        for (message, reason, byte) in cases {
            let glyph = parse_message(message).unwrap();
            assert_eq!(glyph.reason, reason, "reason for {message:?}");
            assert_eq!(glyph.code_bytes, vec![byte], "code for {message:?}");
            assert_eq!(glyph.code_hex, format!("{byte:02X}"));
            assert_eq!(glyph.glyph_name, None, "no name for {message:?}");
            assert_eq!(glyph.font_id, None, "no font id for {message:?}");
            assert_eq!(glyph.distance, None);
        }
    }

    #[test]
    fn parse_type3_named_forms() {
        let cases = [
            (
                "Type3 font: glyph 'ampersand' not found in /CharProcs for code 0x26",
                MissReason::Type3GlyphNotInCharProcs,
                0x26,
                "ampersand",
            ),
            (
                "Type3 font: failed to rasterize glyph 'g27' for code 0x1B",
                MissReason::Type3RasterizeFailed,
                0x1B,
                "g27",
            ),
        ];
        for (message, reason, byte, name) in cases {
            let glyph = parse_message(message).unwrap();
            assert_eq!(glyph.reason, reason, "reason for {message:?}");
            assert_eq!(glyph.code_bytes, vec![byte], "code for {message:?}");
            assert_eq!(glyph.glyph_name.as_deref(), Some(name));
            assert_eq!(glyph.font_id, None);
        }
    }

    #[test]
    fn parse_type3_shape_distance_form() {
        let glyph = parse_message(
            "Type3 font: shape match for 'alpha' (code 0x61) found but distance 12 exceeds threshold",
        )
        .unwrap();
        assert_eq!(glyph.reason, MissReason::Type3ShapeDistanceExceeded);
        assert_eq!(glyph.code_bytes, vec![0x61]);
        assert_eq!(glyph.glyph_name.as_deref(), Some("alpha"));
        assert_eq!(glyph.distance, Some(12));
    }

    /// Round-trip against the producer's own `format!` calls
    /// (resolver.rs `emit_miss_diagnostic`): identical construction,
    /// identical parsing back.
    #[test]
    fn roundtrip_generic_form_against_producer_formatting() {
        for bytes in [&[0x41u8][..], &[0x00, 0xFF][..], &[0xDE, 0xAD, 0xBE, 0xEF][..]] {
            let hex_string: String = bytes.iter().map(|b| format!("{b:02X}")).collect();
            // FontId(usize) has a derived Debug, so {:?} renders "FontId(42)".
            let font_id_debug = "FontId(42)";
            let message = format!(
                "Character code {hex_string} could not be resolved to Unicode (font ID: {font_id_debug})"
            );
            let glyph = parse_message(&message).unwrap();
            assert_eq!(glyph.code_bytes, bytes);
            assert_eq!(glyph.code_hex, hex_string);
            assert_eq!(glyph.font_id.as_deref(), Some("FontId(42)"));
        }
    }

    // ---- parse_message: edge cases and malformed input ----

    #[test]
    fn parse_message_rejects_odd_length_hex() {
        let err = parse_message(
            "Character code 1AB could not be resolved to Unicode (font ID: FontId(7))",
        )
        .unwrap_err();
        assert_eq!(err, MalformedReason::HexOddLength);
    }

    #[test]
    fn parse_message_rejects_distance_overflow() {
        let err = parse_message(
            "Type3 font: shape match for 'a' (code 0x61) found but distance 4294967296 exceeds threshold",
        )
        .unwrap_err();
        assert_eq!(err, MalformedReason::DistanceOverflow);
    }

    #[test]
    fn parse_message_rejects_unrecognized_forms() {
        let bad = [
            "",
            "   ",
            "totally unrelated",
            // Lowercase hex is never produced ({:02X} is uppercase).
            "Character code 1a could not be resolved to Unicode (font ID: FontId(7))",
            "Type3 font: character code 0x0a could not be resolved to Unicode",
            // Missing fields.
            "Character code 41 could not be resolved to Unicode",
            "Type3 font: character code 0x41 could not be resolved",
            // Trailing/leading whitespace — matching is exact.
            "Character code 41 could not be resolved to Unicode (font ID: FontId(7)) ",
            " Type3 font: character code 0x41 has no glyph name in encoding",
            // Empty font id / empty hex.
            "Character code  could not be resolved to Unicode (font ID: )",
            "Character code 41 could not be resolved to Unicode (font ID: )",
            // One-digit hex is odd-length but hits no template boundary:
            // the generic regex requires at least one hex digit, so a lone
            // non-hex word is form-unknown.
            "Character code XYZ could not be resolved to Unicode (font ID: FontId(7))",
        ];
        for message in bad {
            assert_eq!(
                parse_message(message).unwrap_err(),
                MalformedReason::MessageFormUnknown,
                "expected form-unknown for {message:?}"
            );
        }
    }

    // ---- scan_capture / scan_value ----

    #[test]
    fn scan_capture_with_full_metadata() {
        let capture = r#"{
  "schema_version": "1.0",
  "errors": [
    {"code": "STREAM_DECODE_ERROR", "message": "unrelated", "severity": "warning"},
    {"code": "FONT_GLYPH_UNMAPPED",
     "message": "Character code 42 could not be resolved to Unicode (font ID: FontId(9))",
     "severity": "warning",
     "page_index": 3,
     "location": {"object_number": 12, "generation_number": 0},
     "hint": "Install Tesseract for OCR recovery"},
    {"code": "FONT_GLYPH_UNMAPPED",
     "message": "Type3 font: glyph 'bullet' not found in /CharProcs for code 0x95",
     "severity": "warning"}
  ]
}"#;
        let scan = scan_capture(capture).unwrap();
        assert_eq!(scan.schema_version.as_deref(), Some("1.0"));
        assert_eq!(scan.glyphs.len(), 2);
        assert_eq!(scan.other_diagnostics, 1);
        assert!(scan.malformed.is_empty());

        let first = &scan.glyphs[0];
        assert_eq!(first.code_bytes, vec![0x42]);
        assert_eq!(first.font_id.as_deref(), Some("FontId(9)"));
        assert_eq!(first.severity.as_deref(), Some("warning"));
        assert_eq!(first.page_index, Some(3));
        assert_eq!(
            first.location,
            Some(ObjectLocation {
                object_number: 12,
                generation_number: 0
            })
        );
        assert_eq!(first.hint.as_deref(), Some("Install Tesseract for OCR recovery"));

        let second = &scan.glyphs[1];
        assert_eq!(second.glyph_name.as_deref(), Some("bullet"));
        assert_eq!(second.reason, MissReason::Type3GlyphNotInCharProcs);
        // Optional metadata absent -> None, entry still parses.
        assert_eq!(second.severity.as_deref(), Some("warning"));
        assert_eq!(second.page_index, None);
        assert_eq!(second.location, None);
        assert_eq!(second.hint, None);
    }

    #[test]
    fn scan_capture_on_real_captured_output() {
        // The documented capture contains no `errors` key at all; that is a
        // successful scan with zero entries, not an error.
        let scan = scan_capture(REAL_CAPTURE).unwrap();
        assert_eq!(scan.schema_version.as_deref(), Some("1.0"));
        assert!(scan.glyphs.is_empty());
        assert!(scan.malformed.is_empty());
        assert_eq!(scan.other_diagnostics, 0);
    }

    #[test]
    fn scan_capture_without_errors_key() {
        let scan = scan_capture(r#"{"schema_version": "1.0"}"#).unwrap();
        assert!(scan.glyphs.is_empty());
        assert_eq!(scan.schema_version.as_deref(), Some("1.0"));
    }

    #[test]
    fn scan_capture_with_null_errors() {
        let scan = scan_capture(r#"{"schema_version": "1.0", "errors": null}"#).unwrap();
        assert!(scan.glyphs.is_empty());
    }

    #[test]
    fn scan_capture_with_empty_errors_array() {
        let scan = scan_capture(r#"{"errors": []}"#).unwrap();
        assert!(scan.glyphs.is_empty());
        assert_eq!(scan.schema_version, None);
    }

    #[test]
    fn scan_capture_non_object_root_is_empty() {
        // Lenient by design: valid JSON that is not an object has no errors
        // array to scan.
        let scan = scan_capture("5").unwrap();
        assert_eq!(scan, UnmappedGlyphScan::default());
    }

    #[test]
    fn scan_capture_rejects_non_array_errors() {
        let err = scan_capture(r#"{"errors": "oops"}"#).unwrap_err();
        assert!(matches!(err, ScanError::ErrorsNotAnArray));
    }

    #[test]
    fn scan_capture_rejects_invalid_json() {
        let err = scan_capture("{not json").unwrap_err();
        assert!(matches!(err, ScanError::InvalidJson(_)));
    }

    #[test]
    fn scan_capture_collects_malformed_entries() {
        let capture = r#"{
  "errors": [
    42,
    {"code": "FONT_GLYPH_UNMAPPED"},
    {"code": "FONT_GLYPH_UNMAPPED", "message": null},
    {"code": "FONT_GLYPH_UNMAPPED", "message": 5},
    {"code": "FONT_GLYPH_UNMAPPED", "message": "format drift"},
    {"code": "FONT_GLYPH_UNMAPPED", "message": "Character code 4 could not be resolved to Unicode (font ID: FontId(7))"},
    {"code": "FONT_GLYPH_UNMAPPED",
     "message": "Type3 font: shape match for 'a' (code 0x61) found but distance 99999999999 exceeds threshold"}
  ]
}"#;
        let scan = scan_capture(capture).unwrap();
        assert!(scan.glyphs.is_empty());
        let reasons: Vec<MalformedReason> = scan.malformed.iter().map(|m| m.reason).collect();
        assert_eq!(
            reasons,
            vec![
                MalformedReason::EntryNotAnObject,
                MalformedReason::MessageMissing,
                MalformedReason::MessageMissing,
                MalformedReason::MessageNotAString,
                MalformedReason::MessageFormUnknown,
                MalformedReason::HexOddLength,
                MalformedReason::DistanceOverflow,
            ]
        );
        // Malformed entries keep their verbatim message for diagnosis.
        assert_eq!(scan.malformed[4].message.as_deref(), Some("format drift"));
        assert_eq!(scan.malformed[0].message, None);
    }

    #[test]
    fn scan_capture_tolerates_wrongly_typed_metadata() {
        let capture = r#"{
  "errors": [
    {"code": "FONT_GLYPH_UNMAPPED",
     "message": "Type3 font: character code 0x41 could not be resolved to Unicode",
     "severity": 3,
     "page_index": "zero",
     "location": {"object_number": -1},
     "hint": null}
  ]
}"#;
        let scan = scan_capture(capture).unwrap();
        assert_eq!(scan.glyphs.len(), 1);
        let glyph = &scan.glyphs[0];
        assert_eq!(glyph.severity, None);
        assert_eq!(glyph.page_index, None);
        assert_eq!(glyph.location, None);
        assert_eq!(glyph.hint, None);
        assert!(scan.malformed.is_empty());
    }

    #[test]
    fn scan_value_works_on_ndjson_page_frame() {
        // Streaming page frames carry the same DiagnosticJson shape in their
        // `errors` field, so scan_value applies to them directly.
        let frame: Value = serde_json::json!({
            "type": "page",
            "page_index": 0,
            "errors": [
                {"code": "FONT_GLYPH_UNMAPPED",
                 "message": "Type3 font: failed to rasterize glyph 'g3' for code 0x33",
                 "severity": "warning"}
            ]
        });
        let scan = scan_value(&frame).unwrap();
        assert_eq!(scan.glyphs.len(), 1);
        assert_eq!(scan.glyphs[0].glyph_name.as_deref(), Some("g3"));
        assert_eq!(scan.glyphs[0].reason, MissReason::Type3RasterizeFailed);
    }

    #[test]
    fn entries_preserve_errors_order() {
        let capture = r#"{"errors": [
            {"code": "FONT_GLYPH_UNMAPPED", "message": "Type3 font: character code 0x01 could not be resolved to Unicode"},
            {"code": "OTHER", "message": "x"},
            {"code": "FONT_GLYPH_UNMAPPED", "message": "Type3 font: character code 0x02 could not be resolved to Unicode"}
        ]}"#;
        let scan = scan_capture(capture).unwrap();
        let hexes: Vec<&str> = scan.glyphs.iter().map(|g| g.code_hex.as_str()).collect();
        assert_eq!(hexes, vec!["01", "02"]);
    }
}
