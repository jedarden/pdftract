//! Keep the diagnostic-code documentation synchronized with `pdftract-core`.
//!
//! This is intentionally an integration test rather than a unit test in
//! `diagnostics.rs`: the test reads the production source and the published
//! catalog, so a new emission site or documentation row cannot silently drift.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use pdftract_core::diagnostics::{DiagCode, DiagInfo, Diagnostic, ObjRef, DIAGNOSTIC_CATALOG};
use pdftract_core::diagnostics_compat::to_legacy_string;
use pdftract_core::schema::DiagnosticJson;

const SRC_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
const DIAGNOSTICS_SRC: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/diagnostics.rs");
const DOC_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/integrations/diagnostics-codes.md"
);
const MANIFEST_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");

struct Variant {
    ident: String,
    cfg_feature: Option<String>,
}

struct DocRow {
    code: String,
    severity: String,
    line_no: usize,
    feature: Option<String>,
    reserved: bool,
}

struct EmissionSite {
    ident: String,
    path: PathBuf,
    line_no: usize,
}

fn feature_enabled(feature: &str) -> Option<bool> {
    match feature {
        "cjk" => Some(cfg!(feature = "cjk")),
        _ => None,
    }
}

fn declared_features() -> BTreeSet<String> {
    let manifest = fs::read_to_string(MANIFEST_PATH).expect("cannot read pdftract-core/Cargo.toml");
    let mut features = BTreeSet::new();
    let mut in_features = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_features = trimmed == "[features]";
            continue;
        }
        if in_features {
            if let Some((name, _)) = trimmed.split_once('=') {
                let name = name.trim();
                if !name.is_empty() && !name.starts_with('#') {
                    features.insert(name.to_string());
                }
            }
        }
    }
    features
}

/// Extract enum variants and directly attached `#[cfg(feature = "...")]`
/// attributes. This gives the source scanner enough information to ignore a
/// valid emission that is compiled out for the current feature set.
fn parse_enum_variants() -> Vec<Variant> {
    let source = fs::read_to_string(DIAGNOSTICS_SRC).expect("cannot read diagnostics.rs");
    let marker = "pub enum DiagCode {";
    let start = source
        .find(marker)
        .expect("pub enum DiagCode { not found in diagnostics.rs")
        + marker.len();

    let mut depth = 1usize;
    let mut end = None;
    for (offset, byte) in source.bytes().enumerate().skip(start) {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(offset);
                    break;
                }
            }
            _ => {}
        }
    }
    let block = &source[start..end.expect("unbalanced braces in DiagCode enum")];

    let mut variants = Vec::new();
    let mut cfg_feature = None;
    for line in block.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("///") {
            continue;
        }
        if trimmed.starts_with("#[") {
            cfg_feature = trimmed
                .strip_prefix("#[cfg(feature = ")
                .and_then(|rest| rest.strip_suffix(")]"))
                .and_then(|feature| feature.strip_prefix('"'))
                .and_then(|feature| feature.strip_suffix('"'))
                .map(str::to_string);
            continue;
        }

        let ident = trimmed.trim_end_matches(',');
        if trimmed.ends_with(',')
            && !ident.is_empty()
            && ident
                .chars()
                .next()
                .is_some_and(|character| character.is_ascii_alphabetic())
            && ident
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '_')
        {
            variants.push(Variant {
                ident: ident.to_string(),
                cfg_feature: cfg_feature.take(),
            });
        }
    }
    assert!(
        variants.len() >= 100,
        "parsed only {} DiagCode variants; enum parsing is broken",
        variants.len()
    );
    variants
}

