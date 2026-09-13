//! Test helper for accessing the diagnostics array in parsed pdftract JSON output.
//!
//! Extraction output carries diagnostics in up to three shapes depending on
//! which writer produced the JSON:
//!
//! 1. **`errors`** — the top-level array of `DiagnosticJson` objects in the
//!    full v1.0 output schema (`schema_version`, `metadata`, `pages`,
//!    `errors`, ...). Each entry is an object with a `code` field. This is
//!    the canonical, searchable form; `output::unmapped::scan_value` reads
//!    the same field.
//! 2. **`metadata.diagnostics`** — the string array serialized from
//!    `ExtractionMetadata::diagnostics`. Entries are bare message strings
//!    (`extract.rs` pushes `Diagnostic::message` without a code prefix), and
//!    the field is `skip_serializing_if = "Vec::is_empty"`, so it is absent
//!    from clean extractions.
//! 3. **`diagnostics`** — the string array on NDJSON page frames
//!    (`PageFrame::diagnostics`) or a direct dump of `ExtractionMetadata`.
//!
//! [`diagnostics`] returns every entry unified into [`Diagnostic`], searching
//! the locations above in order and using the first one that is present.
//! A missing or `null` field is not an error — it yields an empty array,
//! matching the documented zero-diagnostic capture shape (a schema-1.0
//! document with no `errors` key at all; see `unmapped.rs` tests). Only
//! malformed data (a non-array field, a non-object `errors` entry, a missing
//! or non-string `code`) produces [`DiagnosticsError`].
//!
//! # Example
//!
//! ```rust
//! # // Integration-test module: doc examples are never compiled or run.
//! use test_helpers::diagnostics::{contains_code, diagnostics, find_by_code};
//!
//! let parsed: serde_json::Value = serde_json::from_str(&captured_stdout)?;
//! let diags = diagnostics(&parsed)?;
//!
//! assert!(contains_code(&diags, "FONT_GLYPH_UNMAPPED"));
//! for entry in find_by_code(&diags, "FONT_GLYPH_UNMAPPED") {
//!     eprintln!("page {}: {}", entry.page_index.unwrap_or(0), entry.message);
//! }
//! ```

use serde_json::Value;

/// A single diagnostic entry, unified across the object and string forms.
///
/// Object-form entries populate `code` from the entry's `code` field and the
/// optional fields when present. String-form entries (`metadata.diagnostics`,
/// NDJSON `diagnostics`) carry only a message, so `code` holds the **entire
/// string** — those entries have no separate code at source, and splitting on
/// the first `:` would mangle bare messages that merely contain a colon
/// (e.g. `... (font ID: CustomNoMap)`). Use [`any_code_contains`] to search
/// string-form entries by substring.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Diagnostic code (object form) or the full message string (string form).
    pub code: String,
    /// Human-readable message, when the entry carries one separately.
    pub message: Option<String>,
    /// Severity (`"info"` / `"warning"` / `"error"` / `"fatal"`), when present.
    pub severity: Option<String>,
    /// 0-based page index, `None` for document-level diagnostics.
    pub page_index: Option<usize>,
}

/// Error returned when the JSON is structurally malformed as a diagnostics
/// carrier — never merely because the diagnostics field is missing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticsError {
    /// The root of the parsed JSON is not an object, so no field can be read.
    RootNotAnObject {
        /// JSON type that was found instead (e.g. `"string"`, `"array"`).
        found: String,
    },
    /// A diagnostics field is present but is not an array.
    FieldNotAnArray {
        /// Path of the field, e.g. `"errors"` or `"metadata.diagnostics"`.
        path: String,
        /// JSON type that was found instead.
        found: String,
    },
    /// An `errors` entry is not an object.
    EntryNotAnObject {
        /// Index of the offending entry within the array.
        index: usize,
    },
    /// A string-form diagnostics array contains a non-string entry.
    EntryNotAString {
        /// Path of the field, e.g. `"metadata.diagnostics"`.
        path: String,
        /// Index of the offending entry within the array.
        index: usize,
    },
    /// An `errors` entry object has no `code` field.
    CodeMissing {
        /// Index of the offending entry within the array.
        index: usize,
    },
    /// An `errors` entry's `code` field is not a string.
    CodeNotAString {
        /// Index of the offending entry within the array.
        index: usize,
        /// JSON type that was found instead.
        found: String,
    },
}

impl std::fmt::Display for DiagnosticsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiagnosticsError::RootNotAnObject { found } => {
                write!(
                    f,
                    "expected a JSON object at the root of the parsed output, found {found}"
                )
            }
            DiagnosticsError::FieldNotAnArray { path, found } => {
                write!(
                    f,
                    "diagnostics field `{path}` is not an array (found {found})"
                )
            }
            DiagnosticsError::EntryNotAnObject { index } => {
                write!(f, "`errors` entry at index {index} is not an object")
            }
            DiagnosticsError::EntryNotAString { path, index } => {
                write!(f, "`{path}` entry at index {index} is not a string")
            }
            DiagnosticsError::CodeMissing { index } => {
                write!(f, "`errors` entry at index {index} has no `code` field")
            }
            DiagnosticsError::CodeNotAString { index, found } => {
                write!(
                    f,
                    "`errors` entry at index {index} has a `code` that is not a string (found {found})"
                )
            }
        }
    }
}

impl std::error::Error for DiagnosticsError {}

