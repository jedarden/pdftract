//! Canonical typed diagnostics and the legacy string-array compatibility adapter.
//!
//! pdftract exposes diagnostics on two surfaces. This module is the documented
//! bridge between them: it defines which surface is canonical, which is
//! compatibility-only, and how a typed diagnostic converts into the legacy
//! form without changing a single byte for existing callers.
//!
//! # Canonical surface (typed)
//!
//! [`Diagnostic`](crate::diagnostics::Diagnostic) is the canonical in-process
//! representation: a [`DiagCode`](crate::diagnostics::DiagCode), a
//! human-readable `message`, an optional `byte_offset`, an optional
//! `object_ref` (object/generation pair), and an optional `page_index`.
//! Severity is derived from the code via
//! [`DiagCode::severity`](crate::diagnostics::DiagCode::severity) and is one
//! of the four supported [`Severity`](crate::diagnostics::Severity) variants.
//!
//! For machine-readable output the typed model serializes through
//! [`DiagnosticJson`](crate::schema::DiagnosticJson) with these stable JSON
//! field names:
//!
//! | Field | Type | Present |
//! |---|---|---|
//! | `code` | string (`DiagCode::name` form, e.g. `"XREF_REPAIRED"`) | always |
//! | `message` | string | always |
//! | `severity` | string: `"info"` \| `"warning"` \| `"error"` \| `"fatal"` | always |
//! | `page_index` | number (zero-based) | only when known |
//! | `location` | `{ "object_number", "generation_number" }` | only when known |
//! | `hint` | string (remediation suggestion) | only when known |
//!
//! Optional fields are omitted (not `null`) when unknown, so adding new
//! optional fields is a backward-compatible change. These names are a public
//! contract (consumed by `metadata.diagnostics_detailed` in JSON/NDJSON output
//! and documented in `docs/integrations/diagnostics-codes.md`); renaming or
//! re-typing any of them is a breaking change.
//!
//! # Legacy surface (compatibility)
//!
//! `ExtractionResult.metadata.diagnostics` is `Vec<String>`: one plain string
//! per diagnostic, in emission order, **the diagnostic's `message` verbatim**.
//! This is the shape every current caller sees — the producers in
//! `extract.rs` have always pushed `d.message` into it. The legacy form drops
//! the code, severity, page index, object location, and hint, and it renders
//! two diagnostics that differ only in code identically.
//!
//! Note the deliberate difference from `Diagnostic`'s `Display` impl (which
//! renders `"CODE: message (byte offset N) [obj gen R]"`): the legacy array
//! carries the bare message only. Byte-for-byte compatibility with current
//! callers means `to_legacy_string` returns `message` exactly — no prefix, no
//! suffix, no trimming.
//!
//! # Compatibility contract
//!
//! 1. For a given emission sequence, the legacy strings produced by
//!    [`to_legacy_strings`] must remain byte-for-byte identical across
//!    releases. Any change there breaks every consumer of
//!    `metadata.diagnostics`.
//! 2. New information belongs on the canonical typed/JSON surface (additive,
//!    optional, omitted when unknown). It must not be folded into the legacy
//!    strings.
//! 3. The severity strings `"info"`, `"warning"`, `"error"`, `"fatal"` are
//!    pinned by tests here and in the catalog drift suite; new severities are
//!    a breaking change to both surfaces.
//!
//! # Example
//!
//! ```rust
//! use pdftract_core::diagnostics::{DiagCode, Diagnostic};
//! use pdftract_core::diagnostics_compat::to_legacy_strings;
//!
//! let typed = vec![
//!     Diagnostic::with_static_no_offset(DiagCode::XrefRepaired, "xref rebuilt"),
//!     Diagnostic::with_static_no_offset(DiagCode::FontNotFound, "font missing"),
//! ];
//!
//! // Legacy surface: message verbatim, order and multiplicity preserved.
//! let legacy = to_legacy_strings(&typed);
//! assert_eq!(legacy, vec!["xref rebuilt", "font missing"]);
//! ```

use crate::diagnostics::Diagnostic;