fn parse_doc_rows() -> Vec<DocRow> {
    let document = fs::read_to_string(DOC_PATH).expect("cannot read diagnostics-codes.md");
    let mut rows = Vec::new();

    for (index, line) in document.lines().enumerate() {
        let Some(rest) = line.strip_prefix("| `") else {
            continue;
        };
        let Some((code, after_code)) = rest.split_once('`') else {
            continue;
        };
        if code.is_empty()
            || !code.chars().all(|character| {
                character.is_ascii_uppercase() || character == '_' || character.is_ascii_digit()
            })
        {
            continue;
        }

        let cells: Vec<&str> = after_code.split('|').collect();
        if cells.len() < 4 {
            continue;
        }
        let severity = cells[1].trim();
        let description = cells[2].trim();
        if severity.is_empty() || description.is_empty() {
            continue;
        }

        let feature = description
            .split_once("requires `")
            .and_then(|(_, rest)| rest.split('`').next())
            .map(str::to_string);
        rows.push(DocRow {
            code: code.to_string(),
            severity: severity.to_string(),
            line_no: index + 1,
            feature,
            reserved: description.contains("(reserved"),
        });
    }

    assert!(
        rows.len() >= 100,
        "parsed only {} catalog rows; diagnostics-codes.md parsing is broken",
        rows.len()
    );
    rows
}

fn doc_rows_by_code() -> BTreeMap<String, DocRow> {
    let mut rows_by_code = BTreeMap::new();
    for row in parse_doc_rows() {
        if let Some(previous) = rows_by_code.insert(row.code.clone(), row) {
            let current_line = rows_by_code
                .get(&previous.code)
                .map(|row| row.line_no)
                .expect("inserted duplicate row is present");
            panic!(
                "diagnostics-codes.md documents {} twice (lines {} and {})",
                previous.code, previous.line_no, current_line
            );
        }
    }
    rows_by_code
}

fn debug_ident(code: DiagCode) -> String {
    format!("{code:?}")
        .rsplit("::")
        .next()
        .expect("DiagCode Debug output is not empty")
        .to_string()
}

fn catalog_by_ident() -> BTreeMap<String, &'static DiagInfo> {
    let mut entries = BTreeMap::new();
    for info in DIAGNOSTIC_CATALOG {
        let ident = debug_ident(info.code);
        assert!(
            entries.insert(ident.clone(), info).is_none(),
            "duplicate DIAGNOSTIC_CATALOG entry for {ident}"
        );
    }
    entries
}

fn variant_features() -> BTreeMap<String, Option<String>> {
    parse_enum_variants()
        .into_iter()
        .map(|variant| (variant.ident, variant.cfg_feature))
        .collect()
}

fn feature_is_disabled(features: &BTreeMap<String, Option<String>>, ident: &str) -> bool {
    features
        .get(ident)
        .and_then(|feature| feature.as_deref())
        .and_then(feature_enabled)
        == Some(false)
}