/// Extract the diagnostics array from parsed pdftract JSON output.
///
/// The first present location wins, in this order: `errors`,
/// `metadata.diagnostics`, `diagnostics`. A field that is missing or `null`
/// is skipped; when no location is present the result is an empty `Vec` —
/// the documented shape of a zero-diagnostic capture, not an error.
///
/// # Errors
///
/// [`DiagnosticsError`] for malformed carriers: a root that is not an
/// object, a present-but-non-array diagnostics field, a non-object `errors`
/// entry, a non-string entry in a string-form array, or an `errors` entry
/// whose `code` is missing or not a string. Optional object fields
/// (`message`, `severity`, `page_index`) follow the tolerance policy of
/// `output::unmapped`: missing, `null`, or wrongly-typed fields stay `None`
/// and never malform the entry.
pub fn diagnostics(value: &Value) -> Result<Vec<Diagnostic>, DiagnosticsError> {
    let root = value
        .as_object()
        .ok_or_else(|| DiagnosticsError::RootNotAnObject {
            found: json_type_name(value).to_owned(),
        })?;

    match root.get("errors") {
        None | Some(Value::Null) => {}
        Some(Value::Array(entries)) => return object_entries(entries),
        Some(other) => {
            return Err(DiagnosticsError::FieldNotAnArray {
                path: "errors".to_owned(),
                found: json_type_name(other).to_owned(),
            })
        }
    }

    if let Some(metadata) = root.get("metadata") {
        if !metadata.is_null() {
            return match metadata.get("diagnostics") {
                None | Some(Value::Null) => Ok(Vec::new()),
                Some(Value::Array(entries)) => string_entries(entries, "metadata.diagnostics"),
                Some(other) => Err(DiagnosticsError::FieldNotAnArray {
                    path: "metadata.diagnostics".to_owned(),
                    found: json_type_name(other).to_owned(),
                }),
            };
        }
    }

    match root.get("diagnostics") {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(entries)) => string_entries(entries, "diagnostics"),
        Some(other) => Err(DiagnosticsError::FieldNotAnArray {
            path: "diagnostics".to_owned(),
            found: json_type_name(other).to_owned(),
        }),
    }
}

/// Parse JSON text and extract its diagnostics array in one step.
///
/// Convenience wrapper for tests holding a captured stdout blob.
///
/// # Errors
///
/// `serde_json::Error` if the text is not valid JSON; otherwise the same
/// [`DiagnosticsError`] variants as [`diagnostics`].
pub fn diagnostics_from_str(
    json_text: &str,
) -> Result<Vec<Diagnostic>, Box<dyn std::error::Error>> {
    let value: Value = serde_json::from_str(json_text)?;
    Ok(diagnostics(&value)?)
}

/// Iterate over every entry whose `code` equals `code` exactly.
///
/// For string-form entries the "code" is the whole message string, so exact
/// matching only hits object-form entries; use [`any_code_contains`] for
/// substring searches over string-form entries.
pub fn find_by_code<'a>(
    diagnostics: &'a [Diagnostic],
    code: &'a str,
) -> impl Iterator<Item = &'a Diagnostic> + 'a {
    diagnostics.iter().filter(move |d| d.code == code)
}

/// Whether any entry's `code` equals `code` exactly.
pub fn contains_code(diagnostics: &[Diagnostic], code: &str) -> bool {
    find_by_code(diagnostics, code).next().is_some()
}

/// Number of entries whose `code` equals `code` exactly.
pub fn count_by_code(diagnostics: &[Diagnostic], code: &str) -> usize {
    find_by_code(diagnostics, code).count()
}

/// Whether any entry's `code` contains `needle` (substring, case-sensitive).
///
/// This is the search that reaches string-form entries: their `code` is the
/// full message, so a diagnostic emitted as
/// `"Character code 01 could not be resolved to Unicode ..."` is found with
/// `any_code_contains(&diags, "could not be resolved to Unicode")`.
pub fn any_code_contains(diagnostics: &[Diagnostic], needle: &str) -> bool {
    diagnostics.iter().any(|d| d.code.contains(needle))
}

/// Collect every entry's `code` in order — handy for failure messages.
pub fn codes<'a>(diagnostics: &'a [Diagnostic]) -> Vec<&'a str> {
    diagnostics.iter().map(|d| d.code.as_str()).collect()
}

/// Convert `errors` array entries (object form) into [`Diagnostic`]s.
fn object_entries(entries: &[Value]) -> Result<Vec<Diagnostic>, DiagnosticsError> {
    entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let obj = entry
                .as_object()
                .ok_or(DiagnosticsError::EntryNotAnObject { index })?;

            let code = match obj.get("code") {
                Some(Value::String(s)) => s.as_str(),
                Some(Value::Null) | None => return Err(DiagnosticsError::CodeMissing { index }),
                Some(other) => {
                    return Err(DiagnosticsError::CodeNotAString {
                        index,
                        found: json_type_name(other).to_owned(),
                    })
                }
            };

            Ok(Diagnostic {
                code: code.to_owned(),
                message: optional_string(obj.get("message")),
                severity: optional_string(obj.get("severity")),
                page_index: obj
                    .get("page_index")
                    .and_then(Value::as_u64)
                    .and_then(|v| usize::try_from(v).ok()),
            })
        })
        .collect()
}

/// Convert string-form array entries into [`Diagnostic`]s.
fn string_entries(entries: &[Value], path: &str) -> Result<Vec<Diagnostic>, DiagnosticsError> {
    entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let s = entry
                .as_str()
                .ok_or(DiagnosticsError::EntryNotAString {
                    path: path.to_owned(),
                    index,
                })?
                .trim();
            Ok(Diagnostic {
                code: s.to_owned(),
                message: None,
                severity: None,
                page_index: None,
            })
        })
        .collect()
}

/// Read an optional string field, treating missing, `null`, and non-string
/// values as `None` (the `output::unmapped` tolerance policy).
fn optional_string(value: Option<&Value>) -> Option<String> {
    value.and_then(Value::as_str).map(str::to_owned)
}

/// Human-readable JSON type name for error messages.
fn json_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}