/// Convert one typed diagnostic into its legacy string form.
///
/// The legacy surface (`ExtractionResult.metadata.diagnostics: Vec<String>`)
/// carries the diagnostic's message verbatim; see the module docs for the
/// full canonical/compatibility contract. Order- and byte-stable: callers
/// converting a whole emission sequence get exactly the strings the inline
/// `d.message.as_ref().to_string()` producers in `extract.rs` emit today.
#[inline]
pub fn to_legacy_string(diag: &Diagnostic) -> String {
    diag.message.as_ref().to_string()
}

/// Convert a slice of typed diagnostics into the legacy `Vec<String>` form.
///
/// Preserves length, order, and duplicates: element `i` is
/// [`to_legacy_string`] of `diags[i]`. This is the drop-in replacement for
/// the scattered inline conversion sites; it must remain byte-for-byte
/// identical to them (contract rule 1 in the module docs).
pub fn to_legacy_strings(diags: &[Diagnostic]) -> Vec<String> {
    diags.iter().map(to_legacy_string).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::{DiagCode, ObjRef, Severity};
    use crate::schema::{DiagnosticJson, ObjectLocationJson};

    fn sample(code: DiagCode) -> Diagnostic {
        Diagnostic::with_static(code, 1234, "sample message")
            .with_object_ref(ObjRef::new(7, 0))
            .with_page_index(3)
    }

    // --- typed-to-legacy conversion -------------------------------------

    #[test]
    fn typed_to_legacy_string_is_message_verbatim() {
        let diag = Diagnostic::with_static(DiagCode::StructInvalidName, 42, "bad name at 42");
        assert_eq!(to_legacy_string(&diag), "bad name at 42");

        // No code prefix, no byte-offset suffix, no trimming — the legacy
        // surface is the bare message, unlike Display's
        // "CODE: message (byte offset N)" form.
        let converted = to_legacy_string(&diag);
        assert!(!converted.starts_with("STRUCT_INVALID_NAME"));
        assert!(!converted.contains("byte offset"));
        assert_eq!(converted, converted.trim());

        // Multibyte and colon-bearing messages survive untouched.
        let odd = Diagnostic::with_dynamic_no_offset(
            DiagCode::FontGlyphUnmapped,
            "résumé — page 2: text runs (byte offset 9)".to_string(),
        );
        assert_eq!(
            to_legacy_string(&odd),
            "résumé — page 2: text runs (byte offset 9)"
        );

        // Empty messages stay empty (not skipped or placeholders).
        let empty = Diagnostic::with_static_no_offset(DiagCode::XrefTruncated, "");
        assert_eq!(to_legacy_string(&empty), "");
    }

    #[test]
    fn typed_to_legacy_preserves_order_length_and_duplicates() {
        let diags = vec![
            Diagnostic::with_static_no_offset(DiagCode::XrefRepaired, "first"),
            Diagnostic::with_static_no_offset(DiagCode::XrefRepaired, "first"),
            Diagnostic::with_static_no_offset(DiagCode::StreamBomb, "second"),
        ];
        let legacy = to_legacy_strings(&diags);
        assert_eq!(legacy.len(), diags.len());
        assert_eq!(legacy, vec!["first", "first", "second"]);

        // Empty input converts to an empty legacy array (which the JSON
        // writer then omits via skip_serializing_if = "Vec::is_empty").
        assert!(to_legacy_strings(&[]).is_empty());
    }

    #[test]
    fn golden_legacy_bytes_match_inline_producer_expression() {
        // Byte-for-byte golden: `to_legacy_strings` must produce exactly the
        // bytes of the inline conversion shape the extract.rs producers
        // emitted before the shared helper existed (`d.message.as_ref()
        // .to_string()`), for every combination of optional fields
        // (byte_offset / object_ref / page_index, each set and unset) and
        // every supported severity. Optional fields and severity must never
        // leak into the legacy bytes; duplicates and order survive.
        let tiers = [
            (DiagCode::XrefRepaired, "info-tier note"),
            (DiagCode::StructInvalidName, "warning-tier note"),
            (DiagCode::StreamBomb, "error-tier note"),
            (DiagCode::EncryptionUnsupported, "fatal-tier note"),
        ];
        // The four tiers really do cover all four severities.
        let severities: std::collections::HashSet<_> = tiers
            .iter()
            .map(|(code, _)| code.severity().to_string())
            .collect();
        assert_eq!(severities.len(), 4);

        let mut diags = Vec::new();
        for &(code, base) in &tiers {
            for byte_offset in [None, Some(1234u64)] {
                for object_ref in [None, Some(ObjRef::new(7, 0))] {
                    for page_index in [None, Some(3usize)] {
                        // Exercise both Cow storage variants along the way.
                        let mut diag = match byte_offset {
                            Some(offset) => {
                                Diagnostic::with_dynamic(code, offset, base.to_string())
                            }
                            None => Diagnostic::with_static_no_offset(code, base),
                        };
                        if let Some(reference) = object_ref {
                            diag = diag.with_object_ref(reference);
                        }
                        if let Some(page) = page_index {
                            diag = diag.with_page_index(page);
                        }
                        diags.push(diag);
                    }
                }
            }
        }

        // Golden expectation: the historical inline producer expression,
        // element for element, in emission order.
        let expected: Vec<String> = diags
            .iter()
            .map(|d| d.message.as_ref().to_string())
            .collect();
        assert_eq!(to_legacy_strings(&diags), expected);

        // Within a tier the message is constant, so all 8 optional-field
        // variants must render identical bytes — the bare message, not the
        // Display form (no code prefix, no offset, no object reference).
        for &(code, base) in &tiers {
            let variants: Vec<String> = diags
                .iter()
                .filter(|d| d.code == code)
                .map(to_legacy_string)
                .collect();
            assert_eq!(variants.len(), 8);
            assert!(variants.iter().all(|s| s == base));
            assert!(
                !variants
                    .iter()
                    .any(|s| s.contains("byte offset") || s.contains('[')),
                "optional fields leaked into the legacy string: {variants:?}"
            );
        }

        // Duplicates and mixed order survive the same way: the converted
        // sequence equals the inline expression over that exact sequence.
        let mut mixed = diags.clone();
        mixed.reverse();
        mixed.push(diags[0].clone());
        let mixed_expected: Vec<String> = mixed
            .iter()
            .map(|d| d.message.as_ref().to_string())
            .collect();
        assert_eq!(to_legacy_strings(&mixed), mixed_expected);
        assert_eq!(mixed.len(), mixed_expected.len());
    }

    #[test]
    fn legacy_surface_drops_severity_and_location() {
        // Same message, different codes (hence severities and locations):
        // the legacy strings are identical, because the legacy surface
        // carries the message only.
        let warning = Diagnostic::with_static(DiagCode::StructInvalidName, 9, "same text")
            .with_object_ref(ObjRef::new(1, 0))
            .with_page_index(0);
        let fatal = Diagnostic::with_static(DiagCode::EncryptionUnsupported, 9, "same text")
            .with_object_ref(ObjRef::new(2, 3))
            .with_page_index(4);
        assert_ne!(warning.code, fatal.code);
        assert_ne!(warning.severity(), fatal.severity());
        assert_eq!(to_legacy_string(&warning), to_legacy_string(&fatal));
    }

    // --- supported severities -------------------------------------------

    #[test]
    fn all_supported_severities_render_documented_strings() {
        // The complete set of severities; a new variant must be added here
        // (and to the JSON consumers) — this match is exhaustive, so the
        // compiler enforces the update.
        let documented = |s: Severity| match s {
            Severity::Info => "info",
            Severity::Warning => "warning",
            Severity::Error => "error",
            Severity::Fatal => "fatal",
        };
        let all = [
            Severity::Info,
            Severity::Warning,
            Severity::Error,
            Severity::Fatal,
        ];
        assert_eq!(documented(Severity::Info), "info");
        for s in all {
            assert_eq!(s.to_string(), documented(s));
        }
        // All four renderings are distinct.
        let strings: Vec<_> = all.iter().map(|s| documented(*s)).collect();
        for (i, a) in strings.iter().enumerate() {
            for b in strings.iter().skip(i + 1) {
                assert_ne!(a, b);
            }
        }
    }

    #[test]
    fn code_severities_cover_every_supported_level() {
        // One code per supported severity, pinned so a catalog reshuffle that
        // changes a code's severity fails loudly here.
        assert_eq!(DiagCode::XrefRepaired.severity(), Severity::Info);
        assert_eq!(DiagCode::StructInvalidName.severity(), Severity::Warning);
        assert_eq!(DiagCode::StreamBomb.severity(), Severity::Error);
        assert_eq!(DiagCode::EncryptionUnsupported.severity(), Severity::Fatal);
    }

    // --- canonical JSON surface -----------------------------------------

    #[test]
    fn optional_json_fields_are_omitted_when_none() {
        let json = DiagnosticJson {
            code: DiagCode::XrefRepaired.name().to_string(),
            message: "xref rebuilt".to_string(),
            severity: Severity::Info.to_string(),
            page_index: None,
            location: None,
            hint: None,
        };
        let serialized = serde_json::to_string(&json).unwrap();
        assert_eq!(
            serialized,
            r#"{"code":"XREF_REPAIRED","message":"xref rebuilt","severity":"info"}"#
        );
        let value: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        for absent in ["page_index", "location", "hint"] {
            assert!(value.get(absent).is_none(), "{absent} must be omitted");
        }
    }

    #[test]
    fn json_field_names_are_stable_with_all_fields_populated() {
        let json = DiagnosticJson {
            code: DiagCode::StreamBomb.name().to_string(),
            message: "decompress limit exceeded".to_string(),
            severity: Severity::Error.to_string(),
            page_index: Some(3),
            location: Some(ObjectLocationJson {
                object_number: 7,
                generation_number: 0,
            }),
            hint: Some("Increase --max-decompress-gb if the PDF is trusted".to_string()),
        };
        // Field order and names are the public contract; this exact string is
        // what JSON/NDJSON consumers see.
        assert_eq!(
            serde_json::to_string(&json).unwrap(),
            concat!(
                r#"{"code":"STREAM_BOMB","message":"decompress limit exceeded","#,
                r#""severity":"error","page_index":3,"#,
                r#""location":{"object_number":7,"generation_number":0},"#,
                r#""hint":"Increase --max-decompress-gb if the PDF is trusted"}"#
            )
        );
    }

    #[test]
    fn typed_model_maps_onto_canonical_json_fields() {
        // The typed Diagnostic carries code/message/page_index/object_ref;
        // severity derives from the code. This pins the field-by-field
        // mapping the JSON writer performs (hint is supplied by the caller —
        // it is catalog data, not derivable from the typed model alone).
        for (code, severity) in [
            (DiagCode::XrefRepaired, Severity::Info),
            (DiagCode::StructInvalidName, Severity::Warning),
            (DiagCode::StreamBomb, Severity::Error),
            (DiagCode::EncryptionUnsupported, Severity::Fatal),
        ] {
            let typed = sample(code);
            let json = DiagnosticJson {
                code: typed.code.name().to_string(),
                message: typed.message.as_ref().to_string(),
                severity: typed.severity().to_string(),
                page_index: typed.page_index.map(|p| p as usize),
                location: typed.object_ref.map(|r| ObjectLocationJson {
                    object_number: r.object,
                    generation_number: r.generation,
                }),
                hint: Some("catalog hint".to_string()),
            };
            assert_eq!(json.code, code.name());
            assert_eq!(json.severity, severity.to_string());
            assert_eq!(json.page_index, Some(3));
            assert_eq!(
                json.location,
                Some(ObjectLocationJson {
                    object_number: 7,
                    generation_number: 0,
                })
            );
            // The typed and JSON models agree with the legacy surface on the
            // message bytes.
            assert_eq!(json.message, to_legacy_string(&typed));
        }
    }
}
