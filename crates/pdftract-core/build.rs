use std::env;
use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=build/std14-metrics.json");
    println!("cargo:rerun-if-changed=build/named-encodings.json");
    println!("cargo:rerun-if-changed=build/agl.json");
    println!("cargo:rerun-if-changed=build/font-fingerprints.json");
    println!("cargo:rerun-if-changed=build/predefined-cmaps/");

    let out_dir = env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir);
    let metrics_path = Path::new("build/std14-metrics.json");

    // Generate std14 metrics
    generate_std14_metrics(out_path, metrics_path);

    // Generate named encoding tables
    let encodings_path = Path::new("build/named-encodings.json");
    generate_named_encodings(out_path, encodings_path);

    // Generate AGL phf maps
    let agl_path = Path::new("build/agl.json");
    generate_agl_maps(out_path, agl_path);

    // Generate font fingerprint phf map
    let fingerprints_path = Path::new("build/font-fingerprints.json");
    generate_font_fingerprints(out_path, fingerprints_path);

    // Generate predefined CMap registry
    generate_predefined_cmaps(out_path);
}

fn generate_std14_metrics(out_dir: &Path, metrics_path: &Path) {
    let json_content = fs::read_to_string(metrics_path).expect("Failed to read std14-metrics.json");

    let data: serde_json::Value =
        serde_json::from_str(&json_content).expect("Failed to parse std14-metrics.json");

    let fonts = data["fonts"].as_object().expect("fonts object missing");

    let mut metrics_structs = String::new();

    for (font_name, font_data) in fonts {
        let font_ident = font_name.replace("-", "_");
        let weights = font_data["weights"]
            .as_array()
            .expect("weights array missing");

        let weights_array: Vec<String> = weights
            .iter()
            .map(|v| v.as_u64().unwrap_or(0).to_string())
            .collect();

        let font_bbox = font_data["font_bbox"]
            .as_array()
            .expect("font_bbox array missing");
        let font_bbox: Vec<String> = font_bbox
            .iter()
            .map(|v| v.as_i64().unwrap_or(0).to_string())
            .collect();

        let ascent = font_data["ascent"].as_i64().expect("ascent missing");
        let descent = font_data["descent"].as_i64().expect("descent missing");
        let italic_angle = font_data["italic_angle"]
            .as_f64()
            .expect("italic_angle missing");
        let cap_height = font_data["cap_height"]
            .as_i64()
            .expect("cap_height missing");
        let stem_v = font_data["stem_v"].as_i64().expect("stem_v missing");

        let encoding_str = font_data["encoding"].as_str().expect("encoding missing");
        let encoding = match encoding_str {
            "StandardEncoding" => "NamedEncoding::Standard",
            "SymbolEncoding" => "NamedEncoding::Symbol",
            "ZapfDingbatsEncoding" => "NamedEncoding::ZapfDingbats",
            _ => "NamedEncoding::Standard",
        };

        metrics_structs.push_str(&format!(
            r#"
static {}_WIDTHS: &[u16; 256] = &[{}];
static {}_METRICS: Std14Metrics = Std14Metrics {{
    widths: &{}_WIDTHS,
    ascent: {},
    descent: {},
    italic_angle: {}f32,
    font_bbox: [{}],
    cap_height: {},
    stem_v: {},
    encoding: {},
}};
"#,
            font_ident.to_uppercase(),
            weights_array.join(", "),
            font_ident.to_uppercase(),
            font_ident.to_uppercase(),
            ascent,
            descent,
            italic_angle,
            font_bbox.join(", "),
            cap_height,
            stem_v,
            encoding
        ));
    }

    // Build the phf map using phf_codegen
    let mut map_builder = phf_codegen::Map::new();

    for font_name in fonts.keys() {
        let ident = font_name.replace("-", "_");
        map_builder.entry(
            font_name.as_str(),
            &format!("&{}_METRICS", ident.to_uppercase()),
        );
    }

    let rust_code = format!(
        r#"
// Auto-generated Standard 14 font metrics.
// Do not edit manually.

{}

pub fn get_std14_metrics(name: &str) -> Option<&'static Std14Metrics> {{
    static METRICS: phf::Map<&'static str, &'static Std14Metrics> = {};
    METRICS.get(name).copied()
}}
"#,
        metrics_structs,
        map_builder.build()
    );

    fs::write(Path::new(out_dir).join("std14_registry.rs"), rust_code)
        .expect("Failed to write std14_registry.rs");
}