fn source_files(path: &Path, files: &mut Vec<PathBuf>) {
    for entry in
        fs::read_dir(path).unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
    {
        let entry =
            entry.unwrap_or_else(|error| panic!("cannot read source directory entry: {error}"));
        let path = entry.path();
        if path.is_dir() {
            source_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs")
            && path != Path::new(DIAGNOSTICS_SRC)
            // This legacy module defines a separate, unused `parser::DiagCode`.
            && path != Path::new(SRC_DIR).join("parser/diagnostic.rs")
        {
            files.push(path);
        }
    }
}

/// Replace comments and string literals with spaces while preserving newlines.
/// The scanner only needs to recognize `DiagCode::Variant` and
/// `emit!(..., Variant)`; masking non-code text prevents examples and prose
/// from looking like emissions.
fn mask_non_code(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut masked = bytes.to_vec();
    let mut index = 0;
    let mut block_comment_depth = 0usize;

    while index < bytes.len() {
        if block_comment_depth > 0 {
            if bytes[index..].starts_with(b"/*") {
                masked[index] = b' ';
                masked[index + 1] = b' ';
                block_comment_depth += 1;
                index += 2;
            } else if bytes[index..].starts_with(b"*/") {
                masked[index] = b' ';
                masked[index + 1] = b' ';
                block_comment_depth -= 1;
                index += 2;
            } else {
                if bytes[index] != b'\n' {
                    masked[index] = b' ';
                }
                index += 1;
            }
            continue;
        }

        if bytes[index..].starts_with(b"//") {
            masked[index] = b' ';
            masked[index + 1] = b' ';
            index += 2;
            while index < bytes.len() && bytes[index] != b'\n' {
                masked[index] = b' ';
                index += 1;
            }
            continue;
        }
        if bytes[index..].starts_with(b"/*") {
            masked[index] = b' ';
            masked[index + 1] = b' ';
            block_comment_depth = 1;
            index += 2;
            continue;
        }

        if bytes[index] == b'r' {
            let mut quote = index + 1;
            while quote < bytes.len() && bytes[quote] == b'#' {
                quote += 1;
            }
            if quote < bytes.len() && bytes[quote] == b'"' {
                let hashes = quote - index - 1;
                let close = format!("\"{}", "#".repeat(hashes));
                let content_start = quote + 1;
                let content_end = source[content_start..]
                    .find(&close)
                    .map(|offset| content_start + offset + close.len())
                    .unwrap_or(bytes.len());
                for byte in &mut masked[index..content_end] {
                    if *byte != b'\n' {
                        *byte = b' ';
                    }
                }
                index = content_end;
                continue;
            }
        }

        if bytes[index] == b'"' {
            masked[index] = b' ';
            index += 1;
            while index < bytes.len() {
                let escaped = bytes[index - 1] == b'\\';
                let closing = bytes[index] == b'"' && !escaped;
                if bytes[index] != b'\n' {
                    masked[index] = b' ';
                }
                index += 1;
                if closing {
                    break;
                }
            }
            continue;
        }

        index += 1;
    }

    String::from_utf8(masked).expect("Rust source is UTF-8")
}

fn identifier_at(source: &str, start: usize) -> Option<(String, usize)> {
    let bytes = source.as_bytes();
    if start >= bytes.len() || !(bytes[start].is_ascii_alphabetic() || bytes[start] == b'_') {
        return None;
    }
    let mut end = start + 1;
    while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
        end += 1;
    }
    Some((source[start..end].to_string(), end))
}

fn line_number(source: &str, byte_offset: usize) -> usize {
    source[..byte_offset]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
}

fn collect_direct_codes(source: &str, path: &Path, sites: &mut Vec<EmissionSite>) {
    let marker = "DiagCode::";
    let mut cursor = 0;
    while let Some(relative) = source[cursor..].find(marker) {
        let marker_start = cursor + relative;
        let ident_start = marker_start + marker.len();
        if let Some((ident, _)) = identifier_at(source, ident_start) {
            sites.push(EmissionSite {
                ident,
                path: path.to_path_buf(),
                line_no: line_number(source, marker_start),
            });
        }
        cursor = ident_start;
    }
}

fn collect_macro_codes(source: &str, path: &Path, sites: &mut Vec<EmissionSite>) {
    let marker = "emit!";
    let mut cursor = 0;
    while let Some(relative) = source[cursor..].find(marker) {
        let marker_start = cursor + relative;
        let mut open = marker_start + marker.len();
        while source
            .as_bytes()
            .get(open)
            .is_some_and(|byte| byte.is_ascii_whitespace())
        {
            open += 1;
        }
        if source.as_bytes().get(open) != Some(&b'(') {
            cursor = marker_start + marker.len();
            continue;
        }

        let mut depth = 0usize;
        let mut comma = None;
        for (offset, byte) in source.as_bytes().iter().enumerate().skip(open + 1) {
            match byte {
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' => depth = depth.saturating_sub(1),
                b',' if depth == 0 => {
                    comma = Some(offset);
                    break;
                }
                _ => {}
            }
        }
        if let Some(comma) = comma {
            let mut ident_start = comma + 1;
            while source
                .as_bytes()
                .get(ident_start)
                .is_some_and(|byte| byte.is_ascii_whitespace())
            {
                ident_start += 1;
            }
            if let Some((ident, _)) = identifier_at(source, ident_start) {
                sites.push(EmissionSite {
                    ident,
                    path: path.to_path_buf(),
                    line_no: line_number(source, marker_start),
                });
            }
        }
        cursor = marker_start + marker.len();
    }
}

