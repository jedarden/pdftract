//! Keep the diagnostic-code documentation synchronized with `pdftract-core`.
//!
//! This is intentionally an integration test rather than a unit test in
//! `diagnostics.rs`: the test reads the production source and the published
//! catalog, so a new emission site or documentation row cannot silently drift.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use pdftract_core::diagnostics::{DiagCode, DiagInfo, DIAGNOSTIC_CATALOG};

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