fn generate_named_encodings(out_dir: &Path, encodings_path: &Path) {
    let json_content =
        fs::read_to_string(encodings_path).expect("Failed to read named-encodings.json");

    let data: serde_json::Value =
        serde_json::from_str(&json_content).expect("Failed to parse named-encodings.json");

    let encodings = data.as_object().expect("encodings object missing");

    let mut encoding_arrays = String::new();

    for (encoding_name, encoding_data) in encodings {
        let ident = match encoding_name.as_str() {
            "WinAnsiEncoding" => "WIN_ANSI",
            "MacRomanEncoding" => "MAC_ROMAN",
            "MacExpertEncoding" => "MAC_EXPERT",
            "StandardEncoding" => "STANDARD",
            "SymbolEncoding" => "SYMBOL",
            "ZapfDingbatsEncoding" => "ZAPF_DINGBATS",
            _ => continue,
        };

        let entries = encoding_data
            .as_object()
            .expect("encoding data is not an object");

        let mut array_values = Vec::new();
        for i in 0..256 {
            let key = format!("0x{:02X}", i);
            let value = entries.get(&key).and_then(|v| v.as_str());
            let rust_value = match value {
                Some(glyph_name) => format!("Some(\"{}\")", glyph_name),
                None => "None".to_string(),
            };
            array_values.push(rust_value);
        }

        encoding_arrays.push_str(&format!(
            r#"
pub static {}: [Option<&'static str>; 256] = [
{}];
"#,
            ident,
            array_values.join(", ")
        ));
    }

    let rust_code = format!(
        r#"
// Auto-generated named encoding tables.
// Do not edit manually.
// Source: ISO 32000-1 Annex D

{}

pub fn get_named_encoding_table(encoding: NamedEncoding) -> &'static [Option<&'static str>; 256] {{
    match encoding {{
        NamedEncoding::WinAnsi => &WIN_ANSI,
        NamedEncoding::MacRoman => &MAC_ROMAN,
        NamedEncoding::MacExpert => &MAC_EXPERT,
        NamedEncoding::Standard => &STANDARD,
        NamedEncoding::Symbol => &SYMBOL,
        NamedEncoding::ZapfDingbats => &ZAPF_DINGBATS,
    }}
}}
"#,
        encoding_arrays
    );

    fs::write(Path::new(out_dir).join("named_encodings.rs"), rust_code)
        .expect("Failed to write named_encodings.rs");
}