fn emission_sites() -> Vec<EmissionSite> {
    let mut files = Vec::new();
    source_files(Path::new(SRC_DIR), &mut files);
    files.sort();

    let mut sites = Vec::new();
    for path in files {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let source = mask_non_code(&source);
        collect_direct_codes(&source, &path, &mut sites);
        collect_macro_codes(&source, &path, &mut sites);
    }
    sites
}

#[test]
fn catalog_covers_every_compiled_enum_variant() {
    let variants = parse_enum_variants();
    let features: BTreeMap<String, Option<String>> = variants
        .iter()
        .map(|variant| (variant.ident.clone(), variant.cfg_feature.clone()))
        .collect();
    let catalog = catalog_by_ident();
    let declared = declared_features();
    let mut problems = String::new();

    for variant in &variants {
        if let Some(feature) = &variant.cfg_feature {
            if !declared.contains(feature) {
                let _ = writeln!(
                    problems,
                    "DiagCode::{} uses undeclared cargo feature `{feature}`",
                    variant.ident
                );
            }
        }
        if !feature_is_disabled(&features, &variant.ident) && !catalog.contains_key(&variant.ident)
        {
            let _ = writeln!(
                problems,
                "DiagCode::{} has no DIAGNOSTIC_CATALOG entry",
                variant.ident
            );
        }
    }
    for ident in catalog.keys() {
        if !features.contains_key(ident) {
            let _ = writeln!(
                problems,
                "DIAGNOSTIC_CATALOG contains {ident}, but DiagCode has no such variant"
            );
        }
    }
    for (ident, info) in &catalog {
        let actual_category = info.code.category();
        if info.category != actual_category {
            let _ = writeln!(
                problems,
                "DIAGNOSTIC_CATALOG entry for {ident} says category {:?}, but DiagCode::category() is {:?}",
                info.category,
                actual_category
            );
        }
        let actual = info.code.severity();
        if info.severity != actual {
            let _ = writeln!(
                problems,
                "DIAGNOSTIC_CATALOG entry for {ident} says severity {:?}, but DiagCode::severity() is {:?}",
                info.severity,
                actual
            );
        }
    }
    assert!(problems.is_empty(), "enum to catalog drift:\n{problems}");
}

#[test]
fn category_severity_classification_is_stable() {
    // Categories intentionally contain more than one severity in a few cases
    // (for example STRUCT has both recoverable warnings and informational
    // fallback notices). Keep the allowed severity set explicit so adding a
    // code cannot silently broaden a category's contract.
    let mut expected: BTreeMap<&str, BTreeSet<String>> = [
        ("STRUCT", &["info", "warning"][..]),
        ("XREF", &["info", "warning"][..]),
        ("STREAM", &["error", "warning"][..]),
        ("ENCRYPTION", &["fatal"][..]),
        ("PAGE", &["error", "warning"][..]),
        ("FONT", &["warning"][..]),
        ("OCR", &["warning"][..]),
        ("IMG", &["warning"][..]),
        ("REMOTE", &["error", "fatal", "warning"][..]),
        ("GSTATE", &["warning"][..]),
        ("LAYOUT", &["info", "warning"][..]),
        ("MCP", &["error"][..]),
        ("CACHE", &["warning"][..]),
        ("MARKED_CONTENT", &["info"][..]),
        ("INLINE_IMAGE", &["warning"][..]),
        ("PROFILE", &["error"][..]),
        ("REPAIR", &["info"][..]),
        ("SECURITY", &["info"][..]),
    ]
    .into_iter()
    .map(|(category, severities)| {
        (
            category,
            severities
                .iter()
                .copied()
                .map(str::to_owned)
                .collect::<BTreeSet<_>>(),
        )
    })
    .collect();
    if cfg!(feature = "cjk") {
        expected.insert("CJK", ["warning".to_owned()].into_iter().collect());
    }

    let mut observed: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    for info in DIAGNOSTIC_CATALOG {
        observed
            .entry(info.category)
            .or_default()
            .insert(info.severity.to_string());
    }

    // A catalog category must have a policy entry and exactly the expected
    // severity set. The all-features run also covers the CJK category.
    assert_eq!(observed, expected);

    // Keep this test tied to the public classification methods as well as the
    // catalog copy used for documentation and CLI output.
    for info in DIAGNOSTIC_CATALOG {
        assert_eq!(info.code.category(), info.category);
        assert_eq!(info.code.severity(), info.severity);
    }

    // The declaration-order ALL list and the catalog must cover the same
    // compiled code set: `catalog_covers_every_compiled_enum_variant`
    // re-derives the catalog side from source, this ties `DiagCode::ALL`
    // to it directly so the two cannot drift apart.
    assert_eq!(
        DiagCode::ALL.len(),
        DIAGNOSTIC_CATALOG.len(),
        "DiagCode::ALL and DIAGNOSTIC_CATALOG cover different code sets"
    );
}