fn generate_agl_maps(out_dir: &Path, agl_path: &Path) {
    let json_content = fs::read_to_string(agl_path).expect("Failed to read agl.json");

    let data: serde_json::Value =
        serde_json::from_str(&json_content).expect("Failed to parse agl.json");

    // Single-codepoint map
    let single = data["merged_single"]
        .as_object()
        .expect("merged_single object missing");

    let mut single_map_builder = phf_codegen::Map::new();

    for (name, uvalue) in single {
        let uvalue_str = uvalue.as_str().expect("unicode value is not a string");
        // Parse the JSON unicode escape like "A" into a Rust char literal
        let unicode_char = decode_json_unicode(uvalue_str);
        single_map_builder.entry(name.as_str(), &format!("'\\u{{{}}}'", unicode_char));
    }

    // Multi-codepoint map
    let multi = data["merged_multi"]
        .as_object()
        .expect("merged_multi object missing");

    let mut multi_arrays = String::new();
    let mut multi_map_builder = phf_codegen::Map::new();

    for (name, uvalues) in multi {
        let uvalues_arr = uvalues.as_array().expect("multi value is not an array");
        let ident = name.to_uppercase().replace("-", "_").replace(".", "_");

        let chars: Vec<String> = uvalues_arr
            .iter()
            .map(|v| {
                let uvalue_str = v.as_str().expect("unicode value is not a string");
                let unicode_char = decode_json_unicode(uvalue_str);
                format!("'\\u{{{}}}'", unicode_char)
            })
            .collect();

        multi_arrays.push_str(&format!(
            r#"
static {}: &[char] = &[{}];
"#,
            ident,
            chars.join(", ")
        ));

        multi_map_builder.entry(name.as_str(), &format!("&{}", ident));
    }

    let rust_code = format!(
        r#"
// Auto-generated Adobe Glyph List (AGL) phf maps.
// Do not edit manually.
// Source: Adobe Glyph List 1.4 + AGLFN 1.7
// https://github.com/adobe-type-tools/agl-aglfn

{}

/// AGL phf map for single-codepoint glyph names.
/// Maps glyph names like "A", "quoteright", "Euro" to their Unicode codepoints.
pub static AGL: phf::Map<&'static str, char> = {};

/// AGL phf map for multi-codepoint (ligature) glyph names.
/// Maps glyph names like "dalethatafpatah" to sequences of Unicode codepoints.
pub static AGL_MULTI: phf::Map<&'static str, &[char]> = {};
"#,
        multi_arrays,
        single_map_builder.build(),
        multi_map_builder.build()
    );

    fs::write(Path::new(out_dir).join("agl.rs"), rust_code).expect("Failed to write agl.rs");
}

/// Decode a JSON unicode escape string like "\\u0041" to "0041".
fn decode_json_unicode(s: &str) -> String {
    // The JSON has "\\uXXXX" which Rust reads as "\uXXXX"
    // We need to extract just the hex part
    if s.starts_with("\\u") {
        s[2..].to_string()
    } else {
        s.to_string()
    }
}