#[test]
fn actionable_catalog_codes_have_structured_hints() {
    for info in DIAGNOSTIC_CATALOG {
        assert!(
            !info.suggested_action.trim().is_empty(),
            "{} has an empty catalog action",
            info.code.name()
        );

        let diagnostic = Diagnostic::with_static_no_offset(info.code, "test diagnostic");
        let structured = DiagnosticJson::from(&diagnostic);
        assert_eq!(
            structured.hint.as_deref(),
            Some(info.suggested_action),
            "{} lost its catalog hint during JSON conversion",
            info.code.name()
        );
    }
}

#[test]
fn every_documented_code_round_trips_its_published_contract() {
    let catalog = catalog_by_ident();

    for row in doc_rows_by_code().values() {
        if row.feature.as_deref().and_then(feature_enabled) == Some(false) {
            continue;
        }

        let info = catalog
            .values()
            .find(|info| info.code.name() == row.code)
            .unwrap_or_else(|| panic!("{} has no catalog entry", row.code));
        let expected_severity = row.severity.to_ascii_lowercase();
        assert_eq!(
            info.code.severity().to_string(),
            expected_severity,
            "{}: documented severity must match the typed code",
            row.code
        );

        // First verify the documented document-level shape: code, message,
        // severity, and catalog hint are retained, while page/location are
        // omitted when the emission site has no such context.
        let message = format!("documented message for {}", row.code);
        let document_level = Diagnostic::with_dynamic_no_offset(info.code, message.clone());
        let structured = DiagnosticJson::from(&document_level);
        assert_eq!(structured.code, row.code, "{} code round-trip", row.code);
        assert_eq!(
            structured.message, message,
            "{} message round-trip",
            row.code
        );
        assert_eq!(
            structured.severity, expected_severity,
            "{} severity",
            row.code
        );
        assert_eq!(
            structured.hint.as_deref(),
            Some(info.suggested_action),
            "{} hint policy must come from DIAGNOSTIC_CATALOG",
            row.code
        );
        assert_eq!(structured.page_index, None, "{} page policy", row.code);
        assert_eq!(structured.location, None, "{} location policy", row.code);

        let wire = serde_json::to_value(&structured)
            .unwrap_or_else(|error| panic!("{} does not serialize: {error}", row.code));
        let decoded: DiagnosticJson = serde_json::from_value(wire.clone())
            .unwrap_or_else(|error| panic!("{} does not deserialize: {error}", row.code));
        assert_eq!(decoded, structured, "{} JSON round-trip", row.code);
        assert!(
            wire.get("page_index").is_none(),
            "{} page_index must be omitted",
            row.code
        );
        assert!(
            wire.get("location").is_none(),
            "{} location must be omitted",
            row.code
        );

        // Then verify that the same documented code retains both optional
        // location fields when the typed emission supplies them. The legacy
        // adapter remains message-only in either case.
        let contextual = document_level
            .with_object_ref(ObjRef::new(12, 3))
            .with_page_index(7);
        let contextual_json = DiagnosticJson::from(&contextual);
        assert_eq!(
            contextual_json.page_index,
            Some(7),
            "{} page round-trip",
            row.code
        );
        assert_eq!(
            contextual_json
                .location
                .as_ref()
                .map(|location| { (location.object_number, location.generation_number) }),
            Some((12, 3)),
            "{} object location round-trip",
            row.code
        );
        assert_eq!(
            to_legacy_string(&contextual),
            message,
            "{} legacy compatibility must preserve only the message",
            row.code
        );
    }
}