/// Generate font fingerprint phf map from font-fingerprints.json.
///
/// The JSON format is:
/// ```json
/// [
///   {
///     "sha256_hex": "abc123...",
///     "font_name": "Font Name (informational)",
///     "entries": [[gid1, codepoint1], [gid2, codepoint2], ...]
///   }
/// ]
/// ```
///
/// Each entry maps a glyph ID to a Unicode codepoint for a specific font
/// identified by its SHA-256 hash.
fn generate_font_fingerprints(out_dir: &Path, fingerprints_path: &Path) {
    let json_content =
        fs::read_to_string(fingerprints_path).expect("Failed to read font-fingerprints.json");

    let data: serde_json::Value =
        serde_json::from_str(&json_content).expect("Failed to parse font-fingerprints.json");

    let fonts = data.as_array().expect("font-fingerprints must be an array");

    let mut entries_arrays = String::new();
    let mut map_builder = phf_codegen::Map::new();

    // Store keys and values to ensure they live long enough
    let mut keys = Vec::new();
    let mut values = Vec::new();

    for font_entry in fonts {
        let sha256_hex = font_entry
            .get("sha256_hex")
            .and_then(|v| v.as_str())
            .expect("sha256_hex must be a string");

        // Skip empty hashes (placeholder entries)
        if sha256_hex.is_empty() {
            continue;
        }

        // Validate SHA-256 hex (64 hex chars = 32 bytes)
        if sha256_hex.len() != 64 {
            panic!(
                "SHA-256 hex must be 64 characters, got {}",
                sha256_hex.len()
            );
        }

        // Convert hex string to [u8; 32] bytes
        let hash_bytes: [u8; 32] = hex_decode_to_array(sha256_hex);

        // Get entries
        let entries = font_entry
            .get("entries")
            .and_then(|v| v.as_array())
            .expect("entries must be an array");

        let ident = format!("HASH_{}", sha256_hex.replace('-', "_"));

        // Build the entries array
        let mut entry_values = Vec::new();
        for entry in entries {
            let arr = entry.as_array().expect("entry must be an array");
            let gid = arr
                .get(0)
                .and_then(|v| v.as_u64())
                .expect("gid must be a number") as u16;
            let codepoint = arr
                .get(1)
                .and_then(|v| v.as_u64())
                .expect("codepoint must be a number") as u32;

            // Validate codepoint is a valid Unicode scalar value
            if !is_valid_unicode_scalar(codepoint) {
                panic!("Invalid Unicode scalar: 0x{:X}", codepoint);
            }

            entry_values.push(format!("({}, {})", gid, codepoint));
        }

        entries_arrays.push_str(&format!(
            r#"
static {}: &[(u16, u32)] = &[{}];
"#,
            ident,
            entry_values.join(", ")
        ));

        // Build the phf map key as a byte array literal
        let key_bytes: Vec<String> = hash_bytes.iter().map(|b| format!("0x{:02x}", b)).collect();

        let key = format!("[{}]", key_bytes.join(", "));
        let value = format!("&{}", ident);

        keys.push(key);
        values.push(value);
    }

    // Add entries to the map builder
    for (key, value) in keys.iter().zip(values.iter()) {
        map_builder.entry(key.as_str(), value.as_str());
    }

    let rust_code = format!(
        r#"
// Auto-generated font fingerprint phf map.
// Do not edit manually.
// Source: build/font-fingerprints.json

{}

/// Font fingerprint database.
///
/// Maps SHA-256 hashes of embedded font programs to their glyph ID to
/// Unicode codepoint mappings. This is Level 3 of the encoding fallback
/// chain, used when:
/// - /ToUnicode is missing or empty
/// - The embedded font subset has stripped glyph names
/// - The font binary matches a known fingerprint
///
/// The hash is computed over the DECODED font program bytes (post stream
/// decoding, pre-interpretation).
pub static FONT_FINGERPRINTS: phf::Map<[u8; 32], &'static [(u16, u32)]> = {};
"#,
        entries_arrays,
        map_builder.build()
    );

    fs::write(Path::new(out_dir).join("font_fingerprints.rs"), rust_code)
        .expect("Failed to write font_fingerprints.rs");
}

/// Decode a hex string to a [u8; 32] array.
fn hex_decode_to_array(hex: &str) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    for i in 0..32 {
        let byte_str = &hex[i * 2..i * 2 + 2];
        bytes[i] = u8::from_str_radix(byte_str, 16).expect("Invalid hex string");
    }
    bytes
}

/// Check if a value is a valid Unicode scalar value.
fn is_valid_unicode_scalar(cp: u32) -> bool {
    // Unicode scalar values: 0x0..=0xD7FF, 0xE000..=0x10FFFF
    (0x0..=0xD7FF).contains(&cp) || (0xE000..=0x10FFFF).contains(&cp)
}

/// Generate predefined CMap CID->Unicode mappings.
///
/// Reads JSON files from build/predefined-cmaps/ and generates phf maps
/// for CID->Unicode lookups. The JSON files contain mappings from CIDs
/// to their Unicode codepoint(s).
fn generate_predefined_cmaps(out_dir: &Path) {
    let predefined_cmaps_dir = Path::new("build/predefined-cmaps");

    // Generate each character collection
    generate_collection_cmap(out_dir, predefined_cmaps_dir, "adobe-japan1", "japan1");
    generate_collection_cmap(out_dir, predefined_cmaps_dir, "adobe-gb1", "gb1");
    generate_collection_cmap(out_dir, predefined_cmaps_dir, "adobe-cns1", "cns1");
    generate_collection_cmap(out_dir, predefined_cmaps_dir, "adobe-korea1", "korea1");
}

/// Generate a single character collection's CMap module.
fn generate_collection_cmap(out_dir: &Path, base_dir: &Path, json_name: &str, module_name: &str) {
    let json_path = base_dir.join(format!("{}.json", json_name));
    let out_path = out_dir.join(format!("predefined_cmap_{}.rs", module_name));

    // Check if the JSON file exists
    if !json_path.exists() {
        // Generate a stub implementation
        let rust_code = format!(
            r#"
// Auto-generated {collection} CID to Unicode mapping.
//
// Source: {json_name}.json (not found - stub implementation)
// Do not edit manually.

/// Look up a CID in the {collection} character collection.
///
/// Returns None if the CID is not assigned in {collection} or if the
/// predefined CMap data file is missing.
pub fn cid_to_unicode(cid: u32) -> Option<&'static [char]> {{
    let _ = cid;
    None
}}
"#,
            collection = module_name.to_uppercase(),
            json_name = json_name,
        );

        fs::write(&out_path, rust_code).expect(&format!("Failed to write {}", out_path.display()));
        return;
    }

    let json_content =
        fs::read_to_string(&json_path).expect(&format!("Failed to read {}", json_path.display()));

    let data: serde_json::Value = serde_json::from_str(&json_content)
        .expect(&format!("Failed to parse {}", json_path.display()));

    // Build phf map
    let mut map_builder = phf_codegen::Map::new();
    let mut arrays = String::new();

    if let Some(mappings) = data.as_object() {
        for (cid_str, unicode_value) in mappings {
            let cid: u32 = cid_str
                .parse()
                .expect(&format!("Invalid CID key: {}", cid_str));

            // Parse the Unicode value
            if let Some(unicode_str) = unicode_value.as_str() {
                let chars = parse_unicode_value(unicode_str);

                // Generate array name
                let array_ident = format!("CID_{}_{}", module_name.to_uppercase(), cid);

                // Build the array
                let char_literals: Vec<String> = chars
                    .iter()
                    .map(|c| format!("'\\u{{{:04X}}}'", *c as u32))
                    .collect();

                arrays.push_str(&format!(
                    r#"
static {}: &[char] = &[{}];
"#,
                    array_ident,
                    char_literals.join(", ")
                ));

                // Use u32 key as decimal literal
                map_builder.entry(cid, &format!("&{}", array_ident));
            }
        }
    }

    let rust_code = format!(
        r#"
// Auto-generated {collection} CID to Unicode mapping.
//
// Source: {json_name}.json
// Do not edit manually.

{arrays}

/// Look up a CID in the {collection} character collection.
///
/// Returns None if the CID is not assigned in {collection}.
pub fn cid_to_unicode(cid: u32) -> Option<&'static [char]> {{
    static MAP: phf::Map<u32, &'static [char]> = {map};

    // CIDs are 16-bit in these collections, but we use u32 for the API
    if cid <= u16::MAX as u32 {{
        MAP.get(&cid).copied()
    }} else {{
        None
    }}
}}
"#,
        collection = module_name.to_uppercase(),
        json_name = json_name,
        arrays = arrays,
        map = map_builder.build(),
    );

    fs::write(&out_path, rust_code).expect(&format!("Failed to write {}", out_path.display()));
}

/// Parse a Unicode value from JSON to a Vec<char>.
///
/// The JSON value can be:
/// - A single Unicode escape like "A" (A)
/// - Multiple Unicode escapes for ligatures like "fi" (fi)
fn parse_unicode_value(s: &str) -> Vec<char> {
    let mut chars = Vec::new();
    let mut chars_iter = s.chars();

    while let Some(c) = chars_iter.next() {
        if c == '\\' {
            // Expect \uXXXX
            if chars_iter.next() == Some('u') {
                // Read 4 hex digits
                let mut hex_str = String::new();
                for _ in 0..4 {
                    if let Some(hex_c) = chars_iter.next() {
                        hex_str.push(hex_c);
                    }
                }

                if let Ok(codepoint) = u32::from_str_radix(&hex_str, 16) {
                    if let Some(unicode_char) = char::from_u32(codepoint) {
                        chars.push(unicode_char);
                    }
                }
            }
        }
    }

    if chars.is_empty() && !s.is_empty() {
        // Fallback: try to parse as direct character
        chars.extend(s.chars());
    }

    chars
}