#[test]
fn emission_sites_are_documented_with_matching_severity() {
    let catalog = catalog_by_ident();
    let features = variant_features();
    let docs = doc_rows_by_code();
    let mut problems = String::new();

    for site in emission_sites() {
        if feature_is_disabled(&features, &site.ident) {
            continue;
        }
        let Some(info) = catalog.get(&site.ident) else {
            let _ = writeln!(
                problems,
                "{}:{} emits DiagCode::{} but it has no DIAGNOSTIC_CATALOG entry",
                site.path.display(),
                site.line_no,
                site.ident
            );
            continue;
        };

        let code = info.code.name();
        let Some(row) = docs.get(code) else {
            let _ = writeln!(
                problems,
                "{}:{} emits {code} but diagnostics-codes.md has no catalog row",
                site.path.display(),
                site.line_no
            );
            continue;
        };
        let documented = row.severity.to_ascii_lowercase();
        let actual = info.code.severity().to_string();
        if documented != actual {
            let _ = writeln!(
                problems,
                "{code}: docs line {} says severity {:?}, but the emission code is {:?}",
                row.line_no, row.severity, actual
            );
        }
    }

    assert!(
        problems.is_empty(),
        "emission-site diagnostic drift:\n{problems}"
    );
}

#[test]
fn documented_codes_are_emitted_or_explicitly_reserved() {
    let catalog = catalog_by_ident();
    let features = variant_features();
    let catalog_codes: BTreeSet<String> = catalog
        .values()
        .map(|info| info.code.name().to_string())
        .collect();
    let source_idents: BTreeSet<String> = emission_sites()
        .into_iter()
        .filter(|site| !feature_is_disabled(&features, &site.ident))
        .map(|site| site.ident)
        .collect();
    let source_codes: BTreeSet<String> = source_idents
        .iter()
        .filter_map(|ident| catalog.get(ident).map(|info| info.code.name().to_string()))
        .collect();
    let mut problems = String::new();

    for row in doc_rows_by_code().values() {
        if let Some(feature) = &row.feature {
            if feature_enabled(feature).is_none() {
                let _ = writeln!(
                    problems,
                    "{} (doc line {}): unknown feature `{feature}`",
                    row.code, row.line_no
                );
            }
        }
        if row.reserved {
            continue;
        }
        if row.feature.as_deref().and_then(feature_enabled) == Some(false) {
            continue;
        }
        if !catalog_codes.contains(&row.code) {
            let _ = writeln!(
                problems,
                "{} (doc line {}) is not present in DIAGNOSTIC_CATALOG",
                row.code, row.line_no
            );
            continue;
        }
        if !source_codes.contains(&row.code) {
            let _ = writeln!(
                problems,
                "{} (doc line {}) is not emitted by pdftract-core; mark the row `(reserved)` or add an emission site",
                row.code, row.line_no
            );
        }
    }

    // If a source token did not map to the catalog, the forward test reports it.
    // Keep this explicit so the reverse assertion cannot accidentally pass by
    // dropping an unknown source identifier from `source_codes`.
    for ident in source_idents {
        if !catalog.contains_key(&ident) && !feature_is_disabled(&features, &ident) {
            let _ = writeln!(
                problems,
                "source emits DiagCode::{ident}, which is not in DIAGNOSTIC_CATALOG"
            );
        }
    }

    assert!(
        problems.is_empty(),
        "documented diagnostic drift:\n{problems}"
    );
}
